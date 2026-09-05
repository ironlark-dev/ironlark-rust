//! A sound's declaration, and the two ways a play is handed one.

use crate::error::Result;
use crate::ids::SoundId;

/// A declared sound: one name, and the session id it answers to.
///
/// A sound is a file, not traffic, so it declares nothing in a mod's
/// `protocol.proto` and carries no payload type. It is named in `mod.toml`
/// under `[declares]`, as `sounds = ["slam"]`, and one of these is minted per
/// name into a `sound` module beside the mod's own declarations. The
/// convention is a file named `protocol.rs`, which is why a real crate reaches
/// the item as `protocol::sound::Slam`.
///
/// The file behind the name sits beside `mod.toml`, `slam.wav` for `slam`, as
/// a `.wav` or an `.ogg`. Both realms play it:
/// [`server::audio::play`](crate::server::audio::play) reaches every
/// participant, [`client::audio::play`](crate::client::audio::play) this
/// machine alone, and both take the minted item directly.
///
/// ```
/// use ironlark::server::prelude::*;
/// # use ironlark::protocol::SoundSpec;
/// # mod sound {
/// #     #[derive(Clone, Copy)] pub struct Slam;
/// # }
/// # impl SoundSpec for sound::Slam {
/// #     const NAME: &'static str = "slam";
/// #     fn resolved() -> Result<SoundId> {
/// #         ironlark::state! { static ID: Option<SoundId> = None; }
/// #         ironlark::protocol::resolve_sound_once(&ID, "slam")
/// #     }
/// # }
/// # impl ironlark::protocol::PlayableSound for sound::Slam {
/// #     fn source(self) -> ironlark::protocol::SoundSource {
/// #         ironlark::protocol::SoundSource::Lazy(<Self as SoundSpec>::resolved)
/// #     }
/// # }
/// // The hidden lines above are what `sounds = ["slam"]` mints: the item, its
/// // name, and the cell its id resolves into once. `audio`, `SoundBus`,
/// // `SoundId` and `Result` are the server prelude's.
/// async fn slam_the_door() {
///     if let Err(e) = audio::play(sound::Slam, SoundBus::Effects).volume(0.8).await {
///         log::warn!("the slam was not heard: {e}");
///     }
/// }
/// ```
///
/// # What refuses
///
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName) at the first play, when
/// the manifest that shipped does not declare [`NAME`](Self::NAME). The
/// compile-time check reads the manifest in the source tree; the session checks
/// the one that shipped with the mod.
pub trait SoundSpec: Copy + 'static {
    /// Its declared name.
    const NAME: &'static str;
    /// The session's id for it, resolved on the first play and kept.
    ///
    /// A resolve is a crossing into the host, so it happens once per name per
    /// session rather than once per play.
    fn resolved() -> Result<SoundId>;
}

/// What a play takes as its sound, in either realm.
///
/// There is one verb for playing a sound and two ways a mod can be holding
/// one. A sound of its own arrives as the item minted from its manifest,
/// `sound::Slam` under `sounds = ["slam"]`. That item carries its own name, so
/// the compiler checks it and the resolve happens once, on the first play.
/// Another mod's sound cannot arrive that way: this mod's manifest does not
/// list what another mod ships, so there is no item to mint, and the name is
/// settled at run time by [`resolve::sound`](crate::server::resolve::sound) or
/// its client twin [`resolve::sound`](crate::client::resolve::sound), which
/// answer a [`SoundId`].
///
/// This trait is what lets both reach one play rather than splitting it into
/// two verbs differing only in how the sound was named. Both implementations
/// exist already — the minted item's is generated, [`SoundId`]'s is written
/// here — and an author writes neither.
///
/// It sits in this realm-neutral module because a sound belongs to no realm. A
/// server half's play reaches every participant and a client half's reaches one
/// machine, while what may be played, and how it is named, is the same on both.
///
/// ```
/// // The prelude mints SoundBus, SoundId, audio and the resolve module.
/// // `protocol` is this crate's own module, where the manifest's
/// // `sounds = ["slam"]` mints `sound::Slam`.
/// use ironlark::server::prelude::*;
/// # mod protocol { pub mod sound {
/// #     #[derive(Clone, Copy)] pub struct Slam;
/// #     impl ironlark::protocol::SoundSpec for Slam {
/// #         const NAME: &'static str = "slam";
/// #         fn resolved() -> ironlark::Result<ironlark::SoundId> {
/// #             ironlark::state! {
/// #                 static ID: Option<ironlark::SoundId> = None;
/// #             }
/// #             ironlark::protocol::resolve_sound_once(&ID, "slam")
/// #         }
/// #     }
/// #     impl ironlark::protocol::PlayableSound for Slam {
/// #         fn source(self) -> ironlark::protocol::SoundSource {
/// #             ironlark::protocol::SoundSource::Lazy(
/// #                 <Self as ironlark::protocol::SoundSpec>::resolved)
/// #         }
/// #     }
/// # } }
///
/// // One verb, both ways of naming a sound: the minted item, and an id a
/// // resolve answered for a name this manifest cannot know.
/// async fn slam_and_alarm() {
///     let _ = audio::play(protocol::sound::Slam, SoundBus::Effects).await;
///     if let Ok(id) = resolve::sound("author:mod/sound/alarm") {
///         let _ = audio::play(id, SoundBus::Effects).await;
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// The trait only says which of the two forms the caller holds. A name no mod
/// declares refuses at the resolve or at the first play, with
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName).
pub trait PlayableSound: Copy {
    /// Which of the two it is. Generated, not written by hand.
    #[doc(hidden)]
    fn source(self) -> SoundSource;
}

/// How a [`PlayableSound`] reaches its id. Not a surface an author names.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum SoundSource {
    /// Already resolved.
    Ready(SoundId),
    /// Resolved on first use, behind the spec's own cell.
    Lazy(fn() -> Result<SoundId>),
}

impl PlayableSound for SoundId {
    fn source(self) -> SoundSource {
        SoundSource::Ready(self)
    }
}

/// The body of a generated [`SoundSpec::resolved`]: the cell, then the host.
///
/// The cell is a [`State`](crate::State), which is where this crate already
/// carries the single-threaded-guest reasoning, so a sound needs no second one.
#[doc(hidden)]
pub fn resolve_sound_once(
    cell: &crate::State<Option<SoundId>>,
    name: &'static str,
) -> Result<SoundId> {
    if let Some(id) = cell.get() {
        return Ok(id);
    }
    let id = crate::shared::resolve::sound(name)?;
    cell.set(Some(id));
    Ok(id)
}

/// What a generated hook dispatch does with an id it has no function for: warn
/// once, because a hook the host dispatches and this half does not carry is a
/// load-time disagreement rather than something to report per press.
#[doc(hidden)]
pub fn unrouted_hook(hook: u32) {
    crate::state! {
        static WARNED: bool = false;
    }
    if !WARNED.update(|warned| core::mem::replace(warned, true)) {
        log::warn!("no author hook answers hook id {hook}");
    }
}
