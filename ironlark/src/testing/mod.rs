//! Running a mod's own handlers under `cargo test`, with no session present.
//!
//! The crate compiles for two targets. Built for `wasm32-wasip2` a host verb is
//! a real import and the engine answers it. Built for the machine an author
//! develops on there is no engine, so every host verb carries a second body
//! that answers without one, and the traffic those bodies accept lands in the
//! buffers this module drains. What a mod proves that way is its own logic: its
//! state machine, its refusals, and the traffic it decides to raise.
//!
//! A test has one shape. State what the session holds, run the handler, drain
//! what it did. [`FakePlayer`] states the participant an event is about,
//! [`set_participants`] states who is connected, [`context`] states the event,
//! [`block_on`] runs the handler, and [`take_signals`] is the drain.
//!
//! ```
//! use ironlark::server::prelude::*;
//! use ironlark::testing::{FakePlayer, block_on, context, take_signals};
//! # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
//! // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
//! // writes it: out come the payload types and, per declaration, a marker
//! // carrying the spec. `Latch` here declares `option (ironlark.signal) =
//! // CLIENTS`, so every client half of this mod hears it.
//! ironlark::state! {
//!     static ARRIVED: u32 = 0;
//! }
//!
//! async fn announce(_ctx: Context, player: Player) {
//!     let count = ARRIVED.update(|n| {
//!         *n += 1;
//!         *n
//!     });
//!     log::info!("{player} is number {count}");
//!     // Synchronous: a signal awaits no answer, so there is nothing to await.
//!     if let Err(e) = signal(&protocol::Latch { open: false }) {
//!         log::warn!("the door state did not reach the arrival: {e}");
//!     }
//! }
//!
//! let player = FakePlayer::new(SessionId::new(7)).into_player();
//! block_on(announce(context(Tick::new(54), Tick::new(54)), player));
//!
//! assert_eq!(ARRIVED.get(), 1);
//! let raised = take_signals();
//! // The count first: a drain read straight into a loop passes when empty.
//! assert_eq!(raised.len(), 1);
//! assert_eq!(raised[0].name, "latch");
//! // None is the whole audience; a narrowed raise carries the participant.
//! assert_eq!(raised[0].to, None);
//! ```
//!
//! # The double does not answer one way
//!
//! Read this before trusting a green test. What a native verb answers is chosen
//! per verb, so one verb says nothing about the next and each page below states
//! its own. Five shapes exist.
//!
//! - **It refuses as the host refuses.** A raise or a request whose payload is
//!   over the host's byte cap answers [`TooLarge`](crate::ErrorKind::TooLarge)
//!   here with the same sentence a session gives, on every route. The host holds
//!   a second and smaller cap for a superseding raise, and the double applies
//!   that one to nothing.
//! - **It refuses because there is no world.** Every
//!   [`Entity`](crate::server::Entity) verb, [`find`](crate::server::find),
//!   [`body_of`](crate::server::body_of) and
//!   [`Player::body`](crate::server::Player::body) answer that world verbs need
//!   a session, and [`ErrorKind::Other`](crate::ErrorKind::Other) is the kind on
//!   every one of them.
//! - **It answers what the test stated.**
//!   [`session`](crate::server::session) reads [`set_participants`], and a
//!   request reads [`set_response`].
//! - **It invents a plausible value.**
//!   [`map::spawn_points`](crate::server::map::spawn_points) yields one point
//!   at the origin, [`spatial`](crate::server::spatial) nothing touched,
//!   [`context`] an event id of zero, and name resolution an id for any string.
//! - **It records, or does nothing at all.** Raises reach [`take_signals`],
//!   requests [`take_requests`], playback [`take_plays`], placement
//!   [`take_spawn_commands`]; and
//!   [`ui::set_overlay_text`](crate::client::ui::set_overlay_text) has an empty
//!   body.
//!
//! The invented value and the silence are what let a test pass for the wrong
//! reason. A gamemode reading the spawn list gets one point, places everyone on
//! it and looks correct, while the round-robin the test was written to prove
//! never ran. So assert on what the handler *did*, which the drains carry,
//! rather than on what a verb told it.
//!
//! # What travels, and what only gets recorded
//!
//! In a session a raise really travels. Here it stops at a buffer: the two
//! raise verbs record and answer `Ok`, and the test hands the bytes on itself
//! with [`dispatch_signal`]. Raise, drain, deliver.
//!
//! [`request`](crate::client::request) is the exception, because a caller is
//! waiting and a buffer cannot answer one. It is recorded like a raise and then
//! answered by the first of these that applies: the response [`set_response`]
//! stated; the handler this test registered with
//! [`respond`](crate::protocol::RequestSpec::respond), awaited there and then,
//! so a mod's two halves answer one another; or a typed refusal, which is the
//! shape a session gives when the answering half registered nothing.
//!
//! # What is not here
//!
//! No input dispatch, and none is needed: an author's hook is a plain
//! `async fn` the generated table reaches by [`HookId`](crate::HookId), so a
//! test calls it and runs it under [`block_on`]. No mock host either, and no
//! per-tick raise budget, so a mod a session would refuse for raising too much
//! raises freely here.
//!
//! Every buffer and the stated session are thread-local, so parallel test cases
//! cannot see each other's records. And the crate's headline rule, which this
//! cannot catch: a handle answers only inside the event that lent it, enforced
//! by an epoch the component build alone carries, so a handler that hoards a
//! [`FakePlayer`] drains cleanly here and refuses in a game.
mod executor;
mod logs;
mod players;
mod plays;
mod requests;
mod signals;
mod spawns;

pub use executor::block_on;
pub use logs::{install_logger, take_logs};
pub use players::{FakeParticipant, FakePlayer, set_participants};
pub use plays::take_plays;
pub use requests::{MadeRequest, set_response, take_requests};
pub use signals::{RaisedSignal, dispatch_signal, take_signals};
pub use spawns::{SpawnCommand, take_spawn_commands};

pub(crate) use logs::record_log;
pub(crate) use players::{stated_name, stated_participants};
pub(crate) use plays::record_play;
pub(crate) use requests::{configured_response, request};
pub(crate) use signals::signal;
pub(crate) use spawns::record_spawn_command;

use crate::context::Context;
use crate::error::{Error, Result, code};
use crate::ids::{EventId, Tick};

/// The host's own cap on one payload, mirrored so the double refuses the sizes
/// a session refuses.
const PAYLOAD_MAX: usize = 64 * 1024;

/// The host's cap, applied where the host applies it: whole, at the raise,
/// before anything is recorded.
fn refuse_over_cap(payload: &[u8]) -> Result<()> {
    if payload.len() > PAYLOAD_MAX {
        return Err(Error::from_wire(
            code::TOO_LARGE,
            format!(
                "payload of {} bytes exceeds the {PAYLOAD_MAX}-byte cap",
                payload.len()
            ),
            Vec::new(),
        ));
    }
    Ok(())
}

/// Builds the event a test hands a handler, from the ticks it chooses.
///
/// A session mints one per event, and a handler reads it for when the event was
/// raised and what the clock says now. Natively nothing has happened, so a test
/// states what the handler should believe. The id is the same for every context
/// this makes, because nothing native distinguishes two events. A test that
/// needs distinct ids builds [`Context`] itself, whose fields are public.
///
/// [`Context::cause`] and [`Context::instance`] refuse on a context from here.
/// There is no event behind it, and a made-up answer would be worse than a
/// refusal.
///
/// They refuse under a different kind than a session's refusal for the same
/// situation. Here it reads [`StaleId`](crate::ErrorKind::StaleId), and a
/// session asked about an event that is not the one in flight sends the code
/// that reads [`Refused`](crate::ErrorKind::Refused). Which one is right is not
/// settled, so a test that asserts either kind is asserting one build target
/// and will change when the answer is chosen. Assert that it refused, and read
/// [`Error::message`](crate::Error::message) for what happened.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::context;
///
/// // context(raised_at, now), in that order.
/// let ctx = context(Tick::new(54), Tick::new(70));
/// assert_eq!(ctx.now - ctx.raised_at, 16);
/// ```
pub fn context(raised_at: Tick, now: Tick) -> Context {
    Context {
        id: EventId::new(0),
        raised_at,
        now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::ids::SessionId;
    use crate::protocol::{Clients, Keep, Order, SignalSpec, Transit};

    #[derive(Clone, PartialEq, prost::Message)]
    struct Moved {
        #[prost(bool, tag = "1")]
        open: bool,
    }

    impl SignalSpec for Moved {
        type Audience = Clients;
        const NAME: &'static str = "moved";
        const TRANSIT: Transit = Transit {
            keep: Keep::Newest,
            order: Order::InOrder,
        };
    }

    #[test]
    fn a_raise_over_the_cap_is_refused_whole_and_recorded_nowhere() {
        let over = vec![0u8; PAYLOAD_MAX + 1];
        let e = match signal::<Moved>(None, &over) {
            Err(e) => e,
            Ok(()) => panic!("an over-cap raise must be refused"),
        };
        assert_eq!(e.kind(), ErrorKind::TooLarge);
        assert!(
            take_signals().is_empty(),
            "a refused raise leaves no record"
        );
    }

    #[test]
    fn the_cap_refuses_a_request_the_same_way() {
        let over = vec![0u8; PAYLOAD_MAX + 1];
        assert!(request("big", &over).is_err());
        assert!(take_requests().is_empty());
    }

    #[test]
    fn a_narrowed_raise_carries_its_addressee_and_its_transit() {
        let only = SessionId::new(11);
        assert!(signal::<Moved>(Some(only), &[1]).is_ok());
        let raised = take_signals();
        assert_eq!(raised.len(), 1);
        assert_eq!(raised[0].to, Some(only));
        assert_eq!(raised[0].name, "moved");
        assert_eq!(raised[0].transit.keep, Keep::Newest);
    }
}
