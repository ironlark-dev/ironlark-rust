//! The Rust side of the enums in `ironlark/options.proto`. A test in this
//! module reads the schema back and refuses a mirror that has drifted from it.
//!
//! Each mirror also carries the path a mod's generated code names it by, so one
//! value has one Rust spelling and the emitter never restates it.

use core::fmt;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

/// Who hears a signal, from `option (ironlark.signal)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Audience {
    /// Stays on the server realm's bus.
    ServerMods,
    /// Stays on this machine's client bus.
    ClientMods,
    /// Crosses the network to every client realm.
    Clients,
}

/// What the host retains under pressure, from `option (ironlark.transit.keep)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Keep {
    /// Every raise is delivered.
    KeepAll,
    /// A newer raise supersedes an older.
    Newest,
}

/// Whether raises of one name arrive in raise order, from
/// `option (ironlark.transit.order)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Order {
    /// Raise order per name.
    InOrder,
    /// Any order. Declared, not yet served.
    Any,
}

/// The delivery axes a signal declares, namespaced by `transit` in the schema.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Transit {
    pub(crate) keep: Keep,
    pub(crate) order: Order,
}

/// An enum value the schema holds and this mirror does not.
pub(crate) struct Unknown {
    /// The enum's name in `ironlark/options.proto`.
    pub(crate) schema: &'static str,
    /// The value read from the option.
    pub(crate) number: i32,
}

impl fmt::Display for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} value {} is in ironlark/options.proto and not in this SDK; \
             build the mod against the SDK the schema came from",
            self.schema, self.number
        )
    }
}

impl Audience {
    pub(crate) const SCHEMA: &'static str = "ironlark.Audience";

    pub(crate) fn from_number(number: i32) -> Result<Self, Unknown> {
        match number {
            0 => Ok(Self::ServerMods),
            1 => Ok(Self::ClientMods),
            2 => Ok(Self::Clients),
            _ => Err(Unknown {
                schema: Self::SCHEMA,
                number,
            }),
        }
    }
}

impl Keep {
    pub(crate) const SCHEMA: &'static str = "ironlark.Keep";

    pub(crate) fn from_number(number: i32) -> Result<Self, Unknown> {
        match number {
            0 => Ok(Self::KeepAll),
            1 => Ok(Self::Newest),
            _ => Err(Unknown {
                schema: Self::SCHEMA,
                number,
            }),
        }
    }
}

impl Order {
    pub(crate) const SCHEMA: &'static str = "ironlark.Order";

    pub(crate) fn from_number(number: i32) -> Result<Self, Unknown> {
        match number {
            0 => Ok(Self::InOrder),
            1 => Ok(Self::Any),
            _ => Err(Unknown {
                schema: Self::SCHEMA,
                number,
            }),
        }
    }
}

/// The audience is a type rather than a value on the spec, because it is what
/// the verbs are gated on: a narrowed raise takes a crossing signal only, and a
/// bus a half does not stand on is a signal it cannot raise. A bound reads a
/// type and cannot read a constant.
impl ToTokens for Audience {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::ServerMods => quote!(::ironlark::protocol::ServerMods),
            Self::ClientMods => quote!(::ironlark::protocol::ClientMods),
            Self::Clients => quote!(::ironlark::protocol::Clients),
        });
    }
}

impl ToTokens for Keep {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::KeepAll => quote!(::ironlark::protocol::Keep::KeepAll),
            Self::Newest => quote!(::ironlark::protocol::Keep::Newest),
        });
    }
}

impl ToTokens for Order {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::InOrder => quote!(::ironlark::protocol::Order::InOrder),
            Self::Any => quote!(::ironlark::protocol::Order::Any),
        });
    }
}

impl ToTokens for Transit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (keep, order) = (self.keep, self.order);
        tokens.extend(quote! {
            ::ironlark::protocol::Transit { keep: #keep, order: #order }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Audience, Keep, Order};
    use crate::protocol::schema;

    /// The mirrors above are hand-written, so the schema is read back and every
    /// value it holds has to map. A value added to the proto and not here fails
    /// on the count; a renumbered value fails on the mapping.
    #[test]
    fn the_mirrors_match_the_schema() {
        let pool = schema::platform_pool().expect("the platform schema compiles");
        let file = pool
            .get_file_by_name(schema::OPTIONS)
            .expect("the platform schema is in the pool");

        let mut seen = 0;
        for declared in file.enums() {
            for value in declared.values() {
                let number = value.number();
                let mirrored = match declared.full_name() {
                    Audience::SCHEMA => Audience::from_number(number).is_ok(),
                    Keep::SCHEMA => Keep::from_number(number).is_ok(),
                    Order::SCHEMA => Order::from_number(number).is_ok(),
                    other => panic!("{other} is in the schema and has no mirror in this module"),
                };
                assert!(mirrored, "{} = {number} has no mirror", value.name());
                seen += 1;
            }
        }
        assert_eq!(seen, 7, "the schema's whole set of enum values was read");
    }

    #[test]
    fn a_value_the_schema_does_not_hold_refuses() {
        let Err(refusal) = Keep::from_number(9) else {
            panic!("9 is not a Keep value");
        };
        assert!(refusal.to_string().contains("ironlark.Keep value 9"));
    }
}
