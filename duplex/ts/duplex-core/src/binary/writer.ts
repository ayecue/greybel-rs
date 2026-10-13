import { FieldValidation } from './validations.js';
import type { Pattern } from './pattern.js';
import {
  ASCII_LIMIT,
  BIG_ONE,
  BIG_PAYLOAD,
  BIG_RANGE,
  BIG_SHIFT,
  BIG_WORD_BITS,
  BMP_END,
  BYTE_BITS,
  BYTE_RANGE,
  CONTINUATION,
  ERR_CHARACTER,
  ERR_LONE_SURROGATE,
  F32_BYTES,
  F64_BYTES,
  FALSE_BYTE,
  GROWTH_FACTOR,
  I16_MAX,
  I16_MIN,
  I32_MAX,
  I32_MIN,
  I64_MAX,
  I64_MIN,
  ZIGZAG_SAFE,
  I8_MAX,
  I8_MIN,
  LITTLE_ENDIAN,
  LONE_SURROGATE,
  MAX_CODE_POINT,
  MAX_SAFE,
  MAX_U32_BYTES,
  MAX_U64_BYTES,
  MAX_UTF8_BYTES_PER_UNIT,
  NEWEST_MINOR,
  PAYLOAD,
  SHIFT,
  LOOP_COPY_LIMIT,
  SHORT_STRING_WRITE_LIMIT,
  SHORT_STRING_WRITE_LIMIT_UTF8,
  SURROGATE_END,
  SURROGATE_START,
  TRUE_BYTE,
  U16_MAX,
  U32_BYTES,
  U32_MASK,
  U32_MAX,
  U32_RANGE,
  U64_MAX,
  U8_MAX,
  WRITER_CAPACITY,
  ZIGZAG_SHIFT
} from '../const.js';

const encoder = new TextEncoder();

const NodeBuffer = (globalThis as { Buffer?: { prototype: { utf8Write?: unknown } } }).Buffer;
const utf8Write = NodeBuffer?.prototype.utf8Write as
  ((this: Uint8Array, value: string, offset: number, length: number) => number) | undefined;
const shortStringLimit = utf8Write === undefined ? SHORT_STRING_WRITE_LIMIT : SHORT_STRING_WRITE_LIMIT_UTF8;

export class BinaryWriter {
  static readonly NEWEST = NEWEST_MINOR;

  minor = BinaryWriter.NEWEST;
  validating = false;
  private path: number[] | undefined;
  private buffer: Uint8Array;
  private dataView: DataView | undefined;
  protected offset = 0;

  constructor(capacity = WRITER_CAPACITY) {
    this.buffer = new Uint8Array(capacity);
  }

  get size(): number {
    return this.offset;
  }

  private get view(): DataView {
    this.dataView ??= new DataView(this.buffer.buffer);

    return this.dataView;
  }

  private ensureCapacity(length: number): void {
    const requiredLength = this.offset + length;

    if (requiredLength <= this.buffer.length) return;

    let capacity = this.buffer.length;

    while (capacity < requiredLength) capacity *= GROWTH_FACTOR;

    const buffer = new Uint8Array(capacity);

    buffer.set(this.buffer.subarray(0, this.offset));
    this.buffer = buffer;
    this.dataView = undefined;
  }

  byte(value: number): void {
    this.ensureCapacity(1);
    this.buffer[this.offset++] = value;
  }

  raw(value: Uint8Array): void {
    this.ensureCapacity(value.length);
    this.buffer.set(value, this.offset);
    this.offset += value.length;
  }

  rawRange(source: Uint8Array, start: number, end: number): void {
    const length = end - start;

    this.ensureCapacity(length);
    if (length > LOOP_COPY_LIMIT) {
      this.buffer.set(source.subarray(start, end), this.offset);
      this.offset += length;

      return;
    }

    const buffer = this.buffer;
    let offset = this.offset;

    for (let index = start; index < end; index++) buffer[offset++] = source[index]!;
    this.offset = offset;
  }

  validate(validating: boolean): void {
    this.validating = validating;
    if (this.path === undefined) return;
    this.path.length = 0;
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

  validatedLength(length: number, min: number, max: number): void {
    if (this.validating) FieldValidation.validateLength(this.path, length, min, max);
    this.length(length);
  }

  validatedString(value: string, min: number, max: number, pattern?: Pattern): void {
    const start = this.offset;

    this.string(value);
    if (!this.validating) return;

    let payload = start;

    while ((this.buffer[payload++]! & CONTINUATION) !== 0);
    FieldValidation.validateLength(this.path, this.offset - payload, min, max);
    FieldValidation.validatePattern(this.path, pattern, this.buffer, payload, this.offset);
  }

  validatedBytes(value: Uint8Array, min: number, max: number): void {
    if (this.validating) FieldValidation.validateLength(this.path, value.length, min, max);
    this.bytes(value);
  }

  validateRange<T extends number | bigint>(value: T, min: number | bigint, max: number | bigint): T {
    return this.validating ? FieldValidation.validateRange(this.path, value, min, max) : value;
  }

  bytes(value: Uint8Array): void {
    this.length(value.length);
    this.raw(value);
  }

  boolean(value: boolean): void {
    this.byte(value ? TRUE_BYTE : FALSE_BYTE);
  }

  varuint(value: number): void {
    this.rawVaruint(BinaryWriter.requireInteger(value, 0, Number.MAX_SAFE_INTEGER));
  }

  private rawVaruint(value: number): void {
    this.ensureCapacity(MAX_U64_BYTES);

    const buffer = this.buffer;
    let offset = this.offset;

    while (value >= U32_RANGE) {
      buffer[offset++] = (value % BYTE_RANGE) | CONTINUATION;
      value = Math.floor(value / BYTE_RANGE);
    }

    while (value >= CONTINUATION) {
      buffer[offset++] = (value & PAYLOAD) | CONTINUATION;
      value >>>= SHIFT;
    }

    buffer[offset++] = value;
    this.offset = offset;
  }

  varint(value: number): void {
    BinaryWriter.requireInteger(value, I32_MIN, I32_MAX);
    this.rawVaruint(((value << 1) ^ (value >> 31)) >>> 0);
  }

  varuint64(value: bigint): void {
    if (value <= MAX_SAFE) this.rawVaruint(Number(value));
    else this.writeBigVaruint(value);
  }

  private writeBigVaruint(value: bigint): void {
    this.ensureCapacity(MAX_U64_BYTES);
    while (value >= BIG_RANGE) {
      this.buffer[this.offset++] = Number(value & BIG_PAYLOAD) | CONTINUATION;
      value >>= BIG_SHIFT;
    }

    this.buffer[this.offset++] = Number(value);
  }

  length(length: number): void {
    this.rawVaruint(BinaryWriter.requireInteger(length, 0, U32_MAX));
  }

  string(value: string): void {
    if (value.length <= shortStringLimit && this.writeAscii(value)) return;
    if (LONE_SURROGATE.test(value)) throw new TypeError(ERR_LONE_SURROGATE);
    this.ensureCapacity(MAX_U32_BYTES + value.length * MAX_UTF8_BYTES_PER_UNIT);

    const payload = this.offset + MAX_U32_BYTES;
    const byteLength =
      utf8Write === undefined
        ? encoder.encodeInto(value, this.buffer.subarray(payload)).written
        : utf8Write.call(this.buffer, value, payload, this.buffer.length - payload);

    this.rawVaruint(byteLength);
    this.buffer.copyWithin(this.offset, payload, payload + byteLength);
    this.offset += byteLength;
  }

  private writeAscii(value: string): boolean {
    const start = this.offset;

    this.ensureCapacity(MAX_U32_BYTES + value.length);
    this.rawVaruint(value.length);

    const buffer = this.buffer;
    let offset = this.offset;

    for (let index = 0; index < value.length; index++) {
      const unit = value.charCodeAt(index);

      if (unit >= ASCII_LIMIT) {
        this.offset = start;

        return false;
      }

      buffer[offset++] = unit;
    }

    this.offset = offset;

    return true;
  }

  char(value: string): void {
    const code = value.codePointAt(0);

    if (
      code === undefined ||
      value.length !== (code > BMP_END ? 2 : 1) ||
      code > MAX_CODE_POINT ||
      (code >= SURROGATE_START && code <= SURROGATE_END)
    ) {
      throw new RangeError(ERR_CHARACTER);
    }

    this.rawVaruint(code);
  }

  u8(value: number): void {
    this.byte(BinaryWriter.requireInteger(value, 0, U8_MAX));
  }

  i8(value: number): void {
    this.byte(BinaryWriter.requireInteger(value, I8_MIN, I8_MAX) & U8_MAX);
  }

  u16(value: number): void {
    this.rawVaruint(BinaryWriter.requireInteger(value, 0, U16_MAX));
  }

  u32(value: number): void {
    this.rawVaruint(BinaryWriter.requireInteger(value, 0, U32_MAX));
  }

  u64(value: bigint): void {
    this.varuint64(BinaryWriter.requireBigInteger(value, 0n, U64_MAX));
  }

  i16(value: number): void {
    BinaryWriter.requireInteger(value, I16_MIN, I16_MAX);
    this.rawVaruint(((value << 1) ^ (value >> 31)) >>> 0);
  }

  i32(value: number): void {
    this.varint(value);
  }

  i64(value: bigint): void {
    const number = typeof value === 'bigint' ? Number(value) : NaN;

    if (number <= ZIGZAG_SAFE && number >= -ZIGZAG_SAFE) {
      this.rawVaruint(number < 0 ? -number * 2 - 1 : number * 2);

      return;
    }

    BinaryWriter.requireBigInteger(value, I64_MIN, I64_MAX);
    this.varuint64(value < 0n ? (-value << ZIGZAG_SHIFT) - BIG_ONE : value << ZIGZAG_SHIFT);
  }

  f32(value: number): void {
    this.ensureCapacity(F32_BYTES);
    this.view.setFloat32(this.offset, value, LITTLE_ENDIAN);
    this.offset += F32_BYTES;
  }

  f64(value: number): void {
    this.ensureCapacity(F64_BYTES);
    this.view.setFloat64(this.offset, value, LITTLE_ENDIAN);
    this.offset += F64_BYTES;
  }

  fixed32(value: number): void {
    this.writeU32Le(BinaryWriter.requireInteger(value, 0, U32_MAX));
  }

  fixed64(value: bigint): void {
    BinaryWriter.requireBigInteger(value, 0n, U64_MAX);
    this.writeU32Le(Number(value & U32_MASK));
    this.writeU32Le(Number(value >> BIG_WORD_BITS));
  }

  private writeU32Le(value: number): void {
    this.ensureCapacity(U32_BYTES);

    const buffer = this.buffer;
    let offset = this.offset;

    for (let byte = 0; byte < U32_BYTES; byte++) {
      buffer[offset++] = (value >>> (byte * BYTE_BITS)) & U8_MAX;
    }

    this.offset = offset;
  }

  finishLength(offset: number): void {
    const length = this.offset - offset - 1;

    BinaryWriter.requireInteger(length, 0, U32_MAX);
    if (length < CONTINUATION) {
      this.buffer[offset] = length;

      return;
    }

    let size = 1;

    for (let remainder = length >>> SHIFT; remainder > 0; remainder >>>= SHIFT) size++;

    const end = this.offset;

    this.ensureCapacity(size - 1);

    const buffer = this.buffer;

    buffer.copyWithin(offset + size, offset + 1, end);

    let remainder = length;

    for (let at = offset; at < offset + size - 1; at++) {
      buffer[at] = (remainder & PAYLOAD) | CONTINUATION;
      remainder >>>= SHIFT;
    }

    buffer[offset + size - 1] = remainder;
    this.offset = end + size - 1;
  }

  truncate(length: number): void {
    this.offset = Math.min(this.offset, length);
  }

  reset(): void {
    this.offset = 0;
  }

  asBytes(): Uint8Array {
    return this.buffer.subarray(0, this.offset);
  }

  copyBytes(): Uint8Array {
    return this.buffer.slice(0, this.offset);
  }

  private static requireInteger(value: number, minimum: number, maximum: number): number {
    if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
      throw new RangeError(`${value} isn't a supported integer`);
    }

    return value;
  }

  private static requireBigInteger(value: bigint, minimum: bigint, maximum: bigint): bigint {
    if (typeof value !== 'bigint' || value < minimum || value > maximum) {
      throw new RangeError(`${String(value)} isn't a supported integer`);
    }

    return value;
  }
}
