import type { BinaryReader } from '../binary/reader.js';
import type { BinaryWriter } from '../binary/writer.js';
import { NO_CONNECTION, NO_ALIAS } from '../const.js';
import { Role } from '../definition.generated.js';

export interface Route {
  readonly from: Role;
  readonly channel: number;
  readonly connection: number | undefined;
}

export const Route = {
  write(writer: BinaryWriter, route: Route): void {
    Role.write(writer, route.from);
    writer.fixed32(route.channel);
    writer.fixed32(route.connection ?? NO_CONNECTION);
  },

  read(reader: BinaryReader): Route {
    const from = Role.read(reader);
    const channel = reader.fixed32();
    const connection = reader.fixed32();

    return { from, channel, connection: connection === NO_CONNECTION ? undefined : connection };
  },

  equals(a: Route, b: Route): boolean {
    return a === b || (a.channel === b.channel && a.from === b.from && a.connection === b.connection);
  },

  getPeerKey(route: Route): number {
    return route.channel * 2 + route.from;
  }
};

export enum SliceKeyKind {
  NoAlias,
  Announcement,
  Alias
}

export type SliceKey =
  | { readonly kind: SliceKeyKind.NoAlias; readonly route: Route }
  | { readonly kind: SliceKeyKind.Announcement; readonly alias: number; readonly route: Route }
  | { readonly kind: SliceKeyKind.Alias; readonly alias: number };

export const SliceKey = {
  write(writer: BinaryWriter, key: SliceKey): void {
    switch (key.kind) {
      case SliceKeyKind.NoAlias:
        writer.u32(NO_ALIAS);
        Route.write(writer, key.route);
        break;
      case SliceKeyKind.Announcement:
        writer.u32(key.alias * 2 - 1);
        Route.write(writer, key.route);
        break;
      case SliceKeyKind.Alias:
        writer.u32(key.alias * 2);
        break;
    }
  },

  read(reader: BinaryReader): SliceKey {
    const key = reader.u32();

    if (key === NO_ALIAS) return { kind: SliceKeyKind.NoAlias, route: Route.read(reader) };
    if (key % 2 === 1) return { kind: SliceKeyKind.Announcement, alias: (key + 1) / 2, route: Route.read(reader) };

    return { kind: SliceKeyKind.Alias, alias: key / 2 };
  }
};
