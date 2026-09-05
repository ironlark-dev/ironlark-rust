//! What a mod declares, as the traits its schema reaches Rust through.
//!
//! A mod states what it announces and what it answers in one file beside the
//! manifest, `protocol.proto`. That file is a protobuf schema: it defines the
//! payload types, and it declares them through options and a `service` block
//! written on those same types. [`protocol!`](macro@crate::protocol) compiles
//! it while the crate compiles, generates the payload types, and implements one
//! of the traits here on each declared type. A declaration is never written
//! twice, and the value at a call site IS the declaration.
//!
//! Two acts, two traits. [`SignalSpec`] is an announcement: whoever subscribed
//! to its name hears it, the raiser included, and nobody answers.
//! [`RequestSpec`] is the addressed act, one awaited answer from the half whose
//! `service` block declares it. [`Audience`] and [`Transit`] are what a signal
//! declaration states, and [`SoundSpec`] is the third trait here, declaring no
//! traffic at all because a sound is a file the manifest names.
//!
//! # One file, both halves
//!
//! A door mod's `protocol.proto` sits beside `mod.toml`, one directory above
//! the two crates that are its halves, and both compile against it.
//!
//! ```proto
//! syntax = "proto3";
//! package mods.acme_door;
//! import "ironlark/options.proto";
//!
//! // A signal: one option line says who hears it. CLIENTS crosses the network
//! // to every client realm; SERVER_MODS is the zero value and stays on the
//! // server realm's bus.
//! message Latch {
//!   option (ironlark.signal) = CLIENTS;
//!   bool open = 1;
//! }
//!
//! // A request is a native service block; the service name says which half
//! // answers, and protoc checks both referenced types exist.
//! service Server {
//!   rpc Open(OpenRequest) returns (Latch);
//! }
//!
//! message OpenRequest { uint32 force = 1; }
//! ```
//!
//! Out come the payload types under [`prost`]'s encoding —
//! `Latch`, `OpenRequest` — with [`SignalSpec`] on `Latch` and [`RequestSpec`]
//! on `OpenRequest`. The declaration rides on the type that travels, so the
//! value an author constructs is what every verb reads it off.
//!
//! A declared name is the proto name in the manifest charset: `Latch` declares
//! `latch`, `PlayerPositions` would declare `player-positions`, and `rpc Open`
//! declares `open`. A message carrying no option and named by no `rpc` is a
//! plain type, so helpers and imported types declare nothing.
//!
//! # What the halves then write
//!
//! Nothing names a declaration as a string. The server half raises with
//! [`server::signal`](crate::server::signal) and answers with
//! [`RequestSpec::respond`]; the client half hears with
//! [`SignalSpec::observe`] and asks with
//! [`client::request`](crate::client::request), whose answer comes back at that
//! call site. Each verb reads the declaration off the type it was handed, and
//! the two registrations are methods ON that type.
//!
//! ```
//! use ironlark::server::prelude::*;
//! # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
//! // A real crate writes one line, `ironlark::protocol!("../protocol.proto")`,
//! // in a file both halves include. `signal`, `Context`, `Player` and
//! // `ServerMod` are all the prelude's.
//! struct Door;
//!
//! impl ServerMod for Door {
//!     async fn on_join(_ctx: Context, _player: Player) {
//!         // The payload is what the verb takes; the declaration is read off
//!         // it, so no name is spelled here.
//!         if let Err(e) = signal(&protocol::Latch { open: false }) {
//!             log::warn!("the joiner was not told the door is shut: {e}");
//!         }
//!     }
//! }
//! ```
//!
//! # What the session checks
//!
//! The compiler checks this crate against its own schema; the session checks
//! the schema that shipped. The host reads every enabled mod's compiled
//! descriptor before any of them loads, numbers the names it finds, and
//! [`resolved`](SignalSpec::resolved) hands back that number. A bare name is
//! the caller's own declaration, a name carrying `:` is another mod's, reached
//! by importing that mod's schema. Anything the session does not carry refuses
//! with [`UnresolvedName`](crate::ErrorKind::UnresolvedName), naming which
//! schema to check, at the first use rather than going quiet. Names are owned
//! per mod, so two authors may both declare a `latch`.
//!
//! # Nothing refuses
//!
//! This module holds declarations, not verbs. Every refusal named on these
//! pages happens at the verb that read the declaration, and each verb's page
//! says which.

mod audience;
mod payload;
pub(crate) mod request;
mod resolved;
pub(crate) mod signal;
mod sound;
mod transit;

pub use audience::{Audience, ClientMods, Clients, ServerMods};
pub use request::RequestSpec;
pub use signal::SignalSpec;
pub use sound::{PlayableSound, SoundSource, SoundSpec, resolve_sound_once, unrouted_hook};
pub use transit::{Keep, Order, Transit};

pub(crate) use payload::{decode, decode_refusing, encode};
