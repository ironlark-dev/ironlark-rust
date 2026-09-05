//! `#[ironlark::hooks]`: binding a half's entry points to what its manifest
//! declares, and minting the dispatch the mod's own hooks arrive through.

pub(crate) mod declared;
mod lift;
mod realm;

use crate::manifest::read_manifest;
use declared::{Half, INIT};
use proc_macro2::TokenStream;
use quote::quote;
use realm::{Realm, named};
use syn::spanned::Spanned;
use syn::{ImplItem, ItemImpl, LitStr};

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let block: ItemImpl = match syn::parse2(item.clone()) {
        Ok(block) => block,
        Err(_) => {
            return syn::Error::new(
                item.span(),
                "#[ironlark::hooks] goes on an `impl ServerMod for ..` or \
                 `impl ClientMod for ..` block",
            )
            .to_compile_error();
        }
    };
    match rebuild(attr, &block) {
        Ok(tokens) => tokens,
        // The block is re-emitted so the half's type still implements its
        // trait and the refusal below is the only thing a reader has to read.
        Err(refusal) => {
            let refusal = refusal.to_compile_error();
            quote! { #refusal #block }
        }
    }
}

fn rebuild(attr: TokenStream, block: &ItemImpl) -> syn::Result<TokenStream> {
    let manifest = path(attr)?;
    let realm = Realm::of(block)?;
    let read = read_manifest(&manifest)?;

    let Some(half) = realm.half_of(&read) else {
        return Err(syn::Error::new(
            manifest.span(),
            format!(
                "{} has no [declares.{}] section, so this half is not declared to exist; the \
                 section is what states that the half and its `{INIT}` do",
                manifest.value(),
                realm.section()
            ),
        ));
    };

    engine_hooks(&manifest, realm, half, block)?;
    let split = lift::split(block, realm, half)?;
    let arms = lift::arms(realm, half, &split)?;

    let mut rebuilt = block.clone();
    rebuilt.items = split.trait_items;
    let dispatch = lift::input_dispatch(&arms);
    if !dispatch.is_empty() {
        rebuilt.items.push(syn::parse2::<ImplItem>(dispatch)?);
    }
    let inherent = lift::inherent(block, &split.lifted);
    let tracked = realm.tracked();

    Ok(quote! {
        // Re-reads the manifest on every build, so an edit to its hook list
        // re-runs the check above and renumbers the dispatch below.
        const #tracked: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #manifest));

        #rebuilt

        #inherent
    })
}

fn path(attr: TokenStream) -> syn::Result<LitStr> {
    let span = if attr.is_empty() {
        proc_macro2::Span::call_site()
    } else {
        attr.span()
    };
    syn::parse2(attr).map_err(|_| {
        syn::Error::new(
            span,
            "#[ironlark::hooks] takes the manifest path as a string, \
             as `#[ironlark::hooks(\"../mod.toml\")]`",
        )
    })
}

/// The engine's hooks, checked both ways: the block cannot answer an event the
/// manifest does not promise, and the manifest cannot promise one the block
/// leaves out.
fn engine_hooks(manifest: &LitStr, realm: Realm, half: &Half, block: &ItemImpl) -> syn::Result<()> {
    for hook in half.engine() {
        if !realm.engine_hooks().contains(&hook.name.as_str()) {
            return Err(syn::Error::new(
                manifest.span(),
                format!(
                    "[declares.{}] names `{}`, which is no hook of `{}`; its engine hooks are {}, \
                     and a hook of the mod's own is a table naming its rule",
                    realm.section(),
                    hook.name,
                    realm.trait_name(),
                    named(realm.engine_hooks().iter().copied())
                ),
            ));
        }
        let written = block.items.iter().any(|item| match item {
            ImplItem::Fn(written) => written.sig.ident == hook.name,
            _ => false,
        });
        if !written {
            return Err(syn::Error::new(
                manifest.span(),
                format!(
                    "[declares.{}] names the hook `{}` and this block does not implement it",
                    realm.section(),
                    hook.name
                ),
            ));
        }
    }

    for item in &block.items {
        let ImplItem::Fn(written) = item else {
            continue;
        };
        let name = written.sig.ident.to_string();
        if name == INIT || !realm.engine_hooks().contains(&name.as_str()) {
            continue;
        }
        if half.declares(&name).is_none() {
            return Err(syn::Error::new(
                written.sig.ident.span(),
                format!(
                    "this block implements `{name}` and [declares.{}] does not name it; the host \
                     reads the manifest without starting the mod, so add `{name}` to hooks in {}",
                    realm.section(),
                    manifest.value()
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
