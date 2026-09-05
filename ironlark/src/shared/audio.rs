//! One-shot sound playback. A sound is addressed by the item
//! [`declares!`](crate::declares) minted from the manifest, or by a
//! [`SoundId`](crate::SoundId) resolved at run time; a lane by the case that
//! names it. The call is a builder, so an option the author leaves alone never
//! costs a list to assemble.

use crate::error::Result;
use crate::protocol::{PlayableSound, SoundSource};
use core::future::{Future, IntoFuture};
use core::pin::Pin;

/// The mixing lane a sound plays on.
///
/// The lane is a claim about what kind of sound this is, and it is the whole of
/// what a mod says about how the sound is mixed. Every listening machine keeps
/// one gain per lane, so a player who turns a lane down turns down every sound
/// on it and nothing on the others. A door slamming is something that happened
/// in the world and belongs on [`Effects`](SoundBus::Effects). A click
/// answering a player's own key press belongs on
/// [`Interface`](SoundBus::Interface), where turning it down silences interface
/// noise and leaves the world audible.
///
/// A lane bounds how many sounds it carries at once on a listening machine, so
/// sounds sharing a lane compete for that ceiling, and one arriving at a full
/// lane is dropped there. A lane turned all the way down drops the sound the
/// same way. Neither decision comes back to the mod that played it.
///
/// It is the second argument of [`play`], and it belongs to no realm. Both
/// preludes carry it, and the same case means the same lane whether the sound
/// was played from [`server::audio`](crate::server::audio), where every
/// participant hears it, or from [`client::audio`](crate::client::audio), where
/// the machine running that half hears it alone. The half decides the reach,
/// the lane decides the mixing, and neither decides the other.
///
/// The set is closed and it is the whole set. Nothing here resolves and nothing
/// here refuses. A lane outside these four has no name to spell, so reaching
/// for one fails to compile instead of refusing once the session is already
/// running.
///
/// # What is not built
///
/// The host keeps a voice lane of its own, which no mod may name. The host
/// writes voice, a mod does not, so voice arrives as its own verb rather than
/// as a fifth case here.
///
/// ```
/// // The prelude mints SoundBus and the audio module. `protocol` is this
/// // crate's own module holding `ironlark::declares!("../mod.toml")`, which
/// // mints `sound::Slam` from `sounds = ["slam"]`.
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
/// // The lane says what the sound IS, so a player turning one down silences a
/// // kind of sound rather than a mod.
/// async fn slam_the_door() {
///     if let Err(e) = audio::play(protocol::sound::Slam, SoundBus::Effects).await {
///         log::warn!("the slam was not heard: {e}");
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundBus {
    /// Something that happened in the world: a door, an impact, a footstep.
    Effects,
    /// The place itself — weather, a room tone, anything continuous rather
    /// than an event.
    Environment,
    /// Composed music, which a player commonly turns down on its own.
    Music,
    /// Feedback on this player's own act: the click that answers their key,
    /// the beep that says their request was refused.
    Interface,
}

/// Plays a sound once, heard at the same loudness wherever a listener stands.
///
/// Called from [`server::audio`](crate::server::audio), every participant
/// hears it. Called from [`client::audio`](crate::client::audio), this
/// machine alone hears it. Nothing fires until the returned [`Play`] is
/// awaited. Options chain onto it first, and an unset volume is full volume.
///
/// `sound` is the item [`declares!`](crate::declares) mints from the
/// manifest — `sound::Slam` under `sounds = ["slam"]`, backed by `slam.wav`
/// beside `mod.toml`. The item resolves its id once, on first play, and reads
/// it afterwards, so a sound on a hot path costs one crossing into the host
/// for the whole session. A [`SoundId`](crate::SoundId) from
/// [`resolve::sound`](crate::server::resolve::sound) is accepted in the same
/// position. That is how a mod plays a name it could not know while it
/// compiled, another mod's sound among them.
///
/// `bus` is the mixing lane, one of the four [`SoundBus`] cases. A player who
/// turns a lane down turns down everything on it.
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
/// // `protocol` is this crate's own module holding
/// // `ironlark::declares!("../mod.toml")`, which mints `sound::Slam` from the
/// // manifest's `sounds = ["slam"]` — slam.wav beside it.
/// async fn slam_the_door() {
///     if let Err(e) = audio::play(protocol::sound::Slam, SoundBus::Effects)
///         .volume(0.8)
///         .await
///     {
///         log::warn!("the slam was not heard: {e}");
///     }
/// }
/// ```
///
/// A name no mod declares, or an id the session no longer carries, refuses as
/// [`UnresolvedName`](crate::ErrorKind::UnresolvedName). The mod's playback
/// allowance spent, or a volume that is not a finite number, refuses as
/// [`Other`](crate::ErrorKind::Other). Another mod's sound is not a refusal,
/// and neither is any lane. Naming a foreign sound in full is how one mod
/// plays what another ships. `Ok` means accepted, NOT heard. Each listening
/// machine applies its own mute and bus limits, and a sound dropped there
/// never reaches the caller.
pub fn play(sound: impl PlayableSound, bus: SoundBus) -> Play {
    Play {
        sound: sound.source(),
        bus,
        volume: None,
    }
}

/// A sound settled but not yet played.
///
/// [`play`] returns one, and the await is what fires it. Nothing crosses into
/// the host before that, so a `Play` never awaited plays nothing. The
/// `must_use` warning catches the accidental case. Options chain on before
/// the await, cost nothing when left alone, and a new option arrives as a new
/// method, so a call that compiles keeps compiling.
///
/// Which half built it decides who hears it.
/// [`server::audio`](crate::server::audio) reaches every participant, and
/// [`client::audio`](crate::client::audio) reaches this machine alone. The
/// host runs the same checks on both. Awaiting answers a
/// [`Result`](crate::Result), and `Ok` means accepted, not heard. The
/// refusals are [`play`]'s subject.
///
/// It is `Copy`, so a settled shot can be held and fired when its moment
/// comes:
///
/// ```
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
/// // `protocol` is this crate's own module holding
/// // `ironlark::declares!("../mod.toml")`, which mints `sound::Slam` from the
/// // manifest's `sounds = ["slam"]` — slam.wav beside it.
/// async fn door_closes(latched: bool) {
///     // Inert until the await: sound and options settled into a value.
///     let slam = audio::play(protocol::sound::Slam, SoundBus::Effects).volume(0.8);
///     if latched {
///         if let Err(e) = slam.await {
///             log::warn!("the slam was not heard: {e}");
///         }
///     }
///     // Not latched: the shot is dropped and nothing reaches the host.
/// }
/// ```
///
/// It is not a handle to a playing sound, because there is no such thing here.
/// A play is one shot. Nothing stops it, moves it or follows an entity with it.
#[must_use = "a playback fires when awaited"]
#[derive(Clone, Copy)]
pub struct Play {
    sound: SoundSource,
    bus: SoundBus,
    volume: Option<f32>,
}

impl Play {
    /// How loud, from 0.0 to 1.0. A finite value outside that range is clamped
    /// on each listening machine rather than refused. A value that is no
    /// number — a NaN, or an infinity — refuses the play at the caller and
    /// says which mod gave it. Unset means full volume.
    ///
    /// It scales this one sound. The bus gain and the player's own mute sit on
    /// top of it on every listening machine, so a quiet sound is a claim about
    /// this sound against its neighbours, never about how loud the player ends
    /// up hearing it.
    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }
}

/// The lane as the contract spells it. The one crossing between the author's
/// vocabulary and the generated one.
#[cfg(target_arch = "wasm32")]
fn wire_bus(bus: SoundBus) -> crate::bindings::server::ironlark::host::audio::Bus {
    use crate::bindings::server::ironlark::host::audio::Bus as Wire;
    match bus {
        SoundBus::Effects => Wire::Effects,
        SoundBus::Environment => Wire::Environment,
        SoundBus::Music => Wire::Music,
        SoundBus::Interface => Wire::Interface,
    }
}

/// The volume, as the contract's named-value list. Absent means the host's
/// own default, which is what an author who set nothing asked for.
#[cfg(target_arch = "wasm32")]
fn wire_params(
    volume: Option<f32>,
) -> Vec<(
    String,
    crate::bindings::server::ironlark::host::types::FieldValue,
)> {
    use crate::bindings::server::ironlark::host::types::FieldValue;
    match volume {
        Some(v) => vec![("volume".to_string(), FieldValue::Number(v))],
        None => Vec::new(),
    }
}

impl IntoFuture for Play {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let sound = match self.sound {
                SoundSource::Ready(id) => id,
                SoundSource::Lazy(resolve) => resolve()?,
            };
            #[cfg(target_arch = "wasm32")]
            {
                crate::bindings::server::ironlark::host::audio::play(
                    sound.0,
                    wire_bus(self.bus),
                    wire_params(self.volume),
                )
                .await
                .map_err(|e| crate::Error::from_wire(e.code, e.message, e.data))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::testing::record_play(sound, self.bus, self.volume);
                Ok(())
            }
        })
    }
}
