# greybel-vs

Releases before 3.0.0: [greybel-vs's changelog](https://github.com/ayecue/greybel-rs/blob/02aaff4745e345ea2ca0d125be2fe0883b6e4b51/apps/ts/greybel-vs/CHANGELOG.md).

## 3.0.0 · 2026-11-09

### Major changes

- Greybel v3: greybel is rewritten in Rust. The language server, the runtime and debugger, the builder and the game server are Rust programs now, shipped in the extension as a binary per platform and, on the web, as WebAssembly workers.
- Programs run in the mock world or in Grey Hack through the message hook. All mock runtimes in an editor window share one persistent world, served by the game server.
