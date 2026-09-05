//! Declared traffic: the registration that resolves a name, and the dispatch
//! the export glue enters on every arrival.
//!
//! Nothing here is public. A handler is registered through the payload type it
//! is written against — [`SignalSpec::observe`](crate::protocol::SignalSpec::observe)
//! for an announcement, [`RequestSpec::respond`](crate::protocol::RequestSpec::respond)
//! for a question — and both doors land in this file. Registering resolves the
//! declared name into the compact id the session numbered it with, subscribes
//! where a signal was observed, and puts the handler in the table
//! [`dispatch_signal`] and [`dispatch_request`] look up.

use crate::context::Context;
use crate::error::{Refusal, Result};
use crate::ids::{RequestId, SignalId, SourceId};
use crate::protocol::{self, RequestSpec, SignalSpec, signal};
use crate::server::player::Player;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

type BoxFut<T> = Pin<Box<dyn Future<Output = T>>>;
type SignalHandler = Rc<dyn Fn(Context, SourceId, Vec<u8>) -> BoxFut<()>>;
type RequestHandler = Rc<dyn Fn(Context, Player, Vec<u8>) -> BoxFut<Result<Vec<u8>, Refusal>>>;

#[derive(Default)]
struct Tables {
    signals: HashMap<SignalId, SignalHandler>,
    requests: HashMap<RequestId, RequestHandler>,
    warned: HashSet<(SignalId, SourceId)>,
}

crate::state! {
    static TABLES: Tables = Tables::default();
}

pub(crate) fn observe<P, F, Fut>(handler: F)
where
    P: SignalSpec,
    F: Fn(Context, SourceId, P) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    let name = P::NAME;
    // Resolved and subscribed before anything is built, so a name this session
    // does not carry costs no allocation and reports at the registration.
    let id = match P::resolved().and_then(|id| signal::subscribe(id, name).map(|()| id)) {
        Ok(id) => id,
        Err(e) => {
            log::error!("signal '{name}' is not observed: {}", e.message());
            return;
        }
    };
    let handler = Rc::new(handler);
    let dispatch: SignalHandler = Rc::new(move |ctx, source, bytes| {
        let handler = Rc::clone(&handler);
        Box::pin(async move {
            match protocol::decode::<P>(&bytes) {
                Ok(payload) => handler(ctx, source, payload).await,
                Err(e) => log::warn!("signal {name} dropped: {}", e.message()),
            }
        })
    });
    if TABLES.update(|t| t.signals.insert(id, dispatch)).is_some() {
        log::debug!("signal '{name}' is now heard by the handler registered last");
    }
}

pub(crate) fn respond<R, F, Fut>(handler: F)
where
    R: RequestSpec,
    F: Fn(Context, Player, R) -> Fut + 'static,
    Fut: Future<Output = Result<R::Response, Refusal>> + 'static,
{
    let name = R::NAME;
    let id = match R::resolved() {
        Ok(id) => id,
        Err(e) => {
            log::error!("request '{name}' is not answered: {}", e.message());
            return;
        }
    };
    let handler = Rc::new(handler);
    let dispatch: RequestHandler = Rc::new(move |ctx, caller, bytes| {
        let handler = Rc::clone(&handler);
        Box::pin(async move {
            let request = protocol::decode_refusing::<R>(&bytes)?;
            let answer = handler(ctx, caller, request).await?;
            Ok(protocol::encode(&answer))
        })
    });
    if TABLES.update(|t| t.requests.insert(id, dispatch)).is_some() {
        log::debug!("request '{name}' is now answered by the handler registered last");
    }
}

pub(crate) async fn dispatch_signal(
    ctx: Context,
    id: SignalId,
    source: SourceId,
    payload: Vec<u8>,
) {
    let handler = TABLES.update(|t| t.signals.get(&id).map(Rc::clone));
    match handler {
        Some(h) => h(ctx, source, payload).await,
        None => {
            // Deduped per (signal, source): a hostile raiser repeating an
            // unobserved signal cannot flood the log.
            let fresh = TABLES.update(|t| t.warned.insert((id, source)));
            if fresh {
                log::warn!("no handler observes signal {id} raised by source {source}");
            }
        }
    }
}

pub(crate) async fn dispatch_request(
    ctx: Context,
    caller: Player,
    id: RequestId,
    payload: Vec<u8>,
) -> Result<Vec<u8>, Refusal> {
    let handler = TABLES.update(|t| t.requests.get(&id).map(Rc::clone));
    match handler {
        Some(h) => h(ctx, caller, payload).await,
        None => Err(Refusal::Unknown {
            message: format!("no handler answers request {id}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EventId, SessionId, Tick};
    use crate::testing::{FakePlayer, block_on};

    mod protocol {
        ironlark_macros::protocol!("doctest/protocol.proto");
    }

    fn protocol_bytes<T: prost::Message>(value: &T) -> Vec<u8> {
        crate::protocol::encode(value)
    }

    fn caller() -> Player {
        FakePlayer::new(SessionId::new(7)).into_player()
    }

    fn ctx() -> Context {
        Context {
            id: EventId::new(11),
            raised_at: Tick::new(4),
            now: Tick::new(4),
        }
    }

    fn answer_open(force: u32) -> Result<Vec<u8>, Refusal> {
        let Ok(id) = <protocol::OpenRequest as RequestSpec>::resolved() else {
            panic!("the native resolve cannot fail");
        };
        block_on(dispatch_request(
            ctx(),
            caller(),
            id,
            protocol_bytes(&protocol::OpenRequest { force }),
        ))
    }

    #[test]
    fn a_registered_request_answers_typed() {
        // The closure names no type: `respond` is reached through the request
        // type, so its argument is settled before the body is read.
        protocol::OpenRequest::respond(|_, _, ask| async move {
            Ok(protocol::Latch {
                open: ask.force > 0,
            })
        });
        let Ok(answer) = answer_open(3) else {
            panic!("the handler answers");
        };
        let Ok(latch) = crate::protocol::decode::<protocol::Latch>(&answer) else {
            panic!("the answer decodes");
        };
        assert!(latch.open);
    }

    #[test]
    fn an_unknown_request_is_a_refusal_not_a_panic() {
        let out = block_on(dispatch_request(
            ctx(),
            caller(),
            RequestId::new(9999),
            Vec::new(),
        ));
        assert!(matches!(out, Err(Refusal::Unknown { .. })));
    }

    #[test]
    fn a_second_registration_replaces_the_handler() {
        protocol::OpenRequest::respond(
            |_, _, _ask| async move { Ok(protocol::Latch { open: true }) },
        );
        protocol::OpenRequest::respond(
            |_, _, _ask| async move { Ok(protocol::Latch { open: false }) },
        );
        let Ok(answer) = answer_open(1) else {
            panic!("the handler answers");
        };
        let Ok(latch) = crate::protocol::decode::<protocol::Latch>(&answer) else {
            panic!("the answer decodes");
        };
        assert!(!latch.open, "the handler registered last is the live one");
    }

    #[test]
    fn signals_dispatch_and_unobserved_ones_warn_once_per_source() {
        crate::testing::install_logger();
        protocol::Latch::observe(|_ctx, _source, latch| async move {
            assert!(latch.open);
        });
        let Ok(id) = <protocol::Latch as SignalSpec>::resolved() else {
            panic!("the native resolve cannot fail");
        };
        let source = SourceId::new(3);
        block_on(dispatch_signal(
            ctx(),
            id,
            source,
            protocol_bytes(&protocol::Latch { open: true }),
        ));

        crate::testing::take_logs();
        let stranger = SignalId::new(9999);
        block_on(dispatch_signal(ctx(), stranger, source, Vec::new()));
        block_on(dispatch_signal(ctx(), stranger, source, Vec::new()));
        let warns = crate::testing::take_logs();
        assert_eq!(warns.len(), 1, "the second identical warn is deduped");
    }
}
