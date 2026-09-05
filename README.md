# ironlark

The Rust SDK for writing Ironlark mods.

> [!WARNING]
> Early-stage development. Consider everything here experimental: hard
> breaking changes land over the coming versions, without deprecation cycles.

A mod has up to two halves. The server half runs on the machine hosting a
session and holds authority over the world; the client half runs on every
player's machine and owns what that player sees and does. Each half is one
trait implementation, compiled to a WebAssembly component for the game and
run natively by `cargo test` with no game present.

```toml
[dependencies]
ironlark = { git = "https://github.com/ironlark-dev/ironlark-rust" }
```

Building a component needs the `wasm32-wasip2` target
(`rustup target add wasm32-wasip2`); testing needs nothing beyond stable Rust.
The API reference is the crate documentation:

```sh
cargo doc -p ironlark --no-deps --open
```

## License

Apache-2.0
