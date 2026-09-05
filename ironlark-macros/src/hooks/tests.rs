//! The attribute over `fixture/`: what one written block becomes, and what a
//! manifest and a block that disagree are told.

use super::expand;
use proc_macro2::TokenStream;
use quote::quote;

/// The client half of the fixture mod: one engine hook, one of the mod's own.
fn client(body: TokenStream) -> String {
    expand(quote!("fixture/mod.toml"), body).to_string()
}

fn server(body: TokenStream) -> String {
    expand(quote!("fixture/mod.toml"), body).to_string()
}

#[test]
fn one_of_the_mods_own_hooks_is_lifted_out_of_the_trait_impl() {
    let said = client(quote! {
        impl ClientMod for Door {
            async fn init() {}
            async fn use_door(ctx: Context, edge: InputEdge) {}
            async fn latch(ctx: Context, edge: InputEdge) {}
        }
    });
    assert!(
        said.contains("impl ClientMod for Door { async fn init"),
        "the trait impl keeps what the trait declares, in: {said}"
    );
    assert!(
        said.contains("impl Door { async fn use_door"),
        "the mod's own hook is lifted into an inherent impl, in: {said}"
    );
    assert!(
        !said.contains("compile_error !"),
        "the fixture and this block agree, in: {said}"
    );
}

#[test]
fn the_dispatch_numbers_by_the_order_the_manifest_writes() {
    let said = client(quote! {
        impl ClientMod for Door {
            async fn use_door(ctx: Context, edge: InputEdge) {}
            async fn latch(ctx: Context, edge: InputEdge) {}
        }
    });
    assert!(
        said.contains("async fn on_input"),
        "the input rule's dispatch is minted, in: {said}"
    );
    assert!(
        said.contains("0u32 => Self :: use_door (ctx , edge) . await"),
        "the first hook the manifest writes is nought, in: {said}"
    );
    assert!(
        said.contains("1u32 => Self :: latch (ctx , edge) . await"),
        "the second is one, in: {said}"
    );
    assert!(
        said.contains("unrouted => :: ironlark :: protocol :: unrouted_hook (unrouted)"),
        "a number no arm claims is reported, in: {said}"
    );
}

#[test]
fn a_half_with_no_hook_of_its_own_mints_no_dispatch() {
    let said = server(quote! {
        impl ServerMod for Door {
            async fn init() {}
            async fn on_tick(ctx: Context, dt: f32) {}
        }
    });
    assert!(
        !said.contains("on_input"),
        "nothing arrives at a half that declares no hook of its own, in: {said}"
    );
    assert!(
        !said.contains("impl Door"),
        "nothing is lifted, so there is no inherent impl, in: {said}"
    );
}

#[test]
fn an_engine_hook_the_section_does_not_name_refuses() {
    let said = server(quote! {
        impl ServerMod for Door {
            async fn on_tick(ctx: Context, dt: f32) {}
            async fn on_join(ctx: Context, player: Player) {}
        }
    });
    assert!(
        said.contains("compile_error !") && said.contains("does not name it"),
        "the block promises more than the manifest, in: {said}"
    );
}

#[test]
fn an_engine_hook_the_section_names_and_the_block_omits_refuses() {
    let said = server(quote! {
        impl ServerMod for Door {
            async fn init() {}
        }
    });
    assert!(
        said.contains("compile_error !") && said.contains("does not implement it"),
        "the manifest promises more than the block, in: {said}"
    );
}

#[test]
fn a_function_neither_side_declares_refuses_naming_both_sets() {
    let said = client(quote! {
        impl ClientMod for Door {
            async fn use_door(ctx: Context, edge: InputEdge) {}
            async fn latch(ctx: Context, edge: InputEdge) {}
            async fn slam(ctx: Context, edge: InputEdge) {}
        }
    });
    assert!(said.contains("compile_error !"), "in: {said}");
    for held in ["neither a hook of", "on_tick", "default-bindings"] {
        assert!(said.contains(held), "{held} is said, in: {said}");
    }
}

#[test]
fn one_of_the_mods_own_hooks_declared_and_not_implemented_refuses() {
    let said = client(quote! {
        impl ClientMod for Door {
            async fn use_door(ctx: Context, edge: InputEdge) {}
        }
    });
    assert!(
        said.contains("compile_error !") && said.contains("declares the hook `latch`"),
        "in: {said}"
    );
}

#[test]
fn an_impl_of_something_else_refuses() {
    let said = client(quote! {
        impl Door {
            async fn init() {}
        }
    });
    assert!(
        said.contains("compile_error !") && said.contains("not on an inherent impl"),
        "in: {said}"
    );
}

#[test]
fn a_missing_path_refuses() {
    let said = expand(
        TokenStream::new(),
        quote! {
            impl ClientMod for Door {}
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("takes the manifest path"),
        "in: {said}"
    );
}

#[test]
fn a_half_with_no_section_refuses() {
    let said = expand(
        quote!("fixture/one-half.toml"),
        quote! {
            impl ClientMod for Door {}
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("has no [declares.client] section"),
        "in: {said}"
    );
}

#[test]
fn a_manifest_still_declaring_communication_refuses() {
    let said = expand(
        quote!("fixture/moved-keys.toml"),
        quote! {
            impl ServerMod for Door {}
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("`channels` no longer declares"),
        "in: {said}"
    );
}

#[test]
fn init_named_in_a_hook_list_refuses() {
    let said = expand(
        quote!("fixture/init-listed.toml"),
        quote! {
            impl ServerMod for Door {
                async fn init() {}
            }
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("is not declared anywhere"),
        "in: {said}"
    );
}

#[test]
fn a_hook_of_the_mods_own_on_the_server_half_refuses() {
    let said = expand(
        quote!("fixture/server-input.toml"),
        quote! {
            impl ServerMod for Door {
                async fn use_door(ctx: Context, edge: InputEdge) {}
            }
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("is not served on this half"),
        "in: {said}"
    );
}

#[test]
fn a_field_no_hook_carries_refuses() {
    let said = expand(
        quote!("fixture/unknown-field.toml"),
        quote! {
            impl ClientMod for Door {}
        },
    )
    .to_string();
    assert!(
        said.contains("compile_error !") && said.contains("`every` is no field of a hook"),
        "in: {said}"
    );
}

/// The manifest the crate's own documentation examples declare against.
#[test]
fn the_doctest_manifest_carries_both_halves() {
    let path = quote!("../ironlark/doctest/mod.toml");
    let server = expand(
        path.clone(),
        quote! {
            impl ServerMod for Door {
                async fn init() {}
                async fn on_tick(ctx: Context, dt: f32) {}
            }
        },
    )
    .to_string();
    assert!(!server.contains("compile_error !"), "in: {server}");

    let client = expand(
        path,
        quote! {
            impl ClientMod for Door {
                async fn use_door(ctx: Context, edge: InputEdge) {}
            }
        },
    )
    .to_string();
    assert!(!client.contains("compile_error !"), "in: {client}");
    assert!(client.contains("0u32 => Self :: use_door"), "in: {client}");
}
