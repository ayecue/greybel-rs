import { describe, expect, it } from 'vitest';

import { Hub } from '../../src/index.js';
import { WorkQueue } from '../../src/transport/work-queue.js';
import { CycleDriver, enqueue, runAll } from '../support/work-queue.js';

describe('processing', () => {
  it('should handle what a transport read once its batch was read, in the order it came', () => {
    const driver = new CycleDriver();
    const processing = new WorkQueue(driver, Hub.current);
    const handled: number[] = [];

    enqueue(processing, handled, 3);

    const before = handled.length;

    Hub.current.processAfterBatch(processing);
    expect(before).toBe(0);
    expect(handled).toEqual([0, 1, 2]);
    runAll(driver);
  });

  it('should handle at most its budget in a cycle and go on after other work', () => {
    const driver = new CycleDriver();
    const processing = new WorkQueue(driver, Hub.current);
    const handled: number[] = [];

    enqueue(processing, handled, 1500);
    Hub.current.processAfterBatch(processing);
    driver.runCycle();

    const firstCycle = handled.length;
    const continuesLater = driver.later.length;

    driver.runCycle();
    expect(firstCycle).toBe(1024);
    expect(continuesLater).toBe(1);
    expect(handled.length).toBe(1500);
  });

  it('should stop reading while too much waits to be handled', () => {
    const driver = new CycleDriver();
    const processing = new WorkQueue(driver, Hub.current);
    const handled: number[] = [];

    enqueue(processing, handled, 5000);
    Hub.current.processAfterBatch(processing);

    const paused = driver.paused;

    runAll(driver);
    expect(paused).toBe(true);
    expect(driver.paused).toBe(false);
    expect(handled.length).toBe(5000);
  });

  it('should take turns between transports and stop reading only the busy one', () => {
    const busyDriver = new CycleDriver();
    const calmDriver = new CycleDriver();
    const busy = new WorkQueue(busyDriver, Hub.current);
    const calm = new WorkQueue(calmDriver, Hub.current);
    const busyHandled: number[] = [];
    const calmHandled: number[] = [];

    enqueue(busy, busyHandled, 5000);
    enqueue(calm, calmHandled, 1);
    Hub.current.processAfterBatch(calm);

    const paused = [busyDriver.paused, calmDriver.paused];

    busyDriver.runCycle();
    expect(paused).toEqual([true, false]);
    expect(calmHandled).toEqual([0]);
    runAll(busyDriver, calmDriver);
    expect(busyHandled.length).toBe(5000);
    expect(Hub.current.queuedCount).toBe(0);
  });
});
