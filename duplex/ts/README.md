# Duplex TypeScript packages

This pnpm workspace contains the TypeScript core, generated-code support, and client drivers. Protocol syntax and compatibility rules are defined in [`protocols/README.md`](../../protocols/README.md); runtime behavior is normative in [`duplex/SPEC.md`](../SPEC.md).

## Packages

| Package | Purpose |
|---|---|
| [`duplex-core`](duplex-core/) | Client and provider channels over any driver, plus the binary readers and writers used by generated code |
| [`duplex-driver-worker`](duplex-driver-worker/) | `MessageTargetDriver` for one batch per `postMessage`, and `WorkerHost`/`WorkerScope` for worker-owned channels and relays |
| [`duplex-driver-stdio`](duplex-driver-stdio/) | Node `StreamDriver` for length-prefixed batches over standard input/output, child processes, and TCP connections |
| [`duplex-driver-http`](duplex-driver-http/) | Browser and Node `HttpDriver` client; POST batches upstream and long polls, server-sent events, or a streamed binary body downstream |
| [`duplex-driver-websocket`](duplex-driver-websocket/) | Browser and Node `WebSocketDriver` client; one binary WebSocket message per batch |

The workspace uses `emitter`, `handle-error`, and `presets` from [`common/ts`](../../common/ts/) through the overrides in [`pnpm-workspace.yaml`](pnpm-workspace.yaml). Install that workspace first, then run:

```sh
pnpm install
pnpm test        # Vitest in every package
pnpm typecheck
pnpm build       # dist/ in every package
```

Packages import each other's sources through the `source` export condition, so tests and typechecks do not require a build. Vitest obtains external folders from `inject(name)` using [`config.ts`](config.ts).

## Generated modules

Define a protocol once in a [definition folder](../../protocols/README.md). The `duplex-ts` crate generates a protocol/types module and a client- or provider-role module from a selected major. Rust's `include_protocol!` and the TypeScript generator consume the same resolved plan; both implementations also verify the shared [conformance fixtures](../conformance/fixtures.json).

A generated module contains:

| Declaration | API |
|---|---|
| Struct | A class constructed from its fields, such as `new Step({ name, result })`, with a getter per field |
| Union | An abstract base class and one class per variant in its namespace, such as `new Output.Print({ ... })`; use `instanceof` to discriminate variants |
| Enum | A numeric TypeScript `enum` |
| Binary codecs | `value.write(writer)` and `Type.read(reader)` for structs, unions, and messages |
| Message | A `Message<Sender, Fields, Exchange>` subclass with static `id`, `since`, and `sender`; its exchange is `ValueResponse<R, F>`, `StreamResponse<Start, End, Item, F>`, `StreamExchange<End, Item, F>`, or empty for a notification. Message classes also write and read their exchange segments |
| Peer enum | Protocol peers with their role numbers, such as `IoPeer.Program = 0` and `IoPeer.Page = 1` |
| Protocol | Channel id, versions, fingerprints, deprecation status, peers, messages by receiver, capabilities, and optional handshake |
| Router | A method named `on<Message>` for each handled message; unhandled calls continue through `next()` |
| Channel | A channel class constructed over a `Transport` or `Driver`, with optional `ChannelOptions` and, for a client handshake, an opening payload |

All 64-bit integers use `bigint`; other numbers use `number`, and bytes use `Uint8Array`. Construct a role channel as `new PageChannel(transport, options)`, or pass a declared client opening payload as `new MemberChannel(transport, open)`.

### Channels and streams

```ts
const user = new UserChannel(driver, { healthCheck: false });
const sum = await user.send(new Add({ a: 1n, b: 2n }));
```

`channel.send(message)` returns a value based on the message exchange:

| Exchange | Return value |
|---|---|
| Notification | `void` |
| Single response | `Promise<Response>` |
| Streamed response | `ResponseStream`, with `start()`, async iteration over items, and `end()` |
| Message with an outgoing stream | `{ stream, response }`; use `stream.send(item)`, `stream.sendAll(items)`, and `await stream.end(value)` |

```ts
const response = user.send(new Count({ to: 3 }));
const phase = await response.start();
for await (const item of response) console.log(item);
const total = await response.end();
```

For a streamed response handler, `RouteContext.responseWriter` provides `start`, `send`, `sendAll`, and `finish`. Declare its start before sending items when the response has one, and return the response end from the handler. For an incoming message stream, iterate `RouteContext.incomingStream` and call `end()` for its end value.

At most 4096 items may wait for receiver credit in a TypeScript stream outbox. Beyond that, `send` rejects as `Overload`; await each `send` or use `sendAll`. The core credit, timeout, cancellation, and limit rules are in [`SPEC.md`](../SPEC.md#messages-and-streams).

Pass `{ signal }` as the send options to cancel a message. A request handler observes cancellation through `RouteContext.signal`. Queued messages wait for connection establishment and use the negotiated minor.

### Connection state and errors

Connection state is represented by `ConnectionInit`, `ConnectionActive`, `ConnectionFailure`, or `ConnectionEnd`:

- `ConnectionActive` exposes `ack`, `version`, and `deprecation`.
- `ConnectionFailure` exposes `reason` and `refusal`.
- `onOpen`, `onAck`, `onFailure`, and `onClose` run when the corresponding state is entered; `onHealth` reports health changes.
- Channel startup is deferred one task, so hooks registered immediately after construction observe the first state. A throwing hook does not stop the channel or other hooks.
- `closeEvent` resolves with the channel's final close reason.

A provider router accepts a declared handshake in `onConnect` by returning its acknowledgement payload. It denies the opening by throwing `ChannelError.deny(refusal)`. Protocols without a handshake accept compatible clients automatically.

Response errors are `ChannelError` values:

| `ChannelErrorKind` | Meaning |
|---|---|
| `Failure` | Declared failure in `failure`; handlers create it with `ChannelError.fail(failure)` |
| `Denial` | Handshake denial with its refusal payload |
| `Rejection` | Protocol rejection with an `ErrorCode` |
| `ChannelEnd` | Connection or channel termination with an `ErrorCode` |
| `Unavailable` | The negotiated version cannot carry the value or message |
| `Invalid` | Validation failed with a `Violation` |

### Options, transports, and relays

`ChannelOptions` controls health checks, limits, stream window, maximum message size, validation, and batching. Use `{ bundle: Bundle.key(1) }` to assign a channel or message to a batching bundle. Channels sharing a driver also share its `Transport`, and every transport in a realm shares `Hub.current`. Hubs schedule inbound handling and outbound writes according to the [runtime specification](../SPEC.md#runtime).

Construct a `Transport` with `{ security }` to enable the Noise handshake, encrypted batches, Ed25519 identities, and optional signed batches. The security contract and failure behavior are in [`SPEC.md`](../SPEC.md#security).

`transport.relay(protocol, peer)` returns a relay with a `MessagePort` for a channel in another worker. It follows and closes only that channel's connection as specified in [`SPEC.md`](../SPEC.md#runtime).

### Migrations

`duplex_ts::Module::migration(older, newer, olderModule, newerModule)` generates an abstract migration class for consecutive majors. Identical segments have default rewrites; override generated methods for incompatible segments.

Register a migration on a provider channel:

```ts
// Upgrade extends CalculationMigration1 and implements its required rewrites.
channel.addMigration(new Upgrade());
```

The provider can then serve the older major through the migration path defined in [`protocols/README.md`](../../protocols/README.md#migrations).

### Tags

Generated declarations expose definition tags without affecting their binary layout:

| Definition item | TypeScript metadata |
|---|---|
| Protocol, message, struct, union, or enum | Static `tags`; enum tags are in the enum's namespace |
| Fields of a message, struct, or variant | Static `fields`, each `{ name, property, tags }` |
| Union variant | The variant class's `fields` and `tags` |
| Enum member | The enum namespace's `members` |

Use `findTag`, `findAllTags`, and `fieldWithTag` from `duplex-core` to query this metadata. Tag declaration rules are canonical in [`protocols/README.md`](../../protocols/README.md#tags).

## Worker channels

`WorkerHost` declares each channel once and exposes the typed result as `host.channels`:

```ts
import { WorkerHost } from 'duplex-driver-worker';
import { KeyboardInput, PageChannel, ScreenOutput } from 'greybel-io-web';

const host = new WorkerHost(new Worker(url, { type: 'module' }), {
  io: {
    channelClass: PageChannel,
    routers: [new ScreenOutput((screen) => render(screen.lines)), new KeyboardInput(window)]
  }
});
host.channels.io.onHealth((health) => showHealth(health));
host.channels.io.closeEvent.then((reason) => showFailure(reason));
```

The host sends its own `MessagePort` first, followed by ports for relays declared with `{ relays: [{ transport, protocol, peer }] }`. The message data is a `Uint32Array` of relayed channel ids. `host.transport` can also carry channels not listed in the declarations. Closing the host or a worker failure closes declared and relayed connections with the host's reason.

In the worker, call `await WorkerScope.accept(self)` while the script starts. Open a channel over `scope.transportOf(protocol)` to select its relayed transport or fall back to `scope.transport`. Rust workers use the equivalent `duplex-driver-web::WorkerScope::accept()` API.

## Regenerating checked-in modules

Generator tests fail when checked-in modules are stale. From `duplex/rs`, regenerate a crate's TypeScript with:

```sh
UPDATE_TS=1 cargo test -p <crate>
```

| Generated files | Owning crate |
|---|---|
| `duplex-core/src/definition.generated.ts` from `duplex/core` | `duplex-core` |
| `duplex-core/test/*.generated.ts` | `duplex-core` |
| `duplex-driver-worker/test/support/*.generated.ts` | `duplex-ts` |
| `duplex-driver-stdio/test/calculator.client.generated.ts` | `duplex-driver-stdio` |
| `duplex-driver-http/test/calculator.client.generated.ts` | `duplex-driver-http` |
| `duplex-driver-websocket/test/calculator.client.generated.ts` | `duplex-driver-websocket` |
