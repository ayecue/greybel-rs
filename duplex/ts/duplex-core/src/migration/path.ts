import { ChannelError } from '../errors.js';
import type { Protocol } from '../protocol/index.js';
import type { IMessage, MessageSegment } from '../protocol/message.js';
import type { Migration } from './index.js';

export class MigrationPath {
  constructor(readonly migrations: readonly Migration[]) {}

  get oldestProtocol(): Protocol {
    return this.migrations[0]!.older;
  }

  migrateUp(segment: MessageSegment, type: IMessage, value: unknown): unknown {
    let current = type;
    let rewritten = value;

    for (const migration of this.migrations) {
      rewritten = migration.rewrite(segment, current, rewritten);

      const newer = migration.newerType(current);

      if (newer === undefined) throw ChannelError.unavailable();
      current = newer;
    }

    return rewritten;
  }

  migrateDown(segment: MessageSegment, type: IMessage, value: unknown): unknown {
    let current = type;
    let rewritten = value;

    for (let index = this.migrations.length - 1; index >= 0; index--) {
      const migration = this.migrations[index]!;

      rewritten = migration.rewrite(segment, current, rewritten);

      const older = migration.olderType(current);

      if (older === undefined) throw ChannelError.unavailable();
      current = older;
    }

    return rewritten;
  }

  getNewestType(type: IMessage): IMessage {
    let current = type;

    for (const migration of this.migrations) {
      const newer = migration.newerType(current);

      if (newer === undefined) throw ChannelError.unavailable();
      current = newer;
    }

    return current;
  }

  getOldestType(type: IMessage): IMessage {
    let current = type;

    for (let index = this.migrations.length - 1; index >= 0; index--) {
      const older = this.migrations[index]!.olderType(current);

      if (older === undefined) throw ChannelError.unavailable();
      current = older;
    }

    return current;
  }
}

export class MigrationTable {
  readonly #migrations = new Map<number, Migration>();

  add(migration: Migration): void {
    this.#migrations.set(migration.older.version.major, migration);
  }

  path(major: number, native: number): MigrationPath | undefined {
    const steps: Migration[] = [];

    for (let from = major; from < native; from++) {
      const migration = this.#migrations.get(from);

      if (migration === undefined) return undefined;
      steps.push(migration);
    }

    return steps.length === 0 ? undefined : new MigrationPath(steps);
  }

  getReachableMajors(native: number): number[] {
    const majors: number[] = [];

    for (let major = 0; major < native; major++) if (this.path(major, native) !== undefined) majors.push(major);
    majors.push(native);

    return majors;
  }
}
