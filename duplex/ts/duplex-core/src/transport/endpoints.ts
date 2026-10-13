import { Role, type ErrorCode } from '../definition.generated.js';
import { BatchReader, type Slice } from '../frame/batch.js';
import { InboundAliasTable } from './aliases.js';

export interface InboundSlice {
  readonly batch: Uint8Array;
  readonly slice: Slice;
  readonly connection: number | undefined;
  readonly core: number;
}

export interface Endpoint {
  receive(inbound: InboundSlice): void;
  onDisconnect(): void;
  close(reason: ErrorCode): void;
}

export class EndpointTable {
  readonly #endpoints = new Map<number, Endpoint>();
  readonly #aliases = new InboundAliasTable();

  get size(): number {
    return this.#endpoints.size;
  }

  toArray(): Endpoint[] {
    return [...this.#endpoints.values()];
  }

  attach(channel: number, role: Role, endpoint: Endpoint): () => boolean {
    const peerKey = channel * 2 + role;

    if (this.#endpoints.has(peerKey))
      throw new TypeError(`the transport already has a ${Role[role]} of the channel ${channel}`);
    this.#endpoints.set(peerKey, endpoint);

    return () => this.#endpoints.get(peerKey) === endpoint && this.#endpoints.delete(peerKey);
  }

  routeBatch(bytes: Uint8Array): void {
    const batch = BatchReader.of(bytes);

    if (batch === undefined) return;

    const endpoints = this.#endpoints;
    let slice: Slice | undefined;

    for (;;) {
      try {
        slice = batch.next();
      } catch {
        return;
      }

      if (slice === undefined) return;

      const route = this.#aliases.getRoute(slice.key);

      if (route === undefined) continue;

      const localRole = route.from === Role.Client ? Role.Provider : Role.Client;

      endpoints
        .get(route.channel * 2 + localRole)
        ?.receive({ batch: bytes, slice, connection: route.connection, core: batch.core });
    }
  }

  clearAliases(): void {
    this.#aliases.clear();
  }
}
