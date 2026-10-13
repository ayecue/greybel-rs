import { describe, expect, it } from 'vitest';

import { BatchStream } from '../../src/index.js';
import { batchesOf, streamed } from '../support/batch-stream.js';

describe('BatchStream', () => {
  it('should write a batch after its length', () => {
    const bytes = streamed([7, 8, 9]);

    expect(bytes).toEqual([3, 7, 8, 9]);
  });

  it('should write a long batch after its length as a varuint', () => {
    const encoded = BatchStream.encode(new Uint8Array(300));

    expect([...encoded.subarray(0, 2)]).toEqual([0xac, 0x02]);
    expect(encoded.byteLength).toBe(302);
  });

  it('should read back the batches of one read', () => {
    const batches = batchesOf(new BatchStream(), streamed([1], [2, 2], [3, 3, 3]));

    expect(batches).toEqual([[1], [2, 2], [3, 3, 3]]);
  });

  it('should wait for the rest of a batch split across reads', () => {
    const stream = new BatchStream();
    const long = Array.from({ length: 200 }, (_, index) => index);
    const bytes = streamed([1, 2, 3], long);
    const reads = [bytes.slice(0, 2), bytes.slice(2, 5), bytes.slice(5)].map((read) => batchesOf(stream, read));

    expect(reads).toEqual([[], [[1, 2, 3]], [long]]);
  });

  it('should reject a length past the longest batch', () => {
    const processing = () => batchesOf(new BatchStream(), [0xff, 0xff, 0xff, 0x7f]);

    expect(processing).toThrow(RangeError);
  });

  it('should reject a length of more than four bytes', () => {
    const processing = () => batchesOf(new BatchStream(), [0x80, 0x80, 0x80, 0x80, 0x01]);

    expect(processing).toThrow(RangeError);
  });

  it('should read back, a byte at a time, batches longer than its buffer', () => {
    const stream = new BatchStream();
    const long = Array.from({ length: 10_000 }, (_, index) => index % 251);
    const bytes = streamed([1, 2], long, [3]);
    const batches = bytes.flatMap((byte) => batchesOf(stream, [byte]));

    expect(batches).toEqual([[1, 2], long, [3]]);
  });
});
