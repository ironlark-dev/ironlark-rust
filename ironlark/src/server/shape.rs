//! The probe vocabulary shared by what a mod places and what it asks.

use crate::math::Vec3;

/// The probe an intersection query is asked with.
///
/// The same four kinds an archetype declares under `shape` in its `mod.toml`,
/// so one vocabulary covers what a mod places in the world and what it asks the
/// world about. Every length is a full extent, and a capsule's length is the
/// cylindrical segment alone.
///
/// A probe is not in the world. Nothing collides with it, it blocks nobody, and
/// it exists for the length of the call.
///
/// ```
/// // The prelude mints Shape, Vec3, Quat and the spatial module.
/// use ironlark::server::prelude::*;
///
/// // A five-metre blast, and a doorway-sized box standing on the world's axes.
/// let blast = Shape::Sphere(5.0);
/// let doorway = Shape::Box(Vec3::new(1.2, 2.1, 0.3));
/// assert!(matches!(blast, Shape::Sphere(_)));
/// let _ = (doorway, Quat::IDENTITY);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Full extents on each axis.
    Box(Vec3),
    /// Radius.
    Sphere(f32),
    /// Radius, then the cylindrical segment's length.
    Capsule(f32, f32),
    /// Radius, then height.
    Cylinder(f32, f32),
}
