import type { BinaryWriter } from '../binary/writer.js';
import { ChannelError } from '../errors.js';
import { DECLARED_FAILURE, ID_FLAG, LENGTH_PLACEHOLDER } from '../const.js';
import {
  Ack,
  Close,
  FrameKind,
  Open,
  Refuse,
  Version,
  type ErrorCode,
  Requirement,
  Violation
} from '../definition.generated.js';
import type { VersionNumber } from '../protocol/index.js';
import { MessageSegment, SegmentCodec, type IMessage } from '../protocol/message.js';
import type { Control } from './control.js';

export const MessageKey = {
  of(typeId: number, hasId: boolean): number {
    return typeId * 2 + (hasId ? ID_FLAG : 0);
  },

  typeId(key: number): number {
    return Math.floor(key / 2);
  },

  hasId(key: number): boolean {
    return key % 2 === ID_FLAG;
  }
};

interface Writable {
  write(writer: BinaryWriter): void;
}

export class FrameEncoder {
  lastFrameStart: number | undefined;
  readonly #writer: BinaryWriter;
  #limit = Number.POSITIVE_INFINITY;

  constructor(writer: BinaryWriter) {
    this.#writer = writer;
  }

  get minor(): number {
    return this.#writer.minor;
  }

  set minor(minor: number) {
    this.#writer.minor = minor;
  }

  validate(validating: boolean, limit: number): void {
    this.#writer.validate(validating);
    this.#limit = limit;
  }

  rawFrame(kind: number, payload: Uint8Array): void {
    const writer = this.#writer;
    const start = writer.size;

    writer.byte(kind);
    writer.length(payload.length);
    writer.raw(payload);
    this.lastFrameStart = start;
  }

  message(typeId: number, id: number | undefined, body: Writable): void {
    const writer = this.#writer;
    const start = this.#startFrame(FrameKind.Message);

    try {
      writer.u32(MessageKey.of(typeId, id !== undefined));
      if (id !== undefined) writer.u32(id);

      const bodyStart = writer.size;

      body.write(writer);
      this.#enforceLimit(bodyStart);
    } catch (error) {
      writer.truncate(start);
      throw error;
    }

    this.#finishFrame(start);
  }

  encodedMessage(typeId: number, id: number | undefined, body: Uint8Array): void {
    const writer = this.#writer;
    const start = this.#startFrame(FrameKind.Message);

    writer.u32(MessageKey.of(typeId, id !== undefined));
    if (id !== undefined) writer.u32(id);
    writer.raw(body);
    this.#finishFrame(start);
  }

  segment(segment: MessageSegment, id: number, type: IMessage, value: unknown): void {
    const writer = this.#writer;
    const start = this.#startFrame(FrameEncoder.#kindOf(segment));

    try {
      writer.u32(id);
      if (segment === MessageSegment.Failure) writer.u32(DECLARED_FAILURE);

      const body = writer.size;

      SegmentCodec.write(type, segment, writer, value);
      this.#enforceLimit(body);
    } catch (error) {
      writer.truncate(start);
      throw error;
    }

    this.#finishFrame(start);
  }

  credit(kind: FrameKind.ResponseCredit | FrameKind.MessageCredit, id: number, count: number): void {
    const start = this.#startFrame(kind);

    this.#writer.u32(id);
    this.#writer.u32(count);
    this.#finishFrame(start);
  }

  rejection(id: number, code: ErrorCode, violation?: Violation): void {
    const start = this.#startFrame(FrameKind.Failure);

    this.#writer.u32(id);
    this.#writer.u32(code);
    violation?.write(this.#writer);
    this.#finishFrame(start);
  }

  cancel(id: number): void {
    const start = this.#startFrame(FrameKind.Cancel);

    this.#writer.u32(id);
    this.#finishFrame(start);
  }

  control(control: Control): void {
    const writer = this.#writer;
    const start = this.#startFrame(control.kind);

    try {
      FrameEncoder.#writeControl(writer, control);
    } catch (error) {
      writer.truncate(start);
      throw error;
    }

    this.#finishFrame(start);
  }

  #enforceLimit(bodyStart: number): void {
    if (this.#writer.size - bodyStart > this.#limit)
      throw ChannelError.invalid(new Violation({ path: [], requirement: Requirement.Limit }));
  }

  #startFrame(kind: FrameKind): number {
    const writer = this.#writer;
    const start = writer.size;

    writer.byte(kind);
    writer.byte(LENGTH_PLACEHOLDER);

    return start;
  }

  #finishFrame(start: number): void {
    this.#writer.finishLength(start + 1);
    this.lastFrameStart = start;
  }

  static #writeControl(writer: BinaryWriter, control: Control): void {
    switch (control.kind) {
      case FrameKind.Open:
        new Open({
          id: control.id,
          core: FrameEncoder.#toVersion(control.core),
          protocol: FrameEncoder.#toVersion(control.protocol),
          fingerprint: control.fingerprint
        }).write(writer);
        writer.raw(control.payload);
        break;
      case FrameKind.Ack:
        new Ack({
          id: control.id,
          connection: control.connection,
          core: FrameEncoder.#toVersion(control.core),
          protocol: FrameEncoder.#toVersion(control.protocol),
          fingerprint: control.fingerprint,
          deprecation: control.deprecation
        }).write(writer);
        writer.raw(control.payload);
        break;
      case FrameKind.Refuse:
        new Refuse({
          id: control.id,
          reason: control.reason,
          core: FrameEncoder.#toVersion(control.core),
          protocol: FrameEncoder.#toVersion(control.protocol),
          majors: control.majors
        }).write(writer);
        writer.raw(control.payload);
        break;
      case FrameKind.Close:
        new Close({ reason: control.reason }).write(writer);
        break;
      default:
        break;
    }
  }

  static #kindOf(segment: MessageSegment): FrameKind {
    switch (segment) {
      case MessageSegment.Message:
        throw new TypeError('a message is written by `message`');
      case MessageSegment.MessageItem:
        return FrameKind.MessageItem;
      case MessageSegment.MessageEnd:
        return FrameKind.MessageEnd;
      case MessageSegment.ResponseEnd:
        return FrameKind.ResponseEnd;
      case MessageSegment.ResponseStart:
        return FrameKind.ResponseStart;
      case MessageSegment.ResponseItem:
        return FrameKind.ResponseItem;
      case MessageSegment.Failure:
        return FrameKind.Failure;
    }
  }

  static #toVersion(version: VersionNumber): Version {
    return new Version({ major: version.major, minor: version.minor });
  }
}
