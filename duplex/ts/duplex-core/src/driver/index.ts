import { CONNECTION_RANDOM_BYTES } from '../const.js';
import { Role } from '../definition.generated.js';

export const DEFAULT_STREAM_WINDOW = 4096;

export interface Receiver {
  receive(batch: Uint8Array): void;
  onDisconnect(): void;
}

export type Capabilities = number;

export const Capabilities = {
  CLIENT: 0b001,
  PROVIDER: 0b010,
  PERSISTENT: 0b100,
  NONE: 0,
  ALL: 0b111,

  supports(capabilities: Capabilities, role: Role, requiredFlags: Capabilities): boolean {
    const requiredCapabilities = (role === Role.Client ? Capabilities.CLIENT : Capabilities.PROVIDER) | requiredFlags;

    return (capabilities & requiredCapabilities) === requiredCapabilities;
  }
} as const;

export abstract class Driver {
  abstract readonly transport: string;
  readonly capabilities: Capabilities = Capabilities.ALL;
  readonly streamWindow: number = DEFAULT_STREAM_WINDOW;

  abstract open(receiver: Receiver): () => void;

  abstract send(batch: Uint8Array): void;

  defer(task: () => void): void {
    queueMicrotask(task);
  }

  nextTurn(task: () => void): void {
    const setImmediateFunction = (globalThis as { setImmediate?: (task: () => void) => unknown }).setImmediate;

    if (setImmediateFunction !== undefined) {
      setImmediateFunction(task);

      return;
    }

    const messageChannel = new MessageChannel();

    messageChannel.port1.onmessage = (): void => {
      messageChannel.port1.close();
      task();
    };

    messageChannel.port2.postMessage(undefined);
  }

  interval(period: number, task: () => void): () => void {
    const timer = setInterval(task, period);

    return () => clearInterval(timer);
  }

  after(delay: number, task: () => void): () => void {
    const timer = setTimeout(task, delay);

    return () => clearTimeout(timer);
  }

  now(): number {
    return performance.now();
  }

  random(): Uint8Array {
    return crypto.getRandomValues(new Uint8Array(CONNECTION_RANDOM_BYTES));
  }

  pause(): void {}

  resume(): void {}
}
