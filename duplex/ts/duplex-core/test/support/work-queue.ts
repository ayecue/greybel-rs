import { Driver, type Receiver } from '../../src/index.js';
import { WorkQueue } from '../../src/transport/work-queue.js';

export class CycleDriver extends Driver {
  readonly transport = 'memory';
  readonly deferred: (() => void)[] = [];
  readonly later: (() => void)[] = [];
  paused = false;

  open(_receiver: Receiver): () => void {
    return () => undefined;
  }

  send(): void {}

  override defer(task: () => void): void {
    this.deferred.push(task);
  }

  override nextTurn(task: () => void): void {
    this.later.push(task);
  }

  override now(): number {
    return 0;
  }

  override pause(): void {
    this.paused = true;
  }

  override resume(): void {
    this.paused = false;
  }

  runCycle(): void {
    (this.deferred.shift() ?? this.later.shift())?.();
  }
}

export function enqueue(processing: WorkQueue, handled: number[], count: number): void {
  for (let index = 0; index < count; index++) processing.enqueue({ proceed: () => handled.push(index) });
}

export function runAll(...drivers: CycleDriver[]): void {
  while (drivers.some((driver) => driver.deferred.length + driver.later.length > 0))
    for (const driver of drivers) driver.runCycle();
}
