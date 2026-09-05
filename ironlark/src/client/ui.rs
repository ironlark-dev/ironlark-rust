//! What this player sees on top of the world.
//!
//! The overlay panel the host paints over the running game, and a mod's one
//! line of it. The host keeps one line per client half, keyed by the mod
//! that wrote it, so a mod writes its own line and can neither read nor
//! erase anybody else's.
//!
//! [`set_overlay_text`] is the settled surface. The host owns no widget
//! library and grows no interface for a mod to fill, because presentation
//! belongs to mods. A richer surface arrives as scenes a mod owns rather
//! than as host chrome a mod fills in.
//!
//! ```
//! // The prelude mints ClientMod, Context, InputEdge and the ui module.
//! use ironlark::client::prelude::*;
//!
//! // One line, this mod's own. Writing it again replaces it, and writing
//! // an empty string clears it.
//! async fn pressed(_ctx: Context, edge: InputEdge) {
//!     match edge {
//!         InputEdge::Pressed => ui::set_overlay_text("the door is opening"),
//!         InputEdge::Released => ui::set_overlay_text(""),
//!     }
//! }
//! ```

/// Writes this mod's line of the on-screen overlay, on this machine.
///
/// The host keeps one line per client half, keyed by the mod that wrote it, and
/// paints them in one panel, joined in mod-id order. A write replaces only the
/// caller's line, so two mods that have never heard of each other cannot erase
/// one another, and where a line sits does not depend on who wrote first.
/// Passing an empty string drops this mod's line out of the panel altogether,
/// and so does the mod's client half going away.
///
/// A mod owns its line and rewrites it whole. Nothing appends to a line and
/// nothing edits part of one, so a mod showing a value that changes restates
/// the whole sentence each time the value does.
///
/// Text past 128 characters is cut, counted in characters rather than bytes so
/// the cut cannot split a code point, and the caller is not told. There is
/// nothing else: no styling, no layout, no placement a mod chooses, no second
/// line and no region. The panel belongs to the host.
///
/// Client realm, because an overlay is one machine's screen. A server mod that
/// wants every player told something raises with
/// [`server::signal`](crate::server::signal) and lets each client half write its
/// own line from what arrives.
///
/// It returns nothing and cannot refuse. The text is handed to the host's own
/// frame loop, which paints it, and there is nothing to wait for. Built for the
/// machine you develop on, the body is empty and records nothing, so a test can
/// neither read back what a half wrote nor tell a write from silence. Assert on
/// the value the half computed instead, before it is handed over.
///
/// The example is the client half of a door mod. Everything it names —
/// [`ClientMod`](crate::client::ClientMod) and the `ui` module this verb lives
/// in — arrives with the client prelude, and nothing is declared in `mod.toml`
/// for an overlay line.
///
/// ```
/// // ClientMod and the `ui` module both arrive with this one import.
/// use ironlark::client::prelude::*;
///
/// struct Door;
///
/// impl ClientMod for Door {
///     async fn init() {
///         // This mod's whole line. No other mod's line is touched.
///         show(false);
///     }
/// }
///
/// fn show(open: bool) {
///     ui::set_overlay_text(if open { "door: open" } else { "door: shut" });
/// }
/// ```
pub fn set_overlay_text(text: &str) {
    backend::set_overlay_text(text);
}

#[cfg(target_arch = "wasm32")]
mod backend {
    pub fn set_overlay_text(text: &str) {
        crate::bindings::client::ironlark::host::ui::set_overlay_text(text);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    pub fn set_overlay_text(_text: &str) {}
}
