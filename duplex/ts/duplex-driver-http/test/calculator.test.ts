import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { BatchStream, Bundle, ChannelError } from 'duplex-core';

import { type Downstream } from '../src/index.js';
import { Add, Count, Divide, DivisionFailure, Log, Phase, Quotient, Sum } from './calculator.client.generated.js';
import { BUILD_TIMEOUT, CalculatorExample } from './support/example.js';
import { calculate, calculators } from './support/calculator.js';

beforeAll(() => CalculatorExample.build(), BUILD_TIMEOUT);

afterEach(() => {
  for (const { calculator, user } of calculators.splice(0)) {
    user?.close();
    calculator.stop();
  }
});

describe.each<Downstream>(['long-poll', 'events', 'stream'])(
  'the Rust calculator over HTTP, downstream by %s',
  (downstream) => {
    it('should respond to a message', async () => {
      const { user } = await calculate(downstream);
      const sum = await user.send(new Add({ a: 20n, b: 22n }));

      expect(sum).toBe(42n);
    });

    it('should respond to messages sent at once, each to its own', async () => {
      const { user } = await calculate(downstream);
      const sums = await Promise.all(
        Array.from({ length: 50 }, (_, index) => user.send(new Add({ a: BigInt(index), b: 1n })))
      );

      expect(sums).toEqual(Array.from({ length: 50 }, (_, index) => BigInt(index + 1)));
    });

    it('should fail a division by zero with its declared failure', async () => {
      const { user } = await calculate(downstream);
      const quotient = user.send(new Divide({ dividend: 7n, divisor: 0n }));

      await expect(quotient).rejects.toEqual(ChannelError.fail(DivisionFailure.ByZero));
      await expect(user.send(new Divide({ dividend: 7n, divisor: 2n }))).resolves.toEqual(
        new Quotient.WithRemainder({ value: 3n, remainder: 1n })
      );
    });

    it('should stream a response longer than its window', async () => {
      const { user } = await calculate(downstream);
      const stream = user.send(new Count({ to: 100 }));
      const start = await stream.start();
      const items: number[] = [];

      for await (const item of stream) items.push(item);
      expect(start).toBe(Phase.Active);
      expect(items).toEqual(Array.from({ length: 100 }, (_, index) => index + 1));
      expect(await stream.end()).toBe(5050n);
    });

    it('should respond to what a message streams after it', async () => {
      const { user } = await calculate(downstream);
      const { stream, response } = user.send(new Sum());

      for (let term = 1n; term <= 100n; term++) await stream.send(term);
      await stream.end(undefined);
      expect(await response).toBe(5050n);
    });

    it('should notify the user without a request', async () => {
      const { user, notes } = await calculate(downstream);
      const nextNote = notes.nextNote();

      user.send(new Log({ text: 'halfway' }));
      await nextNote;
      expect(notes.notes).toEqual(['halfway']);
    });
  }
);

describe('the Rust calculator over HTTP, downstream by long polls', () => {
  it('should respond to each message within its POST, with the open poll still waiting', async () => {
    const requests: string[] = [];
    const countingFetch = ((input: string, init?: RequestInit) => {
      requests.push(init?.method ?? 'GET');

      return fetch(input, init);
    }) as typeof fetch;
    const { user } = await calculate('long-poll', countingFetch);

    await user.send(new Add({ a: 0n, b: 0n }));
    requests.length = 0;

    const sums = [];

    for (let index = 0; index < 20; index++) sums.push(await user.send(new Add({ a: BigInt(index), b: 1n })));
    expect(sums).toEqual(Array.from({ length: 20 }, (_, index) => BigInt(index + 1)));
    expect(requests.filter((method) => method === 'POST')).toHaveLength(20);
    expect(requests.filter((method) => method === 'GET').length).toBeLessThanOrEqual(2);
  });
});

describe('the batches of the Rust calculator over HTTP', () => {
  it('should post the batches of every bundle sent at once in one POST', async () => {
    const requests: string[] = [];
    const countingFetch = ((input: string, init?: RequestInit) => {
      requests.push(init?.method ?? 'GET');

      return fetch(input, init);
    }) as typeof fetch;
    const { user } = await calculate('long-poll', countingFetch);

    await user.send(new Add({ a: 0n, b: 0n }));
    requests.length = 0;

    const sums = await Promise.all(
      Array.from({ length: 10 }, (_, index) =>
        user.send(new Add({ a: BigInt(index), b: 1n }), { bundle: Bundle.key(index + 1) })
      )
    );

    expect(sums).toEqual(Array.from({ length: 10 }, (_, index) => BigInt(index + 1)));
    expect(requests.filter((method) => method === 'POST')).toHaveLength(1);
  });
});

describe('the transports of the Rust calculator over HTTP', () => {
  it('should name the transport a first POST opens, by an id of its own', async () => {
    const calculator = await CalculatorExample.start('long-poll');

    calculators.push({ calculator });

    const openingBatch = BatchStream.encode(Uint8Array.of(1, 0));
    const openResponse = await fetch(calculator.url, {
      method: 'POST',
      headers: { 'content-type': 'application/octet-stream' },
      body: openingBatch
    });

    expect(openResponse.headers.get('duplex-transport')).toMatch(/^[0-9a-f]{32}$/);
  });

  it('should refuse a request naming a transport it did not open', async () => {
    const calculator = await CalculatorExample.start('long-poll');

    calculators.push({ calculator });

    const guessedHeaders = { 'duplex-transport': '0123456789abcdef0123456789abcdef' };
    const pollResponse = await fetch(calculator.url, { headers: guessedHeaders });
    const postResponse = await fetch(calculator.url, {
      method: 'POST',
      headers: guessedHeaders,
      body: new Uint8Array(1)
    });
    const unnamedResponse = await fetch(calculator.url);

    expect(pollResponse.status).toBe(404);
    expect(postResponse.status).toBe(404);
    expect(unnamedResponse.status).toBe(400);
  });

  it.each<Downstream>(['long-poll', 'events', 'stream'])(
    'should name the transport only in headers, downstream by %s',
    async (downstream) => {
      const urls: string[] = [];
      const recordingFetch = ((input: string, init?: RequestInit) => {
        urls.push(input);

        return fetch(input, init);
      }) as typeof fetch;
      const { user } = await calculate(downstream, recordingFetch);

      await user.send(new Add({ a: 1n, b: 2n }));
      expect(urls.length).toBeGreaterThan(1);
      expect(urls.filter((url) => url.includes('?'))).toEqual([]);
    }
  );
});
