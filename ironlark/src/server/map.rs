//! Reading the map the session is running.
//!
//! A map is data rather than a mod. It carries geometry, an environment,
//! and the points it suggests bodies appear at. [`spawn_points`] answers
//! with those points, and a gamemode decides what to do with them. Handing
//! them out in turn across joins spreads players out, taking one as an
//! anchor measures from a fixed place, and ignoring them and computing a
//! placement of its own is equally allowed.
//!
//! The answer is never empty. A map that suggests no point of its own is
//! answered with the engine's single fallback point instead, so a gamemode
//! dividing the list across arrivals never divides by nothing.
//!
//! Read-only, because the map belongs to the session rather than to any mod.
//! Nothing here changes what is loaded, and nothing here refuses.
//!
//! ```
//! // The prelude mints ServerMod, Context, Player, Entity and the map
//! // module.
//! use ironlark::server::prelude::*;
//!
//! // Which point the next arrival gets, so participants spread out.
//! ironlark::state! {
//!     static NEXT: usize = 0;
//! }
//!
//! struct Arena;
//!
//! impl ServerMod for Arena {
//!     async fn on_join(_ctx: Context, player: Player) {
//!         let points = map::spawn_points().await;
//!         // An empty answer means the host could not answer at all, so
//!         // the length is taken before indexing.
//!         if points.is_empty() {
//!             log::error!("the map answered no points");
//!             return;
//!         }
//!         let at = points[NEXT.update(|n| {
//!             let i = *n;
//!             *n += 1;
//!             i
//!         }) % points.len()];
//!         // The host's own body archetype: no manifest declares it,
//!         // so there is no item and the name travels as a string.
//!         match Entity::spawn("character", at).await {
//!             Ok(body) => {
//!                 if let Err(e) = body.control(player.session()).await {
//!                     log::error!("the body is nobody's: {e}");
//!                 }
//!             }
//!             Err(e) => log::error!("no body for the arrival: {e}"),
//!         }
//!     }
//! }
//! ```

use crate::server::entity::SpawnPoint;

/// Answers the spawn points the loaded map declares, in the order it declares
/// them.
///
/// Each entry is a [`SpawnPoint`], a position and a
/// yaw in radians, which is what
/// [`Entity::spawn`](crate::server::Entity::spawn) takes. The map stores its
/// yaw in degrees and the host converts.
///
/// The list binds nobody. Two callers read it: the host, which picks one at
/// random per arrival for as long as it is placing arrivals, and a gamemode,
/// which reads it to place arrivals itself. A gamemode may hand the points out
/// in turn, take one as a fixed anchor, or ignore them for a placement it
/// computes. Where a participant stands is whichever of those two acted.
///
/// A gamemode that places arrivals itself stops the host first through
/// [`gamemode::spawn`](crate::server::gamemode::spawn), or both act on one
/// arrival. With the host stopped, nobody is placed until this mod spawns them.
///
/// The answer describes the map loaded now, and a session that changes map
/// answers differently after it.
///
/// It reaches this crate through [`server::map`](crate::server::map), which
/// the server prelude carries. Server realm, because the entity verbs it feeds
/// are. The map belongs to the session, so nothing on this surface changes what
/// is loaded or adds a point to it.
///
/// ```
/// // The prelude mints ServerMod, Context, Player, Entity and the map
/// // module.
/// use ironlark::server::prelude::*;
///
/// // Which point the next arrival gets, so participants spread out.
/// ironlark::state! {
///     static NEXT: usize = 0;
/// }
///
/// struct Arena;
///
/// impl ServerMod for Arena {
///     async fn on_join(_ctx: Context, player: Player) {
///         let points = map::spawn_points().await;
///         // The length is taken before indexing: an empty answer is the host
///         // failing to answer, and indexing it would cost the mod its
///         // instance.
///         if points.is_empty() {
///             log::error!("the map answered no points");
///             return;
///         }
///         let at = points[NEXT.update(|n| {
///             let i = *n;
///             *n += 1;
///             i
///         }) % points.len()];
///         // The host's own body archetype: no manifest declares it, so
///         // there is no item and the name travels as a string.
///         match Entity::spawn("character", at).await {
///             Ok(body) => {
///                 if let Err(e) = body.control(player.session()).await {
///                     log::error!("the body is nobody's: {e}");
///                 }
///             }
///             Err(e) => log::error!("no body for the arrival: {e}"),
///         }
///     }
/// }
/// ```
///
/// # Nothing refuses, and what an empty answer means
///
/// There is no [`Result`](crate::Result) here. A map that declares no point of
/// its own yields the engine's own fallback point instead of nothing, so in a
/// running session the answer holds at least one entry.
///
/// An empty answer is therefore not "this map declares none". It is the host
/// failing to answer at all, which it does by handing back an empty list and
/// writing its own log line. A caller that indexes straight into the result
/// panics on exactly that case, and a panicking guest costs the mod its whole
/// instance. Take the length into account before indexing, and return when the
/// lookup finds nothing.
///
/// # On the build machine
///
/// Off the session target this answers one point at the origin. A gamemode
/// test written against it has exactly one spawn to hand out, so logic that
/// rotates through several never advances and passes for a reason that has
/// nothing to do with what it is proving. Give placement logic its own list to
/// work from when the test is about the distribution rather than about the
/// call.
pub async fn spawn_points() -> Vec<SpawnPoint> {
    backend::spawn_points().await
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use crate::server::entity::SpawnPoint;

    pub async fn spawn_points() -> Vec<SpawnPoint> {
        crate::bindings::server::ironlark::host::map_api::spawn_points()
            .await
            .into_iter()
            .map(|s| SpawnPoint {
                position: crate::Vec3 {
                    x: s.position.x,
                    y: s.position.y,
                    z: s.position.z,
                },
                yaw: s.yaw,
            })
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use crate::server::entity::SpawnPoint;

    pub async fn spawn_points() -> Vec<SpawnPoint> {
        vec![SpawnPoint {
            position: crate::Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
        }]
    }
}
