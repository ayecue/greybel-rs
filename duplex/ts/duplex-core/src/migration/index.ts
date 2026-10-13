import type { Protocol } from '../protocol/index.js';
import type { IMessage, MessageSegment } from '../protocol/message.js';

export abstract class Migration<Older extends Protocol = Protocol, Newer extends Protocol = Protocol> {
  constructor(
    readonly older: Older,
    readonly newer: Newer
  ) {}

  abstract olderType(type: IMessage): IMessage | undefined;

  abstract newerType(type: IMessage): IMessage | undefined;

  abstract rewrite(segment: MessageSegment, type: IMessage, value: unknown): unknown;
}
