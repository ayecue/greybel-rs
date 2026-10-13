# greyhack-test-support-client

Rust client for the test-only Grey Hack fixture/setup/query/script bridge. It
speaks `protocols/greyhack-test-support`, not the game's message-hook protocol.
The test-support plugin owns a second TCP listener; `greyhack-test-support
game run|start` reports it as `testPort`.

See [Game testing](../../../docs/game-testing.md) for runner prerequisites,
shared sessions and proxy tests.

## Use

Connect to `testPort` (select a fixed one with the runner's `--test-port`):

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

let network = client.create_network("45.45.45.45").await?;
assert_eq!(network.public_ip, "45.45.45.45");
client.shutdown();
# Ok(())
# }
```

`file_state` returns `FileStateReply`. `Reply.TestFailure` becomes
`TestClientError::Bridge`, with the plugin's error and stack in its message.
Requests are correlated by ID, unmatched late replies are dropped, and
operations run on Unity's main thread. Player setups are serialized.

`setup_player()` recreates the computer with username and password `test`/`test`,
machine name `fixture`, `tuto_ok = false`, `enable_gift = true` and 4096 MB RAM. `connect_network`
joins through the game's Wi-Fi RPCs. Later scripts run as the fixture user.
The bridge exists only in isolated test games.

## Test and regenerate

From `greybel/rs`:

```sh
cargo test -p greyhack-test-support-client --features game
cargo test -p greyhack-test-support-client --features game --test main parity_
UPDATE_CS=1 cargo test -p greyhack-test-support-client --test main cs::
```

Game tests need .NET, Grey Hack and logged-in Steam. By default, tests share up
to `GREYHACK_TEST_COPIES` sessions (default 4), each reserved by a cross-process
lock. Set `GREYHACK_TEST_REUSE=0` for isolated children and `BEPINEX=6` for
BepInEx 6. Stop a shared session with `greyhack-test-support game stop` and the
same work folder; idle sessions also expire.

The [.NET harness README](../../dotnet/greyhack-test-support/README.md) lists
the runner commands and bridge constraints.
