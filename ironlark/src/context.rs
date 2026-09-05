//! The event a hook is handling, and what the host answers about it.

use crate::error::Result;
use crate::ids::{EventId, SessionId, SourceId, Tick};

/// The event a hook is handling, handed to it as its first argument.
///
/// Every hook but `init` opens with one, on both halves of a mod: the server
/// half's [`ServerMod`](crate::server::ServerMod) and the client half's
/// [`ClientMod`](crate::client::ClientMod), and every handler registered under
/// a declared name in either realm. `init` is the exception, because nothing has
/// happened at the moment it runs.
///
/// It names one event and it says when. The name is [`id`](Context::id),
/// minted once where the event was born and identical on every mod that hears
/// it and on both halves. The when is two ticks, [`raised_at`](Context::raised_at)
/// and [`now`](Context::now), whose difference is how far behind the world the
/// handler is running.
///
/// Those facts are fields, because ordering and lag compensation read them on
/// every event and a field costs nothing. Everything else the host says about
/// an event is a question instead, [`cause`](Context::cause) and
/// [`instance`](Context::instance) among them, because few handlers ask and a
/// call is paid only where it is made.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `ServerMod` and `Context` both come from the prelude imported above.
/// // The same hook, with the same first argument, exists on `ClientMod`.
/// struct Clock;
///
/// impl ServerMod for Clock {
///     async fn on_tick(ctx: Context, _dt: f32) {
///         // Two ticks subtract to a plain count of ticks.
///         let age = ctx.now - ctx.raised_at;
///         if age > 8 {
///             log::warn!("step {} was handled {age} ticks late", ctx.id);
///         }
///     }
/// }
/// ```
///
/// It is `Copy` and carries no borrow, so passing it down into a helper costs
/// nothing and needs no lifetime.
///
/// # What the age is for
///
/// The host measures the same difference. On the server half, an event it
/// delivers far enough behind the tick it was raised on still arrives, because
/// a mod's bookkeeping needs it, but it no longer lets the mod reach the
/// participant the event is about. A handler that acts on somebody's body reads
/// the age first and does bookkeeping only when it is large.
///
/// # A context outlives its event and stops answering
///
/// Nothing reclaims a `Context` the way the host reclaims a
/// [`Player`](crate::server::Player). The three facts stay readable forever, so
/// a mod may keep one in a [`State`](crate::State) cell and print the id of an
/// event long finished. The questions are the part that expires. The host
/// answers about the event in flight and refuses about any other, so a stashed
/// context asked one hook later is refused rather than answered for whatever is
/// running now.
///
/// In a session that refusal reads as
/// [`Refused`](crate::ErrorKind::Refused). Built for the machine you develop
/// on, where no event was ever in flight, it reads as
/// [`StaleId`](crate::ErrorKind::StaleId). Branch on the presence of an error,
/// never on which of those two it is.
///
/// # The field set is final
///
/// The fields never grow. The contract carries them as a record, and a record
/// cannot gain a field without every already-built mod failing to load.
/// Everything the host later learns to say about an event arrives as another
/// question taking [`id`](Context::id), which is why the id is a field in the
/// first place.
///
/// # Raising and receiving events is elsewhere
///
/// Nothing here raises an event and nothing here subscribes to one. Being told
/// that something happened is a hook the host calls. Announcing something is
/// [`server::signal`](crate::server::signal), and whether that reaches the other
/// server halves or every client half is the schema's declaration rather than
/// the call site's. Asking the server half a question is
/// [`client::request`](crate::client::request), answered by a handler the server
/// half registered with [`respond`](crate::protocol::RequestSpec::respond) on
/// the request type before anybody joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    /// This event's identity, the same on every mod that hears it and on both
    /// halves of each. Every question about the event takes it.
    ///
    /// A version 7 UUID, so it sorts by the time it was minted and is already
    /// a valid trace identifier. Putting it in a log line is what lets one
    /// happening be followed across two mods and two machines.
    pub id: EventId,
    /// The tick the event happened on.
    ///
    /// This is the world the event belongs to. A shot is judged against it
    /// rather than against the world the packet landed in, and two events
    /// delivered out of order are re-sorted by it. For a step it is the step's
    /// own number.
    pub raised_at: Tick,
    /// The tick this handler is running on.
    ///
    /// Equal to [`raised_at`](Context::raised_at) only when nothing queued in
    /// between, so `ctx.now - ctx.raised_at` is the event's age in ticks.
    pub now: Tick,
}

/// Who caused an event, as [`Context::cause`] answers it.
///
/// The set is closed, so a match over it needs no arm for something unknown.
/// It cannot grow either. The contract carries it as a variant, and a variant
/// that gains a case fails every already-built mod at load. It answers WHO,
/// which is why a node (a where) and a timer (a when) are absent. Behind a
/// timer stands the mod that armed it.
///
/// It belongs to no realm and both preludes carry it, because both halves are
/// handed a [`Context`] and both may ask.
///
/// # Where each case comes from
///
/// The step is [`Engine`](Cause::Engine), and so is a touch whose other side
/// is not a participant. An entity settling onto another mod's entity is the
/// physics, not somebody. An arrival, a departure, a press, a request from a
/// client half and a touch by a participant's body are
/// [`Player`](Cause::Player). A signal on a realm's own bus and one crossing to
/// the client halves are both [`Mod`](Cause::Mod).
///
/// # What is not built
///
/// Nothing in the host produces [`Operator`](Cause::Operator) or
/// [`Platform`](Cause::Platform). An arm for either is written against a future
/// rather than against a running session. Write it, because the set cannot grow
/// and matching a case costs nothing, and do not expect it to run.
///
/// ```
/// // The prelude mints ServerMod, Context, Target, PhysicsObject, ContactEdge,
/// // Vec3 and Cause.
/// use ironlark::server::prelude::*;
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn on_contact(
///         ctx: Context,
///         _target: Target,
///         _other: PhysicsObject,
///         _point: Vec3,
///         _edge: ContactEdge,
///     ) {
///         // Absent is not a failure: this realm cannot always name a cause.
///         match ctx.cause() {
///             Ok(Some(Cause::Player(who))) => log::info!("{who} walked into it"),
///             Ok(Some(Cause::Engine)) => log::info!("the physics did it"),
///             Ok(Some(_)) | Ok(None) => {}
///             Err(e) => log::warn!("the cause is not answerable here: {e}"),
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The engine itself rather than anybody: the step, and physics acting on
    /// its own.
    Engine,
    /// A participant, as the number that addresses them. A client half's
    /// request is this too. Everything from a client arrives bound to the
    /// participant who sent it, and everything from a client is assumed forged.
    Player(SessionId),
    /// Another mod's server half, as the id the host stamped on it. Compare it
    /// against a [`SourceId`](crate::SourceId) from
    /// [`resolve::source`](crate::server::resolve::source) to make policy of
    /// it. One integer compare is the whole of "only the gamemode commands me".
    Mod(SourceId),
    /// A human with server authority, acting out of band. Which one is an audit
    /// fact and is not carried here.
    Operator,
    /// The platform: an entitlement, a settlement, a ban.
    Platform,
}

/// What a handler may ask the host about the event it is inside.
impl Context {
    /// Answers who caused this event, as one of the [`Cause`] cases.
    ///
    /// A call rather than a field, and a cheap one. The host answers out of the
    /// event it is already holding, without touching the world, which is why it
    /// does not await. Ask it where a decision turns on the cause, a gate or an
    /// audit line, and skip it everywhere else.
    ///
    /// ```
    /// use ironlark::server::prelude::*;
    ///
    /// // `Cause`, `Context`, `Player` and `ServerMod` all arrive with the
    /// // prelude imported above.
    /// struct Gate;
    ///
    /// impl ServerMod for Gate {
    ///     async fn on_join(ctx: Context, player: Player) {
    ///         match ctx.cause() {
    ///             Ok(Some(Cause::Player(who))) => {
    ///                 log::info!("{player} arrived, and {who} is who did it")
    ///             }
    ///             Ok(Some(cause)) => log::info!("{player} arrived, caused by {cause:?}"),
    ///             Ok(None) => log::info!("{player} arrived, and this realm cannot say who by"),
    ///             Err(e) => log::warn!("{} is not the event in flight: {e}", ctx.id),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// `Ok(None)` is the answer where the realm cannot name a cause. A client
    /// half does not know the participant sitting at its own machine, so a key
    /// edge delivered there carries none.
    ///
    /// Refuses when this is not the event in flight, which is the whole of what
    /// it refuses for. Both halves may ask.
    pub fn cause(&self) -> Result<Option<Cause>> {
        backend::cause_of(self.id)
    }

    /// Answers the token of the instance this event is about.
    ///
    /// The world reuses an entity's slot once that entity is gone. This token
    /// carries the generation as well, so it never repeats. A handler that kept
    /// the token from an earlier event compares it against this one to tell the
    /// instance it meant from the stranger that took its place.
    ///
    /// A press on one of this mod's entities and a touch edge on one carry a
    /// token. Ask it inside those handlers and nowhere else. An event about no
    /// instance has nothing to answer with and refuses.
    ///
    /// Refuses for two different reasons with one kind. An event about no
    /// instance and a context kept past its handler both come back
    /// [`Refused`](crate::ErrorKind::Refused) in a session, and only the host's
    /// message says which happened. Treat the kind as "no token" and read the
    /// message in the log line.
    pub fn instance(&self) -> Result<u64> {
        backend::instance_of(self.id)
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::{Cause, EventId};
    use crate::bindings::server::ironlark::host::event as host;
    use crate::bindings::server::ironlark::host::types as wire;
    use crate::error::{Error, Result};
    use crate::ids::{SessionId, SourceId};

    fn wire_error(e: wire::Error) -> Error {
        Error::from_wire(e.code, e.message, e.data)
    }

    pub fn cause_of(event: EventId) -> Result<Option<Cause>> {
        match host::cause_of(event.to_wire()) {
            Ok(cause) => Ok(cause.map(|cause| match cause {
                wire::Cause::Engine => Cause::Engine,
                wire::Cause::Player(s) => Cause::Player(SessionId::new(s)),
                wire::Cause::Mod(s) => Cause::Mod(SourceId(s)),
                wire::Cause::Operator => Cause::Operator,
                wire::Cause::Platform => Cause::Platform,
            })),
            Err(e) => Err(wire_error(e)),
        }
    }

    pub fn instance_of(event: EventId) -> Result<u64> {
        host::instance_of(event.to_wire()).map_err(wire_error)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::{Cause, EventId};
    use crate::error::{Error, Result};

    fn no_event() -> Error {
        Error::from_wire(
            crate::error::code::STALE_ID,
            "the native double holds no event; only a session answers about one".into(),
            Vec::new(),
        )
    }

    pub fn cause_of(_event: EventId) -> Result<Option<Cause>> {
        Err(no_event())
    }

    pub fn instance_of(_event: EventId) -> Result<u64> {
        Err(no_event())
    }
}

/// The context a native call path carries: no event happened, so it names
/// none, and both questions refuse.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn detached() -> Context {
    Context {
        id: EventId::new(0),
        raised_at: Tick::new(0),
        now: Tick::new(0),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn an_age_needs_no_host() {
        let ctx = Context {
            id: EventId::new(1),
            raised_at: Tick::new(54),
            now: Tick::new(70),
        };
        assert_eq!(ctx.now - ctx.raised_at, 16);
    }

    #[test]
    fn the_native_double_refuses_rather_than_inventing_a_cause() {
        let ctx = detached();
        let refused = matches!(ctx.cause(), Err(ref e) if e.kind() == crate::ErrorKind::StaleId);
        assert!(
            refused,
            "a cause with no event behind it must not be made up"
        );
        assert!(ctx.instance().is_err());
    }
}
