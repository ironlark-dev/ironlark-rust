//! Reading a mod's `protocol.proto`: the payload types and what they declare.
//!
//! One place compiles proto in this SDK, and it is here. The file is parsed by
//! [protox](https://docs.rs/protox), so no `protoc` and no C++ toolchain enters
//! the build of a mod or of this crate.
//!
//! A declaration sits on the payload's own schema. A message carrying
//! `option (ironlark.signal)` declares a signal; an `rpc` inside a `service`
//! declares a request, and the service's name says which half answers it.
//! Everything else in the file is a plain type.

mod declaration;
mod generate;
mod items;
mod options;
mod read;
mod schema;

use proc_macro2::TokenStream;
use read::Declarations;
use syn::LitStr;

/// A mod's protocol: what it declares, and the types that travel.
pub(crate) struct Protocol {
    pub(crate) declarations: Declarations,
    /// The prost types, one entry per schema read: this mod's own, then each
    /// one it borrows from.
    pub(crate) payloads: Vec<generate::Generated>,
}

/// The whole of `protocol!`: read the schema named by the one string it takes,
/// then mint the types and the specs.
pub fn expand(input: TokenStream) -> TokenStream {
    let schema: LitStr = match syn::parse2(input) {
        Ok(schema) => schema,
        Err(_) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "`protocol!` takes the path to the mod's schema as a string, as \
                 `ironlark::protocol!(\"../protocol.proto\")`",
            )
            .to_compile_error();
        }
    };
    let minted = read_protocol(&schema)
        .and_then(|protocol| items::expand(&schema, &protocol.declarations, protocol.payloads));
    match minted {
        Ok(tokens) => tokens,
        Err(refusal) => refusal.to_compile_error(),
    }
}

/// Reads the `protocol.proto` named by `lit`, resolved against the directory
/// holding the `Cargo.toml` of the crate being built — the same rule the
/// manifest path follows, which is what lets one file serve both halves.
///
/// One compile answers both halves of the job: the descriptor pool it leaves
/// behind carries the custom options the declaration table is read from, and
/// the file descriptor in it is what prost generates the types from.
pub(crate) fn read_protocol(lit: &LitStr) -> syn::Result<Protocol> {
    let directory = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(lit.span(), "CARGO_MANIFEST_DIR is unset; build with cargo")
    })?;
    let path = std::path::Path::new(&directory).join(lit.value());

    let compiler = schema::compile(&path).map_err(|cause| refuse(lit, cause))?;
    let pool = compiler.descriptor_pool();
    let file = compiler
        .files()
        .find(|file| !file.is_import())
        .and_then(|file| pool.get_file_by_name(file.name()))
        .ok_or_else(|| refuse(lit, format!("{} compiled to nothing", path.display())))?;

    let declarations = read::declarations(&pool, &file).map_err(|cause| refuse(lit, cause))?;
    let payloads = generate::payloads(&declarations.schemas).map_err(|cause| refuse(lit, cause))?;
    Ok(Protocol {
        declarations,
        payloads,
    })
}

/// A compile-time refusal reads as an error on the path the author wrote, which
/// is the one span a proto fault has in Rust source.
fn refuse(lit: &LitStr, cause: impl core::fmt::Display) -> syn::Error {
    syn::Error::new(lit.span(), cause.to_string())
}

#[cfg(test)]
mod tests;
