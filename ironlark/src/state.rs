//! What a mod remembers between events: one macro declares a cell, and three
//! verbs reach what is in it.

#[cfg(target_arch = "wasm32")]
use core::cell::{Cell, UnsafeCell};

/// Declares what a mod remembers between events.
///
/// A mod is a WebAssembly component instantiated once for the session, so a
/// `static` cell is the natural home for what it remembers. This macro is the
/// only door to one. It writes the type once, takes any expression as the
/// starting value, and mints a [`State`] cell reached through
/// [`get`](State::get), [`set`](State::set) and [`update`](State::update). How
/// the cell is laid out belongs to this macro, so nothing a mod writes depends
/// on it.
///
/// One block declares as many cells as the mod wants, each with its own type
/// and starting expression. Doc comments and visibility go where they would on
/// any `static`.
///
/// The starting expression runs on the cell's first touch, and again after a
/// reload, never at load time. A cell no session ever reaches costs nothing,
/// and an expression that reads another cell is fine because nothing has run
/// yet when the mod loads.
///
/// A cell belongs to no realm. Both preludes carry this macro, so a server
/// half and a client half remember things the same way, and mods spell it in
/// full as below so the origin of the name stays visible.
///
/// # Nothing refuses
///
/// A cell is memory inside the guest. Nothing here crosses into the host, so
/// nothing here answers a [`Result`](crate::Result). The one way out is a
/// panic, and reaching it takes reentering one cell from inside its own
/// [`update`](State::update); [`State`] tells that story and what it costs.
///
/// ```
/// // The prelude mints SessionId. `state!` is spelled in full.
/// use ironlark::server::prelude::*;
///
/// ironlark::state! {
///     /// How many doors this mod has opened since it loaded.
///     static OPENED: u32 = 0;
///     /// The participants this mod still owes a greeting.
///     static PENDING: Vec<SessionId> = Vec::new();
/// }
///
/// // A number is copied out and put back. A list is worked on where it lies.
/// fn record_opening(player: SessionId) {
///     OPENED.set(OPENED.get() + 1);
///     PENDING.update(|pending| pending.push(player));
/// }
/// ```
#[macro_export]
macro_rules! state {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $name:ident : $ty:ty = $init:expr;
    )*) => {
        $(
            $(#[$attr])*
            $vis static $name: $crate::State<$ty> = {
                fn init() -> $ty {
                    $init
                }
                $crate::State::__declare(init)
            };
        )*
    };
}

/// Where a mod keeps what it remembers between events.
///
/// [`state!`](macro@crate::state) declares a cell; this is what it declares. Three
/// verbs reach the value inside. [`get`](State::get) copies it out and is
/// offered only where a copy is the whole of the value — a number, a flag, an
/// id, a [`SessionId`](crate::SessionId). [`set`](State::set) replaces it.
/// [`update`](State::update) lends it out in place for the body of a closure
/// and answers whatever that body answers, which is how a collection is
/// worked on without ever being copied. The compiler steers a value that
/// cannot be copied to `update` on its own.
///
/// A cell is not shared with anything. The guest is single-threaded and the
/// cell is reached from one mod's own handlers, so there is no lock here and
/// no cost for one.
///
/// ```
/// // The prelude mints SessionId. `state!` is spelled in full.
/// use ironlark::server::prelude::*;
///
/// // A cell holds whatever the mod's own types are; nothing here is special.
/// struct Opening {
///     player: SessionId,
///     times: u32,
/// }
///
/// ironlark::state! {
///     /// Whether the latch this mod owns is open.
///     static OPEN: bool = false;
///     /// How often each participant has opened it.
///     static OPENINGS: Vec<Opening> = Vec::new();
/// }
///
/// fn opened_by(player: SessionId) {
///     // A flag is copied out and put back; the copy is the whole value.
///     if !OPEN.get() {
///         OPEN.set(true);
///     }
///     // A list is edited where it lies. Nothing is copied out.
///     OPENINGS.update(|tally| match tally.iter_mut().find(|row| row.player == player) {
///         Some(row) => row.times += 1,
///         None => tally.push(Opening { player, times: 1 }),
///     });
/// }
/// ```
///
/// # Nothing refuses
///
/// None of the three verbs reaches the host, so none of them answers a
/// [`Result`](crate::Result). The only failure a cell has is the reentry panic
/// below.
///
/// # Never across an await
///
/// Reach a cell, finish with it, and only then await. The reason is that
/// handlers of one mod interleave: when a handler awaits the host, another
/// handler of the same mod runs before the first resumes. A value read before
/// an await and written back after it is written back from a picture of the
/// world that the second handler has already moved on from, and the second
/// handler's work is what gets lost.
///
/// [`update`](State::update) cannot span an await at all — its closure is
/// synchronous, which is what makes the access safe without a lock. `get` and
/// `set` are single acts that end before they return, so nothing is held. The
/// pattern is to copy out, await, and write back a change rather than a whole
/// value; merging the change with whatever else happened meanwhile is the
/// mod's own decision, because only the mod knows what its two handlers mean
/// to each other.
///
/// ```
/// use ironlark::server::prelude::*;
///
/// ironlark::state! {
///     /// The participants this mod still owes a greeting.
///     static PENDING: Vec<SessionId> = Vec::new();
/// }
///
/// async fn greet_everyone() {
///     // Copy out everything the awaits need. The cell is free from here.
///     let waiting = PENDING.update(|pending| pending.clone());
///     let mut greeted = Vec::with_capacity(waiting.len());
///     for player in waiting {
///         // ... await a verb here, which `update` could not have spanned ...
///         greeted.push(player);
///     }
///     // Another handler may have added to the list while this one awaited,
///     // so write back the change: drop what was greeted, keep the rest.
///     PENDING.update(|pending| pending.retain(|player| !greeted.contains(player)));
/// }
/// ```
///
/// Writing `PENDING.set(waiting)` at the end instead would throw away
/// everything the other handler added.
///
/// # How long a cell lasts
///
/// The host retires a mod whose guest traps or panics, whose call budget runs
/// out, or that falls so far behind that work it cannot drop is piling up. The
/// instance goes and its memory with it, so every cell in the mod is gone at
/// once. A reload after that starts a fresh instance, and every cell is built
/// again from its starting expression. A tally reloaded mid-session is empty,
/// not what it was. A mod retired too often is quarantined for the rest of the
/// session and is not reloaded at all.
///
/// Nothing here survives that, and nothing here is a save. What a mod wants to
/// keep beyond an instance it has to have sent somewhere.
///
/// # Reentry
///
/// Reaching one cell from inside its own [`update`](State::update) panics by
/// design rather than aliasing the value. Only that cell is closed for the
/// duration of the closure: `get`, `set` and `update` on any other cell are
/// fine, and nesting across different cells is the normal thing.
///
/// A panic is not cheap here. The host counts a trapping guest against the mod
/// and retires the instance, so reentering one cell costs the mod every cell
/// it has. The rule that keeps it away is the same one that keeps `update` off
/// an await: do the smallest thing inside the closure, call nothing unknown
/// from it, and leave.
pub struct State<T> {
    init: fn() -> T,
    #[cfg(target_arch = "wasm32")]
    slot: UnsafeCell<Option<T>>,
    #[cfg(target_arch = "wasm32")]
    in_flight: Cell<bool>,
}

// The wasm component is single-threaded; the cell is never actually shared.
#[cfg(target_arch = "wasm32")]
unsafe impl<T> Sync for State<T> {}

impl<T: 'static> State<T> {
    /// The constructor [`state!`](macro@crate::state) calls. Not a door a mod uses:
    /// the layout is the macro's business and grows without a mod changing.
    #[doc(hidden)]
    pub const fn __declare(init: fn() -> T) -> Self {
        Self {
            init,
            #[cfg(target_arch = "wasm32")]
            slot: UnsafeCell::new(None),
            #[cfg(target_arch = "wasm32")]
            in_flight: Cell::new(false),
        }
    }

    /// Copies the value out, building it on first use.
    ///
    /// Offered only where a copy is the whole of the value. Anything else has
    /// no `get`, and [`update`](State::update) is how it is reached, so the
    /// compiler decides this rather than the author remembering to.
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.enter(|slot| *slot.get_or_insert_with(self.init))
    }

    /// Puts a value in, dropping whatever was there.
    ///
    /// This is how a mod replaces what it remembers rather than editing it.
    /// The value that was there is dropped without being built first, so
    /// setting a cell nothing has touched yet costs nothing extra.
    pub fn set(&self, value: T) {
        self.enter(|slot| *slot = Some(value));
    }

    /// Lends the value out for the body of `f`, building it on first use, and
    /// answers whatever `f` answers.
    ///
    /// The closure is synchronous, so it cannot span an `await`. That is what
    /// makes the access safe without a lock, and it is why a handler finishes
    /// with a cell before it awaits anything.
    ///
    /// Reaching this same cell from inside `f` panics with the reason rather
    /// than aliasing the value, and a panicking guest costs the mod its whole
    /// instance. Other cells are untouched by this one being open.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.enter(|slot| f(slot.get_or_insert_with(self.init)))
    }

    /// The one door to the value. Every read and every write in this file goes
    /// through here, so the in-flight check has nowhere to leak past.
    fn enter<R>(&self, f: impl FnOnce(&mut Option<T>) -> R) -> R {
        #[cfg(target_arch = "wasm32")]
        {
            if self.in_flight.replace(true) {
                panic!("{REENTERED}");
            }
            // Exclusive: single thread, and the in-flight flag just rejected reentry.
            let out = f(unsafe { &mut *self.slot.get() });
            self.in_flight.set(false);
            out
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            native::enter(self as *const Self as usize, f)
        }
    }
}

const REENTERED: &str =
    "State cell reentered from inside its own `update`; call nothing unknown from the closure";

// Native `cargo test` semantics: each test thread gets its own value, stored
// per (thread, cell address) so one `static` cell never races across tests.
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::any::Any;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SLOTS: RefCell<HashMap<usize, Box<dyn Any>>> = RefCell::new(HashMap::new());
    }

    // One map write claims the cell and hands back what was in it, so a whole
    // access is two lookups rather than three.
    pub fn enter<T: 'static, R>(key: usize, f: impl FnOnce(&mut Option<T>) -> R) -> R {
        let held = SLOTS.with(|slots| {
            slots
                .borrow_mut()
                .insert(key, Box::new(Marked::<T>::InFlight))
        });
        let mut value = match held {
            Some(boxed) => match boxed.downcast::<Marked<T>>() {
                Ok(marked) => match *marked {
                    Marked::Slot(value) => value,
                    Marked::InFlight => panic!("{}", super::REENTERED),
                },
                Err(_) => panic!("State cell holds a different type; two cells share an address"),
            },
            None => None,
        };
        let out = f(&mut value);
        SLOTS.with(|slots| {
            slots
                .borrow_mut()
                .insert(key, Box::new(Marked::Slot(value)))
        });
        out
    }

    enum Marked<T> {
        Slot(Option<T>),
        InFlight,
    }
}

#[cfg(test)]
mod tests {
    crate::state! {
        static COUNTER: u32 = 7;
        static OTHER: Vec<u8> = Vec::new();
    }

    #[test]
    fn initializes_lazily_and_persists_mutation() {
        crate::state! {
            static CELL: u32 = 7;
        }
        assert_eq!(CELL.get(), 7);
        CELL.update(|held| *held += 1);
        assert_eq!(CELL.get(), 8);
    }

    #[test]
    fn set_replaces_without_building_first() {
        crate::state! {
            static CELL: Vec<u8> = panic!("a set cell must never build its starting value");
        }
        CELL.set(vec![1, 2, 3]);
        assert_eq!(CELL.update(|held| held.len()), 3);
    }

    #[test]
    fn nesting_across_different_cells_works() {
        let sum = COUNTER.update(|count| OTHER.update(|other| *count as usize + other.len()));
        assert_eq!(sum, 7);
    }

    #[test]
    fn any_expression_starts_a_cell() {
        crate::state! {
            static CELL: Vec<u8> = vec![4; 2];
        }
        assert_eq!(CELL.update(|held| held.clone()), vec![4, 4]);
    }

    #[test]
    #[should_panic(expected = "reentered")]
    fn reentry_on_the_same_cell_panics() {
        crate::state! {
            static CELL: u32 = 0;
        }
        CELL.update(|_| CELL.get());
    }
}
