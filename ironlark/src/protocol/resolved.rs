//! Declared name to compact id, resolved once per name per session.
//!
//! A resolve crosses into the host, so it happens on a name's first use and
//! never again. The answers are kept per kind, because the number carries no
//! kind and each verb reads it in the table its own kind keeps.

use crate::error::Result;
use crate::ids::{RequestId, SignalId};
use std::collections::HashMap;

crate::state! {
    static SIGNALS: HashMap<&'static str, SignalId> = HashMap::new();
    static REQUESTS: HashMap<&'static str, RequestId> = HashMap::new();
}

pub(crate) fn signal(name: &'static str) -> Result<SignalId> {
    if let Some(id) = SIGNALS.update(|held| held.get(name).copied()) {
        return Ok(id);
    }
    let id = crate::shared::resolve::signal(name)?;
    SIGNALS.update(|held| held.insert(name, id));
    Ok(id)
}

pub(crate) fn request(name: &'static str) -> Result<RequestId> {
    if let Some(id) = REQUESTS.update(|held| held.get(name).copied()) {
        return Ok(id);
    }
    let id = crate::shared::resolve::request(name)?;
    REQUESTS.update(|held| held.insert(name, id));
    Ok(id)
}
