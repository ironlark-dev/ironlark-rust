//! Session decisions only the gamemode may take.
//!
//! A session names at most one mod to the gamemode role. That mod decides
//! the rules everyone else plays inside, so a decision belonging to the
//! whole session lives here rather than being open to every mod that
//! happens to be enabled. A mod that does not hold the role is refused with
//! [`Refused`](crate::ErrorKind::Refused), and so is every mod in a session
//! that named nobody.
//!
//! # Placement
//!
//! The host puts every arriving participant into a body of its own accord.
//! A gamemode that places bodies itself says so through
//! [`spawn`](spawn()), which opens [`SpawnSettings`] and applies them when
//! awaited, so the two never both act on one arrival. The host holds its
//! first placement until every server half's [`init`](super::ServerMod::init)
//! has run, so a change awaited there lands before any body does.
//!
//! ```
//! // The prelude mints ServerMod, Context, Player and the gamemode module.
//! use ironlark::server::prelude::*;
//!
//! struct Arena;
//!
//! impl ServerMod for Arena {
//!     async fn init() {
//!         // From here on this mod owes every arrival a body.
//!         if let Err(e) = gamemode::spawn().disable_auto().await {
//!             log::error!("arena does not hold the gamemode role: {e}");
//!         }
//!     }
//! }
//! ```

use crate::error::Result;
use std::future::{Future, IntoFuture};
use std::pin::Pin;

/// Opens the session's placement settings, which apply when awaited.
///
/// Placement is who puts an arriving participant into a body. The host does it
/// of its own accord, and a gamemode takes the job over through
/// [`SpawnSettings`], the type this answers with.
///
/// It reaches this crate through
/// [`server::gamemode`](crate::server::gamemode), which the server prelude
/// carries. Server realm, and narrower than it: a session has one gamemode, and
/// only that mod is admitted here. Who has a body and where it stands is the
/// session's rule rather than any content mod's.
///
/// A mod becomes a candidate for the role by declaring it in its manifest, and
/// the session picks the holder from among the candidates.
///
/// # Example
///
/// A gamemode taking placement over. The manifest line that makes the mod a
/// candidate is in a comment, and
/// [`export_server!`](macro@crate::export_server) on the last line is what
/// makes the type the half the host calls.
///
/// ```
/// // The prelude mints ServerMod, Context, Player, the gamemode module and
/// // export_server!.
/// use ironlark::server::prelude::*;
///
/// // A mod is a candidate for the role because mod.toml, beside the crate,
/// // carries:
/// //
/// //     [declares]
/// //     roles = ["gamemode"]
/// //
/// // The session then names the holder in the server's own config, under
/// // `[session] gamemode`, or picks the sole installed candidate.
/// struct Arena;
///
/// impl ServerMod for Arena {
///     // `init` runs once when this half loads, before any event. Settling it
///     // here is what lands the change before anybody is placed.
///     async fn init() {
///         if let Err(e) = gamemode::spawn().disable_auto().await {
///             log::error!("arena does not hold the gamemode role: {e}");
///         }
///     }
///
///     async fn on_join(_ctx: Context, _player: Player) {
///         // This mod now owes every arrival a body of its own.
///     }
/// }
///
/// ironlark::export_server!(Arena);
/// ```
///
/// # What refuses
///
/// [`Refused`](crate::ErrorKind::Refused) when the caller does not hold the
/// gamemode role, and the message names the mod that holds it instead or says
/// the session installed none. Nothing else refuses. Awaiting settings that
/// were asked nothing never crosses into the host, so it cannot refuse either.
///
/// # When it takes effect
///
/// The host holds its own placement until every server mod the session queued
/// has settled, loaded or failed, so a change awaited in `init` lands before
/// any participant is placed. A mod that never loaded runs no `init` and counts
/// as settled rather than being waited for, which keeps one broken mod from
/// stopping placement for the whole session. A change awaited later governs the
/// arrivals after it and moves nobody already in the session. Handing placement
/// back does not replay the joins that happened while the host was not placing,
/// because the host drains those as they arrive.
///
/// The host ends a session in which it is not placing and the mod holding the
/// gamemode role has dropped out, because nothing left in that session can give
/// anyone a body.
///
/// On the build machine there is no session. Every command is recorded instead,
/// and a test reads them back with
/// [`take_spawn_commands`](crate::testing::take_spawn_commands).
pub fn spawn() -> SpawnSettings {
    SpawnSettings { auto: None }
}

/// The session's placement settings, settled but not yet applied.
///
/// [`spawn`] opens one, each method settles one
/// decision and answers the settings back, and awaiting is what carries them to
/// the host. Awaiting settings that were asked nothing never crosses into the
/// host.
///
/// Only the session's gamemode may apply these, and
/// [`spawn`] states what a caller that is not
/// it gets back.
///
/// ```
/// // The prelude mints ServerMod, Context, Player and the gamemode module.
/// use ironlark::server::prelude::*;
///
/// struct Arena;
///
/// impl ServerMod for Arena {
///     async fn init() {
///         // This mod owes every arrival a body from here on.
///         if let Err(e) = gamemode::spawn().disable_auto().await {
///             log::error!("arena does not hold the gamemode role: {e}");
///         }
///     }
/// }
/// ```
#[must_use = "placement settings apply when awaited"]
#[derive(Clone, Copy)]
pub struct SpawnSettings {
    auto: Option<Auto>,
}

/// Which way the host's own placement was asked to go. Private: what an author
/// says is the verb they called, never a value they pass.
#[derive(Clone, Copy)]
enum Auto {
    Enable,
    Disable,
}

impl SpawnSettings {
    /// The host places every arrival in a body of its own accord.
    ///
    /// This is where a session starts, so a gamemode calls it only to hand
    /// placement back after taking it. Nobody who joined while the host was not
    /// placing is placed retroactively.
    pub fn enable_auto(mut self) -> Self {
        self.auto = Some(Auto::Enable);
        self
    }

    /// The host stops placing arrivals, and this gamemode owes every one of
    /// them a body.
    ///
    /// A gamemode that places bodies itself says this once, so the two never
    /// both act on one arrival, and then spawns a body per join with
    /// [`Entity::spawn`](crate::server::Entity::spawn), binds it with
    /// [`Entity::control`](crate::server::Entity::control) and addresses it with
    /// [`Entity::set_id`](crate::server::Entity::set_id), in that order.
    pub fn disable_auto(mut self) -> Self {
        self.auto = Some(Auto::Disable);
        self
    }
}

impl IntoFuture for SpawnSettings {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            match self.auto {
                Some(Auto::Enable) => backend::enable_auto().await,
                Some(Auto::Disable) => backend::disable_auto().await,
                None => Ok(()),
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use crate::error::{Error, Result};

    pub async fn enable_auto() -> Result<()> {
        crate::bindings::server::ironlark::host::spawn::enable_auto()
            .await
            .map_err(|e| Error::from_wire(e.code, e.message, e.data))
    }

    pub async fn disable_auto() -> Result<()> {
        crate::bindings::server::ironlark::host::spawn::disable_auto()
            .await
            .map_err(|e| Error::from_wire(e.code, e.message, e.data))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use crate::error::Result;
    use crate::testing::SpawnCommand;

    pub async fn enable_auto() -> Result<()> {
        crate::testing::record_spawn_command(SpawnCommand::EnableAuto);
        Ok(())
    }

    pub async fn disable_auto() -> Result<()> {
        crate::testing::record_spawn_command(SpawnCommand::DisableAuto);
        Ok(())
    }
}
