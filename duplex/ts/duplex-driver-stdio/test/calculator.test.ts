import { execFileSync } from 'node:child_process';

import { afterEach, beforeAll, describe, expect, inject, it } from 'vitest';
import { ChannelError } from 'duplex-core';

import { Add, Divide, DivisionFailure, Log, Quotient } from './calculator.client.generated.js';
import { BUILD_TIMEOUT, calculate, EXAMPLE, path, calculators } from './support/calculator.js';

beforeAll(() => {
  execFileSync('cargo', ['build', '--quiet', '--package', 'duplex-driver-stdio', '--example', EXAMPLE], {
    cwd: inject('rust'),
    stdio: 'inherit'
  });
  execFileSync(path(), { input: '', stdio: ['pipe', 'ignore', 'inherit'] });
}, BUILD_TIMEOUT);

afterEach(() => {
  for (const calculator of calculators.splice(0)) calculator.kill();
});

describe('the Rust calculator over standard input and output', () => {
  it('should reply to a request', async () => {
    const { user } = calculate();
    const sum = await user.send(new Add({ a: 20n, b: 22n }));

    expect(sum).toBe(42n);
  });

  it("should request the user's confirmation before it replies", async () => {
    const { user } = calculate();
    const quotient = await user.send(new Divide({ dividend: 7n, divisor: 2n }));

    expect(quotient).toEqual(new Quotient.WithRemainder({ value: 3n, remainder: 1n }));
  });

  it('should fail a request the user declines', async () => {
    const { user } = calculate(false);
    const quotient = user.send(new Divide({ dividend: 7n, divisor: 2n }));

    await expect(quotient).rejects.toEqual(ChannelError.fail(DivisionFailure.Refusal));
  });

  it('should fail a division by zero with its declared failure', async () => {
    const { user } = calculate();
    const quotient = user.send(new Divide({ dividend: 7n, divisor: 0n }));

    await expect(quotient).rejects.toEqual(ChannelError.fail(DivisionFailure.ByZero));
  });

  it('should notify the user back', async () => {
    const { user, confirmer } = calculate();
    const nextNote = confirmer.nextNote();

    user.send(new Log({ text: 'halfway' }));
    await nextNote;
    expect(confirmer.notes).toEqual(['halfway']);
  });

  it('should end once its input ends', async () => {
    const { user, calculator } = calculate();

    await user.send(new Add({ a: 1n, b: 1n }));

    const exited = new Promise<number | null>((resolve) => calculator.once('exit', resolve));

    calculator.stdin.end();
    expect(await exited).toBe(0);
  });
});
