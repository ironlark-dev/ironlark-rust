//! The one liveness every copy of an [`Entity`](super::Entity) shares.

/// The handle every copy of one [`Entity`](super::Entity) shares. `owned` is false for the
/// per-event handles the host lends and takes back itself. Built for the
/// session target and for tests, so the ownership rules below are exercised
/// without a host; only the release itself needs one.
pub(super) struct Held {
    raw: u32,
    pub(super) owned: bool,
    alive: core::cell::Cell<bool>,
    /// The event scope a lent handle belongs to; meaningless when owned.
    epoch: u32,
}

impl Held {
    /// A handle this mod owns until it despawns it or drops the last copy.
    pub(super) fn owned(raw: u32) -> Self {
        Self {
            raw,
            owned: true,
            alive: core::cell::Cell::new(true),
            epoch: 0,
        }
    }

    /// A handle the host lends for one event and reclaims after it.
    pub(super) fn lent(raw: u32, epoch: u32) -> Self {
        Self {
            raw,
            owned: false,
            alive: core::cell::Cell::new(true),
            epoch,
        }
    }

    /// Whether this still addresses what it was made for: not spent, and for
    /// a lent handle, still inside the event that lent it.
    pub(super) fn answers_in(&self, epoch: u32) -> bool {
        self.alive.get() && (self.owned || self.epoch == epoch)
    }

    /// The host's handle, whether or not it is still ours to use.
    pub(super) fn raw(&self) -> u32 {
        self.raw
    }

    /// Whether dropping this releases the host's resource: ours to release,
    /// and not already spent by a despawn.
    fn releases_on_drop(&self) -> bool {
        self.owned && self.alive.get()
    }

    /// Marks the handle spent. A second despawn, and every later use of any
    /// copy, is refused rather than releasing twice.
    pub(super) fn spend(&self) {
        self.alive.set(false);
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for Held {
    fn drop(&mut self) {
        if self.releases_on_drop() {
            drop(unsafe {
                crate::bindings::server::ironlark::host::entity::Handle::from_handle(self.raw())
            });
        }
    }
}

#[cfg(test)]
mod held_tests {
    use super::Held;

    #[test]
    fn a_spawned_handle_releases_once_and_only_once() {
        let held = Held::owned(7);
        assert!(held.releases_on_drop());
        held.spend();
        assert!(
            !held.releases_on_drop(),
            "a despawned handle must not be released again"
        );
    }

    #[test]
    fn a_handle_the_host_lent_is_never_released() {
        let held = Held::lent(7, 3);
        assert!(
            !held.releases_on_drop(),
            "releasing a lent handle would burn the host's own"
        );
        held.spend();
        assert!(!held.releases_on_drop());
    }

    #[test]
    fn a_spawned_handle_answers_in_any_event() {
        let held = Held::owned(7);
        assert!(held.answers_in(3));
        assert!(held.answers_in(4), "owning it does not tie it to an event");
        held.spend();
        assert!(!held.answers_in(3), "a despawned entity answers nowhere");
    }

    #[test]
    fn a_lent_handle_answers_only_inside_its_own_event() {
        let held = Held::lent(7, 3);
        assert!(held.answers_in(3));
        assert!(
            !held.answers_in(4),
            "the host reclaimed it; the number now addresses whoever came next"
        );
    }

    #[test]
    fn copies_share_one_liveness() {
        let held = std::rc::Rc::new(Held::owned(7));
        let copy = std::rc::Rc::clone(&held);
        held.spend();
        assert!(
            !copy.releases_on_drop(),
            "a copy must see the despawn its co-owner performed"
        );
        assert_eq!(copy.raw(), 7, "a copy addresses the same host handle");
    }
}
