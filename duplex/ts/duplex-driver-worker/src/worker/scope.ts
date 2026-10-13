import { Transport, type Protocol } from 'duplex-core';

import { MessageTargetDriver } from '../driver/message-target-driver.js';
import type { MessageEventLike, MessageListener, MessageTarget } from '../driver/message-target.js';

export class WorkerScope {
  readonly transport: Transport;
  readonly #relays: ReadonlyMap<number, Transport>;

  private constructor(transport: Transport, relays: ReadonlyMap<number, Transport>) {
    this.transport = transport;
    this.#relays = relays;
  }

  static accept(scope: MessageTarget): Promise<WorkerScope> {
    return new Promise((resolve) => {
      const onPorts: MessageListener = (event) => {
        const workerScope = WorkerScope.#of(event);

        if (workerScope === undefined) return;
        scope.removeEventListener('message', onPorts);
        resolve(workerScope);
      };

      scope.addEventListener('message', onPorts);
      scope.start?.();
    });
  }

  static listen(scope: MessageTarget, onScope: (scope: WorkerScope) => void): () => void {
    const onPorts: MessageListener = (event) => {
      const workerScope = WorkerScope.#of(event);

      if (workerScope === undefined) return;
      onScope(workerScope);
    };

    scope.addEventListener('message', onPorts);
    scope.start?.();

    return () => scope.removeEventListener('message', onPorts);
  }

  static #of({ data, ports = [] }: MessageEventLike): WorkerScope | undefined {
    const [own, ...relayPorts] = ports;

    if (!(data instanceof Uint32Array) || own === undefined || relayPorts.length !== data.length) return undefined;

    const relays = new Map(
      relayPorts.map((port, index) => [data[index]!, new Transport(new MessageTargetDriver(port))] as const)
    );

    return new WorkerScope(new Transport(new MessageTargetDriver(own)), relays);
  }

  portTransportOf(protocol: Protocol): Transport | undefined {
    return this.#relays.get(protocol.channelId);
  }

  transportOf(protocol: Protocol): Transport {
    return this.#relays.get(protocol.channelId) ?? this.transport;
  }
}
