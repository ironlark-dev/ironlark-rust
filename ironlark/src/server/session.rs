//! Who is connected, and what the server says about one of them.
//!
//! [`participants`] lists everyone connected right now, as a snapshot:
//! someone may arrive or leave before the caller acts on the answer.
//! [`name_of`] answers the one fact a mod almost always wants, and
//! [`get_all`] every fact the server resolved about a participant, as
//! named string values.
//!
//! A name is NOT an identity. Two participants may share one and it can
//! change, so address anyone by [`SessionId`](crate::SessionId) and show
//! the name only where a human reads it. Labelling a body over its owner's
//! head, and saying who opened a door, are what it is for.
//!
//! Each call crosses into the host and allocates, so read on the change
//! and keep the answer. A per-tick read of the whole table is the shape to
//! avoid.
//!
//! ```
//! // The prelude mints ServerMod, Context, Player and the session module.
//! use ironlark::server::prelude::*;
//!
//! struct Greeter;
//!
//! impl ServerMod for Greeter {
//!     async fn on_join(_ctx: Context, player: Player) {
//!         // The one fact, asked for directly. Read it on the change.
//!         if let Some(name) = session::name_of(player.session()).await {
//!             log::info!("{} is called {name}", player.session());
//!         }
//!     }
//! }
//! ```

pub use crate::shared::session::{get_all, name_of, participants};
