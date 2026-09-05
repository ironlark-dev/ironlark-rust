//! Who hears a signal, as the three types a declaration can name.

/// Who hears a signal: the one fact every signal declaration answers.
///
/// A signal has no addressee. Where it lands and whether it leaves the machine
/// are one fact with three values, and an author writes it as the single token
/// of `option (ironlark.signal)` in the mod's `protocol.proto`. It reaches Rust
/// as [`SignalSpec::Audience`](super::SignalSpec::Audience), written by
/// [`protocol!`](macro@crate::protocol).
///
/// It is a TYPE rather than a value, because a bound reads a type and cannot
/// read a constant. [`signal_to`](crate::server::signal_to)
/// asks for [`Clients`] and nothing else, so narrowing a raise that never
/// leaves a bus is a compile error rather than a refusal met once a session is
/// running. [`client::signal`](crate::client::signal) asks for [`ClientMods`]
/// the same way.
///
/// The trait is sealed. The three types below are the whole set, because the
/// schema's own enum is closed and a fourth would be a platform change rather
/// than something a mod can declare.
///
/// ```
/// use ironlark::protocol::{Clients, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line above is `ironlark::protocol!("../protocol.proto")` as a
/// // real mod writes it, over the door schema whose `message Latch` carries
/// // `option (ironlark.signal) = CLIENTS`.
///
/// // A helper that will only take a signal that crosses the network, written
/// // with the same bound the narrowed raise uses.
/// fn only_crossing<S: SignalSpec<Audience = Clients>>() -> &'static str {
///     S::NAME
/// }
///
/// assert_eq!(only_crossing::<protocol::Latch>(), "latch");
/// ```
///
/// # Nothing refuses
///
/// These are types, and the one place they are read is a bound. What a wrong
/// audience gets is a compile error at the verb, and that verb's page says so.
pub trait Audience: sealed::Sealed {}

/// The audience that stays on the server realm's bus: one mod's server half
/// announcing to the other server halves in the same process.
///
/// It is one of the three markers [`Audience`] admits, and a marker is a type a
/// declaration names rather than a value a call site passes. An author writes it
/// as `option (ironlark.signal) = SERVER_MODS;` on the message in the mod's
/// `protocol.proto`, and [`protocol!`](macro@crate::protocol) turns that line
/// into `type Audience = ServerMods` on the payload type's
/// [`SignalSpec`](super::SignalSpec) impl. The option line is what declares a
/// signal at all, so it is always written, value included.
///
/// The raise never leaves the machine and never reaches a screen. What it is for
/// is one mod telling the others that something happened in the world the server
/// owns — a door opened, a round ended — so a second mod can act on it without
/// either mod importing the other's code. A client half is told by a separate
/// raise carrying [`Clients`].
///
/// [`server::signal`](crate::server::signal) is the verb. There is no narrowed
/// form: [`signal_to`](crate::server::signal_to) addresses one participant's
/// machine, and this announcement reaches no machine, so that verb's bound asks
/// for [`Clients`] and this marker fails to satisfy it while the mod compiles.
/// [`observe`](super::SignalSpec::observe) in a server half is the receiving
/// end.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::protocol::{ServerMods, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it, over the door schema:
/// //
/// //     message Opened {
/// //       option (ironlark.signal) = SERVER_MODS;
/// //       string name = 1;
/// //     }
/// //
/// // Out comes `protocol::Opened`, carrying this marker. `signal`, `Context`
/// // and `Player` are the server prelude's.
///
/// // The same shape a verb's bound is written in, which is the whole reason
/// // the audience is a type: this is resolved by the compiler.
/// fn on_the_server_bus<S: SignalSpec<Audience = ServerMods>>() -> &'static str {
///     S::NAME
/// }
/// assert_eq!(on_the_server_bus::<protocol::Opened>(), "opened");
///
/// // The author's own line. The payload is the whole argument, and the verb
/// // reads the audience off it.
/// fn say_it_opened(name: String) {
///     if let Err(e) = signal(&protocol::Opened { name }) {
///         log::warn!("no other server half heard the door: {e}");
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// It is a type, and the one place it is read is a bound, so a wrong audience is
/// a compile error at the verb rather than a refusal met mid-session. What can
/// refuse belongs to two other places. A client half that resolves this name is
/// turned down by the host, because the client realm neither raises nor hears a
/// server-bus signal. And a declaration pairing this marker with
/// `option (ironlark.transit.keep) = NEWEST;` is refused as declared and not
/// served: supersession is carried by the network path, which a bus signal never
/// takes. Both arrive the first time the name is resolved, which is the
/// [`observe`](super::SignalSpec::observe) for a name this half hears and the
/// first raise otherwise, rather than partway through a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServerMods;

/// The audience that stays on the client bus of the machine that raised it: one
/// client half announcing to the other client halves running beside it.
///
/// It is one of the three markers [`Audience`] admits, and a marker is a type a
/// declaration names rather than a value a call site passes. An author writes it
/// as `option (ironlark.signal) = CLIENT_MODS;` on the message in the mod's
/// `protocol.proto`, and [`protocol!`](macro@crate::protocol) turns that line
/// into `type Audience = ClientMods` on the payload type's
/// [`SignalSpec`](super::SignalSpec) impl.
///
/// It is the one audience a client half may raise. The announcement reaches the
/// client halves on this one machine that observed the name and goes no further:
/// no other participant hears it, and no server half does. What it is for is two
/// client halves that should not name each other, an overlay painting what a
/// gameplay mod noticed, without either taking the other as a dependency.
///
/// [`client::signal`](crate::client::signal) is the verb, and its bound is this
/// marker, so a payload declared for any other audience fails to compile there.
/// [`observe`](super::SignalSpec::observe) in a client half is the receiving
/// end. Reaching another participant's machine is the authority's act
/// and is declared [`Clients`] instead.
///
/// ```
/// use ironlark::client::prelude::*;
/// use ironlark::protocol::{ClientMods, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it, over the door schema:
/// //
/// //     message Aimed {
/// //       option (ironlark.signal) = CLIENT_MODS;
/// //       string at = 1;
/// //     }
/// //
/// // Out comes `protocol::Aimed`, carrying this marker. `signal` is the client
/// // prelude's, and its bound is the one written below.
///
/// // The verb's own bound, written out. The compiler answers it.
/// fn stays_on_this_machine<S: SignalSpec<Audience = ClientMods>>() -> &'static str {
///     S::NAME
/// }
/// assert_eq!(stays_on_this_machine::<protocol::Aimed>(), "aimed");
///
/// // What the author writes: the payload, and nothing about where it goes.
/// fn tell_the_overlay(at: String) {
///     if let Err(e) = signal(&protocol::Aimed { at }) {
///         log::warn!("nothing on this machine heard it: {e}");
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// It is a type, and the one place it is read is a bound, so a wrong audience is
/// a compile error at the verb. Two refusals sit elsewhere. A server half is not
/// stopped by the compiler from handing one of these to
/// [`server::signal`](crate::server::signal), whose bound is open, and the host
/// refuses it as [`Refused`](crate::ErrorKind::Refused) at the first resolve,
/// naming the realm that does raise it. And a declaration pairing this marker
/// with `option (ironlark.transit.keep) = NEWEST;` is refused as declared and
/// not served, because supersession is carried by the network path and this
/// announcement never takes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientMods;

/// The audience that crosses the network: the server half announcing to every
/// participant's client half.
///
/// It is one of the three markers [`Audience`] admits, and a marker is a type a
/// declaration names rather than a value a call site passes. An author writes it
/// as `option (ironlark.signal) = CLIENTS;` on the message in the mod's
/// `protocol.proto`, and [`protocol!`](macro@crate::protocol) turns that line
/// into `type Audience = Clients` on the payload type's
/// [`SignalSpec`](super::SignalSpec) impl.
///
/// The authority raises it and the screens hear it. This is how a mod's server
/// half tells its own client half, and every other mod's client half that
/// observed the name, what the world now looks like. The two halves of one mod
/// compile against the same `protocol.proto`, so the type at both ends is one
/// type.
///
/// Two verbs take it, and they are one announcement differing in reach.
/// [`server::signal`](crate::server::signal) reaches every participant.
/// [`signal_to`](crate::server::signal_to) narrows the same payload to one
/// participant's machine, which is what a joiner needs, and its bound asks for
/// this marker alone. That bound is the mechanism the marker exists for: a
/// narrowed raise of an announcement that never leaves a bus would be
/// meaningless, so it is a compile error rather than a refusal met once a
/// session is running.
///
/// It is also the one audience that may declare supersession. A message adding
/// `option (ironlark.transit.keep) = NEWEST;` tells the host a newer raise may
/// replace an older one still in flight, which is what high-rate state wants,
/// and the host serves that only on the network path this audience takes. The
/// payload cap is the smaller one there, because such a raise rides a path
/// where one message cannot be split, and a payload over it is refused at the
/// raise as [`TooLarge`](crate::ErrorKind::TooLarge) naming the cap it was
/// measured against.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::protocol::{Clients, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it, over the door schema:
/// //
/// //     message Latch {
/// //       option (ironlark.signal) = CLIENTS;
/// //       bool open = 1;
/// //     }
/// //
/// // Out comes `protocol::Latch`, carrying this marker. `signal`, `signal_to`,
/// // `SessionId`, `Context`, `Player` and `ServerMod` are the server prelude's.
///
/// // The bound `signal_to` is declared with, written out here.
/// fn crosses_the_network<S: SignalSpec<Audience = Clients>>() -> &'static str {
///     S::NAME
/// }
/// assert_eq!(crosses_the_network::<protocol::Latch>(), "latch");
///
/// struct Door;
///
/// impl ServerMod for Door {
///     async fn on_join(_ctx: Context, player: Player) {
///         // The arrival missed every raise so far, so it is told alone.
///         if let Err(e) = signal_to(player.session(), &protocol::Latch { open: false }) {
///             log::warn!("the joiner was not told the door state: {e}");
///         }
///     }
/// }
///
/// // The same payload, and now every screen in the session.
/// fn tell_everyone(open: bool) {
///     if let Err(e) = signal(&protocol::Latch { open }) {
///         log::warn!("no screen was told the door moved: {e}");
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// It is a type, and the one place it is read is a bound, so a wrong audience is
/// a compile error at the verb. A client half handing one of these to
/// [`client::signal`](crate::client::signal) is the case that bound catches:
/// crossing the network is the authority's act, and a client half has no verb
/// that spells it. Every run-time refusal a raise can meet belongs to the verb
/// that read this marker, and [`server::signal`](crate::server::signal) names
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clients;

impl Audience for ServerMods {}
impl Audience for ClientMods {}
impl Audience for Clients {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ServerMods {}
    impl Sealed for super::ClientMods {}
    impl Sealed for super::Clients {}
}
