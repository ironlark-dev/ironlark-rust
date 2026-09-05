//! Filling the declaration table from a compiled `protocol.proto`.
//!
//! Only optioned messages and `service` blocks are read. A message carrying no
//! option is a plain type: helpers and imports declare nothing.

use crate::protocol::declaration::{
    DeclaredName, NotBorrowable, NotMappable, NotNameable, Origin, Owner, Package, PayloadType,
    Realm, Request, Signal, WrongPackage,
};
use crate::protocol::options::{Audience, Keep, Order, Transit, Unknown};
use crate::protocol::schema;
use core::fmt;
use proc_macro2::Ident;
use protox::prost_reflect::{
    DescriptorPool, DynamicMessage, ExtensionDescriptor, FileDescriptor, MessageDescriptor,
};

/// The extension a message writes to become a signal.
const SIGNAL: &str = "ironlark.signal";
/// One transit axis: what the host retains under pressure.
const KEEP: &str = "ironlark.transit.keep";
/// The other: whether raises of one name arrive in raise order.
const ORDER: &str = "ironlark.transit.order";

/// What a mod declares, kind by kind, in the order the file writes it.
pub(crate) struct Declarations {
    pub(crate) signals: Vec<Signal>,
    pub(crate) requests: Vec<Request>,
    /// Every schema whose types this expansion generates: the mod's own first,
    /// then each mod whose schema it imports.
    pub(crate) schemas: Vec<Schema>,
}

/// One schema, and the module its types land in.
pub(crate) struct Schema {
    pub(crate) file: FileDescriptor,
    pub(crate) package: Package,
}

/// Reads `file`'s declarations and the ones it borrows, with `pool` supplying
/// the platform extensions.
///
/// A mod borrows a name by importing the schema of the mod that owns it, so an
/// imported mod schema is read for its signals too. Only the mod's own file is
/// read for requests: borrowing an answer surface is not ruled, so an imported
/// `service` mints nothing.
///
/// The own package is checked and not kept beyond its module name: the host
/// verifies the shipped descriptor's package against the install path, so
/// nothing generated here restates a mod's own id.
pub(crate) fn declarations(
    pool: &DescriptorPool,
    file: &FileDescriptor,
) -> Result<Declarations, Unreadable> {
    let extensions = Extensions::of(pool)?;
    let own = Package::parse(file.package_name())?;

    let mut signals = signals_of(&extensions, file, own.module(), None)?;
    let requests = requests_of(file, own.module())?;
    // Only the own file can hold both kinds, and a borrowed name reaches its
    // own module, so this is the whole of what can reach one Rust name twice.
    collision(&signals, &requests)?;

    let mut schemas = vec![Schema {
        file: file.clone(),
        package: own,
    }];
    for imported in file.dependencies() {
        if !imported.package_name().starts_with(Package::PREFIX) {
            continue;
        }
        let package = Package::parse(imported.package_name())?;
        let owner = package.owner()?;
        signals.extend(signals_of(
            &extensions,
            &imported,
            package.module(),
            Some(owner),
        )?);
        schemas.push(Schema {
            file: imported,
            package,
        });
    }

    Ok(Declarations {
        signals,
        requests,
        schemas,
    })
}

/// The signals one schema declares. A message with no `(ironlark.signal)`
/// declares nothing, and a schema that imports no platform file can hold no
/// such option at all.
fn signals_of(
    extensions: &Option<Extensions>,
    file: &FileDescriptor,
    module: &Ident,
    owner: Option<Owner>,
) -> Result<Vec<Signal>, Unreadable> {
    let Some(extensions) = extensions else {
        return Ok(Vec::new());
    };
    let mut signals = Vec::new();
    for message in file.messages() {
        let origin = Origin {
            module: module.clone(),
            owner: owner.clone(),
        };
        if let Some(signal) = extensions.signal_of(&message, origin)? {
            signals.push(signal);
        }
    }
    Ok(signals)
}

/// The requests one schema declares, from its `service` blocks.
fn requests_of(file: &FileDescriptor, module: &Ident) -> Result<Vec<Request>, Unreadable> {
    let mut requests = Vec::new();
    for service in file.services() {
        let Some(realm) = Realm::from_service(service.name()) else {
            return Err(Unreadable::WrongService {
                service: service.name().to_owned(),
            });
        };
        for method in service.methods() {
            requests.push(Request {
                name: DeclaredName::from_proto(method.name())?,
                payload: PayloadType::from_proto(method.input().name())?,
                response: PayloadType::from_proto(method.output().name())?,
                realm,
                origin: Origin {
                    module: module.clone(),
                    owner: None,
                },
            });
        }
    }
    Ok(requests)
}

/// The platform extensions, looked up once per compile rather than once per
/// message.
struct Extensions {
    signal: Axis,
    keep: Axis,
    order: Axis,
}

/// One extension and the name it was found under, so a value that does not fit
/// the mirror says which option it came from.
struct Axis {
    extension: ExtensionDescriptor,
    name: &'static str,
}

impl Axis {
    /// The option's value as the enum number the schema gives it, defaulted by
    /// proto to the zero value when the message does not write it.
    fn number(&self, options: &DynamicMessage) -> Result<i32, Unreadable> {
        options
            .get_extension(&self.extension)
            .as_enum_number()
            .ok_or(Unreadable::SchemaChanged { option: self.name })
    }
}

impl Extensions {
    /// None where the schema does not import the platform file: no message can
    /// then carry the option, so there is no signal to look for. A file that
    /// does import it and still lacks an extension is this SDK's own drift.
    fn of(pool: &DescriptorPool) -> Result<Option<Self>, Unreadable> {
        if pool.get_file_by_name(schema::OPTIONS).is_none() {
            return Ok(None);
        }
        let find = |name: &'static str| {
            pool.get_extension_by_name(name)
                .map(|extension| Axis { extension, name })
                .ok_or(Unreadable::SchemaChanged { option: name })
        };
        Ok(Some(Self {
            signal: find(SIGNAL)?,
            keep: find(KEEP)?,
            order: find(ORDER)?,
        }))
    }

    /// A message with no `(ironlark.signal)` declares nothing and yields none.
    fn signal_of(
        &self,
        message: &MessageDescriptor,
        origin: Origin,
    ) -> Result<Option<Signal>, Unreadable> {
        let options = message.options();
        if !options.has_extension(&self.signal.extension) {
            return Ok(None);
        }
        Ok(Some(Signal {
            name: DeclaredName::from_proto(message.name())?,
            payload: PayloadType::from_proto(message.name())?,
            audience: Audience::from_number(self.signal.number(&options)?)?,
            transit: Transit {
                keep: Keep::from_number(self.keep.number(&options)?)?,
                order: Order::from_number(self.order.number(&options)?)?,
            },
            origin,
        }))
    }
}

/// Two declarations reaching one declared name would have the host publish the
/// same name twice, so the file is refused instead.
fn collision(signals: &[Signal], requests: &[Request]) -> Result<(), Unreadable> {
    let mut named: Vec<&DeclaredName> = signals
        .iter()
        .map(|signal| &signal.name)
        .chain(requests.iter().map(|request| &request.name))
        .collect();
    named.sort_unstable();
    for pair in named.windows(2) {
        if pair[0] == pair[1] {
            return Err(Unreadable::Collision {
                name: pair[0].to_string(),
            });
        }
    }
    Ok(())
}

/// Why a compiled `protocol.proto` yields no declaration table.
pub(crate) enum Unreadable {
    /// A message or an `rpc` whose name reaches no declared name.
    NotMappable(NotMappable),
    /// A message whose name reaches no Rust type name.
    NotNameable(NotNameable),
    /// A package line that is not `mods.<author>_<mod>`.
    WrongPackage(WrongPackage),
    /// A `service` block naming neither half.
    WrongService { service: String },
    /// An imported package no owner id reads back from.
    NotBorrowable(NotBorrowable),
    /// An option value this SDK's schema does not hold.
    UnknownValue(Unknown),
    /// Two declarations reaching one declared name.
    Collision { name: String },
    /// The platform schema no longer carries what this reader reads.
    SchemaChanged { option: &'static str },
}

impl From<NotMappable> for Unreadable {
    fn from(cause: NotMappable) -> Self {
        Self::NotMappable(cause)
    }
}

impl From<NotNameable> for Unreadable {
    fn from(cause: NotNameable) -> Self {
        Self::NotNameable(cause)
    }
}

impl From<NotBorrowable> for Unreadable {
    fn from(cause: NotBorrowable) -> Self {
        Self::NotBorrowable(cause)
    }
}

impl From<WrongPackage> for Unreadable {
    fn from(cause: WrongPackage) -> Self {
        Self::WrongPackage(cause)
    }
}

impl From<Unknown> for Unreadable {
    fn from(cause: Unknown) -> Self {
        Self::UnknownValue(cause)
    }
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMappable(cause) => cause.fmt(f),
            Self::NotNameable(cause) => cause.fmt(f),
            Self::WrongPackage(cause) => cause.fmt(f),
            Self::NotBorrowable(cause) => cause.fmt(f),
            Self::WrongService { service } => write!(
                f,
                "service `{service}` names no half; a service is `{}` or `{}`, and its name is \
                 what says which half answers the requests inside it",
                Realm::Server,
                Realm::Client
            ),
            Self::UnknownValue(cause) => cause.fmt(f),
            Self::Collision { name } => write!(
                f,
                "two declarations reach the declared name `{name}`; the host publishes a name \
                 once, so rename one of them"
            ),
            Self::SchemaChanged { option } => {
                write!(f, "the SDK's own {} carries no `{option}`", schema::OPTIONS)
            }
        }
    }
}
