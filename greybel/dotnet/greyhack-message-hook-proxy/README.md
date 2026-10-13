# greyhack-message-hook-proxy

Game-independent `netstandard2.1` message proxy for the message hook. It has no
Unity, BepInEx or game-assembly dependency; the plugin compiles these sources
and the duplex core into its own assembly. The test-support tool can also run
it with standard streams as a fake game.

See the [integration guide](../../../docs/greyhack-integration.md#message-proxy-and-wire-format)
and [proxy test guide](../../../docs/game-testing.md#proxy-bridge-without-grey-hack).

## Behavior and API

`MessageProxy` accepts one duplex provider channel per TCP client on its hub
thread. Messages from clients go to `IGame.Send`; messages passed to
`MessageProxy.Publish` are broadcast to every client.

`RawMessage` is the game's JSON representation:

```json
{"method":1011,"params":["context-id",1]}
```

`method` is the direction-specific game or plugin ID and `params` is positional.
There are no `strValues` collections or per-type `Add*` calls. The corresponding
game API is `NetworkMessages.RpcMessage`, including `GetParam<T>()`.

`Converter` translates generated protocol messages with `Encode` and
`Decode(raw, sender)`. `Role.Client` decodes a message sent to the game server;
`Role.Provider` decodes one sent to its client. `IsExtension` distinguishes
plugin messages, and `GetWindowPids` reads `pid` metadata. Structs become JSON
objects with their game field names. Optional trailing parameters may be
omitted, and nullable parameters may be explicit JSON nulls. Conversion maps
both an absent optional and an explicit null to null; it does not preserve them
as distinct values.

The generated `Protocol.g.cs` metadata supplies:

- `game_version`, `game_message_id` and `plugin_message_id`;
- each parameter's `game_param_type` and optional `game_param_optional` marker;
- `pid` for window routing and `game_field_name` for struct fields.

Converter construction validates these tags and duplicate IDs per sender so an
invalid protocol fails during plugin startup.

## Build and test

From this folder:

```sh
dotnet build src
```

The end-to-end proxy tests are Rust tests driven by
`greyhack-test-support proxy`:

```sh
cd ../../rs
cargo nextest run -p greyhack-message-hook-client --features dotnet
```

After changing `protocols/greyhack`, regenerate the C# bindings from
`greybel/rs`:

```sh
UPDATE_CS=1 cargo test -p greyhack-message-hook-client --test main cs::
```
