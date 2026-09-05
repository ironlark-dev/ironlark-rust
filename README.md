# ironlark

[![crates.io](https://img.shields.io/crates/v/ironlark?logo=rust&logoColor=white&color=b7410e)](https://crates.io/crates/ironlark)
[![docs.rs](https://img.shields.io/docsrs/ironlark?logo=docs.rs&label=docs.rs)](https://docs.rs/ironlark)
[![license](https://img.shields.io/crates/l/ironlark?color=blue)](LICENSE)
[![Discord](https://img.shields.io/badge/discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/jAQU93uMy4)

The Rust SDK for writing Ironlark mods.

> [!WARNING]
> Early-stage development. Consider everything here experimental: hard
> breaking changes land over the coming versions, without deprecation cycles.

A mod has up to two halves. The server half runs on the machine hosting a
session and holds authority over the world; the client half runs on every
player's machine and owns what that player sees and does. Each half is one
trait implementation, compiled to a WebAssembly component for the game and
run natively by `cargo test` with no game present.

## Install

```toml
[dependencies]
ironlark = "0.1"
```

Building a component needs the `wasm32-wasip2` target
(`rustup target add wasm32-wasip2`); testing needs nothing beyond stable Rust.

## Documentation

- Ironlark modding docs: [docs.ironlark.net](https://docs.ironlark.net)
- This crate's API reference: [docs.rs/ironlark](https://docs.rs/ironlark)

## Community

Join the Ironlark [Discord](https://discord.gg/jAQU93uMy4)

## License

Apache-2.0
