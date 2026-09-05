//! Name to id resolution, one implementation per kind the contract can
//! resolve. A name resolves once and every hot verb takes the id it answered.
//! An id is numbered per session, never persisted, and never a save key.
//!
//! A hook has no function here. The declarations mint hook ids and the dispatch
//! carries one, so there is nothing for a mod to ask by name.

use crate::error::Result;
use crate::ids::{ComponentId, FieldId, RequestId, SignalId, SoundId, SourceId};

#[cfg(target_arch = "wasm32")]
mod backend {
    use crate::bindings::server::ironlark::host::resolve as host;
    use crate::error::{Error, Result};

    fn map(r: Result<u32, host::Error>) -> Result<u32> {
        r.map_err(|e| Error::from_wire(e.code, e.message, e.data))
    }

    pub fn signal(name: &str) -> Result<u32> {
        map(host::signal(name))
    }
    pub fn request(name: &str) -> Result<u32> {
        map(host::request(name))
    }
    pub fn sound(name: &str) -> Result<u32> {
        map(host::sound(name))
    }
    pub fn component(name: &str) -> Result<u32> {
        map(host::component(name))
    }
    pub fn field(component: u32, path: &str) -> Result<u32> {
        map(host::field(component, path))
    }
    pub fn source(name: &str) -> Result<u32> {
        map(host::source(name))
    }
}

// The native double numbers names deterministically per kind, in first-use
// order, so dispatch logic is testable without a session.
#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use crate::error::Result;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static TABLES: RefCell<HashMap<(&'static str, String), u32>> =
            RefCell::new(HashMap::new());
        static NEXT: RefCell<u32> = const { RefCell::new(1) };
    }

    fn resolve(kind: &'static str, name: &str) -> Result<u32> {
        TABLES.with(|t| {
            let mut t = t.borrow_mut();
            if let Some(id) = t.get(&(kind, name.to_string())) {
                return Ok(*id);
            }
            let id = NEXT.with(|n| {
                let mut n = n.borrow_mut();
                let id = *n;
                *n += 1;
                id
            });
            t.insert((kind, name.to_string()), id);
            Ok(id)
        })
    }

    pub fn signal(name: &str) -> Result<u32> {
        resolve("signal", name)
    }
    pub fn request(name: &str) -> Result<u32> {
        resolve("request", name)
    }
    pub fn sound(name: &str) -> Result<u32> {
        resolve("sound", name)
    }
    pub fn component(name: &str) -> Result<u32> {
        resolve("component", name)
    }
    pub fn field(component: u32, path: &str) -> Result<u32> {
        resolve("field", &format!("{component}/{path}"))
    }
    pub fn source(name: &str) -> Result<u32> {
        resolve("source", name)
    }
}

/// The session's number for a declared signal.
///
/// Neither realm's door re-exports it, because no author calls it. A signal is
/// met as its own payload type — the mod's own, or another mod's imported into
/// its schema — and [`signal`](crate::server::signal),
/// [`signal_to`](crate::server::signal_to) and
/// [`SignalSpec::observe`](crate::protocol::SignalSpec::observe) all take that
/// type. The name behind it is resolved here, once, on first use, and the answer is kept.
/// This is where every kind the contract can resolve has one implementation and
/// one place a refusal is shaped. See [`SignalId`].
///
/// # What refuses
///
/// A name no enabled mod declares, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName). The caller's own
/// declarations are consulted for a bare name, the named owner's for a name
/// carrying `:`. The same refusal covers a name declared in the other realm and
/// one declared as a different kind: a session numbers signals in their own
/// table, so asking for a request's name here is refused rather than answered
/// with a number that would mean something else.
pub fn signal(name: &str) -> Result<SignalId> {
    backend::signal(name).map(SignalId)
}

/// The session's number for a declared request.
///
/// Not re-exported by either door, for the reason [`signal`] gives.
/// [`request`](crate::client::request) and
/// [`RequestSpec::respond`](crate::protocol::RequestSpec::respond) are given the
/// request's own payload type, and the name behind it is resolved here when the
/// call or the registration first needs it. See [`RequestId`].
///
/// # What refuses
///
/// A name no enabled mod declares, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName), and equally a name
/// that is declared as a signal rather than a request.
pub fn request(name: &str) -> Result<RequestId> {
    backend::request(name).map(RequestId)
}

/// The session's number for a sound, from the name a declaration gives it.
///
/// The route for a name nothing can know while the crate compiles. A mod's own
/// declared sounds arrive as items a play takes directly, resolved once behind
/// their own cell, so a mod playing a sound it ships itself never calls this.
/// What is left for it is a name settled at run time, and another mod's sound,
/// written in full as `"author:mod/sound/name"`. This mod's manifest does not
/// list what another mod ships, so there is no item to mint and the name has to
/// be carried as text until the session can answer it.
///
/// A bare name is this mod's own declaration. A name carrying `:` names
/// another mod's sound and is checked against that mod's manifest instead,
/// which is how one mod plays a sound another mod ships.
///
/// It belongs to no realm and both doors carry it,
/// [`server::resolve`](crate::server::resolve) and
/// [`client::resolve`](crate::client::resolve) alike. The id it answers is
/// taken by the server realm's [`audio::play`](crate::server::audio::play),
/// which every participant hears, and by the client realm's
/// [`audio::play`](crate::client::audio::play), which this machine alone
/// hears. What may be played, and how it is named, is the same on both.
///
/// Resolve once and keep the answer. A resolve is a crossing into the host, so
/// one inside a per-press or per-tick path pays it every time. The id is `Copy`
/// and session-scoped, and it hands out no number, so nothing can fill a schema
/// field with it and no payload or save can carry one. See [`SoundId`].
///
/// The listing below imports the server realm's prelude. A client half writes
/// the same call under [`client::prelude`](crate::client::prelude), which
/// carries `resolve`, `audio`, [`State`](crate::State), [`SoundId`] and
/// [`SoundBus`](crate::SoundBus) as well.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// // The cell that keeps the answer for the rest of the session.
/// ironlark::state! {
///     static ALARM: Option<SoundId> = None;
/// }
///
/// // The name is another mod's, spelled in full: its author, its mod, the
/// // kind, and the name that mod declared under `sounds` in its own mod.toml.
/// async fn sound_the_alarm() {
///     let id = match ALARM.get() {
///         Some(id) => id,
///         None => match resolve::sound("author:mod/sound/alarm") {
///             Ok(id) => {
///                 ALARM.set(Some(id));
///                 id
///             }
///             Err(e) => {
///                 log::error!("this session carries no such sound: {e}");
///                 return;
///             }
///         },
///     };
///     if let Err(e) = audio::play(id, SoundBus::Effects).await {
///         log::warn!("the alarm was not heard: {e}");
///     }
/// }
/// ```
///
/// # What refuses
///
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName), when nobody declares
/// the name. This mod naming a sound absent from its own `[declares]` sounds
/// is one case, naming another mod's sound that mod does not declare is the
/// other, and the message says which. An undeclared sound dies here, by name,
/// instead of playing as silence nobody can account for.
pub fn sound(name: &str) -> Result<SoundId> {
    backend::sound(name).map(SoundId)
}

/// The session's number for a host-published accessible component.
///
/// A component is one row of world data, and the set a mod may name is a
/// whitelist the host publishes rather than the engine's own component list.
/// So this call is the boundary of what world data a mod may touch at all.
/// Capability grows by the host publishing another row, never by a mod
/// reaching further into the world.
///
/// This is the long way round, and it is the way to a row that has no verb of
/// its own. The typed helpers on [`Entity`](crate::server::Entity), among them
/// [`set_label`](crate::server::Entity::set_label) and
/// [`set_base_color`](crate::server::Entity::set_base_color), resolve these
/// same rows once internally, so a mod writing a row one of them covers never
/// names the component itself. Resolve one by hand when assembling a
/// [`Field`](crate::server::Field) for a row those helpers do not cover.
///
/// Resolving a component says which row. It says nothing about which entities
/// the caller may write to. That is the ownership rule and it is taken at the
/// write, so an id in hand is not permission to use it on a body another mod
/// spawned. Resolve in `init`, keep the id in a [`State`](crate::State) cell,
/// and pair it with the [`field`](crate::server::resolve::field) ids resolved
/// under it. See [`ComponentId`].
///
/// ```
/// // The prelude mints ServerMod, ComponentId and the resolve module.
/// use ironlark::server::prelude::*;
///
/// // One crossing into the host for the session, not one per write.
/// ironlark::state! {
///     static MATERIAL: Option<ComponentId> = None;
/// }
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn init() {
///         // `material` is a row the host publishes. A name outside the
///         // published set is what refuses here.
///         match resolve::component("material") {
///             Ok(row) => MATERIAL.set(Some(row)),
///             Err(e) => log::error!("this session publishes no material row: {e}"),
///         }
///     }
/// }
/// ```
///
/// # What refuses
///
/// A name outside the published set, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) saying it is not an
/// accessible component.
pub fn component(name: &str) -> Result<ComponentId> {
    backend::component(name).map(ComponentId)
}

/// The session's number for one dotted path inside one component.
///
/// `"translation"` within `transform`, `"base_color"` within `material`. The
/// pairing is what is numbered. The same path under a different component is a
/// different id, and passing one to a call that names another component is
/// refused rather than written into the wrong row. Carry the answer in a
/// [`Field`](crate::server::Field) alongside the value to write.
///
/// Resolving does not prove the path exists. The id is only the session's
/// number for a string, minted the first time it is asked for, and whether the
/// world has such a field is judged when the field is first used. So a
/// misspelt path resolves happily and then refuses at the first
/// [`set_component`](crate::server::Entity::set_component). See [`FieldId`].
///
/// ```
/// // The prelude mints ServerMod, ComponentId, FieldId and the resolve
/// // module.
/// use ironlark::server::prelude::*;
///
/// // The row and the path under it, kept together: a path is numbered under
/// // one component and means nothing under another.
/// ironlark::state! {
///     static EMISSIVE: Option<(ComponentId, FieldId)> = None;
/// }
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn init() {
///         // `material` is a row the host publishes, `emissive` a path in it.
///         let Ok(row) = resolve::component("material") else {
///             log::error!("this session publishes no material row");
///             return;
///         };
///         match resolve::field(row, "emissive") {
///             Ok(path) => EMISSIVE.set(Some((row, path))),
///             Err(e) => log::error!("the session's path table refused: {e}"),
///         }
///     }
/// }
/// ```
///
/// # What refuses
///
/// A component id that names no accessible row, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName). Beyond that, the
/// session caps how many distinct paths it has minted. Paths come from mod
/// code, so the cap bounds what a mod minting garbage can grow the table to,
/// and meeting it answers [`TooLarge`](crate::ErrorKind::TooLarge) saying the
/// session's table is full rather than blaming this path. Resolving the
/// handful of paths a mod uses in `init` and keeping them stays under the cap
/// by construction. Resolving inside a per-tick loop is what eventually meets
/// it.
pub fn field(component: ComponentId, path: &str) -> Result<FieldId> {
    backend::field(component.0, path).map(FieldId)
}

/// The session's number for an enabled mod, from its full id `"author:mod"`.
///
/// The one resolve whose answer is compared rather than passed to a verb. An
/// [`observe`](crate::protocol::SignalSpec::observe) handler receives a
/// host-stamped [`SourceId`] between the context and the payload, saying which mod raised
/// the signal. The raiser never supplies it and cannot forge it, because the
/// host looks the raising mod up and stamps it.
///
/// It matters more than it looks. Raising a name is open to any mod in the
/// realm, not only the mod that declared it, so a subscriber that must obey
/// one raiser and ignore the rest has this compare and nothing else.
///
/// A mod with something to protect resolves the id it will accept once, keeps
/// it, and tests the stamped one against it. A policy as strict as "only the
/// gamemode commands me" is then one integer compare per signal instead of a
/// string match, and a mod that acts on any raiser is one that decided the act
/// is harmless whoever asked.
///
/// Resolve in `init` and keep the answer in a [`State`](crate::State) cell.
/// See [`SourceId`].
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // `protocol` is generated from the mod's own protocol.proto; `Opened` is
/// // the payload type, and its one option line is the declaration.
/// ironlark::state! {
///     static ARENA: Option<SourceId> = None;
/// }
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn init() {
///         // A full mod id: its author, then its mod.
///         match resolve::source("author:arena") {
///             Ok(id) => ARENA.set(Some(id)),
///             Err(e) => log::error!("arena is not in this session: {e}"),
///         }
///         protocol::Opened::observe(on_open);
///     }
/// }
///
/// // The host stamps `from`, and the raiser cannot forge it. One integer
/// // compare decides whether this command is obeyed.
/// async fn on_open(_ctx: Context, from: SourceId, fact: protocol::Opened) {
///     if ARENA.get() != Some(from) {
///         log::warn!("not the arena's door to report: {}", fact.name);
///         return;
///     }
///     log::info!("the arena says {} opened", fact.name);
/// }
/// ```
///
/// # What refuses
///
/// A mod the session did not enable, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName). That is the honest
/// answer rather than a placeholder id, because a mod that is not here raises
/// nothing to compare against. A mod that cannot work without another one says
/// so in its manifest's `[needs]`, and an unsatisfied need refuses the whole
/// session by name rather than leaving this call to refuse once the session is
/// already running.
pub fn source(name: &str) -> Result<SourceId> {
    backend::source(name).map(SourceId)
}
