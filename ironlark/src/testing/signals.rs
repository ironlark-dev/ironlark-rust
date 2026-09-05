//! The raise double: records what was raised, and delivers it on demand.

use super::refuse_over_cap;
use crate::context::Context;
use crate::error::Result;
use crate::ids::SessionId;
use crate::protocol::{SignalSpec, Transit};
use std::cell::RefCell;

thread_local! {
    static SIGNALS: RefCell<Vec<RaisedSignal>> = const { RefCell::new(Vec::new()) };
}

/// One raise the code under test made: which signal, who hears it, whether it
/// was narrowed, and what it carried.
///
/// [`signal`](crate::server::signal) and
/// [`signal_to`](crate::server::signal_to) both land here, because they are one
/// announcement differing by exactly one thing: whether it was narrowed to one
/// participant's machine. That difference is [`to`](RaisedSignal::to), so a
/// test that cares reads the field and a test that does not ignores it.
///
/// [`take_signals`] drains these, in the order they were raised. Nothing was
/// delivered: a session would have carried each one to its subscribers, and here
/// the raise stopped at this record. Handing
/// [`payload`](RaisedSignal::payload) to [`dispatch_signal`] is how a test
/// carries it the rest of the way.
///
/// Reading the payload back is [`prost`] decoding against the
/// declared type, which is what proves the mod put the right values in rather
/// than merely that it raised something.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::prost::Message;
/// use ironlark::testing::take_signals;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it. `Latch` declares `option (ironlark.signal) = CLIENTS`, so the
/// // narrowed verb accepts it. `signal`, `signal_to` and `SessionId` are the
/// // server prelude's; `Message` is what gives a generated type `decode`.
/// fn tell_the_joiner_then_everyone(joiner: SessionId) {
///     if let Err(e) = signal_to(joiner, &protocol::Latch { open: false }) {
///         log::warn!("the joiner was not told the door state: {e}");
///     }
///     if let Err(e) = signal(&protocol::Latch { open: true }) {
///         log::warn!("no screen was told the door moved: {e}");
///     }
/// }
///
/// tell_the_joiner_then_everyone(SessionId::new(11));
///
/// let raised = take_signals();
/// // The count first: a drain read straight into a loop passes when empty.
/// assert_eq!(raised.len(), 2);
///
/// // The declared name, as the schema's `message Latch` spells it.
/// assert_eq!(raised[0].name, "latch");
/// // The narrowing is one field. Some(who) is the narrowed raise.
/// assert_eq!(raised[0].to, Some(SessionId::new(11)));
/// assert_eq!(raised[1].to, None);
///
/// // What the mod actually put in it, rather than that it raised at all.
/// let Ok(shut) = protocol::Latch::decode(&raised[0].payload[..]) else {
///     panic!("a recorded payload is what the raise encoded");
/// };
/// assert!(!shut.open);
/// ```
///
/// # Nothing refuses
///
/// It is a record a drain hands over, so there is no call here to turn down. The
/// refusal a raise can meet happens before the record exists: a payload over the
/// host's byte cap is refused whole, and nothing is recorded for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaisedSignal {
    /// The declared name, as the payload type's own declaration spells it.
    pub name: &'static str,
    /// How the host is to carry it, from the declaration. A raise cannot
    /// choose this, so it is here for the one assertion that is not circular:
    /// that a mod raising high-rate state declared it supersedable, and a mod
    /// raising a one-shot did not.
    pub transit: Transit,
    /// The one participant's machine it was narrowed to, or `None` for the
    /// whole audience.
    pub to: Option<SessionId>,
    /// The encoded payload, ready to hand to [`dispatch_signal`].
    pub payload: Vec<u8>,
}

/// The double's body for both raise verbs, reading the declaration off the
/// signal's own spec. `to` is `None` for the whole audience.
pub(crate) fn signal<S: SignalSpec>(to: Option<SessionId>, payload: &[u8]) -> Result<()> {
    refuse_over_cap(payload)?;
    SIGNALS.with(|s| {
        s.borrow_mut().push(RaisedSignal {
            name: S::NAME,
            transit: S::TRANSIT,
            to,
            payload: payload.to_vec(),
        });
    });
    Ok(())
}

/// Drains the raises the code under test made.
///
/// Every [`signal`](crate::server::signal) and
/// [`signal_to`](crate::server::signal_to) since the last drain, in the order
/// they were raised, as [`RaisedSignal`] rows. The buffer is emptied, so a
/// second call answers with what has happened since.
///
/// Assert on [`name`](RaisedSignal::name) and [`to`](RaisedSignal::to) here,
/// and hand [`payload`](RaisedSignal::payload) to [`dispatch_signal`] when the
/// test wants a subscriber to act on it.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::take_signals;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The narrowed raise: one participant's machine hears it, the others do
/// // not. Both raise verbs are synchronous, so nothing here is awaited.
/// fn tell_one(who: SessionId) {
///     if let Err(e) = signal_to(who, &protocol::Latch { open: false }) {
///         log::warn!("the door state did not go out: {e}");
///     }
/// }
///
/// let only = SessionId::new(11);
/// tell_one(only);
///
/// let raised = take_signals();
/// // The count first: a drain read straight into a loop passes when empty.
/// assert_eq!(raised.len(), 1);
/// assert_eq!(raised[0].name, "latch");
/// assert_eq!(raised[0].to, Some(only));
/// assert!(take_signals().is_empty());
/// ```
///
/// # What raising does here, and what it does in a session
///
/// The double encodes the payload, refuses it if it is over the host's byte
/// cap, records it and answers `Ok`. Nothing is delivered, neither to a
/// subscriber nor to this mod's own other half. So an `Ok` says the payload
/// encoded and the mod decided to raise it, and nothing about anyone hearing
/// it — which is nearly what `Ok` says in a session, where it says the host
/// took the raise.
///
/// A session refuses three things this cannot. A mod raising more in one tick
/// than its budget allows is refused typed, and no number of raises refuses
/// here. A signal id the session cannot name is refused, where native
/// resolution mints an id for any string. And a signal declaring
/// [`Keep::Newest`](crate::protocol::Keep::Newest) is measured against the
/// smaller of the host's two caps, where the double knows one cap and applies
/// it to every raise, so a superseding payload between the two sizes is
/// recorded here and refused in a game. A session also qualifies the name with
/// the mod that declared it, where the recorded name is the bare one the
/// declaration spells.
pub fn take_signals() -> Vec<RaisedSignal> {
    SIGNALS.with(|s| s.borrow_mut().drain(..).collect())
}

/// Delivers a raised signal to this mod, as the engine's `on-signal` would.
///
/// One export carries every signal in both realms, so this is one verb for
/// every route: a signal that stayed on the server realm's bus, one that stayed
/// on this machine's client bus, and one that crossed the network from a server
/// half. A subscriber registers its handler with
/// [`observe`](crate::protocol::SignalSpec::observe) on the payload type, and a
/// session enters that handler with the signal, the raising mod's host-stamped
/// id and the bytes. This is that entry, performed by the test.
///
/// The signal is named by its own payload type rather than by a string, so a
/// misspelling cannot reach here: it is a compile error instead of a delivery
/// that quietly finds no handler. `source` is the raising mod's full id, which
/// is a run-time string because it names another mod. `payload` is encoded
/// bytes, and the only way to obtain them is to have a raise produce them.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{block_on, context, dispatch_signal, take_signals};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The mod's own id, "acme:doors" below, is the <author>/<mod> directory it
/// // installs to.
/// ironlark::state! {
///     static SEEN: Vec<String> = Vec::new();
/// }
///
/// // The handler takes the payload directly; the source is host-stamped.
/// async fn on_opened(_ctx: Context, from: SourceId, fact: protocol::Opened) {
///     if resolve::source("acme:doors") == Ok(from) {
///         SEEN.update(|seen| seen.push(fact.name));
///     }
/// }
///
/// protocol::Opened::observe(on_opened);
///
/// // Raising records the bytes; here the test does the delivering.
/// assert!(signal(&protocol::Opened { name: "front".to_string() }).is_ok());
///
/// let event = context(Tick::new(30), Tick::new(30));
/// for raised in take_signals() {
///     block_on(dispatch_signal::<protocol::Opened>(event, "acme:doors", raised.payload));
/// }
///
/// assert_eq!(SEEN.update(|seen| seen.clone()), vec!["front".to_string()]);
/// ```
///
/// # What delivery checks here, and what a session checks
///
/// This is one of the doubles that does the thing rather than refusing or
/// pretending. The handler really runs, on the [`Context`] the test chose. What
/// is missing is every gate around it:
///
/// - **Subscription is not enforced.** Registering an observer subscribes on
///   the wasm build and does nothing here, so any registered handler is
///   reachable. In a session the bus fans a signal only to the mods that
///   subscribed to that name.
/// - **Self-delivery is the test's choice.** A session delivers to every
///   subscriber including the raiser. Here nothing is delivered until the test
///   delivers it, so the loop above is what self-delivery looks like.
/// - **Any source resolves.** Native resolution mints an id for whatever string
///   it is first given, so a mod id a session would refuse as not enabled is
///   accepted here. A misspelled `source` becomes a different id, and a handler
///   gating on origin then stays quiet.
/// - **Nothing is superseded or reordered.** A declaration asking for the
///   newest raise to supersede an older one is served by the host, and here
///   every delivery the test makes is made.
pub async fn dispatch_signal<S: SignalSpec>(ctx: Context, source: &str, payload: Vec<u8>) {
    let from = match crate::shared::resolve::source(source) {
        Ok(id) => id,
        Err(e) => panic!("test signal from unresolvable source {source}: {e}"),
    };
    match S::resolved() {
        Ok(id) => crate::host::dispatch::dispatch_signal(ctx, id, from, payload).await,
        Err(e) => panic!("test signal on unresolvable name {}: {e}", S::NAME),
    }
}
