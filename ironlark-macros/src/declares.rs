//! `declares!`: the names a manifest holds everything about, as items the
//! compiler checks.
//!
//! A sound and an archetype are the two, because neither carries a Rust type:
//! a sound is a file and an archetype is a row of manifest fields, so the
//! manifest is the whole of what either needs. Names that carry a payload type
//! live on the mod's own schema and reach Rust through [`crate::protocol`].

use crate::manifest::read_manifest;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr};

pub fn expand(input: TokenStream) -> TokenStream {
    match syn::parse2(input).and_then(generate) {
        Ok(tokens) => tokens,
        Err(refusal) => refusal.to_compile_error(),
    }
}

/// The whole of the macro's input: one manifest path.
struct Declares {
    manifest: LitStr,
}

impl Parse for Declares {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let manifest: LitStr = input.parse()?;
        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "`declares!` takes the manifest path and nothing else; a signal and a request are \
                 declared on the payload's own schema and reach Rust through \
                 `ironlark::protocol!(\"../protocol.proto\")`, and a sound and an archetype need \
                 no line because the manifest holds everything about them",
            ));
        }
        Ok(Self { manifest })
    }
}

/// One declared name and the Rust item minted for it. The name is borrowed
/// from the manifest read, which outlives every use of it here.
struct Minted<'a> {
    item: Ident,
    name: &'a str,
}

fn generate(declares: Declares) -> syn::Result<TokenStream> {
    let manifest = &declares.manifest;
    let read = read_manifest(manifest)?;

    let sounds = mint(manifest, "sound", &read.sounds)?;
    let archetypes = mint(manifest, "archetype", &read.archetypes)?;

    let sound_items = sounds.iter().map(|minted| {
        let item = &minted.item;
        let doc = format!(
            "The declared sound `{}`, as `audio::play` takes it.",
            minted.name
        );
        quote! {
            #[doc = #doc]
            #[derive(Clone, Copy)]
            pub struct #item;
        }
    });
    let sound_module = (!sounds.is_empty()).then(|| {
        quote! {
            /// The sounds this mod's manifest declares, one item each.
            pub mod sound {
                #(#sound_items)*
            }
        }
    });

    let archetype_items = archetypes.iter().map(|minted| {
        let item = &minted.item;
        let doc = format!(
            "The declared archetype `{}`, as `Entity::spawn` takes it.",
            minted.name
        );
        quote! {
            #[doc = #doc]
            #[derive(Clone, Copy)]
            pub struct #item;
        }
    });
    let archetype_module = (!archetypes.is_empty()).then(|| {
        quote! {
            /// The archetypes this mod's manifest declares, one item each.
            pub mod archetype {
                #(#archetype_items)*
            }
        }
    });

    let sound_impls = sounds.iter().map(sound_impl);
    let archetype_impls = archetypes.iter().map(archetype_impl);

    Ok(quote! {
        // Re-reads the manifest on every build of this crate, so an edit to it
        // mints the items again.
        const __IRONLARK_MANIFEST_TRACKED: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #manifest));

        #sound_module
        #archetype_module

        #(#sound_impls)*
        #(#archetype_impls)*
    })
}

/// The items one kind's declared names reach, refusing a name no Rust name
/// reaches and two names reaching one.
fn mint<'a>(manifest: &LitStr, kind: &str, names: &'a [String]) -> syn::Result<Vec<Minted<'a>>> {
    let mut minted: Vec<Minted<'a>> = Vec::with_capacity(names.len());
    for name in names {
        let item = rust_name(name).ok_or_else(|| {
            syn::Error::new(
                manifest.span(),
                format!(
                    "no Rust name reaches the {kind} `{name}`; a declared name holds lowercase \
                     letters, digits and `-`, so rename it in {}",
                    manifest.value()
                ),
            )
        })?;
        if let Some(taken) = minted.iter().find(|taken| taken.item == item) {
            return Err(syn::Error::new(
                manifest.span(),
                format!(
                    "the {kind}s `{}` and `{name}` both reach the Rust name `{item}`; rename one \
                     of them in {}",
                    taken.name,
                    manifest.value()
                ),
            ));
        }
        minted.push(Minted { item, name });
    }
    Ok(minted)
}

/// A declared name as a Rust name: every dash-separated part capitalised, and
/// the leading underscore prost's own mapping gives a name opening with a
/// digit, so one rule covers a manifest name and a schema name alike.
fn rust_name(name: &str) -> Option<Ident> {
    let mut reached = String::with_capacity(name.len() + 1);
    for part in name.split('-') {
        let mut characters = part.chars();
        if let Some(first) = characters.next() {
            reached.extend(first.to_uppercase());
            reached.extend(characters);
        }
    }
    if reached.starts_with(|c: char| c.is_ascii_digit()) {
        reached.insert(0, '_');
    }
    syn::parse_str::<Ident>(&reached).ok()
}

fn sound_impl(minted: &Minted<'_>) -> TokenStream {
    let item = &minted.item;
    let name = minted.name;
    quote! {
        impl ::ironlark::protocol::SoundSpec for sound::#item {
            const NAME: &'static str = #name;
            fn resolved() -> ::ironlark::Result<::ironlark::SoundId> {
                ::ironlark::state! {
                    static ID: ::core::option::Option<::ironlark::SoundId> = ::core::option::Option::None;
                }
                ::ironlark::protocol::resolve_sound_once(&ID, #name)
            }
        }

        impl ::ironlark::protocol::PlayableSound for sound::#item {
            fn source(self) -> ::ironlark::protocol::SoundSource {
                ::ironlark::protocol::SoundSource::Lazy(
                    <Self as ::ironlark::protocol::SoundSpec>::resolved,
                )
            }
        }
    }
}

/// An archetype reaches a spawn as the name the manifest spells, so the item is
/// the declared name and nothing else. No id is resolved: the host takes the
/// name itself.
fn archetype_impl(minted: &Minted<'_>) -> TokenStream {
    let item = &minted.item;
    let name = minted.name;
    quote! {
        impl ::core::convert::AsRef<str> for archetype::#item {
            fn as_ref(&self) -> &str {
                #name
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rust_name;

    #[test]
    fn a_declared_name_reaches_a_rust_name() {
        for (declared, item) in [
            ("slam", "Slam"),
            ("use-door", "UseDoor"),
            ("hit1", "Hit1"),
            ("3-way", "_3Way"),
        ] {
            let Some(reached) = rust_name(declared) else {
                panic!("{declared} reaches {item}");
            };
            assert_eq!(reached.to_string(), item);
        }
    }
}
