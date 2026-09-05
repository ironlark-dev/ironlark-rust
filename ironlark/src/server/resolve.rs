//! The server realm's name-to-id crossing: whatever a server mod has to
//! name as text, it turns into a session id here before using it.
//!
//! Names are strings in a manifest and numbers on the wire. An id is
//! numbered per session and dies with it, so it is never a save key and
//! never rides inside a payload. Resolve once, in
//! [`init`](super::ServerMod::init) or on first use, and keep the answer in
//! a [`State`](crate::State) cell. A resolve is a call into the host, while
//! an id is `Copy` and comparing two costs nothing, so a resolve inside a
//! per-tick loop pays the crossing every tick for nothing.
//!
//! # The whole shape, once
//!
//! A door mod dimming a world row no typed helper covers. It resolves the
//! row and the path inside it while it loads, keeps both, and writes with
//! ids from then on.
//!
//! ```
//! // The prelude mints ServerMod, Context, Player, Target, Entity, Field,
//! // Value, Vec3, ComponentId, FieldId and the resolve module.
//! use ironlark::server::prelude::*;
//!
//! // Resolved once, kept for the session. Both ids are Copy.
//! ironlark::state! {
//!     static LIGHT: Option<(ComponentId, FieldId)> = None;
//! }
//!
//! struct Doorman;
//!
//! impl ServerMod for Doorman {
//!     async fn init() {
//!         // `material` is a row the host publishes; `emissive` is a path
//!         // inside it. A path is numbered under its own component, so the
//!         // two are resolved and kept together.
//!         let row = match resolve::component("material") {
//!             Ok(row) => row,
//!             Err(e) => {
//!                 log::error!("this session publishes no material row: {e}");
//!                 return;
//!             }
//!         };
//!         match resolve::field(row, "emissive") {
//!             Ok(path) => LIGHT.set(Some((row, path))),
//!             Err(e) => log::error!("no emissive path under material: {e}"),
//!         }
//!     }
//!
//!     async fn on_interact(
//!         _ctx: Context,
//!         _player: Player,
//!         target: Target,
//!         _hit_point: Vec3,
//!         _distance: f32,
//!     ) {
//!         // No resolve on this path: the ids were settled in `init`.
//!         let Some((row, path)) = LIGHT.get() else {
//!             return;
//!         };
//!         // `Target::entity` is an option the contract's shape gives it.
//!         let Some(door) = target.entity else {
//!             return;
//!         };
//!         let dim = Field { field: path, value: Value::Number(0.1) };
//!         if let Err(e) = door.set_component(row, [dim]).await {
//!             log::warn!("the door did not dim: {e}");
//!         }
//!     }
//! }
//! ```
//!
//! # What comes through here, and what does not
//!
//! Most names never come through here. A signal and a request are reached
//! as the payload types the mod's own schema generated, and every verb on
//! one — the raises, [`request`](crate::client::request),
//! [`observe`](crate::protocol::SignalSpec::observe) and
//! [`respond`](crate::protocol::RequestSpec::respond) — resolves the
//! declared name itself.
//!
//! [`component`] and [`field`] name a world row and a path inside it. The
//! typed helpers on [`Entity`](super::Entity) resolve the rows they cover
//! internally, so resolve by hand for a row those helpers do not cover.
//!
//! [`source`] names an enabled mod, and its answer is compared rather than
//! passed to a verb. It is how an
//! [`observe`](crate::protocol::SignalSpec::observe) handler decides whose
//! commands it obeys.
//!
//! [`sound`] is not a leftover. A mod's own sounds are minted from its
//! manifest, and [`audio::play`](super::audio::play) takes the minted item
//! directly. What has no item to be is another mod's sound, which this
//! manifest does not list, and a name settled at run time. That is what
//! this call is for.

pub use crate::shared::resolve::{component, field, sound, source};
