import { Socket } from 'node:net';
import { PassThrough } from 'node:stream';

import { describe, expect, it } from 'vitest';

import { StreamDriver } from '../src/index.js';
import { stubReceiver, encode } from './support/stream-driver.js';

describe('StreamDriver', () => {
  it('should write each batch after its length', () => {
    const written = new PassThrough();
    const driver = new StreamDriver(new PassThrough(), written);

    driver.send(new Uint8Array([7, 8]));
    expect([...(written.read() as Uint8Array)]).toEqual([2, 7, 8]);
  });

  it('should name its transport by what it is over', () => {
    const transports = [new StreamDriver(new PassThrough(), new PassThrough()), StreamDriver.tcp(new Socket())].map(
      (driver) => driver.transport
    );

    expect(transports).toEqual(['stream', 'tcp']);
  });

  it('should hand on the batches whose bytes arrived, however they were split', () => {
    const readable = new PassThrough();
    const receiver = stubReceiver();

    new StreamDriver(readable, new PassThrough()).open(receiver);

    const bytes = Buffer.concat([encode([1, 2, 3]), encode([4])]);

    readable.write(bytes.subarray(0, 5));
    readable.write(bytes.subarray(5));
    expect(receiver.batches).toEqual([[1, 2, 3], [4]]);
  });

  it('should tell the receiver when the stream ends', async () => {
    const readable = new PassThrough();
    const receiver = stubReceiver();

    new StreamDriver(readable, new PassThrough()).open(receiver);
    readable.resume();
    readable.end();
    await new Promise((resolve) => readable.once('end', resolve));
    expect(receiver.disconnections).toBe(1);
  });

  it('should stop handing on batches once closed', () => {
    const readable = new PassThrough();
    const receiver = stubReceiver();
    const stop = new StreamDriver(readable, new PassThrough()).open(receiver);

    stop();
    readable.write(encode([1]));
    expect(receiver.batches).toEqual([]);
  });
});
