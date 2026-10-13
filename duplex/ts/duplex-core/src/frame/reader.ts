import { BinaryReader } from '../binary/reader.js';
import { DECLARED_FAILURE, ERR_CUT_SHORT, FRAME_KIND_MASK, LAST_FRAME } from '../const.js';
import { Ack, Close, ErrorCode, FrameKind, Open, Refuse, type Version, Violation } from '../definition.generated.js';
import type { VersionNumber } from '../protocol/index.js';
import type { Control } from './control.js';
import { MessageKey } from './encoder.js';

export type SegmentKind =
  | FrameKind.ResponseEnd
  | FrameKind.ResponseStart
  | FrameKind.ResponseItem
  | FrameKind.MessageItem
  | FrameKind.MessageEnd;

export type Frame =
  | { kind: FrameKind.Message; typeId: number; id: number | undefined; body: BinaryReader }
  | { kind: SegmentKind; id: number; body: BinaryReader }
  | { kind: FrameKind.Failure; id: number; code: ErrorCode | undefined; violation?: Violation; body: BinaryReader }
  | { kind: FrameKind.ResponseCredit | FrameKind.MessageCredit; id: number; count: number }
  | { kind: FrameKind.Cancel; id: number }
  | { kind: FrameKind.Fragment; isLastPiece: boolean; bytes: Uint8Array }
  | Control;

export interface FrameHead {
  readonly kind: FrameKind;
  readonly length: number;
  readonly id: number | undefined;
}

export interface RawFrame {
  readonly kind: number;
  readonly start: number;
  readonly end: number;
}

export class FrameReader {
  readonly #reader: BinaryReader;
  readonly #end: number;
  coreMinor = BinaryReader.NEWEST;
  protocolMinor = BinaryReader.NEWEST;
  lastFrameStart = 0;
  lastFrameLength = 0;

  constructor(batch: Uint8Array, start: number, end: number) {
    this.#reader = new BinaryReader(batch, start, end);
    this.#end = end;
  }

  static isFrozen(frame: Frame): boolean {
    return frame.kind === FrameKind.Listen || frame.kind === FrameKind.Open || frame.kind === FrameKind.Refuse;
  }

  nextRaw(): RawFrame | undefined {
    const reader = this.#reader;

    reader.end = this.#end;
    if (reader.unreadLength === 0) return undefined;

    const kind = reader.byte();
    const length = (kind & LAST_FRAME) === 0 ? reader.length() : reader.unreadLength;
    const start = reader.offset;
    const end = start + length;

    if (end > this.#end) throw new RangeError(ERR_CUT_SHORT);
    reader.offset = end;

    return { kind: kind & FRAME_KIND_MASK, start, end };
  }

  next(): Frame | undefined {
    const reader = this.#reader;
    const sliceEnd = this.#end;

    for (;;) {
      reader.end = sliceEnd;
      if (reader.unreadLength === 0) return undefined;

      const start = reader.offset;
      const byte = reader.byte();
      const end = ((byte & LAST_FRAME) === 0 ? reader.length() : reader.unreadLength) + reader.offset;

      if (end > sliceEnd) throw new RangeError(ERR_CUT_SHORT);
      this.lastFrameStart = start;
      this.lastFrameLength = end - start;

      const kind = byte & FRAME_KIND_MASK;

      reader.end = end;
      reader.minor = FrameReader.isControl(kind) ? this.coreMinor : this.protocolMinor;

      const frame = FrameReader.#readFrame(kind, reader);

      if (frame !== undefined) return frame;
      reader.offset = end;
    }
  }

  skipFrame(): void {
    this.#reader.offset = this.#reader.end;
  }

  static #readFrame(kind: number, reader: BinaryReader): Frame | undefined {
    switch (kind) {
      case FrameKind.Message: {
        const key = reader.u32();

        return {
          kind,
          typeId: MessageKey.typeId(key),
          id: MessageKey.hasId(key) ? reader.u32() : undefined,
          body: reader
        };
      }

      case FrameKind.ResponseEnd:
      case FrameKind.ResponseStart:
      case FrameKind.ResponseItem:
      case FrameKind.MessageItem:
      case FrameKind.MessageEnd:
        return { kind, id: reader.u32(), body: reader };
      case FrameKind.Failure: {
        const id = reader.u32();
        const code = reader.u32();

        if (code === DECLARED_FAILURE) return { kind, id, code: undefined, body: reader };
        if (ErrorCode[code] === undefined) throw new RangeError(`no error code ${code}`);
        if (code === ErrorCode.Invalid) return { kind, id, code, violation: Violation.read(reader), body: reader };

        return { kind, id, code, body: reader };
      }

      case FrameKind.ResponseCredit:
      case FrameKind.MessageCredit: {
        const id = reader.u32();

        return { kind, id, count: reader.u32() };
      }

      case FrameKind.Cancel:
        return { kind, id: reader.u32() };
      case FrameKind.Fragment: {
        const isLastPiece = reader.boolean();

        return { kind, isLastPiece, bytes: reader.buffer.subarray(reader.offset, reader.end) };
      }

      case FrameKind.Listen:
      case FrameKind.Ping:
      case FrameKind.Pong:
        return { kind };
      case FrameKind.Open: {
        const { id, core, protocol, fingerprint } = Open.read(reader);

        return {
          kind,
          id,
          core: FrameReader.#toVersionNumber(core),
          protocol: FrameReader.#toVersionNumber(protocol),
          fingerprint,
          payload: FrameReader.#getRemainder(reader)
        };
      }

      case FrameKind.Ack: {
        const { id, connection, core, protocol, fingerprint, deprecation } = Ack.read(reader);

        return {
          kind,
          id,
          connection,
          core: FrameReader.#toVersionNumber(core),
          protocol: FrameReader.#toVersionNumber(protocol),
          fingerprint,
          deprecation,
          payload: FrameReader.#getRemainder(reader)
        };
      }

      case FrameKind.Refuse: {
        const { id, reason: code, core, protocol, majors } = Refuse.read(reader);
        const reason: ErrorCode = ErrorCode[code] === undefined ? ErrorCode.VersionMismatch : code;

        return {
          kind,
          id,
          reason,
          core: FrameReader.#toVersionNumber(core),
          protocol: FrameReader.#toVersionNumber(protocol),
          majors,
          payload: FrameReader.#getRemainder(reader)
        };
      }

      case FrameKind.Close:
        return { kind, reason: Close.read(reader).reason };
      default:
        return undefined;
    }
  }

  static isControl(kind: number): boolean {
    return kind >= FrameKind.Listen && kind <= FrameKind.Close;
  }

  static head(bytes: Uint8Array): FrameHead | undefined {
    const reader = new BinaryReader(bytes);
    let kind: number;
    let length: number;

    try {
      const byte = reader.byte();

      kind = byte & FRAME_KIND_MASK;
      length = ((byte & LAST_FRAME) === 0 ? reader.length() : reader.unreadLength) + reader.offset;
    } catch {
      return undefined;
    }

    if (FrameKind[kind] === undefined) return undefined;

    return { kind, length, id: FrameReader.#idAt(kind, reader) };
  }

  static #idAt(kind: number, reader: BinaryReader): number | undefined {
    try {
      if (kind !== FrameKind.Message)
        return FrameReader.isControl(kind) || kind === FrameKind.Fragment ? undefined : reader.u32();

      const key = reader.u32();

      return MessageKey.hasId(key) ? reader.u32() : undefined;
    } catch {
      return undefined;
    }
  }

  static #toVersionNumber(version: Version): VersionNumber {
    return { major: version.major, minor: version.minor };
  }

  static #getRemainder(reader: BinaryReader): Uint8Array {
    const remainder = reader.buffer.slice(reader.offset, reader.end);

    reader.offset = reader.end;

    return remainder;
  }
}
