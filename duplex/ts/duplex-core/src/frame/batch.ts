import { BinaryReader } from '../binary/reader.js';
import type { BinaryWriter } from '../binary/writer.js';
import { Varuint } from '../binary/varint.js';
import { ERR_CUT_SHORT, ERR_NO_SLICES, LAST_PIECE_FLAG, FRAGMENT_HEAD_BYTES, LAST_FRAME } from '../const.js';
import { FrameKind } from '../definition.generated.js';
import { CORE } from '../protocol/index.js';
import { SliceKey } from './route.js';

export interface Slice {
  readonly key: SliceKey;
  readonly start: number;
  readonly end: number;
}

export class BatchReader {
  readonly bytes: Uint8Array;
  readonly core: number;
  readonly #reader: BinaryReader;
  #slicesLeft: number;

  private constructor(bytes: Uint8Array) {
    this.bytes = bytes;

    const reader = new BinaryReader(bytes);

    this.core = reader.u8();
    this.#slicesLeft = reader.u32();
    if (this.#slicesLeft === 0) throw new RangeError(ERR_NO_SLICES);
    this.#reader = reader;
  }

  static of(data: unknown): BatchReader | undefined {
    const bytes = BatchReader.#bytesOf(data);

    if (bytes === undefined) return undefined;
    try {
      return new BatchReader(bytes);
    } catch {
      return undefined;
    }
  }

  static #bytesOf(data: unknown): Uint8Array | undefined {
    if (data instanceof Uint8Array) return data;
    if (data instanceof ArrayBuffer) return new Uint8Array(data);

    return undefined;
  }

  get isCurrentMajor(): boolean {
    return this.core === CORE.major;
  }

  next(): Slice | undefined {
    if (this.#slicesLeft === 0) return undefined;
    this.#slicesLeft--;

    const reader = this.#reader;
    const key = SliceKey.read(reader);
    const length = this.#slicesLeft === 0 ? reader.unreadLength : reader.length();
    const start = reader.offset;

    reader.skip(length);

    return { key, start, end: start + length };
  }
}

export class BatchWriter {
  readonly #writer: BinaryWriter;
  #slicesLeft: number;

  constructor(writer: BinaryWriter, core: number, slices: number) {
    writer.reset();
    writer.u8(core);
    writer.u32(slices);
    this.#writer = writer;
    this.#slicesLeft = slices;
  }

  slice(key: SliceKey, bytes: Uint8Array, start: number, end: number, lastFrameStart: number): void {
    this.#slicesLeft--;

    const writer = this.#writer;

    SliceKey.write(writer, key);

    const lastBodyStart = Varuint.getEnd(bytes, lastFrameStart + 1);

    if (lastBodyStart === undefined || lastBodyStart > end) throw new RangeError(ERR_CUT_SHORT);
    if (this.#slicesLeft > 0) writer.length(end - start - (lastBodyStart - lastFrameStart - 1));
    writer.rawRange(bytes, start, lastFrameStart);
    writer.byte(bytes[lastFrameStart]! | LAST_FRAME);
    writer.rawRange(bytes, lastBodyStart, end);
  }

  fragment(key: SliceKey, bytes: Uint8Array, start: number, end: number, isLastPiece: boolean): void {
    this.#slicesLeft--;

    const writer = this.#writer;

    SliceKey.write(writer, key);
    if (this.#slicesLeft > 0) writer.length(FRAGMENT_HEAD_BYTES + end - start);
    writer.byte(FrameKind.Fragment | LAST_FRAME);
    writer.byte(isLastPiece ? LAST_PIECE_FLAG : 0);
    writer.rawRange(bytes, start, end);
  }

  lastSlice(key: SliceKey, frames: Uint8Array): void {
    this.#slicesLeft--;
    SliceKey.write(this.#writer, key);
    this.#writer.raw(frames);
  }
}
