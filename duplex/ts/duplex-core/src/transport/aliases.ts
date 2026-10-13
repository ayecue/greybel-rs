import { FIRST_ALIAS, MAX_ALIASES } from '../const.js';
import { SliceKeyKind, type Route, type SliceKey } from '../frame/route.js';

export class OutboundAliasTable {
  readonly #aliasesByPeer = new Map<number, Map<number, number>>();
  readonly #vacantAliases: number[] = [];
  readonly #releasedRoutes: Route[] = [];
  #nextAlias = FIRST_ALIAS;

  assignKey(route: Route): SliceKey {
    const { connection } = route;

    if (connection === undefined) return { kind: SliceKeyKind.NoAlias, route };

    const peerKey = route.channel * 2 + route.from;
    let connections = this.#aliasesByPeer.get(peerKey);
    const knownAlias = connections?.get(connection);

    if (knownAlias !== undefined) return { kind: SliceKeyKind.Alias, alias: knownAlias };

    const alias = this.#allocateAlias();

    if (alias === undefined) return { kind: SliceKeyKind.NoAlias, route };
    if (connections === undefined) {
      connections = new Map();
      this.#aliasesByPeer.set(peerKey, connections);
    }

    connections.set(connection, alias);

    return { kind: SliceKeyKind.Announcement, alias, route };
  }

  release(route: Route): void {
    this.#releasedRoutes.push(route);
  }

  reclaimAliases(): void {
    const releasedRoutes = this.#releasedRoutes;

    if (releasedRoutes.length === 0) return;
    for (const route of releasedRoutes) {
      if (route.connection === undefined) continue;

      const connections = this.#aliasesByPeer.get(route.channel * 2 + route.from);
      const alias = connections?.get(route.connection);

      if (alias === undefined) continue;
      connections!.delete(route.connection);
      this.#vacantAliases.push(alias);
    }

    releasedRoutes.length = 0;
  }

  clear(): void {
    this.#aliasesByPeer.clear();
    this.#vacantAliases.length = 0;
    this.#releasedRoutes.length = 0;
    this.#nextAlias = FIRST_ALIAS;
  }

  #allocateAlias(): number | undefined {
    if (this.#vacantAliases.length > 0) {
      const smallest = Math.min(...this.#vacantAliases);

      this.#vacantAliases.splice(this.#vacantAliases.indexOf(smallest), 1);

      return smallest;
    }

    if (this.#nextAlias > MAX_ALIASES) return undefined;

    return this.#nextAlias++;
  }
}

export class InboundAliasTable {
  readonly #routes = new Map<number, Route>();

  getRoute(key: SliceKey): Route | undefined {
    switch (key.kind) {
      case SliceKeyKind.NoAlias:
        return key.route;
      case SliceKeyKind.Announcement:
        if (key.alias <= MAX_ALIASES) this.#routes.set(key.alias, key.route);

        return key.route;
      case SliceKeyKind.Alias:
        return this.#routes.get(key.alias);
    }
  }

  clear(): void {
    this.#routes.clear();
  }
}
