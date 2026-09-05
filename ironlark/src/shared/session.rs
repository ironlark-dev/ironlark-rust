//! The session: who is connected right now, and what the server says about one
//! of them as named string values. Read-only, because the host owns the table.
//!
//! Names are not identities. Address a participant by the
//! [`SessionId`](crate::SessionId) the host hands out, and use a name only to
//! show a human something.

use crate::ids::SessionId;

/// The key the server states a display name under, and the one fact the table
/// carries.
const NAME: &str = "name";

/// Everyone connected right now, as the numbers the host addresses them by.
///
/// A snapshot and nothing more. Someone may arrive or leave between the answer
/// and the caller acting on it, so treat the list as what was true when it was
/// asked rather than as a set to keep. What a mod keeps instead is its own
/// list, grown in [`on_join`](crate::server::ServerMod::on_join) and shrunk in
/// [`on_leave`](crate::server::ServerMod::on_leave), and this call is how a mod
/// that missed those hooks — one enabled mid-session, or one recovering after a
/// reload — learns who is already here.
///
/// It is the same verb at two paths.
/// [`server::session`](crate::server::session) and
/// [`client::session`](crate::client::session) both re-export it, and both read
/// the same table: the host fills its copy as it admits participants and
/// announces each row on the wire, and every other machine's copy follows those
/// announcements.
///
/// Bots are in it. Every participant has a [`SessionId`](crate::SessionId),
/// whether a person or the server itself is behind them, so a count taken here
/// is a count of occupied player slots and not of people.
///
/// # Example
///
/// A scoreboard that seeds itself from whoever is already connected.
///
/// ```
/// use ironlark::server::prelude::*;
/// // The verb's origin, spelled out: the session module under the realm door.
/// use ironlark::server::session;
///
/// // What the mod keeps. The snapshot seeds it; the hooks maintain it.
/// ironlark::state! {
///     static PRESENT: Vec<SessionId> = Vec::new();
/// }
///
/// struct Scoreboard;
///
/// impl ServerMod for Scoreboard {
///     async fn init() {
///         // One crossing into the host, once, rather than a read per tick.
///         let present = session::participants().await;
///         PRESENT.set(present);
///     }
///
///     async fn on_join(_ctx: Context, player: Player) {
///         let who = player.session();
///         PRESENT.update(|held| held.push(who));
///     }
///
///     async fn on_leave(_ctx: Context, player: Player, _reason: LeaveReason) {
///         let who = player.session();
///         PRESENT.update(|held| held.retain(|held| *held != who));
///     }
/// }
/// ```
///
/// # Nothing refuses
///
/// There is no [`Result`](crate::Result) to match on. An empty list is the
/// honest answer for a session nobody has joined yet, and it is also what a
/// half that asks before the host has filled its copy sees, so an empty answer
/// is not proof that a session is empty.
///
/// # What it costs
///
/// One crossing into the host and one allocation, sized to the number of
/// participants. Nothing is cached on either side of the boundary, so a read
/// per tick is a read per tick. Ask when the set of participants changes.
///
/// # On the build machine
///
/// The answer is whatever the test stated with
/// [`set_participants`](crate::testing::set_participants), and an empty list
/// until it states something. There is no session, so nobody joins or leaves on
/// their own.
pub async fn participants() -> Vec<SessionId> {
    backend::participants().await
}

/// What a participant is called, or `None` when the session has no name for
/// them.
///
/// The one fact a mod usually wants, so it is one call instead of the search
/// through [`get_all`] that every caller wrote by hand. `player` is the
/// [`SessionId`](crate::SessionId) the host addresses a participant by. The
/// name is what the server stated when it admitted them.
///
/// It reads the whole table row and keeps the one value, because that is what
/// the contract offers: there is no keyed read. So this costs exactly what
/// [`get_all`] costs, and the saving is in the mod's own code rather than at
/// the boundary.
///
/// The answer is for showing to a human. A name is not unique, nothing
/// authenticates it, and a participant may have none, so never key anything on
/// one. Address a participant by their [`SessionId`](crate::SessionId).
///
/// It is the same verb at two paths, [`server::session`](crate::server::session)
/// and [`client::session`](crate::client::session), over one table.
///
/// # Example
///
/// The label a mod shows for a participant, with a fallback that still names
/// them.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::server::session;
/// // The double stands in for the host's table on the build machine.
/// use ironlark::testing::{FakeParticipant, block_on, set_participants};
///
/// async fn label(player: SessionId) -> String {
///     match session::name_of(player).await {
///         Some(name) => name,
///         // Nobody by that number, or nobody named. SessionId is Display.
///         None => format!("player#{player}"),
///     }
/// }
///
/// set_participants(vec![FakeParticipant {
///     session: SessionId::new(7),
///     name: Some("Ada".to_string()),
/// }]);
///
/// assert_eq!(block_on(label(SessionId::new(7))), "Ada");
/// assert_eq!(block_on(label(SessionId::new(8))), "player#8");
/// ```
///
/// # Nothing refuses
///
/// `None` covers every way this can come back empty, and they are not the same
/// situation: a number that names nobody, a participant who has left, a
/// participant the server admitted without stating a name, and a host that has
/// not filled its copy of the table yet. The signature has no error to tell
/// them apart with, and only the last of them writes a host log line.
///
/// # What is not built
///
/// A name is stated once, when the participant is admitted, and nothing
/// restates it. A participant who changes their name mid-session keeps the name
/// every mod already read for the rest of the session. The table under this
/// call accepts an update, so the day a rename is announced this call answers
/// the new name without changing shape.
///
/// # On the build machine
///
/// The answer is the [`name`](crate::testing::FakeParticipant::name) the test
/// stated for that participant, and `None` for anyone it did not state. A test
/// about how a mod renders names has to state them.
pub async fn name_of(player: SessionId) -> Option<String> {
    get_all(player)
        .await
        .into_iter()
        .find(|(key, _)| key == NAME)
        .map(|(_, value)| value)
}

/// Every fact the session holds about one participant, as pairs of name and
/// value.
///
/// The honest name, because that is what the verb does: it answers the whole
/// row. `player` is the [`SessionId`](crate::SessionId) the host addresses a
/// participant by. Reach for [`name_of`] when the display name is what is
/// wanted, which is what a mod rendering a label or a chat line wants, and reach
/// for this one when a mod means to read the row itself.
///
/// The list shape is what lets a second fact arrive without changing this
/// call, so a caller looks its key up rather than indexing.
///
/// It is the same verb at two paths, [`server::session`](crate::server::session)
/// and [`client::session`](crate::client::session), over one table.
///
/// # What the row holds
///
/// Exactly one fact: the display name, under the key `"name"`, stated by the
/// server when it admits a participant. A mod written against the list handles
/// the second fact the day it arrives; a mod that indexed `[0]` does not.
///
/// # Example
///
/// Showing the whole row, whatever it holds.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::server::session;
/// use ironlark::testing::{FakeParticipant, block_on, install_logger, set_participants};
///
/// async fn describe(player: SessionId) {
///     // One crossing into the host, answering every fact it holds.
///     for (fact, value) in session::get_all(player).await {
///         log::info!("{player}: {fact} = {value}");
///     }
/// }
///
/// install_logger();
/// set_participants(vec![FakeParticipant {
///     session: SessionId::new(7),
///     name: Some("Ada".to_string()),
/// }]);
///
/// block_on(describe(SessionId::new(7)));
/// ```
///
/// # Nothing refuses
///
/// An empty list is every negative answer at once: an id that names nobody, one
/// belonging to a participant who left, and a host that has not filled its copy
/// of the table. A caller holding a stale id gets nothing rather than an error
/// to handle.
///
/// The values are for showing to a human. A name is not unique and nothing
/// authenticates it, so never key anything on one. Address a participant by
/// their [`SessionId`](crate::SessionId).
///
/// # What it costs
///
/// One crossing into the host per call, and the host allocates and copies a
/// string pair over the boundary for every fact in the row. There is no keyed
/// read to ask for less. A mod rendering a name per participant per tick pays
/// that every tick, so read when the set of participants changes rather than
/// once per frame.
///
/// # On the build machine
///
/// The row is built from what the test stated with
/// [`set_participants`](crate::testing::set_participants): one `"name"` pair
/// for a participant it named, and an empty list for anyone else.
pub async fn get_all(player: SessionId) -> Vec<(String, String)> {
    backend::get_all(player).await
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use crate::ids::SessionId;

    pub async fn participants() -> Vec<SessionId> {
        crate::bindings::server::ironlark::host::session::participants()
            .await
            .into_iter()
            .map(SessionId::new)
            .collect()
    }

    pub async fn get_all(player: SessionId) -> Vec<(String, String)> {
        crate::bindings::server::ironlark::host::session::get_all(player.to_wire()).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::NAME;
    use crate::ids::SessionId;

    pub async fn participants() -> Vec<SessionId> {
        crate::testing::stated_participants()
    }

    pub async fn get_all(player: SessionId) -> Vec<(String, String)> {
        match crate::testing::stated_name(player) {
            Some(name) => vec![(NAME.to_string(), name)],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeParticipant, block_on, set_participants};

    #[test]
    fn a_name_is_read_out_of_the_row() {
        set_participants(vec![FakeParticipant {
            session: SessionId::new(3),
            name: Some("Ada".to_string()),
        }]);
        assert_eq!(block_on(name_of(SessionId::new(3))).as_deref(), Some("Ada"));
        assert_eq!(block_on(get_all(SessionId::new(3))).len(), 1);
    }

    #[test]
    fn a_stranger_has_no_name_and_an_empty_row() {
        set_participants(Vec::new());
        assert_eq!(block_on(name_of(SessionId::new(99))), None);
        assert!(block_on(get_all(SessionId::new(99))).is_empty());
    }

    #[test]
    fn participants_answer_in_the_order_they_were_stated() {
        set_participants(vec![
            FakeParticipant {
                session: SessionId::new(1),
                name: None,
            },
            FakeParticipant {
                session: SessionId::new(2),
                name: None,
            },
        ]);
        assert_eq!(
            block_on(participants()),
            vec![SessionId::new(1), SessionId::new(2)]
        );
    }
}
