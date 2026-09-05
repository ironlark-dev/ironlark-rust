//! The platform schema and the compiler that reads a mod's `protocol.proto`
//! against it.
//!
//! `ironlark/options.proto` ships inside this crate and is served from memory,
//! so `import "ironlark/options.proto"` resolves with nothing on the author's
//! include path and no author copy can stand in for it.

use protox::Compiler;
use protox::file::{
    ChainFileResolver, File, FileResolver, GoogleFileResolver, IncludeFileResolver,
};
use std::path::Path;

/// The import path a mod's `protocol.proto` writes.
pub(crate) const OPTIONS: &str = "ironlark/options.proto";

/// The platform schema's own text, compiled from this crate rather than from
/// the filesystem the mod is built on.
const SOURCE: &str = include_str!("../../proto/ironlark/options.proto");

/// Serves the platform schema and nothing else.
struct Platform;

impl FileResolver for Platform {
    fn open_file(&self, name: &str) -> Result<File, protox::Error> {
        if name == OPTIONS {
            File::from_source(name, SOURCE)
        } else {
            Err(protox::Error::file_not_found(name))
        }
    }
}

/// Compiles `protocol.proto` at `path` with the platform schema and the
/// well-known types available, and the workshop root as an include root.
///
/// A mod is installed at `<workshop>/<author>/<mod>`, so the workshop root is
/// two directories above the file and a borrower names another mod's schema by
/// that mod's install path, `import "<author>/<mod>/protocol.proto"`. Opening
/// this file through the same root is what names it
/// `<author>/<mod>/protocol.proto` rather than by where it happens to sit on
/// the machine that built it, and that name is the identity the shipped
/// descriptor carries and the load-time source check compares.
///
/// The own directory stays an include root behind the workshop, so a mod may
/// still split its own schema across files beside it.
pub(crate) fn compile(path: &Path) -> Result<Compiler, protox::Error> {
    // A half's crate sits under the mod, so the written path holds `..` and
    // the ancestors are only the install path once it is settled.
    let settled = match path.canonicalize() {
        Ok(settled) => settled,
        // Not there at all: `open_file` below is what reports that, with the
        // path in the message.
        Err(_) => path.to_owned(),
    };

    let mut resolver = ChainFileResolver::new();
    resolver.add(Platform);
    resolver.add(GoogleFileResolver::new());
    let workshop = settled
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent);
    if let Some(workshop) = workshop {
        resolver.add(IncludeFileResolver::new(workshop.to_owned()));
    }
    if let Some(directory) = settled.parent() {
        resolver.add(IncludeFileResolver::new(directory.to_owned()));
    }

    let mut compiler = Compiler::with_file_resolver(resolver);
    compiler.include_source_info(true).include_imports(true);
    compiler.open_file(&settled)?;
    Ok(compiler)
}

/// The platform schema on its own, for the test that checks the Rust mirrors of
/// its enums against it.
#[cfg(test)]
pub(crate) fn platform_pool() -> Result<protox::prost_reflect::DescriptorPool, protox::Error> {
    let mut compiler = Compiler::with_file_resolver({
        let mut resolver = ChainFileResolver::new();
        resolver.add(Platform);
        resolver.add(GoogleFileResolver::new());
        resolver
    });
    compiler.open_file(OPTIONS)?;
    Ok(compiler.descriptor_pool())
}
