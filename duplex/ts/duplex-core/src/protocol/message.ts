import type { BinaryReader } from '../binary/reader.js';
import type { BinaryWriter } from '../binary/writer.js';
import { ANY_ITEMS, DEFAULT_MESSAGE_LIMIT } from '../const.js';
import type { Exchange, NoResponse, ReplyOf } from './exchange.js';
import type { FieldTagContainer, Tag } from './tags.js';

export const BODY: unique symbol = Symbol('body');

declare const SENDER: unique symbol;
declare const EXCHANGE: unique symbol;

export enum MessageSegment {
  Message = 0,
  MessageItem = 1,
  MessageEnd = 2,
  ResponseStart = 3,
  ResponseItem = 4,
  ResponseEnd = 5,
  Failure = 6
}

export abstract class Message<
  Sender extends number = number,
  Body extends object = object,
  X extends Exchange = NoResponse
> {
  declare readonly [SENDER]?: Sender;
  declare readonly [EXCHANGE]?: X;
  static readonly responds: boolean = false;
  static readonly streams: boolean = false;
  static readonly streamsResponse: boolean = false;
  static readonly startsResponse: boolean = false;
  static readonly limit: number = DEFAULT_MESSAGE_LIMIT;
  static readonly messageItemRange: readonly [number, number] = ANY_ITEMS;
  static readonly messageItemSize: number | undefined = undefined;
  static readonly responseItemSize: number | undefined = undefined;
  static readonly responseItemRange: readonly [number, number] = ANY_ITEMS;

  static readonly tags: readonly Tag[] = [];

  static readonly fields: readonly FieldTagContainer[] = [];
  protected readonly [BODY]: Body;

  constructor(body: Body) {
    this[BODY] = body;
  }

  abstract write(writer: BinaryWriter): void;
}

export type AnyMessage = Message<number, object, Exchange>;

export interface IMessage<M extends AnyMessage = AnyMessage> {
  new (body: never): M;
  readonly id: number;
  readonly since: number;
  readonly sender: number;
  readonly responds: boolean;
  readonly streams: boolean;
  readonly streamsResponse: boolean;
  readonly startsResponse: boolean;

  readonly timeout?: number;
  readonly limit: number;
  readonly messageItemRange: readonly [number, number];
  readonly messageItemSize: number | undefined;
  readonly responseItemSize: number | undefined;
  readonly responseItemRange: readonly [number, number];
  readonly tags: readonly Tag[];
  readonly fields: readonly FieldTagContainer[];
  read(reader: BinaryReader): M;
}

export type ExchangeOf<M> = M extends { readonly [EXCHANGE]?: infer X } ? Exclude<X, undefined> : never;

export type Reply<M> = ReplyOf<ExchangeOf<M>>;

interface StaticSegmentCodec {
  writeMessageItem?(writer: BinaryWriter, value: unknown): void;
  writeMessageEnd?(writer: BinaryWriter, value: unknown): void;
  writeResponseStart?(writer: BinaryWriter, value: unknown): void;
  writeResponseItem?(writer: BinaryWriter, value: unknown): void;
  writeResponseEnd?(writer: BinaryWriter, value: unknown): void;
  writeFailure?(writer: BinaryWriter, value: unknown): void;
  readMessageItem?(reader: BinaryReader): unknown;
  readMessageEnd?(reader: BinaryReader): unknown;
  readResponseStart?(reader: BinaryReader): unknown;
  readResponseItem?(reader: BinaryReader): unknown;
  readResponseEnd?(reader: BinaryReader): unknown;
  readFailure?(reader: BinaryReader): unknown;
}

export const SegmentCodec = {
  isResponse(segment: MessageSegment): boolean {
    return segment >= MessageSegment.ResponseStart;
  },

  write(type: IMessage, segment: MessageSegment, writer: BinaryWriter, value: unknown): void {
    const codecs = type as unknown as StaticSegmentCodec;

    switch (segment) {
      case MessageSegment.MessageItem:
        return codecs.writeMessageItem?.(writer, value);
      case MessageSegment.MessageEnd:
        return codecs.writeMessageEnd?.(writer, value);
      case MessageSegment.ResponseStart:
        return codecs.writeResponseStart?.(writer, value);
      case MessageSegment.ResponseItem:
        return codecs.writeResponseItem?.(writer, value);
      case MessageSegment.ResponseEnd:
        return codecs.writeResponseEnd?.(writer, value);
      case MessageSegment.Failure:
        return codecs.writeFailure?.(writer, value);
      case MessageSegment.Message:
        return undefined;
    }
  },

  read(type: IMessage, segment: MessageSegment, reader: BinaryReader): unknown {
    const codecs = type as unknown as StaticSegmentCodec;

    switch (segment) {
      case MessageSegment.MessageItem:
        return codecs.readMessageItem?.(reader);
      case MessageSegment.MessageEnd:
        return codecs.readMessageEnd?.(reader);
      case MessageSegment.ResponseStart:
        return codecs.readResponseStart?.(reader);
      case MessageSegment.ResponseItem:
        return codecs.readResponseItem?.(reader);
      case MessageSegment.ResponseEnd:
        return codecs.readResponseEnd?.(reader);
      case MessageSegment.Failure:
        return codecs.readFailure?.(reader);
      case MessageSegment.Message:
        return undefined;
    }
  }
};
