//! The per-session compact ids: one macro, seven declared-name tables.

use core::fmt;

macro_rules! compact_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        /// It hands out no number, so nothing can fill a schema field with it.
        /// A compact id is one session's own numbering, and it would mean
        /// something else in the next one.
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) u32);

        impl $name {
            /// The id as the session numbered it, for a test or a fixture. A
            /// mod receives these rather than inventing them, and a number
            /// invented here is refused by a session that never minted it.
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            /// The bare number the contract carries, for the one place that
            /// hands it over. Nothing a mod can see calls this.
            #[allow(dead_code)]
            pub(crate) const fn to_wire(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

compact_id!(
    /// The session's number for a declared signal: the announcement one mod
    /// raises and whoever subscribed hears.
    ///
    /// Nothing on this surface hands a mod one, and nothing takes one. A signal
    /// is met as the payload type its own `protocol.proto` declares, and that
    /// type is what [`signal`](crate::server::signal),
    /// [`signal_to`](crate::server::signal_to) and
    /// [`observe`](crate::protocol::SignalSpec::observe) are given. The
    /// declared name behind it becomes this number once, where the handler
    /// registers. The type is published so that the number the contract
    /// carries has a name in Rust, not because a mod is meant to hold one.
    ///
    /// The number is the session's, assigned from the enabled set before any
    /// mod loads and gone when the session ends, and nothing renumbers it while
    /// that session runs. It does not travel. A signal that crosses to the
    /// client realms carries its declared name, and the host on the other
    /// machine resolves that name into its own table. What makes a signal mean
    /// one thing across the session is the declaration; the number is only how
    /// this peer spells it.
    ///
    /// A signal's number belongs to the signal table alone. Numbering starts at
    /// 1 in every kind's own table, so this integer and a [`RequestId`]'s can
    /// be equal and mean unrelated things. The type is what keeps them apart,
    /// because the host, handed a bare number, cannot.
    ///
    /// There is no verb that answers one. A mod names a signal by its payload
    /// type, its own or another mod's, and the type carries the declared name,
    /// so nothing is left for an author to resolve by hand. What is left for
    /// this type is a fixture: a number stated so a test can read what a log
    /// line will say.
    ///
    /// ```
    /// // The crate root's, because a signal belongs to no realm. It is
    /// // Display, which is how a warning about an unobserved signal names one.
    /// use ironlark::SignalId;
    ///
    /// let signal = SignalId::new(7);
    /// assert_eq!(signal.to_string(), "7");
    /// ```
    SignalId
);
compact_id!(
    /// The session's number for a declared request: the named question a client
    /// half asks the half that declared it.
    /// [`request`](crate::client::request) asks it and
    /// [`respond`](crate::protocol::RequestSpec::respond) answers it.
    ///
    /// Like a signal, it is never held by a mod. Both ends are given the
    /// request's own payload type, the registration resolves its declared name
    /// where it is written, and a request resolves it once and remembers it. Spelling a
    /// request's name at a call site is therefore impossible, which is the
    /// point: a name nothing declares has no type to name it with and fails to
    /// compile, rather than becoming a question that goes nowhere.
    ///
    /// A request's number belongs to the request table alone, so this integer
    /// and a [`SignalId`]'s can be equal and mean unrelated things.
    ///
    /// As with a signal, no verb answers one: the payload type carries the
    /// declared name, so there is nothing to resolve by hand. A fixture states
    /// a number when a test wants to read one.
    ///
    /// ```
    /// // The crate root's, because a request belongs to no realm. It is
    /// // Display, which is how a refusal names the request nothing answered.
    /// use ironlark::RequestId;
    ///
    /// let request = RequestId::new(3);
    /// assert_eq!(request.to_string(), "3");
    /// ```
    RequestId
);
compact_id!(
    /// The session's number for a declared hook: one entry point into a mod,
    /// engine-invoked or author-defined.
    ///
    /// One concept covers both. The engine's own hooks have a predefined
    /// invocation — an arrival, a tick, an interaction — and an author's hooks
    /// are declared with the rule that invokes them, of which an input binding
    /// is the first. The id is how the host names the one it is dispatching.
    ///
    /// It arrives; it is never resolved. The declarations mint it, so a hook is
    /// the one declared kind [`resolve`](crate::server::resolve) has no
    /// function for, and a mod cannot ask for a hook's number by name. What
    /// carries it is the dispatch: the host names the hook it is invoking, and
    /// the generated dispatch table enters the author's own function.
    ///
    /// Which key raises an author hook is the host's business and reaches no
    /// mod. A manifest states default bindings, the player rebinds whatever
    /// they like, and the handler is entered with the edge that crossed rather
    /// than with a keycode.
    ///
    /// A hook's number belongs to the hook table alone, so this integer and a
    /// [`SignalId`]'s can be equal and mean unrelated things.
    ///
    /// ```
    /// // The crate root's, because a hook belongs to no realm. It is Display.
    /// use ironlark::HookId;
    ///
    /// // A hook id is a number the host chose, so a fixture states one rather
    /// // than resolving it. Nothing in a session lets a mod mint one.
    /// let hook = HookId::new(3);
    /// assert_eq!(hook.to_string(), "3");
    /// ```
    HookId
);
compact_id!(
    /// The session's number for a declared sound, and the one compact id a mod
    /// does hold.
    ///
    /// Most mods never see one. [`declares!`](crate::declares) mints an item
    /// for every name under the manifest's `sounds`, and
    /// [`audio::play`](crate::server::audio::play) takes that item and resolves
    /// it once behind its own cell, so a mod playing a sound it ships itself
    /// holds nothing. What is left for this type is a name the manifest cannot
    /// state at compile time, one settled at run time or another mod's sound
    /// written in full as `"author:mod/sound/name"`.
    ///
    /// That is what [`server::resolve::sound`](crate::server::resolve::sound)
    /// and [`client::resolve::sound`](crate::client::resolve::sound) mint, and
    /// the id they answer is taken by a play wherever the declared item would
    /// be. Resolution refuses with
    /// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) when nobody
    /// declares the name, and it refuses at that moment rather than leaving a
    /// silent gap where the sound should have been.
    ///
    /// Resolve once and keep the answer in a [`State`](crate::State) cell. The
    /// id is `Copy` and a resolve is a crossing into the host, so one inside a
    /// per-press or per-tick path pays for a lookup nothing about the session
    /// made necessary.
    ///
    /// It is not the sound's data and it is not a playing sound. Nothing here
    /// stops, moves or follows anything. A play is one shot, and the id is only
    /// what says which shot.
    ///
    /// ```
    /// // The prelude mints ServerMod, SoundId, SoundBus, audio and the
    /// // resolve module.
    /// use ironlark::server::prelude::*;
    ///
    /// // Resolved once, kept for the session: the id is Copy and the resolve
    /// // is a crossing into the host.
    /// ironlark::state! {
    ///     static ALARM: Option<SoundId> = None;
    /// }
    ///
    /// async fn sound_the_alarm() {
    ///     // A name settled at run time has no minted item, so it arrives as
    ///     // text and becomes this id here.
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
    SoundId
);
compact_id!(
    /// One row of world data a mod may read and write, resolved by name with
    /// [`resolve::component`](crate::server::resolve::component) and passed to
    /// [`Entity::get_component`](crate::server::Entity::get_component) and
    /// [`Entity::set_component`](crate::server::Entity::set_component) as the
    /// row to touch.
    ///
    /// The names a mod may resolve are a whitelist the host publishes, not the
    /// engine's component list, and anything outside it refuses with
    /// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) saying it is not an
    /// accessible component. Capability grows by publishing another row, never
    /// by a mod reaching further into the world.
    ///
    /// The typed shortcuts on [`Entity`](crate::server::Entity), among them
    /// [`set_label`](crate::server::Entity::set_label) and
    /// [`set_base_color`](crate::server::Entity::set_base_color), resolve these
    /// same rows once and are the easier way to reach a row that already has a
    /// verb. This type is what those shortcuts carry underneath.
    ///
    /// Resolving a component says which row. It says nothing about which
    /// entities the caller may touch. That answer belongs to the ownership rule
    /// and is taken at the write, so a component id in hand is not permission
    /// to use it on a body somebody else spawned.
    ///
    /// ```
    /// // The prelude mints ServerMod, ComponentId and the resolve module.
    /// use ironlark::server::prelude::*;
    ///
    /// ironlark::state! {
    ///     static MATERIAL: Option<ComponentId> = None;
    /// }
    ///
    /// struct Doorman;
    ///
    /// impl ServerMod for Doorman {
    ///     async fn init() {
    ///         // `material` is a row the host publishes. The id is Copy and
    ///         // good for the session.
    ///         match resolve::component("material") {
    ///             Ok(row) => MATERIAL.set(Some(row)),
    ///             Err(e) => log::error!("no material row this session: {e}"),
    ///         }
    ///     }
    /// }
    /// ```
    ComponentId
);
compact_id!(
    /// One dotted path inside one component — `"translation"` within
    /// `transform`, `"base_color"` within `material` — resolved by
    /// [`resolve::field`](crate::server::resolve::field) and carried in a
    /// [`Field`](crate::server::Field) alongside the value to write.
    ///
    /// It belongs to the [`ComponentId`] it was resolved under, and that
    /// pairing is checked on every read and every write. The same path under a
    /// different component is a different id, and using one under a component
    /// it was not resolved against refuses and says so, rather than writing
    /// into the wrong row.
    ///
    /// Resolving does not prove the path exists. The id is only the session's
    /// number for a string, minted on first ask, and whether the world has such
    /// a field is judged when the field is first used. So a misspelt path
    /// resolves happily and then refuses at the first
    /// [`set_component`](crate::server::Entity::set_component). Resolve the
    /// paths a mod uses in `init` and keep them, which is also what keeps a mod
    /// inside the session's ceiling on how many distinct paths it may mint.
    ///
    /// ```
    /// // The prelude mints ServerMod, Context, Player, Target, Vec3, Field,
    /// // Value, ComponentId, FieldId and the resolve module.
    /// use ironlark::server::prelude::*;
    ///
    /// // A path is numbered under one component, so the pair is kept together.
    /// ironlark::state! {
    ///     static EMISSIVE: Option<(ComponentId, FieldId)> = None;
    /// }
    ///
    /// struct Doorman;
    ///
    /// impl ServerMod for Doorman {
    ///     async fn init() {
    ///         let Ok(row) = resolve::component("material") else {
    ///             return;
    ///         };
    ///         if let Ok(path) = resolve::field(row, "emissive") {
    ///             EMISSIVE.set(Some((row, path)));
    ///         }
    ///     }
    ///
    ///     async fn on_interact(
    ///         _ctx: Context,
    ///         _player: Player,
    ///         target: Target,
    ///         _hit_point: Vec3,
    ///         _distance: f32,
    ///     ) {
    ///         let Some((row, path)) = EMISSIVE.get() else {
    ///             return;
    ///         };
    ///         let Some(door) = target.entity else {
    ///             return;
    ///         };
    ///         // The id says which path. The value goes beside it.
    ///         let dim = Field { field: path, value: Value::Number(0.1) };
    ///         if let Err(e) = door.set_component(row, [dim]).await {
    ///             log::warn!("the door did not dim: {e}");
    ///         }
    ///     }
    /// }
    /// ```
    FieldId
);
compact_id!(
    /// An enabled mod, as the host's stamp on where a signal came from.
    ///
    /// It arrives rather than being asked for. An [`observe`] handler
    /// receives it between the context and the payload, and
    /// [`Cause::Mod`](crate::Cause::Mod) carries it for an event another mod
    /// caused. The raiser never supplies it, because the host looks up the
    /// raising mod's own id and stamps it, so it cannot be forged by putting a
    /// different name in the payload.
    ///
    /// What a mod does with it is compare. Resolve the full id of the mod whose
    /// signals it will act on, `"author:mod"`, with
    /// [`resolve::source`](crate::server::resolve::source), keep the answer in
    /// a [`State`](crate::State) cell, and test the stamped id against it. A
    /// policy as strict as "only the gamemode tells me to do this" is then one
    /// integer compare per signal instead of a string match. Resolving a mod
    /// the session did not enable refuses with
    /// [`UnresolvedName`](crate::ErrorKind::UnresolvedName), which is the
    /// honest answer, because a mod that is not here sends nothing to compare
    /// against.
    ///
    /// A mod that takes the stamp, logs it and acts whoever sent it has decided
    /// the act is harmless. The compare is what it adds the day the act is not.
    ///
    /// It names the mod, not the machine, not the participant and not the
    /// event. [`Cause::Player`](crate::Cause::Player) says who among the players
    /// caused something, and [`Context::id`](crate::Context::id) says which
    /// event this is.
    ///
    /// [`observe`]: crate::protocol::SignalSpec::observe
    ///
    /// ```
    /// // The prelude mints ServerMod, Context, SourceId, SignalSpec and the
    /// // resolve module. `protocol` is generated from the mod's own
    /// // protocol.proto; `Opened` is the payload type and its one option line
    /// // is the declaration.
    /// use ironlark::server::prelude::*;
    /// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
    ///
    /// ironlark::state! {
    ///     static ARENA: Option<SourceId> = None;
    /// }
    ///
    /// struct Doorman;
    ///
    /// impl ServerMod for Doorman {
    ///     async fn init() {
    ///         if let Ok(id) = resolve::source("author:arena") {
    ///             ARENA.set(Some(id));
    ///         }
    ///         protocol::Opened::observe(on_open);
    ///     }
    /// }
    ///
    /// // The host stamps `from`; a raiser cannot forge it. Obeying is one
    /// // integer compare.
    /// async fn on_open(_ctx: Context, from: SourceId, fact: protocol::Opened) {
    ///     if ARENA.get() != Some(from) {
    ///         return;
    ///     }
    ///     log::info!("the arena says {} opened", fact.name);
    /// }
    /// ```
    SourceId
);
