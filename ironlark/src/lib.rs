//! Write Ironlark mods in Rust. One crate serves both halves of a mod, the
//! names a mod declares become items the compiler checks, and the handlers
//! run under `cargo test` with no game present.
//!
//! A mod is the unit the game installs, enables and loads, and it has up to
//! two halves. The **server half** runs once per session on the machine
//! hosting it and holds authority over the world. The **client half** runs on
//! every player's machine and owns what that player sees and does. Each half
//! is a trait an author implements and a WebAssembly component of its own,
//! installed beside the manifest as `<mod>_server.wasm` and
//! `<mod>_client.wasm`. The surface each half may touch is a module of this
//! crate, [`server`] or [`client`].
//!
//! # Getting started
//!
//! [`server::prelude`] is the one import a server half needs. It carries
//! [`ServerMod`](server::ServerMod), the trait whose methods the host calls,
//! [`Context`], the event a hook is handling, and [`Player`](server::Player),
//! the handle standing for a participant. It carries [`export_server!`] too,
//! spelled in full below so the macro's origin stays visible. `log` is the
//! mod crate's own dependency, bridged to the host so a line lands in the
//! session log attributed to the mod's id.
//!
//! ```
//! // Brings ServerMod, Context and Player into scope.
//! use ironlark::server::prelude::*;
//!
//! struct Door;
//!
//! // Every method of ServerMod has an empty default body, so a half writes
//! // only the ones it answers to. `on_join` is one of them.
//! impl ServerMod for Door {
//!     async fn on_join(_ctx: Context, player: Player) {
//!         log::info!("{player} arrived");
//!     }
//! }
//!
//! // The last line of a server half's src/lib.rs.
//! ironlark::export_server!(Door);
//! ```
//!
//! That is a whole server half, and a mod with nothing to draw needs no
//! other. `cargo build --target wasm32-wasip2` produces the component, and
//! `cargo test` runs those same handlers natively against [`testing`].
//!
//! # Where to look next
//!
//! - [`server`] — the authority. Entities, possession, the raise and its
//!   narrowed form, answering requests, spatial queries. Gameplay starts here.
//! - [`client`] — one player's machine. Arriving signals, requests to the
//!   server half, the overlay, and the step, which reaches this half until its
//!   manifest section leaves it off.
//! - [`Context`] — the event a hook is handling. Its id, the tick it was
//!   raised on, the tick the handler runs on, and what else the host answers
//!   about it.
//! - [`state!`](macro@crate::state) and [`State`] — the one door to what a
//!   mod remembers between events, and the cell it declares.
//! - [`protocol`](mod@protocol) — the traits a declaration arrives as, and
//!   where a handler for a declared name is registered. A mod's own
//!   `protocol.proto` declares what it announces and what it answers, ON the
//!   payload types, and the generated types carry
//!   [`SignalSpec`](protocol::SignalSpec) or
//!   [`RequestSpec`](protocol::RequestSpec) — which is what puts
//!   [`observe`](protocol::SignalSpec::observe) and
//!   [`respond`](protocol::RequestSpec::respond) on the type itself, so
//!   `Latch::observe(shown)` is the whole registration. The convention is one
//!   file named `protocol.rs` holding the generated module, so the names minted
//!   from the manifest read `protocol::sound::Slam` and
//!   `protocol::archetype::Door`.
//! - [`prost`] and [`prost_types`] — the encoder the generated types ride, so
//!   a mod's `Cargo.toml` names `ironlark` and nothing else.
//! - [`hooks`](macro@crate::hooks), [`state!`](macro@crate::state),
//!   [`export_server!`] and [`export_client!`] — the macros. The manifest's
//!   hook list held to an impl block in both directions with the author hooks
//!   lifted out of it, the state cells, and each half's export. A prelude
//!   carries every one but the other realm's export, so a mod may name them
//!   bare.
//! - [`testing`] — the doubles that let `cargo test` drive handlers with no
//!   host present.
//! - [`Error`], [`Refusal`], [`ErrorKind`] — how a refusal reaches a mod, and
//!   how a mod refuses a caller.
//!
//! # How the surface is arranged
//!
//! One rule, no exceptions. **A module is for verbs, the root is for data,
//! and the realm is always the first level.** The root holds what has no
//! realm — errors, math, ids, [`SoundBus`], [`Context`], [`State`].
//! [`server`] and [`client`] hold everything a mod can do and every type
//! bound to one side. Inside a realm a module exists only where verbs have no
//! object of their own, as [`audio`](server::audio),
//! [`resolve`](server::resolve) and [`ui`](client::ui) do. Each realm carries
//! a prelude, and it is the one import a mod needs.
//!
//! # The two rules a mod is written against
//!
//! **Every hook but `init` opens with the event it is handling.** That
//! [`Context`] is where the ticks live and where anything else about the
//! event is asked for, so a hook never has to grow a parameter again. It is
//! `Copy` and holds no borrow, so pass it down freely.
//!
//! **A handle the host lends lives for one event.**
//! [`Player`](server::Player) and the entity inside a
//! [`Target`](server::Target) are minted as the handler is entered and
//! deleted as it returns, and the number a stored one carries goes on to
//! address whoever came next. Using one after its event is a typed error, so
//! keep the ids a handle exposes and not the handle. An
//! [`Entity`](server::Entity) the mod spawned is the mod's own, a clone of it
//! is a co-owner, and caching one is the intended thing.
//!
//! The stamp that enforces this is carried only by the component build, so a
//! handler that hoards a handle and reads it a tick later passes `cargo test`
//! and refuses in a game. It is the one rule in this crate that reading
//! catches and [`testing`] does not.
//!
//! **What the contract calls immutable is already in hand.**
//! [`Player::session`](server::Player::session) and
//! [`Player::user`](server::Player::user) were stamped into the handle when it
//! was lent, so they cost nothing and cannot fail.
//! [`profile`](server::Player::profile) and [`body`](server::Player::body)
//! cross into the host, so both are `async` and both answer a [`Result`].
//! `body` is the one the host itself declines, when nothing is being
//! controlled or the participant has gone. `profile` refuses on a handle
//! whose event is over and on nothing else.
//!
//! # What refuses
//!
//! A hook returns nothing, so a refusal has nowhere to be thrown and `?` has
//! no place in a hook body. The verbs that can refuse answer a [`Result`] at
//! the call site, and the mod decides there what the refusal means. Match on
//! it, say what happened, and continue or stop. A refusal carries the host's
//! own message naming what happened, so a mod that logs it says more than that
//! something failed. Whether the host also wrote a line of its own is the
//! host's business and not a mod's to count on.
//!
//! # What is not built
//!
//! Personas. [`Player::profile`](server::Player::profile) answers `None` for
//! every participant, because the host mints none. The option is the
//! contract's, so a mod that writes the `Some` branch now keys correctly the
//! day the host starts answering.
//!
//! The structural hooks. The contract exports a body created, a map loaded, a
//! profile hot-swapped and a body spawn requested among others. Nothing fires
//! them, they are not on [`ServerMod`](server::ServerMod), and an author
//! cannot implement one.

#![deny(missing_docs)]

// The generated code names `ironlark::`, and the doctest fixtures compile
// inside this crate, which cannot otherwise name itself by its package name.
extern crate self as ironlark;

pub mod client;
mod context;
mod error;
mod host;
mod ids;
mod math;
pub mod protocol;
pub mod server;
mod shared;
mod state;
#[cfg(not(target_arch = "wasm32"))]
pub mod testing;

// The exported macros and the schema generator name `ironlark::bindings`;
// this re-export is that path, so the module's file lives under host/.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use host::bindings;

pub use context::{Cause, Context};
pub use error::{Error, ErrorKind, Refusal, Result};
pub use ids::{
    ComponentId, EventId, FieldId, HookId, ProfileId, RequestId, SessionId, SignalId, SoundId,
    SourceId, Tick, UserId,
};
pub use math::{Quat, Rgba, Vec3};
pub use shared::audio::SoundBus;
pub use state::State;

pub use ironlark_macros::{declares, hooks, protocol};

/// The protobuf encoder every payload type rides, re-exported so a mod's
/// `Cargo.toml` names no encoder of its own.
///
/// A mod's payload types are generated from its `protocol.proto` and the
/// generated code reaches prost through this path, which is why a mod depends
/// on `ironlark` and nothing else. An author writes `ironlark::prost` only when
/// hand-writing a type the schema did not generate — rare, and the reason it is
/// public rather than hidden.
///
/// ```
/// // The trait the verbs' bounds are written against, reached through this
/// // re-export rather than through a dependency of the mod's own.
/// use ironlark::prost::Message;
///
/// #[derive(Clone, PartialEq, ironlark::prost::Message)]
/// #[prost(prost_path = "ironlark::prost")]
/// pub struct Latch {
///     #[prost(bool, tag = "1")]
///     pub open: bool,
/// }
///
/// // One allocation, sized exactly: what the SDK's own encode does.
/// let bytes = Latch { open: true }.encode_to_vec();
/// assert_eq!(bytes.len(), Latch { open: true }.encoded_len());
/// ```
///
/// # Nothing refuses
///
/// It is a re-export. Encoding a message cannot fail; decoding answers a
/// `Result`, and the SDK's own decodes turn that into an
/// [`Error`] or a [`Refusal`] at the verb that did the decoding.
pub use prost;

/// The protobuf well-known types, re-exported for the same reason
/// [`prost`] is.
///
/// A schema importing `google/protobuf/timestamp.proto` or any other
/// well-known type generates code naming this path. A mod that imports none
/// never mentions it, and no mod ever adds the dependency itself.
///
/// ```
/// // The one name a schema's well-known import reaches: nothing else in the
/// // SDK hands a mod one of these.
/// let epoch = ironlark::prost_types::Timestamp { seconds: 0, nanos: 0 };
/// assert_eq!(epoch.seconds, 0);
/// ```
///
/// # Nothing refuses
///
/// It is a re-export of plain types.
pub use prost_types;

/// Compiles the repository README's examples as doctests, so the page a
/// newcomer reads first cannot drift from the crate it describes.
#[doc = include_str!("../../README.md")]
#[cfg(doctest)]
struct ReadmeIsChecked;

/// Makes a crate the server half of a mod. The named
/// [`ServerMod`](server::ServerMod) implementation becomes what the host
/// calls.
///
/// The trait says what the half does. This line is what lets the host reach
/// it, so it goes last in a server half's `src/lib.rs`. Built for
/// `wasm32-wasip2`, the crate produces the component installed beside the
/// manifest as `<mod>_server.wasm`.
///
/// The exported half is the authority. It may use everything under
/// [`server`] — entities, the session's participants, the raise, answering
/// requests, spatial queries.
///
/// [`server::prelude`] brings [`ServerMod`](server::ServerMod), [`Context`]
/// and [`Player`](server::Player) into scope, and carries this macro too. The
/// example spells the macro in full so its origin stays visible.
///
/// ```
/// // Brings ServerMod, Context and Player into scope.
/// use ironlark::server::prelude::*;
///
/// struct Door;
///
/// // `on_join` is one of ServerMod's methods. The rest keep their empty
/// // default bodies.
/// impl ServerMod for Door {
///     async fn on_join(_ctx: Context, player: Player) {
///         log::info!("{player} arrived");
///     }
/// }
///
/// // The whole argument: one identifier naming the type above.
/// ironlark::export_server!(Door);
/// ```
///
/// It takes that identifier and reads nothing else. What the mod declares is
/// its `protocol.proto`'s business and its manifest's, and a half that names
/// no traffic has no `protocol.proto` at all. A mod that declares no signal,
/// request, sound or archetype is a trait impl and this line, and nothing
/// more.
/// [`#[ironlark::hooks]`](macro@crate::hooks) joins them only once the
/// manifest carries a hook list for this half, because a manifest that names
/// none leaves the attribute nothing to hold the impl block to.
///
/// # What refuses
///
/// Both refusals are compile errors. The argument is a bare identifier naming
/// a type in this scope, so a path or a generic does not parse. A type that
/// does not implement [`ServerMod`](server::ServerMod) is reported against the
/// trait.
///
/// Under `cargo test` there is no host to export to, so the expansion is only
/// a reference that keeps the implementation live for the compiler. That is
/// why one file both builds a component and runs its handlers natively against
/// [`testing`].
#[macro_export]
macro_rules! export_server {
    ($ty:ident) => {
        #[cfg(target_arch = "wasm32")]
        $crate::bindings::server::export_server_world!($ty with_types_in $crate::bindings::server);
        // Natively the host is absent; the reference keeps the exported entry
        // points live for the compiler.
        #[cfg(not(target_arch = "wasm32"))]
        const _: () = {
            let _ = <$ty as $crate::server::ServerMod>::init;
        };
    };
}

/// Makes a crate the client half of a mod. The named
/// [`ClientMod`](client::ClientMod) implementation becomes what this player's
/// machine calls.
///
/// The mirror of [`export_server!`], and the last line of a client half's
/// `src/lib.rs`. Built for `wasm32-wasip2`, the crate produces the component
/// installed beside the manifest as `<mod>_client.wasm`. The two halves are
/// separate crates compiling against one `protocol.proto`, so both sides of a
/// signal or a request are the same types.
///
/// The client realm is narrower than the server realm by the contract's
/// design. [`client`] offers the overlay, arriving signals, its own machine's
/// raise, audio on this machine, the session's participants and requests to
/// the server half. It offers no entities, no narrowed raise and no spatial
/// queries, because the host hands a client component none of them.
///
/// [`client::prelude`] brings [`ClientMod`](client::ClientMod) into scope and
/// carries this macro too. The example spells the macro in full so its origin
/// stays visible.
///
/// ```
/// // Brings ClientMod into scope.
/// use ironlark::client::prelude::*;
///
/// struct Doorbell;
///
/// // `init` is ClientMod's, and it runs once on this machine before anything
/// // else. `log` is the mod crate's own dependency.
/// impl ClientMod for Doorbell {
///     async fn init() {
///         log::info!("doorbell: ready");
///     }
/// }
///
/// // The whole argument: one identifier naming the type above.
/// ironlark::export_client!(Doorbell);
/// ```
///
/// # What refuses
///
/// As with [`export_server!`], the argument is a bare identifier naming a type
/// in this scope, and a type that does not implement
/// [`ClientMod`](client::ClientMod) is a compile error against the trait.
///
/// Reaching out of the realm is not one. A client half may name a [`server`]
/// verb and the crate still compiles. The component it produces then asks the
/// host for something a client component was never handed, and the host cannot
/// bring it up. Keep a client half inside [`client`], and let the server half
/// answer for anything else.
///
/// Under `cargo test` there is no host to export to, so the expansion is only
/// a reference that keeps the implementation live for the compiler, and the
/// [`testing`] doubles drive the same handlers natively.
#[macro_export]
macro_rules! export_client {
    ($ty:ident) => {
        #[cfg(target_arch = "wasm32")]
        $crate::bindings::client::export_client_world!($ty with_types_in $crate::bindings::client);
        #[cfg(not(target_arch = "wasm32"))]
        const _: () = {
            let _ = <$ty as $crate::client::ClientMod>::init;
        };
    };
}
