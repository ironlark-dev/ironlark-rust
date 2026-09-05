//! Procedural macros for the `ironlark` crate: the `hooks` attribute,
//! `protocol!` and `declares!`.
//!
//! All three are re-exported from `ironlark`, and that is where a mod reaches
//! them: `#[ironlark::hooks]`, `ironlark::protocol!` and
//! `ironlark::declares!`. What they generate names `ironlark` paths, so this
//! crate is never a mod's dependency on its own.

mod declares;
mod hooks;
mod manifest;
mod protocol;

use proc_macro::TokenStream;

/// Reads a mod's `protocol.proto` while the crate compiles: the types that
/// travel, and the declarations that say where each one goes.
///
/// A mod declares its traffic on the payload's own schema. A message carrying
/// `option (ironlark.signal)` declares a signal; an `rpc` inside
/// `service Server` declares a request. This macro compiles that file and
/// generates the Rust type for every message in it, carrying the declaration.
/// A name is written once, in the schema, and reaches every call site as a type
/// the compiler resolves.
///
/// No `protoc` is involved and nothing in the file is executed: the schema is
/// compiled in memory, so a mod's build needs no C++ toolchain.
///
/// # The schema, and what comes out of it
///
/// A door mod announces its latch, keeps a position signal whose newer raise
/// supersedes the older, and answers one request.
///
/// ```proto
/// syntax = "proto3";
/// package mods.ironlark_door;
/// import "ironlark/options.proto";
///
/// // The latch moved.
/// message Latch {
///   option (ironlark.signal) = CLIENTS;
///   bool open = 1;
/// }
/// // Where the doors stand. A newer raise supersedes an older.
/// message Positions {
///   option (ironlark.signal) = CLIENTS;
///   option (ironlark.transit.keep) = NEWEST;
///   repeated Position list = 1;
/// }
/// // No option, so no declaration: a helper carried by another payload.
/// message Position { string name = 1; float x = 2; float z = 3; }
///
/// service Server { rpc Open(OpenRequest) returns (Latch); }
/// message OpenRequest { uint32 force = 1; }
/// ```
///
/// One line in a file both halves include reads it:
///
/// ```ignore
/// ironlark::protocol!("../protocol.proto");
/// ```
///
/// Out come the payload types — `Latch`, `Positions`, `Position`,
/// `OpenRequest` — carrying the comment above each message as their own
/// documentation, and the declaration written onto the ones that declare. The
/// verbs take the payload itself, `signal(&Latch { open: true })`, so what a
/// payload declares is readable from the payload and nowhere else. A message
/// declaring twice is one trait implemented twice for one type, refused in the
/// compiler's own words.
///
/// A signal's declaration carries the declared name, who hears it and how the
/// host retains it under pressure; the audience is a type, not a value, which
/// is what lets the narrowed raise take a crossing signal and refuse a bus-only
/// one before anything runs. A request's carries its name and its answer type.
///
/// Beside the types stands a module per kind, `signal` and `request`, indexing
/// what the mod declares rather than standing in for it:
/// `protocol::signal::Latch` and `protocol::Latch` are one type. A declared
/// name is the message or `rpc` name mapped to the charset a manifest and a
/// qualified id use, `PlayerPositions` reaching `player-positions`.
///
/// # Borrowing another mod's signal
///
/// Declaring a name owns it; raising it is open to any mod in its realm. A mod
/// borrows a name by importing the owner's schema, proto's own `import`, naming
/// it by where that mod is installed:
///
/// ```proto
/// import "ironlark/buttons/protocol.proto";
/// ```
/// That mints the owner's types under the owner's own schema module and lists
/// its signals one level deeper, as `protocol::ironlark_buttons::Pressed` and
/// `protocol::signal::ironlark_buttons::Pressed`, so a borrowed and an own
/// `pressed` never rename each other.
///
/// A borrowed declaration carries the qualified name the host resolves it by,
/// `ironlark:buttons/signal/pressed`, read back from the owner's package, and
/// its audience and retention are the owner's. Its types are generated into
/// this crate rather than depended on, replacing the hand-copied struct and its
/// silent drift.
///
/// A request cannot be borrowed: that is not ruled, so an imported `service` is
/// listed nowhere and there is no name to reach one under.
///
/// The path this line names is resolved against the directory holding the
/// `Cargo.toml` of the crate being built, which is what lets one schema serve
/// both halves. An `import` inside the schema resolves against the workshop a
/// mod is installed in instead, so it names another mod's schema by that mod's
/// install path. The text is included as a string as well as compiled, so
/// editing it rebuilds the crate, and one module holds one `protocol!`.
///
/// # What refuses
///
/// Every refusal is a compile error on the path the line names, which is the
/// one span a fault inside a `.proto` has in Rust source.
///
/// A schema that does not compile refuses with protobuf's own message, and an
/// `import` naming no installed mod's schema is one such refusal. A `package`
/// that is not `mods.<author>_<mod>` refuses, flat under the prefix because a
/// nested one shadows the platform package. A `service` named anything but
/// `Server` or `Client` refuses, its name being what says which half answers; a
/// request in `service Client` refuses too, that direction not being served.
///
/// A message or an `rpc` whose name reaches no declared name refuses, naming
/// the rule. Two declarations reaching one declared name refuse, the host
/// publishing a name once. An option value this SDK's schema does not hold
/// refuses, naming the value and its enum, which is what a mod built against a
/// newer schema gets instead of a dropped declaration.
///
/// An imported schema whose package holds more than one `_` refuses: the `-` of
/// a manifest name becomes `_` there, leaving the owner's `author:mod` id with
/// no one reading back, and a borrowed name is worthless without it.
///
/// A message carrying no option refuses nothing: it is a plain type.
#[proc_macro]
pub fn protocol(input: TokenStream) -> TokenStream {
    protocol::expand(input.into()).into()
}

/// Mints one item per sound and per archetype a mod's manifest declares.
///
/// Those two are the declarations the manifest holds everything about. A sound
/// is a file and an archetype is a row of manifest fields, so neither carries a
/// payload type and nothing about either has to be written in Rust: the macro
/// reads `mod.toml` while the crate compiles and mints one zero-sized item per
/// declared name, into a module named for its kind. Only this macro mints such
/// an item, which is what makes an item's existence proof that the manifest
/// declares it.
///
/// # The two files, side by side
///
/// A door mod ships one sound, as `slam.wav` beside `mod.toml`, and publishes
/// one archetype:
///
/// ```toml
/// [mod]
/// version = "0.1.0"
///
/// [declares]
/// sounds = ["slam"]
///
/// [[declares.archetype]]
/// id = "door"
/// scene = "door.glb"
/// ```
///
/// One line in a file both halves include reads it:
///
/// ```ignore
/// ironlark::declares!("../mod.toml");
/// ```
///
/// That mints `sound::Slam` and `archetype::Door`, reached in a real crate as
/// `protocol::sound::Slam` and `protocol::archetype::Door`. Both realms'
/// `audio::play` take the sound item directly, and
/// `ironlark::server::Entity::spawn` takes the archetype item directly.
///
/// The two items differ in what they own. A sound item owns its id: the first
/// play resolves the name once and every later play reads the cell it filled,
/// so a sound in a per-press path costs one crossing into the host for the
/// session rather than one per press. An archetype has no id to own — the host
/// spawns by name — so the item is the declared name, handed over through
/// `AsRef<str>` and costing nothing at all.
///
/// The path is resolved against the directory holding the `Cargo.toml` of the
/// crate being built, exactly as [`protocol!`](macro@protocol) resolves its
/// own, and the manifest's text is included as a string, so editing it rebuilds
/// the crate.
///
/// A declared name reaches Rust with every dash-separated part capitalised, so
/// `front-door` is reached as `sound::FrontDoor`, and a name opening with a
/// digit takes the leading underscore prost's own mapping gives it, `3-way`
/// reaching `_3Way`. One rule covers a manifest name and a schema name alike,
/// and a `sound` and an `archetype` of one name are two items in two modules.
///
/// # The names no item can stand for
///
/// A name this manifest does not carry cannot be minted from it, and each kind
/// keeps its string door for that case. Another mod's sound, and one settled at
/// run time, go through `ironlark::server::resolve::sound`, which answers an id
/// `audio::play` takes. Another mod's archetype is spelled in full,
/// `author:mod/archetype/door`, and handed to `Entity::spawn` as the string it
/// is. Both verbs take either form, so the string door is the other half of the
/// surface rather than an older one, and writing this mod's own name as a
/// string stays legal too.
///
/// # What refuses
///
/// A manifest that is missing or is not TOML is a compile error on the path.
/// Two declared names of one kind reaching one Rust name is another, and the
/// message spells both out so it is fixed in the manifest rather than read out
/// of generated code. A name no Rust name reaches at all is a third.
///
/// Writing anything after the path is a fourth, and it is about the grammar
/// rather than the files: this macro takes the path alone, because a name
/// carrying a payload type is declared on the mod's schema and reaches Rust
/// through [`protocol!`](macro@protocol).
///
/// A manifest still writing `channels`, `methods` or `actions` under
/// `[declares]` is a fifth, named key by key, each one saying where its
/// declarations went. Nothing reads those keys, so a manifest holding one is a
/// mod written against the previous grammar rather than a mod with a harmless
/// extra line.
///
/// One refusal is at run time instead: the compile-time check reads the
/// manifest in the source tree, and a manifest that shipped without the name it
/// saw refuses at the first use — an unresolved name at the play, an unknown
/// archetype at the spawn.
#[proc_macro]
pub fn declares(input: TokenStream) -> TokenStream {
    declares::expand(input.into()).into()
}

/// Binds a half's entry points to what its manifest declares, and mints the
/// dispatch the mod's own hooks arrive through.
///
/// A hook is a mod entry point. An engine hook has a predefined invocation, an
/// event of the engine's choosing at a time of its choosing; a hook of the
/// mod's own is declared with the rule that invokes it. Both kinds are written
/// in one impl block, and this attribute is what makes that legal: the realm
/// traits are closed, so a function the mod named rather than the contract is
/// lifted into an inherent impl of the same type, where `Self` still means what
/// the author wrote it to mean.
///
/// The manifest is the other half of the job. Stable Rust cannot be asked which
/// trait defaults a type overrode, and the host has to know before it runs the
/// guest: the mailbox opens ahead of instantiation, and a packer, an operator
/// and the launcher all read a mod without starting one. So the file carries
/// the answer, and the list in it and the functions in the block are checked
/// against each other, both ways, on every build.
///
/// # The two files, side by side
///
/// A door mod's server half answers the step; its client half answers a key the
/// player presses. One attribute goes above each block, naming the manifest
/// they share:
///
/// ```ignore
/// #[ironlark::hooks("../mod.toml")]
/// impl ServerMod for Door {
///     async fn init() { .. }
///     async fn on_tick(ctx: Context, dt: f32) { .. }
/// }
///
/// #[ironlark::hooks("../mod.toml")]
/// impl ClientMod for Door {
///     async fn init() { .. }
///
///     // The mod's own hook, and no hook of `ClientMod`: lifted out.
///     async fn use_door(ctx: Context, edge: InputEdge) { .. }
/// }
/// ```
///
/// The manifest gains a section per half:
///
/// ```toml
/// [declares.server]
/// hooks = ["on_tick"]
///
/// [declares.client]
/// hooks = [
///     { name = "use_door", default-bindings = ["key:f5", "pad:north"] },
/// ]
/// ```
///
/// An engine hook is its bare name; one of the mod's own is a table naming
/// itself and its rule, the short and long forms under one key being the Cargo
/// dependency idiom. `default-bindings` is the input rule, an entry per device
/// class spelled `device:control`, and they are defaults: the host owns the
/// binding table, the player rebinds, the host resolves conflicts.
///
/// The path is resolved as [`protocol!`](macro@protocol) resolves its own, and
/// the file's text is included, so editing the list re-runs the check.
///
/// # A section per half, and `init` in neither
///
/// Each half gets its own section: an impl of `ServerMod` is checked against
/// `[declares.server]` and nothing else, an impl of `ClientMod` against
/// `[declares.client]`. That is what lets one half say it answers the step
/// while the other says it does not. A half's section existing is the statement
/// that the half and its `init` exist, so `init` is never written in a hook
/// list, and a section carrying no `hooks` key is a half that answers to `init`
/// alone.
///
/// A section may name only the engine hooks its own trait spells: `on_join`,
/// `on_leave`, `on_tick`, `on_interact` and `on_contact` under
/// `[declares.server]`, and `on_tick` under `[declares.client]`. The host
/// enters a half for more than that, but a signal, a request and an input are
/// none of them a hook of the trait: the first two arrive under a declared name
/// and are answered on the payload type that carries it, and an input arrives at
/// one of the mod's own hooks.
///
/// # How one of the mod's own hooks is reached
///
/// An input edge arrives carrying the number the declarations minted for the
/// hook, never its name, and no verb resolves a hook name. The number is the
/// hook's position among its half's own hooks, in the order the manifest writes
/// them, and the macro mints the match that turns it back into a call. Nothing
/// is allocated and nothing is searched. The input rule reaches the client half
/// alone, because that is the half an input edge is delivered to.
///
/// # What refuses
///
/// Every refusal is a compile error, and the block is emitted anyway so the
/// refusal is the only thing a reader has to read.
///
/// The attribute refuses on anything but an impl of `ServerMod` or `ClientMod`,
/// and refuses a missing or non-string path. A manifest with no section for the
/// half being implemented refuses, because the section is what states the half
/// exists.
///
/// The check on engine hooks runs both ways. A hook the block implements that
/// its section does not name is reported on the function, saying which section
/// of which file to add it to; a hook the section names that the block does not
/// implement is reported on the manifest path. A section naming something that
/// is no hook of its trait is reported with that trait's hooks listed.
///
/// A function that is neither a hook of the trait nor one the section declares
/// is reported on the function with both sets named, which tells an author
/// whether the name is misspelled or the declaration is missing. One of the
/// mod's own hooks that is declared and not implemented is reported on the
/// manifest path.
///
/// `init` named in a hook list refuses, and so does a hook named twice. A table
/// entry with no `name`, with no rule, or with a field that is neither refuses,
/// naming the field. A hook whose rule its half does not serve refuses, which
/// today is an input declared under `[declares.server]`.
///
/// A key other than `hooks` inside either section refuses: TOML puts every key
/// after a section header inside it, so a `sounds` line drifting below
/// `[declares.server]` leaves `[declares]` entirely with nothing malformed
/// about the file, and the message says to move it back above the sections.
#[proc_macro_attribute]
pub fn hooks(attr: TokenStream, item: TokenStream) -> TokenStream {
    hooks::expand(attr.into(), item.into()).into()
}
