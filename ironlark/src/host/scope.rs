//! The per-call counter that tells one hook's handles from the next one's.
//! Private to the crate: an author never sees it, and it is not the event id.

#[cfg(any(target_arch = "wasm32", test))]
crate::state! {
    /// Counts event scopes. The host lends its player and entity handles for
    /// one handler and reclaims them after, so a handle carries the count it
    /// was stamped with and refuses once the count has moved on. Wrapping is
    /// harmless: a handle would have to survive four billion events to
    /// collide, and every use in between refuses.
    static EPOCH: u32 = 0;
}

/// The scope a handle must have been stamped in to still answer.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn epoch() -> u32 {
    EPOCH.get()
}

/// Opens the next event scope. Called by the export glue before it hands a
/// handler anything the host owns.
///
/// Deliberately not the event id. One physical event becomes several calls
/// into one mod once dispatch runs per feature, and a handle lent in the first
/// call must not answer in the third; a count that moves per call refuses
/// there, and an id shared across them would not.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn open_scope() {
    EPOCH.set(EPOCH.get().wrapping_add(1));
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_scope_is_a_new_one() {
        let first = super::epoch();
        super::open_scope();
        let second = super::epoch();
        assert_ne!(first, second, "a handler must not inherit the last scope");
        super::open_scope();
        assert_ne!(second, super::epoch());
    }
}
