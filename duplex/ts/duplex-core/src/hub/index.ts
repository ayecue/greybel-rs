import { Emitter } from 'emitter';

import { Fifo } from '../channel/streams/fifo.js';
import { CLOCK_READ_INTERVAL, CYCLE_TIME_LIMIT_MS, CYCLE_ITEM_LIMIT, TURN_ITEM_LIMIT } from '../const.js';

export interface Schedulable {
  processTurn(limit: number): number;
  readonly hasMore: boolean;
  defer(task: () => void): void;
  nextTurn(task: () => void): void;
  now(): number;
}

export class Hub {
  static readonly current = new Hub();

  readonly #pendingTransports = new Fifo<Schedulable>();
  #afterEventEmitter = new Emitter<() => void>();
  readonly #cycleTask = (): void => {
    this.#scheduled = false;
    this.#runCycle();
  };

  #queuedCount = 0;
  #eventDepth = 0;
  #running = false;
  #scheduled = false;

  private constructor() {}

  get queuedCount(): number {
    return this.#queuedCount;
  }

  onEnqueue(transport: Schedulable, joins: boolean): void {
    this.#queuedCount++;
    if (joins) this.#pendingTransports.push(transport);
    if (this.#eventDepth !== 0) return;
    this.#schedule(transport.defer.bind(transport));
  }

  dropQueuedWork(count: number): void {
    this.#queuedCount = Math.max(this.#queuedCount - count, 0);
  }

  processAfterBatch(transport: Schedulable): void {
    if (this.#running || this.#queuedCount === 0) return;
    if (this.#queuedCount <= CYCLE_ITEM_LIMIT) this.#runCycle();
    else this.#schedule(transport.defer.bind(transport));
  }

  get inEvent(): boolean {
    return this.#eventDepth > 0;
  }

  afterEvent(task: () => void): void {
    this.#afterEventEmitter.emitter.add(task);
  }

  beginEvent(): void {
    this.#eventDepth++;
  }

  endEvent(): void {
    if (--this.#eventDepth !== 0) return;
    for (let tasks = this.#afterEventEmitter; tasks.emitter.isActive; tasks = this.#afterEventEmitter) {
      this.#afterEventEmitter = new Emitter();
      tasks.emit();
    }

    const front = this.#pendingTransports.peek();

    if (this.#running || this.#queuedCount <= 0 || front === undefined) return;
    this.#schedule(front.defer.bind(front));
  }

  #schedule(defer: (task: () => void) => void): void {
    if (this.#scheduled) return;
    this.#scheduled = true;
    defer(this.#cycleTask);
  }

  #runCycle(): void {
    if (this.#running) return;
    this.#running = true;
    this.beginEvent();

    const transports = this.#pendingTransports;
    let startTime: number | undefined;
    let completedWork = 0;
    let lastTransport: Schedulable | undefined;

    try {
      for (let transport = transports.shift(); transport !== undefined; transport = transports.shift()) {
        lastTransport = transport;
        startTime ??= transport.now();

        const processedCount = transport.processTurn(Math.min(TURN_ITEM_LIMIT, CYCLE_ITEM_LIMIT - completedWork));

        this.#queuedCount -= processedCount;
        if (transport.hasMore) transports.push(transport);

        const workBefore = completedWork;

        completedWork += processedCount;
        if (
          completedWork >= CYCLE_ITEM_LIMIT ||
          (Math.floor(completedWork / CLOCK_READ_INTERVAL) > Math.floor(workBefore / CLOCK_READ_INTERVAL) &&
            transport.now() - startTime >= CYCLE_TIME_LIMIT_MS)
        )
          break;
      }
    } finally {
      this.#running = false;

      const front = transports.peek() ?? lastTransport;

      if (this.#queuedCount > 0 && front !== undefined && !this.#scheduled) {
        this.#scheduled = true;
        front.nextTurn(this.#cycleTask);
      }

      this.endEvent();
    }
  }
}
