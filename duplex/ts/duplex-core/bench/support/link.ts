import { Driver, type Receiver } from '../../src/index.js';
import * as client from '../../test/calculator.client.generated.js';
import * as provider from '../../test/calculator.provider.generated.js';

class LinkDriver extends Driver {
  readonly transport = 'memory';
  other: LinkDriver | undefined;
  receiver: Receiver | undefined;

  open(receiver: Receiver): () => void {
    this.receiver = receiver;

    return () => {
      this.receiver = undefined;
    };
  }

  send(batch: Uint8Array): void {
    const other = this.other;

    queueMicrotask(() => other?.receiver?.receive(batch));
  }

  override defer(task: () => void): void {
    queueMicrotask(task);
  }
}

const NO_CHECKS = { healthCheck: false } as const;

export async function connectedPair(...routers: provider.CalculatorRouter[]) {
  const userDriver = new LinkDriver();
  const calculatorDriver = new LinkDriver();

  userDriver.other = calculatorDriver;
  calculatorDriver.other = userDriver;

  const calculator = new provider.CalculatorChannel(calculatorDriver, NO_CHECKS);

  calculator.use(...routers);

  const user = new client.UserChannel(userDriver, NO_CHECKS);

  await new Promise<void>((resolve) => user.onAck(() => resolve()));

  return { user, calculator };
}
