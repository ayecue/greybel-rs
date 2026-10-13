import { do_not_optimize } from 'mitata';

import * as client from '../../test/calculator.client.generated.js';
import * as provider from '../../test/calculator.provider.generated.js';
import { Bundle } from '../../src/index.js';
import type { BenchGroup } from '../case.js';
import { connectedPair } from '../support/link.js';

class Adder extends provider.CalculatorRouter {
  override onAdd(message: provider.Add): bigint {
    return message.a + message.b;
  }
}

class AsyncAdder extends provider.CalculatorRouter {
  override async onAdd(message: provider.Add): Promise<bigint> {
    return message.a + message.b;
  }
}

const ADD = new client.Add({ a: 1n, b: 2n });

function burst(count: number): BenchGroup['cases'][number] {
  return {
    name: `${count} at once`,
    async setup() {
      const { user } = await connectedPair(new Adder());

      return async () => {
        const sent: Promise<bigint>[] = new Array(count);

        for (let index = 0; index < count; index++) sent[index] = user.send(ADD);
        do_not_optimize(await Promise.all(sent));
      };
    }
  };
}

function keyedBurst(count: number): BenchGroup['cases'][number] {
  return {
    name: `${count} at once, each in a bundle of its own`,
    async setup() {
      const { user } = await connectedPair(new Adder());

      return async () => {
        const sent: Promise<bigint>[] = new Array(count);

        for (let index = 0; index < count; index++) sent[index] = user.send(ADD, { bundle: Bundle.key(index + 1) });
        do_not_optimize(await Promise.all(sent));
      };
    }
  };
}

export const requests: BenchGroup = {
  name: 'a message and its response',
  cases: [
    {
      name: 'one, synchronous handler',
      async setup() {
        const { user } = await connectedPair(new Adder());

        return async () => do_not_optimize(await user.send(ADD));
      }
    },
    {
      name: 'one, asynchronous handler',
      async setup() {
        const { user } = await connectedPair(new AsyncAdder());

        return async () => do_not_optimize(await user.send(ADD));
      }
    },
    burst(128),
    burst(8192),
    keyedBurst(1024)
  ]
};
