//! The placement double: records what a gamemode asked of the host.

use std::cell::RefCell;

thread_local! {
    static SPAWN_COMMANDS: RefCell<Vec<SpawnCommand>> = const { RefCell::new(Vec::new()) };
}

/// One placement command a gamemode issued.
///
/// [`SpawnSettings`](crate::server::gamemode::SpawnSettings) carries a
/// gamemode's decisions to the host, and each verb on it that a test drove
/// arrives here as one case. A session has no such record: there the command
/// travels and the host answers it.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{SpawnCommand, block_on, take_spawn_commands};
///
/// // What a gamemode's `init` says when it takes placement over.
/// block_on(async {
///     let _ = gamemode::spawn().disable_auto().await;
/// });
///
/// assert_eq!(take_spawn_commands(), vec![SpawnCommand::DisableAuto]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnCommand {
    /// The host was asked to place arrivals itself.
    EnableAuto,
    /// The host was asked to stop placing arrivals.
    DisableAuto,
}

pub(crate) fn record_spawn_command(command: SpawnCommand) {
    SPAWN_COMMANDS.with(|c| c.borrow_mut().push(command));
}

/// Drains the placement commands the code under test issued.
///
/// Every awaited [`SpawnSettings`](crate::server::gamemode::SpawnSettings), in
/// the order the awaits completed. Settings nothing was asked of leave no
/// record, because they never reach the host either. The buffer is emptied, so
/// a second call answers with what has happened since.
///
/// This is the whole of what a test can see here. The build machine runs no
/// session, so nothing places anybody and nothing refuses: a half that does not
/// hold the gamemode role is turned down by a session and recorded by this
/// double.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{SpawnCommand, block_on, take_spawn_commands};
///
/// // What a gamemode's `init` says when it owns placement.
/// block_on(async {
///     let _ = gamemode::spawn().disable_auto().await;
/// });
///
/// assert_eq!(take_spawn_commands(), vec![SpawnCommand::DisableAuto]);
/// ```
pub fn take_spawn_commands() -> Vec<SpawnCommand> {
    SPAWN_COMMANDS.with(|c| c.borrow_mut().drain(..).collect())
}
