import { handleError } from "handle-error";
import type { Callback, Disposable, Handle } from "./types.js";

class ConsumerEmitter<TCallback extends Callback<any[]> = Callback> {
  protected _items: Array<TCallback | undefined> | undefined;
  protected _slots: Array<number> | undefined;

  get isActive(): boolean {
    return this._items !== undefined;
  }

  get size(): number {
    if (this._items === undefined) return 0;
    if (this._slots === undefined) return this._items.length;
    return this._items.length - this._slots.length;
  }

  add(item: TCallback): void {
    this._insert(item);
  }

  addDisposable(item: TCallback): Disposable {
    const handle: Handle<TCallback> = [this._insert(item), item, this._items!];
    return () => {
      this._remove(handle);
    };
  }

  protected _insert(item: TCallback): number {
    this._items ??= [];

    if (this._slots !== undefined) {
      const idx = this._slots.pop();
      if (idx === undefined) {
        this._slots = undefined;
        return this._insert(item);
      }
      this._items[idx] = item;
      return idx;
    }

    return this._items.push(item) - 1;
  }

  protected _remove<TCallback extends Callback<any[]> = Callback>(handle: Handle<TCallback>): boolean {
    const [idx, callback, origin] = handle;
    if (origin !== this._items) return false;
    // @ts-expect-error: this._items is initialized before this access.
    if (this._items?.[idx] !== callback) return false;
    if (this.size === 1) {
      this._items = undefined;
      this._slots = undefined;
      return true;
    }
    this._items[idx] = undefined;
    this._slots ??= [];
    this._slots.push(idx);
    return true;
  }

  dispose(): void {
    this._items = undefined;
    this._slots = undefined;
  }
}

export class Emitter<TCallback extends Callback<any[]> = Callback> {
  protected _consumer: ConsumerEmitter<TCallback> | undefined;
  protected _processing: boolean | undefined;

  get emitter(): ConsumerEmitter<TCallback> {
    return this._consumer ??= new ConsumerEmitter();
  }

  emit(...args: Parameters<TCallback>): void {
    if (this._processing) {
      throw new Error("Cannot emit while processing.");
    }

    this._processing = true;
    this._process(...args);
    this._processing = undefined;
  }

  protected _process(...args: Parameters<TCallback>) {
    // @ts-expect-error: accesses ConsumerEmitter._items, which is protected.
    const items = this._consumer?._items;
    if (items === undefined) return;
    const end = items.length;

    for (let index = 0; index < end; index++) {
      const callback = items[index];
      if (callback === undefined) continue;
      try {
        callback(...args);
      } catch (error) {
        handleError(error, this);
      }
      if (this._processing === undefined) return;
    }
  }

  dispose(): void {
    this._consumer = undefined;
    this._processing = undefined;
  }
}
