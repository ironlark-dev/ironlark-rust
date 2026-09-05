//! The typed shapes the host delivers alongside a participant or an entity
//! event in the server realm.

use super::entity::Entity;

/// How a participant reached this session.
///
/// One node handing a participant to another is a handover, and this is the
/// case that tells a handover from a plain connection.
/// [`Fresh`](Arrival::Fresh) is somebody connecting to this server.
/// [`Migrated`](Arrival::Migrated) carries the opaque bytes the node they came
/// from held for them.
///
/// Server realm, and the server realm's prelude carries it.
///
/// # What is not built
///
/// No hook takes one. The arrival hook on
/// [`ServerMod`](crate::server::ServerMod) receives the event and the
/// [`Player`](crate::server::Player) and nothing else, because no node hands a
/// participant to another and a parameter whose value is always
/// [`Fresh`](Arrival::Fresh) tells an author nothing.
///
/// The case is frozen in the contract regardless, and the host fills it in on
/// every arrival, because a variant that gains a case fails every already-built
/// mod at load. Widening the hook to carry it is a change to this crate alone.
///
/// ```
/// // The prelude mints Arrival.
/// use ironlark::server::prelude::*;
///
/// // A hook is not handed one, so a mod that will branch on it writes the
/// // match against a value of its own until the contract carries it across.
/// fn greet(arrival: &Arrival) -> &'static str {
///     match arrival {
///         Arrival::Fresh => "welcome",
///         Arrival::Migrated(_) => "welcome back",
///         // The set may grow, so a match on it carries this arm.
///         _ => "welcome",
///     }
/// }
///
/// assert_eq!(greet(&Arrival::Fresh), "welcome");
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub enum Arrival {
    /// Connected to this server.
    Fresh,
    /// Handed over from another node, carrying its opaque state.
    Migrated(Vec<u8>),
}

/// Why a participant left, delivered with the departure hook.
///
/// The departure hook on [`ServerMod`](crate::server::ServerMod) is where a mod
/// forgets somebody it was tracking. A mod holding a row per participant drops
/// that row and raises the list again. A mod holding a set of
/// [`SessionId`](crate::SessionId) values drops the number. Both ignore the
/// reason, which is the ordinary case. Leaving is what matters. Why is
/// decoration on top of it.
///
/// Server realm, because only the server sees a participation end. A hook
/// receives this, so there is nothing here to refuse.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `SessionId`, `LeaveReason`, `Player` and `ServerMod` all arrive with
/// // the prelude imported above.
/// ironlark::state! {
///     static PRESENT: Vec<SessionId> = Vec::new();
/// }
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn on_leave(_ctx: Context, player: Player, reason: LeaveReason) {
///         // The number is what was kept. The handle is not keepable.
///         let id = player.session();
///         PRESENT.update(|present| present.retain(|held| *held != id));
///         log::info!("{player} is gone ({reason:?})");
///     }
/// }
/// ```
///
/// # What is not built
///
/// Every departure the host observes is reported as [`Quit`](LeaveReason::Quit),
/// whatever brought it about. A connection ending is the whole of what the host
/// watches, so somebody closing the game and a connection that stopped
/// answering reach the hook the same way.
/// [`Kicked`](LeaveReason::Kicked) and
/// [`Timeout`](LeaveReason::Timeout) are contract cases the host does not tell
/// apart, so a mod that branches on one branches on something it is not handed.
///
/// They exist because the shape is the expensive part to change. The host can
/// learn to tell a removal from a dropped connection with no mod rebuilt for
/// it.
///
/// # What the catch-all arm can and cannot catch
///
/// The type is `#[non_exhaustive]`, so a `match` over it needs an arm for a
/// case that does not exist. Nothing reaches that arm. This mirrors a contract
/// variant, and a variant's case set cannot grow without failing every compiled
/// mod at load, so on the day it does grow every mod is rebuilt anyway. Written
/// this way and known to be so.
#[non_exhaustive]
#[derive(Debug)]
pub enum LeaveReason {
    /// Left of their own accord.
    Quit,
    /// Removed by an operator or a gamemode.
    Kicked,
    /// The connection stopped answering.
    Timeout,
}

/// Which of this mod's instances an event was about.
///
/// A press and a touch edge reach a mod because the instance's archetype is one
/// that mod declared in its manifest, whoever spawned the instance. This is how
/// the handler learns WHICH instance, and it answers twice over. There is a
/// handle to act through now, and there is this mod's own id for the instance
/// to act through later.
///
/// Checking which instance arrived is not ceremony. One archetype can have many
/// live instances and they all route to the same handler, and a mod that
/// declares two archetypes gets both through that same handler as well.
///
/// Server realm, because a press and a touch are decided where the world is.
///
/// # The two fields answer different questions
///
/// [`entity`](Target::entity) is a handle the host lends for this event alone.
/// It reaches the instance with no round-trip, and it stops answering when the
/// handler returns. A copy kept in a [`State`](crate::State) cell refuses
/// afterwards as [`StaleId`](crate::ErrorKind::StaleId) rather than resolving
/// to whatever took its place. Use it inside the handler and let it go.
///
/// [`id`](Target::id) is this mod's own id for the instance, the one
/// [`Entity::set_id`](crate::server::Entity::set_id) stamped on it. An id costs
/// a lookup and outlives everything, so it is what a handler keeps when it must
/// act on the instance on some later tick. Hand it to
/// [`Entity::by_id`](crate::server::Entity::by_id) for that one instance, or
/// use it as a `/`-prefixed family path for [`find`](crate::server::find).
///
/// # Example
///
/// A mod that owns doors, acting on one of them. Every type named below arrives
/// with the server realm's prelude on the first line. The archetype that makes
/// a door pressable, and the hook list that admits the press, come from the
/// manifest, as the comments mark.
///
/// ```
/// use ironlark::server::prelude::*;
/// // A real crate has one more line here:
/// //
/// //     #[ironlark::hooks("../mod.toml")]
/// //
/// // on the impl block below, which fails the build if the manifest and the
/// // implemented hooks disagree.
/// //
/// // The press arrives at all because mod.toml, beside the crate, declares
/// // the archetype and marks it pressable:
/// //
/// //     [[declares.archetype]]
/// //     id = "door"
/// //     shape = { kind = "box", size = [1.0, 2.0, 0.2] }
/// //     interact = true
/// //
/// //     [declares.server]
/// //     hooks = ["on_interact"]
/// //
/// // and the id below is one this mod stamped on the instance itself with
/// // `Entity::set_id` when it spawned it.
/// const FRONT: &str = "front";
///
/// struct Doors;
///
/// impl ServerMod for Doors {
///     async fn on_interact(
///         _ctx: Context,
///         player: Player,
///         target: Target,
///         _hit_point: Vec3,
///         distance: f32,
///     ) {
///         let Some(id) = target.id else {
///             return;
///         };
///         if id != FRONT {
///             return;
///         }
///         log::info!("{player} opened the front door from {distance:.2}m");
///     }
/// }
/// ```
pub struct Target {
    /// The entity the event was about, reachable with no round-trip.
    ///
    /// Present on every event the host delivers, including one whose instance
    /// died between the raise and the handler. What refuses then is the verb,
    /// not this. The option is the contract's rather than a fact about the
    /// host, so a mod that handles the absent case keeps compiling if the host
    /// ever withholds a handle.
    ///
    /// It is lent, like a [`Player`](crate::server::Player) is. Act through it
    /// inside the handler and keep [`id`](Target::id) for anything later.
    pub entity: Option<Entity>,
    /// This mod's own id for the instance, absent when this mod never gave it
    /// one.
    ///
    /// Absent is ordinary. A mod that spawns a hundred walls and addresses none
    /// of them gets a hundred anonymous targets, and they are not one identity.
    /// An id that IS present feeds
    /// [`Entity::by_id`](crate::server::Entity::by_id) and parses as a `/`
    /// family path for [`find`](crate::server::find).
    ///
    /// The host never invents one from the archetype, because two anonymous
    /// instances of one archetype would then share an identity.
    ///
    /// # Absent does not mean it stays absent
    ///
    /// Spawning and setting the id are two round-trips, and the instance
    /// carries the proxies that report presses and touches from the first one.
    /// An event raised in the gap reaches the handler with no id, and every
    /// event after it carries one. A mod cannot close that window, because the
    /// id goes on through the very handle the spawn is still returning.
    /// Treating an absent id as "not mine" therefore drops a real event.
    ///
    /// A mod that owns exactly one archetype should read an absent id as its
    /// own. A mod counting the two edges of a touch has to, because the host
    /// looks the id up per edge and a touch that begins before the id lands
    /// ends after it. A mod that owns two archetypes cannot, and the event
    /// carries nothing else to ask, so such a mod gives every instance it
    /// spawns an id.
    pub id: Option<String>,
}

/// What a mod's entity touched, or was touched by.
///
/// The touch hook on [`ServerMod`](crate::server::ServerMod) receives one with
/// every edge, and it is the whole answer about the other side. A participant's
/// body, another mod's entity, or the map. The `_` arm a `#[non_exhaustive]`
/// match needs is dead for the reason given on [`LeaveReason`].
///
/// Which case arrives also decides the event's cause. A body makes the touch
/// [`Cause::Player`](crate::Cause::Player), a person's doing. Anything else
/// makes it
/// [`Cause::Engine`](crate::Cause::Engine), the physics rather than somebody.
///
/// Server realm. A hook receives this, so there is nothing here to refuse.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `Context`, `PhysicsObject`, `ContactEdge`, `Target`, `Vec3` and
/// // `ServerMod` all arrive with the prelude imported above. The touch
/// // reaches this mod at all because mod.toml, beside the crate, declares an
/// // archetype with `contact = true` under `[[declares.archetype]]`.
/// struct Threshold;
///
/// impl ServerMod for Threshold {
///     async fn on_contact(
///         _ctx: Context,
///         _target: Target,
///         other: PhysicsObject,
///         _point: Vec3,
///         _edge: ContactEdge,
///     ) {
///         match other {
///             PhysicsObject::Player(who) => log::info!("{who} stepped into the doorway"),
///             PhysicsObject::Entity(Some(id)) => log::info!("{id} came to rest in it"),
///             PhysicsObject::Entity(None) => log::info!("something anonymous came to rest in it"),
///             _ => {}
///         }
///     }
/// }
/// ```
#[non_exhaustive]
pub enum PhysicsObject {
    /// A participant's body, as the [`SessionId`](crate::SessionId) that
    /// addresses them. The body itself is reached with
    /// [`body_of`](crate::server::body_of), which is what carries the standing
    /// to act on it.
    Player(crate::ids::SessionId),
    /// Another mod's entity, carrying the full id its owner knows it by when
    /// that owner gave the instance one.
    ///
    /// Full means the owning mod's own id and the local one together, because a
    /// local id alone means nothing outside the mod that minted it. It is not
    /// what [`Target::id`] carries, which is this mod's own local id for its own
    /// instance.
    ///
    /// Absent covers both ways a thing arrives anonymous. No enabled archetype
    /// owns it, or its owner never gave that instance an id. Neither is an error
    /// and neither is comparable, so match on whether an id is there rather than
    /// on its text.
    Entity(Option<String>),
    /// The map's own geometry.
    MapGeometry,
}

/// Which side of a touch interval this contact event marks.
///
/// Only the two boundary crossings reach a mod. Per-frame contact data, its
/// depth and its impulse and the pair still being in the graph this tick, never
/// does. A mod that wants to know whether anything is standing on its entity
/// right now answers that by counting the edges rather than by being told each
/// tick.
///
/// A doorway that lights while somebody is in it is written that way.
/// [`Started`](ContactEdge::Started) increments a count,
/// [`Ended`](ContactEdge::Ended) decrements it, and the light changes only on
/// the transitions through zero. A second person stepping in changes nothing,
/// and the doorway goes dark when the last one steps out.
///
/// Only an archetype the mod declared with `contact = true` in its manifest
/// produces these at all. Everything else in the world carries no proxy that
/// reports edges.
///
/// Server realm. A hook receives this, so there is nothing here to refuse.
///
/// # What the host guarantees about the pairing
///
/// A count kept this way is safe, because the host is careful with pairs.
///
/// - Edges are deduplicated per instance and per touching object. Only the
///   first [`Started`](ContactEdge::Started) and the last
///   [`Ended`](ContactEdge::Ended) of one pair become events, so an instance
///   whose several colliders all meet the same body fires once.
/// - An instance touching another of its own colliders is not contact and is
///   dropped.
/// - Under queue pressure the host cancels a whole queued pair rather than
///   dropping half of one. A mod flooded with events can miss a touch entirely.
///   It cannot be left holding an unbalanced count.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `ironlark::state!` declares what this mod remembers between events.
/// // `ContactEdge`, `PhysicsObject`, `Target`, `Vec3`, `Context` and
/// // `ServerMod` all arrive with the prelude imported above.
/// ironlark::state! {
///     static INSIDE: u32 = 0;
/// }
///
/// struct Threshold;
///
/// impl ServerMod for Threshold {
///     async fn on_contact(
///         _ctx: Context,
///         _target: Target,
///         _other: PhysicsObject,
///         _point: Vec3,
///         edge: ContactEdge,
///     ) {
///         let now = INSIDE.update(|count| {
///             match edge {
///                 ContactEdge::Started => *count += 1,
///                 ContactEdge::Ended => *count = count.saturating_sub(1),
///             }
///             *count
///         });
///         log::info!("{now} standing in the doorway");
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactEdge {
    /// The touch began.
    Started,
    /// The touch ended.
    Ended,
}

impl core::fmt::Debug for Target {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Target")
            .field("id", &self.id)
            .field("entity", &self.entity.is_some())
            .finish()
    }
}
