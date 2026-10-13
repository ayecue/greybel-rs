# Grey Hack contracts

Language-neutral descriptions of the game's `NetworkMessages.RpcMessage` JSON
messages and the message hook's extensions. A wire message has a numeric
`method` and positional `params`; field order and CLR types come primarily from
the receiver's `GetParam<T>()` calls, not the old `strValues`/`Get*` model.

| File | Contents |
|---|---|
| `game.bundle` | Bundled `game.json`, extracted from the installed game's `Assembly-CSharp.dll`. |
| `fingerprint.json` | Game version and SHA-256 hashes of the bundle's unpacked files. |
| `extension.json` | Hand-written plugin messages for contexts, builds and status; extension IDs start at 1000. |

From `greybel/dotnet`:

```sh
make extract   # Requires GREY_HACK_PATH or GREY_HACK_MANAGED_PATH
make generate  # Writes protocols/greyhack from game.bundle + extension.json
make contracts # Both
```

Set `PROTOCOL_VERSION=<major>.<minor>` when generating another protocol version.
Only additive changes belong in a minor version; changed or removed messages
require a new major.

See [`greyhack-util`](../dotnet/greyhack-util/README.md) for extraction rules,
the [wire-format guide](../../docs/greyhack-integration.md#message-proxy-and-wire-format),
and [`protocols/greyhack`](../../protocols/greyhack) for generated output.
