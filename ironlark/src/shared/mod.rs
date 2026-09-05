//! One implementation per verb family both realms re-export. The realm
//! modules [`server`](crate::server) and [`client`](crate::client) each carry
//! their own page for these; nothing here is a public path of its own.

pub(crate) mod audio;
pub(crate) mod resolve;
pub(crate) mod session;
