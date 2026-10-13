import { Requirement, Violation } from '../../src/definition.generated.js';
import {
  ChannelError,
  type ChannelOptions,
  type IncomingStream,
  type Next,
  type RouteContext
} from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { Phase, Quotient, Step } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { MemoryLink, NO_CHECKS, settle } from './channel.js';

export async function exchanged(link: MemoryLink): Promise<void> {
  for (let round = 0; round < 4; round++) {
    await settle();
    link.deliver();
  }
}

export const UNVALIDATED: ChannelOptions = { ...NO_CHECKS, unvalidated: true };

export function invalid(path: number[], requirement: Requirement): ChannelError<never> {
  return ChannelError.invalid(new Violation({ path, requirement }));
}

export function connected(user: ChannelOptions = NO_CHECKS, calculator: ChannelOptions = NO_CHECKS) {
  const link = new MemoryLink();
  const handled: unknown[] = [];
  const retained: IncomingStream<void, bigint>[] = [];
  const channel = new provider.CalculatorChannel(link.provider, calculator).use(
    new (class extends provider.CalculatorRouter {
      override onDivide(message: provider.Divide): Quotient {
        handled.push(message);

        return new Quotient.Exact({ value: message.dividend / message.divisor });
      }

      override onCount(message: provider.Count): Promise<bigint> {
        handled.push(message);

        return new Promise(() => undefined);
      }

      override onSum(
        _message: provider.Sum,
        items: IncomingStream<void, bigint>,
        _next: Next<bigint>,
        _context: RouteContext
      ): Promise<bigint> {
        retained.push(items);

        return new Promise(() => undefined);
      }
    })()
  );
  const userChannel = new client.UserChannel(link.client, user);

  link.deliver();

  return { link, user: userChannel, calculator: channel, handled, retained };
}

export function progress(...names: string[]): provider.Progress {
  return new provider.Progress({
    phase: Phase.Active,
    doneShare: null,
    steps: names.map((name) => new Step({ name, result: null })),
    counts: {}
  });
}
