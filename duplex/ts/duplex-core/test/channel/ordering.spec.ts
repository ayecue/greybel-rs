import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { FrameKind } from '../../src/definition.generated.js';
import { Role } from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { Phase } from '../calculator.generated.js';
import { CONNECTION, connectedClient, restoreMocksAndTimers, settle, zeroRandomValues } from '../support/channel.js';
import { written, type TestFrame } from '../support/frames.js';
import { addResponse, notificationsThen } from '../support/ordering.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('response ordering', () => {
  it('should deliver a response after two thousand earlier notifications', async () => {
    const { driver, user } = await connectedClient();
    const received: string[] = [];

    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          received.push(message.phase.toString());
        }
      })()
    );

    const response = user.send(new client.Add({ a: 1n, b: 2n }));
    let handledWhenSettled = -1;

    void response.then(() => (handledWhenSettled = received.length));
    driver.deliver(Role.Client, CONNECTION, ...notificationsThen(2000, addResponse()));

    await settle();
    expect(handledWhenSettled).toBe(-1);
    await expect(response).resolves.toBe(3n);
    expect(handledWhenSettled).toBe(2000);
  });

  it('should keep response order when notification handlers exceed a cycle', async () => {
    const { driver, user } = await connectedClient();
    const received: client.Progress[] = [];
    let now = 0;

    vi.spyOn(driver, 'now').mockImplementation(() => now);
    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          received.push(message);
          now += 2;
        }
      })()
    );

    const response = user.send(new client.Add({ a: 1n, b: 2n }));
    let handledWhenSettled = -1;

    void response.then(() => (handledWhenSettled = received.length));
    driver.deliver(Role.Client, CONNECTION, ...notificationsThen(2000, addResponse()));

    await settle();
    expect(received).toHaveLength(64);
    expect(handledWhenSettled).toBe(-1);
    await expect(response).resolves.toBe(3n);
    expect(handledWhenSettled).toBe(2000);
  });

  it('should deliver a response with an empty queue in the same read', async () => {
    const { driver, user } = await connectedClient();
    const deferred = vi.spyOn(driver, 'defer');
    const response = user.send(new client.Add({ a: 1n, b: 2n }));
    let settled = false;

    void response.then(() => (settled = true));
    deferred.mockClear();
    driver.deliver(Role.Client, CONNECTION, addResponse());

    expect(deferred).not.toHaveBeenCalled();
    await settle();
    expect(settled).toBe(true);
    await expect(response).resolves.toBe(3n);
  });

  it('should neither time out nor cancel a received response waiting for earlier work', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });

    const { driver, user } = await connectedClient();
    const received: client.Progress[] = [];
    const controller = new AbortController();

    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          received.push(message);
        }
      })()
    );

    const response = user.send(new client.Add({ a: 1n, b: 2n }), { signal: controller.signal });

    driver.deliver(Role.Client, CONNECTION, ...notificationsThen(1025, addResponse()));
    vi.advanceTimersByTime(2000);
    controller.abort();

    await expect(response).resolves.toBe(3n);
    expect(received).toHaveLength(1025);
  });

  it("should deliver a streamed response's items after earlier notifications", async () => {
    const { driver, user } = await connectedClient();
    const received: client.Progress[] = [];

    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          received.push(message);
        }
      })()
    );

    const stream = user.send(new client.Count({ to: 1 }));
    const start: TestFrame = {
      kind: FrameKind.ResponseStart,
      id: 0,
      body: written((writer) => client.Count.writeResponseStart(writer, Phase.Active))
    };
    const item: TestFrame = {
      kind: FrameKind.ResponseItem,
      id: 0,
      body: written((writer) => client.Count.writeResponseItem(writer, 1))
    };
    const end: TestFrame = {
      kind: FrameKind.ResponseEnd,
      id: 0,
      body: written((writer) => client.Count.writeResponseEnd(writer, 1n))
    };

    driver.deliver(Role.Client, CONNECTION, ...notificationsThen(1025, start, item, end));

    await expect(stream.start()).resolves.toBe(Phase.Active);

    let handledWhenRead = -1;
    const next = stream.next().then((result) => {
      handledWhenRead = received.length;

      return result;
    });

    await settle();
    expect(handledWhenRead).toBe(-1);
    await expect(next).resolves.toEqual({ done: false, value: 1 });
    expect(handledWhenRead).toBe(1025);
    await expect(stream.end()).resolves.toBe(1n);
  });
});
