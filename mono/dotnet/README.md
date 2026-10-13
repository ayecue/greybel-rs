# Mono .NET projects

| Project | Purpose |
|---|---|
| [`mono-regex`](mono-regex/) | Builds the Native AOT regex library used by the Rust `mono-regex` crate's `dotnet` feature. |
| [`mono-fixtures`](mono-fixtures/) | Runs on Grey Hack's Mono runtime and records number, string, collation, and dictionary fixtures for the Rust crates. |

Run `make help` for all targets. Common commands are:

```sh
make regex                 # host runtime identifier
make regex RIDS=all        # every supported desktop target
make fixtures              # requires the macOS game
make clean
```

`Directory.Build.props` defines `MonoRust` so projects can read and write the
Mono Rust workspace.
