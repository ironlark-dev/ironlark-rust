//! The client half of a mod: what a player sees and does.
//!
//! One instance of this half runs on every machine a participant plays on. It
//! renders, it reads that player's input, and it asks the mod's own server half
//! for anything that has to be true for everyone. It decides nothing. The
//! entities, the bodies participants possess and the world itself belong to
//! [`server`](crate::server) and are reached from there.
//!
//! A half is a component the host loads out of the mod's folder. The trait it
//! implements is [`ClientMod`], the names it may use are declared in `mod.toml`
//! beside the crate, and [`export_client!`](crate::export_client) is what makes
//! one type the host enters.
//!
//! # Every hook opens with the event
//!
//! [`Context`](crate::Context) is the first argument of every hook and every
//! registered handler but `init`. It carries the event's id and the two ticks,
//! the one it was raised on and the one you are handling it on.
//!
//! The id names one delivery on this machine, not one exchange across the
//! network. A raise the server half made is a fresh event where it is
//! admitted here, so the id this half reads and the id the raising half held
//! are two different ids. What it is for is tying together everything one
//! handler says about the one thing it is handling.
//!
//! # What arrives, and in what order
//!
//! A signal carries an announcement, and a handler for one registers on the
//! payload type itself, with
//! [`observe`](crate::protocol::SignalSpec::observe). The line opens with the
//! signal's name, so a name is never spelled as a string and a type that
//! declares nothing is a compile error rather than a handler that silently
//! never fires.
//!
//! Both routes arrive that way. A raise on this machine's own client bus and a
//! raise crossing from the server realm reach this half through one export, so
//! there is one kind of registration and no second noun for the crossing case.
//! Every delivery carries the raiser's host-stamped [`SourceId`](crate::SourceId).
//!
//! An input edge is the other arrival, and it is not traffic. It enters an
//! author hook — a mod entry point declared in `mod.toml` with the rule that
//! invokes it, `default-bindings` naming the keys it answers by default. The
//! host owns the binding table and the player rebinds, so a hook is a name the
//! mod answers to rather than a key it chooses. A hook is reached as a function
//! the [`#[ironlark::hooks]`](macro@crate::hooks) attribute lifts out of the
//! impl block, not as a registration.
//!
//! [`ClientMod::on_tick`] is the arrival that carries no name. The host hands it
//! to a half whose `[declares.client]` section lists `on_tick` among its
//! `hooks`. A half that writes that section without it is never entered for the
//! step and never pays for it, and a half with no section at all is unknown
//! rather than empty and is handed everything, which is what lets a mod adopt
//! the sections one half at a time.
//!
//! Only the newest step is held. A half that fell behind is handed the current
//! tick, never the one it missed, and the seconds a replaced tick carried are
//! not added to it. A mod that is behind should look behind rather than have
//! its motion quietly reconstructed.
//!
//! The order is fixed. An arriving signal is taken first, then an input edge,
//! then the tick. A burst of traffic is drained before the step it belongs to,
//! and a tick held behind a run of other items is taken anyway rather than
//! starved, so a mod that drives motion from the step keeps moving while it
//! catches up.
//!
//! Going the other way there are two verbs. [`request`] asks this mod's own
//! server half a typed question and awaits the answer. [`signal`] raises an
//! announcement on this machine's client bus, where this machine's other
//! client halves hear it and nothing leaves the machine.
//!
//! # A half that shows a door and asks to open it
//!
//! It paints what the server half raises, and on the press of the key bound to
//! its own hook it asks the server half to open the door.
//! [`ui::set_overlay_text`] is this mod's line of the overlay, [`InputEdge`]
//! says which way the key moved, and
//! [`observe`](crate::protocol::SignalSpec::observe) on the payload type is
//! where the handler for the raise is registered.
//!
//! ```
//! use ironlark::client::prelude::*;
//! # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
//! // A real crate writes `mod protocol;` here, holding what the generator
//! // wrote for the mod's own protocol.proto — the same file the server half
//! // compiles against:
//! //
//! //     message Latch { option (ironlark.signal) = CLIENTS; bool open = 1; }
//! //     service Server { rpc Open(OpenRequest) returns (Latch); }
//! //     message OpenRequest { uint32 force = 1; }
//! //
//! // The hidden lines above are that output. `SignalSpec`, `request`,
//! // `Context`, `SourceId`, `InputEdge`, `ui` and `ClientMod` are the
//! // prelude's.
//! use protocol::{Latch, OpenRequest};
//!
//! struct Door;
//!
//! impl ClientMod for Door {
//!     async fn init() {
//!         Latch::observe(shown);
//!     }
//! }
//!
//! // What the server half raised, already decoded. `from` is the raiser the
//! // host stamped, and no registration names the signal.
//! async fn shown(_ctx: Context, _from: SourceId, latch: Latch) {
//!     ui::set_overlay_text(if latch.open { "door: open" } else { "door: shut" });
//! }
//!
//! // An author hook. Both edges arrive, so one that acts on the press says so.
//! async fn pressed(_ctx: Context, edge: InputEdge) {
//!     if edge != InputEdge::Pressed {
//!         return;
//!     }
//!     match request(&OpenRequest { force: 1 }).await {
//!         Ok(answer) => log::info!("the server says open = {}", answer.open),
//!         Err(e) => log::warn!("the door did not open: {e}"),
//!     }
//! }
//! ```
//!
//! # What is shared with the other half
//!
//! Errors, ids, math and [`State`](crate::State) live at the crate root and
//! mean the same thing in both halves, and so do [`resolve`] and [`session`].
//! [`audio::play`] is here too and is the one verb whose meaning differs.
//! Called from this half, the machine running it hears the sound alone. The
//! same call from the server half is heard by every participant.

pub mod audio;
pub(crate) mod half;
pub(crate) mod input;
pub mod resolve;
pub mod session;
pub mod ui;

pub use half::ClientMod;
pub use input::InputEdge;

/// Asks this mod's server half a declared question and decodes its answer.
///
/// The one awaited verb in the SDK. The request type carries the declaration,
/// so nothing names it here; the answer is
/// [`Response`](crate::protocol::RequestSpec::Response). This is the only way
/// a client half reaches the authority, and the only place in a mod where a
/// [`Result`](crate::Result) is an ANSWER rather than a report. The handler on
/// the other end answers with the response type or a
/// [`Refusal`](crate::Refusal), because its no is something a caller waits on.
///
/// Every act the player performs that has to be true for everyone comes
/// through here. [`ErrorKind`](crate::ErrorKind) is at the crate root rather
/// than in the prelude, so it is the one name here with an import of its own.
///
/// ```
/// use ironlark::client::prelude::*;
/// use ironlark::ErrorKind;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // A real crate writes `mod protocol;` here. The hidden lines above are what
/// // the generator wrote for
/// //
/// //     service Server { rpc Open(OpenRequest) returns (Latch); }
/// //
/// // in the mod's protocol.proto: the two prost types, and the `RequestSpec`
/// // impl carrying the declared name `open`, the response type, the mod's
/// // request obligations and the cell its compact id resolves into once.
/// // `request`, `Context`, `InputEdge` and `ui` are the prelude's.
/// use protocol::OpenRequest;
///
/// // An author hook, lifted out of the impl block by
/// // `#[ironlark::hooks("../mod.toml")]`; the manifest names the keys it
/// // answers by default.
/// async fn pressed(_ctx: Context, edge: InputEdge) {
///     if edge != InputEdge::Pressed {
///         return;
///     }
///     match request(&OpenRequest { force: 1 }).await {
///         Ok(answer) if answer.open => ui::set_overlay_text("door: open"),
///         Ok(_) => ui::set_overlay_text("door: stuck"),
///         Err(e) if e.kind() == ErrorKind::Refused => {
///             ui::set_overlay_text("door: locked");
///             log::info!("the server turned the press down: {e}");
///         }
///         Err(e) => log::warn!("the press never reached the server: {e}"),
///     }
/// }
/// ```
///
/// Client realm, because this is the half that has to ask, and its own
/// interface in the contract: the server world does not import it, having
/// nobody above it to ask. The answering side is
/// [`respond`](crate::protocol::RequestSpec::respond), written on the request
/// type in the server half's `init`.
///
/// # A request is never another mod's
///
/// Routing takes a request to the mod whose schema declares it, so the pair of
/// types is written once, in that mod's own `protocol.proto`, and both halves
/// compile against the same file.
///
/// # What refuses
///
/// [`Refused`](crate::ErrorKind::Refused) is the request answered with a no,
/// and it covers more than the handler. The handler's own
/// [`Refusal`](crate::Refusal) arrives here as this kind, carrying the
/// sentence that refusal formats to; the typed variant does not survive the
/// crossing, because the wire carries a code and a text. Under the same kind
/// arrive the failures before the handler ever runs: no enabled mod declares
/// the name, the answering mod is not running, that mod is overloaded and did
/// not take it, the request never got out. Match on
/// [`kind`](crate::Error::kind) and log the message, which is the part that
/// says which of them happened.
///
/// A request over the host's byte cap is
/// [`TooLarge`](crate::ErrorKind::TooLarge), refused where it enters the host
/// and never sent to the answering half. A name this session does not carry is
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName). A bridge that closed,
/// an answer that was lost, and a wait the host bounded and gave up on all
/// read as [`Other`](crate::ErrorKind::Other), and so does an answer that
/// arrives but does not parse as the declared response type.
///
/// `Ok` is worth as much as the handler that produced it, and says the answer
/// really came back from the authority. A mod whose handler also raises the
/// result should render the raise and ignore the return, so a broken raise
/// cannot still look right on the caller's own screen.
///
/// On the build machine there is no host and no network. It goes straight to
/// this mod's own registered handler, so a client half's logic is testable
/// against the server half it ships with.
pub async fn request<R: crate::protocol::RequestSpec>(req: &R) -> crate::Result<R::Response> {
    crate::protocol::request::request(req).await
}

/// Raises an announcement on this machine's own client bus.
///
/// The client realm's raise. It reaches the other client halves on this
/// machine that observed the name, and it leaves the machine for nowhere: a
/// client half signals its own machine only, and reaching another
/// participant's machine is not a thing this realm can spell. Crossing the
/// network is the authority's, and [`server::signal`](crate::server::signal) is
/// the verb that does it.
///
/// What it is for is two client halves on one machine that should not name
/// each other — an overlay mod painting what a gameplay mod noticed, without
/// either importing the other's code.
///
/// Synchronous, because a raise awaits nothing. `Ok` means the host took it.
///
/// ```
/// use ironlark::client::prelude::*;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden lines above are what the generator wrote for
/// //
/// //     message Aimed { option (ironlark.signal) = CLIENT_MODS; string at = 1; }
/// //
/// // `CLIENT_MODS` is the audience that stays on this machine. `signal`,
/// // `Context` and `InputEdge` are the prelude's.
/// async fn tell_this_machine(_ctx: Context, at: String) {
///     if let Err(e) = signal(&protocol::Aimed { at }) {
///         log::warn!("nothing on this machine was told: {e}");
///     }
/// }
/// ```
///
/// # What the compiler refuses
///
/// A declaration whose audience is not
/// [`ClientMods`](crate::protocol::ClientMods) is a compile error
/// here, naming the audience it read. A server-bus name is not this realm's to
/// raise, and a crossing name is raised by the authority.
///
/// # What refuses
///
/// The same as [`server::signal`](crate::server::signal), read from this side.
/// [`TooLarge`](crate::ErrorKind::TooLarge) is a payload over the host's byte
/// cap, refused whole. [`UnresolvedName`](crate::ErrorKind::UnresolvedName) is
/// a name this session does not carry, or carries in the other realm.
/// [`Other`](crate::ErrorKind::Other) is the host's own trouble, a full queue
/// or a spent per-tick budget included.
///
/// `Ok` means the host took it, never that anybody acted on it.
pub fn signal<P>(raised: &P) -> crate::Result<()>
where
    P: crate::protocol::SignalSpec<Audience = crate::protocol::ClientMods>,
{
    crate::protocol::signal::signal(raised)
}

/// One import for a client half.
///
/// Everything the player-facing half writes against, under one glob: the
/// [`ClientMod`] trait, [`request`] and [`signal`], the declaration trait that
/// puts [`observe`](crate::protocol::SignalSpec::observe) on the payload type an
/// arriving signal carries, [`InputEdge`], the verb modules, and the root types
/// that belong to no realm at all. A client half's first line is this and its
/// second line is its own business.
///
/// ```
/// use ironlark::client::prelude::*;
/// struct Mod;
/// impl ClientMod for Mod {}
/// ```
///
/// That is not a starting point a half outgrows. One that observes a signal in
/// `init`, paints what arrives with [`ui::set_overlay_text`], sounds it with
/// [`audio::play`], keeps what it knows in a [`State`](crate::State) cell and
/// asks its server half for the rest with [`request`] adds no second import
/// for any of it.
///
/// # The prelude is the shape of the realm
///
/// The modules the verbs live in are private, and each realm re-exports the ones
/// it is entitled to. So the import list IS the surface, and what is absent says
/// as much as what is here. There is no
/// [`Entity`](crate::server::Entity), no [`Player`](crate::server::Player), no
/// [`signal_to`](crate::server::signal_to) and no spatial query, because this
/// half decides nothing. It renders what it is told and asks for everything
/// else.
///
/// Read the door as a door, not as a wall. Every realm is a public path, so
/// `ironlark::server::Entity` is a name a client mod can spell and the compiler
/// will accept. What refuses it is the game: a client half is instantiated into
/// a world that offers no entity interface, and a component reaching for one
/// does not load. Importing the prelude and taking what it offers is how that
/// never comes up.
///
/// Nothing here is a verb, so nothing here refuses.
pub mod prelude {
    pub use super::{ClientMod, InputEdge, request, signal};
    pub use super::{audio, resolve, session, ui};
    pub use crate::protocol::SignalSpec;
    pub use crate::{
        Cause, Context, Error, EventId, ProfileId, Quat, Refusal, RequestId, Result, Rgba,
        SessionId, SignalId, SoundBus, SoundId, SourceId, State, Tick, UserId, Vec3, export_client,
        hooks, state,
    };
}
