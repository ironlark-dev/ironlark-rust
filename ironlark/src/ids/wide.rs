//! The 128-bit identity ids: one macro, three names, and their wire crossing.

use core::fmt;

/// A 128-bit id, printed as a UUID because that is what it is. One shape
/// serves the account, the persona and the event, and none of them lets a
/// caller reach a half.
macro_rules! wide_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u128);

        impl $name {
            /// The id as the platform minted it, for a test or a fixture. A
            /// mod receives these rather than inventing them.
            pub const fn new(value: u128) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_uuid(self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self)
            }
        }
    };
}

/// The canonical 8-4-4-4-12 form, so an id pasted into a log, a ticket or a
/// trace store is the same text everywhere.
fn write_uuid(value: u128, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (value >> 96) as u32,
        (value >> 80) as u16,
        (value >> 64) as u16,
        (value >> 48) as u16,
        (value & 0xffff_ffff_ffff) as u64
    )
}

wide_id!(
    /// The platform account behind a participant: one UUID, minted centrally
    /// and meaning the same thing on every server the platform serves.
    ///
    /// [`Player::user`](crate::server::Player::user) hands it over, and it
    /// costs nothing. The contract calls the account immutable for the
    /// session, so the handle was stamped with it and the read cannot fail. It
    /// answers `Option`, and `None` is not a failure. The participant role does
    /// not require an account behind it, and a mod that assumes one breaks on
    /// the first participant that has none.
    ///
    /// It is plain data. It is `Copy`, and unlike the
    /// [`Player`](crate::server::Player) that produced it, it outlives the
    /// event, so a [`State`](crate::State) cell may keep it for as long as the
    /// mod wants it. It prints as the canonical UUID text, which is what a log
    /// line carries and what a payload field carries: a schema names it as a
    /// `string`, and the text is the same everywhere it is written.
    ///
    /// It is not authority. Nothing in this crate takes a `UserId` and acts on
    /// it, and the right to be obeyed arrives only as a host-stamped
    /// [`Player`](crate::server::Player), so an account id that arrived inside
    /// a payload proves nothing about who sent that payload. It is not a save
    /// key either. What a server remembers about someone is keyed on
    /// [`ProfileId`].
    ///
    /// ```
    /// // The prelude mints ServerMod, Context, Player and UserId.
    /// use ironlark::server::prelude::*;
    ///
    /// // The id outlives the handle it came from, so a cell may keep it.
    /// ironlark::state! {
    ///     static ACCOUNTS: Vec<UserId> = Vec::new();
    /// }
    ///
    /// struct Doorman;
    ///
    /// impl ServerMod for Doorman {
    ///     async fn on_join(_ctx: Context, player: Player) {
    ///         // None is an ordinary answer: a participant need not have an
    ///         // account behind them.
    ///         match player.user() {
    ///             Some(user) => ACCOUNTS.update(|seen| seen.push(user)),
    ///             None => log::info!("{player} arrived with no account"),
    ///         }
    ///     }
    /// }
    /// ```
    UserId
);

wide_id!(
    /// The persona a participant is wearing: one server's own key for what it
    /// remembers about them, opaque and unique within that server alone.
    ///
    /// [`Player::profile`](crate::server::Player::profile) answers it, and
    /// that is a live read rather than a stamped field, so it awaits and it
    /// can refuse. `Ok(None)` says the participant wears no persona, and a
    /// handler whose answer needs one says so with
    /// [`Refusal::no_profile`](crate::Refusal::no_profile) rather than
    /// guessing. What refuses is a handle held past the event it was lent for.
    /// The host never declines the question itself. Do not hold one across an
    /// await. The read is a call precisely because the persona is not promised
    /// to stay the same for the session.
    ///
    /// This is the id a save is keyed on, and the only id a mod meets that
    /// still means the same thing next session. A [`SessionId`](crate::SessionId) is a fresh
    /// number on every join, and a [`UserId`] belongs to the platform rather
    /// than to this server's records. Its scope is one server. The platform
    /// neither stores it nor links it to an account, so two servers' profile
    /// ids say nothing about each other and comparing them across servers is
    /// meaningless.
    ///
    /// # What is not built
    ///
    /// Personas. The host mints none, and
    /// [`Player::profile`](crate::server::Player::profile) answers `None` for
    /// every participant, so a handler that requires one refuses every caller.
    /// The type and the `Option` exist so that a mod written against them keys
    /// its saved state correctly the day the host starts answering.
    ///
    /// ```
    /// // The prelude mints ServerMod, Context, Player and Refusal.
    /// use ironlark::server::prelude::*;
    ///
    /// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
    /// // A request handler keyed on the persona. It refuses rather than
    /// // guessing when the participant wears none. `protocol::Latch` is the
    /// // answer the mod's own schema declares for this request.
    /// async fn my_latch(_ctx: Context, caller: Player) -> Result<protocol::Latch, Refusal> {
    ///     let profile = match caller.profile().await {
    ///         Ok(Some(profile)) => profile,
    ///         Ok(None) => return Err(Refusal::no_profile()),
    ///         Err(e) => return Err(Refusal::Unknown { message: e.to_string() }),
    ///     };
    ///     log::info!("the door remembers {profile}");
    ///     Ok(protocol::Latch { open: false })
    /// }
    /// ```
    ProfileId
);

wide_id!(
    /// One raised event, for the whole life of that event.
    ///
    /// Minted where the event is born and carried from there, never re-minted
    /// per mod. One join, one key edge, one raised signal is one id however
    /// many mods hear it, so two mods that both react to the same fact are
    /// looking at the same event and their log lines join up. It arrives on
    /// [`Context::id`](crate::Context::id), and every question about the event
    /// takes it.
    ///
    /// It is a version 7 UUID, so it sorts by time and is already a valid
    /// trace identifier. A log line carrying one needs no translation to be
    /// found again.
    ///
    /// It does not cross the boundary between the realms. A signal crossing to
    /// a client half is raised there as its own event with its own id, so a
    /// mod that wants its two halves joined up puts something of its own in the
    /// payload rather than expecting the ids to match.
    ///
    /// ```
    /// // The prelude mints ServerMod, Context and Player.
    /// use ironlark::server::prelude::*;
    ///
    /// struct Doorman;
    ///
    /// impl ServerMod for Doorman {
    ///     async fn on_join(ctx: Context, player: Player) {
    ///         // The same id every other mod hearing this join sees, so two
    ///         // mods' log lines join up on it.
    ///         log::info!("{} joined, event {}", player, ctx.id);
    ///     }
    /// }
    /// ```
    EventId
);

#[cfg(target_arch = "wasm32")]
mod wire {
    use super::{EventId, ProfileId, UserId};
    use crate::bindings::server::ironlark::host::types as host;

    fn joined(id: host::Uuid) -> u128 {
        ((id.high as u128) << 64) | id.low as u128
    }

    fn split(value: u128) -> host::Uuid {
        host::Uuid {
            high: (value >> 64) as u64,
            low: value as u64,
        }
    }

    impl UserId {
        pub(crate) fn from_wire(id: host::Uuid) -> Self {
            Self::new(joined(id))
        }
    }

    impl ProfileId {
        pub(crate) fn from_wire(id: host::Uuid) -> Self {
            Self::new(joined(id))
        }
    }

    impl EventId {
        pub(crate) fn from_wire(id: host::Uuid) -> Self {
            Self::new(joined(id))
        }

        pub(crate) fn to_wire(self) -> host::Uuid {
            split(self.0)
        }
    }
}
