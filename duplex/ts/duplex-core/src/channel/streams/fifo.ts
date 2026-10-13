import { FIFO_COMPACT_THRESHOLD } from '../../const.js';

export class Fifo<T> {
  #values: (T | undefined)[] = [];
  #head = 0;

  get size(): number {
    return this.#values.length - this.#head;
  }

  peek(): T | undefined {
    return this.#values[this.#head];
  }

  push(value: T): void {
    this.#values.push(value);
  }

  shift(): T | undefined {
    const values = this.#values;

    if (this.#head === values.length) return undefined;

    const value = values[this.#head];

    values[this.#head++] = undefined;
    if (this.#head === values.length) {
      values.length = 0;
      this.#head = 0;
    } else if (this.#head >= FIFO_COMPACT_THRESHOLD && this.#head * 2 >= values.length) {
      this.#values = values.slice(this.#head);
      this.#head = 0;
    }

    return value;
  }

  drain(): T[] {
    const remainingValues = this.#values.slice(this.#head) as T[];

    this.#values = [];
    this.#head = 0;

    return remainingValues;
  }
}
