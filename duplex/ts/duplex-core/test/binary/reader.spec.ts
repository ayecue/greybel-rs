import { describe, expect, it } from 'vitest';

import { BinaryReader } from '../../src/index.js';

describe('BinaryReader', () => {
  it('should read floats of a buffer that starts inside its ArrayBuffer', () => {
    const bytes = new Uint8Array(1 + 8 + 4);
    const view = new DataView(bytes.buffer);

    view.setFloat64(1, 1.5, true);
    view.setFloat32(9, -2.25, true);

    const reader = new BinaryReader(bytes.subarray(1));

    const read = [reader.f64(), reader.f32()];

    expect(read).toEqual([1.5, -2.25]);
  });

  it('should refuse a record key of `__proto__`, leaving the record and its prototype as they are', () => {
    const record: Record<string, object> = {};

    const set = () => BinaryReader.setObjectProperty(record, '__proto__', { injected: true });

    expect(set).toThrow(RangeError);
    expect([Object.getPrototypeOf(record) === Object.prototype, Object.keys(record)]).toEqual([true, []]);
  });
});
