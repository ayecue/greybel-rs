import { onTestFinished } from 'vitest';
import type { ChannelOptions } from 'duplex-core';

import { WorkerHost, type WorkerEventType, type WorkerLike } from '../../src/index.js';
import { PageChannel, Shout } from './echo.page.generated.js';
import { WorkerRouter } from './echo.worker.generated.js';
import * as offered from './offered-echo.worker.generated.js';

export const NO_CHECKS: ChannelOptions = { healthCheck: false };

export class Echoer extends WorkerRouter {
  override onShout(request: Shout): string {
    return request.text;
  }
}

export class OfferedEchoer extends offered.WorkerRouter {
  override onShout(request: offered.Shout): string {
    return request.text;
  }
}

export class FakeWorker implements WorkerLike {
  readonly scope: MessagePort;
  readonly posted: unknown[] = [];
  readonly #port: MessagePort;
  readonly #events = new EventTarget();
  #listeners = 0;
  terminated = false;

  constructor() {
    const { port1, port2 } = new MessageChannel();

    this.#port = port1;
    this.scope = port2;
    port1.start();
  }

  get listenerCount(): number {
    return this.#listeners;
  }

  postMessage(message: unknown, transfer?: Transferable[]): void {
    this.posted.push(message instanceof Uint8Array ? message.slice() : message);
    this.#port.postMessage(message, transfer ?? []);
  }

  addEventListener(type: WorkerEventType, listener: (event: Event) => void): void {
    this.#listeners += 1;
    (type === 'message' ? this.#port : this.#events).addEventListener(type, listener);
  }

  removeEventListener(type: WorkerEventType, listener: (event: Event) => void): void {
    this.#listeners -= 1;
    (type === 'message' ? this.#port : this.#events).removeEventListener(type, listener);
  }

  terminate(): void {
    this.terminated = true;
    this.close();
  }

  fail(message: string): void {
    this.#events.dispatchEvent(Object.assign(new Event('error'), { message }));
  }

  close(): void {
    this.#port.close();
    this.scope.close();
  }
}

export function fakeWorker(): FakeWorker {
  const worker = new FakeWorker();

  onTestFinished(() => worker.close());

  return worker;
}

export function handedPort(scope: MessagePort): Promise<MessagePort> {
  return new Promise((resolve) => {
    scope.addEventListener('message', ({ ports }) => resolve(ports[0]!), { once: true });
    scope.start();
  });
}

export function echoHost(worker: FakeWorker) {
  return new WorkerHost(worker, { echo: { channelClass: PageChannel, options: NO_CHECKS } });
}

const ROUTE_LENGTH = 9;
const UNALIASED_KEY = 0;
const LAST_FRAME = 0x80;
const CONTINUATION = 0x80;
const PAYLOAD = 0x7f;
const SHIFT = 7;

export const CLOSE = 10;

export function parseFrames(batch: Uint8Array): { kind: number; payload: Uint8Array }[] {
  let offset = 0;
  const varuint = () => {
    let value = 0;

    for (let shift = 0; ; shift += SHIFT) {
      const byte = batch[offset++]!;

      value |= (byte & PAYLOAD) << shift;
      if ((byte & CONTINUATION) === 0) return value;
    }
  };

  offset++;

  const sliceCount = varuint();
  const frames = [];

  for (let slice = 0; slice < sliceCount; slice++) {
    const key = varuint();

    if (key % 2 === 1 || key === UNALIASED_KEY) offset += ROUTE_LENGTH;

    const end = slice + 1 === sliceCount ? batch.length : varuint() + offset;

    while (offset < end) {
      const kind = batch[offset++]!;
      const length = (kind & LAST_FRAME) === 0 ? varuint() : end - offset;

      frames.push({ kind: kind & ~LAST_FRAME, payload: batch.subarray(offset, offset + length) });
      offset += length;
    }
  }

  return frames;
}
