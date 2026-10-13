import { Emitter } from 'emitter';

import { MAX_PENDING_ITEMS } from '../../const.js';
import { ErrorCode } from '../../definition.generated.js';
import { ChannelError } from '../../errors.js';
import type { MessageSegment } from '../../protocol/message.js';
import { Fifo } from './fifo.js';

export interface ItemSink {
  writeSegment(segment: MessageSegment, value: unknown): void;
}

interface PendingItem {
  readonly item: unknown;
  resolve(): void;
  reject(error: unknown): void;
}

const RESOLVED_WRITE: Promise<void> = Promise.resolve();

export class Outbox {
  readonly #sink: ItemSink;
  readonly #segment: MessageSegment;
  #credit = 0;
  #opened = false;
  #closed = false;
  #error: ChannelError | undefined;
  #pendingItems: Fifo<PendingItem> | undefined;
  #drainWaiters: Emitter<() => void> | undefined;
  #creditWaiters: Emitter<() => void> | undefined;

  constructor(sink: ItemSink, segment: MessageSegment) {
    this.#sink = sink;
    this.#segment = segment;
  }

  get error(): ChannelError | undefined {
    if (this.#error === undefined && this.#closed) this.#error = ChannelError.channelEnd(ErrorCode.ClosedChannel);

    return this.#error;
  }

  get isWaitingForCredit(): boolean {
    return this.#credit === 0 && !this.isDrained && this.#error === undefined;
  }

  get isDrained(): boolean {
    return this.#pendingItems === undefined || this.#pendingItems.size === 0;
  }

  send(item: unknown): Promise<void> {
    if (this.#closed || this.#error !== undefined) return Promise.reject(this.error);
    if (this.#credit === 0 || !this.isDrained) {
      const pendingItems = (this.#pendingItems ??= new Fifo());

      if (pendingItems.size >= MAX_PENDING_ITEMS) return Promise.reject(ChannelError.reject(ErrorCode.Overload));

      return new Promise((resolve, reject) => pendingItems.push({ item, resolve, reject }));
    }

    this.#credit--;
    try {
      this.#sink.writeSegment(this.#segment, item);
    } catch (error) {
      return Promise.reject(error as Error);
    }

    return RESOLVED_WRITE;
  }

  async sendAll(items: Iterable<unknown>): Promise<void> {
    const iterator = items[Symbol.iterator]();

    for (;;) {
      if (this.#closed || this.#error !== undefined) throw this.error;
      while (this.#credit > 0 && this.isDrained) {
        const next = iterator.next();

        if (next.done === true) return;
        this.#credit--;
        this.#sink.writeSegment(this.#segment, next.value);
      }

      await new Promise<void>((resolve) => (this.#creditWaiters ??= new Emitter()).emitter.add(resolve));
    }
  }

  end(): Promise<void> {
    if (this.#closed || this.#error !== undefined || (this.#opened && this.isDrained)) return RESOLVED_WRITE;

    return new Promise((resolve) => (this.#drainWaiters ??= new Emitter()).emitter.add(resolve));
  }

  grant(count: number): void {
    this.#credit += count;
    if (count > 0) this.#opened = true;

    const pendingItems = this.#pendingItems;

    if (pendingItems === undefined) {
      this.#wakeDrainWaiters();

      return this.#wakeCreditWaiters();
    }

    while (this.#credit > 0 && pendingItems.size > 0) {
      const pendingItem = pendingItems.shift()!;

      this.#credit--;
      try {
        this.#sink.writeSegment(this.#segment, pendingItem.item);
        pendingItem.resolve();
      } catch (error) {
        pendingItem.reject(error);
      }
    }

    if (pendingItems.size === 0) this.#wakeDrainWaiters();
    this.#wakeCreditWaiters();
  }

  close(): void {
    if (!this.isDrained) return this.fail(ChannelError.channelEnd(ErrorCode.ClosedChannel));
    this.#closed = true;
    this.#wakeDrainWaiters();
  }

  fail(error: ChannelError): void {
    this.#error = this.error ?? error;

    const pendingItems = this.#pendingItems;

    if (pendingItems !== undefined) for (const pendingItem of pendingItems.drain()) pendingItem.reject(this.#error);
    this.#wakeDrainWaiters();
    this.#wakeCreditWaiters();
  }

  #wakeCreditWaiters(): void {
    const waiters = this.#creditWaiters;

    if (waiters === undefined || (this.#credit === 0 && this.#error === undefined)) return;
    this.#creditWaiters = undefined;
    waiters.emit();
  }

  #wakeDrainWaiters(): void {
    const waiters = this.#drainWaiters;

    if (waiters === undefined) return;
    this.#drainWaiters = undefined;
    waiters.emit();
  }
}
