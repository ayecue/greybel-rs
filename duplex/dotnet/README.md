# Duplex .NET

`duplex-core` is the C# implementation of [`duplex/SPEC.md`](../SPEC.md). It targets `netstandard2.1` so Unity's Mono can load it.

## Provider-only scope

The current implementation serves protocols as their **provider**. It handles notifications in both directions, but does not implement exchanges that wait for a response.

Implemented behavior includes:

- Binary layouts, batches, slices, and aliases.
- Reassembling fragmented frames. The sender does not currently emit frames large enough to require fragmentation.
- `Listen`, `Open`/`Ack`/`Refuse`, version and fingerprint negotiation, `Ping`/`Pong`, `Close`, and health checks.
- Validation of client values against declared requirements and limits.
- Rejection of messages that wait for a response as `UnhandledMessage`, or as `MalformedMessage`, `Invalid`, or `UnsupportedVersion` when they cannot be read, violate a requirement, or exceed the connection's minor.
- A TCP provider through `TcpProvider` and `StreamDriver`, with a varuint length before each batch, matching Rust's `StreamDriver::tcp`.

Responses, streams, credit, cancellation, timeouts, migrations, handshake payloads, security, and the client role are not implemented. A protocol with a handshake does not generate. A channel whose protocol requires security closes as `Insecure` when it starts.

## Runtime API

A `Hub` runs a thread's transports and channels. Their work runs as hub tasks, and each task's writes form one batch per transport. Other threads submit work with `Hub.Post`.

- `Hub.Start` runs the hub on its own thread.
- `Hub.RunReady` runs due work on the calling thread; tests use it with a manual clock.

The cross-language scheduling and batching contract is defined in [`SPEC.md`](../SPEC.md#runtime).

## Generated code

The `duplex/rs/duplex-cs` crate generates C# from a [protocol definition](../../protocols/README.md), using the same resolved plan as the TypeScript generator. Output includes:

- Types and their binary codecs.
- Messages and the protocol's `ProtocolDefinition`.
- A provider router with one method per received notification.
- A provider channel with a `Send` method per notification its client handles.
- Tag and field metadata for routing and conversion without reflection.

Generated code qualifies core types with `global::Duplex.`, preventing project or game types from shadowing them.

### Tags

Generated C# exposes [definition tags](../../protocols/README.md#tags) as data:

| Definition item | C# metadata |
|---|---|
| Protocol | `ProtocolDefinition.Tags` |
| Message | `MessageType.Tags`, `MessageType.Fields`, and `MessageType.Create` |
| Struct | `StructType.Tags`, `StructType.Fields`, and `StructType.Create` |
| Union | `UnionType.Tags` and its `VariantType` values |
| Enum | `{Enum}Codec.EnumType.Tags` and its `MemberType` values |
| Message, struct, or variant field | `FieldType`, with its name, property type, tags, and `Get` method |
| Union variant | `VariantType.Tags` and `VariantType.Fields` |
| Enum member | `MemberType.Tags` and `MemberType.Value` |

`MessageType.Create` and `StructType.Create` construct values from field values. `FieldType.Get` reads a field from an existing value.

### Regenerating checked-in code

The crate that owns a protocol writes its C# from a generator test. Run the listed command with `UPDATE_CS=1` from the indicated workspace:

| Generated files | Workspace | Command |
|---|---|---|
| `duplex-core/src/Definition.g.cs` from `duplex/core` | `duplex/rs` | `UPDATE_CS=1 cargo test -p duplex-core --test main cs::` |
| `duplex-core/tests/Generated/*.g.cs` for the conformance calculator and layouts | `duplex/rs` | `UPDATE_CS=1 cargo test -p duplex-core --test main cs::` |
| `greybel/dotnet/greyhack-message-hook-proxy/src/Protocol/Protocol.g.cs` from `protocols/greyhack` | `greybel/rs` | `UPDATE_CS=1 cargo test -p greyhack-message-hook-client --test main cs::` |

`DUPLEX_DOTNET` and `GREYBEL_DOTNET` in the root [`.cargo/config.toml`](../../.cargo/config.toml) select the output folders.

## Including the core in a Unity plugin

Unity loads one assembly per plugin, so a plugin compiles the core sources into its own assembly. `$(DuplexDotnet)` is defined by `greybel/dotnet/Directory.Build.props`:

```xml
<Compile Include="$(DuplexDotnet)duplex-core/src/**/*.cs" Exclude="$(DuplexDotnet)duplex-core/src/obj/**;$(DuplexDotnet)duplex-core/src/bin/**" LinkBase="duplex" />
```

## Tests

From `duplex/dotnet`, run:

```sh
cd duplex-core
dotnet test --solution DuplexCore.slnx
```

| Suite | Coverage |
|---|---|
| `ScenarioTests` | Runs calculator major 1 provider scenarios from `duplex/scenarios` whose `requires` entries are supported by `Scenario.Supported`: `health-check` and `fragments`. The suite requires at least 18 applicable scenarios and uses the same expectations as the Rust and TypeScript runners. |
| `FixtureTests` | Reads and writes `duplex/conformance/fixtures.json`, lists fixtures for omitted features such as declared failures, handshakes, and security, and checks that every fixture is accounted for. |
| `TagTests` | Checks tags, fields, and construction using `duplex/conformance/tags` and `Generated/Sensor.g.cs`. |
| `TcpTests` | Serves a raw TCP client. |
| `RustClientTests` | Builds and serves duplex-driver-stdio's `tcp-user` example, a Rust core client, over TCP. |
