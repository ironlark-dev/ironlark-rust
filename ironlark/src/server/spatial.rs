//! Asking the world what is there.
//!
//! A contact event tells a mod when something touched it. These go the other
//! way. They are a probe the mod performs when it wants to know, without
//! waiting to be touched.
//!
//! [`raycast`] is the one that answers about the world. Casting straight
//! down over a spot is how a mod stands a door on ground it did not author,
//! and casting toward a participant is how it decides whether the line to
//! them is blocked. [`intersections`] asks the other way round: place a
//! [`Shape`] somewhere and it answers everything in the world
//! the probe touches, a participant's body and another author's entity
//! included.
//!
//! Neither refuses. Both answer nothing where a verb elsewhere would say
//! why, so a mod cannot tell an empty world from a host that could not
//! answer.
//!
//! Each call crosses into the host and back, so the cost of a scan is the
//! number of calls it makes rather than the arithmetic in it. A loop asking
//! once per participant per tick is the shape to avoid.
//!
//! ```
//! // The prelude mints Vec3, Entity, SpawnPoint and the spatial module.
//! use ironlark::server::prelude::*;
//! # mod protocol { ironlark::declares!("doctest/mod.toml"); }
//!
//! // Standing a prop on ground this mod did not author: cast down from
//! // above the spot and read the height out of the hit. `archetype::Door`
//! // is what this mod's own `[[declares.archetype]]` mints.
//! async fn stand_a_door_at(x: f32, z: f32) {
//!     let from = Vec3::new(x, 50.0, z);
//!     let down = Vec3::new(0.0, -1.0, 0.0);
//!     let Some(hit) = spatial::raycast(from, down, 100.0).await else {
//!         log::warn!("nothing under ({x}, {z}) within 100 metres");
//!         return;
//!     };
//!     let at = SpawnPoint { position: hit.position, yaw: 0.0 };
//!     if let Err(e) = Entity::spawn(protocol::archetype::Door, at).await {
//!         log::error!("no door this session: {e}");
//!     }
//! }
//! ```

use super::entity::Entity;
use super::events::PhysicsObject;
use super::shape::Shape;
use crate::math::{Quat, Vec3};

/// Everything [`raycast`] reports about the first surface a ray met.
///
/// This is how a mod reads the world instead of assuming it. Two shapes cover
/// most of the uses.
///
/// A mod stands a prop on terrain it did not author by casting straight down
/// over the spot it is considering and taking
/// [`position`](RayHit::position)`.y` as the ground height there. Finding no
/// hit at all is how it learns the map's colliders do not exist yet, so it
/// returns and probes again on the next tick rather than leaving the prop in
/// the air.
///
/// A mod asks whether one point can see a participant by casting toward them
/// and calling the line blocked when [`distance`](RayHit::distance) comes back
/// notably shorter than the distance to them. That works because character
/// bodies are excluded from every cast. A ray never reports a participant, so
/// a clear line to one reports no hit at all, and anything reported between
/// here and there is cover.
///
/// Server realm, like [`raycast`] itself, because the world the ray travels
/// through is the server's.
///
/// # What always arrives, and what does not
///
/// [`position`](RayHit::position), [`normal`](RayHit::normal) and
/// [`distance`](RayHit::distance) are always there. `position` is the origin
/// advanced by `distance` along the direction, and `normal` is the surface
/// normal at that point.
///
/// [`entity`](RayHit::entity) is present only when the thing hit resolves to
/// an instance [`Entity::set_id`](crate::server::Entity::set_id) addressed for
/// THIS mod. Unlike the entity
/// inside a [`Target`](crate::server::Target), it is a handle of the mod's
/// own. It keeps answering after the handler returns, and the host's side of
/// it is released when the last copy drops.
///
/// Absent `entity` collapses unrelated facts into one value. The ray met map
/// geometry. Or it met another author's entity, which blocks the ray and
/// reports its geometry without becoming addressable. Or it met an entity
/// whose identity did not resolve. Or the instance was the caller's own and
/// the host could not put a handle in its table for it, in which case the hit
/// still arrives and the handle is dropped rather than the cast refused. A mod
/// cannot tell a wall from another author's prop here, nor either of those
/// from one of its own that the host withheld.
/// [`on_contact`](crate::server::ServerMod::on_contact) answers the same
/// question in a richer vocabulary, because
/// [`PhysicsObject`] names a participant, a
/// foreign entity with its owner's id, or the map. A mod that needs the
/// distinction gets it by being touched rather than by looking.
///
/// # Nothing refuses
///
/// The cast answers with a hit or with nothing, and no error can come back.
/// Nothing therefore covers both a clear line and a host that could not answer
/// at all, and the two are indistinguishable at the call site.
///
/// # Example
///
/// Probing the ground under a spot. The server prelude mints
/// [`Vec3`] and the [`spatial`](crate::server::spatial) module
/// the verb lives in.
///
/// ```
/// // The prelude mints Vec3, Rgba and the spatial module.
/// use ironlark::server::prelude::*;
///
/// /// The map's ground height at (x, z), probed from above.
/// async fn ground_height(x: f32, z: f32, from_y: f32, fallback: f32) -> f32 {
///     // Start well above the spot, so the ray begins in open air.
///     let origin = Vec3::new(x, from_y + 20.0, z);
///     // Straight down. Length is free, because the host normalizes it.
///     let down = Vec3::new(0.0, -1.0, 0.0);
///     // Third argument is how far the ray travels, in metres.
///     match spatial::raycast(origin, down, 100.0).await {
///         Some(hit) => hit.position.y,
///         // Off the map, or its colliders are not loaded yet.
///         None => fallback,
///     }
/// }
/// ```
///
/// # What is not built
///
/// Nothing here reports who owns the surface a ray met. A hit is a place and a
/// distance, plus a handle when the instance is the caller's own, and there is
/// no field for a foreign owner's id. A mod that needs that distinction has to
/// get it from a contact.
pub struct RayHit {
    /// The instance hit, when this mod gave it an id. Absent for map geometry, for
    /// another author's entity, for an entity whose identity did not resolve,
    /// and for one of this mod's own that the host withheld.
    pub entity: Option<Entity>,
    /// Where the ray met the surface, in world space. The origin advanced by
    /// [`distance`](RayHit::distance) along the direction cast.
    pub position: Vec3,
    /// The surface normal at [`position`](RayHit::position).
    pub normal: Vec3,
    /// Metres from the ray's origin to [`position`](RayHit::position).
    pub distance: f32,
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::{RayHit, Shape};
    use crate::bindings::server::ironlark::host::spatial as host;
    use crate::math::{Quat, Vec3};
    use crate::server::entity::Entity;
    use crate::server::events::PhysicsObject;

    fn to_wire(v: Vec3) -> crate::bindings::server::ironlark::host::types::Vec3 {
        crate::bindings::server::ironlark::host::types::Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }

    fn quat_to_wire(q: Quat) -> crate::bindings::server::ironlark::host::types::Quat {
        crate::bindings::server::ironlark::host::types::Quat {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }

    /// The wire's three cases as the author's, with an empty id read as no id
    /// rather than as an id that is empty.
    fn physics_object(
        p: crate::bindings::server::ironlark::host::types::PhysicsObject,
    ) -> PhysicsObject {
        use crate::bindings::server::ironlark::host::types::PhysicsObject as Wire;
        match p {
            Wire::Player(session) => PhysicsObject::Player(crate::SessionId::new(session)),
            Wire::Entity(id) => PhysicsObject::Entity((!id.is_empty()).then_some(id)),
            Wire::MapGeometry => PhysicsObject::MapGeometry,
        }
    }

    fn from_wire(v: crate::bindings::server::ironlark::host::types::Vec3) -> Vec3 {
        Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }

    /// The closest hit within `max_distance`, if any.
    pub async fn raycast(origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
        host::raycast(to_wire(origin), to_wire(direction), max_distance)
            .await
            .map(|hit| RayHit {
                entity: hit.entity.map(Entity::from_owned),
                position: from_wire(hit.position),
                normal: from_wire(hit.normal),
                distance: hit.distance,
            })
    }

    /// Everything in the world the probe touches.
    pub async fn intersections(shape: Shape, at: Vec3, facing: Quat) -> Vec<PhysicsObject> {
        use crate::bindings::server::ironlark::host::spatial::Shape as Wire;
        let shape = match shape {
            Shape::Box(size) => Wire::Box(to_wire(size)),
            Shape::Sphere(radius) => Wire::Sphere(radius),
            Shape::Capsule(radius, length) => Wire::Capsule((radius, length)),
            Shape::Cylinder(radius, height) => Wire::Cylinder((radius, height)),
        };
        host::intersections(shape, to_wire(at), quat_to_wire(facing))
            .await
            .into_iter()
            .map(physics_object)
            .collect()
    }
}

/// Off the session target there is no world, and no colliders, so no hits.
#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::{RayHit, Shape};
    use crate::math::{Quat, Vec3};
    use crate::server::events::PhysicsObject;

    /// Casting needs a session, so nothing is hit here.
    pub async fn raycast(_origin: Vec3, _direction: Vec3, _max_distance: f32) -> Option<RayHit> {
        None
    }

    /// An intersection needs a world, so nothing is found here.
    pub async fn intersections(_shape: Shape, _at: Vec3, _facing: Quat) -> Vec<PhysicsObject> {
        Vec::new()
    }
}

/// Casts a ray through the world and answers with the closest thing it meets.
///
/// Travels from `origin` along `direction` for at most `max_distance` metres
/// and reports the FIRST surface, not a list. This is the verb a mod reaches
/// for when it has to place something on ground it did not author, or decide
/// whether one point can see another. Casting straight down over a spot gives
/// the ground height there. Casting from one point toward another, and
/// comparing the reported [`distance`](RayHit::distance) against the distance
/// between them, says whether anything stands in the way.
///
/// `origin` is a place in the world and `direction` is a direction through it,
/// both [`Vec3`]. The host normalizes the direction, so its
/// length is free. `max_distance` is in metres. It reaches this crate through
/// [`server::spatial`](crate::server::spatial), which the server prelude
/// carries. See [`RayHit`] for the shape of an answer and an example of the
/// downward probe.
///
/// Server realm, because the world the ray travels through is the server's. A
/// client half has nothing authoritative to cast against.
///
/// # What the ray can see
///
/// Map geometry, and the collision shapes of instances whose archetype
/// declares `interact` or `contact` in its `mod.toml` entry. Everything else
/// in the world is invisible to it. An archetype that declares neither carries
/// no collision shape at all, and a pass-through zone declared `solid = false`
/// that is not also interactable is deliberately transparent, so a trigger
/// volume never swallows a cast aimed through it.
///
/// Character bodies are excluded from every cast, so a ray never reports a
/// participant. That is what the line-of-sight shape stands on, and it is also
/// why a mod cannot use this to learn it hit somebody.
///
/// A hit always carries where and how far. It carries an [`Entity`] only when
/// [`Entity::set_id`](crate::server::Entity::set_id) addressed the thing hit
/// for this mod. Another author's
/// entity blocks the ray and reports its geometry without becoming
/// addressable. See [`RayHit`] for what that absence does and does not
/// distinguish.
///
/// # Example
///
/// Standing a prop on ground this mod did not author: cast straight down from
/// above the spot and read the height out of the hit.
///
/// ```
/// // The prelude mints Vec3, Entity, SpawnPoint and the spatial module.
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
///
/// // `archetype::Door` is what this mod's own `[[declares.archetype]]` mints,
/// // through `ironlark::declares!("../mod.toml")`.
/// async fn stand_a_door_at(x: f32, z: f32) {
///     let from = Vec3::new(x, 50.0, z);
///     let down = Vec3::new(0.0, -1.0, 0.0);
///     let Some(hit) = spatial::raycast(from, down, 100.0).await else {
///         log::warn!("nothing under ({x}, {z}) within 100 metres");
///         return;
///     };
///     let at = SpawnPoint { position: hit.position, yaw: 0.0 };
///     if let Err(e) = Entity::spawn(protocol::archetype::Door, at).await {
///         log::error!("no door this session: {e}");
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// There is no [`Result`](crate::Result). A ray that meets nothing, a
/// direction of zero length, a direction that is not finite, and a host that
/// could not answer within its own reply window all arrive as `None`. The host
/// writes a log line for the last of those alone, so a direction the physics
/// world would not accept leaves no record anywhere and reads exactly like a
/// ray that met nothing. Check a computed direction before casting along it.
///
/// Each call is one crossing into the host and back, so the cost of a scan is
/// the number of casts it makes rather than the arithmetic around them. On the
/// build machine there is no world, so every cast answers `None` and a test
/// written against a hit has to supply one itself.
pub async fn raycast(origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
    backend::raycast(origin, direction, max_distance).await
}

/// Answers everything in the world the probe touches.
///
/// The probe is a [`Shape`] placed at `at` and turned by `facing`. What comes
/// back is one [`PhysicsObject`] per thing it
/// meets: a participant's body, another mod's entity under the full id its
/// owner knows it by, or the map's own geometry.
///
/// Intersecting, not containing. A body the probe catches by an edge is in the
/// answer, which is what a zone wants: somebody standing on the boundary of a
/// radiation field is in it.
///
/// It reaches this crate through [`server::spatial`](crate::server::spatial),
/// which the server prelude carries. Server realm, like [`raycast`], and for
/// the same reason.
///
/// # Identity, never a handle
///
/// The answer names things and hands over nothing to act through. A caller that
/// wants to act on one of its own instances takes the id and resolves it with
/// [`Entity::by_id`](crate::server::Entity::by_id), which is a map lookup
/// and takes no `await`. Reading the world is unrestricted; what a mod may DO
/// is the ownership rule's to decide, and that decision belongs at the write.
///
/// # Example
///
/// A radiation field: everybody the zone catches this tick.
///
/// ```
/// // The prelude mints Shape, PhysicsObject, Quat, Vec3 and the spatial module.
/// use ironlark::server::prelude::*;
///
/// async fn irradiate(centre: Vec3) {
///     let field = Shape::Sphere(8.0);
///     for object in spatial::intersections(field, centre, Quat::IDENTITY).await {
///         match object {
///             PhysicsObject::Player(session) => log::info!("{session} is in the field"),
///             // Another author's prop, with an id or without, and the map itself.
///             PhysicsObject::Entity(_) | PhysicsObject::MapGeometry => {}
///             // The set may grow, so a match on it carries this arm.
///             _ => {}
///         }
///     }
/// }
/// ```
///
/// # Nothing refuses, and the answer can be short
///
/// There is no [`Result`](crate::Result). An empty list covers "nothing is in
/// there" and "the host could not answer" alike, and only the host's log
/// separates them.
///
/// The list is capped at the same limit [`find`](crate::server::find) uses.
/// Reaching that cap stops the scan without a word anywhere, so a truncated
/// answer and a complete one are identical to the caller.
///
/// An archetype that declares neither `interact` nor `contact` carries no
/// collision shape, so no probe can meet it whatever its identity. That is the
/// world's own limit rather than this verb's.
///
/// On the build machine there is no world, so it answers with an empty list.
pub async fn intersections(shape: Shape, at: Vec3, facing: Quat) -> Vec<PhysicsObject> {
    backend::intersections(shape, at, facing).await
}
