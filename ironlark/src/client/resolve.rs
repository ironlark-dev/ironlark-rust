//! The client realm's name-to-id crossing: whatever a client mod has to
//! name as text, it turns into a session id here before using it.
//!
//! Names are strings in a manifest and numbers on the wire.
//!
//! A client half rarely comes here. A sound of its own is reached as the
//! item its manifest name mints, which resolves itself on the first play.
//! A signal and a request are reached as the payload types the mod's own
//! schema generated, and every verb on one resolves the declared name
//! itself. A mixing lane is a [`SoundBus`](crate::SoundBus) case,
//! which never needed resolving at all.
//!
//! What is left is a name settled while the session runs rather than written
//! in the source, another mod's sound among them. A manifest cannot know
//! what a mod it never heard of ships, and a name computed at run time has
//! no item to be. [`sound`] takes that name and answers the
//! [`SoundId`](crate::SoundId) the play takes. Ask once, in `init` or on
//! first use, and keep the id in a [`State`](crate::State) cell. A resolve
//! is a call into the host, while the id it answers is `Copy` and good for
//! the rest of the session.
//!
//! It refuses by name. A name no enabled mod declares is
//! [`UnresolvedName`](crate::ErrorKind::UnresolvedName) here, at the resolve,
//! rather than a sound that plays as silence later.
//!
//! # The whole shape, once
//!
//! A client half playing a sound another mod ships, on this machine alone.
//!
//! ```
//! // The prelude mints ClientMod, SoundId, SoundBus, the audio and
//! // resolve modules.
//! use ironlark::client::prelude::*;
//!
//! // Resolved on first use, kept for the session.
//! ironlark::state! {
//!     static ALARM: Option<SoundId> = None;
//! }
//!
//! // The name is another mod's, spelled in full: its author, its mod, the
//! // kind, and the name that mod declared under `sounds` in its own
//! // mod.toml.
//! async fn warn_this_machine() {
//!     let id = match ALARM.get() {
//!         Some(id) => id,
//!         None => match resolve::sound("author:mod/sound/alarm") {
//!             Ok(id) => {
//!                 ALARM.set(Some(id));
//!                 id
//!             }
//!             Err(e) => {
//!                 log::error!("this session carries no such sound: {e}");
//!                 return;
//!             }
//!         },
//!     };
//!     if let Err(e) = audio::play(id, SoundBus::Interface).await {
//!         log::warn!("the alarm was not heard: {e}");
//!     }
//! }
//! ```

pub use crate::shared::resolve::sound;
