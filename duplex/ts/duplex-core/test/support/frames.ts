import { BinaryWriter } from '../../src/binary/writer.js';
import { DECLARED_FAILURE, NO_CONNECTION } from '../../src/const.js';
import { FrameKind, type ErrorCode, type Role } from '../../src/definition.generated.js';
import { BatchReader, BatchWriter } from '../../src/frame/batch.js';
import type { Control } from '../../src/frame/control.js';
import { FrameEncoder } from '../../src/frame/encoder.js';
import { FrameReader, type SegmentKind } from '../../src/frame/reader.js';
import { SliceKeyKind, type SliceKey } from '../../src/frame/route.js';
import { InboundAliasTable } from '../../src/transport/aliases.js';
import { CORE } from '../../src/protocol/index.js';
import type { AnyMessage } from '../../src/protocol/message.js';

export type TestFrame =
  | { kind: FrameKind.Message; typeId: number; id: number | undefined; body: string }
  | { kind: SegmentKind; id: number; body: string }
  | { kind: FrameKind.Failure; id: number; code: ErrorCode | undefined; body: string }
  | { kind: FrameKind.ResponseCredit | FrameKind.MessageCredit; id: number; count: number }
  | { kind: FrameKind.Cancel; id: number }
  | { kind: FrameKind.Fragment; ends: boolean; bytes: string }
  | Control;

export interface TestBatch {
  channel: number;
  from: Role;
  connection: number;
  frames: TestFrame[];
}

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

export function unhex(text: string): Uint8Array {
  return Uint8Array.from(text.match(/../g) ?? [], (byte) => Number.parseInt(byte, 16));
}

export function written(write: (writer: BinaryWriter) => void): string {
  const writer = new BinaryWriter();

  write(writer);

  return hex(writer.asBytes());
}

export function body(message: AnyMessage): string {
  return written((writer) => message.write(writer));
}

export class TestAliases {
  readonly #aliases = new InboundAliasTable();

  read(posted: unknown): TestBatch[] {
    return this.readKeyed(posted).map(({ batch }) => batch);
  }

  readKeyed(posted: unknown): { key: SliceKey; batch: TestBatch }[] {
    const batch = BatchReader.of(posted);

    if (batch === undefined) throw new TypeError(`no batch: ${String(posted)}`);

    const slices: { key: SliceKey; batch: TestBatch }[] = [];

    for (let slice = batch.next(); slice !== undefined; slice = batch.next()) {
      const route = this.#aliases.getRoute(slice.key);

      if (route === undefined) throw new TypeError(`an alias never announced: ${JSON.stringify(slice.key)}`);

      const frames: TestFrame[] = [];
      const reader = new FrameReader(batch.bytes, slice.start, slice.end);

      for (let frame = reader.next(); frame !== undefined; frame = reader.next()) {
        if (frame.kind === FrameKind.Fragment) {
          frames.push({ kind: frame.kind, ends: frame.isLastPiece, bytes: hex(frame.bytes) });
        } else if (frame.kind === FrameKind.Failure && frame.violation !== undefined) {
          const { violation, ...failure } = frame;
          const tail = hex(frame.body.buffer.subarray(frame.body.offset, frame.body.end));

          frames.push({ ...failure, body: written((writer) => violation.write(writer)) + tail } as TestFrame);
        } else if ('body' in frame) {
          const tail = hex(frame.body.buffer.subarray(frame.body.offset, frame.body.end));

          frames.push({ ...frame, body: tail } as TestFrame);
        } else {
          frames.push(frame);
        }

        reader.skipFrame();
      }

      const { channel, from, connection } = route;

      slices.push({ key: slice.key, batch: { channel, from, connection: connection ?? NO_CONNECTION, frames } });
    }

    return slices;
  }
}

export function decode(posted: unknown): TestBatch {
  const slices = new TestAliases().read(posted);

  if (slices.length !== 1) throw new TypeError(`a batch of ${slices.length} slices`);

  return slices[0]!;
}

export interface UnknownFrame {
  readonly at: number;
  readonly kind: number;
  readonly payload: string;
}

export function encodedFrame(frame: TestFrame): Uint8Array {
  return encodeFrames([frame], []).writer.copyBytes();
}

export function encode(batch: TestBatch, unknown: readonly UnknownFrame[] = []): Uint8Array {
  const { writer, last } = encodeFrames(batch.frames, unknown);
  const output = new BinaryWriter();
  const route = {
    from: batch.from,
    channel: batch.channel,
    connection: batch.connection === NO_CONNECTION ? undefined : batch.connection
  };
  const slice = new BatchWriter(output, CORE.major, 1);

  if (last === undefined) throw new TypeError('a batch of no frames');
  slice.slice({ kind: SliceKeyKind.NoAlias, route }, writer.asBytes(), 0, writer.size, last);

  return output.copyBytes();
}

function encodeFrames(
  testFrames: readonly TestFrame[],
  unknown: readonly UnknownFrame[]
): { writer: BinaryWriter; last: number | undefined } {
  const writer = new BinaryWriter();
  const frames = new FrameEncoder(writer);
  const segment = (kind: FrameKind, id: number, body: string, failure = false) => {
    const payload = new BinaryWriter();

    payload.u32(id);
    if (failure) payload.u32(DECLARED_FAILURE);
    payload.raw(unhex(body));
    frames.rawFrame(kind, payload.asBytes());
  };

  const writeUnknown = (at: number) => {
    for (const frame of unknown.filter((frame) => frame.at === at)) frames.rawFrame(frame.kind, unhex(frame.payload));
  };

  for (const [index, frame] of testFrames.entries()) {
    writeUnknown(index);
    switch (frame.kind) {
      case FrameKind.Message:
        frames.encodedMessage(frame.typeId, frame.id, unhex(frame.body));
        break;
      case FrameKind.ResponseEnd:
      case FrameKind.ResponseStart:
      case FrameKind.ResponseItem:
      case FrameKind.MessageItem:
      case FrameKind.MessageEnd:
        segment(frame.kind, frame.id, frame.body);
        break;
      case FrameKind.Failure:
        if (frame.code === undefined) segment(FrameKind.Failure, frame.id, frame.body, true);
        else if (frame.body === '') frames.rejection(frame.id, frame.code);
        else {
          const payload = new BinaryWriter();

          payload.u32(frame.id);
          payload.u32(frame.code);
          payload.raw(unhex(frame.body));
          frames.rawFrame(FrameKind.Failure, payload.asBytes());
        }

        break;
      case FrameKind.ResponseCredit:
      case FrameKind.MessageCredit:
        frames.credit(frame.kind, frame.id, frame.count);
        break;
      case FrameKind.Cancel:
        frames.cancel(frame.id);
        break;
      case FrameKind.Fragment: {
        const bytes = unhex(frame.bytes);
        const payload = new Uint8Array(bytes.length + 1);

        payload[0] = frame.ends ? 1 : 0;
        payload.set(bytes, 1);
        frames.rawFrame(FrameKind.Fragment, payload);
        break;
      }

      default:
        frames.control(frame);
    }
  }

  writeUnknown(testFrames.length);

  return { writer, last: frames.lastFrameStart };
}
