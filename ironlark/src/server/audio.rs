//! Sound, from the server half.
//!
//! The server realm's audio. What a server mod plays here, every
//! participant hears. The client realm's own
//! [`audio`](crate::client::audio) reaches this machine alone. The host
//! runs the same checks on both, so the half decides reach and nothing
//! else.
//!
//! One call reaches every connection. Feedback on one
//! player's own act — the click answering their key — belongs on the
//! client half, which crosses no network for it.
//!
//! # Where a sound comes from
//!
//! Every audio name in the example below starts in the mod's own
//! directory:
//!
//! - **The file.** A `.wav` or `.ogg` beside `mod.toml`, named after the
//!   declared name — `slam.wav` for `slam`. Decoded at session start. A
//!   missing or malformed file is refused there, with the reason in the
//!   session log, not at play time.
//! - **The manifest.** `mod.toml` declares the name:
//!   `sounds = ["slam"]` under `[declares]`. The file alone is not
//!   enough.
//! - **The item.** The manifest's name is minted as `sound::Slam` into
//!   the module holding the mod's declarations. The convention is a file
//!   named `protocol.rs`, which is why the example reaches it as
//!   `protocol::sound::Slam`. It resolves its id once, on first play. The
//!   trait behind it is [`SoundSpec`](crate::protocol::SoundSpec).
//!
//! Another mod's sound — a name unknowable at compile time — is resolved
//! at run time with [`resolve::sound`](super::resolve::sound), and the
//! [`SoundId`](crate::SoundId) it returns is accepted wherever a minted
//! item is.
//!
//! The second argument is the mixing lane, one of the four
//! [`SoundBus`](crate::SoundBus) cases:
//! [`Effects`](crate::SoundBus::Effects) for what happened in the world,
//! [`Environment`](crate::SoundBus::Environment) for the place itself,
//! [`Music`](crate::SoundBus::Music), and
//! [`Interface`](crate::SoundBus::Interface) for feedback on the player's
//! own act. It is a case, not a name, so a lane that does not exist is a
//! compile error, not a refusal mid-session. A player who turns a lane
//! down turns down everything on it. Voice is absent deliberately. The
//! host writes voice, a mod does not.
//!
//! # Example
//!
//! A mod playing its declared sound when a player uses its entity. The
//! hook and its arguments are [`ServerMod`](super::ServerMod)'s subject.
//!
//! ```
//! use ironlark::server::prelude::*;
//! # mod protocol { pub mod sound {
//! #     #[derive(Clone, Copy)] pub struct Slam;
//! #     impl ironlark::protocol::SoundSpec for Slam {
//! #         const NAME: &'static str = "slam";
//! #         fn resolved() -> ironlark::Result<ironlark::SoundId> {
//! #             ironlark::state! {
//! #                 static ID: Option<ironlark::SoundId> = None;
//! #             }
//! #             ironlark::protocol::resolve_sound_once(&ID, "slam")
//! #         }
//! #     }
//! #     impl ironlark::protocol::PlayableSound for Slam {
//! #         fn source(self) -> ironlark::protocol::SoundSource {
//! #             ironlark::protocol::SoundSource::Lazy(
//! #                 <Self as ironlark::protocol::SoundSpec>::resolved)
//! #         }
//! #     }
//! # } }
//! // A real crate has one more line here:
//! //
//! //     mod protocol;
//! //
//! // pointing at src/protocol.rs. The manifest beside the crate carries
//! //
//! //     [declares]
//! //     sounds = ["slam"]     # ships slam.wav beside it
//! //
//! // and the hidden lines above are what that mints: `sound::Slam`,
//! // reached below as `protocol::sound::Slam`.
//! struct Door;
//!
//! impl ServerMod for Door {
//!     async fn on_interact(
//!         _ctx: Context,
//!         _player: Player,
//!         _target: Target,
//!         _hit_point: Vec3,
//!         _distance: f32,
//!     ) {
//!         // play(sound, lane), options chained on. The await fires it.
//!         if let Err(e) = audio::play(protocol::sound::Slam, SoundBus::Effects)
//!             .volume(0.8)
//!             .await
//!         {
//!             // The engine declined the sound. The interaction stands.
//!             log::warn!("the slam was not heard: {e}");
//!         }
//!     }
//! }
//! ```
//!
//! # What refuses
//!
//! A name no mod declares, or an id the session no longer carries, is
//! [`UnresolvedName`](crate::ErrorKind::UnresolvedName). The rest read as
//! [`Other`](crate::ErrorKind::Other): the mod's playback allowance
//! spent, or a volume that is not a finite number. Another mod's sound is
//! not a refusal — naming it in full is how one mod plays what another
//! ships — and neither is any lane.
//!
//! `Ok` means accepted, NOT heard. Each listening machine applies its own
//! mute and bus limits, and a sound dropped there never reaches the
//! caller. A refusal is worth logging and rarely worth stopping for. Log
//! it and let the interaction stand.
//!
//! # What is not built
//!
//! A sound plays at one loudness for everyone. Placing it in the world —
//! moving with an entity, fading with distance — arrives as an emitter
//! with its own position and lifetime, not as a second way to call
//! [`play`].

pub use crate::shared::audio::{Play, play};
