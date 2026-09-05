//! The wasm boundary: blanket impls turning the generated export traits into
//! the SDK's realm traits plus the dispatch behind a declared name, converting
//! wire shapes to the typed ones. Structural hooks answer allow until their
//! mechanism fires.

use crate::bindings::client::exports::ironlark::host::client_api as capi;
use crate::bindings::server::exports::ironlark::host::server_api as sapi;
use crate::bindings::server::ironlark::host::player as host_player;
use crate::bindings::server::ironlark::host::types as wire;
use crate::client::half::ClientMod;
use crate::client::input::InputEdge;
use crate::context::Context;
use crate::error::{Result, code};
use crate::host::{dispatch, log_bridge};
use crate::ids::{EventId, RequestId, SessionId, SignalId, SourceId, Tick};
use crate::math::Vec3;
use crate::server::events::{ContactEdge, LeaveReason, PhysicsObject, Target};
use crate::server::half::ServerMod;
use crate::server::player::Player;

fn vec3(v: wire::Vec3) -> Vec3 {
    Vec3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

fn context(c: wire::Context) -> Context {
    Context {
        id: EventId::from_wire(c.id),
        raised_at: Tick::new(c.raised_at),
        now: Tick::new(c.now),
    }
}

/// An instance nobody addressed arrives with an empty id, which no legal id can
/// be. Every id crossing the boundary passes through here, so absent is what an
/// author sees and nothing compares against "".
fn absent_when_empty(id: String) -> Option<String> {
    (!id.is_empty()).then_some(id)
}

fn target(t: sapi::TargetRef<'_>) -> Target {
    Target {
        entity: t
            .entity
            .map(|h| crate::server::entity::Entity::from_borrowed_raw(h.handle())),
        id: absent_when_empty(t.id),
    }
}

fn player(p: &host_player::Player) -> Player {
    Player::from_borrow(p)
}

impl<T: ServerMod> sapi::Guest for T {
    async fn init() {
        log_bridge::install();
        <T as ServerMod>::init().await;
    }

    async fn on_join(ctx: wire::Context, p: &host_player::Player, arrival: sapi::Arrival) {
        crate::host::scope::open_scope();
        // Until meshing exists every arrival is `fresh`, so the hook does not
        // take an argument whose value is known.
        let _ = arrival;
        <T as ServerMod>::on_join(context(ctx), player(p)).await;
    }

    async fn on_leave(ctx: wire::Context, p: &host_player::Player, reason: sapi::LeaveReason) {
        crate::host::scope::open_scope();
        let reason = match reason {
            sapi::LeaveReason::Quit => LeaveReason::Quit,
            sapi::LeaveReason::Kicked => LeaveReason::Kicked,
            sapi::LeaveReason::Timeout => LeaveReason::Timeout,
        };
        <T as ServerMod>::on_leave(context(ctx), player(p), reason).await;
    }

    async fn on_tick(ctx: wire::Context, dt: f32) {
        crate::host::scope::open_scope();
        <T as ServerMod>::on_tick(context(ctx), dt).await;
    }

    async fn on_interact(
        ctx: wire::Context,
        p: &host_player::Player,
        t: sapi::TargetRef<'_>,
        hit: wire::Vec3,
        distance: f32,
    ) {
        crate::host::scope::open_scope();
        <T as ServerMod>::on_interact(context(ctx), player(p), target(t), vec3(hit), distance)
            .await;
    }

    async fn on_contact(
        ctx: wire::Context,
        t: sapi::TargetRef<'_>,
        other: sapi::PhysicsObject,
        point: wire::Vec3,
        edge: sapi::ContactEdge,
    ) {
        crate::host::scope::open_scope();
        let other = match other {
            sapi::PhysicsObject::Player(session) => PhysicsObject::Player(SessionId::new(session)),
            sapi::PhysicsObject::Entity(id) => PhysicsObject::Entity(absent_when_empty(id)),
            sapi::PhysicsObject::MapGeometry => PhysicsObject::MapGeometry,
        };
        let edge = match edge {
            sapi::ContactEdge::Started => ContactEdge::Started,
            sapi::ContactEdge::Ended => ContactEdge::Ended,
        };
        <T as ServerMod>::on_contact(context(ctx), target(t), other, vec3(point), edge).await;
    }

    async fn on_request(
        ctx: wire::Context,
        p: &host_player::Player,
        request: u32,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, wire::Error> {
        crate::host::scope::open_scope();
        dispatch::dispatch_request(context(ctx), player(p), RequestId::new(request), payload)
            .await
            .map_err(|refusal| wire::Error {
                code: code::REFUSAL,
                message: refusal.to_string(),
                data: Vec::new(),
            })
    }

    async fn on_signal(ctx: wire::Context, signal: u32, source: u32, payload: Vec<u8>) {
        crate::host::scope::open_scope();
        dispatch::dispatch_signal(
            context(ctx),
            SignalId::new(signal),
            SourceId::new(source),
            payload,
        )
        .await;
    }

    async fn on_body_created(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_body_destroyed(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_entity_spawned(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_entity_despawned(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_component_changed(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_map_loaded(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_map_unloaded(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_session_started(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_session_ended(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_possession_changed(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn on_profile_swapped(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
    async fn body_spawn_requested(_ctx: wire::Context) -> sapi::Verdict {
        sapi::Verdict::Allow
    }
}

impl<T: ClientMod> capi::Guest for T {
    async fn init() {
        log_bridge::install();
        <T as ClientMod>::init().await;
    }

    async fn on_tick(ctx: wire::Context, dt: f32) {
        crate::host::scope::open_scope();
        <T as ClientMod>::on_tick(context(ctx), dt).await;
    }

    /// This machine's own client bus and the raises crossing from the server
    /// realm arrive through this one export, so one binding kind serves both.
    async fn on_signal(ctx: wire::Context, signal: u32, source: u32, payload: Vec<u8>) {
        crate::host::scope::open_scope();
        dispatch::dispatch_signal(
            context(ctx),
            SignalId::new(signal),
            SourceId::new(source),
            payload,
        )
        .await;
    }

    async fn on_input(ctx: wire::Context, hook: u32, edge: capi::InputEdge) {
        crate::host::scope::open_scope();
        let edge = match edge {
            capi::InputEdge::Pressed => InputEdge::Pressed,
            capi::InputEdge::Released => InputEdge::Released,
        };
        <T as ClientMod>::on_input(context(ctx), hook, edge).await;
    }
}
