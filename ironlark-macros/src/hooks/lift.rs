//! Splitting one written block into the two the compiler needs.
//!
//! The realm traits are closed, so a function named by the mod rather than by
//! the contract cannot stand inside the trait impl. It is lifted into an
//! inherent impl of the same type, which keeps `Self` meaning what the author
//! wrote it to mean, and the trait impl keeps only what the trait declares.

use super::declared::{Half, INIT, Rule};
use super::realm::{Realm, named};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ImplItem, ImplItemFn, ItemImpl};

/// One written block, split by where each function has to live.
pub(crate) struct Split {
    /// What the trait declares, in the order it was written.
    pub(crate) trait_items: Vec<ImplItem>,
    /// The mod's own hooks, lifted out whole.
    pub(crate) lifted: Vec<ImplItemFn>,
}

/// One arm of a rule's dispatch: the number a hook arrives as, and the
/// function it reaches.
pub(crate) struct Arm {
    id: u32,
    called: Ident,
    rule: Rule,
}

/// Splits `block` against what `half` declares.
///
/// A function is the trait's if the trait spells it, the mod's own if the
/// section declares it, and neither is a refusal naming both sets — the one
/// message that tells an author whether the name is misspelled or the
/// declaration is missing.
pub(crate) fn split(block: &ItemImpl, realm: Realm, half: &Half) -> syn::Result<Split> {
    let mut trait_items = Vec::with_capacity(block.items.len());
    let mut lifted = Vec::new();

    for item in &block.items {
        let ImplItem::Fn(written) = item else {
            trait_items.push(item.clone());
            continue;
        };
        let name = written.sig.ident.to_string();
        if name == INIT || realm.engine_hooks().contains(&name.as_str()) {
            trait_items.push(item.clone());
            continue;
        }
        match half.declares(&name) {
            Some(hook) if hook.rule != Rule::Engine => lifted.push(written.clone()),
            _ => {
                return Err(syn::Error::new(
                    written.sig.ident.span(),
                    format!(
                        "`{name}` is neither a hook of `{}` nor one this half declares; its \
                         engine hooks are {}, and a hook of the mod's own is declared under \
                         [declares.{}] as `{{ name = \"{name}\", default-bindings = [...] }}`",
                        realm.trait_name(),
                        named(realm.engine_hooks().iter().copied()),
                        realm.section()
                    ),
                ));
            }
        }
    }

    Ok(Split {
        trait_items,
        lifted,
    })
}

/// Numbers this half's author hooks and pairs each with the function it
/// reaches.
///
/// The number is the position among the half's own hooks in the order the
/// manifest writes them, which is the numbering the host mints from the same
/// file. Reproducing it is what lets a dispatch arrive as a number and reach a
/// function with no resolve in between.
pub(crate) fn arms(realm: Realm, half: &Half, split: &Split) -> syn::Result<Vec<Arm>> {
    let mut arms = Vec::new();
    for (position, hook) in half.author().enumerate() {
        if !realm.serves(hook.rule) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "the hook `{}` declares its rule under [declares.{}], and that rule is not \
                     served on this half: `default-bindings` names an input, and an input reaches \
                     the client half alone",
                    hook.name,
                    realm.section()
                ),
            ));
        }
        let Some(written) = split
            .lifted
            .iter()
            .find(|written| written.sig.ident == hook.name)
        else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "[declares.{}] declares the hook `{}` and this block does not implement it",
                    realm.section(),
                    hook.name
                ),
            ));
        };
        arms.push(Arm {
            id: position as u32,
            called: written.sig.ident.clone(),
            rule: hook.rule,
        });
    }
    Ok(arms)
}

/// Mints the dispatch the input rule's export enters.
///
/// Nothing is allocated and nothing is searched: the match is the table, and
/// the compiler lays it out. A number no arm claims is a host and manifest
/// that disagree, so it is reported rather than dropped.
pub(crate) fn input_dispatch(arms: &[Arm]) -> TokenStream {
    let written = arms
        .iter()
        .filter(|arm| arm.rule == Rule::Input)
        .map(|arm| {
            let (id, called) = (arm.id, &arm.called);
            quote! { #id => Self::#called(ctx, edge).await, }
        });
    let written: Vec<TokenStream> = written.collect();
    if written.is_empty() {
        return TokenStream::new();
    }
    quote! {
        #[doc(hidden)]
        async fn on_input(
            ctx: ::ironlark::Context,
            hook: u32,
            edge: ::ironlark::client::InputEdge,
        ) {
            match hook {
                #(#written)*
                unrouted => ::ironlark::protocol::unrouted_hook(unrouted),
            }
        }
    }
}

/// The inherent impl the lifted functions land in, carrying the trait impl's
/// own generics so a generic half stays generic.
pub(crate) fn inherent(block: &ItemImpl, lifted: &[ImplItemFn]) -> TokenStream {
    if lifted.is_empty() {
        return TokenStream::new();
    }
    let (generics, _, where_clause) = block.generics.split_for_impl();
    let ty = &block.self_ty;
    quote! {
        impl #generics #ty #where_clause {
            #(#lifted)*
        }
    }
}
