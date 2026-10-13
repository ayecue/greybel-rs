# Testing against Grey Hack and the message proxy

The test harness has three entry points:

- `greyhack-test-support game` runs an isolated, headless copy of Grey Hack
  with the message hook and a test-only support plugin.
- `greyhack-test-support proxy` runs the message proxy without the game, using
  standard input/output as the game side.
- `greyhack-test-support-client` is the Rust client for the support plugin's
  separate fixture/setup/query/script protocol.

For installing and using the hook in your own game, see
[Grey Hack integration](greyhack-integration.md).

## Prerequisites

- macOS or Linux for the headless game runner.
- Grey Hack in a Steam library, with Steam running and logged in.
- The .NET 10 SDK.
- Rust and `cargo-nextest` for the commands below.
- Network access on the first run to download the pinned BepInEx release.

Set `GREY_HACK_PATH` to the game folder before running the tests. The .NET tool
can discover the game through Steam, but plugin builds require `GREY_HACK_PATH`
or `GREY_HACK_MANAGED_PATH`. BepInEx 5 is the default; set `BEPINEX=6` for Rust
game tests or pass `--bepinex 6` to the .NET tool. BepInEx 6 uses Rosetta on
Apple Silicon.

Run the configured repository game suites from the root, or focused suites from
`greybel/rs`:

```sh
make test-game

cargo nextest run -p greyhack-message-hook-client --features game
cargo nextest run -p greyhack-test-support-client --features game
cargo nextest run -p greyhack-test-support-client --features game --test main parity_
BEPINEX=6 cargo nextest run -p greyhack-message-hook-client --features game
```

## Headless .NET game runner

From `greybel/dotnet/greyhack-test-support`:

```sh
dotnet run --project src -- game run [--bepinex 5|6] [--port N] [--test-port N] [--work-folder D] [--timeout S]
dotnet run --project src -- game start [--bepinex 5|6] [--port N] [--test-port N] [--work-folder D] [--timeout S] [--idle-timeout S]
dotnet run --project src -- game stop [--bepinex 5|6] [--work-folder D]
```

`game host` is the detached host used by `game start`, not a separate frontend.
Port 0 selects a free port. The default work folder is the repository's
`target/greyhack-game-<major>` and the default startup timeout is 120 seconds.

The runner locks the work folder, clones the installed game into `game`, and
installs BepInEx plus Debug builds of the message hook and test-support plugin
in that clone. The clone has its own configuration and single-player database;
the installed game is not modified. macOS uses an APFS copy and Linux uses
`cp --reflink=auto`. The runner also selects a free port for the game's
single-player server so separate clones do not contend for the fixed game port.

Once the desktop has booted and both listeners accept connections, stdout gets
one JSON line:

```json
{"port":43123,"testPort":43124,"log":"/path/to/player.log"}
```

`port` serves `protocols/greyhack`; `testPort` serves the independent
`protocols/greyhack-test-support`. `game run` keeps the game alive until stdin
closes, SIGINT/SIGTERM arrives, or the game exits. Startup diagnostics and log
tails on failure go to stderr.

### Shared sessions and test concurrency

`game start` starts or reuses one detached session per work folder and prints
the same connection JSON. `game stop` stops the session in the selected work
folder. An idle session stops after 15 minutes unless `--idle-timeout` says
otherwise.

The Rust `greybel-test-support::game::Game` harness uses shared sessions by
default. It maintains up to `GREYHACK_TEST_COPIES` copies (default 4), named
`greyhack-game-<major>-<n>` below the test target folder. A cross-process
`test.lock` gives each test exclusive use of one copy, so nextest's `game`
group can run up to four tests concurrently without sharing a live world.
Tests from different processes and crates using the same target folder join
the same sessions.

Call `setup_player()` before a test's other operations. It serializes player
replacement, restores changes tracked from the previous test, deletes and
recreates the player computer, and installs the deterministic fixture. It does
not make concurrent use of one copy safe; the per-copy test lock provides that
isolation.

Set `GREYHACK_TEST_REUSE=0` to use an isolated `game run` child per test instead
of a detached shared session. Set `GREYHACK_TEST_COPIES=1` to reduce resource
use. Stop a manually created shared session by passing the same work folder:

```sh
dotnet run --project src -- game stop --bepinex 5 --work-folder /path/to/greyhack-game-5-0
```

## Proxy bridge without Grey Hack

From the same .NET project:

```sh
dotnet run --project src -- proxy [--port N]
```

The first stdout line is `{"port":N}`. After that, every client message to the
game is written as one game-JSON line. Write one game-JSON message per line to
stdin to broadcast it to every connected client. stderr carries logs and
closing stdin shuts the bridge down.

The JSON format is `{"method":ID,"params":[...]}` with positional parameters,
not the obsolete `ID`/`strValues` representation. For example, a test acting
as the game can answer a health-check request with the matching extension
message ID and its positional fields. Generated protocol metadata supplies the
IDs and parameter types.

Run the Rust client/proxy integration suite from `greybel/rs`:

```sh
cargo nextest run -p greyhack-message-hook-client --features dotnet
```

See the [proxy README](../greybel/dotnet/greyhack-message-hook-proxy/README.md)
for conversion semantics and the [wire-format overview](greyhack-integration.md#message-proxy-and-wire-format).

## Rust test-support client

Connect `TestClient` to `testPort`, not the message-hook `port`:

```rust
use greyhack_test_support_client::{TestClient, TestClientOptions};

# async fn example(test_port: u16) -> Result<(), Box<dyn std::error::Error>> {
let client = TestClient::connect(TestClientOptions::with_port(test_port)).await?;
let player = client.setup_player().await?;
let path = format!("/home/{}/example.txt", player.user);

client.write_file(&path, "hello").await?;
let state = client.file_state(&path).await?;
assert!(state.exists);
assert_eq!(state.content, "hello");

let execution = client
    .execute("print(\"hello\")", "/project/test.src", "/project", "test")
    .await?;
assert!(!execution.failed);

client.shutdown();
# Ok(())
# }
```

Requests carry IDs matched by `ReplyRpc`, so one connection can await multiple
replies. A late reply with no waiting request is dropped. Operations run on
Unity's main thread; asynchronous fixture and script operations may wait for
game activity, while player replacements are explicitly serialized.

A plugin exception becomes `Reply.TestFailure`, whose message contains the
error and stack. The Rust client maps that reply to
`TestClientError::Bridge(message)`. Fixture operations without a value return
`Reply.Applied`; state queries have typed replies such as `FileStateReply`.

The bridge can set up players and networks, execute scripts with queued or
stepwise input, arrange game fixtures, and query file/player/network state.
Normal message-hook builds and contexts still use `port`; this second protocol
is installed only in test clones. API details and regeneration commands are in
the [Rust client README](../greybel/rs/greyhack-test-support-client/README.md).

## Troubleshooting

- **No game found:** set `GREY_HACK_PATH` to the folder containing the game
  executable and data, not directly to `Assembly-CSharp.dll`.
- **Startup times out or exits before single player:** check that Steam is
  logged in and inspect the `log` path from the ready JSON or the stderr log
  tail. Increase `--timeout` for a slow environment.
- **A work folder stays busy:** another `game run`/host owns its lock. Stop that
  process, or use `game stop` for its shared session; choose another folder for
  an independent game.
- **BepInEx 6 fails on Apple Silicon:** ensure Rosetta is installed and retain
  the launcher's x86_64 preference, or use BepInEx 5.
- **Tests interfere with one another:** do not bypass the per-copy lock or share
  one `TestClient` game between independent tests. Recreate the player with
  `setup_player()` and use separate copies for concurrent tests.
- **Proxy JSON is rejected:** verify `method`, positional `params`, parameter
  CLR types and direction. Plugin and game IDs share a numeric range but have
  distinct meanings, and game IDs are direction-specific.
