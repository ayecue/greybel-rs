import { describe, expect, it } from 'vitest';

import { Egress } from '../../src/transport/egress.js';
import { Bundle, ChannelError, ErrorCode } from '../../src/index.js';
import { BATCH_BYTES, flushAll, OTHER, payloadsOf, ROUTE, write } from '../support/egress.js';

describe('the egress of a transport', () => {
  it('should refuse writes once it has the most a transport has', () => {
    const egress = new Egress();

    write(egress, ROUTE, new Uint8Array(16 * 1024 * 1024));

    const overloading = () => write(egress, ROUTE, new Uint8Array(1));

    expect(overloading).toThrow(ChannelError.reject(ErrorCode.Overload));
  });

  it('should write nothing of a frame past the most a frame takes', () => {
    const egress = new Egress();
    const overloading = () => write(egress, ROUTE, new Uint8Array(100), 100);

    expect(overloading).toThrow(ChannelError.reject(ErrorCode.Overload));
    expect(egress.isEmpty).toBe(true);
  });

  it('should send what waits past a full batch in the next, by whole frames and in order', () => {
    const egress = new Egress();

    for (let index = 0; index < 200; index++) write(egress, ROUTE, new Uint8Array(1000).fill(index));

    const sent = flushAll(egress);

    expect(sent).toHaveLength(4);
    expect(sent.every((batch) => batch.length <= BATCH_BYTES + 16)).toBe(true);
    expect(payloadsOf(sent).map((payload) => payload[0])).toEqual(Array.from({ length: 200 }, (_, index) => index));
  });

  it('should send a frame past a batch in fragments that fit one', () => {
    const egress = new Egress();
    const frame = Uint8Array.from({ length: 300_000 }, (_, index) => index & 0xff);

    write(egress, ROUTE, frame);

    const sent = flushAll(egress);

    expect(sent).toHaveLength(Math.floor(300_000 / BATCH_BYTES) + 1);
    expect(sent.every((batch) => batch.length <= BATCH_BYTES + 16)).toBe(true);
    expect(payloadsOf(sent, ROUTE.channel)).toEqual([frame]);
  });

  it('should share batches with other channels while a frame goes in fragments', () => {
    const egress = new Egress();
    const large = new Uint8Array(4 * BATCH_BYTES).fill(1);
    const small = new Uint8Array(100).fill(2);

    write(egress, ROUTE, large);
    write(egress, OTHER, small);

    const first: Uint8Array[] = [];

    egress.flush((batch) => first.push(batch));
    expect(first).toHaveLength(1);
    expect(first[0]!.length).toBeLessThanOrEqual(BATCH_BYTES / 2 + 16 * 1024);

    const sent = [...first, ...flushAll(egress)];

    expect(payloadsOf(sent, OTHER.channel)).toEqual([small]);
    expect(payloadsOf(sent, ROUTE.channel)).toEqual([large]);
  });

  it('should send nothing more of a channel after its first frame past its share', () => {
    const egress = new Egress();
    const large = new Uint8Array(BATCH_BYTES / 5 + 1000).fill(5);

    write(egress, ROUTE, large);
    write(egress, { ...ROUTE, connection: 7 }, new Uint8Array(10).fill(6));
    for (let channel = 2; channel <= 5; channel++) write(egress, { ...OTHER, channel }, new Uint8Array(10).fill(7));

    const first: Uint8Array[] = [];

    egress.flush((batch) => first.push(batch));
    expect(payloadsOf(first, ROUTE.channel)).toEqual([large]);
  });

  it('should retain the frames after a fragmented one in their order', () => {
    const egress = new Egress();
    const large = new Uint8Array(2 * BATCH_BYTES).fill(3);
    const after = new Uint8Array(10).fill(4);

    write(egress, ROUTE, large);
    write(egress, ROUTE, after);

    const sent = flushAll(egress);

    expect(payloadsOf(sent, ROUTE.channel)).toEqual([large, after]);
  });

  it('should send a batch of each bundle in one flush, in the order written', () => {
    const egress = new Egress();

    for (let key = 1; key <= 3; key++) {
      write(egress, ROUTE, Uint8Array.of(key), undefined, Bundle.key(key));
      write(egress, OTHER, Uint8Array.of(10 + key), undefined, Bundle.key(key));
    }

    const sent: Uint8Array[] = [];

    egress.flush((batch) => sent.push(batch));
    expect(sent).toHaveLength(3);
    expect(egress.isEmpty).toBe(true);
    sent.forEach((batch, index) => {
      expect(payloadsOf([batch], ROUTE.channel)).toEqual([Uint8Array.of(index + 1)]);
      expect(payloadsOf([batch], OTHER.channel)).toEqual([Uint8Array.of(11 + index)]);
    });
  });

  it('should send what a channel wrote after another bundle in the next flush', () => {
    const egress = new Egress();

    write(egress, ROUTE, Uint8Array.of(1));
    write(egress, ROUTE, Uint8Array.of(2), undefined, Bundle.key(1));
    write(egress, ROUTE, Uint8Array.of(3));

    const first: Uint8Array[] = [];
    const second: Uint8Array[] = [];

    egress.flush((batch) => first.push(batch));
    egress.flush((batch) => second.push(batch));
    expect(first).toHaveLength(2);
    expect(payloadsOf(first, ROUTE.channel)).toEqual([Uint8Array.of(1), Uint8Array.of(2)]);
    expect(payloadsOf(second, ROUTE.channel)).toEqual([Uint8Array.of(3)]);
    expect(egress.isEmpty).toBe(true);
  });
});
