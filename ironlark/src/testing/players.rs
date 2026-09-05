//! The stated participants: the one an event is about, and the session table.

use crate::ids::{ProfileId, SessionId, UserId};
use std::cell::RefCell;

thread_local! {
    static PARTICIPANTS: RefCell<Vec<FakeParticipant>> = const { RefCell::new(Vec::new()) };
}

/// The participant a test hands to a handler, in place of the one a host lends.
///
/// In a session a [`Player`](crate::server::Player) is a pass the engine mints
/// for one call and reclaims on the way out. The handler cannot construct one,
/// which is the point of it. On the build machine nobody mints anything, so a
/// test states the facts itself and turns them into a handle with
/// [`into_player`](FakePlayer::into_player).
///
/// It is the participant an event is ABOUT. [`FakeParticipant`] is the other
/// half of the picture, a line in the session's own list of who is connected,
/// and the two are independent: a test whose handler looks up the name of the
/// player it was handed states that player in both places.
///
/// The fields are public. A participant with no account and no persona, a bot
/// or anyone whose persona the test does not turn on, is [`FakePlayer::new`].
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{FakePlayer, block_on, context};
///
/// // `Context` and `Player` are that prelude's. A session mints both. Here
/// // `context` and `into_player` do.
/// async fn tag(_ctx: Context, player: Player) -> String {
///     match player.profile().await {
///         Ok(Some(profile)) => format!("{profile}"),
///         Ok(None) => "no persona".to_string(),
///         Err(e) => format!("the persona could not be read: {e}"),
///     }
/// }
///
/// let bot = FakePlayer::new(SessionId::new(3));
/// let named = FakePlayer {
///     profile: Some(ProfileId::new(0x5eed)),
///     ..bot
/// };
///
/// let event = context(Tick::new(12), Tick::new(12));
/// assert_eq!(block_on(tag(event, bot.into_player())), "no persona");
/// assert_ne!(block_on(tag(event, named.into_player())), "no persona");
/// ```
///
/// # What it answers, and where that is not what a session answers
///
/// [`session`](crate::server::Player::session) and
/// [`user`](crate::server::Player::user) answer what the test set, exactly as a
/// session answers them. Both are stamped into the handle rather than asked
/// for.
///
/// [`profile`](crate::server::Player::profile) answers what the test set, and
/// never fails. That is close to a session and not the same. The contract's
/// persona read carries no error arm of its own, so the only refusal it can
/// produce is the stale-handle check, which is the one thing this double has no
/// way to produce. A handler's `Err` branch is therefore never taken here. Its
/// `Ok(Some(_))` branch is the opposite case and the reason to set the field at
/// all. A session answers no persona for everyone, because personas are not
/// built, so a test is the only place a mod's persona logic runs.
///
/// [`body`](crate::server::Player::body) refuses. The double controls no
/// entity, and inventing one would make possession logic pass against nothing.
/// The refusal carries no host code, so
/// [`ErrorKind::Other`](crate::ErrorKind::Other) is its kind. That is also the
/// class a session's body failure arrives as, so a mod matching on the kind
/// matches the same arm on both, and only the message says which happened.
///
/// The larger difference is staleness, and it is the crate's headline rule. A
/// real handle refuses every call once its event is over, because the host
/// reclaims the number and it would go on to address whoever came next. The
/// stamp that enforces it is a field of the component build alone. Reviewing
/// for a hoarded handle is a reading job, not something a native test catches.
#[derive(Debug, Clone, Copy)]
pub struct FakePlayer {
    /// The connection id the double answers with.
    pub session: SessionId,
    /// The account, or none for a bot.
    pub user: Option<UserId>,
    /// The persona, or none.
    pub profile: Option<ProfileId>,
}

impl FakePlayer {
    /// A participant with no account and no persona.
    ///
    /// The common case, because most handlers care only about the connection
    /// they are answering. Set [`user`](FakePlayer::user) and
    /// [`profile`](FakePlayer::profile) directly where the logic under test
    /// turns on them.
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            user: None,
            profile: None,
        }
    }

    /// Hands it to a handler as the real thing.
    ///
    /// The handler's signature names [`Player`](crate::server::Player), which
    /// only a host can build, so this is the one door into that type on the
    /// build machine. Nothing at the call site distinguishes the result from a
    /// lent handle. That is what keeps the handler under test the same code
    /// that runs in a session.
    pub fn into_player(self) -> crate::server::player::Player {
        crate::server::player::Player::from_fake(self)
    }
}

/// One row of the session table a test states: who is connected, and what they
/// are called.
///
/// [`session`](crate::server::session) reads the host's table, and on the build
/// machine there is no host, so a test states the table itself with
/// [`set_participants`]. One of these is one participant in it.
///
/// Both fields are public, so a row is written as a literal. `name` is `None`
/// for a participant the server admitted without stating a name, which is the
/// case that makes [`name_of`](crate::server::session::name_of) answer `None`
/// for somebody who is genuinely here.
///
/// It carries a name and nothing else because the host's table holds a name and
/// nothing else. The contract answers a list of pairs so a second fact can
/// arrive without changing the call, and the day one does, this type grows the
/// field the host grew.
///
/// It is not a [`FakePlayer`]. That one is lent for one event; this one is a
/// line in the session's list.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::server::session;
/// use ironlark::testing::{FakeParticipant, block_on, set_participants};
///
/// set_participants(vec![
///     FakeParticipant { session: SessionId::new(1), name: Some("Ada".to_string()) },
///     // Connected, and the server stated no name for them.
///     FakeParticipant { session: SessionId::new(2), name: None },
/// ]);
///
/// assert_eq!(block_on(session::participants()).len(), 2);
/// assert_eq!(block_on(session::name_of(SessionId::new(2))), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeParticipant {
    /// The number the host addresses them by.
    pub session: SessionId,
    /// What they are called, or `None` where the session states no name.
    pub name: Option<String>,
}

/// States the session table [`session`](crate::server::session) reads.
///
/// It replaces the whole table rather than adding to it, so one call states the
/// session as a test means it to be and a later call states a different
/// session. That is what a join and a leave look like here: state the list
/// again. Nothing runs, so nobody arrives or departs unasked and nothing decays
/// between calls.
///
/// [`participants`](crate::server::session::participants) answers these in the
/// order they were stated. [`name_of`](crate::server::session::name_of) and
/// [`get_all`](crate::server::session::get_all) answer for the row whose
/// [`session`](FakeParticipant::session) matches, and answer empty for a number
/// no row carries — which is what a session answers for a stranger too.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::server::session;
/// use ironlark::testing::{FakeParticipant, block_on, set_participants};
///
/// // What a mod does when it is enabled into a session already in progress:
/// // it asks who is here instead of waiting for arrivals it already missed.
/// set_participants(vec![
///     FakeParticipant { session: SessionId::new(4), name: Some("Ada".to_string()) },
///     FakeParticipant { session: SessionId::new(9), name: Some("Grace".to_string()) },
/// ]);
///
/// let present = block_on(session::participants());
/// assert_eq!(present, vec![SessionId::new(4), SessionId::new(9)]);
///
/// // Stating it again is how a test moves the session on.
/// set_participants(Vec::new());
/// assert!(block_on(session::participants()).is_empty());
/// ```
///
/// # Nothing refuses
///
/// The session verbs have no error arm, so nothing about a stated table can be
/// turned down. Two rows carrying one [`SessionId`] are not refused either: the
/// first match wins, and that is a test stating what a session could not
/// produce.
pub fn set_participants(present: Vec<FakeParticipant>) {
    PARTICIPANTS.with(|p| *p.borrow_mut() = present);
}

pub(crate) fn stated_participants() -> Vec<SessionId> {
    PARTICIPANTS.with(|p| p.borrow().iter().map(|row| row.session).collect())
}

pub(crate) fn stated_name(player: SessionId) -> Option<String> {
    PARTICIPANTS.with(|p| {
        p.borrow()
            .iter()
            .find(|row| row.session == player)
            .and_then(|row| row.name.clone())
    })
}
