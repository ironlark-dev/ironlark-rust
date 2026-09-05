//! Generating the payload types from a mod's own file and the ones it borrows.
//!
//! prost is driven in memory: no `protoc` runs, nothing is written to disk, and
//! the generated code names the prost re-exported from `ironlark` so a mod's
//! `Cargo.toml` names no encoder.
//!
//! One call generates every schema at once, because a type in one package
//! referring to a type in another is written as a path relative to the
//! packages, and prost resolves that path from the package names rather than
//! from where the source is put. So each schema's types land in a module named
//! for its package's own segment, all of them siblings, which is the one layout
//! those paths are correct in.

use super::read::Schema;
use core::fmt;
use proc_macro2::{Ident, TokenStream};
use prost_build::{Config, Module};
use std::str::FromStr;

/// Where the generated code reaches prost and the well-known types. Both are
/// re-exports, so the only crate a mod depends on is `ironlark`.
const PROST: &str = "ironlark::prost";
const PROST_TYPES: &str = "ironlark::prost_types";

/// One schema's generated types, in the module they land in.
pub(crate) struct Generated {
    pub(crate) module: Ident,
    pub(crate) types: TokenStream,
}

/// Generates the payload types for every schema in `schemas` and nothing else.
///
/// The platform schema and the well-known types are not among them: neither
/// mints a type a mod names.
pub(crate) fn payloads(schemas: &[Schema]) -> Result<Vec<Generated>, Ungenerated> {
    let requested: Vec<(Module, _)> = schemas
        .iter()
        .map(|schema| {
            (
                Module::from_protobuf_package_name(schema.file.package_name()),
                schema.file.file_descriptor_proto().clone(),
            )
        })
        .collect();

    let mut config = Config::new();
    config.prost_path(PROST).prost_types_path(PROST_TYPES);

    let generated = config
        .generate(requested.clone())
        .map_err(|cause| Ungenerated::Prost {
            cause: cause.to_string(),
        })?;

    let mut out = Vec::with_capacity(schemas.len());
    for (schema, (module, _)) in schemas.iter().zip(&requested) {
        // A schema holding no message and no service generates no source, and it
        // still gets its module: one entry per schema in the order read is what
        // makes the first entry this mod's own rather than an owner's.
        let types = match generated.get(module) {
            Some(source) => {
                TokenStream::from_str(source).map_err(|cause| Ungenerated::NotRust {
                    cause: cause.to_string(),
                })?
            }
            None => TokenStream::new(),
        };
        out.push(Generated {
            module: schema.package.module().clone(),
            types,
        });
    }
    Ok(out)
}

/// Why a compiled `protocol.proto` yields no payload types.
pub(crate) enum Ungenerated {
    /// prost refused the descriptor.
    Prost { cause: String },
    /// prost produced something that is not Rust.
    NotRust { cause: String },
}

impl fmt::Display for Ungenerated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prost { cause } => write!(f, "the payload types were not generated: {cause}"),
            Self::NotRust { cause } => {
                write!(f, "the generated payload types are not Rust: {cause}")
            }
        }
    }
}
