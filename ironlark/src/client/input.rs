//! The typed shape an author hook is handed with a key edge.

/// Which side of a key press crossed the boundary.
///
/// It is the second argument of an author hook: a function of the mod's own that
/// the player's input reaches. Both edges arrive, and most handlers want one of
/// them. A mod whose key asks its server half a question returns immediately on
/// [`Released`](InputEdge::Released), so holding the key is one round trip
/// rather than two. The release is the same key event finishing rather than a
/// second instruction.
///
/// Client realm, and client only. Input happens on the machine a person is
/// sitting at. A server half declaring a hook with default bindings is refused
/// when the session starts, and what a server half learns about a press is
/// whatever its client half chooses to ask it with
/// [`request`](crate::client::request).
///
/// The tick an edge belongs to is
/// [`Context::raised_at`](crate::Context::raised_at), the same clock every
/// other event is stamped against. A handler receives this, so there is nothing
/// here to refuse.
///
/// # Where an author hook comes from
///
/// The mod names it, and the name is the function's own. `mod.toml` declares it
/// under `[declares.client]`, and
/// [`#[ironlark::hooks]`](macro@crate::hooks) on the impl block lifts the
/// function out and mints the dispatch that reaches it:
///
/// ```toml
/// [declares.client]
/// hooks = [
///     { name = "use_door", default-bindings = ["key:f5", "pad:north"] },
/// ]
/// ```
///
/// `default-bindings` is the list of physical controls the mod suggests, one per
/// device, and the player rebinds whatever they like. An empty list is legal and
/// means a hook waiting to be bound. The list absent altogether is refused when
/// the session starts, because a hook with no rule is one nothing can ever
/// invoke. Which control fired never reaches the mod: the handler is entered
/// with the edge, and a keycode is the host's business.
///
/// A function in the impl block that the manifest does not declare fails the
/// build, and so does a declaration with no function, which is the drift the
/// attribute exists to end.
///
/// ```
/// use ironlark::client::prelude::*;
///
/// // `ClientMod`, `Context` and `InputEdge` all arrive with the prelude
/// // imported above. `use_door` is not the trait's; it is this mod's own hook,
/// // and the manifest declares it by that name. A real half sits one level
/// // below its manifest and writes `#[ironlark::hooks("../mod.toml")]`; the
/// // path below is this crate's own copy of that file.
/// struct Door;
///
/// #[ironlark::hooks("doctest/mod.toml")]
/// impl ClientMod for Door {
///     async fn use_door(_ctx: Context, edge: InputEdge) {
///         if edge != InputEdge::Pressed {
///             return;
///         }
///         log::debug!("the door key went down");
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEdge {
    /// The key went down.
    Pressed,
    /// The key came up.
    Released,
}
