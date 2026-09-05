//! The announcement's declaration, and the two raises that read it.

use crate::context::Context;
#[cfg(target_arch = "wasm32")]
use crate::error::Error;
use crate::error::Result;
use crate::ids::{SessionId, SignalId, SourceId};
use crate::protocol::{Audience, Transit, encode, resolved};
use core::future::Future;
use prost::Message;

/// A declared signal: one name, one payload type, who hears it and how the
/// host is to carry it.
///
/// [`protocol!`](macro@crate::protocol) writes this impl on the payload type
/// itself, from a message carrying `option (ironlark.signal)` in the mod's
/// `protocol.proto`. The declaration rides on the type that travels, so the
/// value an author constructs is the whole of what a verb needs.
///
/// A raise announces and stops. Whoever subscribed to the name hears it, the
/// raiser included when it subscribed; there is no addressee, and no answer
/// comes back. [`Audience`](Self::Audience) is the whole of where it goes.
///
/// Both realms raise. [`server::signal`](crate::server::signal) puts one on
/// the server realm's bus or across to the client realms;
/// [`client::signal`](crate::client::signal) puts one on the client bus of the
/// machine it runs on. [`server::signal_to`](crate::server::signal_to) is the
/// same announcement narrowed to one participant's machine, and
/// [`observe`](Self::observe) — a method on this type, in either realm — is the
/// receiving end.
///
/// An author writes none of this. What an author writes is the schema, and then
/// the payload value: every verb takes that value and reads the declaration off
/// it.
///
/// ```
/// use ironlark::protocol::{Clients, SignalSpec};
/// use ironlark::server::signal;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` as a mod
/// // writes it, over the door schema:
/// //
/// //     message Latch {
/// //       option (ironlark.signal) = CLIENTS;
/// //       bool open = 1;
/// //     }
/// //
/// // Out comes `protocol::Latch`, carrying the impl below.
/// assert_eq!(<protocol::Latch as SignalSpec>::NAME, "latch");
///
/// // The audience is a type, which is what a bound can read.
/// fn crossing<S: SignalSpec<Audience = Clients>>() {}
/// crossing::<protocol::Latch>();
///
/// // The author's own line: the payload alone, and `signal` finds the rest.
/// fn tell_every_screen(open: bool) {
///     if let Err(e) = signal(&protocol::Latch { open }) {
///         log::warn!("nobody was told the door moved: {e}");
///     }
/// }
/// ```
///
/// # What refuses
///
/// Nothing on this trait but [`resolved`](Self::resolved), which answers
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) when the session's
/// schemas do not carry [`NAME`](Self::NAME), or carry it in the other realm.
/// [`observe`](Self::observe) meets that same refusal and has no caller to hand
/// it to, so it writes it; its own page says what that costs. Every other
/// refusal belongs to the verb that read the declaration, and
/// [`server::signal`](crate::server::signal) names them.
pub trait SignalSpec: Message + Default {
    /// Who hears it, as one of [`ServerMods`](super::ServerMods),
    /// [`ClientMods`](super::ClientMods) or [`Clients`](super::Clients).
    type Audience: Audience;

    /// The declared name, as a manifest and a qualified id spell it: the proto
    /// message name with a word boundary at each capital, so `PlayerPositions`
    /// declares `player-positions`.
    const NAME: &'static str;

    /// How the host is to carry it. See [`Transit`].
    const TRANSIT: Transit;

    /// The session's number for this name, resolved on first use and kept.
    ///
    /// Provided, because the number is the session's rather than the
    /// declaration's: the generated impl states the facts a schema holds and
    /// this reads the session. Every raise after a name's first pays a table
    /// lookup instead of a crossing into the host.
    fn resolved() -> Result<SignalId> {
        resolved::signal(Self::NAME)
    }

    /// Hears this signal, in either realm: the handler runs whenever the name
    /// this type declares is raised.
    ///
    /// The registration opens with the payload type, so the line names the
    /// signal and nothing else has to. `handler` is given the event, the
    /// raiser's host-stamped [`SourceId`], and the payload already decoded into
    /// `Self`. There is no table to build and nothing to install afterwards:
    /// the call resolves the declared name, tells the host this half wants it,
    /// and the handler is live when the call returns.
    ///
    /// A closure needs no type annotations, because `Self` settles the third
    /// argument before the body is read:
    ///
    /// ```
    /// use ironlark::client::prelude::*;
    /// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
    /// // A real crate writes `mod protocol;` here. The hidden line is what the
    /// // generator wrote for
    /// //
    /// //     message Latch { option (ironlark.signal) = CLIENTS; bool open = 1; }
    /// //
    /// // in the mod's protocol.proto: the payload type, and the `SignalSpec`
    /// // impl carrying the declared name `latch`. `ClientMod`, `Context`,
    /// // `SourceId`, `SignalSpec` and `ui` are the prelude's.
    /// use protocol::Latch;
    ///
    /// struct Door;
    ///
    /// impl ClientMod for Door {
    ///     async fn init() {
    ///         // A named handler, which is what a half of any size writes.
    ///         Latch::observe(shown);
    ///     }
    /// }
    ///
    /// async fn shown(_ctx: Context, _from: SourceId, latch: Latch) {
    ///     ui::set_overlay_text(if latch.open { "door: open" } else { "door: shut" });
    /// }
    ///
    /// // The same registration as a closure: `latch` is a `Latch` because
    /// // `Latch::observe` says so.
    /// Latch::observe(|_ctx, _from, latch| async move {
    ///     ui::set_overlay_text(if latch.open { "open" } else { "shut" });
    /// });
    /// ```
    ///
    /// One registration covers every route the name can arrive by. A raise on
    /// this machine's own bus and one that crossed the network from a server
    /// half reach a half through the same export, so there is no second thing
    /// to register for the crossing case. Which realm hears the name at all is
    /// the declaration's [`Audience`](Self::Audience), not this call's.
    ///
    /// A borrowed signal is observed the same way. The type another mod's
    /// schema declares arrives under its owner, `protocol::acme_doors::Latch`,
    /// carrying the qualified name the host resolves it by, and `observe` on it
    /// reads like `observe` on this mod's own.
    ///
    /// # When to register, and what a second one does
    ///
    /// [`init`](crate::server::ServerMod::init) is the usual place, and it is
    /// not the only legal one. A subscription is idempotent and a handler may
    /// be registered at any moment a mod is running, so a half that starts
    /// listening once a round begins is a mod written the way it reads.
    ///
    /// Registering the same signal again REPLACES the handler and writes a
    /// `debug` line saying so. That is how a half swaps behaviour without a
    /// branch inside one handler; it is also what a half that registers in two
    /// places gets, and the log line is where that shows up.
    ///
    /// # What refuses
    ///
    /// Nothing, because there is nobody to refuse to: registering answers no
    /// [`Result`](crate::Result). What can go wrong is the name — one no
    /// enabled mod declares, or one declared for the other realm — and that
    /// arrives as an `error` line naming the signal, written at this call. The
    /// handler is not installed and the signal is simply never heard.
    ///
    /// The payload is the other thing that can be wrong, and it is wrong per
    /// arrival rather than per registration. Bytes that do not decode as `Self`
    /// are dropped with a `warn` naming the signal, so one raiser's bad payload
    /// costs this handler one delivery and nothing else. A signal arriving with
    /// no handler at all is warned about once per name and raiser, never once
    /// per raise.
    fn observe<F, Fut>(handler: F)
    where
        F: Fn(Context, SourceId, Self) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        crate::host::dispatch::observe::<Self, F, Fut>(handler);
    }
}

/// Resolve first so a name this session does not carry costs no encode, then
/// encode once into one exactly sized buffer and cross once.
pub(crate) fn signal<P: SignalSpec>(raised: &P) -> Result<()> {
    let id = P::resolved()?;
    let bytes = encode(raised);
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::server::ironlark::host::signal::signal(id.to_wire(), &bytes)
            .map_err(|e| Error::from_wire(e.code, e.message, e.data))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        crate::testing::signal::<P>(None, &bytes)
    }
}

pub(crate) fn signal_to<P: SignalSpec>(to: SessionId, raised: &P) -> Result<()> {
    let id = P::resolved()?;
    let bytes = encode(raised);
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::server::ironlark::host::signal_to::signal_to(
            to.to_wire(),
            id.to_wire(),
            &bytes,
        )
        .map_err(|e| Error::from_wire(e.code, e.message, e.data))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        crate::testing::signal::<P>(Some(to), &bytes)
    }
}

/// Told to the host as a handler registers; subscriptions die with the mod and
/// a repeat is idempotent.
#[cfg(target_arch = "wasm32")]
pub(crate) fn subscribe(id: SignalId, name: &str) -> Result<()> {
    crate::bindings::server::ironlark::host::signal::subscribe(id.to_wire())
        .map_err(|e| Error::from_wire(e.code, format!("subscribing {name}: {}", e.message), e.data))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn subscribe(_id: SignalId, _name: &str) -> Result<()> {
    Ok(())
}
