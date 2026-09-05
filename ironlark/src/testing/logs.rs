//! The log double: the bridge installer and the drained lines.

use log::Level;
use std::cell::RefCell;

thread_local! {
    static LOGS: RefCell<Vec<(Level, String)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record_log(level: Level, msg: &str) {
    LOGS.with(|l| l.borrow_mut().push((level, msg.to_string())));
}

/// Points the `log` facade at the buffer [`take_logs`] drains.
///
/// A mod never installs a logger. The export glue installs one at the top of
/// the mod's `init`, so `log::info!` works in the mod and in everything the mod
/// depends on. A test calls no glue, so the facade would otherwise discard
/// every line. Call this once, at the top of a test that intends to read what
/// was logged.
///
/// It installs the same bridge the wasm build installs. It is idempotent, so
/// calling it in every test is fine, and it sets the maximum level to `Info`,
/// which the author raises with `log::set_max_level` where a test needs to see
/// `debug!` or `trace!`. A handler that only raises, requests or plays is fully
/// observable without a logger.
///
/// # Where a session differs
///
/// A session attributes each line to the mod that wrote it and holds budgets a
/// test does not: sixty-four lines per call into the mod, the last of them
/// spent on a loud line saying the rest of that call's output is dropped, and a
/// cut at eight kilobytes per line. Neither budget exists here, so a handler
/// that logs per entity per tick drains in full here and loses most of its
/// output in a game.
///
/// ```
/// use ironlark::server::prelude::*;
/// use ironlark::testing::{block_on, install_logger, take_logs};
/// use log::Level;
///
/// // Call it once, before the handler under test runs.
/// install_logger();
///
/// async fn announce() {
///     log::info!("the door opened");
/// }
///
/// block_on(announce());
///
/// // The whole line compared whole: the level and the text are one record.
/// assert_eq!(
///     take_logs(),
///     vec![(Level::Info, "the door opened".to_string())],
/// );
/// ```
pub fn install_logger() {
    crate::host::log_bridge::install();
}

/// Drains what the code under test logged: level and formatted message.
///
/// Every line since the last drain, in order, with the arguments already
/// formatted into a `String`. The buffer is emptied, so a second call answers
/// with what has happened since. Empty unless [`install_logger`] was called,
/// and empty for `debug!` and `trace!` unless the test also raised the maximum
/// level.
///
/// A mod writes its lines with the `log` facade, which this crate depends on
/// and re-exports nothing of. The level a drained line carries is that crate's
/// own `Level`, so the example imports it from there.
///
/// ```
/// use ironlark::testing::{install_logger, take_logs};
/// use log::Level;
///
/// // Without this the facade discards the line and the drain answers empty.
/// install_logger();
/// log::warn!("the door is stuck");
///
/// assert_eq!(take_logs(), vec![(Level::Warn, "the door is stuck".to_string())]);
/// assert!(take_logs().is_empty());
/// ```
///
/// # What is recorded here, and what a session records
///
/// This is the one drain whose contents also exist in a session, through the
/// same bridge, feeding the engine's own log instead of a buffer and attributed
/// to the mod that wrote each line. What a session additionally does is bound
/// each call into the mod to sixty-four log lines and cut a line at eight
/// kilobytes, so a chatty handler drains in full here and loses most of its
/// output there.
///
/// The buffer is not the mod's alone. Dispatch writes into it too: the warning
/// for a signal nothing observes, and the one for a payload that did not decode.
/// A line a test did not expect is worth reading before it is filtered out.
pub fn take_logs() -> Vec<(Level, String)> {
    LOGS.with(|l| l.borrow_mut().drain(..).collect())
}
