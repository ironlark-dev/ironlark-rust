//! How the host is to carry a signal: the delivery axes a declaration states.

/// The delivery axes a signal declares, beside its audience.
///
/// Both axes have a zero value that is the real default rather than a missing
/// answer, so a declaration that states neither is the ordinary must-arrive,
/// in-raise-order signal and writes nothing. The axes name the DATA's nature,
/// never a transport: what a raise rides on is the host's to choose, and it
/// changes when the transport does.
///
/// It reaches Rust as [`SignalSpec::TRANSIT`](super::SignalSpec::TRANSIT),
/// written by [`protocol!`](macro@crate::protocol) from
/// `option (ironlark.transit.keep)` and `option (ironlark.transit.order)`.
///
/// ```
/// use ironlark::protocol::{Keep, Order, SignalSpec, Transit};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // The hidden line is `ironlark::protocol!("../protocol.proto")` over the
/// // door schema. `message Positions` there carries
/// //
/// //     option (ironlark.signal) = CLIENTS;
/// //     option (ironlark.transit.keep) = NEWEST;
/// //
/// // and `message Latch` states no axis at all.
/// assert_eq!(
///     <protocol::Positions as SignalSpec>::TRANSIT,
///     Transit { keep: Keep::Newest, order: Order::InOrder },
/// );
///
/// // What a declaration writing no axis gets.
/// assert_eq!(
///     <protocol::Latch as SignalSpec>::TRANSIT,
///     Transit { keep: Keep::KeepAll, order: Order::InOrder },
/// );
/// ```
///
/// # Nothing refuses
///
/// It is a value read off a declaration. A declared axis the host does not
/// serve is refused when the mod loads, by name, rather than degrading quietly,
/// and each value's own page says what it is served with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Transit {
    /// What the host retains under pressure.
    pub keep: Keep,
    /// Whether raises of one name arrive in raise order.
    pub order: Order,
}

/// What the host retains under pressure.
///
/// The marked case is supersession and nothing wider: [`Newest`](Keep::Newest)
/// says a newer raise stands in for an older one, which is the shape of
/// high-rate state where a listener wants the current answer rather than every
/// step toward it. It does NOT say "loss is acceptable in general", so a value
/// for expendable one-shots stays purely additive.
///
/// ```
/// use ironlark::protocol::{Keep, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // `message Positions` writes `option (ironlark.transit.keep) = NEWEST;`.
/// assert_eq!(
///     <protocol::Positions as SignalSpec>::TRANSIT.keep,
///     Keep::Newest,
/// );
/// ```
///
/// # Nothing refuses
///
/// Reading it cannot fail, and the two refusals it leads to are elsewhere.
/// [`Newest`](Keep::Newest) is served only where the signal crosses the
/// network, so a declaration pairing it with an audience that stays on a bus is
/// refused by name when the mod loads: there is no supersession to perform on a
/// raise that is delivered in process. And a superseding raise rides a path
/// where one message cannot be split, so its size budget is the smaller of the
/// two and a payload over it is refused at the raise by
/// [`signal`](crate::server::signal), which says so on its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keep {
    /// Every raise is delivered. The zero value and the default.
    KeepAll,
    /// A newer raise supersedes an older; the loss is inside the word.
    Newest,
}

/// Whether raises of one name arrive in the order they were raised.
///
/// [`InOrder`](Order::InOrder) is the zero value and the default, per name.
/// [`Any`](Order::Any) is declared and not yet served: it is in the schema so
/// the axis is named from day one, and a mod declaring it is refused by name
/// when it loads rather than being handed ordering it did not ask for.
///
/// ```
/// use ironlark::protocol::{Order, SignalSpec};
/// # mod protocol { ironlark::protocol!("doctest/protocol.proto"); }
/// // No door signal writes the axis, so every one of them is in raise order.
/// assert_eq!(
///     <protocol::Latch as SignalSpec>::TRANSIT.order,
///     Order::InOrder,
/// );
/// ```
///
/// # Nothing refuses
///
/// Reading it cannot fail. [`Any`](Order::Any) refuses at load, in the host,
/// with the name of the declaration that wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Order {
    /// Raise order, per name.
    InOrder,
    /// Any order. Declared, not yet served.
    Any,
}
