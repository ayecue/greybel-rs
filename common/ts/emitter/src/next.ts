import { handleError } from "handle-error";
import type { Callback, Disposable, Handle } from "./types.js";

class ConsumerEmitter<TCallback extends Callback<any[]> = Callback> {
  protected _items: Array<TCallback | undefined> | undefined;
  protected _undef: number | undefined;

  get size(): number {
    if (this._items === undefined) return 0;
    if (this._undef === undefined) return this._items.length;
    return this._items.length - this._undef;
  }

  add(item: TCallback): void {
    this._insert(item);
  }

  addDisposable(item: TCallback): Disposable {
    const handle: Handle<TCallback> = [this._insert(item), item, this._items];
    return () => {
      this._remove(handle);
    };
  }

  protected _insert(item: TCallback): number {
    return (this._items ??= []).push(item) - 1;
  }

  protected _remove(item: Handle<TCallback>): boolean {
    const [idx, callback, origin] = item;
    if (origin?.[idx] !== callback) return false;
    origin[idx] = undefined;
    if (origin === this._items) {
      this._undef ??= 0;
      this._undef++;
    }
    return true;
  }

  protected _clear(): void {
    this._items = undefined;
    this._undef = undefined;
  }
}

export class NextEmitter<TCallback extends Callback<any[]> = Callback> {
  protected _consumer: ConsumerEmitter<TCallback> | undefined;
  protected _processing: boolean | undefined;

  get emitter(): ConsumerEmitter<TCallback> {
    return this._consumer ??= new ConsumerEmitter();
  }

  emit(...args: Parameters<TCallback>): void {
    if (this._processing) throw new Error("Cannot emit while processing.");

    // @ts-expect-error: accesses ConsumerEmitter._items, which is protected.
    const items = this._consumer?._items;
    if (items === undefined) return;
    // @ts-expect-error: accesses ConsumerEmitter._clear, which is protected.
    this._consumer?._clear();
    const end = items.length;

    this._processing = true;

    for (let index = 0; index < end; index++) {
      const callback = items[index];
      if (callback === undefined) continue;
      try {
        callback(...args);
      } catch (error) {
        handleError(error, this);
      }
    }

    this._processing = undefined;
  }

  dispose(): void {
    this._consumer = undefined;
    this._processing = undefined;
  }
}
