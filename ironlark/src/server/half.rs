//! The trait the server half of a mod implements. Every hook has an empty
//! default body, so a mod implements only what it answers to. Name-keyed
//! traffic registers on the payload type it travels as rather than widening it.

use super::events::{ContactEdge, LeaveReason, PhysicsObject, Target};
use super::player::Player;
use crate::context::Context;
use crate::math::Vec3;

/// The server half of a mod, as the set of events the host will call it about.
///
/// A type that implements this and is handed to
/// [`export_server!`](crate::export_server) is the mod's server half. Nothing
/// else is registered and nothing else runs, so this trait is the whole of what
/// the host knows how to ask.
///
/// Every hook has an empty default body, so `impl ServerMod for Door {}`
/// compiles and loads. Implement the ones the mod answers to and leave the rest
/// alone. A door mod that places its door in [`init`](ServerMod::init), swings
/// it a little further each step in [`on_tick`](ServerMod::on_tick) and reacts
/// to a press in [`on_interact`](ServerMod::on_interact) implements three, and
/// never sees a join or a leave, because a door does not care who is in the
/// session.
///
/// # The event comes first
///
/// Every hook but [`init`](ServerMod::init) opens with a
/// [`Context`](crate::Context) carrying the event's id, the tick it was raised
/// on and the tick the handler is running on. `init` is the exception because
/// nothing has happened yet, so there is no event to describe. Anything further
/// the host learns to answer about an event arrives as a method on `Context`,
/// so no hook signature here has to widen for it.
///
/// # A hook returns nothing
///
/// A hook has no caller, so an error it produced would have nowhere to go, and
/// `?` therefore has no place in a hook body. The verbs that can refuse answer a
/// [`Result`](crate::Result) at the call site, and what a refusal means is the
/// mod's decision taken there. Match on it, say what happened, and continue or
/// stop:
///
/// ```
/// // `ServerMod`, `Context` and `Player` all arrive with the prelude.
/// use ironlark::server::prelude::*;
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn on_join(_ctx: Context, player: Player) {
///         // `body()` is a live read into the host, so it can refuse.
///         match player.body().await {
///             Ok(_) => log::info!("{player} arrived already holding a body"),
///             Err(e) => log::info!("{player} arrived with no body yet: {e}"),
///         }
///     }
/// }
/// ```
///
/// # The manifest says which of these the half answers
///
/// Implementing a hook is half of the statement. The other half is a
/// `[declares.server]` section in the manifest, whose `hooks` list the host
/// reads before it hands this half anything:
///
/// ```toml
/// [declares.server]
/// hooks = ["on_tick", "on_interact"]
/// ```
///
/// [`init`](ServerMod::init) is never written there. The section existing IS the
/// statement that this half exists and inits, and a mandatory thing is not
/// declared, so a list naming it fails the build.
///
/// Two lanes are decided by that list alone, because nothing else routes them.
/// One is the tick. A half that leaves [`on_tick`](ServerMod::on_tick) off is
/// never woken for a step and never pays for one. The other is the join and the
/// leave, which answer to one declaration between them. Naming either admits
/// both, because a leave may cancel a join the mod has not yet been handed and
/// a pair must stay a pair. A half naming neither is woken for neither.
///
/// The rest are decided by what the mod declares elsewhere. A press reaches
/// [`on_interact`](ServerMod::on_interact) and a touch reaches
/// [`on_contact`](ServerMod::on_contact) because an archetype of this mod's
/// carries `interact = true` or `contact = true`, so those two arrive on the
/// strength of the archetype rather than the hook list, and the empty default
/// body is what absorbs one a mod did not want. `init` runs whatever the list
/// says.
///
/// A half with no section at all is a half that named nothing: it inits, it is
/// woken for no step and no arrival, and only the two archetype-routed hooks
/// still reach it. A half that wants a step says so.
///
/// The host writes one line per half at load, naming every hook it will call
/// and why each of the rest stays silent. Writing
/// [`#[ironlark::hooks("../mod.toml")]`](macro@crate::hooks) on the impl block
/// is what makes that line true. It reads the manifest while the crate compiles
/// and fails the build in both directions, on a hook implemented here and
/// unlisted there, and on one listed there and not implemented here, so a half
/// the line does not describe cannot ship.
///
/// # What is deliberately absent
///
/// Anything that arrives under a declared name. A request a client half asked
/// and a signal another mod raised are both routed by name, and each registers
/// on its own payload type — with
/// [`respond`](crate::protocol::RequestSpec::respond) and
/// [`observe`](crate::protocol::SignalSpec::observe) — inside `init` instead.
/// That is why teaching a mod one more request adds a line to `init` rather than
/// a hook here, and why two mods that declare different names implement the same
/// trait.
///
/// The contract also exports a set of structural hooks, among them a body
/// created, a map loaded, a profile hot-swapped and a body spawn requested. They
/// are declared ahead of the mechanism that will fire them, and this crate
/// answers every one of them with `allow` from a blanket impl over every
/// `ServerMod`. They are not on this trait and an author cannot implement one.
/// Nothing fires them, so there is nothing to observe.
#[allow(async_fn_in_trait)]
pub trait ServerMod {
    /// Runs once when this half loads, before it is handed any event. Answer
    /// the mod's requests with
    /// [`respond`](crate::protocol::RequestSpec::respond) and hear its signals
    /// with [`observe`](crate::protocol::SignalSpec::observe) here, and spawn
    /// whatever content the mod places.
    ///
    /// The one hook with no context, because nothing has happened yet and there
    /// is no event to describe. It runs whether or not the manifest's hook list
    /// names it.
    async fn init() {}
    /// A participant joined the session.
    ///
    /// Admitted only when the manifest's hook list names this hook or
    /// [`on_leave`](ServerMod::on_leave). One declaration covers the pair.
    async fn on_join(_ctx: Context, _player: Player) {}
    /// A participant left, and why.
    ///
    /// Admitted on the same declaration as [`on_join`](ServerMod::on_join).
    /// [`LeaveReason`] is the why.
    async fn on_leave(_ctx: Context, _player: Player, _reason: LeaveReason) {}
    /// The step, with the seconds it covers.
    ///
    /// [`Context::raised_at`](crate::Context::raised_at) carries the step's
    /// number. A half that does not name this hook in its manifest is never
    /// woken for a step. A step missed under load is replaced by the current
    /// one rather than accumulated, so `dt` covers the step it arrived with and
    /// no more.
    async fn on_tick(_ctx: Context, _dt: f32) {}
    /// A participant pressed something this mod owns.
    ///
    /// Arrives because an archetype of this mod's declares `interact = true`,
    /// not because the hook list names it. [`Target`] says which instance,
    /// `_hit_point` is where the aim met it, and `_distance` is how far the
    /// presser stood.
    async fn on_interact(
        _ctx: Context,
        _player: Player,
        _target: Target,
        _hit_point: Vec3,
        _distance: f32,
    ) {
    }
    /// Two things started or stopped touching.
    ///
    /// Arrives because an archetype of this mod's declares `contact = true`,
    /// not because the hook list names it. [`Target`] says which instance of
    /// this mod's, [`PhysicsObject`] says what touched it, and [`ContactEdge`]
    /// says which side of the touch this is.
    async fn on_contact(
        _ctx: Context,
        _target: Target,
        _other: PhysicsObject,
        _point: Vec3,
        _edge: ContactEdge,
    ) {
    }
}
