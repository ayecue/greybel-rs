import { do_not_optimize } from 'mitata';

import * as client from '../../test/calculator.client.generated.js';
import { Phase, Quotient, Step } from '../../test/calculator.generated.js';
import * as provider from '../../test/calculator.provider.generated.js';
import type { BenchGroup } from '../case.js';
import { connectedPair } from '../support/link.js';

const COUNT = 1000;

class Countdown {
  #left = 0;
  #done: (() => void) | undefined;

  expect(count: number): Promise<void> {
    this.#left = count;

    return new Promise((resolve) => (this.#done = resolve));
  }

  take(value: unknown): void {
    do_not_optimize(value);
    if (--this.#left !== 0) return;
    this.#done?.();
  }
}

class LogCounter extends provider.CalculatorRouter {
  constructor(private readonly counted: Countdown) {
    super();
  }

  override onLog(message: provider.Log): void {
    this.counted.take(message);
  }
}

class ProgressCounter extends client.UserRouter {
  constructor(private readonly counted: Countdown) {
    super();
  }

  override onProgress(message: client.Progress): void {
    this.counted.take(message);
  }
}

const PROGRESS = new provider.Progress({
  phase: Phase.Active,
  doneShare: 0.5,
  steps: [
    new Step({ name: 'divide', result: new Quotient.WithRemainder({ value: 2n, remainder: 1n }) }),
    new Step({ name: 'wait', result: null })
  ],
  counts: { divisions: 1, waits: 2 },
  note: 'almost'
});

export const notifications: BenchGroup = {
  name: `${COUNT} notifications`,
  cases: [
    {
      name: 'a short text',
      async setup() {
        const counted = new Countdown();
        const { user } = await connectedPair(new LogCounter(counted));
        const log = new client.Log({ text: 'a line for the log' });

        return async () => {
          const received = counted.expect(COUNT);

          for (let index = 0; index < COUNT; index++) user.send(log);
          await received;
        };
      }
    },
    {
      name: 'nested fields',
      async setup() {
        const counted = new Countdown();
        const { user, calculator } = await connectedPair();

        user.use(new ProgressCounter(counted));

        return async () => {
          const received = counted.expect(COUNT);

          for (let index = 0; index < COUNT; index++) calculator.send(PROGRESS);
          await received;
        };
      }
    }
  ]
};
