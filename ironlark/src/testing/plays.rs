//! The playback double: records what was asked to play.

use crate::ids::SoundId;
use crate::shared::audio::SoundBus;
use std::cell::RefCell;

thread_local! {
    static PLAYS: RefCell<Vec<(SoundId, SoundBus, Option<f32>)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record_play(sound: SoundId, bus: SoundBus, volume: Option<f32>) {
    PLAYS.with(|p| p.borrow_mut().push((sound, bus, volume)));
}

/// Drains what the code under test asked to play: sound, bus, volume.
///
/// Every playback awaited since the last drain, in the order the awaits
/// completed, from either realm's door onto the same verb,
/// [`server::audio::play`](crate::server::audio::play) and
/// [`client::audio::play`](crate::client::audio::play) alike. The buffer is
/// emptied, so a second call answers with what has happened since.
///
/// Volume is exactly what the builder was given, and `None` where the author
/// set none. A session substitutes its own default at that point, so a test
/// asserting on full volume asserts on `None` rather than on `Some(1.0)`.
///
/// The example addresses its sound by name with
/// [`resolve::sound`](crate::server::resolve::sound), which is how a mod
/// reaches a name it could not know while it compiled. Native resolution mints
/// an id for any string at all, so the id below is one this thread minted on
/// first use rather than one a session carries.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{block_on, take_plays};
///
/// // `audio`, `resolve`, `SoundBus` and `SoundId` are all that prelude's.
/// async fn alarm(sound: SoundId) {
///     if let Err(e) = audio::play(sound, SoundBus::Effects).volume(0.5).await {
///         log::warn!("the alarm did not sound: {e}");
///     }
/// }
///
/// let Ok(sound) = resolve::sound("alarm") else {
///     return;
/// };
/// block_on(alarm(sound));
///
/// assert_eq!(take_plays(), vec![(sound, SoundBus::Effects, Some(0.5))]);
/// assert!(take_plays().is_empty());
/// ```
///
/// # What playing does here, and what it does in a session
///
/// The double records and answers `Ok`, always. It never refuses, so an `Ok`
/// proves the mod asked and nothing more. A session refuses an id it no longer
/// carries, a name whose owner does not declare it, a volume that is not a
/// finite number, and a mod that has spent its playback allowance. None of
/// those can be provoked on the build machine. Naming another mod's sound in
/// full is not a refusal there, and neither is any [`SoundBus`] case.
///
/// Even in a session `Ok` means the request was accepted rather than heard,
/// because each listening machine then applies its own mute and bus limits. So
/// the honest assertion is the one this drain supports: that the mod decided to
/// play this sound, on this bus, at this volume.
pub fn take_plays() -> Vec<(SoundId, SoundBus, Option<f32>)> {
    PLAYS.with(|p| p.borrow_mut().drain(..).collect())
}
