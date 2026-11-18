# greyhack-util

Extracts Grey Hack's RPC contracts, world data and terminal text assets, and
generates `protocols/greyhack`. Contract and asset outputs are bundles with a
`fingerprint.json` recording the game version and SHA-256 of each unpacked file.

## Prerequisites and commands

Use the .NET 10 SDK and an installed game. `GREY_HACK_PATH` names its folder;
`GREY_HACK_MANAGED_PATH` can name `Managed` directly, and `GREY_HACK_DATA` can
name `Grey Hack_Data` or macOS's `Contents/Resources/Data`.

From this folder:

```sh
dotnet run --project src -- extract <Assembly-CSharp.dll> --output <contracts-folder>
dotnet run --project src -- generate --contracts <contracts-folder> --output <major>.<minor>.protocol
dotnet run --project src -- assets <data-folder> --world <world-assets-folder> --text-mesh <text-mesh-assets-folder> --cache <download-folder>
dotnet test --solution GreyHackUtil.slnx
```

From `greybel/dotnet`, the repository-path equivalents are:

```sh
make extract
make generate                 # PROTOCOL_VERSION=1.0 by default
make contracts                # extract + generate
make assets
make test
```

`make assets` writes `assets.bundle` and `fingerprint.json` into
`assets/rs/greyhack-world-assets` and `assets/rs/greyhack-text-mesh-assets`.
Downloads are cached in `target/greyhack-util`.

## Contract extraction

`extract` writes `game.bundle` containing
`game.json`, plus its fingerprint. The current game API is
`NetworkMessages.RpcMessage`, serialized as JSON `method` and positional
`params`:

- `fieldTypes` catalogs the CLR types passed to `RpcMessage.GetParam<T>()`:
  scalar and byte/list values, enums and structs. Nullable and trailing
  optional parameters are recorded separately.
- `server` and `client` contain `IdServer` and `IdClient` messages. Receiver
  layouts come from the ordered `GetParam<T>()` calls in `PlayerServer` /
  `PlayerClient.ProcessReceivedData` (and the login receiver). Names come from
  the receiving handler's parameters or local assignments.
- `senders` supplies layouts when a receiver does not. The extractor finds
  `MessageServer` / `MessageClient` construction and reads the constructor's
  positional arguments; there are no `Add*` calls to follow. Conditional or
  inconsistent layouts can produce a warning and no contract layout. Senders
  of receiver-defined messages are checked for compatible coverage.
- `optional` marks a conditionally read trailing parameter. A conditional read
  before a required parameter is rejected because positional omission would be
  ambiguous.

The receiver is authoritative for order and types. Messages with neither a
usable receiver nor sender layout, currently `AdminAction` and
`DebugKickServerRpc`, have no fields and are omitted from the generated typed
protocol.

### Window PID detection

At most one field is tagged `window`, later emitted as `@pid`. It can be one
integer PID, an integer list, or a compressed JSON PID list. Detection tries,
in order:

1. a client receiver value passed to `PlayerClientMethods.GetVentana`, traced
   through locals, loops and payload decoding;
2. a client-sent field every sender fills with its own window PID;
3. a field every sender fills with some window PID, including values traced
   through game methods and callers;
4. a client-sent field the server handler reuses as the window for replies.

Ambiguous or disagreeing matches produce no window tag and a warning. Plugin
messages declare their window fields in `extension.json`.

## Protocol generation

`generate` loads `game.bundle` and the hand-written `extension.json`, validates
IDs and names per direction, and writes one duplex protocol. Server-bound game
messages belong to provider `MessageHook`; client-bound messages belong to
client `Greybel`. All are notifications, with fields in contract order and a
16 MiB protocol limit.

Metadata carries the game mapping:

- `game_version`, `game_message_id`, `plugin_message_id`;
- `game_param_type` for the CLR type read from each positional parameter;
- `game_param_optional` for absent trailing parameters, distinct from null;
- `pid` and `game_field_name` for window routing and struct JSON.

The proxy and Rust client consume these tags. Regenerate the proxy's C# bindings
from `greybel/rs` after generation:

```sh
UPDATE_CS=1 cargo test -p greyhack-message-hook-client --test main cs::
```

Message IDs in the duplex protocol are positions in their role blocks. For an
addition-only minor version, the writer preserves the previous major's order
and appends new messages. Removed messages or other incompatible changes fail
generation and require a new major; select it with, for example,
`make generate PROTOCOL_VERSION=2.0`.

See the [contracts README](../../contracts/README.md) and
[wire-format guide](../../../docs/greyhack-integration.md#message-proxy-and-wire-format).

## Asset extraction

`assets` reads serialized game files with AssetsTools.NET. Engine types use a
class package built from the Unity version's AssetRipper type-tree dump,
downloaded at a pinned commit and checked against its recorded SHA-256;
MonoBehaviour types come from the game's `Managed` assemblies. A new Unity
version requires a matching pinned dump and hash.

- **World bundle:** command scripts, deterministic shuffles of word lists,
  English texts, base filesystem/service configuration, SSH scripts and the
  default website.
- **Text-mesh bundle:** terminal TextMesh Pro layout/theme settings, fallback
  fonts, font atlases as alpha bytes, sprite RGBA data and original font files.
  Publisher font downloads are cached and checked against pinned SHA-256 hashes.

The [asset-bundle library](../../../assets/dotnet/README.md) defines the bundle
and fingerprint format. Review fingerprints after game updates to see content
changes without committing game text as clear text.
