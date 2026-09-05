//! Reading a mod's `mod.toml`: the names it ships and what each half declares.
//!
//! Communication is not here. A signal and a request live on the payload's own
//! schema, read by [`crate::protocol`]; what the manifest still carries is
//! identity, assets and the hooks each half answers to, because a hook's rule
//! and its default bindings are operator-and-player-facing configuration.

use crate::hooks::declared::Half;
use syn::LitStr;
use toml::de::{DeTable, DeValue};

/// A mod's `[declares]` block, as the macros read it.
pub(crate) struct Manifest {
    /// `[declares] sounds`, one name per sound the mod ships.
    pub(crate) sounds: Vec<String>,
    /// The `id` of each `[[declares.archetype]]`, in the order they are written.
    pub(crate) archetypes: Vec<String>,
    /// `[declares.server]`, absent when the half has no section.
    pub(crate) server: Option<Half>,
    /// `[declares.client]`, absent when the half has no section.
    pub(crate) client: Option<Half>,
}

/// The keys that carried communication before it moved onto the schema. Each
/// one names where its declarations went, because a manifest still holding one
/// was written against the previous grammar.
const MOVED: [(&str, &str); 3] = [
    (
        "channels",
        "a signal is a message in the mod's protocol.proto carrying \
         `option (ironlark.signal)`",
    ),
    (
        "methods",
        "a request is an `rpc` inside `service Server` in the mod's protocol.proto",
    ),
    (
        "actions",
        "an input is an author hook, `{ name = \"...\", default-bindings = [...] }` \
         under [declares.client]",
    ),
];

/// Reads the manifest named by `lit`, resolved against the directory holding
/// the `Cargo.toml` of the crate being built.
pub(crate) fn read_manifest(lit: &LitStr) -> syn::Result<Manifest> {
    let directory = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(lit.span(), "CARGO_MANIFEST_DIR is unset; build with cargo")
    })?;
    let path = std::path::Path::new(&directory).join(lit.value());
    let text = std::fs::read_to_string(&path).map_err(|cause| {
        syn::Error::new(
            lit.span(),
            format!("cannot read {}: {cause}", path.display()),
        )
    })?;
    let table = DeTable::parse(&text).map_err(|cause| {
        syn::Error::new(
            lit.span(),
            format!("{} is not valid TOML: {cause}", path.display()),
        )
    })?;

    let declares = match key(table.get_ref(), "declares") {
        Some(DeValue::Table(declares)) => Some(declares),
        _ => None,
    };

    if let Some(declares) = declares {
        for (dead, went) in MOVED {
            if key(declares, dead).is_some() {
                return Err(syn::Error::new(
                    lit.span(),
                    format!(
                        "`{dead}` no longer declares anything and {} still writes it: {went}",
                        lit.value()
                    ),
                ));
            }
        }
    }

    Ok(Manifest {
        sounds: declares
            .and_then(|declares| strings(key(declares, "sounds")))
            .unwrap_or_default(),
        archetypes: declares
            .and_then(|declares| archetype_ids(key(declares, "archetype")))
            .unwrap_or_default(),
        server: half(lit, declares, "server")?,
        client: half(lit, declares, "client")?,
    })
}

/// One half's section. Absent means the half does not exist: a section's
/// existence is the statement that the half and its `init` do.
fn half(lit: &LitStr, declares: Option<&DeTable<'_>>, name: &str) -> syn::Result<Option<Half>> {
    let Some(declares) = declares else {
        return Ok(None);
    };
    let Some(DeValue::Table(section)) = key(declares, name) else {
        return Ok(None);
    };
    for (written, _) in section.iter() {
        if written.get_ref() == "hooks" {
            continue;
        }
        return Err(syn::Error::new(
            lit.span(),
            format!(
                "`{}` sits under [declares.{name}] and belongs in [declares]; TOML puts every \
                 key after a section header inside it, so move it above the sections",
                written.get_ref()
            ),
        ));
    }
    Half::read(lit, key(section, "hooks"), name).map(Some)
}

/// One key of a TOML table. The parse keeps its spans, so a key is a borrowed
/// pair rather than a lookup in a map.
pub(crate) fn key<'a>(table: &'a DeTable<'a>, name: &str) -> Option<&'a DeValue<'a>> {
    table
        .iter()
        .find(|(written, _)| written.get_ref() == name)
        .map(|(_, value)| value.get_ref())
}

/// The `id` of every `[[declares.archetype]]`. An entry naming none declares no
/// archetype the host can key, so it mints no item either; the manifest check
/// that runs at load is what reports it.
fn archetype_ids(value: Option<&DeValue<'_>>) -> Option<Vec<String>> {
    let DeValue::Array(entries) = value? else {
        return None;
    };
    Some(
        entries
            .iter()
            .filter_map(|entry| match entry.get_ref() {
                DeValue::Table(fields) => match key(fields, "id")? {
                    DeValue::String(id) => Some(id.to_string()),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
    )
}

/// An array of strings, or nothing where the value is neither.
fn strings(value: Option<&DeValue<'_>>) -> Option<Vec<String>> {
    let DeValue::Array(items) = value? else {
        return None;
    };
    Some(
        items
            .iter()
            .filter_map(|item| match item.get_ref() {
                DeValue::String(held) => Some(held.to_string()),
                _ => None,
            })
            .collect(),
    )
}
