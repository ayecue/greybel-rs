import { Emitter } from 'emitter';

import { Fifo } from './fifo.js';

export enum FinalEventKind {
  Value,
  Error
}

export type FinalEvent =
  | { readonly kind: FinalEventKind.Value; readonly value: unknown }
  | { readonly kind: FinalEventKind.Error; readonly error: Error };

export const NOTHING: unique symbol = Symbol('nothing');

export class Inbox {
  #startValue: unknown = NOTHING;
  #items: Fifo<unknown> | undefined;
  #finalEvent: FinalEvent | undefined;
  #waiters: Emitter<() => void> | undefined;

  get finalEvent(): FinalEvent | undefined {
    return this.#items === undefined || this.#items.size === 0 ? this.#finalEvent : undefined;
  }

  get hasItems(): boolean {
    return this.#items !== undefined && this.#items.size > 0;
  }

  start(value: unknown): void {
    if (this.#finalEvent !== undefined) return;
    this.#startValue = value;
    this.#wakeWaiters();
  }

  push(value: unknown): void {
    if (this.#finalEvent !== undefined) return;
    (this.#items ??= new Fifo()).push(value);
    this.#wakeWaiters();
  }

  end(finalEvent: FinalEvent): void {
    if (this.#finalEvent !== undefined) return;
    this.#finalEvent = finalEvent;
    this.#wakeWaiters();
  }

  takeStart(): unknown {
    const start = this.#startValue;

    this.#startValue = NOTHING;

    return start;
  }

  nextItem(): unknown {
    const items = this.#items;

    return items === undefined || items.size === 0 ? NOTHING : items.shift();
  }

  wait(): Promise<void> {
    return new Promise((resolve) => (this.#waiters ??= new Emitter()).emitter.add(resolve));
  }

  #wakeWaiters(): void {
    const waiters = this.#waiters;

    if (waiters === undefined) return;
    this.#waiters = undefined;
    waiters.emit();
  }
}
