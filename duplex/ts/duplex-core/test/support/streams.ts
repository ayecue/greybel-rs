import type { ChannelOptions, IncomingStream, Next, ResponseWriter } from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { Phase } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { MemoryLink, NO_CHECKS } from './channel.js';

export class Counter extends provider.CalculatorRouter {
  readonly sent: unknown[] = [];

  override async onCount(
    message: provider.Count,
    stream: ResponseWriter<Phase, number>,
    _next: Next<bigint>
  ): Promise<bigint> {
    stream.start(Phase.Active);
    for (let number = 1; number <= message.to; number++) {
      try {
        await stream.send(number);
      } catch (error) {
        this.sent.push(error);
        throw error;
      }

      this.sent.push(number);
    }

    return BigInt((message.to * (message.to + 1)) / 2);
  }

  override async onSum(_message: provider.Sum, terms: IncomingStream<void, bigint>): Promise<bigint> {
    let sum = 0n;

    for await (const term of terms) sum += term;

    return sum;
  }
}

export class BatchCounter extends provider.CalculatorRouter {
  override async onCount(
    message: provider.Count,
    stream: ResponseWriter<Phase, number>,
    _next: Next<bigint>
  ): Promise<bigint> {
    stream.start(Phase.Active);
    await stream.sendAll(Array.from({ length: message.to }, (_, index) => index + 1));

    return BigInt((message.to * (message.to + 1)) / 2);
  }
}

export async function exchange(link: MemoryLink): Promise<void> {
  for (let round = 0; round < 6; round++) {
    link.deliver();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

export const WINDOW = 32;

export const STREAMING: ChannelOptions = { ...NO_CHECKS, streamWindow: WINDOW };

export async function connected() {
  const link = new MemoryLink();
  const user = new client.UserChannel(link.client, STREAMING);
  const counter = new Counter();
  const calculator = new provider.CalculatorChannel(link.provider, STREAMING).use(counter);

  await exchange(link);

  return { link, user, calculator, counter };
}
