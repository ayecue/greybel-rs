import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import * as client from '../calculator.client.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { NO_CHECKS, restoreMocksAndTimers, zeroRandomValues } from '../support/channel.js';
import { hex } from '../support/frames.js';
import { authenticated, fixtures, SECURE_FIXTURES } from '../support/security.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('transport security', () => {
  it('should send nothing a channel writes in the clear', () => {
    const { link, near, far } = authenticated(true);
    const user = new client.UserChannel(near, NO_CHECKS);

    new provider.CalculatorChannel(far, NO_CHECKS);
    link.deliver();
    user.send(new client.Log({ text: 'a secret' }));
    link.deliver();

    const secret = hex(new TextEncoder().encode('a secret'));

    expect(link.delivered.every((batch) => !hex(batch).includes(secret))).toBe(true);
  });

  it('should secure a transport as the crate does', () => {
    const { link, near, far } = authenticated(true);

    new client.UserChannel(near, NO_CHECKS);
    new provider.CalculatorChannel(far, NO_CHECKS);
    link.deliver();
    expect(link.delivered.slice(0, SECURE_FIXTURES.length).map(hex)).toEqual(
      SECURE_FIXTURES.map((name) => fixtures[name])
    );
  });
});
