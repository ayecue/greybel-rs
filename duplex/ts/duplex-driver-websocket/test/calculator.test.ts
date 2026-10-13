import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { ChannelError } from 'duplex-core';

import { Add, Count, Divide, DivisionFailure, Log, Phase, Quotient, Sum } from './calculator.client.generated.js';
import { BUILD_TIMEOUT, CalculatorExample } from './support/example.js';
import { calculate, calculators } from './support/calculator.js';

beforeAll(() => CalculatorExample.build(), BUILD_TIMEOUT);

afterEach(() => {
  for (const { calculator, user } of calculators.splice(0)) {
    user.close();
    calculator.stop();
  }
});

describe('the Rust calculator over a WebSocket', () => {
  it('should respond to a message', async () => {
    const { user } = await calculate();
    const sum = await user.send(new Add({ a: 20n, b: 22n }));

    expect(sum).toBe(42n);
  });

  it('should respond to messages sent at once, each to its own', async () => {
    const { user } = await calculate();
    const sums = await Promise.all(
      Array.from({ length: 50 }, (_, index) => user.send(new Add({ a: BigInt(index), b: 1n })))
    );

    expect(sums).toEqual(Array.from({ length: 50 }, (_, index) => BigInt(index + 1)));
  });

  it('should fail a division by zero with its declared failure', async () => {
    const { user } = await calculate();
    const quotient = user.send(new Divide({ dividend: 7n, divisor: 0n }));

    await expect(quotient).rejects.toEqual(ChannelError.fail(DivisionFailure.ByZero));
    await expect(user.send(new Divide({ dividend: 7n, divisor: 2n }))).resolves.toEqual(
      new Quotient.WithRemainder({ value: 3n, remainder: 1n })
    );
  });

  it('should stream a response longer than its window', async () => {
    const { user } = await calculate();
    const stream = user.send(new Count({ to: 100 }));
    const start = await stream.start();
    const items: number[] = [];

    for await (const item of stream) items.push(item);
    expect(start).toBe(Phase.Active);
    expect(items).toEqual(Array.from({ length: 100 }, (_, index) => index + 1));
    expect(await stream.end()).toBe(5050n);
  });

  it('should respond to what a message streams after it', async () => {
    const { user } = await calculate();
    const { stream, response } = user.send(new Sum());

    for (let term = 1n; term <= 100n; term++) await stream.send(term);
    await stream.end(undefined);
    expect(await response).toBe(5050n);
  });

  it('should notify the user without a request', async () => {
    const { user, notes } = await calculate();
    const nextNote = notes.nextNote();

    user.send(new Log({ text: 'halfway' }));
    await nextNote;
    expect(notes.notes).toEqual(['halfway']);
  });
});
