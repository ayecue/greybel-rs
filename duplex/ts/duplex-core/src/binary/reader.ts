import {
  ASCII_LIMIT,
  BIG_ONE,
  BIG_SHIFT,
  BIG_WORD_BITS,
  BYTE_BITS,
  CONTINUATION,
  ERR_INVALID_BOOLEAN,
  ERR_INVALID_CHARACTER,
  ERR_INVALID_FLAGS,
  ERR_PROTO_KEY,
  ERR_TOO_LARGE,
  ERR_TRAILING_INPUT,
  ERR_UNEXPECTED_END,
  SAFE_VARUINT_BYTES,
  F32_BYTES,
  F64_BYTES,
  I16_MAX,
  I16_MIN,
  I32_MAX,
  I32_MIN,
  I8_SIGN_SHIFT,
  LITTLE_ENDIAN,
  MAX_CODE_POINT,
  MAX_U64_BYTES,
  NEWEST_MINOR,
  PAYLOAD,
  PROTO_KEY,
  SHIFT,
  SHORT_STRING_READ_LIMIT,
  LATIN1_STRING_READ_LIMIT,
  TINY_STRING_READ_LIMIT,
  SHIFT_VARUINT_BYTES,
  SURROGATE_END,
  SURROGATE_START,
  TRUE_BYTE,
  U16_MAX,
  U32_BYTES,
  U32_MAX,
  U64_MAX
} from '../const.js';

import { FieldValidation } from './validations.js';
import type { Pattern } from './pattern.js';

const decoder = new TextDecoder('utf-8', { fatal: true });
const TEMP_BUFFER_VIEW = new DataView(new ArrayBuffer(F64_BYTES));
const TEMP_BUFFER = new Uint8Array(TEMP_BUFFER_VIEW.buffer);

const NodeBuffer = (globalThis as { Buffer?: { prototype: { latin1Slice?: unknown } } }).Buffer;
const latin1Slice = NodeBuffer?.prototype.latin1Slice as
  ((this: Uint8Array, start: number, end: number) => string) | undefined;

export class BinaryReader {
  static readonly NEWEST = NEWEST_MINOR;
  minor = NEWEST_MINOR;
  validating = false;
  private path: number[] | undefined;
  offset: number;
  end: number;

  constructor(
    readonly buffer: Uint8Array,
    start = 0,
    end = buffer.length
  ) {
    this.offset = start;
    this.end = end;
  }

  getRemainderReader(): BinaryReader {
    const remainder = new BinaryReader(this.buffer, this.offset, this.end);

    remainder.minor = this.minor;

    return remainder;
  }

  get unreadLength(): number {
    return this.end - this.offset;
  }

  finish(): void {
    if (this.offset !== this.end) throw new RangeError(ERR_TRAILING_INPUT);
  }

  skip(length: number): void {
    const end = this.offset + length;

    if (!Number.isSafeInteger(length) || length < 0 || end > this.end) throw new RangeError(ERR_UNEXPECTED_END);
    this.offset = end;
  }

  byte(): number {
    if (this.offset >= this.end) throw new RangeError(ERR_UNEXPECTED_END);

    return this.buffer[this.offset++]!;
  }

  flags(mask: number): number {
    const flags = this.byte();

    if ((flags & ~mask) !== 0) throw new RangeError(ERR_INVALID_FLAGS);

    return flags;
  }

  boolean(): boolean {
    const value = this.byte();

    if (value > TRUE_BYTE) throw new RangeError(ERR_INVALID_BOOLEAN);

    return value === TRUE_BYTE;
  }

  varuint(): number {
    let value = 0;

    for (let shift = 0; shift < SHIFT_VARUINT_BYTES * SHIFT; shift += SHIFT) {
      const byte = this.byte();

      value |= (byte & PAYLOAD) << shift;
      if ((byte & CONTINUATION) === 0) return value >>> 0;
    }

    const last = this.byte();

    if (last > U32_MAX >>> (SHIFT_VARUINT_BYTES * SHIFT)) throw new RangeError(ERR_TOO_LARGE);

    return (value >>> 0) + last * 2 ** (SHIFT_VARUINT_BYTES * SHIFT);
  }

  varuint64(): bigint {
    let value = 0;
    let scale = 1;

    for (let index = 0; index < SAFE_VARUINT_BYTES; index++) {
      const byte = this.byte();

      value += (byte & PAYLOAD) * scale;
      if ((byte & CONTINUATION) === 0) return BigInt(value);
      scale *= CONTINUATION;
    }

    const big = this.continueVaruint64(BigInt(value), BigInt(SAFE_VARUINT_BYTES) * BIG_SHIFT);

    if (big > U64_MAX) throw new RangeError(ERR_TOO_LARGE);

    return big;
  }

  varint(): number {
    const zigzag = this.varuint();

    return zigzag % 2 === 0 ? zigzag / 2 : -(zigzag + 1) / 2;
  }

  private continueVaruint64(value: bigint, shift: bigint): bigint {
    for (let index = Number(shift / BIG_SHIFT); index < MAX_U64_BYTES; index++) {
      const byte = this.byte();

      value |= BigInt(byte & PAYLOAD) << shift;
      if ((byte & CONTINUATION) === 0) return value;
      shift += BIG_SHIFT;
    }

    throw new RangeError(ERR_TOO_LARGE);
  }

  length(): number {
    return this.varuint();
  }

  string(): string {
    const length = this.length();
    const start = this.offset;

    this.skip(length);

    return BinaryReader.decodeUtf8(this.buffer, start, this.offset);
  }

  enter(position: number): void {
    if (!this.validating) return;
    this.path = FieldValidation.enterField(this.path, position);
  }

  advance(): void {
    if (!this.validating) return;
    FieldValidation.advance(this.path);
  }

  exit(): void {
    if (!this.validating) return;
    this.path?.pop();
  }

  validatedLength(min: number, max: number): number {
    const length = this.length();

    if (this.validating) FieldValidation.validateLength(this.path, length, min, max);

    return length;
  }

  validatedString(min: number, max: number, pattern?: Pattern): string {
    if (!this.validating) return this.string();

    const length = this.length();

    FieldValidation.validateLength(this.path, length, min, max);

    const start = this.offset;

    this.skip(length);
    FieldValidation.validatePattern(this.path, pattern, this.buffer, start, this.offset);

    return BinaryReader.decodeUtf8(this.buffer, start, this.offset);
  }

  validatedBytes(min: number, max: number): Uint8Array {
    if (!this.validating) return this.bytes();

    const length = this.length();

    FieldValidation.validateLength(this.path, length, min, max);

    const start = this.offset;

    this.skip(length);

    return this.buffer.slice(start, this.offset);
  }

  validateRange<T extends number | bigint>(value: T, min: number | bigint, max: number | bigint): T {
    return this.validating ? FieldValidation.validateRange(this.path, value, min, max) : value;
  }

  bytes(): Uint8Array {
    const length = this.length();
    const start = this.offset;

    this.skip(length);

    return this.buffer.slice(start, this.offset);
  }

  char(): string {
    const code = this.varuint();

    if (code > MAX_CODE_POINT || (code >= SURROGATE_START && code <= SURROGATE_END)) {
      throw new RangeError(ERR_INVALID_CHARACTER);
    }

    return String.fromCodePoint(code);
  }

  u8(): number {
    return this.byte();
  }

  i8(): number {
    return (this.byte() << I8_SIGN_SHIFT) >> I8_SIGN_SHIFT;
  }

  u16(): number {
    return BinaryReader.requireRange(this.varuint(), 0, U16_MAX);
  }

  u32(): number {
    return this.varuint();
  }

  u64(): bigint {
    return this.varuint64();
  }

  i16(): number {
    return BinaryReader.requireRange(this.varint(), I16_MIN, I16_MAX);
  }

  i32(): number {
    return BinaryReader.requireRange(this.varint(), I32_MIN, I32_MAX);
  }

  i64(): bigint {
    const buffer = this.buffer;
    const start = this.offset;
    let value = 0;
    let scale = 1;

    for (let index = start; index < this.end && index - start < SAFE_VARUINT_BYTES; index++) {
      const byte = buffer[index]!;

      value += (byte & PAYLOAD) * scale;
      if ((byte & CONTINUATION) === 0) {
        this.offset = index + 1;

        return BigInt(value % 2 === 0 ? value / 2 : -(value + 1) / 2);
      }

      scale *= CONTINUATION;
    }

    const zigzag = this.varuint64();

    return (zigzag & BIG_ONE) === 0n ? zigzag >> BIG_ONE : -((zigzag + BIG_ONE) >> BIG_ONE);
  }

  f32(): number {
    const start = this.offset;

    this.skip(F32_BYTES);
    BinaryReader.copyToTemp(this.buffer, start, F32_BYTES);

    return TEMP_BUFFER_VIEW.getFloat32(0, LITTLE_ENDIAN);
  }

  f64(): number {
    const start = this.offset;

    this.skip(F64_BYTES);
    BinaryReader.copyToTemp(this.buffer, start, F64_BYTES);

    return TEMP_BUFFER_VIEW.getFloat64(0, LITTLE_ENDIAN);
  }

  fixed32(): number {
    const start = this.offset;

    this.skip(U32_BYTES);

    const bytes = this.buffer;

    return (
      (bytes[start]! |
        (bytes[start + 1]! << BYTE_BITS) |
        (bytes[start + 2]! << (2 * BYTE_BITS)) |
        (bytes[start + 3]! << (3 * BYTE_BITS))) >>>
      0
    );
  }

  fixed64(): bigint {
    const low = this.fixed32();
    const high = this.fixed32();

    return (BigInt(high) << BIG_WORD_BITS) | BigInt(low);
  }

  private static copyToTemp(bytes: Uint8Array, start: number, count: number): void {
    for (let index = 0; index < count; index++) TEMP_BUFFER[index] = bytes[start + index]!;
  }

  static setObjectProperty<V>(obj: Record<string, V>, key: string, value: V): void {
    if (key === PROTO_KEY) throw new RangeError(ERR_PROTO_KEY);
    obj[key] = value;
  }

  private static requireRange(value: number, minimum: number, maximum: number): number {
    if (value < minimum || value > maximum) throw new RangeError(ERR_TOO_LARGE);

    return value;
  }

  private static decodeUtf8(bytes: Uint8Array, start: number, end: number): string {
    const length = end - start;

    if (latin1Slice !== undefined && length > TINY_STRING_READ_LIMIT && length <= LATIN1_STRING_READ_LIMIT) {
      for (let index = start; index < end; index++)
        if (bytes[index]! >= ASCII_LIMIT) return decoder.decode(bytes.subarray(start, end));

      return latin1Slice.call(bytes, start, end);
    }

    if (length > SHORT_STRING_READ_LIMIT) return decoder.decode(bytes.subarray(start, end));

    let text = '';

    for (let index = start; index < end; index++) {
      const byte = bytes[index]!;

      if (byte >= ASCII_LIMIT) return decoder.decode(bytes.subarray(start, end));
      text += String.fromCharCode(byte);
    }

    return text;
  }
}
