# greyhack-message-hook

BepInEx plugin that connects Greybel to Grey Hack. It listens on
`127.0.0.1:7777` by default and provides in-game file deployments, builds,
script contexts and debugging through the generated `protocols/greyhack`
duplex protocol.

Start with the [integration guide](../../../docs/greyhack-integration.md).

## Prerequisites

- .NET 10 SDK to build and test.
- Grey Hack's assemblies from `GREY_HACK_PATH` (game folder) or
  `GREY_HACK_MANAGED_PATH` (its `Managed` folder).
- BepInEx 5 or 6 in the game that loads the plugin.

## Build and install

From `greybel/dotnet`:

```sh
make message-hook             # Both Release builds
make message-hook-bepinex5
make message-hook-bepinex6
make message-hook-debug
make test-message-hook        # Uses the installed game's assemblies
```

Or build one configuration from this folder:

```sh
dotnet build src -c Release-BepInEx5
dotnet test --solution GreyHackMessageHook.slnx
```

| Configuration | Assembly and output below `src/bin/` |
|---|---|
| `Release-BepInEx5`, `Debug-BepInEx5` | `<configuration>/BepInEx5/netstandard2.1/GreyHackMessageHook5.dll` |
| `Release-BepInEx6`, `Debug-BepInEx6` | `<configuration>/BepInEx6/netstandard2.1/GreyHackMessageHook.dll` |

Install with the [setup wizard](../../../apps/dotnet/greyhack-setup-wizard/README.md),
or copy only the matching DLL to `BepInEx/plugins`. Do not ship a duplicate
Newtonsoft.Json. The BepInEx `General.Port` setting controls the listener.

The game tests use the Rust client and the isolated runner:

```sh
cd ../../rs
cargo nextest run -p greyhack-message-hook-client --features game
BEPINEX=6 cargo nextest run -p greyhack-message-hook-client --features game
```

See [Game testing](../../../docs/game-testing.md) for shared-session and
concurrency behavior.

## Protocol and updates

The proxy converts generated messages to `NetworkMessages.RpcMessage` JSON:
`method` plus positional `params`, read by the game with `GetParam<T>()`.
`Converter.Encode`/`Decode`, `IsExtension` and `GetWindowPids` implement the
mapping from protocol metadata. Builds and contexts remain notifications;
clients correlate them using `build_id` and `context_id`.

After a game update, from `greybel/dotnet` and then `greybel/rs`:

```sh
make contracts
make message-hook
make test-message-hook

UPDATE_CS=1 cargo test -p greyhack-message-hook-client --test main cs::
```

The last command regenerates the proxy's C# bindings. Review
`greybel/contracts/fingerprint.json` and `protocols/greyhack`; additions can use
a new minor protocol version, while changed or removed messages need a major.
The startup version warning and reflection/Harmony tests identify game changes
that need code updates. Contract details are in
[`greyhack-util`](../greyhack-util/README.md), and proxy behavior in
[`greyhack-message-hook-proxy`](../greyhack-message-hook-proxy/README.md).
