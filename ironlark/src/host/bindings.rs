//! Generated contract bindings, one module per world. The client world remaps
//! every shared interface into the server world's module so one crate carries
//! both without duplicate types; remap keys carry the package version.

pub mod server {
    wit_bindgen::generate!({
        path: "wit",
        world: "ironlark:host/server-mod",
        pub_export_macro: true,
        export_macro_name: "export_server_world",
        default_bindings_module: "ironlark::bindings::server",
    });
}

pub mod client {
    wit_bindgen::generate!({
        path: "wit",
        world: "ironlark:host/client-mod",
        pub_export_macro: true,
        export_macro_name: "export_client_world",
        default_bindings_module: "ironlark::bindings::client",
        with: {
            "ironlark:host/types@0.1.0": crate::bindings::server::ironlark::host::types,
            "ironlark:host/event@0.1.0": crate::bindings::server::ironlark::host::event,
            "ironlark:host/resolve@0.1.0": crate::bindings::server::ironlark::host::resolve,
            "ironlark:host/log@0.1.0": crate::bindings::server::ironlark::host::log,
            "ironlark:host/session@0.1.0": crate::bindings::server::ironlark::host::session,
            "ironlark:host/audio@0.1.0": crate::bindings::server::ironlark::host::audio,
            "ironlark:host/signal@0.1.0": crate::bindings::server::ironlark::host::signal,
            "ironlark:host/ui@0.1.0": generate,
            "ironlark:host/request@0.1.0": generate,
        },
    });
}
