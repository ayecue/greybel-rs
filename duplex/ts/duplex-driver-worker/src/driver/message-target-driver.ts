import { Driver, type Receiver } from 'duplex-core';

import type { MessageListener, MessageTarget } from './message-target.js';

export class MessageTargetDriver extends Driver {
  readonly transport = 'worker';
  readonly #target: MessageTarget;

  constructor(target: MessageTarget) {
    super();
    this.#target = target;
  }

  open(receiver: Receiver): () => void {
    const listener: MessageListener = ({ data }) => {
      if (data instanceof Uint8Array) receiver.receive(data);
      else if (data instanceof ArrayBuffer) receiver.receive(new Uint8Array(data));
    };

    this.#target.addEventListener('message', listener);
    this.#target.start?.();

    return () => this.#target.removeEventListener('message', listener);
  }

  send(batch: Uint8Array): void {
    this.#target.postMessage(batch, [batch.buffer as ArrayBuffer]);
  }
}
