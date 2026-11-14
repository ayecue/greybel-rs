# Duplex core

## Scope

Duplex multiplexes versioned request/response and streaming protocols over message or stream transports.

Every core implementation provides:

- [`core/`](core/): binary layouts and error codes.
- This specification: transport and message behavior.
- [`scenarios/`](scenarios/): required executable behavior.
- [`conformance/fixtures.json`](conformance/fixtures.json): binary conformance fixtures.

The newest core version is normative. Every core version must remain able to read batch and frame headers, routes, and the opening handshake.

## Runtime

- A **driver** provides I/O, timers, randomness, and optional read pausing.
- A **transport** is one connection shared by channels. It authenticates and routes incoming batches and batches outgoing frames.
- A **hub** schedules all transports on its thread. A transport, its channels, and their handlers stay on that thread.
- A **relay** may connect a channel running on another thread or worker. It forwards only that channel's slices.

A relay follows the channel's current connection from forwarded frames: an `Ack` establishes it and a `Close` ends it. It closes the connection on each remaining side as follows:

| Event | Peer of the transport | Relayed side |
|---|---|---|
| Relayed side disappears | `ConnectionLoss` | Already gone |
| Transport closes the channel | Transport's reason | Transport's reason |
| Transport disconnects | Already disconnected | `ConnectionLoss` |

### Incoming processing

1. **Read:** Authenticate and parse each batch. Apply control frames, credit, cancellation, responses, and immediate rejections. Do not decode message bodies yet.
2. **Handle:** After reading the whole batch, decode, validate, and invoke handlers in arrival order. Process at most 1024 messages or 1 ms per task, fairly across transports.
3. **Write:** Encode responses directly into the transport outbox in write order. Later cancellation or timeout has no effect.

A response, rejection, or response stream item is delivered to its caller only after every notification the same peer sent before it has been handled. Credit, cancellation, and control frames remain immediate.

When the driver supports read pausing, pause transport reads above 4096 queued incoming messages and resume below 1024. A disconnected or insecure transport closes all its channels with that reason.

## Wire format

### Batches and slices

A batch contains `BatchHeader` followed by slices. Stream transports prefix it with a varuint length of at most four bytes. Batches are at most 64 MiB.

A slice contains frames for one `Route`: sender role, channel, and connection. The connection is `0` before establishment. The slice's varuint key selects its route:

| Key | Following data | Meaning |
|---|---|---|
| `0` | Full route | Unaliased route |
| `2a - 1` | Full route | Announce alias `a` |
| `2a` | Nothing | Use announced alias `a` |

A varuint slice length follows every slice except the final one, which ends with the batch.

Senders allocate the smallest available aliases, up to 8191; larger aliases are ignored. An alias may be reused only after the batch containing the old route's final frames. Aliases are sender-local and forgotten on disconnect.

### Frames and messages

A frame contains a one-byte `FrameKind`, a varuint payload length, and its payload. The final frame of a slice sets `0x80` on its kind and omits the length. Fragmented frames use `Fragment` payloads and are reassembled in route order.

Message bodies extend to the end of their payload and use the negotiated protocol minor. Exchange ids are varuints allocated by the message sender and scoped to its receiver. An exchange id remains allocated until its response settles. Exact frame payloads are declared in [`core/`](core/).

Apply malformed or unknown input as follows:

- Drop slices with unknown routes or aliases.
- Stop at malformed slices or frames, preserving frames already read.
- Skip unknown frame kinds.
- Require the current connection for message frames. Control frames follow the connection rules below.

## Sending and limits

- Group frames written during one task into batches by bundle and route while preserving per-channel order. Flush a batch at 64 KiB.
- Give each channel a 64 KiB share divided by the channels waiting in its bundle. Always admit its first frame; complete it when it fits the larger of its share and 16 KiB.
- Fragment larger frames across batches. Later frames from that channel wait for the final fragment.
- Preserve bundle order by oldest frame and channel order by first pending write. Closing flushes all batches.
- Queue messages until connected. Drop a notification newer than the negotiated minor; fail a request newer than it as `UnsupportedVersion`.
- Fail or reject traffic above configured limits as `Overload`.
- Fail new writes when at least 16 MiB is awaiting transport transmission.

Default channel limits are:

| Limit | Default |
|---|---:|
| Queued messages | 2048 |
| Pending messages | 16384 |
| Incoming messages | 16384 |
| Actively responding messages | 8192 |

Message frames default to a 64 MiB limit unless channel options or the transport flavour specify one. Control frames are exempt. This core `max_message` limit is distinct from the definition's default 64 KiB limit on client-sent message frames described in [`protocols/README.md`](../protocols/README.md#requirements-and-limits).

## Security

Secure transports use `Noise_XX_25519_ChaChaPoly_SHA256` (Noise revision 34), prologue `duplex`, and the core major. Handshake messages are slice-free batches. Static and ephemeral keys must be fresh and cryptographically random.

Identity payloads may contain an Ed25519 public key and a signature of `duplex identity` plus the handshake hash. Reject invalid, missing, or untrusted identities and mismatched batch-signing policy.

After the handshake:

- Keep `BatchHeader` clear and authenticate it as associated data.
- Encrypt the remainder with the handshake cipher and a per-batch counter nonce.
- When required, append an Ed25519 signature of `duplex batch` plus the batch.
- On handshake, decryption, or verification failure, close all channels as `Insecure` and send nothing more.

Protocols may require encryption, authenticated identities, or signed batches. Channels needing unavailable security close as `Insecure` before sending application data.

## Versions and connections

### Negotiation

A client sends its core and protocol versions and protocol fingerprint in `Open`. A provider serves its major and older majors reachable through migrations.

- Refuse incompatible majors or fingerprints as `VersionMismatch`, and unsupported minors as `UnsupportedVersion`.
- `Ack` selects the client's major and the older minor, includes its fingerprint, and reports deprecation.
- Clients reject an `Ack` they cannot speak as `VersionMismatch`.
- Providers migrate older-major traffic up on receipt and down on send.
- A protocol without a handshake accepts every client. Otherwise, the connect handler supplies the acceptance or denial payload.

### Lifetime

The client sends `Open`, retries it while opening, and queues application messages until `Ack`. The provider sends `Listen`, permits one pending open, and accepts or denies it through its connect handler. Repeated opens receive the existing `Ack`; another client's open receives `Busy`.

Connection ids come from driver randomness. Providers hash four little-endian u32 words with `hash = (hash + word) × 0x93d765dd mod 2³²`, rotate left 15 bits, and replace zero with 1.

`Close` ends the current connection with its reason. A provider returns to listening; a closed channel ignores further input. Driver disconnection closes locally without notifying the peer.

### Health checks

Health checks use these implemented defaults unless configured otherwise:

| Setting | Default |
|---|---:|
| `interval` | 5s |
| `unhealthy_threshold` | 2 |
| `close_threshold` | 3 |

Both peers run the following algorithm at `interval` while connected:

1. Treat anything received from the remote on the connection, including messages, `Ping`, and `Pong`, as activity. Activity resets the missed-check count to zero and marks activity since the previous check.
2. If a check finds activity since the previous check, clear that mark. Do not increment the missed-check count and do not send a `Ping`.
3. Otherwise, increment the missed-check count by one.
4. If the count is at most `close_threshold`, send a `Ping`. Every peer answers a `Ping` with a `Pong`.
5. A count greater than `unhealthy_threshold` marks the connection unhealthy. Hooks observe health changes.
6. A count greater than `close_threshold` closes the connection as `Timeout` instead of sending another `Ping`.

Both peers must check alike. Configure health checks in the definition's flavours rather than in only one peer's channel options: a peer without health checks never pings, and the other peer closes the connection after more than `close_threshold` consecutive checks without activity.

## Messages and streams

- Responses may be single values or streams. Declared failures produce `failed`; protocol rejections produce `rejected`.
- Closing fails pending responses and streams as `closed` and rejects new sends.
- Streams are credit-based. Initial credit is one item; receivers grant the remaining window at stream start and refill it when outstanding credit falls to half the window.
- Window size is bounded by 1 MiB divided by the maximum item size and by channel, protocol-flavour, or driver configuration. Defaults are 16384 for HTTP and 4096 otherwise. Existing grants remain valid if the window shrinks.
- Cancellation drops queued messages, fails sent requests as `Cancellation`, and sends `Cancel`. Receivers stop handling and discard later responses; already completed responses ignore it. Rejection releases the exchange id.
- Providers validate client messages, streams, and responses against declared types, requirements, limits, and item counts. Invalid requests are rejected with `Invalid` and a `Violation`; invalid notifications are dropped. Senders validate unless disabled; clients do not validate provider traffic.
- Timeouts come from the message definition or transport flavour. They measure from arrival or latest stream activity, excluding time waiting for response credit. Receivers reject timed-out requests as `Timeout`; senders cancel their own.
- Unsupported versions, malformed segments, duplicate pending ids, oversized traffic, and responses for ended connections are rejected, cancelled, or dropped according to their frame direction and the scenarios.
