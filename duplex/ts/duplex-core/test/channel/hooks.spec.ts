import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { Transport } from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { MemoryDriver, NO_CHECKS, restoreMocksAndTimers, settle, zeroRandomValues } from '../support/channel.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('the hooks of a channel', () => {
  it('should run after the channel cleaned up after its close', async () => {
    const transport = new Transport(new MemoryDriver());
    const user = new client.UserChannel(transport, NO_CHECKS);

    await settle();

    let reattached: client.UserChannel | undefined;

    user.onClose(() => {
      reattached = new client.UserChannel(transport, NO_CHECKS);
    });
    user.close();
    expect(reattached).toBeInstanceOf(client.UserChannel);
  });
});
