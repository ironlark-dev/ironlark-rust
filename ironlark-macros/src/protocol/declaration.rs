//! What a mod declares, as one record per declaration.
//!
//! Kind is the list a declaration sits in rather than a field beside it: a
//! signal carries an audience and a transit, a request carries the type it
//! answers with, and neither set means anything on the other.

use crate::protocol::options::{Audience, Transit};
use core::fmt;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;

/// An announcement. Whoever subscribed to its name hears it.
pub(crate) struct Signal {
    pub(crate) name: DeclaredName,
    pub(crate) payload: PayloadType,
    pub(crate) audience: Audience,
    pub(crate) transit: Transit,
    pub(crate) origin: Origin,
}

/// Whose declaration it is, and the module its payload type lands in.
///
/// A borrowed declaration differs from an own one by who owns it, so that is a
/// field here rather than a second kind of record.
pub(crate) struct Origin {
    /// Named for the schema's package, because that is the name the generated
    /// code reaches a type in another package by.
    pub(crate) module: Ident,
    /// Absent where the declaration is this mod's own.
    pub(crate) owner: Option<Owner>,
}

/// Another mod's id, as the identity grammar spells it.
#[derive(Clone)]
pub(crate) struct Owner(String);

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An addressed act: one awaited answer from the half whose service declares it.
pub(crate) struct Request {
    pub(crate) name: DeclaredName,
    /// The `rpc`'s input, which the calling half hands the verb.
    pub(crate) payload: PayloadType,
    /// The `rpc`'s output.
    pub(crate) response: PayloadType,
    /// Which half answers, from the service's name.
    pub(crate) realm: Realm,
    pub(crate) origin: Origin,
}

/// The half a service block names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Realm {
    Server,
    Client,
}

impl Realm {
    pub(crate) fn from_service(name: &str) -> Option<Self> {
        match name {
            "Server" => Some(Self::Server),
            "Client" => Some(Self::Client),
            _ => None,
        }
    }
}

/// The service spelling, which is the one a proto file and a refusal both use.
impl fmt::Display for Realm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Server => "Server",
            Self::Client => "Client",
        })
    }
}

/// A declared name in the charset a manifest and a qualified id use: ASCII
/// lowercase letters, digits and `-`.
///
/// It is the proto name mapped, never a second spelling an author writes: the
/// mapping is the exact inverse of the one that reaches a proto name back from
/// Rust, so a round trip through both changes nothing.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DeclaredName(String);

impl DeclaredName {
    /// Maps an UpperCamel proto name, splitting a word at each capital.
    ///
    /// Refuses anything the inverse mapping would not produce, because a name
    /// that does not round-trip is a name the two files disagree about.
    pub(crate) fn from_proto(proto: &str) -> Result<Self, NotMappable> {
        mapped(proto).map(Self).ok_or_else(|| NotMappable {
            proto: proto.to_owned(),
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeclaredName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One pass, one allocation: a capital opens a word, and anything the inverse
/// mapping cannot have produced yields nothing.
fn mapped(proto: &str) -> Option<String> {
    let mut name = String::with_capacity(proto.len() + 2);
    for (position, character) in proto.char_indices() {
        match character {
            capital if capital.is_ascii_uppercase() => {
                if position > 0 {
                    name.push('-');
                }
                name.push(capital.to_ascii_lowercase());
            }
            inner if position > 0 && (inner.is_ascii_lowercase() || inner.is_ascii_digit()) => {
                name.push(inner);
            }
            _ => return None,
        }
    }
    (!name.is_empty()).then_some(name)
}

/// A proto name no declared name maps onto.
pub(crate) struct NotMappable {
    pub(crate) proto: String,
}

impl fmt::Display for NotMappable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` declares, so its name has to reach a declared name: an initial capital, then \
             ASCII letters and digits, a capital opening each further word, as `PlayerPositions` \
             reaches `player-positions`",
            self.proto
        )
    }
}

/// The name prost mints for a proto name, by prost's own two steps: upper
/// camel, then the sanitizing that keeps a Rust keyword nameable.
///
/// One mapping serves the generated payload type and the marker item beside
/// it, so the two cannot drift apart on a name either has to sanitize.
fn item_name(proto: &str) -> Option<Ident> {
    let camel = proto.to_upper_camel_case();
    let reached = match camel.as_str() {
        "" => return None,
        "Self" => "Self_".to_owned(),
        digit if digit.starts_with(|c: char| c.is_numeric()) => format!("_{digit}"),
        _ => camel,
    };
    Some(Ident::new(&reached, Span::call_site()))
}

/// A generated payload type, as the name it reaches Rust under.
pub(crate) struct PayloadType(Ident);

impl PayloadType {
    pub(crate) fn from_proto(proto: &str) -> Result<Self, NotNameable> {
        item_name(proto).map(Self).ok_or_else(|| NotNameable {
            proto: proto.to_owned(),
        })
    }
}

impl fmt::Display for PayloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToTokens for PayloadType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

/// A message name no Rust type name maps onto.
pub(crate) struct NotNameable {
    pub(crate) proto: String,
}

impl fmt::Display for NotNameable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "message `{}` reaches no Rust type name and cannot be generated; give it a name \
             holding letters and digits",
            self.proto
        )
    }
}

/// A mod's proto package: `mods.` and one flat segment.
///
/// Flat under the prefix because proto resolves names from the innermost scope
/// outward: with any `mods.<author>.*` package in a compile set,
/// `(ironlark.signal)` written inside another `mods.*` package resolves to a
/// nonexistent `mods.<author>.ironlark.signal`.
pub(crate) struct Package {
    segment: String,
    /// The module the schema's types land in. Minted here because the
    /// generated code reaches another package by exactly this name, so a
    /// segment no module name maps onto is refused before anything is
    /// generated against it.
    module: Ident,
}

impl Package {
    /// The prefix every mod's package sits directly under.
    pub(crate) const PREFIX: &'static str = "mods.";

    pub(crate) fn parse(package: &str) -> Result<Self, WrongPackage> {
        let wrong = || WrongPackage {
            package: package.to_owned(),
        };
        let segment = package.strip_prefix(Self::PREFIX).ok_or_else(wrong)?;
        let legible = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
        if !segment.starts_with(|c: char| c.is_ascii_lowercase()) || !segment.chars().all(legible) {
            return Err(wrong());
        }
        let module = syn::parse_str::<Ident>(segment).map_err(|_| wrong())?;
        Ok(Self {
            segment: segment.to_owned(),
            module,
        })
    }

    pub(crate) fn module(&self) -> &Ident {
        &self.module
    }

    /// The mod id the segment maps back to, for a name this mod borrows.
    ///
    /// The mapping into a package replaces the `-` of a manifest name with
    /// `_`, so reading it back is only unambiguous while the segment holds one
    /// `_`. A segment holding more is refused rather than split on a guess.
    pub(crate) fn owner(&self) -> Result<Owner, NotBorrowable> {
        let refuse = || NotBorrowable {
            package: format!("{}{}", Self::PREFIX, self.segment),
        };
        let (author, name) = self.segment.split_once('_').ok_or_else(refuse)?;
        if author.is_empty() || name.is_empty() || name.contains('_') {
            return Err(refuse());
        }
        Ok(Owner(format!("{author}:{name}")))
    }
}

/// A package an owner's id cannot be read back from.
pub(crate) struct NotBorrowable {
    pub(crate) package: String,
}

impl fmt::Display for NotBorrowable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "package {} is imported, so its declarations are borrowed and each one needs the \
             owner's `author:mod` id, which this package holds more than one `_` to read back \
             from; a borrowed schema's owner needs no `-` in its author or its name",
            self.package
        )
    }
}

/// A package line no mod id maps onto.
pub(crate) struct WrongPackage {
    pub(crate) package: String,
}

impl fmt::Display for WrongPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "package {} is not `mods.<author>_<mod>`; the segment after `mods.` is one flat \
             name, holding lowercase letters, digits and `_`, with the `-` of a manifest name \
             written as `_`",
            self.package
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{DeclaredName, Package, PayloadType, Realm};

    #[test]
    fn a_message_name_maps_to_the_manifest_charset() {
        for (proto, declared) in [
            ("Pressed", "pressed"),
            ("Positions", "positions"),
            ("PlayerPositions", "player-positions"),
            ("Advance", "advance"),
            ("Hit1", "hit1"),
        ] {
            let Ok(name) = DeclaredName::from_proto(proto) else {
                panic!("{proto} maps to {declared}");
            };
            assert_eq!(name.as_str(), declared);
        }
    }

    #[test]
    fn a_name_the_mapping_does_not_reach_refuses() {
        for proto in ["pressed", "player_positions", "1st", ""] {
            let Err(refusal) = DeclaredName::from_proto(proto) else {
                panic!("{proto} has no declared name");
            };
            assert_eq!(refusal.proto, proto);
        }
    }

    #[test]
    fn a_package_is_flat_under_the_prefix() {
        let Ok(package) = Package::parse("mods.ironlark_echo") else {
            panic!("the flat form is the ruled one");
        };
        assert_eq!(package.module().to_string(), "ironlark_echo");
        let Ok(owner) = package.owner() else {
            panic!("one `_` reads back as `author:mod`");
        };
        assert_eq!(owner.to_string(), "ironlark:echo");

        for wrong in [
            "mods.ironlark.echo",
            "ironlark_echo",
            "mods.",
            "mods.Echo",
            "mods.9echo",
        ] {
            let Err(refusal) = Package::parse(wrong) else {
                panic!("{wrong} is not a mod package");
            };
            assert_eq!(refusal.package, wrong);
        }
    }

    /// The `-` of a manifest name becomes `_` in a package, so a segment
    /// holding more than one `_` has no one reading back.
    #[test]
    fn an_owner_no_id_reads_back_from_refuses() {
        for ambiguous in ["mods.acme_x_door", "mods.echo"] {
            let Ok(package) = Package::parse(ambiguous) else {
                panic!("{ambiguous} is a legible package");
            };
            let Err(refusal) = package.owner() else {
                panic!("{ambiguous} has no one owner id");
            };
            assert_eq!(refusal.package, ambiguous);
        }
    }

    #[test]
    fn a_service_names_the_answering_half() {
        assert_eq!(Realm::from_service("Server"), Some(Realm::Server));
        assert_eq!(Realm::from_service("Client"), Some(Realm::Client));
        assert_eq!(Realm::from_service("Echo"), None);
    }

    #[test]
    fn a_payload_type_reaches_the_name_prost_mints() {
        for (proto, rust) in [
            ("Pressed", "Pressed"),
            ("PlayerPos", "PlayerPos"),
            ("player_pos", "PlayerPos"),
            ("Self", "Self_"),
        ] {
            let Ok(payload) = PayloadType::from_proto(proto) else {
                panic!("{proto} reaches {rust}");
            };
            assert_eq!(payload.to_string(), rust);
        }
    }
}
