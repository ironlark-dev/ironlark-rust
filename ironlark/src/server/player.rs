//! The participant a hook is handed, and the ids it exposes.

use crate::ids::{SessionId, UserId};
use core::fmt;

/// The participant an event is about, lent to the handler for that event.
///
/// A hook on [`ServerMod`](crate::server::ServerMod) that concerns somebody
/// receives one: an arrival, a departure, a press on this mod's entity, a touch
/// by somebody's body. So does a request handler registered with
/// [`respond`](crate::protocol::RequestSpec::respond), because a request from a
/// client half arrives bound to whoever sent it. It is a lease rather than a
/// value. The host mints it on the way into the handler and reclaims it on the
/// way out. A mod cannot fabricate one.
///
/// A participant is not an entity and carries no id of its own. What it carries
/// is the ids that address them and the questions the host answers about them,
/// all named below. It belongs to the server realm, so its door is
/// [`server`](crate::server), and every example here opens with that realm's
/// prelude.
///
/// # Keep the ids, not the handle
///
/// Everything the handle exposes is yours to keep for as long as you like. The
/// handle itself is not. The host reclaims the lent number when the handler
/// returns, so a stored copy asked afterwards refuses as
/// [`StaleId`](crate::ErrorKind::StaleId) instead of addressing whoever came
/// next.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `SessionId`, `Context`, `Player` and `ServerMod` all arrive with the
/// // prelude imported above. `ironlark::state!` declares what this mod
/// // remembers between events, spelled in full so its origin stays visible.
/// ironlark::state! {
///     static ROSTER: Vec<SessionId> = Vec::new();
/// }
///
/// struct Doorman;
///
/// // `ServerMod` is the server half's trait. Every hook has a default, so a
/// // mod writes only the ones it wants.
/// impl ServerMod for Doorman {
///     async fn on_join(_ctx: Context, player: Player) {
///         // The id outlives the handle. A `Player` in that cell would not.
///         ROSTER.update(|seen| seen.push(player.session()));
///     }
/// }
/// ```
///
/// # A build machine does not enforce the event rule
///
/// The event stamp exists only in the component this crate compiles to for the
/// game. Built for the machine you develop on, a handle carries no stamp and
/// answers whenever it is asked, and
/// [`FakePlayer`](crate::testing::FakePlayer) is what stands in for the host's
/// own. So a mod that hoards a handle and reads it three events later passes
/// `cargo test` and refuses in a session. Reading catches this. No test on
/// this side of the boundary does.
///
/// # Which id to save under
///
/// [`user`](Player::user) is the platform account, absent where no account
/// stands behind the participant. [`profile`](Player::profile) is the persona,
/// and it is what durable state is keyed on. The host mints no persona, so that
/// call answers `None` and its own page says what a handler does about it.
/// [`session`](Player::session) is this participation, never reused inside the
/// session and never a save key.
///
/// The account and the participation are stamped into the handle, so both cost
/// nothing and cannot fail. The persona is not promised to hold still for the
/// session, so reading it is a call that awaits and must not be held across
/// another await.
#[derive(Clone, Copy)]
pub struct Player {
    #[cfg(target_arch = "wasm32")]
    raw: u32,
    #[cfg(target_arch = "wasm32")]
    epoch: u32,
    /// Stamped rather than asked for, both of them: the contract calls the
    /// session id and the account session-immutable, so neither can go stale
    /// while the handle is valid. `profile` is a live read and stays a call.
    #[cfg(target_arch = "wasm32")]
    session: SessionId,
    #[cfg(target_arch = "wasm32")]
    user: Option<UserId>,
    #[cfg(not(target_arch = "wasm32"))]
    fake: crate::testing::FakePlayer,
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::Player;
    use crate::bindings::server::ironlark::host::player as host;
    use crate::error::{Error, Result};
    use crate::ids::{ProfileId, SessionId, UserId};

    impl Player {
        /// The handle stays the host's. The wrapper keeps its number and the
        /// event that delivered it, and re-materializes a non-dropping view
        /// per call.
        pub(crate) fn from_borrow(p: &host::Player) -> Self {
            Self {
                raw: p.handle(),
                epoch: crate::host::scope::epoch(),
                session: SessionId::new(p.session()),
                user: p.user().map(UserId::from_wire),
            }
        }

        /// A view of the host's handle, refused once its event is over: the
        /// host reclaims the handle and the number addresses whoever comes
        /// next.
        fn view(&self) -> Result<core::mem::ManuallyDrop<host::Player>> {
            if self.epoch != crate::host::scope::epoch() {
                return Err(Error::from_wire(
                    crate::error::code::STALE_ID,
                    "this player belongs to an event that is over; keep the ids, not the handle"
                        .into(),
                    Vec::new(),
                ));
            }
            Ok(core::mem::ManuallyDrop::new(unsafe {
                host::Player::from_handle(self.raw)
            }))
        }

        /// Free: taken when the handle was stamped, and the contract calls it
        /// session-immutable.
        pub(crate) fn user_at_target(&self) -> Option<UserId> {
            self.user
        }

        /// A live read rather than a stamped field, because the persona a
        /// participant wears is not promised to hold for the session.
        pub(crate) async fn profile_at_target(&self) -> Result<Option<ProfileId>> {
            let view = self.view()?;
            Ok(view.profile().await.map(ProfileId::from_wire))
        }

        /// Free: no host call and no way to fail, because the id was taken
        /// when the handle was stamped.
        pub(crate) fn session_at_target(&self) -> SessionId {
            self.session
        }

        /// The controlled body, reachable while handling this participant's
        /// event.
        pub(crate) async fn body_at_target(&self) -> Result<crate::server::entity::Entity> {
            let view = self.view()?;
            match view.body().await {
                Ok(handle) => Ok(crate::server::entity::Entity::from_owned(handle)),
                Err(e) => Err(Error::from_wire(e.code, e.message, e.data)),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::Player;
    use crate::error::{Error, Result};
    use crate::ids::{ProfileId, SessionId, UserId};

    impl Player {
        pub(crate) fn from_fake(fake: crate::testing::FakePlayer) -> Self {
            Self { fake }
        }

        /// The account the double was built with.
        pub(crate) fn user_at_target(&self) -> Option<UserId> {
            self.fake.user
        }

        /// The persona the double was built with.
        pub(crate) async fn profile_at_target(&self) -> Result<Option<ProfileId>> {
            Ok(self.fake.profile)
        }

        /// The connection id the double was built with.
        pub(crate) fn session_at_target(&self) -> SessionId {
            self.fake.session
        }

        /// A body needs a session, so the double refuses.
        pub(crate) async fn body_at_target(&self) -> Result<crate::server::entity::Entity> {
            Err(Error::from_wire(
                0,
                "the native test double controls no body; a session does".into(),
                Vec::new(),
            ))
        }
    }
}

impl fmt::Display for Player {
    /// Prints the stamped participation number. No host call and no read of the
    /// session table, so it is free to put in a log line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "player#{}", self.session())
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

use crate::error::Result;
use crate::ids::ProfileId;

/// What a handler may ask about the participant it was handed.
impl Player {
    /// The platform account behind this participant, or `None` where none
    /// stands behind them.
    ///
    /// Free. It was taken when the handle was stamped, and the contract calls
    /// an account immutable for the session, so this is a field read that
    /// cannot fail and cannot go stale while the handle is valid.
    ///
    /// `None` is not an error. Occupying the player role does not require an
    /// account, and a mod that assumes one breaks on the first participant
    /// admitted without it. Key on
    /// [`profile`](Player::profile) for what must survive the session and on
    /// [`session`](Player::session) for what must not.
    pub fn user(&self) -> Option<UserId> {
        self.user_at_target()
    }

    /// The persona this participant is wearing.
    ///
    /// This is the id to key a save on. It belongs to the server rather than to
    /// the platform, and it is the only id here that means the same thing next
    /// session.
    ///
    /// A call rather than a stamped field, and that is the contract's choice
    /// rather than a detail of this crate. One account holds several personas
    /// and wears one at a time, and the worn one may change while the session
    /// runs. Take a fresh answer per decision. Do not hold one across another
    /// await.
    ///
    /// Refuses only on a handle whose event is over. The host itself never
    /// declines the question.
    ///
    /// # What is not built
    ///
    /// Personas are not built and the host mints none, so the answer is `None`.
    /// A handler that needs one refuses its caller, and
    /// [`Refusal::no_profile`](crate::Refusal::no_profile) is the way to say
    /// so. The option is the contract's, so write the `Some` branch now and a
    /// mod keys correctly the day the host starts answering.
    pub async fn profile(&self) -> Result<Option<ProfileId>> {
        self.profile_at_target().await
    }

    /// This participation, as the number that addresses it.
    ///
    /// Free, like [`user`](Player::user), and present for every participant
    /// including one with no account. Every arrival mints a fresh number and
    /// the session never reuses one, so it addresses exactly one participant
    /// for as long as the session runs. It is what
    /// [`signal_to`](crate::server::signal_to) narrows an announcement to, and
    /// what a mod holds while a departed participant is being forgotten.
    ///
    /// It is not a save key. Nothing about it outlives the session, and the
    /// same person rejoining is a new participation with a new number.
    pub fn session(&self) -> SessionId {
        self.session_at_target()
    }

    /// The entity this participant is controlling.
    ///
    /// A live read, because possession moves. What answers is whatever they
    /// control at the moment of the call, not whatever they controlled when the
    /// event was raised. This is the handle-shaped way to ask. The same
    /// question by [`SessionId`](crate::SessionId) is
    /// [`body_of`](crate::server::body_of), which is what a mod uses on an id
    /// it kept from an earlier event.
    ///
    /// A body is what carries the host's character marker, and
    /// [`Entity::control`](crate::server::Entity::control) refuses an entity
    /// that does not, so this answers a body or refuses.
    ///
    /// The handle that comes back is owned and keeps addressing the body after
    /// the handler returns. What ends with the handler is the standing to act
    /// through it. A press or a touch by this participant makes them the
    /// subject while that handler runs, and being the subject is what admits an
    /// ordinary component write on their body. An arrival, a departure and a
    /// step make nobody the subject, and neither does an event the host
    /// delivered far behind the tick it was raised on. A row this mod declared
    /// under `body-rows` in its manifest is the exception, written on any
    /// possessed body whatever the event.
    /// [`Entity::set_component`](crate::server::Entity::set_component) carries
    /// the whole reach rule.
    ///
    /// Refuses once the event that handed this handle over is finished, and
    /// that one is [`StaleId`](crate::ErrorKind::StaleId). It also refuses when
    /// they control nothing, which a participant who has already left reads as
    /// too, because leaving takes their body with it. That one arrives as
    /// [`Other`](crate::ErrorKind::Other) carrying the host's own message, so
    /// put the message in the log line rather than branching on the kind.
    pub async fn body(&self) -> Result<crate::server::entity::Entity> {
        self.body_at_target().await
    }
}
