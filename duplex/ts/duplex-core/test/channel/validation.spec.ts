import { describe, expect, it } from 'vitest';

import { Requirement } from '../../src/definition.generated.js';
import * as client from '../calculator.client.generated.js';
import { Quotient } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { NO_CHECKS, settle } from '../support/channel.js';
import { connected, exchanged, invalid, progress, UNVALIDATED } from '../support/validation.js';

describe('validation', () => {
  it('should fail a message whose field breaks its requirement before sending it', async () => {
    const { link, user, handled } = connected();
    const quotient = user.send(new client.Divide({ dividend: 6n, divisor: 3n, precision: 13 }));

    link.deliver();
    await expect(quotient).rejects.toEqual(invalid([2], Requirement.Maximum));
    expect(handled).toEqual([]);
  });

  it("should reject a field breaking its requirement that the client didn't validate", async () => {
    const { link, user, handled } = connected(UNVALIDATED);
    const quotient = user.send(new client.Divide({ dividend: 6n, divisor: 3n, precision: 13 }));

    link.deliver();
    await expect(quotient).rejects.toEqual(invalid([2], Requirement.Maximum));
    expect(handled).toEqual([]);
  });

  it('should accept what meets its requirements', async () => {
    const { link, user } = connected();
    const quotient = user.send(new client.Divide({ dividend: 6n, divisor: 3n, precision: 12 }));

    link.deliver();
    await expect(quotient).resolves.toEqual(new Quotient.Exact({ value: 2n }));
  });

  it('should name where a nested value breaks its requirement', () => {
    const { calculator } = connected();
    const errors = [progress('divide', 'Add'), progress(''), progress('a'.repeat(33))].map((message) => {
      try {
        calculator.send(message);

        return undefined;
      } catch (error) {
        return error;
      }
    });

    expect(errors).toEqual([
      invalid([2, 1, 0], Requirement.Pattern),
      invalid([2, 0, 0], Requirement.MinLength),
      invalid([2, 0, 0], Requirement.MaxLength)
    ]);
  });

  it('should omit what the provider sends unvalidated by the client', async () => {
    const { link, user, calculator } = connected(NO_CHECKS, UNVALIDATED);
    const names: string[] = [];

    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          names.push(message.steps[0]!.name);
        }
      })()
    );
    calculator.send(progress('Not Validated'));
    link.deliver();
    await settle();
    expect(names).toEqual(['Not Validated']);
  });

  it('should fail a message past its limit before sending it', () => {
    const { user } = connected();
    const sending = () => user.send(new client.Log({ text: 'x'.repeat(4 * 1024 * 1024) }));

    expect(sending).toThrow(invalid([], Requirement.Limit));
  });

  it('should reject a number past its bounds', async () => {
    const { link, user } = connected(UNVALIDATED);
    const stream = user.send(new client.Count({ to: 10_001 }));

    link.deliver();
    await Promise.resolve();
    link.deliver();
    await expect(stream.start()).rejects.toEqual(invalid([0], Requirement.Maximum));
  });

  it('should reject a stream item past its bounds', async () => {
    const { link, user, retained } = connected(UNVALIDATED);
    const { stream, response } = user.send(new client.Sum());

    link.deliver();
    void stream.send(1_000_001n);
    link.deliver();
    await expect(response).rejects.toEqual(invalid([], Requirement.Maximum));
    expect(retained).toHaveLength(1);
  });

  it('should reject a stream past the most items it has', async () => {
    const window = { ...UNVALIDATED, streamWindow: 2000 };
    const { link, user } = connected(window, window);
    const { stream, response } = user.send(new client.Sum());

    link.deliver();
    for (let index = 0; index <= 1000; index++) void stream.send(1n);
    await exchanged(link);
    await expect(response).rejects.toEqual(invalid([], Requirement.MaxLength));
  });

  it('should fail an item past the most a stream has before sending it', async () => {
    const window = { ...NO_CHECKS, streamWindow: 2000 };
    const { link, user } = connected(window, window);
    const { stream } = user.send(new client.Sum());

    link.deliver();
    for (let index = 0; index < 1000; index++) await stream.send(1n);

    const past = stream.send(1n);

    await expect(past).rejects.toEqual(invalid([], Requirement.MaxLength));
  });
});
