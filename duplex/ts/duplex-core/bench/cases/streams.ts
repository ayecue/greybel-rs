import { do_not_optimize } from 'mitata';

import type { IncomingStream, ResponseWriter } from '../../src/index.js';
import * as client from '../../test/calculator.client.generated.js';
import { Phase } from '../../test/calculator.generated.js';
import * as provider from '../../test/calculator.provider.generated.js';
import type { BenchGroup } from '../case.js';
import { connectedPair } from '../support/link.js';

const ITEMS = 256;

class Streamer extends provider.CalculatorRouter {
  override async onCount(message: provider.Count, stream: ResponseWriter<Phase, number>): Promise<bigint> {
    stream.start(Phase.Active);
    for (let number = 1; number <= message.to; number++) await stream.send(number);

    return BigInt(message.to);
  }

  override async onSum(_message: provider.Sum, terms: IncomingStream<void, bigint>): Promise<bigint> {
    let sum = 0n;

    for await (const term of terms) sum += term;

    return sum;
  }
}

export const streams: BenchGroup = {
  name: `a stream of ${ITEMS} items`,
  cases: [
    {
      name: 'streamed response',
      async setup() {
        const { user } = await connectedPair(new Streamer());
        const count = new client.Count({ to: ITEMS });

        return async () => {
          let total = 0;

          for await (const number of user.send(count)) total += number;
          do_not_optimize(total);
        };
      }
    },
    {
      name: 'message stream',
      async setup() {
        const { user } = await connectedPair(new Streamer());
        const sum = new client.Sum();

        return async () => {
          const { stream, response } = user.send(sum);

          for (let term = 0n; term < BigInt(ITEMS); term++) await stream.send(term);
          await stream.end(undefined);
          do_not_optimize(await response);
        };
      }
    }
  ]
};
