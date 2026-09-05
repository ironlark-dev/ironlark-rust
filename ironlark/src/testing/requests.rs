//! The request double: records the question, answers with what the test stated.

use super::refuse_over_cap;
use crate::error::Result;
use crate::protocol::RequestSpec;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REQUESTS: RefCell<Vec<MadeRequest>> = const { RefCell::new(Vec::new()) };
    static RESPONSES: RefCell<HashMap<&'static str, Vec<u8>>> = RefCell::new(HashMap::new());
}

/// One request the code under test made: which request, and what it carried.
///
/// [`take_requests`] drains these, in the order they were made. What the request
/// was answered with is not in the record, because the code under test already
/// has it: the answer is what [`request`](crate::client::request) returned. The
/// record is the other half of that picture, the question, which a caller that
/// only looked at the answer never proves it asked correctly.
///
/// It has no counterpart to [`RaisedSignal::to`](super::RaisedSignal::to), because a request has no
/// narrowed form. Routing takes it to the mod whose schema declares it, so there
/// is no addressee to record.
///
/// Reading the payload back is [`prost`] decoding against the
/// request's own type, which is how a test asserts what was asked rather than
/// only that something was.
///
/// ```
/// use ironlark::client::prelude::*;
/// use ironlark::prost::Message;
/// use ironlark::testing::{block_on, set_response, take_requests};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it, over the door schema's
/// //
/// //     service Server { rpc Open(OpenRequest) returns (Latch); }
/// //
/// // `request` is the client prelude's; `Message` is what gives a generated
/// // type `decode`.
/// async fn shove(force: u32) -> bool {
///     match request(&protocol::OpenRequest { force }).await {
///         Ok(latch) => latch.open,
///         Err(e) => {
///             log::error!("the server did not answer: {e}");
///             false
///         }
///     }
/// }
///
/// // No server half stands up here, so the test states the answer.
/// set_response::<protocol::OpenRequest>(protocol::Latch { open: true });
/// assert!(block_on(shove(7)));
///
/// let asked = take_requests();
/// // The count first: a drain read straight into a loop passes when empty.
/// assert_eq!(asked.len(), 1);
/// // The declared name, as `rpc Open` spells it. The Request suffix is on the
/// // payload type and never on the name.
/// assert_eq!(asked[0].name, "open");
///
/// let Ok(sent) = protocol::OpenRequest::decode(&asked[0].payload[..]) else {
///     panic!("a recorded payload is what the request encoded");
/// };
/// assert_eq!(sent.force, 7);
/// ```
///
/// # Nothing refuses
///
/// It is a record a drain hands over, so there is no call here to turn down.
/// What can be turned down happens before the record exists: a payload over the
/// host's byte cap leaves nothing behind. A request nothing answered is
/// recorded all the same, and its refusal reaches the call site instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadeRequest {
    /// The declared name, as the request payload type's declaration spells it.
    pub name: &'static str,
    /// The encoded payload.
    pub payload: Vec<u8>,
}

/// The double's record of a request, under the same cap. Finding the answer is
/// the caller's: [`configured_response`] first, then the registered handler.
pub(crate) fn request(name: &'static str, payload: &[u8]) -> Result<()> {
    refuse_over_cap(payload)?;
    REQUESTS.with(|r| {
        r.borrow_mut().push(MadeRequest {
            name,
            payload: payload.to_vec(),
        });
    });
    Ok(())
}

pub(crate) fn configured_response(name: &'static str) -> Option<Vec<u8>> {
    RESPONSES.with(|r| r.borrow().get(name).cloned())
}

/// States what a request answers, instead of letting a handler answer it.
///
/// [`request`](crate::client::request) is answered by the handler this same
/// test registered with [`respond`](crate::protocol::RequestSpec::respond)
/// whenever there is one, which is how a mod's two halves prove each other. Two
/// cases have no such handler: a request another mod declares, whose answering
/// half is not in this process at all, and a client half a test means to drive
/// without standing its server half up. This is the answer for both.
///
/// The response is stated typed and encoded here, so a test never touches
/// bytes. It replaces any answer stated before it for the same request, and it
/// stays stated until replaced: this is session state rather than a queue, so
/// one call answers every request the test makes.
///
/// A stated answer wins over a registered handler. A test that states an answer
/// means to control it, and that order is the one way the two can meet.
///
/// ```
/// use ironlark::client::prelude::*;
/// use ironlark::testing::{block_on, set_response, take_requests};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // A real crate's src/protocol.rs is generated from its protocol.proto,
/// // where `service Server { rpc Open(OpenRequest) returns (Latch); }` is the
/// // declaration and the request's payload type carries the suffix.
/// async fn shove(force: u32) -> bool {
///     match request(&protocol::OpenRequest { force }).await {
///         Ok(latch) => latch.open,
///         Err(e) => {
///             log::error!("the server did not answer: {e}");
///             false
///         }
///     }
/// }
///
/// // No server half stands up in this test, so the test answers.
/// set_response::<protocol::OpenRequest>(protocol::Latch { open: true });
///
/// assert!(block_on(shove(1)));
/// // The request is recorded either way, so what was asked stays assertable.
/// let asked = take_requests();
/// assert_eq!(asked.len(), 1);
/// assert_eq!(asked[0].name, "open");
/// ```
///
/// # Nothing refuses
///
/// A generated payload type always encodes, so stating an answer cannot fail.
/// An answer stated for a request the code under test never makes is simply
/// never read.
pub fn set_response<R: RequestSpec>(response: R::Response) {
    let bytes = crate::protocol::encode(&response);
    RESPONSES.with(|r| {
        r.borrow_mut().insert(R::NAME, bytes);
    });
}

/// Drains the requests the code under test made.
///
/// Every [`request`](crate::client::request) since the last drain, in order, as
/// [`MadeRequest`] rows, whether the answer came from [`set_response`], from a
/// registered handler, or was a refusal. The buffer is emptied, so a second
/// call answers with what has happened since.
///
/// This is where a test asserts what a client half asked for, separately from
/// what it did with the answer.
///
/// ```
/// use ironlark::client::prelude::*;
/// use ironlark::testing::{block_on, set_response, take_requests};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// set_response::<protocol::OpenRequest>(protocol::Latch { open: true });
///
/// block_on(async {
///     let _ = request(&protocol::OpenRequest { force: 3 }).await;
/// });
///
/// let asked = take_requests();
/// assert_eq!(asked.len(), 1);
/// assert_eq!(asked[0].name, "open");
/// assert!(take_requests().is_empty());
/// ```
///
/// # What a request does here, and what it does in a session
///
/// The payload is encoded, refused if it is over the host's byte cap, recorded,
/// and then answered in the order the module page states. A handler that
/// answers runs in this process, on a [`Context`](crate::Context) naming no event and holding a
/// participant the test never chose, so a handler that reads its caller is one
/// to call directly instead.
///
/// A session refuses what this cannot: an answering half that is not running,
/// one too far behind to take the call, and a guest that failed outright all
/// reach the caller as [`Refused`](crate::ErrorKind::Refused).
pub fn take_requests() -> Vec<MadeRequest> {
    REQUESTS.with(|r| r.borrow_mut().drain(..).collect())
}
