//! Who is connected, and what the server says about one of them.
//!
//! The same table the server realm's own
//! [`session`](crate::server::session) reads. The server states a
//! participant's facts as it admits them and restates them on every
//! change, and each machine's copy follows those announcements.
//! [`participants`] lists who is here, [`name_of`] answers the one fact a
//! half that labels a body wants, and [`get_all`] answers every fact.
//!
//! A name is not an identity. Two participants may carry the same one and
//! either can change mid-session, so address anyone by their
//! [`SessionId`](crate::SessionId) and show the name only where a human
//! reads it.
//!
//! Each call crosses into the host and allocates, so read on the change
//! and keep the answer. A read of the whole table once per frame is the
//! shape to avoid.
//!
//! ```
//! // The prelude mints Context, the session module and the ui module.
//! use ironlark::client::prelude::*;
//! # use ironlark::SessionId;
//!
//! // Read on the change, and paint what was read.
//! async fn label(who: SessionId) {
//!     if let Some(name) = session::name_of(who).await {
//!         ui::set_overlay_text(&name);
//!     }
//! }
//! ```

pub use crate::shared::session::{get_all, name_of, participants};
