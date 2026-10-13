import type { ChildProcessByStdio } from 'node:child_process';
import type { Socket } from 'node:net';
import type { Readable, Writable } from 'node:stream';

import { BatchStream, Driver, type Receiver } from 'duplex-core';

export class StreamDriver extends Driver {
  readonly transport: string;
  readonly #readable: Readable;
  readonly #writable: Writable;

  constructor(readable: Readable, writable: Writable, transport = 'stream') {
    super();
    this.transport = transport;
    this.#readable = readable;
    this.#writable = writable;
  }

  static stdio(): StreamDriver {
    return new StreamDriver(process.stdin, process.stdout, 'stdio');
  }

  static child(child: ChildProcessByStdio<Writable, Readable, Readable | null>): StreamDriver {
    return new StreamDriver(child.stdout, child.stdin, 'stdio');
  }

  static tcp(socket: Socket): StreamDriver {
    socket.setNoDelay(true);

    return new StreamDriver(socket, socket, 'tcp');
  }

  open(receiver: Receiver): () => void {
    const stream = new BatchStream();
    const receive = (batch: Uint8Array) => receiver.receive(batch);
    const onData = (bytes: Uint8Array) => {
      try {
        stream.process(bytes, receive);
      } catch {
        receiver.onDisconnect();
      }
    };

    const onEnd = () => receiver.onDisconnect();
    const onError = () => receiver.onDisconnect();

    this.#readable.on('data', onData);
    this.#readable.on('end', onEnd);
    this.#readable.on('error', onError);
    this.#writable.on('error', onError);

    return () => {
      this.#readable.off('data', onData);
      this.#readable.off('end', onEnd);
      this.#readable.off('error', onError);
      this.#writable.off('error', onError);
    };
  }

  send(batch: Uint8Array): void {
    if (!this.#writable.writable) return;
    this.#writable.write(BatchStream.encode(batch));
  }

  override pause(): void {
    this.#readable.pause();
  }

  override resume(): void {
    this.#readable.resume();
  }
}
