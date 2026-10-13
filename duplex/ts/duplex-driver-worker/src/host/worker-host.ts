import { ErrorCode, Transport, type Relay } from 'duplex-core';

import { MessageTargetDriver } from '../driver/message-target-driver.js';
import type { AnyChannel, ChannelDeclarationTable } from './channel-declaration.js';
import type { WorkerHostOptions } from './relay-declaration.js';
import type { WorkerLike } from './worker.js';

export class WorkerHost<C extends Record<string, AnyChannel>> {
  readonly channels: { readonly [K in keyof C]: C[K] };

  readonly transport: Transport;
  readonly closeEvent: Promise<ErrorCode>;
  readonly #worker: WorkerLike;
  readonly #relays: readonly Relay[];

  #isClosed = false;
  #resolveCloseEvent!: (reason: ErrorCode) => void;

  readonly #onError = () => {
    this.#end(ErrorCode.Panic);
  };

  constructor(
    worker: WorkerLike,
    declarations: ChannelDeclarationTable<C>,
    { relays = [], ports = [] }: WorkerHostOptions = {}
  ) {
    this.#worker = worker;
    this.closeEvent = new Promise((resolve) => {
      this.#resolveCloseEvent = resolve;
    });
    worker.addEventListener('error', this.#onError);

    const { port1, port2 } = new MessageChannel();

    this.#relays = relays.map(({ transport, protocol, peer, bundle }) => transport.relay(protocol, peer, bundle));
    worker.postMessage(
      Uint32Array.from([...relays, ...ports], ({ protocol }) => protocol.channelId),
      [port2, ...this.#relays.map(({ port }) => port), ...ports.map(({ port }) => port)]
    );
    this.transport = new Transport(new MessageTargetDriver(port1));

    const channels: Partial<Record<keyof C, AnyChannel>> = {};

    for (const name of Object.keys(declarations) as (keyof C)[]) {
      const { channelClass: channel, routers = [], options } = declarations[name];

      channels[name] = new channel(this.transport, options).use(...routers);
    }

    this.channels = channels as { readonly [K in keyof C]: C[K] };
  }

  open(): MessagePort {
    const { port1, port2 } = new MessageChannel();

    this.#worker.postMessage(new Uint32Array(), [port2]);

    return port1;
  }

  close(): void {
    if (!this.#end(ErrorCode.ClosedChannel)) return;
    this.#worker.terminate();
  }

  #end(reason: ErrorCode): boolean {
    if (this.#isClosed) return false;
    this.#isClosed = true;
    this.#worker.removeEventListener('error', this.#onError);
    for (const channel of Object.values<AnyChannel>(this.channels)) {
      channel.close(reason);
    }

    for (const relay of this.#relays) relay.close(reason);

    this.#resolveCloseEvent(reason);

    return true;
  }
}
