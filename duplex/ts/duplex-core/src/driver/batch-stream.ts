import {
  CONTINUATION,
  MAX_STREAM_BATCH_BYTES,
  MAX_STREAM_LENGTH_BYTES,
  PAYLOAD,
  SHIFT,
  BATCH_STREAM_CAPACITY
} from '../const.js';

export class BatchStream {
  #buffer: Uint8Array | undefined;
  #start = 0;
  #end = 0;

  static getLengthPrefix(batch: Uint8Array): Uint8Array {
    const length = new Uint8Array(BatchStream.#getLengthPrefixSize(batch.byteLength));

    BatchStream.#writeLength(batch.byteLength, length);

    return length;
  }

  static encode(batch: Uint8Array): Uint8Array<ArrayBuffer> {
    const prefixSize = BatchStream.#getLengthPrefixSize(batch.byteLength);
    const streamBatch = new Uint8Array(prefixSize + batch.byteLength);

    BatchStream.#writeLength(batch.byteLength, streamBatch);
    streamBatch.set(batch, prefixSize);

    return streamBatch;
  }

  process(bytes: Uint8Array, receive: (batch: Uint8Array) => void): void {
    if (this.#start === this.#end) {
      const read = BatchStream.#split(bytes, 0, bytes.byteLength, receive, false);

      if (read < bytes.byteLength) this.#append(bytes.subarray(read));

      return;
    }

    const buffer = this.#append(bytes);

    try {
      this.#start = BatchStream.#split(buffer, this.#start, this.#end, receive, true);
    } catch (error) {
      this.#start = this.#end = 0;
      throw error;
    }

    if (this.#start !== this.#end) return;
    this.#start = this.#end = 0;
  }

  static #split(
    bytes: Uint8Array,
    start: number,
    end: number,
    receive: (batch: Uint8Array) => void,
    copies: boolean
  ): number {
    let batchStart = start;

    for (;;) {
      let length = 0;
      let index = batchStart;
      let hasLength = false;

      while (index < end && index - batchStart < MAX_STREAM_LENGTH_BYTES) {
        const byte = bytes[index]!;

        length += (byte & PAYLOAD) * 2 ** (SHIFT * (index - batchStart));
        index++;
        if ((byte & CONTINUATION) !== 0) continue;
        hasLength = true;
        break;
      }

      if (!hasLength) {
        if (index - batchStart >= MAX_STREAM_LENGTH_BYTES)
          throw new RangeError(`a batch's length takes more than ${MAX_STREAM_LENGTH_BYTES} bytes`);

        return batchStart;
      }

      if (length > MAX_STREAM_BATCH_BYTES)
        throw new RangeError(`a batch of ${length} bytes is longer than ${MAX_STREAM_BATCH_BYTES}`);
      if (index + length > end) return batchStart;
      receive(copies ? bytes.slice(index, index + length) : bytes.subarray(index, index + length));
      batchStart = index + length;
    }
  }

  static #getLengthPrefixSize(length: number): number {
    let bytes = 1;

    for (let remainder = length; remainder >= CONTINUATION; remainder = Math.floor(remainder / CONTINUATION)) bytes++;

    return bytes;
  }

  static #writeLength(length: number, out: Uint8Array): void {
    let index = 0;
    let remainder = length;

    while (remainder >= CONTINUATION) {
      out[index++] = (remainder & PAYLOAD) | CONTINUATION;
      remainder = Math.floor(remainder / CONTINUATION);
    }

    out[index] = remainder;
  }

  #append(bytes: Uint8Array): Uint8Array {
    let buffer = (this.#buffer ??= new Uint8Array(BATCH_STREAM_CAPACITY));
    const unread = this.#end - this.#start;

    if (this.#end + bytes.byteLength > buffer.byteLength) {
      const requiredLength = unread + bytes.byteLength;
      let capacity = buffer.byteLength;

      while (capacity < requiredLength) capacity *= 2;
      if (capacity === buffer.byteLength) {
        buffer.copyWithin(0, this.#start, this.#end);
      } else {
        const largerBuffer = new Uint8Array(capacity);

        largerBuffer.set(buffer.subarray(this.#start, this.#end));
        buffer = this.#buffer = largerBuffer;
      }

      this.#start = 0;
      this.#end = unread;
    }

    buffer.set(bytes, this.#end);
    this.#end += bytes.byteLength;

    return buffer;
  }
}
