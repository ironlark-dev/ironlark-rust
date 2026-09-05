//! Identity and compact-id newtypes. The wire carries plain numbers, a wide id
//! as two 64-bit halves and a compact id as one `u32`, and the
//! non-interchange between kinds lives here, in the types.
//!
//! Nothing here hands out its insides. Every id a mod names something with
//! prints itself, so no mod writes `.0` to show a number, and no mod can pass a
//! tick where a session id belongs.

use core::fmt;
use core::ops::Sub;
mod compact;
mod wide;

pub use compact::{ComponentId, FieldId, HookId, RequestId, SignalId, SoundId, SourceId};
pub use wide::{EventId, ProfileId, UserId};

/// A participant's number for as long as they stay connected, and the id a mod
/// addresses a participant by.
///
/// It arrives free from [`Player::session`](crate::server::Player::session)
/// and inside [`Cause::Player`](crate::Cause::Player), and it goes back in
/// wherever a verb names someone. [`signal_to`](crate::server::signal_to)
/// narrows an announcement to one participant's machine,
/// [`body_of`](crate::server::body_of) and
/// [`Entity::control`](crate::server::Entity::control) reach the body they are
/// in, and [`session::name_of`](crate::server::session::name_of) says what they
/// are called. [`session::participants`](crate::server::session::participants)
/// answers a list of these.
///
/// Keep this, not the handle. A [`Player`](crate::server::Player) is lent for
/// one event and reclaimed when the handler returns, so a mod that wants to
/// remember who was here keeps their session ids in a [`State`](crate::State)
/// cell and looks nothing up until it has to. Every participant has one,
/// including one with no [`UserId`] behind them.
///
/// It is not shaped like the ids beside it. Sixty-four bits rather than a UUID,
/// and it prints as a plain number. Naming a participant to the other half, in a
/// table of scores or a feed of what happened, is done with this: a schema names
/// a participant as a `uint64`, `u64::from` on this type fills the field, and
/// [`SessionId::new`] reads one back at the other end.
///
/// Both realms address a participant by this number. The client realm's own
/// [`session`](crate::client::session) reads the same table with it.
///
/// It is not a save key. The number is the session's own count of joins, so the
/// same person reconnecting is a different participant with a different number,
/// and nothing about it survives the session that issued it. The next session
/// counts from the start again. What outlives a session is [`ProfileId`].
///
/// The two hooks that bracket a participation are
/// [`ServerMod`](crate::server::ServerMod)'s, and the
/// [`LeaveReason`](crate::server::LeaveReason) the second one carries is that
/// trait's own subject.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `SessionId`, `ServerMod`, `Context`, `Player` and `LeaveReason` all
/// // arrive with the server prelude imported above. The hooks are
/// // `ServerMod`'s, and a mod carries them because its mod.toml names them.
/// ironlark::state! {
///     static PRESENT: Vec<SessionId> = Vec::new();
/// }
///
/// struct Door;
///
/// impl ServerMod for Door {
///     async fn on_join(_ctx: Context, player: Player) {
///         let who = player.session();
///         PRESENT.update(|present| present.push(who));
///     }
///
///     async fn on_leave(_ctx: Context, player: Player, _reason: LeaveReason) {
///         let who = player.session();
///         PRESENT.update(|present| present.retain(|held| *held != who));
///     }
/// }
/// ```
///
/// # Holding one past the participant
///
/// Within the running session the number is never handed out twice, which is
/// the whole guarantee. A held id can never be answered for whoever joined
/// after. It is not, however, forgotten when they leave. The session keeps old
/// numbers resolvable so an event still in the queue can be delivered, so what
/// a verb does with the id of a departed participant is not one answer, and a
/// mod that keeps ids has to expect all three.
///
/// [`body_of`](crate::server::body_of) refuses, because the body is gone rather
/// than because the number is.
/// [`session::name_of`](crate::server::session::name_of) answers `None`, because
/// its signature has no error to answer with.
/// [`signal_to`](crate::server::signal_to) answers `Ok` and the bytes are then
/// dropped by the host, which logs one line about it. That `Ok` says the host
/// took the raise, never that anybody received it, and it says the same for a
/// participant who is still here.
///
/// So dropping a departed participant from a mod's own bookkeeping, as the hook
/// above does, is not tidiness. It is the only thing standing between the third
/// case and a mod raising into nothing every tick.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

impl SessionId {
    /// The id as the session minted it, for a test or a fixture.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) const fn to_wire(self) -> u64 {
        self.0
    }
}

/// The number itself, for the one place a mod hands it over: a field of a
/// payload. A schema names a participant as a `uint64`, and
/// [`SessionId::new`] reads one back at the other end.
impl From<SessionId> for u64 {
    fn from(id: SessionId) -> Self {
        id.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionId({})", self.0)
    }
}

/// One fixed step of the simulation, counted from the session's start.
///
/// Two of them arrive with every event on [`Context`](crate::Context), the tick
/// it was raised on and the tick the handler is running on. Subtracting the
/// first from the second is the event's age in ticks, which is the one
/// arithmetic a mod does with these.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `Context` carries both ticks and arrives with the server prelude
/// // imported above. Subtracting two of them answers a plain `u64`.
/// fn queued_for(ctx: Context) -> u64 {
///     ctx.now - ctx.raised_at
/// }
/// ```
///
/// It names nothing. A wide id identifies a thing and a compact id numbers a
/// declared name. A tick is a position on the session's own clock, so it
/// compares and orders and hands out no number to key anything on. Compare two
/// that arrived together, on one event. A tick is its own machine's count, so
/// one taken from a client half and one taken from the server half are two
/// clocks rather than one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(u64);

impl Tick {
    /// A tick number, for a test or a fixture. The host supplies the real ones.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl Sub for Tick {
    type Output = u64;

    /// How many ticks lie between two of them, as a plain `u64`. Never
    /// negative. An earlier tick subtracted from a later one is the distance,
    /// and the other way round is zero rather than a wrap.
    fn sub(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tick({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_id_prints_as_a_uuid() {
        let id = EventId::new(0x0198_e0c1_7a2b_7c3d_9e4f_a0b1_c2d3_e4f5);
        assert_eq!(id.to_string(), "0198e0c1-7a2b-7c3d-9e4f-a0b1c2d3e4f5");
        assert_eq!(
            UserId::new(0).to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn an_age_is_the_distance_between_two_ticks() {
        assert_eq!(Tick::new(70) - Tick::new(54), 16);
        assert_eq!(
            Tick::new(54) - Tick::new(70),
            0,
            "a handler must never see an event raised in the future"
        );
    }

    #[test]
    fn a_session_id_prints_its_number() {
        assert_eq!(SessionId::new(7).to_string(), "7");
        assert_eq!(Tick::new(9).to_string(), "9");
    }
}
