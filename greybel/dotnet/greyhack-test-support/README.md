# greyhack-test-support

.NET command-line harness for automated Grey Hack tests: isolated headless game
sessions and the message-hook proxy with standard streams as the game. Rust's
`greybel-test-support::game` uses it for the `game` and `dotnet` test features.

The complete workflow is in [Game testing](../../../docs/game-testing.md).

## Prerequisites

- macOS or Linux for the headless game runner.
- .NET 10 SDK, Grey Hack and a logged-in Steam client.
- `GREY_HACK_PATH` for plugin builds. Alternatively, use `GREY_HACK_MANAGED_PATH` together with a game discoverable through Steam as app 605230.
- Network access for the initial pinned BepInEx download.

## Commands

From this folder:

```sh
dotnet run --project src -- game run [--bepinex 5|6] [--port N] [--test-port N] [--work-folder D] [--timeout S]
dotnet run --project src -- game start [--bepinex 5|6] [--port N] [--test-port N] [--work-folder D] [--timeout S] [--idle-timeout S]
dotnet run --project src -- game stop [--bepinex 5|6] [--work-folder D]
dotnet run --project src -- proxy [--port N]
dotnet test --project tests
```

`game host` is the internal detached-session host. Port 0 chooses a free port;
the default startup timeout is 120 seconds and shared sessions idle out after
15 minutes by default.

### Game sessions

The runner locks its work folder, clones the installed game, installs BepInEx
and Debug builds of the message hook and test-support plugin only in that
clone, and starts single player headlessly. It also gives the game's own server
a free port. The installed game's plugins, config and world are left alone.
The test plugin waits for the booted desktop, not merely an early player
connection, and mutes audio in batch mode.

Ready output is one stdout JSON line:

```json
{"port":N,"testPort":N,"log":"/path/to/player.log"}
```

`game run` stops when stdin closes or on SIGINT/SIGTERM. `game start` reuses a
detached session per work folder; `game stop` stops it. Diagnostics and failure
log tails go to stderr.

The Rust harness maintains `GREYHACK_TEST_COPIES` shared games (default 4),
with one cross-process `test.lock` per copy. Nextest's game group therefore
runs up to four tests concurrently, each with exclusive use of a separate
world. `GREYHACK_TEST_REUSE=0` selects an isolated child per test, and
`BEPINEX=6` selects BepInEx 6.

### Stdio proxy

`proxy` prints `{"port":N}` first. Each later stdout line is a client message
encoded as game JSON; each stdin line is a game message to decode and broadcast
to all clients. The current JSON is `method` plus positional `params`. stdin
EOF stops the proxy and logs go to stderr. See the
[proxy README](../greyhack-message-hook-proxy/README.md).

### Test-only bridge

The test plugin exposes a second listener and the standalone
`protocols/greyhack-test-support` protocol. `port` remains the message-hook
protocol; `testPort` is the fixture/setup/query/script bridge.

Requests have IDs repeated by one `ReplyRpc` on the originating connection.
Operations run on Unity's main thread, asynchronous operations may await game
events, and player setups are serialized. Failures return `Reply.TestFailure`
with the exception and stack; the Rust client exposes them as
`TestClientError::Bridge`. Valueless fixture successes return `Reply.Applied`.

The bridge provides:

- player replacement with tracked world restoration and network joining;
- script execution, queued/stepwise input and paired contexts;
- networks, libraries, routers, firewalls, accounts, economy, NPCs, clocks,
  traffic, appliances and player files;
- typed player, file and full-network state queries.

`setup_player` deletes and recreates the player computer with deterministic
fixture hardware and credentials. Each test still needs exclusive use of its
game copy; setup is reset, not a concurrency lock.

The plugin is installed only in test clones, never in a player's ordinary game.
Use [`greyhack-test-support-client`](../../rs/greyhack-test-support-client/README.md)
for its typed Rust API.
