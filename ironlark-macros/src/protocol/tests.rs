//! The round trip over `fixture/`: compile a schema the way a mod is built,
//! then read the table, the generated types and the emitted surface back.

use super::declaration::{Realm, Signal};
use super::options::{Audience, Keep, Order};
use super::{Protocol, expand, read_protocol};
use proc_macro2::Span;
use quote::quote;
use syn::LitStr;

/// The fixture holds the echo mod's shapes: a bus signal, a superseding
/// crossing signal, a plain helper type, and a request answered by the server.
fn read(name: &str) -> syn::Result<Protocol> {
    read_protocol(&LitStr::new(name, Span::call_site()))
}

fn fixture() -> Protocol {
    match read("fixture/protocol.proto") {
        Ok(protocol) => protocol,
        Err(refusal) => panic!("the fixture is read: {refusal}"),
    }
}

fn signal<'a>(protocol: &'a Protocol, name: &str) -> &'a Signal {
    let found = protocol
        .declarations
        .signals
        .iter()
        .find(|signal| signal.name.as_str() == name);
    match found {
        Some(signal) => signal,
        None => panic!("{name} is declared in the fixture"),
    }
}

/// What a mod's own module holds after the line expands.
fn emitted(name: &str) -> String {
    expand(quote!(#name)).to_string()
}

/// The types generated for the mod's own schema, which is the first read.
fn own_types(protocol: &Protocol) -> String {
    match protocol.payloads.first() {
        Some(generated) => generated.types.to_string(),
        None => panic!("the mod's own schema generates types"),
    }
}

#[test]
fn an_optioned_message_declares_a_signal_and_carries_its_delivery() {
    let protocol = fixture();

    let declared: Vec<&str> = protocol
        .declarations
        .signals
        .iter()
        .map(|signal| signal.name.as_str())
        .collect();
    assert_eq!(declared, ["pressed", "positions", "value"]);

    let pressed = signal(&protocol, "pressed");
    assert_eq!(pressed.payload.to_string(), "Pressed");
    assert_eq!(pressed.audience, Audience::ServerMods);
    assert_eq!(pressed.transit.keep, Keep::KeepAll);
    assert_eq!(pressed.transit.order, Order::InOrder);

    let positions = signal(&protocol, "positions");
    assert_eq!(positions.payload.to_string(), "Positions");
    assert_eq!(positions.audience, Audience::Clients);
    assert_eq!(positions.transit.keep, Keep::Newest);
    assert_eq!(positions.transit.order, Order::InOrder);

    let value = signal(&protocol, "value");
    assert_eq!(value.payload.to_string(), "Value");
    assert_eq!(value.audience, Audience::Clients);
    assert_eq!(value.transit.keep, Keep::KeepAll);
}

#[test]
fn a_service_rpc_declares_a_request_with_both_types() {
    let protocol = fixture();
    let requests = &protocol.declarations.requests;
    assert_eq!(requests.len(), 1);

    let advance = &requests[0];
    assert_eq!(advance.name.as_str(), "advance");
    assert_eq!(advance.payload.to_string(), "AdvanceRequest");
    assert_eq!(advance.response.to_string(), "Value");
    assert_eq!(advance.realm, Realm::Server);
}

#[test]
fn the_payload_types_are_generated_with_their_tags() {
    let generated = own_types(&fixture());
    for message in [
        "Pressed",
        "Positions",
        "PlayerPos",
        "Value",
        "AdvanceRequest",
    ] {
        assert!(
            generated.contains(&format!("struct {message}")),
            "{message} is generated, in: {generated}"
        );
    }
    assert!(
        generated.contains("tag = \"1\""),
        "a tag comes from the grammar, in: {generated}"
    );
    assert!(
        generated.contains("ironlark :: prost"),
        "the generated code names the prost the SDK re-exports, in: {generated}"
    );
}

#[test]
fn every_declaration_reaches_a_marker_in_its_kind_module() {
    let said = emitted("fixture/protocol.proto");
    for held in [
        "pub mod signal",
        "pub use super :: ironlark_echo :: Pressed",
        "pub use super :: ironlark_echo :: Positions",
        "pub use super :: ironlark_echo :: Value",
        "pub mod request",
        "pub use super :: ironlark_echo :: AdvanceRequest",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
    assert!(
        !said.contains("pub use super :: ironlark_echo :: PlayerPos"),
        "a message with no option is listed under no kind, in: {said}"
    );
}

#[test]
fn a_signal_declaration_carries_the_name_the_audience_and_the_transit() {
    let said = emitted("fixture/protocol.proto");
    for held in [
        "impl :: ironlark :: protocol :: SignalSpec for ironlark_echo :: Positions",
        "pub use super :: ironlark_echo :: Positions",
        "type Audience = :: ironlark :: protocol :: Clients",
        "const NAME : & 'static str = \"positions\"",
        "keep : :: ironlark :: protocol :: Keep :: Newest",
        "order : :: ironlark :: protocol :: Order :: InOrder",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
    assert!(
        said.contains("type Audience = :: ironlark :: protocol :: ServerMods"),
        "a bus signal carries its own audience, in: {said}"
    );
}

#[test]
fn a_request_declaration_carries_its_name_and_its_response() {
    let said = emitted("fixture/protocol.proto");
    for held in [
        "impl :: ironlark :: protocol :: RequestSpec for ironlark_echo :: AdvanceRequest",
        "pub use super :: ironlark_echo :: AdvanceRequest",
        "type Response = ironlark_echo :: Value",
        "const NAME : & 'static str = \"advance\"",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
}

#[test]
fn a_service_naming_neither_half_refuses() {
    let Err(refusal) = read("fixture/wrong-service.proto") else {
        panic!("`Echo` names no half");
    };
    let said = refusal.to_string();
    assert!(
        said.contains("service `Echo` names no half"),
        "said: {said}"
    );
}

#[test]
fn a_package_nested_under_the_prefix_refuses() {
    let Err(refusal) = read("fixture/nested-package.proto") else {
        panic!("mods.ironlark.echo is not one flat segment");
    };
    let said = refusal.to_string();
    assert!(
        said.contains("is not `mods.<author>_<mod>`"),
        "said: {said}"
    );
}

#[test]
fn a_request_the_client_half_answers_refuses() {
    let said = emitted("fixture/client-service.proto");
    assert!(
        said.contains("compile_error !") && said.contains("is not served"),
        "the direction that is not served refuses, in: {said}"
    );
}

/// The fixture the crate's own documentation examples declare against. A page
/// that will not compile is a page nobody reads, so it is read here too.
#[test]
fn the_doctest_schema_declares_the_door_mod() {
    let said = emitted("../ironlark/doctest/protocol.proto");
    assert!(!said.contains("compile_error !"), "in: {said}");
    for held in [
        "pub struct Latch",
        "pub struct Positions",
        "pub struct Opened",
        "pub use super :: ironlark_door :: OpenRequest",
        "type Response = ironlark_door :: Latch",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
}

/// The borrow round trip: an owner's schema, and a borrower importing it.
/// The borrower is `mods.ironlark_freeroam` importing the owner by its install
/// path, `ironlark/buttons/protocol.proto`, and one of its own fields is the
/// owner's type, so the cross-package path the generated code writes is exercised.
fn borrower() -> String {
    emitted("fixture/workshop/ironlark/freeroam/protocol.proto")
}

#[test]
fn a_borrowed_signal_is_listed_under_its_owner() {
    let said = borrower();
    assert!(!said.contains("compile_error !"), "in: {said}");
    for held in [
        "pub mod ironlark_freeroam",
        "pub mod ironlark_buttons",
        "pub use ironlark_freeroam :: *",
        "pub use super :: ironlark_freeroam :: Roamed",
        "pub mod ironlark_buttons { # [doc = \"The signal `pressed`, borrowed from `ironlark:buttons`",
        "pub use super :: super :: ironlark_buttons :: Pressed",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
}

#[test]
fn a_borrowed_signal_carries_the_qualified_name_and_the_owners_delivery() {
    let said = borrower();
    for held in [
        "impl :: ironlark :: protocol :: SignalSpec for ironlark_buttons :: Pressed",
        "const NAME : & 'static str = \"ironlark:buttons/signal/pressed\"",
        "type Audience = :: ironlark :: protocol :: ServerMods",
        "const NAME : & 'static str = \"ironlark:buttons/signal/lit\"",
        "keep : :: ironlark :: protocol :: Keep :: Newest",
    ] {
        assert!(said.contains(held), "{held} is emitted, in: {said}");
    }
    assert!(
        said.contains("const NAME : & 'static str = \"roamed\""),
        "an own name stays bare, in: {said}"
    );
}

#[test]
fn the_generated_cross_package_path_resolves_in_the_layout_emitted() {
    let said = borrower();
    assert!(
        said.contains("super :: ironlark_buttons :: Plain"),
        "prost writes the owner's type relative to the packages, in: {said}"
    );
    // Both schemas are introduced by the one line that emits a type module, so
    // they are siblings of one parent and `super` reaches from either to the
    // other. Two of that doc is the whole proof the layout holds.
    for owner in ["ironlark_freeroam", "ironlark_buttons"] {
        let doc = format!("The types the `{owner}` schema describes.\"] pub mod {owner}");
        assert!(said.contains(&doc), "{doc} is emitted, in: {said}");
    }
}

#[test]
fn a_borrowed_plain_type_and_a_borrowed_request_are_listed_nowhere() {
    let said = borrower();
    assert!(
        !said.contains("pub mod request"),
        "the owner's service mints nothing here, in: {said}"
    );
    assert!(
        said.contains("pub struct Plain"),
        "the owner's plain type is generated, in: {said}"
    );
    assert!(
        !said.contains("pub use super :: super :: ironlark_buttons :: Plain"),
        "a message with no option declares nothing, in: {said}"
    );
}

/// A mod may borrow and mint nothing itself. Its own module then generates no
/// source, and it must still be the one the bare re-export reaches: taking the
/// first module that generated something would publish an owner's whole schema
/// under this mod's own name.
#[test]
fn a_borrower_minting_nothing_keeps_its_own_module() {
    let said = emitted("fixture/workshop/ironlark/watchman/protocol.proto");
    assert!(!said.contains("compile_error !"), "in: {said}");
    assert!(
        said.contains("pub mod ironlark_watchman { }")
            && said.contains("pub use ironlark_watchman :: *"),
        "the own module is emitted empty and re-exported, in: {said}"
    );
    assert!(
        !said.contains("pub use ironlark_buttons :: *"),
        "the owner's schema is not this mod's own, in: {said}"
    );
}

/// The name the compile settles on for the mod's own file. It is the identity
/// the shipped descriptor carries, so it has to be the install path and not
/// where the crate happened to be built.
fn compiled_names(relative: &str) -> Vec<String> {
    let directory = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(directory) => directory,
        Err(_) => panic!("cargo sets the manifest directory"),
    };
    let path = std::path::Path::new(&directory).join(relative);
    let compiler = match super::schema::compile(&path) {
        Ok(compiler) => compiler,
        Err(refusal) => panic!("{relative} compiles: {refusal}"),
    };
    compiler
        .files()
        .map(|file| file.name().to_owned())
        .collect()
}

#[test]
fn the_workshop_root_names_a_schema_by_its_install_path() {
    let names = compiled_names("fixture/workshop/ironlark/freeroam/protocol.proto");
    assert!(
        names.contains(&"ironlark/freeroam/protocol.proto".to_owned()),
        "the mod's own file is named by its install path, in: {names:?}"
    );
    assert!(
        names.contains(&"ironlark/buttons/protocol.proto".to_owned()),
        "the owner is imported under the same install path, in: {names:?}"
    );
    assert!(
        names.contains(&super::schema::OPTIONS.to_owned()),
        "the platform schema resolves ahead of every include root, in: {names:?}"
    );
}

/// A half's crate sits under the mod, so the path an author writes holds `..`
/// and the workshop root is only two directories up once it is settled. Taking
/// the ancestors of the written path instead would climb from `..` and land a
/// directory short.
#[test]
fn a_path_holding_a_parent_step_settles_to_the_same_name() {
    let names = compiled_names("fixture/workshop/ironlark/freeroam/../freeroam/protocol.proto");
    assert!(
        names.contains(&"ironlark/freeroam/protocol.proto".to_owned()),
        "the written `..` does not reach a different name, in: {names:?}"
    );
}
