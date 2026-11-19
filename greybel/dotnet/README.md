# Greybel .NET projects

| Project | Purpose |
|---|---|
| [`greyhack-message-hook`](greyhack-message-hook/README.md) | BepInEx 5/6 plugin for in-game builds, imports, contexts and debugging. |
| [`greyhack-message-hook-proxy`](greyhack-message-hook-proxy/README.md) | Game-independent duplex listener and `RpcMessage` converter compiled into the plugin. |
| [`greyhack-test-support`](greyhack-test-support/README.md) | Headless game runner, shared sessions and stdio proxy bridge. |
| [`greyhack-util`](greyhack-util/README.md) | Contract extraction, protocol generation and game asset extraction. |

The setup wizard is Rust: [`greyhack-setup`](../rs/greyhack-setup/README.md) has its steps,
[`apps/rs/greyhack-setup-wizard`](../../apps/rs/greyhack-setup-wizard/README.md) its window and command line.
For use and testing, start with [Grey Hack integration](../../docs/greyhack-integration.md)
and [Game testing](../../docs/game-testing.md).

## Prerequisites and commands

Use the .NET 10 SDK. Building the plugin or extracting from the game also needs
`GREY_HACK_PATH` (the game folder) or `GREY_HACK_MANAGED_PATH` (its `Managed`
folder). Game assemblies are not committed. Asset commands accept
`GREY_HACK_DATA` for the game's data folder.

From this directory:

```sh
make message-hook       # Release plugin for BepInEx 5 and 6
make message-hook-debug # Debug plugin for both majors
make contracts          # Extract contracts and generate protocols/greyhack
make assets             # Extract world and terminal-text bundles
make test               # Utility and test-support tests
make test-message-hook  # Reflection, Harmony and wire tests; needs game assemblies
make help
```

`make test` does not include the Rust proxy/game suites; their
commands are in the guides above. `global.json` selects
Microsoft.Testing.Platform for the xunit test projects.
