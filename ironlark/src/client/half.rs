//! The trait the client half of a mod implements. Both hooks have empty
//! default bodies, and name-keyed traffic registers on the payload type it
//! travels as rather than widening it.

use crate::context::Context;

/// The client half of a mod, as the set of events the host will call it about.
///
/// A type that implements this and is handed to
/// [`export_client!`](crate::export_client) is the mod's client half, and one
/// instance of it runs on every player's machine. Both hooks have empty default
/// bodies, so `impl ClientMod for Door {}` compiles. The useful minimum is
/// [`init`](ClientMod::init).
///
/// Two hooks is the whole trait because nearly everything a client half does
/// arrives under a name rather than as an event. A door mod's client half
/// implements `init` alone, writing its overlay line and then registering a
/// handler for the signal its server half raises. That registration goes on the
/// signal's own payload type, with
/// [`observe`](crate::protocol::SignalSpec::observe), which is where a client
/// half's real surface is. The key that asks the door to open reaches an author hook
/// instead: a function of the mod's own, declared in the manifest and lifted out
/// of this same block by the [`hooks`](macro@crate::hooks) attribute.
///
/// # The event comes first
///
/// [`on_tick`](ClientMod::on_tick) opens with a [`Context`](crate::Context)
/// carrying the event's id and the two ticks, and so does every registered
/// handler. [`init`](ClientMod::init) is the exception, for the same reason as
/// the server half's. Nothing has happened on this machine yet.
///
/// # A hook returns nothing
///
/// As on the server half, there is no caller for a hook and so no `?` in one.
/// The verb a client half calls that can refuse across the network is
/// [`request`](crate::client::request). It answers a [`Result`](crate::Result),
/// and the mod decides at that call site what a refusal means.
///
/// # The tick reaches a half that declared it
///
/// One step is raised for both halves of every mod at once, so the client tick
/// carries the same step number and the same `dt` the server half is handed.
/// This side is entered for it only when the manifest says the half implements
/// it, under `[declares.client]`:
///
/// ```toml
/// [declares.client]
/// hooks = ["on_tick"]
/// ```
///
/// A half that leaves the hook off that list is never entered for a step and
/// never pays for one. A client half with nothing to integrate over time wants
/// exactly that, because its work is a message arriving and a key going down.
///
/// A client half is served its messages and its input edges before its tick, so
/// a burst of traffic is drained before the step it belongs to. A tick held
/// behind a long enough run of that traffic jumps the queue rather than being
/// starved.
#[allow(async_fn_in_trait)]
pub trait ClientMod {
    /// Runs once on this machine when the half loads, before it is handed
    /// anything. Hear the signals this half listens for with
    /// [`observe`](crate::protocol::SignalSpec::observe) on each payload type
    /// here.
    ///
    /// The one hook with no context, for the same reason as the server half's.
    /// It runs whether or not the manifest's hook list names it.
    async fn init() {}
    /// The step, with the seconds it covers.
    ///
    /// [`Context::raised_at`](crate::Context::raised_at) carries the step's
    /// number. A half that does not name this hook in its manifest is never
    /// woken for a step. A step missed under load is replaced by the current
    /// one rather than accumulated, so `dt` covers the step it arrived with and
    /// no more.
    async fn on_tick(_ctx: Context, _dt: f32) {}

    /// Where an input edge enters this half, filled by
    /// [`#[ironlark::hooks]`](macro@crate::hooks) rather than by an author.
    ///
    /// An author hook is a function of the author's own, named in the manifest
    /// with the rule that invokes it. The attribute lifts those functions out
    /// of the impl block and writes this method to enter them by the id the
    /// host dispatched, so nothing about a hook is registered at run time. A
    /// half with no author hooks keeps the empty body.
    #[doc(hidden)]
    async fn on_input(_ctx: Context, _hook: u32, _edge: super::input::InputEdge) {}
}
