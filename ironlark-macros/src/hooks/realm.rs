//! Which half an impl block is, and what that half's hooks may be.

use super::declared::{Half, Rule};
use crate::manifest::Manifest;
use proc_macro2::Ident;
use quote::format_ident;
use syn::ItemImpl;
use syn::spanned::Spanned;

/// The hooks the engine invokes on a server half. `init` is absent because it
/// is never declared, and the exports that arrive under a declared name are
/// absent because they are answered on the payload type instead.
const SERVER: &[&str] = &[
    "on_join",
    "on_leave",
    "on_tick",
    "on_interact",
    "on_contact",
];

/// The client half's, which is the step and nothing else.
const CLIENT: &[&str] = &["on_tick"];

/// The half a block implements.
#[derive(Clone, Copy)]
pub(crate) enum Realm {
    Server,
    Client,
}

impl Realm {
    /// Reads the half off the trait the block implements.
    pub(crate) fn of(block: &ItemImpl) -> syn::Result<Self> {
        let Some((negative, path, _)) = &block.trait_ else {
            return Err(syn::Error::new(
                block.self_ty.span(),
                "#[ironlark::hooks] goes on an impl of `ServerMod` or `ClientMod`, \
                 not on an inherent impl",
            ));
        };
        if let Some(negative) = negative {
            return Err(syn::Error::new(
                negative.span(),
                "a negative impl declares no hooks",
            ));
        }
        let Some(last) = path.segments.last() else {
            return Err(syn::Error::new(
                path.span(),
                "expected `ServerMod` or `ClientMod`",
            ));
        };
        match last.ident.to_string().as_str() {
            "ServerMod" => Ok(Self::Server),
            "ClientMod" => Ok(Self::Client),
            other => Err(syn::Error::new(
                last.ident.span(),
                format!(
                    "`{other}` has no hooks; this attribute goes on an impl of `ServerMod` or \
                     `ClientMod`"
                ),
            )),
        }
    }

    pub(crate) fn trait_name(self) -> &'static str {
        match self {
            Self::Server => "ServerMod",
            Self::Client => "ClientMod",
        }
    }

    pub(crate) fn engine_hooks(self) -> &'static [&'static str] {
        match self {
            Self::Server => SERVER,
            Self::Client => CLIENT,
        }
    }

    /// The `[declares.<half>]` section this half's hooks are listed under.
    pub(crate) fn section(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Client => "client",
        }
    }

    /// That section, as the manifest read it.
    pub(crate) fn half_of(self, manifest: &Manifest) -> Option<&Half> {
        match self {
            Self::Server => manifest.server.as_ref(),
            Self::Client => manifest.client.as_ref(),
        }
    }

    /// Whether this half is entered for a rule, which is what says the rule's
    /// dispatch export exists in its world.
    pub(crate) fn serves(self, rule: Rule) -> bool {
        matches!(
            (self, rule),
            (Self::Client, Rule::Input) | (_, Rule::Engine)
        )
    }

    /// The constant holding the manifest's text, so an edit to the hook list
    /// re-runs this half's check.
    pub(crate) fn tracked(self) -> Ident {
        match self {
            Self::Server => format_ident!("__IRONLARK_SERVER_HOOKS_TRACKED"),
            Self::Client => format_ident!("__IRONLARK_CLIENT_HOOKS_TRACKED"),
        }
    }
}

/// A list of names as a refusal spells one.
pub(crate) fn named<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut said = String::new();
    for name in names {
        if !said.is_empty() {
            said.push_str(", ");
        }
        said.push('`');
        said.push_str(name);
        said.push('`');
    }
    if said.is_empty() {
        said.push_str("none");
    }
    said
}
