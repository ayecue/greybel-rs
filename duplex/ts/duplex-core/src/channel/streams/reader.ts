import { ERR_RESPONSE_END, ERR_NO_START } from '../../const.js';
import { ErrorCode } from '../../definition.generated.js';
import { ChannelError } from '../../errors.js';
import { FinalEventKind, NOTHING, type FinalEvent, type Inbox } from './inbox.js';

export interface ItemSource {
  ackItem(): void;
}

export interface ResponseSource extends ItemSource {
  cancel(): void;
}

const ITERATOR_END: IteratorResult<never> = { done: true, value: undefined };

const FINAL_EVENT_MARKER: unique symbol = Symbol('final event');

class ItemReader {
  readonly inbox: Inbox;
  readonly #source: ItemSource;
  #finalEvent: FinalEvent | undefined;

  constructor(source: ItemSource, inbox: Inbox) {
    this.#source = source;
    this.inbox = inbox;
  }

  get finalEvent(): FinalEvent | undefined {
    return this.#finalEvent;
  }

  nextEntry(): unknown {
    if (this.#finalEvent !== undefined) return FINAL_EVENT_MARKER;

    const inbox = this.inbox;
    const item = inbox.nextItem();

    if (item !== NOTHING) {
      this.#source.ackItem();

      return item;
    }

    const finalEvent = inbox.finalEvent;

    if (finalEvent === undefined) return NOTHING;
    this.#finalEvent = finalEvent;

    return FINAL_EVENT_MARKER;
  }

  async next(): Promise<IteratorResult<unknown>> {
    for (;;) {
      const entry = this.nextEntry();

      if (entry === NOTHING) await this.inbox.wait();
      else if (entry !== FINAL_EVENT_MARKER) return { done: false, value: entry };
      else if (this.#finalEvent!.kind === FinalEventKind.Error) throw (this.#finalEvent as { error: Error }).error;
      else return ITERATOR_END;
    }
  }

  async drain(): Promise<FinalEvent> {
    for (;;) {
      const entry = this.nextEntry();

      if (entry === FINAL_EVENT_MARKER) return this.#finalEvent!;
      if (entry !== NOTHING) continue;
      await this.inbox.wait();
    }
  }

  endWith(finalEvent: FinalEvent): void {
    this.#finalEvent ??= finalEvent;
  }
}

export class ResponseStream<S, E, I> implements AsyncIterableIterator<I> {
  readonly #source: ResponseSource;
  readonly #reader: ItemReader;

  constructor(source: ResponseSource, inbox: Inbox) {
    this.#source = source;
    this.#reader = new ItemReader(source, inbox);
  }

  async start(): Promise<S> {
    const items = this.#reader;

    for (;;) {
      const finalEvent = items.finalEvent;

      if (finalEvent !== undefined) throw ResponseStream.#getError(finalEvent, ERR_NO_START);

      const inbox = items.inbox;
      const start = inbox.takeStart();

      if (start !== NOTHING) return start as S;
      if (inbox.hasItems) throw ChannelError.reject(ErrorCode.MalformedMessage);
      if (items.nextEntry() !== NOTHING) continue;
      await inbox.wait();
    }
  }

  async end(): Promise<E> {
    const finalEvent = await this.#reader.drain();

    if (finalEvent.kind === FinalEventKind.Value) return finalEvent.value as E;
    throw ResponseStream.#getError(finalEvent, ERR_RESPONSE_END);
  }

  cancel(): void {
    if (this.#reader.finalEvent !== undefined) return;
    this.#reader.endWith({ kind: FinalEventKind.Error, error: ChannelError.reject(ErrorCode.Cancellation) });
    this.#source.cancel();
  }

  next(): Promise<IteratorResult<I>> {
    return this.#reader.next() as Promise<IteratorResult<I>>;
  }

  return(): Promise<IteratorResult<I>> {
    this.cancel();

    return Promise.resolve(ITERATOR_END);
  }

  [Symbol.asyncIterator](): AsyncIterableIterator<I> {
    return this;
  }

  static #getError(finalEvent: FinalEvent, message: string): Error {
    return finalEvent.kind === FinalEventKind.Error ? finalEvent.error : new RangeError(message);
  }
}

export class IncomingStream<E, I> implements AsyncIterableIterator<I> {
  readonly #reader: ItemReader;

  constructor(source: ItemSource, inbox: Inbox) {
    this.#reader = new ItemReader(source, inbox);
  }

  async end(): Promise<E> {
    const finalEvent = await this.#reader.drain();

    if (finalEvent.kind === FinalEventKind.Error) throw finalEvent.error;

    return finalEvent.value as E;
  }

  next(): Promise<IteratorResult<I>> {
    return this.#reader.next() as Promise<IteratorResult<I>>;
  }

  [Symbol.asyncIterator](): AsyncIterableIterator<I> {
    return this;
  }
}
