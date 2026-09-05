//! The two error positions. [`Error`] is what a host verb yields, [`Refusal`]
//! is what an author returns from a request handler. The wire carries an
//! opaque `code + message + data`, and the taxonomy lives here and grows on
//! the SDK's own semver.

use core::fmt;
use core::ops::RangeInclusive;

/// The crate's result, with [`Error`] in the error position.
///
/// Every verb that crosses into the host answers one. A signature that names
/// its own error overrides the default, which one signature in this crate
/// does. A request handler registered with
/// [`respond`](crate::protocol::RequestSpec::respond) returns
/// `Result<Resp, Refusal>`, because its no is an answer somebody asked for.
/// See [`Refusal`].
///
/// ```
/// // The prelude mints Result, Entity, SpawnPoint and Vec3.
/// use ironlark::server::prelude::*;
/// # mod protocol { ironlark::declares!("doctest/mod.toml"); }
///
/// // The second parameter defaults to Error, so a verb's result is written
/// // with one type. A helper of the author's own may still name its own.
/// // `archetype::Door` is what this mod's own `[[declares.archetype]]` mints.
/// async fn hang_a_door() -> Result<Entity> {
///     let at = SpawnPoint { position: Vec3::new(0.0, 0.0, 4.0), yaw: 0.0 };
///     Entity::spawn(protocol::archetype::Door, at).await
/// }
/// ```
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// The codes this SDK reads. The first four are the host's own numbers and
/// arrive over the wire. `STALE_ID` never does, and is minted here in the
/// guest by the liveness check on a lent handle. A code this list does not
/// name is a host domain code, surfaced as [`ErrorKind::Other`].
pub(crate) mod code {
    pub const REFUSAL: u32 = 1;
    pub const TOO_LARGE: u32 = 2;
    pub const UNRESOLVED_NAME: u32 = 3;
    pub const OVERLOADED: u32 = 4;
    pub const STALE_ID: u32 = 5;
}

/// A verb that did not happen.
///
/// Everything a mod asks the world to do can be turned down, so those verbs
/// answer [`Result`] and this is the error position. It carries a triple
/// through untouched. A numeric [`code`](Error::code), a
/// [`message`](Error::message) in the words of whoever refused, and a
/// [`data`](Error::data) blob. Branch on [`kind`](Error::kind), which is the
/// reading of the code. Put the message in the log line, because it is the one
/// part that says which of a kind's several causes happened.
///
/// It belongs to no realm. Both preludes carry it, and a verb under
/// [`server`](crate::server) and one under [`client`](crate::client) answer
/// the same type with the same codes in it.
///
/// # There is no `?` in mod code
///
/// A hook returns nothing. There is nowhere for `?` to throw to, and the crate
/// never asks an author to define what a failed hook means, because the answer
/// is different every time. A colour that would not apply is a shrug. A body
/// that never arrived may end the round. So a mod matches the result where the
/// verb was called, says what happened, and continues or stops.
///
/// # Example
///
/// A mod painting each arriving participant's body.
/// [`ServerMod`](crate::server::ServerMod) is the trait a server half
/// implements and [`on_join`](crate::server::ServerMod::on_join) is its
/// arrival hook. [`Player`](crate::server::Player) is the handle the host
/// lends for that event, [`body`](crate::server::Player::body) asks for the
/// character body that participant controls, and
/// [`set_base_color`](crate::server::Entity::set_base_color) paints every mesh
/// under it. [`Rgba`](crate::Rgba) is the colour and [`ErrorKind`] is what a
/// refusal turns out to be. The last line is
/// [`export_server!`](macro@crate::export_server), which is what makes the
/// type the half the host calls.
///
/// ```
/// // The prelude mints ServerMod, Context, Player, Rgba and export_server!.
/// use ironlark::server::prelude::*;
/// // The kinds are not in the prelude. A mod that branches names them.
/// use ironlark::ErrorKind;
/// // `log` is the standard facade, added as `log = "0.4"` in the mod's own
/// // Cargo.toml. This crate routes it to the host's session log.
///
/// // Four components, 0.0 to 1.0. `const`, because the colour never varies.
/// const RED: Rgba = Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
///
/// struct Painter;
///
/// impl ServerMod for Painter {
///     // Returns nothing, so every Result is matched right here.
///     async fn on_join(_ctx: Context, player: Player) {
///         let body = match player.body().await {
///             Ok(body) => body,
///             Err(e) => {
///                 log::warn!("{player} arrived with nothing to paint: {e}");
///                 return;
///             }
///         };
///         if let Err(e) = body.set_base_color(RED).await {
///             match e.kind() {
///                 ErrorKind::UnresolvedName => {
///                     log::error!("this session carries no material row: {e}");
///                 }
///                 // Any other kind: the colour is not worth stopping for.
///                 _ => log::warn!("{player} keeps the colour they had: {e}"),
///             }
///         }
///     }
/// }
///
/// ironlark::export_server!(Painter);
/// ```
///
/// The host writes its own line for every refusal it issues, so a mod that
/// drops one is not deleting the only record of it. Dropping one silently is
/// still worse than logging it, because the host's line names the mod and not
/// the place in it.
///
/// # Where one comes from
///
/// Almost always the host, across the component boundary. A few are minted in
/// the guest before any call is made — a payload that will not encode, a handle
/// whose event is over — and those carry a code the host does not use, so
/// [`kind`](Error::kind) still separates them from anything the world said.
///
/// # What is not built
///
/// [`data`](Error::data) is always empty. No refusal the host builds attaches
/// bytes and neither does any the SDK mints, so the field is the contract's
/// room for a structured payload and nothing fills it. Read the message.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    code: u32,
    message: String,
    data: Vec<u8>,
}

/// What a refusal was about.
///
/// This is the reading of [`Error::code`], and the thing to match on. The code
/// itself is the host's number and belongs in a log line rather than in a
/// condition. A code this SDK does not name arrives as
/// [`Other`](ErrorKind::Other) with the number intact, so an unknown refusal
/// is still a refusal a mod can handle rather than a panic or a lost error.
///
/// `#[non_exhaustive]`, so a match needs a `_` arm and a kind added in a later
/// minor does not break a mod that compiled against this one.
///
/// Each variant below states what it means and where it is issued.
///
/// # Which codes the host sends
///
/// The codes that read as [`Refused`](ErrorKind::Refused),
/// [`TooLarge`](ErrorKind::TooLarge),
/// [`UnresolvedName`](ErrorKind::UnresolvedName) and
/// [`Overloaded`](ErrorKind::Overloaded), and its own failure code, which
/// reads as [`Other`](ErrorKind::Other). Anything outside that set is a host
/// domain code and reads as [`Other`](ErrorKind::Other) too, with the number
/// left intact.
///
/// [`StaleId`](ErrorKind::StaleId) is not among them and never crosses the
/// wire. In a session every one is minted in the guest, by the SDK's own
/// liveness check on a lent handle, before the call leaves.
///
/// # What no kind covers
///
/// A declaration the host does not serve yet — an ordering the transport
/// cannot give, a retention rule that has no implementation — is refused at
/// load, by name, with the mod left out of the session. It is never degraded
/// silently and it is never an [`Error`], because there is no running half to
/// hand one to. What a mod meets instead is that it did not load, and the
/// session log says which declaration did it.
///
/// # What is not built
///
/// On the build machine the liveness check has nothing to check. The doubles
/// carry no event scope, so a handle kept past its handler answers there and
/// refuses only in a session.
///
/// One situation answers differently on the two build targets, and which
/// answer is the right one is not settled. Asking a
/// [`Context`](crate::Context) about an event that is not in flight refuses in
/// a session with [`Refused`](ErrorKind::Refused) and on the build machine
/// with [`StaleId`](ErrorKind::StaleId). Neither is the contract's answer. The
/// two disagree, and the disagreement is a defect rather than a design. A test
/// that asserts a kind for that case pins one target and fails on the other,
/// so assert that it refused and read the message.
///
/// ```
/// // The prelude mints ServerMod, Context, Player, Target, Vec3 and Rgba.
/// // ErrorKind is not in it, so it is imported by name.
/// use ironlark::ErrorKind;
/// use ironlark::server::prelude::*;
///
/// struct Doorman;
///
/// impl ServerMod for Doorman {
///     async fn on_interact(
///         _ctx: Context,
///         _player: Player,
///         target: Target,
///         _hit_point: Vec3,
///         _distance: f32,
///     ) {
///         let Some(door) = target.entity else {
///             return;
///         };
///         let red = Rgba { r: 1.0, g: 0.2, b: 0.2, a: 1.0 };
///         if let Err(e) = door.set_base_color(red).await {
///             // Branch on the kind, and read the message for the detail.
///             match e.kind() {
///                 ErrorKind::StaleId => log::info!("the door is gone already"),
///                 ErrorKind::Refused => log::warn!("not this mod's door: {e}"),
///                 // The set may grow, so a match on it carries this arm.
///                 _ => log::error!("the door did not redden: {e}"),
///             }
///         }
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A well-formed act the host declined anyway. Several unrelated things
    /// arrive under it, and the message is the only thing that tells them
    /// apart.
    ///
    /// On [`request`](crate::client::request), the answer to a question a
    /// client half asked: the handler's own [`Refusal`], forwarded with the
    /// host's prefix naming which mod refused which request. The same kind
    /// comes back when the answering half is not running at all, when it is
    /// too far behind to take the call, and when the guest failed outright.
    ///
    /// On [`Context::cause`](crate::Context::cause) and
    /// [`Context::instance`](crate::Context::instance), a question about an
    /// event that is not the one being handled. Both answer only about the
    /// event in flight, so a context kept past its handler refuses instead of
    /// reporting whatever ran last. `instance` refuses this way for a second
    /// reason: most events are about no instance at all, and only an
    /// interaction or a contact names one.
    Refused,
    /// A payload over the host's byte cap, refused whole.
    ///
    /// One cap, one kind, on every route. A raise through
    /// [`signal`](crate::server::signal) or
    /// [`signal_to`](crate::server::signal_to) and a question through
    /// [`request`](crate::client::request) all meet the same number and all
    /// answer this, so a mod that handles it once handles it everywhere.
    /// Nothing goes out truncated or in part, and the cap and the size that
    /// arrived are both in the message.
    ///
    /// The session's table of field paths has a cap of its own, and a mod that
    /// resolves paths it invents rather than paths a component has will exhaust
    /// it. [`resolve::field`](crate::server::resolve::field) answers this kind
    /// from then on, saying the table is full rather than blaming the path.
    TooLarge,
    /// A name no enabled mod declares, or an id this session does not carry.
    ///
    /// Both readings are the same fact. Names are numbered per session, and a
    /// number only means something in the session that minted it. So
    /// [`resolve`](crate::server::resolve) refuses a signal, request, sound,
    /// component, field path or source that no enabled mod declares — which
    /// includes a name declared in the other realm, and a name declared as a
    /// different kind — and a verb refuses an id this session's tables never
    /// minted, whether a mod invented the number or held it over from a session
    /// that has ended. It is also what a mod meets when it hands a component's
    /// field id to a different component.
    UnresolvedName,
    /// The host took nothing, because it is carrying as much as it will.
    ///
    /// A raise is synchronous and its `Ok` means the host accepted it, so this
    /// is the answer when it cannot: a full queue, a full inbox at the other
    /// end, or a mod that has spent the raises its budget allows for one tick.
    /// It is typed rather than a silent shed precisely so a mod can tell the
    /// difference between having announced something and only having tried.
    ///
    /// It says try again, unlike every other kind here. The condition is a
    /// moment's, not a mistake's: the next tick has a fresh budget and a queue
    /// that has drained. What it does not say is which of the two happened, so
    /// a mod tuning how much it raises reads the message.
    Overloaded,
    /// A handle that has been spent or whose event is over. Keep the ids
    /// instead and resolve again.
    ///
    /// An [`Entity`](crate::server::Entity) is spent for every clone at once by
    /// [`despawn`](crate::server::Entity::despawn). A
    /// [`Player`](crate::server::Player), or the entity inside a
    /// [`Target`](crate::server::Target), belongs to the event that lent it and
    /// is reclaimed when that handler returns. The SDK catches all of those in
    /// the guest, before the call leaves, which is the point: the alternative
    /// to a typed refusal here is the number resolving to whoever took it next.
    StaleId,
    /// A code this SDK does not name. Read [`Error::code`] and the message.
    ///
    /// In a session it is one host code, the one the host uses for its own side
    /// of the line — a bridge that closed, a reply that was lost, an entity
    /// handle it could not resolve. Less obviously, every verdict the world
    /// passes back on a write arrives here too — a non-finite number, a
    /// component the entity does not carry, a body this mod may not touch — so
    /// this is the kind where an author reads the message instead of branching.
    ///
    /// Code zero is the SDK's own and crossed no wire: a payload that would not
    /// encode or decode, a session-only verb called on the build machine, a
    /// world that answered a field with the wrong shape.
    Other,
}

impl Error {
    pub(crate) fn from_wire(code: u32, message: String, data: Vec<u8>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    /// Reads [`code`](Error::code) as one of the [`ErrorKind`] cases, or
    /// [`Other`](ErrorKind::Other) with the number left intact. This is the
    /// value a mod branches on.
    pub fn kind(&self) -> ErrorKind {
        match self.code {
            code::REFUSAL => ErrorKind::Refused,
            code::TOO_LARGE => ErrorKind::TooLarge,
            code::UNRESOLVED_NAME => ErrorKind::UnresolvedName,
            code::OVERLOADED => ErrorKind::Overloaded,
            code::STALE_ID => ErrorKind::StaleId,
            _ => ErrorKind::Other,
        }
    }

    /// The number as the host sent it, unmapped. For a log line, and for
    /// telling two [`Other`](ErrorKind::Other) errors apart. Zero is the SDK's
    /// own and crossed no wire.
    pub fn code(&self) -> u32 {
        self.code
    }

    /// The sentence about this refusal, in the words of whoever issued it. It
    /// is the only part that separates the several causes a single
    /// [`ErrorKind`] covers, so it belongs in the log line a mod writes.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The refusal's structured payload, which is the contract's room for one.
    /// Nothing the host or this crate builds attaches bytes, so this is empty.
    /// Read [`message`](Error::message).
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "host refused (code {}): {}", self.code, self.message)
    }
}

impl core::error::Error for Error {}

/// An answer, and the answer is no.
///
/// A handler registered with
/// [`respond`](crate::protocol::RequestSpec::respond) on the request type
/// returns `Result<Resp, Refusal>`, and both arms travel. The response and the refusal
/// ride the same wire back to the client half whose
/// [`request`](crate::client::request) is waiting. This is the one place in the
/// crate where an author writes an error rather than handling one, and the
/// reason is that somebody asked.
///
/// # Not the same thing as an [`Error`]
///
/// An [`Error`] is a verb that did not happen. The mod asked the world for
/// something and the world declined. A `Refusal` is a reply the mod composed
/// on purpose, and refusing is a normal outcome of answering rather than a
/// fault.
///
/// They meet at the caller. [`request`](crate::client::request) answers
/// `Result<Resp, Error>`, so a handler's refusal arrives on the client half as
/// an [`Error`] of kind [`ErrorKind::Refused`], carrying this value's
/// `Display` text as its message. The caller never sees the variant.
///
/// # Refuse with numbers
///
/// The variants exist so a refusal says what the caller has to change, and the
/// constructors exist so the numbers cannot drift from the check that produced
/// them. [`out_of_range`](Refusal::out_of_range) takes the range itself.
///
/// The example refuses a request to set a volume. `Refusal` and
/// [`Result`] both live at the crate root, and a handler that returns one
/// names the pair directly rather than through a prelude.
///
/// ```
/// // Both come from the crate root and belong to no realm.
/// use ironlark::{Refusal, Result};
///
/// // The response type a handler answers with when it accepts.
/// struct Set { volume: i64 }
///
/// fn set_volume(asked: i64) -> Result<Set, Refusal> {
///     let accepted = 0..=100;
///     if !accepted.contains(&asked) {
///         // The check and the refusal read the same range.
///         return Err(Refusal::out_of_range("volume", accepted));
///     }
///     Ok(Set { volume: asked })
/// }
///
/// assert!(set_volume(30).is_ok());
/// // Display is what the caller receives, so the bounds are in the sentence.
/// assert_eq!(
///     set_volume(400).err().map(|no| no.to_string()).as_deref(),
///     Some("volume is out of range 0..=100"),
/// );
/// ```
///
/// [`Unknown`](Refusal::Unknown) is the escape hatch and carries a sentence
/// and nothing else. It is the right answer when the reason genuinely has no
/// numbers — a rule of the gamemode, a phase of the round — and the wrong one
/// when the reason does, because a caller cannot act on prose.
///
/// # What crosses the wire
///
/// Only the text. The guest turns a handler's `Err` into an error whose
/// message is this value's `Display` and whose data is empty. The host then
/// prefixes which mod refused which request and forwards that. The caller reads
/// a string, never the fields. That is what makes the built-in `Display`
/// load-bearing. It puts the field name and both bounds in the sentence, and
/// that sentence is the only place the caller can find them.
///
/// The vocabulary is `#[non_exhaustive]` and grows on this crate's own semver
/// rather than the host's, because the host neither reads nor validates it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// A field fell outside the range the handler accepts. Build it with
    /// [`out_of_range`](Refusal::out_of_range), which reads both bounds off
    /// the range the check used.
    OutOfRange {
        /// Which field.
        field: &'static str,
        /// The lowest value accepted.
        min: i64,
        /// The highest value accepted.
        max: i64,
    },
    /// The caller wears no persona, and this answer needs one. See
    /// [`no_profile`](Refusal::no_profile) for what a session answers.
    NoProfile,
    /// The request exceeded a cap the handler publishes. The host's own byte cap
    /// is [`ErrorKind::TooLarge`](ErrorKind::TooLarge), a different kind
    /// entirely.
    TooLarge {
        /// The cap.
        cap: usize,
        /// What arrived.
        got: usize,
    },
    /// A reason the vocabulary above does not name. The sentence is the whole
    /// answer, so it is right only when the reason has no numbers in it.
    Unknown {
        /// What to tell the caller.
        message: String,
    },
}

impl Refusal {
    /// Builds an [`OutOfRange`](Refusal::OutOfRange) from the range the check
    /// itself used, so the bounds a caller is told cannot drift from the
    /// bounds that rejected it. `field` names what to change and rides into
    /// the `Display` sentence the caller reads.
    pub fn out_of_range(field: &'static str, range: RangeInclusive<i64>) -> Self {
        Refusal::OutOfRange {
            field,
            min: *range.start(),
            max: *range.end(),
        }
    }

    /// Builds a [`NoProfile`](Refusal::NoProfile), for a handler whose answer
    /// needs the persona the caller is playing.
    ///
    /// [`Player::profile`](crate::server::Player::profile) answers `None` for
    /// every participant in a session, because personas are not built. A
    /// handler gated on one therefore refuses every caller. Write the gate
    /// only where the answer genuinely has no meaning without a persona.
    pub fn no_profile() -> Self {
        Refusal::NoProfile
    }

    /// Builds a [`TooLarge`](Refusal::TooLarge), naming the handler's own cap
    /// and what arrived, so both numbers reach the caller in the sentence.
    pub fn too_large(cap: usize, got: usize) -> Self {
        Refusal::TooLarge { cap, got }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::OutOfRange { field, min, max } => {
                write!(f, "{field} is out of range {min}..={max}")
            }
            Refusal::NoProfile => write!(f, "the caller plays no profile"),
            Refusal::TooLarge { cap, got } => {
                write!(f, "payload of {got} bytes exceeds the cap of {cap}")
            }
            Refusal::Unknown { message } => f.write_str(message),
        }
    }
}

impl core::error::Error for Refusal {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_codes_stay_other_with_code_intact() {
        let e = Error::from_wire(9001, "vault sealed".into(), Vec::new());
        assert_eq!(e.kind(), ErrorKind::Other);
        assert_eq!(e.code(), 9001);
    }

    #[test]
    fn known_codes_map() {
        let e = Error::from_wire(code::TOO_LARGE, String::new(), Vec::new());
        assert_eq!(e.kind(), ErrorKind::TooLarge);
    }

    #[test]
    fn an_overloaded_host_reads_as_its_own_kind() {
        let e = Error::from_wire(code::OVERLOADED, "budget spent".into(), Vec::new());
        assert_eq!(e.kind(), ErrorKind::Overloaded);
        assert_ne!(
            e.kind(),
            ErrorKind::Other,
            "the code must not read as Other"
        );
    }
}
