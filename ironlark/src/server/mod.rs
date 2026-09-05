//! The server half of a mod: the authority over one session.
//!
//! One instance runs on the machine hosting the session. It spawns entities,
//! owns their component rows, answers the client half's calls and publishes
//! facts other mods subscribe to. A client half renders and reads input. Where
//! the two disagree, this half is right.
//!
//! Open a server half with the prelude, implement [`ServerMod`], and export it
//! with [`export_server!`](crate::export_server):
//!
//! ```
//! // The prelude is the whole import. `ServerMod`, `Context`, `Player` and
//! // `export_server!` all arrive with it.
//! use ironlark::server::prelude::*;
//!
//! struct Doorman;
//!
//! impl ServerMod for Doorman {
//!     async fn on_join(_ctx: Context, player: Player) {
//!         log::info!("{player} arrived");
//!     }
//! }
//!
//! ironlark::export_server!(Doorman);
//! ```
//!
//! Every hook has an empty default body, so implement only what the mod
//! answers to. A hook returns nothing, so what to do about a refused verb is
//! decided where the verb was called.
//!
//! # The manifest says which hooks this half implements
//!
//! Implementing a hook is half of the statement. The other half is a
//! `[declares.server]` section at the bottom of the manifest's `[declares]`
//! block, naming hooks as [`ServerMod`] spells them:
//!
//! ```toml
//! [declares.server]
//! hooks = ["on_tick", "on_interact"]
//! ```
//!
//! [`init`](ServerMod::init) is never written there. The section existing IS
//! the statement that this half exists and inits, so a mandatory thing is not
//! declared, and a section with no `hooks` key means exactly that: only init.
//!
//! The host reads the list before it wakes this half. A tick is admitted only
//! when the section names [`on_tick`](ServerMod::on_tick). A join and a leave
//! answer to one declaration between them, so naming either admits both, and a
//! half naming neither is woken for neither. An interaction and a contact
//! arrive on the strength of the archetypes this mod declares and are not
//! gated on the list.
//!
//! Writing [`#[ironlark::hooks("../mod.toml")]`](macro@crate::hooks) on the
//! impl block is what stops the two from drifting apart. It reads the manifest
//! while the crate compiles and fails the build in both directions, on a hook
//! implemented here and unlisted there, and on one listed there and not
//! implemented here. The examples on this page leave it off because a doctest
//! has no manifest to point at.
//!
//! # Every hook opens with the event
//!
//! [`Context`](crate::Context) is the first argument of every hook but
//! [`init`](ServerMod::init). It carries the event's id, the tick the event was
//! raised on and the tick the handler runs on, and it answers
//! [`cause`](crate::Context::cause) and [`instance`](crate::Context::instance)
//! about the event. It is `Copy` and borrows nothing, so pass it into a helper
//! freely.
//!
//! # Which handles outlive their event
//!
//! [`Player`] and the entity inside a [`Target`] are lent by the host for the
//! handler that received them and reclaimed after it. Each carries the event it
//! belongs to, so using one later refuses with
//! [`StaleId`](crate::ErrorKind::StaleId) rather than resolving to whoever came
//! next.
//!
//! Keep the ids, not the handle:
//!
//! ```
//! // `SessionId`, `Context`, `Player` and `ServerMod` are all the prelude's;
//! // `ironlark::state!` is spelled in full so its origin stays visible.
//! use ironlark::server::prelude::*;
//!
//! ironlark::state! {
//!     static SEEN: Vec<SessionId> = Vec::new();
//! }
//!
//! struct Register;
//!
//! impl ServerMod for Register {
//!     async fn on_join(_ctx: Context, player: Player) {
//!         // The session id was stamped into the handle and outlives it.
//!         SEEN.update(|seen| seen.push(player.session()));
//!     }
//! }
//! ```
//!
//! An [`Entity`] this mod spawned is owned rather than lent. A clone is a
//! co-owner, and caching one in a [`State`](crate::State) cell is the intended
//! thing to do. So is the body [`Player::body`] and [`body_of`] answer with,
//! though for a body the handle outliving the event is not the same as the
//! STANDING to act through it, which does not. See [`Entity`].
//!
//! # What is free and what costs a call
//!
//! The contract marks each participant fact immutable or live, and this surface
//! follows it. [`Player::session`] and [`Player::user`] were taken when the
//! handle was stamped, so they cost nothing and cannot fail. [`Player::body`]
//! is a live read, because possession moves, so it is a call and it can refuse.
//! [`Player::profile`] is shaped as a live read for the same reason and answers
//! `None` in every session, because personas are not built. The call exists so
//! that a mod written against it keys correctly the day they arrive.
//!
//! # Talking to everyone else
//!
//! [`signal`] raises an announcement, and where it lands is the declaration's
//! own business: the server realm's bus, where other mods' server halves hear
//! it, or across the network to every client realm. [`signal_to`] is that same
//! announcement narrowed to one participant's machine. Nothing is addressed
//! and nothing answers.
//!
//! # Where a handler for a declared name is registered
//!
//! On the payload type. A request the mod declared is answered with
//! [`respond`](crate::protocol::RequestSpec::respond), a raised signal is heard
//! with [`observe`](crate::protocol::SignalSpec::observe), and both are methods
//! on the type the schema generated:
//!
//! ```
//! use ironlark::server::prelude::*;
//! # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
//! // A real crate writes `mod protocol;` here. `RequestSpec` and `SignalSpec`
//! // are the prelude's, and they are what puts the two verbs on the types.
//! use protocol::{Latch, Opened, OpenRequest};
//!
//! struct Door;
//!
//! impl ServerMod for Door {
//!     async fn init() {
//!         OpenRequest::respond(open); // the one handler with a caller waiting
//!         Opened::observe(noticed);   // an announcement, answering nothing
//!     }
//! }
//!
//! async fn open(_ctx: Context, _caller: Player, ask: OpenRequest) -> Result<Latch, Refusal> {
//!     Ok(Latch { open: ask.force > 0 })
//! }
//!
//! async fn noticed(_ctx: Context, _from: SourceId, fact: Opened) {
//!     log::info!("another server half says {} opened", fact.name);
//! }
//! ```
//!
//! The line opens with the name, there is no table to build and nothing to
//! install, so a registration that was written cannot fail to take effect.
//! Declared traffic does not widen [`ServerMod`]: teaching a mod one more
//! request adds one line, not a hook. [`init`](ServerMod::init) is the usual
//! place to write those lines and it is not the only legal one — each page says
//! what a later registration and a second one for the same name do.

pub mod audio;
pub(crate) mod entity;
pub(crate) mod events;
pub mod gamemode;
pub(crate) mod half;
pub mod map;
pub(crate) mod player;
pub mod resolve;
pub mod session;
pub(crate) mod shape;
pub mod spatial;

pub use entity::{Entity, Field, SpawnPoint, Value, body_of, find};
pub use events::{Arrival, ContactEdge, LeaveReason, PhysicsObject, Target};
pub use half::ServerMod;
pub use player::Player;
pub use shape::Shape;

/// Raises an announcement. Whoever subscribed to it hears it, and nobody
/// answers.
///
/// One typed value, one call. The payload type carries the declaration, so
/// what travels and where it lands are settled where the mod's
/// `protocol.proto` declared them, not at this call site. The audience is the
/// declaration's: [`ServerMods`](crate::protocol::ServerMods) keeps
/// the raise on the server realm's bus, where other mods' server halves hear
/// it; [`Clients`](crate::protocol::Clients) crosses the network to
/// every client realm.
///
/// It is how two mods that have never heard of each other end up connected. A
/// door mod announces that its door opened and stops there, because who cares
/// is not its business. It is equally how the authority tells the screens what
/// is true — and for that, raise the WHOLE state rather than a change to it,
/// so a client half that joined late is right after one raise instead of
/// depending on having heard every earlier one.
///
/// Synchronous, because a raise awaits nothing. `Ok` means the host took it.
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // A real crate writes `mod protocol;` here, holding what the generator
/// // wrote for the mod's own protocol.proto:
/// //
/// //     message Latch {
/// //       option (ironlark.signal) = CLIENTS;
/// //       bool open = 1;
/// //     }
/// //
/// // The hidden lines above are exactly that output: the prost type, and the
/// // `SignalSpec` impl carrying the declared name `latch`, the audience, and
/// // the cell its compact id resolves into once. `signal`, `Context`,
/// // `Player`, `Target`, `Vec3` and `ServerMod` are all the prelude's.
/// struct Door;
///
/// impl ServerMod for Door {
///     async fn on_interact(
///         _ctx: Context,
///         _player: Player,
///         _target: Target,
///         _hit_point: Vec3,
///         _distance: f32,
///     ) {
///         // The whole state, so a screen that missed the last raise is still
///         // right after this one.
///         if let Err(e) = signal(&protocol::Latch { open: true }) {
///             log::warn!("the door state did not go out: {e}");
///         }
///     }
/// }
/// ```
///
/// Use [`signal_to`] where the announcement is one participant's business.
/// [`observe`](crate::protocol::SignalSpec::observe) on the payload type is the
/// receiving end, in either realm.
///
/// # Who may raise what
///
/// Existence is checked and ownership is not. A mod may raise a name another
/// mod declared, which is how a gamemode commands content it does not own by
/// importing that mod's schema. What is refused is a name no enabled mod
/// declares. The host stamps the raiser's own id onto every delivery, so an
/// observer that obeys only one raiser can tell.
///
/// A raise from [`init`](ServerMod::init) is legal and normal. Until every
/// server half has loaded, the bus holds raises instead of dropping them, then
/// delivers them in the order they were made.
///
/// # What refuses
///
/// A payload over the host's byte cap is
/// [`TooLarge`](crate::ErrorKind::TooLarge), refused whole: nothing is
/// truncated and nothing goes partly out. There are two caps, and the
/// declaration picks which one applies: a signal declaring
/// [`Keep::Newest`](crate::protocol::Keep::Newest) rides a path where one
/// message cannot be split, so its cap is the smaller. The refusal reads the
/// same either way and names the cap it measured against. A name this session
/// does not carry,
/// or carries in the other realm, is
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName), which is what a mod
/// meets when the declaring mod is not enabled. Overload — a full queue, or
/// more raises in one tick than this mod's budget — is the host's own trouble
/// and reads as [`Other`](crate::ErrorKind::Other), never as a silent shed.
///
/// `Ok` means the host took it, never that anybody acted on it. A name nobody
/// subscribes to is inert and says so in the host's log alone; an observer too
/// far behind sheds. Neither reaches the raiser, so this call cannot be used
/// to learn whether anyone is listening.
pub fn signal<P: crate::protocol::SignalSpec>(raised: &P) -> crate::Result<()> {
    crate::protocol::signal::signal(raised)
}

/// Raises the same announcement, narrowed to one participant's machine.
///
/// The same declaration and the same payload type as [`signal`], addressed by
/// [`SessionId`](crate::SessionId), so a mod can tell one participant
/// something the others do not learn. Narrowing is per raise; nothing about it
/// is declared.
///
/// What it is for is the joiner. A client half that arrives mid-session has
/// missed every raise so far, and the next one may be far away. Answering that
/// one participant costs the others nothing.
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden lines above are what the generator wrote for the same
/// // `message Latch { option (ironlark.signal) = CLIENTS; bool open = 1; }`
/// // [`signal`]'s example declares. `signal_to`, `Context`, `Player` and
/// // `ServerMod` are all the prelude's.
/// ironlark::state! {
///     static OPEN: bool = false;
/// }
///
/// struct Door;
///
/// impl ServerMod for Door {
///     async fn on_join(_ctx: Context, player: Player) {
///         let state = protocol::Latch { open: OPEN.get() };
///         // `player.session()` is the address, and it outlives the handle.
///         if let Err(e) = signal_to(player.session(), &state) {
///             log::warn!("the joiner was not told the door state: {e}");
///         }
///     }
/// }
/// ```
///
/// Server realm, and its own interface in the contract, so a client half
/// simply does not import it: a client half raises on its own machine, and
/// reaching another participant's machine is not a thing it can spell.
///
/// # What the compiler refuses
///
/// Narrowing means nothing for an announcement that never leaves a bus, so
/// this verb accepts only a declaration whose audience is
/// [`Clients`](crate::protocol::Clients). Anything else is a compile
/// error naming the audience it read, rather than a refusal met once a session
/// is running.
///
/// # What refuses
///
/// The same three as [`signal`]. [`TooLarge`](crate::ErrorKind::TooLarge) is a
/// payload over the host's byte cap, refused whole.
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) is a name this session
/// does not carry. [`Other`](crate::ErrorKind::Other) is the host's own
/// trouble, overload included.
///
/// The address is NOT among them. `Ok` means the host took it for delivery,
/// and addressing happens after this call has answered. A session id that
/// names nobody, and a participant whose connection is already gone, are both
/// a host-side warning the raiser never sees.
pub fn signal_to<P>(to: crate::SessionId, raised: &P) -> crate::Result<()>
where
    P: crate::protocol::SignalSpec<Audience = crate::protocol::Clients>,
{
    crate::protocol::signal::signal_to(to, raised)
}

/// One import for a server half.
///
/// Everything the authority writes against, under one glob. That is the
/// [`ServerMod`] trait, the verbs, the handles the hooks hand over, the two
/// declaration traits that put [`observe`](crate::protocol::SignalSpec::observe)
/// and [`respond`](crate::protocol::RequestSpec::respond) on a payload type,
/// the verb modules, and the root types that belong to no realm at all. A
/// server half's first line is this and its second line is its own business.
///
/// ```
/// use ironlark::server::prelude::*;
/// struct Door;
/// impl ServerMod for Door {}
/// ```
///
/// That is not a starting point a mod outgrows. A mod that keeps
/// [`SessionId`](crate::SessionId)s in a [`State`](crate::State) cell, reads
/// them out of a [`Player`], branches on a [`LeaveReason`] and raises with
/// [`signal`] adds no second import for any of it.
///
/// # The prelude is the shape of the realm
///
/// The modules the verbs live in are private, and each realm re-exports the
/// ones it is entitled to, so the import list IS the surface. A name that is
/// here is a thing a server half does, and a name that is missing is a thing it
/// does not. [`request`](crate::client::request) and
/// [`ui`](crate::client::ui) are missing because the authority has nobody to
/// ask and nothing to paint. The client realm's prelude is missing [`Entity`]
/// and [`signal_to`] for the mirror-image reason.
///
/// Read the door as a door rather than as a wall. Every realm is a public path,
/// so `ironlark::client::request` is a name a server half can spell and the
/// compiler will accept. What refuses it is the game. A server half is
/// instantiated into a world that imports no request interface, and a
/// component reaching for one does not load. Importing the prelude and taking
/// what it offers is how that never comes up.
///
/// The glob costs nothing at run time. It is a list of names, and what each one
/// does is that name's own page.
pub mod prelude {
    pub use super::{
        Arrival, ContactEdge, Entity, Field, LeaveReason, PhysicsObject, Player, ServerMod, Shape,
        SpawnPoint, Target, Value, body_of, find, signal, signal_to,
    };
    pub use super::{audio, gamemode, map, resolve, session, spatial};
    pub use crate::protocol::{RequestSpec, SignalSpec};
    pub use crate::{
        Cause, ComponentId, Context, Error, EventId, FieldId, ProfileId, Quat, Refusal, Result,
        Rgba, SessionId, SignalId, SoundBus, SoundId, SourceId, State, Tick, UserId, Vec3,
        export_server, hooks, state,
    };
}
