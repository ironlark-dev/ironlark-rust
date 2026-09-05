//! Typed entity verbs over the host's generic set, re-exported into
//! [`server`](crate::server) and reached from there. A world verb needs a
//! session, so off the session target every one of them answers a typed error
//! saying so rather than pretending to have acted.

use crate::error::{Error, Result};
use crate::ids::{ComponentId, FieldId};
use crate::math::{Rgba, Vec3};

mod backend;
#[cfg(any(target_arch = "wasm32", test))]
mod held;

use backend::{body_of_at_target, by_id_at_target, find_at_target};

/// A place something may appear and which way it faces: a world-space position
/// in metres, and a yaw about the up axis in radians.
///
/// One type covers both directions. [`Entity::spawn`] takes it as where to put
/// a new instance, and
/// [`map::spawn_points`](crate::server::map::spawn_points) answers with the
/// ones the loaded map suggests, so a point read from the map feeds straight
/// into a spawn with nothing to convert.
///
/// The host writes exactly these two onto the instance. There is no ground
/// snap, no collision resolve and no nudge, because placement belongs to the
/// mod that asked for it. A `y` under the floor puts the instance under the
/// floor. A mod that owns one fixed spot hard-codes it. A mod that must stand
/// something on terrain it did not author computes the spot instead, casting a
/// ray straight down and reading the ground height out of the
/// [`RayHit`](crate::server::spatial::RayHit) that comes back.
///
/// `yaw` turns the instance about the up axis and nothing else. There is no
/// pitch and no roll here. Zero is the archetype's authored facing.
/// [`map::spawn_points`](crate::server::map::spawn_points) answers in radians
/// too, so one of the map's suggested points feeds straight back into a spawn.
///
/// Server realm, because spawning is. A client mod renders what the session
/// tells it about and creates nothing of its own.
///
/// Nothing in this struct is refused. Every value is legal, one that puts the
/// instance inside a wall among them, and the transform is applied as handed
/// over. What a spawn refuses on is the archetype name, and that is
/// [`Entity::spawn`]'s subject.
///
/// The example places one instance of an archetype this mod declares and gives
/// it an id. [`Entity::set_id`] is what puts that id into the events the
/// instance raises, so it is worth the second call.
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
///
/// // `SpawnPoint`, `Vec3` and `Entity` all arrive from the prelude above.
/// // `door` is this mod's own archetype, declared in mod.toml beside the
/// // crate:
/// //
/// //     [[declares.archetype]]
/// //     id = "door"
/// //     scene = "door.glb"
/// //
/// // `ironlark::declares!("../mod.toml")` reads that row and mints
/// // `archetype::Door` from the `id`, so the name is checked here rather
/// // than in a session.
/// async fn hang_a_door() {
///     let at = SpawnPoint {
///         position: Vec3::new(-4.0, 0.0, 4.0),
///         yaw: 0.0,
///     };
///     match Entity::spawn(protocol::archetype::Door, at).await {
///         Ok(door) => {
///             if let Err(e) = door.set_id("door/north").await {
///                 log::error!("the door is unreachable by id: {e}");
///             }
///         }
///         Err(e) => log::error!("no door this session: {e}"),
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnPoint {
    /// Where it lands.
    pub position: Vec3,
    /// Which way it faces, in radians.
    pub yaw: f32,
}

/// One assignment inside a component write. Which field, and the value to put
/// there.
///
/// [`Entity::set_component`] takes a batch of these and
/// [`Entity::get_component`] hands the same shape back, so one type covers both
/// directions. The field is a [`FieldId`] that
/// [`resolve::field`](crate::server::resolve::field) minted from a path written
/// as a string. That resolve belongs in `init` and never on a per-tick call.
/// What goes into the field is a [`Value`], one of six shapes.
///
/// This is what the typed shortcuts on [`Entity`] assemble for an author.
/// [`Entity::set_base_color`] is one `Field` carrying a [`Value::Rgba`] into
/// the `material` row and [`Entity::set_label`] is one carrying a
/// [`Value::Text`] into the `label` row. Build one by hand for a row those
/// shortcuts do not cover.
///
/// # What refuses
///
/// A batch is checked whole before anything is applied, so a refusal leaves the
/// entity exactly as it was and never half-writes.
///
/// - A field id this session does not carry, or one that belongs to a component
///   other than the one named on the call. The pairing is what was numbered, so
///   the `base_color` id of `material` handed to a `transform` write is caught
///   here rather than written into the wrong row. Both read as
///   [`UnresolvedName`](crate::ErrorKind::UnresolvedName).
/// - The same field twice in one batch. The call contradicts itself, and the
///   host will not guess which of the two was meant.
/// - A path the component does not have. An id is only the session's number for
///   a string, minted on request without asking the world, so a misspelt path
///   resolves happily and dies here at the first write.
/// - A value of a different kind from the one the field holds. The refusal
///   names both kinds.
/// - A value the content rules reject. Those are per-kind and listed on
///   [`Value`].
///
/// Everything but the id checks is the world declining the write, so it arrives
/// as [`Other`](crate::ErrorKind::Other) with the host's own sentence in the
/// message. Read the message. The kind alone will not tell a mismatched value
/// from an unknown path.
///
/// A field restating the value the entity already holds is dropped rather than
/// replicated, so recomputing a colour every tick and writing it back costs
/// nothing on the wire while it does not change.
///
/// # Example
///
/// Painting one entity red through the general form, with the two ids resolved
/// once and kept. An id is `Copy` and a resolve is a crossing into the host, so
/// the resolve belongs in `init` and never on the painting path.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `material` is a row the host publishes and `base_color` is a path inside
/// // it. Neither is declared in mod.toml and neither is minted by
/// // `declares!`. The two resolve calls below are the whole origin of both
/// // ids. `resolve`, `ComponentId`, `FieldId`, `Field`, `Value` and `Rgba`
/// // all arrive from the prelude above.
/// ironlark::state! {
///     static MATERIAL: Option<ComponentId> = None;
///     static BASE_COLOR: Option<FieldId> = None;
/// }
///
/// // Called once, from this mod's `init` hook.
/// fn learn_the_material_row() {
///     let Ok(material) = resolve::component("material") else {
///         log::error!("this session publishes no material row");
///         return;
///     };
///     let Ok(base_color) = resolve::field(material, "base_color") else {
///         log::error!("the material row has no base_color path");
///         return;
///     };
///     MATERIAL.set(Some(material));
///     BASE_COLOR.set(Some(base_color));
/// }
///
/// async fn paint_red(door: &Entity) {
///     let Some(material) = MATERIAL.get() else {
///         return;
///     };
///     let Some(base_color) = BASE_COLOR.get() else {
///         return;
///     };
///     let red = Field {
///         field: base_color,
///         value: Value::Rgba(Rgba { r: 0.9, g: 0.2, b: 0.15, a: 1.0 }),
///     };
///     if let Err(e) = door.set_component(material, [red]).await {
///         log::warn!("the door kept the colour it had: {e}");
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Which field of the component.
    pub field: FieldId,
    /// What to write there.
    pub value: Value,
}

/// The host's standard rows, resolved once on first typed access.
#[derive(Clone, Copy)]
struct StdIds {
    transform: ComponentId,
    translation: FieldId,
    scale: FieldId,
    label: ComponentId,
    text: FieldId,
    through_walls: FieldId,
    material: ComponentId,
    base_color: FieldId,
}

crate::state! {
    static STD_IDS: Option<StdIds> = None;
}

fn std_ids() -> Result<StdIds> {
    if let Some(ids) = STD_IDS.get() {
        return Ok(ids);
    }
    let transform = crate::shared::resolve::component("transform")?;
    let label = crate::shared::resolve::component("label")?;
    let material = crate::shared::resolve::component("material")?;
    let ids = StdIds {
        transform,
        translation: crate::shared::resolve::field(transform, "translation")?,
        scale: crate::shared::resolve::field(transform, "scale")?,
        label,
        text: crate::shared::resolve::field(label, "text")?,
        through_walls: crate::shared::resolve::field(label, "through_walls")?,
        material,
        base_color: crate::shared::resolve::field(material, "base_color")?,
    };
    STD_IDS.set(Some(ids));
    Ok(ids)
}

/// The six primitive shapes a component field carries across the boundary, in
/// either direction.
///
/// The vocabulary is closed on purpose. Making a new component reachable is a
/// registration on the host and costs a mod nothing. Adding a seventh shape
/// here is a change to the contract itself, which every mod ever built is
/// compiled against. So the six cover the field types the host exposes, and a
/// field whose engine type is outside them is neither readable nor writable
/// through this surface at all.
///
/// A colour change is a [`Value::Rgba`], a door turning red while it is locked.
/// A line of text over something is a [`Value::Text`]. A moving prop's position
/// each tick is a [`Value::Vec3`]. Every one of them travels inside a
/// [`Field`], which pairs it with the field it is going into.
///
/// The enum is `#[non_exhaustive]`, so a match on a value read back needs a
/// catch-all arm. Constructing any of the six is unaffected. The arm never
/// runs, because the contract case set it mirrors is the whole set and cannot
/// grow without every compiled mod being rebuilt. Write `_ => {}` and move on.
///
/// # What refuses
///
/// Checked before any write path applies anything, so a refused value exists
/// nowhere in the engine and never reaches the wire.
///
/// - [`Number`](Value::Number), [`Vec3`](Value::Vec3), [`Quat`](Value::Quat)
///   and [`Rgba`](Value::Rgba) must be finite in every number they carry. One
///   NaN or infinity refuses the whole call.
/// - [`Text`](Value::Text) is capped at 256 **bytes**, not characters, so
///   sixty-four four-byte characters fit and sixty-five do not. No control
///   characters, newlines included. Non-empty text must put at least one
///   visible character on screen, which rules out whitespace-only and
///   zero-width-only strings, because an invisible value would squat a row
///   while displaying nothing. Text is refused whole and never truncated, since
///   a silently shortened line would replicate as something nobody wrote. The
///   empty string is legal and is the inert state.
/// - [`Boolean`](Value::Boolean) has nothing to refuse.
///
/// On top of these, the kind must match the kind the field holds. See
/// [`Field`].
///
/// # Example
///
/// Reading one text field back and matching the shape it came in. The `label`
/// row and its `text` path are names the host publishes, turned into the two
/// ids by [`resolve::component`](crate::server::resolve::component) and
/// [`resolve::field`](crate::server::resolve::field) once in `init`. [`Field`]
/// shows that resolve in full.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // `label` is the row and `text` the path inside it, both resolved in
/// // `init` and handed in here. `Entity`, `Value` and the two id types all
/// // arrive from the prelude above.
/// async fn read_the_sign(door: &Entity, label: ComponentId, text: FieldId) {
///     match door.get_component(label, &[text]).await {
///         Ok(fields) => match fields.into_iter().next().map(|f| f.value) {
///             Some(Value::Text(line)) => log::info!("the door reads {line}"),
///             Some(other) => log::warn!("the label line is not text: {other:?}"),
///             None => log::warn!("the label row answered with no field"),
///         },
///         Err(e) => log::warn!("the label is unreadable: {e}"),
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A scalar, a distance or a factor among them.
    Number(f32),
    /// A flag.
    Boolean(bool),
    /// One line of text, under the rules above.
    Text(String),
    /// A position or a direction, in metres where it is a position.
    Vec3(Vec3),
    /// An orientation.
    Quat(crate::math::Quat),
    /// A colour, sRGB with an alpha channel.
    Rgba(crate::math::Rgba),
}

/// A live thing in the world: a prop, a vehicle, a player's body.
///
/// The host issues these and a mod cannot fabricate one. What a mod may DO
/// through a given entity is checked at every use, not at the moment the handle
/// was obtained, so holding one does not hold the standing that came with it.
///
/// Server realm. It arrives through [`server`](crate::server), which is also
/// where every verb on it lives.
///
/// # Owning one, and sharing it
///
/// Nearly every handle is owned. [`Entity::spawn`], [`Entity::part`],
/// [`Entity::by_id`], [`find`], [`body_of`],
/// [`Player::body`](crate::server::Player::body) and a hit from
/// [`spatial::raycast`](crate::server::spatial::raycast) all answer with one.
/// Cloning yields a co-owner rather than a view, so a copy cached in a
/// [`State`](crate::State) cell stays valid for as long as anything holds it,
/// and the host's resource is released only when the last copy goes.
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
///
/// // `door` is this mod's own archetype, from its mod.toml:
/// //
/// //     [[declares.archetype]]
/// //     id = "door"
/// //     scene = "door.glb"
/// //
/// // `ironlark::declares!("../mod.toml")` mints `archetype::Door` from that
/// // row. `Entity`, `SpawnPoint` and `Vec3` arrive from the prelude above.
/// ironlark::state! {
///     static DOOR: Option<Entity> = None;
/// }
///
/// async fn remember_the_door() {
///     let at = SpawnPoint { position: Vec3::new(0.0, 0.0, 0.0), yaw: 0.0 };
///     let Ok(door) = Entity::spawn(protocol::archetype::Door, at).await else {
///         log::error!("the door did not spawn");
///         return;
///     };
///     // The clone is a co-owner, so the handle outlives this function.
///     DOOR.set(Some(door.clone()));
///     if let Err(e) = door.set_label("push").await {
///         log::warn!("the door carries no label: {e}");
///     }
/// }
/// ```
///
/// Dropping the last copy releases the handle and nothing else. The instance
/// stays in the world, still visible, still replicating. Only this mod's way of
/// addressing it is gone. Removing the thing itself is [`Entity::despawn`],
/// which is a separate act and which spends the entity for every copy at once.
/// A copy used after a despawn refuses with
/// [`StaleId`](crate::ErrorKind::StaleId) rather than trapping, so a mod that
/// raced itself gets an error it can log instead of dying.
///
/// # The one the host lends
///
/// The entity inside a [`Target`](crate::server::Target) is the exception. The
/// host lends it for the event being handled and takes it back when the handler
/// returns, and a copy kept past that refuses with the same
/// [`StaleId`](crate::ErrorKind::StaleId) rather than resolving to whatever
/// instance took its number. When something must outlive the event, keep the
/// target's id instead and look the instance up again with
/// [`Entity::by_id`], or a whole family of them with [`find`].
///
/// # A body is a handle whose standing is not in it
///
/// [`Player::body`](crate::server::Player::body) and [`body_of`] hand back an
/// owned handle, and it keeps addressing the body after the handler returns, so
/// unlike a target it does not go stale. What does end with the handler is the
/// standing to act through it. The host checks reach at every use against the
/// event in flight, and only an interaction or a contact makes its participant
/// the subject. A join, a leave, a tick and a signal make nobody one. So a mod
/// that holds a body handle it did not itself spawn, and moves that body from
/// its tick, meets a refusal. The handle was never the thing permitting the
/// write.
///
/// Two callers are admitted anyway, on grounds that have nothing to do with the
/// event. A mod reads and moves a body it spawned itself, as its spawner, which
/// is how a gamemode publishes where everybody is from its own tick. And a mod
/// that declared a decoration row under `body-rows` in its manifest may write
/// THAT row on any possessed body at any time, which is how one mod puts a line
/// of text over every participant. See [`Entity::set_component`], which is
/// where the whole reach rule is written down.
#[derive(Clone)]
pub struct Entity {
    #[cfg(target_arch = "wasm32")]
    inner: std::rc::Rc<held::Held>,
    #[cfg(not(target_arch = "wasm32"))]
    _no_session: core::convert::Infallible,
}

/// The typed shortcuts onto the rows the host publishes.
///
/// `transform`, `material` and `label` are rows the host publishes, and each
/// shortcut below is one [`Field`] into one of them, with
/// both names resolved once per session behind the scenes. An author writes
/// [`set_translation`](Entity::set_translation) and never spells `transform` or
/// `translation`, never holds a [`ComponentId`] and never resolves anything.
///
/// They do not cover every path of every row. `transform` also carries a
/// rotation, and `label` carries an offset, a maximum display distance and a
/// fade width beside the two paths reached here. Those go through
/// [`Entity::set_component`], which is the general form and takes a resolved
/// [`ComponentId`] and [`FieldId`]. Those two ids come from
/// [`resolve::component`](crate::server::resolve::component) and
/// [`resolve::field`](crate::server::resolve::field), one call each.
///
/// The shortcuts inherit everything from the verb underneath. The reach rule
/// decides whether the write lands, the content rules on [`Value`] decide
/// whether the value is legal, and a refusal reads exactly as it would from
/// [`set_component`](Entity::set_component). What they do not inherit is the
/// batch. Each call is one crossing into the host, so writing two rows is two
/// calls, and two paths of one row are better written as one
/// [`set_component`](Entity::set_component) carrying both [`Field`]s.
impl Entity {
    /// The entity's position in world space, in metres.
    ///
    /// A read across the boundary, not a cached value. It costs a crossing and
    /// it can refuse. The ordinary shape is a mod that opens its tick with it,
    /// adds a step along a path and writes the result back with
    /// [`set_translation`](Entity::set_translation). A mod that already knows
    /// where it put something does not need to ask.
    ///
    /// For a player's body the answer comes from the physics pose rather than
    /// the render transform, because the pose is what actually moves a body.
    ///
    /// Refuses for everything [`get_component`](Entity::get_component) refuses
    /// for, and for one thing of its own. A world that answered `translation`
    /// with something that is not a position is refused here, by the SDK rather
    /// than the host, and that cannot happen while the row is what it is.
    pub async fn translation(&self) -> Result<Vec3> {
        let ids = std_ids()?;
        let fields = self
            .get_component(ids.transform, &[ids.translation])
            .await?;
        match fields.into_iter().next().map(|f| f.value) {
            Some(Value::Vec3(v)) => Ok(v),
            _ => Err(Error::from_wire(
                0,
                "the world answered translation with a non-vec3".into(),
                Vec::new(),
            )),
        }
    }

    /// Moves the entity to a position in world space.
    ///
    /// An absolute placement in metres, never a step. The caller computes where
    /// the thing should now be. A mod that crawls a prop along a line writes one
    /// of these per tick, and the prop stays pressable while it moves, because
    /// its collision proxy is kept at the visual's pose every frame.
    ///
    /// A player's body is moved through its physics pose rather than its render
    /// transform, so this is also how a body is teleported. Moving somebody
    /// else's body needs the reach that
    /// [`set_component`](Entity::set_component) describes, and for a mod that
    /// did not spawn the body that means the handler for an event that
    /// participant caused — an interaction or a contact, and nothing else.
    ///
    /// A write that restates the position the entity already holds is dropped
    /// before it reaches the wire, so a walk that has arrived costs nothing to
    /// keep asserting. A player's body is the exception. Its pose is written
    /// through a different route, whose sent fields are announced as they are.
    pub async fn set_translation(&self, v: Vec3) -> Result<()> {
        let ids = std_ids()?;
        self.set_component(
            ids.transform,
            [Field {
                field: ids.translation,
                value: Value::Vec3(v),
            }],
        )
        .await
    }

    /// Resizes the entity, one factor per axis.
    ///
    /// A multiplier on the authored size, so `Vec3::splat(1.0)` is the model as
    /// its artist built it and three equal factors keep its proportions. A mod
    /// that wants a prop to read from further away writes the same factor above
    /// one on all three axes.
    ///
    /// It changes what a player SEES and not what they can touch. An instance's
    /// collision proxy is kept at the visual's position and rotation and its
    /// scale is dropped, so a scaled-up interactable is still pressed at its
    /// original size and a scaled-down one is still solid out to its old edge.
    /// Nothing refuses the write and nothing reports the difference. A mod whose
    /// collider matters sizes its content at authoring time instead.
    pub async fn set_scale(&self, v: Vec3) -> Result<()> {
        let ids = std_ids()?;
        self.set_component(
            ids.transform,
            [Field {
                field: ids.scale,
                value: Value::Vec3(v),
            }],
        )
        .await
    }

    /// Writes the one line of text the world shows above this entity.
    ///
    /// An entity carries no `label` row until somebody writes one. The first
    /// write attaches the row whole, built from the host's own defaults with
    /// this line applied, and every later write updates it. The rest of the row
    /// arrives with those defaults, and each of them is a path
    /// [`set_component`](Entity::set_component) reaches: `offset` lifts the
    /// line above the entity's origin, `max_distance` is the camera distance
    /// past which it stops drawing, and `fade` is the band inside that distance
    /// where it fades out. The empty string is the inert value for the line
    /// itself, and writing it is how the line is taken away. There is no
    /// separate verb for that.
    ///
    /// A mod may write this on anything it has reach over, which covers what it
    /// spawned and every instance of an archetype it declares. Writing it on a
    /// player's body needs one more thing that nothing about this call shows.
    /// `label` is a decoration row, so a mod that names it under `body-rows` in
    /// its manifest may write it on any possessed body at any time, and a mod
    /// that does not is refused there.
    /// [`set_component`](Entity::set_component) carries the whole rule.
    ///
    /// # The row answers to whoever wrote it
    ///
    /// While the row holds anything but its defaults, only the mod that put it
    /// there may change it, and another mod with write reach over the entity is
    /// refused. The carve-out is a write that lands the WHOLE row back on its
    /// defaults, which anyone with reach may do and which frees the row for the
    /// next writer. So two mods labelling one thing do not fight over it. The
    /// first to speak holds it until it goes quiet.
    ///
    /// A possession ending takes the row with it. Every claimed row on a body is
    /// reset to its defaults when [`release`](Entity::release) runs, because
    /// after that nothing has the reach to update or clear it.
    ///
    /// The text rules are the ones on [`Value`]. At most 256 bytes, no control
    /// characters, and at least one visible character if it is not empty.
    /// Refused whole, never truncated.
    pub async fn set_label(&self, text: &str) -> Result<()> {
        let ids = std_ids()?;
        self.set_component(
            ids.label,
            [Field {
                field: ids.text,
                value: Value::Text(text.to_string()),
            }],
        )
        .await
    }

    /// Whether this entity's label stays readable through geometry.
    ///
    /// Off is the default. The label is occluded like anything else in the world
    /// and vanishes behind a wall. On, it draws over whatever is in front of it,
    /// which is what makes a marker findable from across a map. A mod that wants
    /// a player to see where the far end of something is before walking there
    /// turns it on. A line of text over a participant's body leaves it off,
    /// because a name showing through a wall tells a player where somebody is
    /// standing.
    ///
    /// It is a presentation intent and not information control. The row
    /// replicates to every peer either way.
    ///
    /// A second path of the same `label` row, so everything on
    /// [`set_label`](Entity::set_label) applies, the row answering to whoever
    /// wrote it included. Two calls are two crossings, so write the line and
    /// this flag as one [`set_component`](Entity::set_component) when both are
    /// being decided at once.
    pub async fn set_label_through_walls(&self, on: bool) -> Result<()> {
        let ids = std_ids()?;
        self.set_component(
            ids.label,
            [Field {
                field: ids.through_walls,
                value: Value::Boolean(on),
            }],
        )
        .await
    }

    /// Paints the entity's surface.
    ///
    /// How a mod shows a state without words. A door goes red while it is
    /// locked, a marker goes green once a player has reached it.
    ///
    /// The colour is sRGB `r`, `g`, `b`, `a`, each of which must be finite. One
    /// NaN refuses the whole call. A write is dropped before the wire when every
    /// surface it covers already shows that colour, so recomputing a colour
    /// every tick and writing it back costs nothing while it does not change.
    ///
    /// # A subtree, not one surface
    ///
    /// `material` means every mesh under the entity this is called on, so
    /// calling it on the root of a scene instance paints the whole model. To
    /// paint one piece, reach the authored node with [`part`](Entity::part)
    /// first and call this on what comes back — the panel of a door rather than
    /// its frame and hinges as well.
    ///
    /// Instances spawned from one archetype share their material asset until
    /// somebody writes it. The first write clones the asset for that mesh alone,
    /// so recolouring one door never recolours the others.
    ///
    /// Unlike `label`, this row is never attached by a write. An entity with no
    /// mesh anywhere under it refuses rather than growing one.
    pub async fn set_base_color(&self, color: Rgba) -> Result<()> {
        let ids = std_ids()?;
        self.set_component(
            ids.material,
            [Field {
                field: ids.base_color,
                value: Value::Rgba(color),
            }],
        )
        .await
    }
}

use crate::ids::SessionId;

/// The world verbs, each documented once and answering the same way on both
/// build targets.
impl Entity {
    /// Spawns an instance of an archetype and hands back the handle to it.
    ///
    /// This is how content enters a session. A content mod spawns its props in
    /// `init` and keeps the handles for the rest of the run. A gamemode spawns
    /// a body per arrival and then binds the arriving participant to it with
    /// [`control`](Entity::control).
    ///
    /// Server realm, and only that. A client half renders what the session tells
    /// it about and creates nothing of its own.
    ///
    /// # What the archetype may be
    ///
    /// There are two forms, and which of them is reachable depends on whose
    /// archetype it is.
    ///
    /// **The mod's own, as the item its manifest mints.** The declaration is a
    /// row in `mod.toml`,
    ///
    /// ```toml
    /// [[declares.archetype]]
    /// id = "door"
    /// scene = "door.glb"
    /// ```
    ///
    /// and [`declares!`](crate::declares) turns that `id` into
    /// `archetype::Door`, reached in a real crate as
    /// `protocol::archetype::Door`. The item carries the declared name, so a
    /// misspelling is a compile error rather than a refusal in a session. A
    /// mod's own declarations are looked up first, so it can always spawn its
    /// own content whatever else the session runs.
    ///
    /// **Any name, as the string it is.** Another mod's archetype is spelled in
    /// full, `author:mod/archetype/door`, which is the id the host keys it
    /// under; this mod's manifest does not list what another mod ships, so
    /// there is no item to mint. The one name the host itself publishes is
    /// `character`, the built-in player body, which is what a gamemode spawns
    /// when it owns placement. Writing this mod's own name as a string is legal
    /// too, and reaches the same archetype the item does.
    ///
    /// # Example
    ///
    /// A door mod hanging its own door, and a gamemode's body beside it, so
    /// both ways of naming an archetype stand next to each other.
    ///
    /// ```
    /// use ironlark::server::prelude::*;
    /// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
    /// // A real crate writes `ironlark::declares!("../mod.toml")` in a file
    /// // both halves include, and the `[[declares.archetype]] id = "door"`
    /// // above is what mints `archetype::Door`. `Entity`, `SpawnPoint` and
    /// // `Vec3` all arrive from the prelude on the first line.
    /// async fn hang_a_door() {
    ///     let at = SpawnPoint {
    ///         position: Vec3::new(-4.0, 0.0, 4.0),
    ///         yaw: 0.0,
    ///     };
    ///     match Entity::spawn(protocol::archetype::Door, at).await {
    ///         Ok(door) => {
    ///             if let Err(e) = door.set_id("door/north").await {
    ///                 log::error!("the door is unreachable by id: {e}");
    ///             }
    ///         }
    ///         Err(e) => log::error!("no door this session: {e}"),
    ///     }
    ///     // The host's own body archetype is declared in no manifest, so
    ///     // there is no item for it and the name travels as a string.
    ///     if let Err(e) = Entity::spawn("character", at).await {
    ///         log::error!("no body this session: {e}");
    ///     }
    /// }
    /// ```
    ///
    /// # What refuses
    ///
    /// A name that is neither one of the caller's own declarations, nor a full
    /// id the registry carries, nor the host's own, which reads as
    /// [`Other`](crate::ErrorKind::Other) with the name in the message. Nothing
    /// about the transform can refuse. See [`SpawnPoint`].
    ///
    /// # What you get back
    ///
    /// An owned handle. Clone it freely, and the host's side of it is released
    /// when the last copy drops, which frees the handle and leaves the instance
    /// standing in the world. To keep one past the handler, put a clone in a
    /// [`State`](crate::State) cell. See [`Entity`].
    ///
    /// The instance exists from the moment this returns, but a scene-backed
    /// archetype loads asynchronously, so its authored nodes are not reachable
    /// with [`part`](Entity::part) for the first few frames. It also carries no
    /// id until [`set_id`](Entity::set_id) gives it one, and an event raised in
    /// that window reaches its mod with no id in it.
    pub async fn spawn(archetype: impl AsRef<str>, at: SpawnPoint) -> Result<Entity> {
        Self::spawn_at_target(archetype.as_ref(), at).await
    }

    /// Binds a participant's camera and input to this entity, making it their
    /// body.
    ///
    /// The second half of owning placement. A gamemode spawns a `character` at a
    /// point it chose and calls this to put an arriving participant in it. From
    /// that moment their keys drive it and their camera rides it, and it is what
    /// [`body_of`] and [`Player::body`](crate::server::Player::body) answer for
    /// them.
    ///
    /// # Gamemode only
    ///
    /// A mod that does not hold the session's gamemode role is refused, and so
    /// is every mod when the session has no gamemode installed. The reason is
    /// the size of what the verb gives away. A mod able to bind any participant
    /// to a body it owns takes that participant's camera and input, which is not
    /// a thing content should be able to do to a person.
    ///
    /// # What refuses
    ///
    /// The gamemode gate. A session id that names nobody in this session. An
    /// entity that is not a body, which means the host's `character` and nothing
    /// else. A body that no longer exists. For a remote participant, one whose
    /// connection the host cannot resolve. All of them read as
    /// [`Other`](crate::ErrorKind::Other) with the reason in the message.
    ///
    /// # One body at a time is the body's rule, not the participant's
    ///
    /// A body already bound to somebody is released from them first, so no two
    /// participants ever share one. Nothing checks the other direction. Binding
    /// a participant who already controls something else leaves them marked on
    /// both, both bodies keep obeying their input, and what the host then
    /// answers for their body depends on which participant it is. For the person
    /// at the machine hosting the session it resolves neither, so [`body_of`]
    /// refuses. For everybody else it resolves whichever of the two it reaches
    /// first, and nothing says which. A gamemode that may re-body somebody calls
    /// [`release`](Entity::release) on the old body first and stays out of this.
    pub async fn control(&self, player: SessionId) -> Result<()> {
        self.control_at_target(player).await
    }

    /// Ends possession without ending the entity.
    ///
    /// The inverse of [`control`](Entity::control) and the gamemode's alone for
    /// the same reason. The participant loses camera and input. The entity stays
    /// exactly where it was, still replicating, now controlled by nobody. This
    /// is how a body outlives the player who wore it — a corpse left standing
    /// where it fell, a body kept in place while its player is put into another.
    ///
    /// Every writer-scoped row claimed on the body goes back to its defaults
    /// here, whichever mod wrote it, because after the release nothing has the
    /// reach to update or clear it. A line of text over somebody does not
    /// outlive the possession it named. See [`set_label`](Entity::set_label).
    ///
    /// # What refuses
    ///
    /// The gamemode gate, and a body that no longer exists. Both read as
    /// [`Other`](crate::ErrorKind::Other).
    ///
    /// # What is not built
    ///
    /// Camera teardown. The released body keeps the camera child that possession
    /// gave it, because that work belongs with the body-teardown half of
    /// replication.
    pub async fn release(&self) -> Result<()> {
        self.release_at_target().await
    }

    /// Removes the entity and everything beneath it from the world.
    ///
    /// The whole instance goes, children included, so a scene-backed archetype
    /// leaves nothing of its tree behind. Every id this mod or any other gave
    /// it goes with it, so a later [`by_id`](Entity::by_id) answers nothing
    /// rather than a dead handle.
    ///
    /// # Only the mod that spawned it
    ///
    /// Deliberately narrower than every other verb here. A mod that declares the
    /// archetype may drive an instance and write its rows, because it ships the
    /// behaviour. It may not destroy one, because the instance exists on some
    /// other mod's account and that mod's bookkeeping is what a despawn
    /// invalidates. Handling a participant's event does not reach here either.
    /// An event lets a mod affect a player, never end that player's body.
    ///
    /// Two things cross from outside. An operator's `[grants]` row naming
    /// `remove` over the entity's owner, and the gamemode role, which carries a
    /// remove grant over every owner for the session. Clearing the field between
    /// rounds is the session's own business. Neither reaches a player's body,
    /// however wide the row is written, and that is settled before any grant is
    /// consulted. The spawner still reaches its own, so a gamemode that spawned
    /// a body may remove it.
    ///
    /// # What refuses
    ///
    /// Two refusals are the SDK's, minted in the guest before the call leaves. A
    /// handle the host lent for an event refuses as
    /// [`Refused`](crate::ErrorKind::Refused), because spending one would burn
    /// the host's own handle. A handle already spent by another clone's despawn
    /// refuses as [`StaleId`](crate::ErrorKind::StaleId).
    ///
    /// The rest are the host's, and read as
    /// [`Other`](crate::ErrorKind::Other): not the spawner and holding no grant,
    /// a body somebody else spawned, an entity no mod owns, an entity already
    /// gone.
    ///
    /// It consumes the handle, and it spends the entity for every clone at once.
    /// A copy used afterwards refuses with
    /// [`StaleId`](crate::ErrorKind::StaleId) rather than resolving to whatever
    /// took its number. That holds even when the host refuses the removal, so a
    /// mod cannot retry through the copy it kept.
    pub async fn despawn(self) -> Result<()> {
        self.despawn_at_target().await
    }

    /// Writes fields into one component row on this entity.
    ///
    /// The general form of the typed shortcuts above, and the way to a row or a
    /// path they do not cover. The component and the fields are resolved ids, so
    /// the names are spelled once in `init` and never on this call. The batch is
    /// what makes it worth reaching for, since two paths of one row written
    /// together are one crossing and one announce instead of two.
    ///
    /// `transform`, `material` and `label` are rows a session publishes, and
    /// the published set is the capability boundary. It grows by the host
    /// publishing another row, never by a mod reaching further.
    /// [`resolve::component`](crate::server::resolve::component) resolves a row
    /// and [`resolve::field`](crate::server::resolve::field) resolves its paths.
    /// [`Field`] carries the example.
    ///
    /// A write restating the value the entity already holds is dropped by the
    /// host rather than replicated, and a batch that nets to nothing never
    /// reaches the wire. A player's body is the exception. Its pose is written
    /// through a route of its own, whose sent fields are announced as they are.
    ///
    /// # Whether the write lands is the reach rule's answer
    ///
    /// Not this call's, and it is taken at the write rather than when the handle
    /// was obtained. Four things admit a caller, checked in this order.
    ///
    /// - It spawned the entity. This is why a mod may move and paint its own
    ///   props, and why a gamemode may read and move the bodies it spawned at
    ///   any time, with no event to stand on.
    /// - It declares the archetype the entity instantiates, whoever spawned it.
    ///   The mod that ships the content ships the behaviour its instances run,
    ///   wherever they came from. Never true of a player's body, whose archetype
    ///   is the host's own.
    /// - The entity is the body of the participant whose event it is handling.
    ///   Only an interaction and a contact confer that. A join, a leave, a tick
    ///   and a signal confer nobody, so this path is open exactly inside the
    ///   handler that was given the participant, and only while it runs. It also
    ///   ages out. An interaction or a contact the host admitted many ticks
    ///   before the handler ran still arrives, because the mod's bookkeeping
    ///   needs it, and it carries no standing over the player by then.
    /// - The entity is a possessed body and the caller named THIS row under
    ///   `body-rows` in its manifest. `label` is a decoration row and qualifies.
    ///   That is how a mod puts a line of text over everybody from its own tick
    ///   without handling anybody's event, and it confers that row and nothing
    ///   else. One mod holds a body row per session. A second mod declaring a
    ///   row already held is refused it at session start, with both mods named
    ///   in the log.
    ///
    /// Failing all four, an operator's `[grants]` row naming `write` over the
    /// entity's owner is the only way across mods, and no grant reaches a
    /// player's body. A grant writes the rows an entity already carries and
    /// never attaches one, so a write through a grant onto a missing `label`
    /// refuses instead of creating it. An entity no mod owns — map content, the
    /// host's own bodies — is outside every scope.
    ///
    /// # A row that answers to its writer
    ///
    /// `label` is writer-scoped. While it holds anything but its defaults, only
    /// the mod that wrote it may change it, however much reach another mod has.
    /// Anyone with reach may still write the whole row back to its defaults,
    /// which releases it for the next writer.
    ///
    /// # What refuses
    ///
    /// The reach rule above, an id the session does not carry, and everything on
    /// [`Field`] and [`Value`]. Reach refusals arrive as
    /// [`Other`](crate::ErrorKind::Other) carrying a sentence that names the
    /// caller, the owner and the remedy, so log the message rather than the kind.
    pub async fn set_component(
        &self,
        component: crate::ids::ComponentId,
        fields: impl IntoIterator<Item = Field>,
    ) -> Result<()> {
        self.set_component_at_target(component, fields).await
    }

    /// Reads named fields of one component row.
    ///
    /// The answer carries one [`Field`] per path asked for, in the order they
    /// were asked for, each labelled with the id the caller used. Nothing is
    /// omitted and nothing is added. A read of three paths answers three values
    /// or refuses. [`Value`] carries the example.
    ///
    /// # Gated like a write
    ///
    /// The same reach rule as [`set_component`](Entity::set_component), with two
    /// differences. A grant needs `read` rather than `write`, and the gamemode
    /// role carries one of those over every owner for the session. The second
    /// difference is that the decoration path is not open to reads at all. A mod
    /// that may LABEL every body may not READ every body, because a standing
    /// read of every body is a positional feed of the whole server.
    ///
    /// That is the one place this surface withholds a fact rather than an act,
    /// and it is deliberate. Everything else a mod wants to know about the world
    /// it may know.
    ///
    /// # What refuses
    ///
    /// An id the session does not carry reads as
    /// [`UnresolvedName`](crate::ErrorKind::UnresolvedName), and so does a field
    /// id paired with a component it does not belong to.
    ///
    /// The rest read as [`Other`](crate::ErrorKind::Other): the reach rule, a
    /// path the component does not have, a field whose engine type is outside
    /// the six shapes on [`Value`], and an entity that does not carry the row at
    /// all. A row that is merely unattached says so in its own words, so `label`
    /// before its first write reads back as absent-for-now rather than absent.
    pub async fn get_component(
        &self,
        component: crate::ids::ComponentId,
        fields: &[crate::ids::FieldId],
    ) -> Result<Vec<Field>> {
        self.get_component_at_target(component, fields).await
    }

    /// Resolves a named descendant of this entity to a handle of its own.
    ///
    /// **This surface is subject to change.** Reading into a model's node tree
    /// is planned to move onto a surface of its own, so code written against
    /// `part` should expect a rewrite when that lands.
    ///
    /// The path is the names the artist gave the nodes, `"a/b"` stepping down.
    /// An empty path is this entity. Unnamed nodes are transparent, so the path
    /// follows what is named rather than the file's shape, and a model
    /// re-exported with different wrapper nodes still resolves the same way.
    ///
    /// It exists because a row usually belongs to one piece of a model rather
    /// than the whole of it. A mod recolouring a door reaches the `panel` node
    /// through here first and paints that, so the write lands on the panel
    /// rather than on the frame and the hinges as well. See
    /// [`set_base_color`](Entity::set_base_color), which paints a whole subtree.
    ///
    /// # What refuses
    ///
    /// A path no named descendant answers to, and an entity that is gone. Both
    /// read as [`Other`](crate::ErrorKind::Other).
    ///
    /// A scene loads asynchronously, so a part is absent for the first frames of
    /// an entity's life and the call refuses until it arrives. That is not an
    /// error state to give up on. Log it and let the next tick or the next press
    /// try again. It is also why resolving parts in `init`, right after the
    /// spawn, does not work.
    ///
    /// # What you get back
    ///
    /// A handle of the mod's own, not one lent for an event, so it may be kept
    /// in a [`State`](crate::State) cell. No reach check runs here at all. What
    /// a mod may do through a part is decided at that write, so holding one
    /// proves nothing about being allowed to write it.
    pub async fn part(&self, path: &str) -> Result<Entity> {
        self.part_at_target(path).await
    }

    /// The one entity carrying exactly this id, in this mod's own ids.
    ///
    /// [`find`] matches a family. This matches one id. It is a map lookup on the
    /// host's side rather than a scan, and it never leaves the modding thread,
    /// so it takes no `await` and does not wait on the simulation.
    ///
    /// This is the lookup for the id a contact or an interaction handed over in
    /// a [`Target`](crate::server::Target). The handle in that target is lent
    /// for the event and stops answering after it, while the id outlives
    /// everything, so a handler that must come back to the instance on a later
    /// tick keeps the id and comes back through here. See [`Entity`].
    ///
    /// `Ok(None)` is an answer and not a refusal. Nothing this mod addressed
    /// holds that id. That includes an instance nobody has given one, so a
    /// spawn with no id is not findable and was never meant to be. Ids are
    /// scoped to the caller, so another author's `door` is not this one and
    /// never collides with it.
    ///
    /// Nothing else refuses in a session. Off the session target there is no
    /// world, and the call answers a typed error saying so.
    pub fn by_id(id: &str) -> Result<Option<Entity>> {
        by_id_at_target(id)
    }

    /// Gives this entity a stable id in this mod's own namespace.
    ///
    /// A handle is a thing of the moment. An id outlives everything. An id is
    /// what lets a mod come back to an instance on a later tick with
    /// [`by_id`](Entity::by_id), reach a whole family of them with [`find`],
    /// and recognise which of its instances an event is about, because the id
    /// is what a [`Target`](crate::server::Target) carries. A mod that spawns
    /// content usually gives it one in the same breath, `door/north` for one of
    /// two doors it hung.
    ///
    /// The id is scoped to the caller, so two mods may use the same one without
    /// meeting. Setting it a second time moves this mod's id to the new entity
    /// and leaves other mods' ids for it alone.
    ///
    /// An id routes nothing. An interaction or a contact reaches the mod that
    /// DECLARED the archetype, addressed or not. What the id changes is whether
    /// the event says which instance it was about.
    ///
    /// # What refuses
    ///
    /// Reach over the entity, as [`set_component`](Entity::set_component)
    /// describes it, minus both of its body paths. This takes neither the
    /// subject path nor the declared body row, so handling a participant's event
    /// lets a mod act on their body and never give it an id of its own, and a
    /// mod that declared a decoration row buys nothing here. A gamemode may
    /// still address a body it spawned itself, on the strength of having spawned
    /// it.
    ///
    /// An id a different live entity already holds also refuses, so the mapping
    /// from id to entity stays one to one. Both read as
    /// [`Other`](crate::ErrorKind::Other).
    ///
    /// What does NOT refuse is the spelling. Everywhere else a name is declared,
    /// the manifest parser insists on a charset. Here nothing checks, so an id
    /// with capitals or underscores is accepted and then cannot be spelled in a
    /// manifest. Spell each segment of an id the way a declared name is spelled
    /// — lowercase ASCII letters, digits and `-` — and it will never come up.
    pub async fn set_id(&self, id: &str) -> Result<()> {
        self.set_id_at_target(id).await
    }
}

/// The entities this mod has addressed, as a family.
///
/// A pattern names a family rather than a prefix. It matches the entity whose
/// id is exactly the pattern, plus everything under it in the hierarchy the two
/// separators mark out. `door` finds an entity with the id `door`, and it finds
/// `door/north` under the path separator and `door:north` under the instance
/// separator. It does not find `door-2`, because the pattern carries no
/// trailing separator of its own and the host adds the boundary itself. An
/// empty pattern is every id this mod minted.
///
/// This is the verb for acting on a set. A mod that gave its instances the ids
/// `door/north` and `door/south` reaches both with `door`, and a mod that went
/// one level deeper with `door/north/panel` reaches that one alone with
/// `door/north`. For exactly one id, [`Entity::by_id`] answers with one entity
/// or with nothing and no list to unwrap. The ids themselves come from
/// [`Entity::set_id`].
///
/// Only this mod's own ids are searched, so the answer never contains another
/// author's entity and two authors both using `door` never meet.
///
/// # A pattern that matches nothing
///
/// An empty list is the ordinary answer for a family that has no members. In a
/// session the call answers from an index on the host's own thread without
/// waiting on the simulation, so it takes no `await` and nothing about the
/// world can make it fail. Off the session target there is no world and it
/// answers a typed error saying so.
///
/// # The answer can be short and will not say so
///
/// The result is capped at 1024 entities. Past that the host truncates, writes
/// one warning line of its own, and hands back a full-looking list, so a mod
/// cannot tell a complete answer from a truncated one. A mod that might address
/// that many instances tracks them itself rather than re-deriving them here.
///
/// Every match costs the host a handle, so a broad pattern called every tick
/// churns the handle table for an answer the caller already had. Keep the
/// handles that came back from [`Entity::spawn`] in a [`State`](crate::State)
/// cell instead, and reach for this when the set is not the one the mod is
/// already holding.
/// ```
/// // The prelude mints find, Entity and Rgba.
/// use ironlark::server::prelude::*;
///
/// // Every instance this mod addressed under one prefix. The pattern matches
/// // the ids this mod gave with `Entity::set_id`, never another mod's.
/// async fn redden_the_doors() {
///     let doors = match find("door/*") {
///         Ok(doors) => doors,
///         Err(e) => {
///             log::error!("the door lookup failed: {e}");
///             return;
///         }
///     };
///     // The count first: an empty answer is a real answer.
///     log::info!("{} doors answer to this mod", doors.len());
///     for door in doors {
///         let red = Rgba { r: 1.0, g: 0.2, b: 0.2, a: 1.0 };
///         if let Err(e) = door.set_base_color(red).await {
///             log::warn!("a door did not redden: {e}");
///         }
///     }
/// }
/// ```
pub fn find(pattern: &str) -> Result<Vec<Entity>> {
    find_at_target(pattern)
}

/// The entity a participant is controlling at this moment.
///
/// The one direction the contract offers between a person and a thing in the
/// world, and the way a mod reaches somebody it did not spawn. A mod that puts
/// a line of text over every participant takes each body from its own tick this
/// way. A mod already handling that participant's event does not need this call
/// at all, because [`Player::body`](crate::server::Player::body) answers from
/// the handle the hook was given.
///
/// The argument is the participant's [`SessionId`], which
/// [`Player::session`](crate::server::Player::session) hands over free and which
/// addresses one participant for as long as they stay connected.
///
/// Holding a body is not being allowed to move it. What may be done through the
/// handle is the reach rule on [`Entity::set_component`], so a mod that neither
/// spawned the body nor is handling that participant's own event gets the handle
/// here and a refusal at the write.
///
/// Possession is what this answers, and not identity. A participant put into a
/// second body resolves to that one, and the body they were taken out of is not
/// reachable through them at all.
///
/// # What refuses
///
/// A session id that names nobody this session, and a participant controlling
/// nothing. Both read as [`Other`](crate::ErrorKind::Other), and the second is
/// ordinary rather than exceptional. A body appears a moment after the arrival,
/// so a mod that wants every body treats a refusal as "not yet" and retries on
/// its next tick rather than giving up on that participant.
///
/// A participant bound to two bodies at once answers differently depending on
/// which participant it is, which is the case described on [`Entity::control`]
/// seen from this end. For the person at the machine hosting the session it
/// refuses. For everybody else it answers one of the two, and nothing says
/// which.
///
/// # The handle is not the standing
///
/// What comes back is a handle of the mod's own that keeps addressing the body
/// afterwards. What may be DONE through it is checked at every use against the
/// event in flight, so this call succeeding says nothing about a write
/// succeeding. See [`Entity`] and [`Entity::set_component`].
///
/// # What is not built
///
/// The question the other way. An entity in hand cannot be asked whose body it
/// is, so a mod that found something by casting a ray cannot learn that a person
/// is in it.
/// ```
/// // The prelude mints ServerMod, Context, Player, SessionId and body_of.
/// use ironlark::server::prelude::*;
///
/// // Keep the id, never the handle: a Player is lent for one event.
/// ironlark::state! {
///     static PRESENT: Vec<SessionId> = Vec::new();
/// }
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn on_join(_ctx: Context, player: Player) {
///         PRESENT.update(|present| present.push(player.session()));
///     }
///
///     async fn on_tick(_ctx: Context, _dt: f32) {
///         for session in PRESENT.update(|present| present.clone()) {
///             // A fresh handle each time, from the id that outlived the event.
///             match body_of(session).await {
///                 Ok(body) => match body.translation().await {
///                     Ok(at) => log::trace!("{session} stands at {at:?}"),
///                     Err(e) => log::warn!("{session}: no position: {e}"),
///                 },
///                 Err(e) => log::trace!("{session} controls no body: {e}"),
///             }
///         }
///     }
/// }
/// ```
pub async fn body_of(player: SessionId) -> Result<Entity> {
    body_of_at_target(player).await
}
