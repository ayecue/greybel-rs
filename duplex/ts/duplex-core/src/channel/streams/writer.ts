import { ErrorCode } from '../../definition.generated.js';
import { ChannelError } from '../../errors.js';
import { MessageSegment } from '../../protocol/message.js';
import type { ItemSink, Outbox } from './outbox.js';

export interface StreamSink extends ItemSink {
  cancel(): void;
}

export class StreamWriter<E, I> {
  readonly #sink: StreamSink;
  readonly #outbox: Outbox;
  #ended = false;

  constructor(sink: StreamSink, outbox: Outbox) {
    this.#sink = sink;
    this.#outbox = outbox;
  }

  send(item: I): Promise<void> {
    if (this.#ended) return Promise.reject(ChannelError.channelEnd(ErrorCode.ClosedChannel));

    return this.#outbox.send(item);
  }

  sendAll(items: Iterable<I>): Promise<void> {
    if (this.#ended) return Promise.reject(ChannelError.channelEnd(ErrorCode.ClosedChannel));

    return this.#outbox.sendAll(items);
  }

  async end(value: E): Promise<void> {
    if (this.#ended) throw ChannelError.channelEnd(ErrorCode.ClosedChannel);
    this.#ended = true;
    await this.#outbox.end();

    const error = this.#outbox.error;

    if (error !== undefined) throw error;
    this.#sink.writeSegment(MessageSegment.MessageEnd, value);
  }

  cancel(): void {
    if (this.#ended) return;
    this.#ended = true;
    this.#sink.cancel();
  }
}

export class ResponseWriter<S, I> {
  readonly #sink: ItemSink;
  readonly #outbox: Outbox;
  readonly #declaresStart: boolean;
  #started = false;

  constructor(sink: ItemSink, outbox: Outbox, declaresStart: boolean) {
    this.#sink = sink;
    this.#outbox = outbox;
    this.#declaresStart = declaresStart;
  }

  start(value: S): void {
    if (this.#started) throw new RangeError('the response started already');
    this.#started = true;
    this.#sink.writeSegment(MessageSegment.ResponseStart, value);
  }

  send(item: I): Promise<void> {
    try {
      this.#startWithoutValue();
    } catch (error) {
      return Promise.reject(error as Error);
    }

    return this.#outbox.send(item);
  }

  sendAll(items: Iterable<I>): Promise<void> {
    try {
      this.#startWithoutValue();
    } catch (error) {
      return Promise.reject(error as Error);
    }

    return this.#outbox.sendAll(items);
  }

  finish(): Promise<void> {
    try {
      this.#startWithoutValue();
    } catch (error) {
      return Promise.reject(error as Error);
    }

    return this.#outbox.end();
  }

  #startWithoutValue(): void {
    if (this.#started) return;
    if (this.#declaresStart) throw new RangeError('the response sends its start first');
    this.start(undefined as S);
  }
}
