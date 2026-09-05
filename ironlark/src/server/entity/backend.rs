//! The host-crossing arm of the entity verbs: the wasm arm calls the
//! contract, the native arm answers the typed no-session error.

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::super::{Entity, Field, SpawnPoint, Value};
    use crate::bindings::server::ironlark::host::entity as host;
    use crate::bindings::server::ironlark::host::types as wire;
    use crate::error::{Error, Result};
    use crate::ids::SessionId;
    use crate::math::Vec3;
    use core::mem::ManuallyDrop;
    use std::rc::Rc;

    use super::super::held::Held;

    fn wire_error(e: host::Error) -> Error {
        Error::from_wire(e.code, e.message, e.data)
    }

    fn to_wire_vec3(v: Vec3) -> wire::Vec3 {
        wire::Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }

    fn to_wire_value(v: Value) -> wire::FieldValue {
        match v {
            Value::Number(n) => wire::FieldValue::Number(n),
            Value::Boolean(b) => wire::FieldValue::Boolean(b),
            Value::Text(t) => wire::FieldValue::Text(t),
            Value::Vec3(v) => wire::FieldValue::Vec3(to_wire_vec3(v)),
            Value::Quat(q) => wire::FieldValue::Quat(wire::Quat {
                x: q.x,
                y: q.y,
                z: q.z,
                w: q.w,
            }),
            Value::Rgba(c) => wire::FieldValue::Rgba(wire::Rgba {
                r: c.r,
                g: c.g,
                b: c.b,
                a: c.a,
            }),
        }
    }

    fn from_wire_value(v: wire::FieldValue) -> Value {
        match v {
            wire::FieldValue::Number(n) => Value::Number(n),
            wire::FieldValue::Boolean(b) => Value::Boolean(b),
            wire::FieldValue::Text(t) => Value::Text(t),
            wire::FieldValue::Vec3(v) => Value::Vec3(Vec3 {
                x: v.x,
                y: v.y,
                z: v.z,
            }),
            wire::FieldValue::Quat(q) => Value::Quat(crate::math::Quat {
                x: q.x,
                y: q.y,
                z: q.z,
                w: q.w,
            }),
            wire::FieldValue::Rgba(c) => Value::Rgba(crate::math::Rgba {
                r: c.r,
                g: c.g,
                b: c.b,
                a: c.a,
            }),
        }
    }

    fn to_wire_fields(fields: impl IntoIterator<Item = Field>) -> Vec<host::ComponentField> {
        fields
            .into_iter()
            .map(|f| host::ComponentField {
                field: f.field.0,
                value: to_wire_value(f.value),
            })
            .collect()
    }

    impl Entity {
        pub(crate) fn from_owned(handle: host::Handle) -> Self {
            let raw = ManuallyDrop::new(handle).handle();
            Self {
                inner: Rc::new(Held::owned(raw)),
            }
        }

        /// A per-event handle the host still owns; dropping it releases
        /// nothing.
        pub(crate) fn from_borrowed_raw(raw: u32) -> Self {
            Self {
                inner: Rc::new(Held::lent(raw, crate::host::scope::epoch())),
            }
        }

        /// A non-dropping view of the shared handle, refused once the entity
        /// has been despawned.
        pub(crate) fn as_wire(&self) -> Result<ManuallyDrop<host::Handle>> {
            if !self.inner.answers_in(crate::host::scope::epoch()) {
                return Err(Error::from_wire(
                    crate::error::code::STALE_ID,
                    if self.inner.owned {
                        "the entity was despawned"
                    } else {
                        "this entity belongs to an event that is over; find it by id instead"
                    }
                    .into(),
                    Vec::new(),
                ));
            }
            Ok(ManuallyDrop::new(unsafe {
                host::Handle::from_handle(self.inner.raw())
            }))
        }

        /// Spawn an instance of an archetype: your own, or `:`-qualified
        /// another mod's content.
        pub(crate) async fn spawn_at_target(archetype: &str, at: SpawnPoint) -> Result<Entity> {
            let transform = host::SpawnPoint {
                position: to_wire_vec3(at.position),
                yaw: at.yaw,
            };
            match host::spawn(archetype.to_string(), transform).await {
                Ok(handle) => Ok(Entity::from_owned(handle)),
                Err(e) => Err(wire_error(e)),
            }
        }

        /// Bind a participant's camera and input to this body; gamemode-only.
        pub(crate) async fn control_at_target(&self, player: SessionId) -> Result<()> {
            let handle = self.as_wire()?;
            host::control(player.to_wire(), &handle)
                .await
                .map_err(wire_error)
        }

        /// Release this body from its controller; gamemode-only.
        pub(crate) async fn release_at_target(&self) -> Result<()> {
            let handle = self.as_wire()?;
            host::release(&handle).await.map_err(wire_error)
        }

        /// Destroys the entity and spends the handle, refused or not. Copies
        /// of this entity survive the call and refuse every later use.
        /// A handle the host lent for one event is refused: spending it would
        /// burn the host's own and kill the mod on its next use.
        pub(crate) async fn despawn_at_target(self) -> Result<()> {
            if !self.inner.owned {
                return Err(Error::from_wire(
                    crate::error::code::REFUSAL,
                    "the host owns this handle; only a spawned entity is yours to despawn".into(),
                    Vec::new(),
                ));
            }
            // Reads the liveness before spending it, so a second despawn is a
            // typed refusal rather than a double release.
            let handle = ManuallyDrop::into_inner(self.as_wire()?);
            self.inner.spend();
            host::despawn(handle).await.map_err(wire_error)
        }

        /// Writes fields into one registered component. A write restating
        /// the held value is dropped by the host.
        pub(crate) async fn set_component_at_target(
            &self,
            component: crate::ids::ComponentId,
            fields: impl IntoIterator<Item = Field>,
        ) -> Result<()> {
            let handle = self.as_wire()?;
            host::set_component(&handle, component.0, to_wire_fields(fields))
                .await
                .map_err(wire_error)
        }

        /// Reads named fields of one registered component, gated exactly as
        /// writing them is.
        pub(crate) async fn get_component_at_target(
            &self,
            component: crate::ids::ComponentId,
            fields: &[crate::ids::FieldId],
        ) -> Result<Vec<Field>> {
            let handle = self.as_wire()?;
            let ids: Vec<u32> = fields.iter().map(|f| f.0).collect();
            match host::get_component(&handle, component.0, ids).await {
                Ok(fields) => Ok(fields
                    .into_iter()
                    .map(|f| Field {
                        field: crate::ids::FieldId(f.field),
                        value: from_wire_value(f.value),
                    })
                    .collect()),
                Err(e) => Err(wire_error(e)),
            }
        }

        /// Resolve a named descendant; errors while an async scene load has
        /// not produced it yet.
        pub(crate) async fn part_at_target(&self, path: &str) -> Result<Entity> {
            let handle = self.as_wire()?;
            match host::part(&handle, path.to_string()).await {
                Ok(handle) => Ok(Entity::from_owned(handle)),
                Err(e) => Err(wire_error(e)),
            }
        }

        /// Give this entity a stable id in the caller's namespace.
        pub(crate) async fn set_id_at_target(&self, id: &str) -> Result<()> {
            let handle = self.as_wire()?;
            host::set_id(&handle, id.to_string())
                .await
                .map_err(wire_error)
        }
    }

    /// Resolve an id or `/`-prefix to live entities, within this mod's own ids.
    pub(crate) fn find_at_target(pattern: &str) -> Result<Vec<Entity>> {
        match host::find(pattern) {
            Ok(handles) => Ok(handles.into_iter().map(Entity::from_owned).collect()),
            Err(e) => Err(wire_error(e)),
        }
    }

    /// The one entity carrying exactly this id, in this mod's own ids.
    pub(crate) fn by_id_at_target(id: &str) -> Result<Option<Entity>> {
        match host::by_id(id) {
            Ok(handle) => Ok(handle.map(Entity::from_owned)),
            Err(e) => Err(wire_error(e)),
        }
    }

    /// The body a participant controls, while handling that participant's
    /// event.
    pub(crate) async fn body_of_at_target(player: SessionId) -> Result<Entity> {
        match host::body_of(player.to_wire()).await {
            Ok(handle) => Ok(Entity::from_owned(handle)),
            Err(e) => Err(wire_error(e)),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) use wasm::{body_of_at_target, by_id_at_target, find_at_target};

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::super::{Entity, Field, SpawnPoint};
    use crate::error::{Error, Result};
    use crate::ids::SessionId;

    fn no_session() -> Error {
        Error::from_wire(
            0,
            "world verbs need a session; run inside the host".into(),
            Vec::new(),
        )
    }

    impl Entity {
        /// Spawning needs a session.
        pub(crate) async fn spawn_at_target(_archetype: &str, _at: SpawnPoint) -> Result<Entity> {
            Err(no_session())
        }
        /// Possession needs a session.
        pub(crate) async fn control_at_target(&self, _player: SessionId) -> Result<()> {
            Err(no_session())
        }
        /// Releasing needs a session.
        pub(crate) async fn release_at_target(&self) -> Result<()> {
            Err(no_session())
        }
        /// Despawning needs a session.
        pub(crate) async fn despawn_at_target(self) -> Result<()> {
            Err(no_session())
        }
        /// Writing a component needs a session.
        pub(crate) async fn set_component_at_target(
            &self,
            _component: crate::ids::ComponentId,
            _fields: impl IntoIterator<Item = Field>,
        ) -> Result<()> {
            Err(no_session())
        }
        /// Reading a component needs a session.
        pub(crate) async fn get_component_at_target(
            &self,
            _component: crate::ids::ComponentId,
            _fields: &[crate::ids::FieldId],
        ) -> Result<Vec<Field>> {
            Err(no_session())
        }
        /// Resolving a part needs a session.
        pub(crate) async fn part_at_target(&self, _path: &str) -> Result<Entity> {
            Err(no_session())
        }
        /// Giving an entity an id needs a session.
        pub(crate) async fn set_id_at_target(&self, _id: &str) -> Result<()> {
            Err(no_session())
        }
    }

    /// Off the session target: no world, so nothing is ever found.
    pub(crate) fn find_at_target(_pattern: &str) -> Result<Vec<Entity>> {
        Err(no_session())
    }

    /// Off the session target: no world, so no id resolves.
    pub(crate) fn by_id_at_target(_id: &str) -> Result<Option<Entity>> {
        Err(no_session())
    }

    /// Off the session target: no world, so nobody controls a body.
    pub(crate) async fn body_of_at_target(_player: SessionId) -> Result<Entity> {
        Err(no_session())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::{body_of_at_target, by_id_at_target, find_at_target};
