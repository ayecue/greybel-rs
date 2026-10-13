import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { ChannelError, ErrorCode, type ResponseWriter } from '../../../src/index.js';
import * as client from '../../calculator.client.generated.js';
import { Phase } from '../../calculator.generated.js';
import * as provider from '../../calculator.provider.generated.js';
import { MemoryLink, NO_CHECKS, restoreMocksAndTimers, zeroRandomValues } from '../../support/channel.js';
import { BatchCounter, connected, Counter, exchange, STREAMING, WINDOW } from '../../support/streams.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('streams', () => {
  it('should stream a response: its start, its items and its end', async () => {
    const { link, user } = await connected();
    const stream = user.send(new client.Count({ to: 3 }));

    await exchange(link);
    expect(await stream.start()).toBe(Phase.Active);

    const items: number[] = [];

    for await (const item of stream) items.push(item);
    expect(items).toEqual([1, 2, 3]);
    expect(await stream.end()).toBe(6n);
  });

  it('should send no more items than the receiver takes', async () => {
    const { link, user, counter } = await connected();
    const stream = user.send(new client.Count({ to: WINDOW * 4 }));

    await exchange(link);

    const before = counter.sent.length;
    const items = stream[Symbol.asyncIterator]();

    for (let taken = 0; taken < WINDOW / 2; taken++) await items.next();
    await exchange(link);
    expect(before).toBe(WINDOW);
    expect(counter.sent.length).toBe(WINDOW + WINDOW / 2);
  });

  it('should cancel a stream broken off before its end, which stops its handler', async () => {
    const { link, user, counter } = await connected();
    const stream = user.send(new client.Count({ to: WINDOW * 4 }));

    await exchange(link);
    for await (const item of stream) if (item === 2) break;
    await exchange(link);
    expect(counter.sent.at(-1)).toEqual(ChannelError.reject(ErrorCode.Cancellation));
    await expect(stream.end()).rejects.toEqual(ChannelError.reject(ErrorCode.Cancellation));
  });

  it('should respond to what a message streams after it', async () => {
    const { link, user } = await connected();
    const { stream, response } = user.send(new client.Sum());
    const sending = (async () => {
      for (const term of [4n, 5n, 6n]) await stream.send(term);
      await stream.end(undefined);
    })();

    await exchange(link);
    await sending;
    await exchange(link);
    await expect(response).resolves.toBe(15n);
  });

  it('should send all items of a response as the receiver takes them', async () => {
    const link = new MemoryLink();
    const user = new client.UserChannel(link.client, STREAMING);

    new provider.CalculatorChannel(link.provider, STREAMING).use(new BatchCounter());
    await exchange(link);

    const to = WINDOW * 3;
    const stream = user.send(new client.Count({ to }));
    const items: number[] = [];
    const taking = (async () => {
      for await (const item of stream) items.push(item);
    })();

    while (items.length < to) await exchange(link);
    await exchange(link);
    await taking;
    expect(items).toEqual(Array.from({ length: to }, (_, index) => index + 1));
    expect(await stream.end()).toBe(BigInt((to * (to + 1)) / 2));
  });

  it('should send all items a message streams after it', async () => {
    const { link, user } = await connected();
    const count = WINDOW * 3;
    const { stream, response } = user.send(new client.Sum());
    const sending = stream.sendAll(Array.from({ length: count }, () => 1n)).then(() => stream.end(undefined));

    for (let round = 0; round < 8; round++) await exchange(link);
    await sending;
    await exchange(link);
    await expect(response).resolves.toBe(BigInt(count));
  });

  it('should end the stream of a queued message once it is sent', async () => {
    const link = new MemoryLink();
    const user = new client.UserChannel(link.client, NO_CHECKS);

    new provider.CalculatorChannel(link.provider, NO_CHECKS).use(new Counter());

    const { stream, response } = user.send(new client.Sum());
    const ended = stream.end(undefined);

    await new Promise((resolve) => setTimeout(resolve, 0));
    await exchange(link);
    await ended;
    await exchange(link);
    await expect(response).resolves.toBe(0n);
  });

  it('should wait for credit past its initial credit until the receiver grants more', async () => {
    const { link, user } = await connected();
    const { stream, response } = user.send(new client.Sum());

    await stream.send(1n);

    let sent = false;
    const waiting = stream.send(1n).then(() => (sent = true));

    await Promise.resolve();

    const before = sent;

    await exchange(link);
    await waiting;
    await stream.end(undefined);
    await exchange(link);
    expect(before).toBe(false);
    expect(sent).toBe(true);
    await expect(response).resolves.toBe(2n);
  });

  it('should reject an item past the most that wait for credit', async () => {
    const { user } = await connected();
    const { stream } = user.send(new client.Sum());
    const waiting = Array.from({ length: 1 + 4096 }, () => stream.send(1n));
    const past = stream.send(1n);

    await expect(past).rejects.toEqual(ChannelError.reject(ErrorCode.Overload));
    for (const send of waiting) send.catch(() => undefined);
  });

  it('should end a stream after the items sent before its end, though they wait for credit', async () => {
    const { link, user } = await connected();
    const { stream, response } = user.send(new client.Sum());
    const sends = Array.from({ length: WINDOW + 3 }, () => stream.send(1n));
    const ended = stream.end(undefined);

    await exchange(link);
    await Promise.all([...sends, ended]);
    await exchange(link);
    await expect(response).resolves.toBe(BigInt(WINDOW + 3));
  });

  it('should refuse to end a stream twice, or send after its end', async () => {
    const { user } = await connected();
    const { stream } = user.send(new client.Sum());
    const ended = stream.end(undefined);

    await expect(stream.end(undefined)).rejects.toEqual(ChannelError.channelEnd(ErrorCode.ClosedChannel));
    await expect(stream.send(1n)).rejects.toEqual(ChannelError.channelEnd(ErrorCode.ClosedChannel));
    await ended;
  });

  it('should fail a handler that sends an item before the start its response declares', async () => {
    const link = new MemoryLink();
    const user = new client.UserChannel(link.client, NO_CHECKS);

    new provider.CalculatorChannel(link.provider, NO_CHECKS).use(
      new (class extends provider.CalculatorRouter {
        override async onCount(_message: provider.Count, stream: ResponseWriter<Phase, number>): Promise<bigint> {
          await stream.send(1);

          return 1n;
        }
      })()
    );
    await exchange(link);

    const stream = user.send(new client.Count({ to: 1 }));

    await exchange(link);
    await expect(stream.end()).rejects.toEqual(ChannelError.reject(ErrorCode.HandlerFailure));
  });

  it('should fail a stream whose connection ends', async () => {
    const { link, user, calculator } = await connected();
    const stream = user.send(new client.Count({ to: WINDOW * 4 }));

    await exchange(link);
    calculator.close();
    await exchange(link);

    const items: number[] = [];

    await expect(
      (async () => {
        for await (const item of stream) items.push(item);
      })()
    ).rejects.toEqual(ChannelError.channelEnd(ErrorCode.ClosedChannel));
    expect(items).toHaveLength(WINDOW);
  });
});
