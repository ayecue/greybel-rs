import { BinaryReader } from '../binary/reader.js';
import { BinaryWriter } from '../binary/writer.js';
import type { Bundle } from '../channel/options.js';
import { ONE_SLICE } from '../const.js';
import { Ack, ErrorCode, FrameKind, Role } from '../definition.generated.js';
import { FrameEncoder } from '../frame/encoder.js';
import { BatchReader, BatchWriter, type Slice } from '../frame/batch.js';
import { FrameReader } from '../frame/reader.js';
import { SliceKeyKind, type Route } from '../frame/route.js';
import { CORE } from '../protocol/index.js';
import { InboundAliasTable } from './aliases.js';
import type { Endpoint, InboundSlice } from './endpoints.js';
import type { Transport } from './index.js';

export class Relay implements Endpoint {
  readonly port: MessagePort;
  readonly #transport: Transport;
  readonly #channel: number;
  readonly #role: Role;
  readonly #remote: Role;
  readonly #own: MessagePort;
  readonly #bundle: Bundle;
  readonly #detach: () => void;
  readonly #aliases = new InboundAliasTable();
  readonly #batchWriter = new BinaryWriter();
  #connection: number | undefined;
  #isClosed = false;

  constructor(transport: Transport, channel: number, role: Role, bundle: Bundle) {
    const { port1, port2 } = new MessageChannel();

    this.port = port2;
    this.#transport = transport;
    this.#channel = channel;
    this.#role = role;
    this.#remote = role === Role.Client ? Role.Provider : Role.Client;
    this.#own = port1;
    this.#bundle = bundle;
    this.#detach = transport.attach(channel, role, this);
    port1.onmessage = ({ data }: MessageEvent) => this.#forward(data);
  }

  receive({ batch, slice, connection, core }: InboundSlice): void {
    const writer = new BatchWriter(this.#batchWriter, core, ONE_SLICE);

    this.#observe(connection, batch, slice.start, slice.end);
    writer.lastSlice(
      { kind: SliceKeyKind.NoAlias, route: { from: this.#remote, channel: this.#channel, connection } },
      batch.subarray(slice.start, slice.end)
    );
    this.#post();
  }

  onDisconnect(): void {
    this.#end(ErrorCode.ConnectionLoss, false);
  }

  close(reason: ErrorCode): void {
    this.#end(reason, true);
  }

  #end(reason: ErrorCode, notifiesRemote: boolean): void {
    if (this.#isClosed) return;
    this.#isClosed = true;

    const connection = this.#connection;

    this.#connection = undefined;
    if (connection !== undefined) {
      if (notifiesRemote) this.#closeRemote(connection, reason);
      this.#closePort(connection, reason);
    }

    this.#own.close();
    this.#detach();
  }

  #closeRemote(connection: number, reason: ErrorCode): void {
    const route = { from: this.#role, channel: this.#channel, connection };

    try {
      this.#transport.begin(route, this.#bundle, BinaryWriter.NEWEST).control({ kind: FrameKind.Close, reason });
    } finally {
      this.#transport.commit();
    }
  }

  #closePort(connection: number, reason: ErrorCode): void {
    const frames = new BinaryWriter();
    const encoder = new FrameEncoder(frames);

    encoder.control({ kind: FrameKind.Close, reason });
    new BatchWriter(this.#batchWriter, CORE.major, ONE_SLICE).slice(
      { kind: SliceKeyKind.NoAlias, route: { from: this.#remote, channel: this.#channel, connection } },
      frames.asBytes(),
      0,
      frames.size,
      encoder.lastFrameStart!
    );
    this.#post();
  }

  #post(): void {
    const bytes = this.#batchWriter.copyBytes();

    this.#own.postMessage(bytes, [bytes.buffer]);
  }

  #observe(connection: number | undefined, bytes: Uint8Array, start: number, end: number): void {
    if (connection !== undefined) this.#connection = connection;

    const reader = new FrameReader(bytes, start, end);

    for (let frame = reader.nextRaw(); frame !== undefined; frame = reader.nextRaw()) {
      if (frame.kind === FrameKind.Ack) {
        this.#connection = Ack.read(new BinaryReader(bytes, frame.start, frame.end)).connection;
      } else if (frame.kind === FrameKind.Close) this.#connection = undefined;
    }
  }

  #forward(data: unknown): void {
    const batch = BatchReader.of(data);

    if (batch === undefined || !batch.isCurrentMajor) return;
    try {
      for (let slice = batch.next(); slice !== undefined; slice = batch.next()) {
        const route = this.#aliases.getRoute(slice.key);

        if (route === undefined || route.channel !== this.#channel || route.from !== this.#role) continue;
        this.#observe(route.connection, batch.bytes, slice.start, slice.end);
        this.#forwardSlice(batch.bytes, slice, route);
      }
    } catch {
      return;
    }
  }

  #forwardSlice(bytes: Uint8Array, slice: Slice, route: Route): void {
    const reader = new FrameReader(bytes, slice.start, slice.end);
    const encoder = this.#transport.begin(route, this.#bundle, BinaryWriter.NEWEST);

    try {
      for (let frame = reader.nextRaw(); frame !== undefined; frame = reader.nextRaw()) {
        encoder.rawFrame(frame.kind, bytes.subarray(frame.start, frame.end));
      }
    } finally {
      this.#transport.commit();
    }
  }
}
