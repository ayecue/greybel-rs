import { Fifo } from '../channel/streams/fifo.js';
import { PAUSE_THRESHOLD, RESUME_THRESHOLD } from '../const.js';
import type { Driver } from '../driver/index.js';
import type { Hub, Schedulable } from '../hub/index.js';

export interface Work {
  proceed(): void;
}

export class WorkQueue implements Schedulable {
  readonly #driver: Driver;
  readonly #hub: Hub;
  readonly #queue = new Fifo<Work>();
  #pending = false;
  #paused = false;

  constructor(driver: Driver, hub: Hub) {
    this.#driver = driver;
    this.#hub = hub;
  }

  enqueue(work: Work): void {
    const queue = this.#queue;

    queue.push(work);
    if (queue.size > PAUSE_THRESHOLD && !this.#paused) {
      this.#paused = true;
      this.#driver.pause();
    }

    const joins = !this.#pending;

    this.#pending = true;
    this.#hub.onEnqueue(this, joins);
  }

  clear(): void {
    this.#hub.dropQueuedWork(this.#queue.drain().length);
  }

  get hasMore(): boolean {
    return this.#pending;
  }

  get hasWork(): boolean {
    return this.#queue.size > 0;
  }

  processTurn(limit: number): number {
    const queue = this.#queue;
    let processedCount = 0;

    while (processedCount < limit) {
      const work = queue.shift();

      if (work === undefined) break;
      work.proceed();
      processedCount++;
    }

    if (this.#paused && queue.size < RESUME_THRESHOLD) {
      this.#paused = false;
      this.#driver.resume();
    }

    this.#pending = queue.size > 0;

    return processedCount;
  }

  defer(task: () => void): void {
    this.#driver.defer(task);
  }

  nextTurn(task: () => void): void {
    this.#driver.nextTurn(task);
  }

  now(): number {
    return this.#driver.now();
  }
}
