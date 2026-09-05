//! Sound, heard by this machine alone.
//!
//! The client realm's audio. A one-shot sound only the player running this
//! half hears, which is what feedback on that player's own act should be:
//! the latch answering their key, the beep saying their own request was
//! refused. The server realm's own [`audio`](crate::server::audio) reaches
//! every participant instead. The host runs the same checks on both, so the
//! half decides reach and nothing else.
//!
//! Nothing crosses the network for a sound played here. The request never
//! leaves this machine, and it puts no noise in anybody else's ears for
//! something only this player did.
//!
//! # Where a sound comes from
//!
//! Every audio name in the example below starts in the mod's own directory:
//!
//! - **The file.** A `.wav` or `.ogg` beside `mod.toml`, named after the
//!   declared name — `slam.wav` for `slam`. Decoded at session start. A
//!   missing or malformed file is refused there, with the reason in the
//!   session log, not at play time.
//! - **The manifest.** `mod.toml` declares the name: `sounds = ["slam"]`
//!   under `[declares]`. The file alone is not enough.
//! - **The item.** The manifest's name is minted as `sound::Slam` into
//!   the module holding the mod's declarations. The convention is a file
//!   named `protocol.rs`, which is why the example reaches it as
//!   `protocol::sound::Slam`. It resolves its id once, on first play. The
//!   trait behind it is [`SoundSpec`](crate::protocol::SoundSpec).
//!
//! Another mod's sound — a name unknowable at compile time — goes through
//! [`resolve::sound`](super::resolve::sound) at run time. The
//! [`SoundId`](crate::SoundId) it returns is accepted wherever a minted item
//! is.
//!
//! The second argument is the mixing lane, one of the four
//! [`SoundBus`](crate::SoundBus) cases:
//! [`Interface`](crate::SoundBus::Interface) for feedback on the player's
//! own act, which is what a client sound usually is,
//! [`Effects`](crate::SoundBus::Effects) for what happened in the world,
//! [`Environment`](crate::SoundBus::Environment) for the place itself, and
//! [`Music`](crate::SoundBus::Music). It is a case, not a name, so a lane
//! that does not exist is a compile error rather than a refusal mid-session.
//! A player who turns a lane down turns down everything on it. Voice is
//! absent deliberately. The host writes voice, a mod does not.
//!
//! # Example
//!
//! A door mod's client half, sounding the latch it answers a key with.
//! The handler is an author hook, and [`InputEdge`](super::InputEdge) is
//! what it is handed.
//!
//! ```
//! use ironlark::client::prelude::*;
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
//! //     sounds = ["slam"]      # ships slam.wav beside it
//! //
//! //     [declares.client]
//! //     hooks = [{ name = "pressed", default-bindings = ["key:f5"] }]
//! //
//! // and the hidden lines above are what the sound name mints:
//! // `sound::Slam`, reached below as `protocol::sound::Slam`.
//! async fn pressed(_ctx: Context, edge: InputEdge) {
//!     if edge != InputEdge::Pressed {
//!         return;
//!     }
//!     // play(sound, lane), options chained on. The await fires it.
//!     if let Err(e) = audio::play(protocol::sound::Slam, SoundBus::Interface)
//!         .volume(0.6)
//!         .await
//!     {
//!         // The engine declined the sound. The press stands.
//!         log::warn!("the latch did not sound: {e}");
//!     }
//! }
//! ```
//!
//! # What refuses
//!
//! An `Err` is the host declining the request, and none of the reasons is
//! [`Refused`](crate::ErrorKind::Refused). A name no enabled mod declares,
//! or an id the session no longer carries, is
//! [`UnresolvedName`](crate::ErrorKind::UnresolvedName). The rest read as
//! [`Other`](crate::ErrorKind::Other), with the host's own sentence in the
//! message: this mod's playback allowance spent, either on the refill rate
//! or on how many of its sounds are already sounding, or a volume that is
//! not a finite number. The lane is never a reason, and a volume outside
//! 0.0 to 1.0 is clamped rather than refused.
//!
//! `Ok` means accepted, NOT heard. This machine then applies its own mute
//! and lane limits, and a sound dropped there never reaches the caller. A
//! refusal is worth logging and rarely worth stopping for.
//!
//! # What is not built
//!
//! A sound plays at one loudness wherever the player stands. Placing it in
//! the world — moving with an entity, fading with distance — arrives as an
//! emitter with its own position and lifetime, not as a second way to call
//! [`play`].

pub use crate::shared::audio::{Play, play};
