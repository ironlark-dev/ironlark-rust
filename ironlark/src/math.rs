//! Plain math carriers mirroring the contract shapes. Re-exported at the crate
//! root and carried by both preludes.

/// A place in the world, or a direction through it, in metres.
///
/// Three floats, Y up. `y` is height and `x` and `z` span the ground. Whether
/// a value is a point or a direction is the verb's to say and not the type's.
/// [`raycast`](crate::server::spatial::raycast) takes an origin and a
/// direction in that order and means something different by each.
///
/// It belongs to no realm. Both preludes carry it, and it means the same
/// metres under [`server`](crate::server) and under
/// [`client`](crate::client).
///
/// # Which way the axes point
///
/// A body faces its own `-z` and yaw turns it about `y`, so at yaw zero
/// forward is `-z`, behind is `+z`, right is `+x` and up is `+y`. Those are
/// the only bearings there are. Nothing in the world is north, and a mod that
/// wants a facing computes it from a yaw rather than from a compass.
///
/// [`Vec3::new`] is `const`, so a fixed spot or a fixed offset is a `const`
/// item beside the handler that uses it rather than three literals inside it.
/// A pad meant to rest on the floor takes a small positive `y`, because at
/// `y = 0.0` it sits inside the ground instead of on it.
///
/// ```
/// // The prelude mints Vec3.
/// use ironlark::server::prelude::*;
///
/// // Four metres left of the origin, four metres back, resting on the floor.
/// const PAD: Vec3 = Vec3::new(-4.0, 0.05, 4.0);
/// // Straight down, for probing the ground under a spot.
/// const DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);
/// ```
///
/// # Where one arrives from
///
/// The world hands them back:
/// [`Entity::translation`](crate::server::Entity::translation) for where an
/// entity stands, [`RayHit::position`](crate::server::spatial::RayHit::position)
/// and [`normal`](crate::server::spatial::RayHit::normal) from a cast, the
/// contact point in [`on_contact`](crate::server::ServerMod::on_contact), and
/// [`SpawnPoint::position`](crate::server::SpawnPoint::position) in every entry
/// [`spawn_points`](crate::server::map::spawn_points) returns.
///
/// # Where one goes
///
/// [`Entity::set_translation`](crate::server::Entity::set_translation) and
/// [`set_scale`](crate::server::Entity::set_scale) take one, and so do
/// [`raycast`](crate::server::spatial::raycast) and
/// [`intersections`](crate::server::spatial::intersections). Spawning takes a
/// [`SpawnPoint`](crate::server::SpawnPoint) built around one. Everything else goes
/// through [`Value::Vec3`](crate::server::Value::Vec3) in a
/// [`Field`](crate::server::Field) written with
/// [`set_component`](crate::server::Entity::set_component) — which is the same
/// road the two named setters take, spelled out.
///
/// Every one of those verbs is in the server realm, because the world they
/// read and write is the server's. The [`client`](crate::client) prelude
/// carries the type all the same. A place reaches the half that draws it as
/// the fields of a payload the mod's own `protocol.proto` declares — three
/// floats, because a schema names what protobuf knows — and the client half
/// builds a `Vec3` out of them to compute with.
///
/// # What the host checks
///
/// A component write must be finite in all three components. An infinity or a
/// NaN refuses the whole call before anything is applied, with the field named
/// in the message, so a bad number never half-lands and never reaches the
/// wire.
///
/// That refusal has no kind of its own. It arrives as
/// [`ErrorKind::Other`](crate::ErrorKind::Other), like every other verdict the
/// world passes back on a write, so the message is the only thing that
/// separates a bad number from a body this mod may not touch. Log it rather
/// than branching on it.
///
/// A cast is not a write and does not refuse. The host normalizes the
/// direction, so its length is free. A direction of zero length, or one that
/// is not finite, yields no hit at all — the same answer as a clear ray. That
/// is the one place a bad `Vec3` is silent rather than typed, so check a
/// computed direction before casting along it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// Across the ground. `+x` is a body's right at yaw zero.
    pub x: f32,
    /// Height. `+y` is up.
    pub y: f32,
    /// Along the ground. A body at yaw zero faces `-z`.
    pub z: f32,
}

/// Which way a thing faces, as a quaternion.
///
/// `x`, `y` and `z` are the imaginary components and `w` the real one. That
/// order is the wire's and the host reads it that way, so the four fields are
/// not interchangeable and `w` in particular is last rather than first.
///
/// It belongs to no realm and both preludes carry it. The verbs that read and
/// write one are all in the server realm, because a rotation is a fact about
/// the server's world. The [`client`](crate::client) prelude carries the type
/// all the same, for the half that draws a facing it read out of the four
/// floats a payload carried.
///
/// # Where one arrives from and where it goes
///
/// One component field takes a quaternion, `rotation` on the `transform` row.
/// A mod writes it as [`Value::Quat`](crate::server::Value::Quat) inside a
/// [`Field`](crate::server::Field) handed to
/// [`Entity::set_component`](crate::server::Entity::set_component), and reads
/// it back the same way from
/// [`get_component`](crate::server::Entity::get_component). Unlike position
/// and scale, rotation has no named setter of its own, so the resolved
/// component and field ids are how a mod addresses it. A mod that needs only a
/// facing about the vertical is better served by the yaw on
/// [`SpawnPoint`](crate::server::SpawnPoint), which takes an angle in radians and
/// needs no quaternion at all.
///
/// # What the host checks, and what it does not
///
/// It checks that all four components are finite and refuses the whole write
/// if any is not. It does not normalize and does not refuse a quaternion that
/// is not of unit length. Whatever arrives becomes the rotation. Keeping the
/// value normalized is the mod's job, and a value that has drifted shows up as
/// something looking wrong in the world rather than as an error to handle.
///
/// # What is not built
///
/// There is no constructor, no identity constant and no `Default`, so a value
/// is written field by field. `Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }` is the
/// identity, meaning no rotation, and is the value to start from.
///
/// Nothing shortens the road through
/// [`set_component`](crate::server::Entity::set_component). Writing a rotation
/// means resolving the `transform` component and its `rotation` field with
/// [`resolve::component`](crate::server::resolve::component) and
/// [`resolve::field`](crate::server::resolve::field), then handing over a
/// [`Field`](crate::server::Field).
///
/// ```
/// // The prelude mints Quat and Vec3.
/// use ironlark::server::prelude::*;
///
/// // A quarter turn about the up axis, as the world hands one back.
/// let facing = Quat { x: 0.0, y: 0.707, z: 0.0, w: 0.707 };
/// assert!((facing.y - 0.707).abs() < 0.001);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    /// The imaginary component about x.
    pub x: f32,
    /// The imaginary component about y.
    pub y: f32,
    /// The imaginary component about z.
    pub z: f32,
    /// The real component.
    pub w: f32,
}

impl Quat {
    /// No rotation at all.
    ///
    /// What a verb taking a facing is given when the caller has none to state,
    /// which is every probe whose shape a turn cannot change and every one
    /// aligned with the world's own axes.
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
}

/// A colour, four channels running 0.0 to 1.0.
///
/// The host reads the three colour channels as sRGB — the numbers a colour
/// picker shows, divided by 255 — and hands them back the same way, so a
/// colour read from the world and written straight back is the colour that was
/// there. `a` is opacity, 0.0 invisible and 1.0 solid.
///
/// It belongs to no realm and both preludes carry it. Painting is a server
/// verb, because the material it changes belongs to the server's world and
/// replicates from there. The [`client`](crate::client) prelude carries the
/// type all the same, for the half that draws a colour it read out of the four
/// channels a payload carried.
///
/// # Where one arrives from and where it goes
///
/// One component field takes a colour, `base_color` on the `material` row.
/// [`Entity::set_base_color`](crate::server::Entity::set_base_color) is the
/// named way to write it, and it is a one-line wrapper over the general road.
/// That road is [`Value::Rgba`](crate::server::Value::Rgba) inside a
/// [`Field`](crate::server::Field) handed to
/// [`set_component`](crate::server::Entity::set_component). Reading the colour
/// back goes through [`get_component`](crate::server::Entity::get_component),
/// which is the one direction the named setter does not cover.
///
/// A write reaches every mesh in the entity's subtree, so painting a prop made
/// of several parts is one call. To paint part of a prop instead, narrow to it
/// first with [`Entity::part`](crate::server::Entity::part) — a door's handle
/// rather than the whole door — and every mesh under that part is painted by
/// the one write.
///
/// # What the host checks
///
/// Every channel must be finite, and a non-finite one refuses the whole write.
/// The range is neither clamped nor refused. A channel outside 0.0 to 1.0 is
/// applied as given, and what a renderer does with it is not this crate's
/// promise. An entity with no material anywhere in its subtree refuses the
/// write.
///
/// The host drops a write when every mesh in the subtree already shows that
/// colour. No clone, no work, nothing replicated. A subtree whose meshes have
/// diverged is written anyway, because a partial match is still a change. So
/// painting on every tick costs the call and usually nothing beyond it. The
/// call is still a crossing into the host and back, so a mod that paints on a
/// state change rather than on a tick pays neither.
///
/// # What is not built
///
/// There is no constructor and no `Default`, so a colour is written field by
/// field. `material.base_color` is the whole of what a colour can be set on.
/// There is no light, no ambient and no per-mod palette.
///
/// ```
/// // The prelude mints Rgba, Entity, SpawnPoint and Vec3.
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
///
/// // Channels run 0.0 to 1.0, alpha included. `archetype::Door` is what this
/// // mod's own `[[declares.archetype]]` mints.
/// async fn redden_a_door() {
///     let at = SpawnPoint { position: Vec3::new(0.0, 0.0, 4.0), yaw: 0.0 };
///     let Ok(door) = Entity::spawn(protocol::archetype::Door, at).await else {
///         return;
///     };
///     let red = Rgba { r: 1.0, g: 0.2, b: 0.2, a: 1.0 };
///     if let Err(e) = door.set_base_color(red).await {
///         log::warn!("the door did not redden: {e}");
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red, 0.0 to 1.0.
    pub r: f32,
    /// Green, 0.0 to 1.0.
    pub g: f32,
    /// Blue, 0.0 to 1.0.
    pub b: f32,
    /// Opacity, 0.0 transparent to 1.0 opaque.
    pub a: f32,
}

impl Vec3 {
    /// Builds a [`Vec3`] from its three components, in metres.
    ///
    /// `const`, so a fixed place or a fixed direction is a `const` item beside
    /// the handler that uses it rather than three literals inside it.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}
