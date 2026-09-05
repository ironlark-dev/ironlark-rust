//! Bridges the standard `log` facade to the host import, so `log::info!`
//! works in the mod and in its dependencies. Installed by the export glue
//! before `init`.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::cell::RefCell;
use std::fmt::Write as _;

struct Bridge;

thread_local! {
    // Reused between records; taken out while formatting so a Display that
    // itself logs borrows a fresh buffer instead of aliasing this one.
    static BUFFER: RefCell<String> = const { RefCell::new(String::new()) };
}

impl Log for Bridge {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        let mut buffer = BUFFER.with(|b| b.take());
        buffer.clear();
        if write!(buffer, "{}", record.args()).is_err() {
            buffer.clear();
            let _ = write!(buffer, "<a Display impl failed while logging>");
        }
        emit(record.level(), &buffer);
        BUFFER.with(|b| b.replace(buffer));
    }

    fn flush(&self) {}
}

#[cfg(target_arch = "wasm32")]
fn emit(level: Level, msg: &str) {
    use crate::bindings::server::ironlark::host::log as host;
    let lvl = match level {
        Level::Trace => host::Level::Trace,
        Level::Debug => host::Level::Debug,
        Level::Info => host::Level::Info,
        Level::Warn => host::Level::Warn,
        Level::Error => host::Level::Error,
    };
    host::log(lvl, msg);
}

#[cfg(not(target_arch = "wasm32"))]
fn emit(level: Level, msg: &str) {
    crate::testing::record_log(level, msg);
}

static BRIDGE: Bridge = Bridge;

/// Idempotent; the default max level is Info until the author raises it.
pub(crate) fn install() {
    if log::set_logger(&BRIDGE).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
