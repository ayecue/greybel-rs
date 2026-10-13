import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';

import { Health } from '../../src/channel/connection/health.js';
import { NO_CONNECTION } from '../../src/const.js';
import { FrameKind } from '../../src/definition.generated.js';
import { ChannelError, ErrorCode, Role, type Next, type ResponseWriter, type RouteContext } from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { DivisionFailure, Phase, Quotient, Step } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import {
  ack,
  Adder,
  batch,
  Confirmer,
  connectedClient,
  connectedProvider,
  CONNECTION,
  Delayer,
  Divider,
  Failer,
  HEALTH_CHECK,
  logFrame,
  MemoryDriver,
  MemoryLink,
  nextLog,
  nextProgress,
  NO_CHECKS,
  pair,
  pongEach,
  progress,
  restoreMocksAndTimers,
  routedPair,
  settle,
  watchedClient,
  Wrong,
  zeroRandomValues
} from '../support/channel.js';
import { body } from '../support/frames.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('the batches', () => {
  it('should post what one task writes in one batch, after the task', async () => {
    const { driver, user } = await connectedClient();

    user.send(new client.Log({ text: 'a' }));
    user.send(new client.Log({ text: 'b' }));

    const before = driver.take();

    await settle();
    expect(before).toEqual([]);
    expect(driver.take()).toEqual([batch(Role.Client, CONNECTION, logFrame('a'), logFrame('b'))]);
  });

  it('should post a full batch at once', async () => {
    const { driver, user } = await connectedClient();

    user.send(new client.Log({ text: 'x'.repeat(70_000) }));
    expect(driver.take()).toHaveLength(1);
  });

  it('should carry a message past a batch in batches of a batch at most', async () => {
    const link = new MemoryLink();
    const calculator = new provider.CalculatorChannel(link.provider, NO_CHECKS);
    const user = new client.UserChannel(link.client, NO_CHECKS);

    link.deliver();

    const logged = nextLog(calculator);
    const text = 'x'.repeat(1 << 20);

    user.send(new client.Log({ text }));
    link.deliver();
    expect((await logged).text).toBe(text);

    const large = link.delivered.filter((batch) => batch.length > 1024);

    expect(large.length).toBeGreaterThanOrEqual(16);
    expect(link.delivered.every((batch) => batch.length <= 64 * 1024 + 16)).toBe(true);
  });

  it('should fail a message past the most a message takes and send nothing', async () => {
    const { driver, user } = await connectedClient({ ...NO_CHECKS, maxMessage: 1024 });
    const sending = () => user.send(new client.Log({ text: 'x'.repeat(2048) }));

    expect(sending).toThrow(ChannelError.reject(ErrorCode.Overload));
    await settle();
    expect(driver.take()).toEqual([]);
  });

  it('should reject a message whose handler takes longer than its timeout', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });

    const link = new MemoryLink();
    const calculator = new provider.CalculatorChannel(link.provider, NO_CHECKS);
    let signal: AbortSignal | undefined;

    calculator.use(
      new (class extends provider.CalculatorRouter {
        override onCount(
          _message: provider.Count,
          _stream: ResponseWriter<Phase, number>,
          _next: Next<bigint>,
          context: RouteContext
        ): Promise<bigint> {
          signal = context.signal;

          return new Promise(() => undefined);
        }
      })()
    );

    const user = new client.UserChannel(link.client, NO_CHECKS);

    link.deliver();

    const stream = user.send(new client.Count({ to: 3 }));

    link.deliver();
    await Promise.resolve();
    link.deliver();
    vi.advanceTimersByTime(1000);
    link.deliver();
    await expect(stream.start()).rejects.toEqual(ChannelError.reject(ErrorCode.Timeout));
    expect(signal?.aborted).toBe(true);
  });

  it('should ping, and post at once on close', async () => {
    const { driver, user } = await connectedClient(HEALTH_CHECK);

    user.runHealthCheck();
    user.close();
    expect(driver.take()).toEqual([
      batch(
        Role.Client,
        CONNECTION,
        { kind: FrameKind.Ping },
        { kind: FrameKind.Close, reason: ErrorCode.ClosedChannel }
      )
    ]);
  });
});

describe('Channel', () => {
  it('should respond to a request the client sends', async () => {
    const { user, calculator } = pair();

    calculator.use(new Divider());

    const quotient = await user.send(new client.Divide({ dividend: 7n, divisor: 3n }));

    expect(quotient).toEqual(new Quotient.WithRemainder({ value: 2n, remainder: 1n }));
  });

  it('should respond to a request the provider sends', async () => {
    const { user, calculator } = pair();

    user.use(new Confirmer());

    const confirmed = calculator.send(new provider.Confirm({ question: 'continue?' }));

    await expect(confirmed).resolves.toBe(true);
  });

  it('should route notifications as message instances', async () => {
    const { user, calculator } = pair();
    const logged = nextLog(calculator);

    user.send(new client.Log({ text: 'hello' }));

    const message = await logged;

    expect(message).toBeInstanceOf(provider.Log);
    expect(message.text).toBe('hello');
  });

  it('should make the data a message carries instances of its classes', async () => {
    const { user, calculator } = pair();
    const received = nextProgress(user);

    calculator.send(progress('almost'));

    const message = await received;

    expect(message.phase).toBe(Phase.Active);
    expect(message.steps[0]).toBeInstanceOf(Step);
    expect(message.steps[0]!.result).toEqual(new Quotient.WithRemainder({ value: 2n, remainder: 1n }));
    expect(message.steps[1]!.result).toBeNull();
    expect(message.counts).toEqual({ divisions: 1 });
    expect(message.note).toBe('almost');
  });

  it('should run its routers in order, `next()` returning the reply after it', async () => {
    const { user } = routedPair();
    const added = user.send(new client.Add({ a: 1n, b: 2n }));

    await expect(added).resolves.toBe(6n);
  });

  it('should pass a message its routers do not handle on to the next ones', async () => {
    const { user } = routedPair();
    const divided = user.send(new client.Divide({ dividend: 6n, divisor: 3n }));

    await expect(divided).resolves.toEqual(new Quotient.Exact({ value: 2n }));
  });

  it('should route a notification to its first router handling it', async () => {
    const { user, logger } = routedPair();
    const logged = logger.next();

    user.send(new client.Log({ text: 'adding' }));
    await logged;
    expect(logger.logged).toEqual(['adding']);
  });

  it('should fail a request whose router throws', async () => {
    const { user, calculator } = pair();

    calculator.use(new Failer());

    const added = user.send(new client.Add({ a: 1n, b: 2n }));

    await expect(added).rejects.toEqual(ChannelError.reject(ErrorCode.HandlerFailure));
  });

  it('should fail a request with the failure its router rejects with', async () => {
    const { user, calculator } = pair();

    calculator.use(new Failer());

    const divided = user.send(new client.Divide({ dividend: 1n, divisor: 0n }));

    await expect(divided).rejects.toEqual(ChannelError.fail(DivisionFailure.ByZero));
  });

  it('should fail a request no router handles', async () => {
    const { user } = pair();
    const added = user.send(new client.Add({ a: 1n, b: 2n }));

    await expect(added).rejects.toEqual(ChannelError.reject(ErrorCode.UnhandledMessage));
  });

  it('should fail a request whose reply does not write', async () => {
    const { user, calculator } = pair();

    calculator.use(new Wrong());

    const added = user.send(new client.Add({ a: 1n, b: 2n }));

    await expect(added).rejects.toEqual(ChannelError.reject(ErrorCode.MalformedMessage));
  });

  it('should reject a request that does not write, retaining nothing of it', async () => {
    const { driver, user } = await connectedClient();
    const added = user.send(new client.Add({ a: 2n ** 64n, b: 0n }));

    user.send(new client.Log({ text: 'after' }));
    await settle();
    await expect(added).rejects.toThrow(RangeError);
    expect(driver.take()).toEqual([batch(Role.Client, CONNECTION, logFrame('after'))]);
  });

  it('should run the health check on a timer of the driver', async () => {
    const driver = new MemoryDriver();
    const interval = vi.spyOn(driver, 'interval');
    const user = new client.UserChannel(driver, HEALTH_CHECK);

    await settle();
    user.close();
    expect(interval).toHaveBeenCalledWith(HEALTH_CHECK.healthCheck.interval, expect.any(Function));
  });

  it('should type messages, replies and routers by the protocol', () => {
    const user = new client.UserChannel(new MemoryDriver(), NO_CHECKS);
    const calculator = new provider.CalculatorChannel(new MemoryDriver(), NO_CHECKS);

    onTestFinished(() => {
      user.close();
      calculator.close();
    });
    // @ts-expect-error: the provider sends `Confirm`, not the client
    void user.send(new client.Confirm({ question: '?' })).catch(() => undefined);
    // @ts-expect-error: `Add` takes numbers
    void user.send(new client.Add({ a: '1n', b: 2n })).catch(() => undefined);

    // @ts-expect-error: a notification has no response
    const notified: Promise<unknown> = user.send(new client.Log({ text: '' }));

    void notified;
    // @ts-expect-error: the provider's router routes what the provider handles
    user.use(new Adder());
    calculator.use(new Wrong());
    expect(true).toBe(true);
  });
  it('should take a burst of messages its default limits allow', async () => {
    const BURST = 8192;
    const { user, calculator } = pair();
    const releases: (() => void)[] = [];

    calculator.use(
      new (class extends provider.CalculatorRouter {
        override onAdd(message: provider.Add): Promise<bigint> {
          return new Promise((resolve) => releases.push(() => resolve(message.a + message.b)));
        }
      })()
    );
    await vi.waitFor(() => expect(user.connection).toBeDefined());

    const sums = Array.from({ length: BURST }, (_, a) => user.send(new client.Add({ a: BigInt(a), b: 1n })));

    await vi.waitFor(() => expect(releases).toHaveLength(BURST));
    for (const release of releases) release();
    expect(await Promise.all(sums)).toEqual(Array.from({ length: BURST }, (_, a) => BigInt(a + 1)));
  });
  it('should take a burst past what it responds to at once when its handlers respond promptly', async () => {
    const BURST = 10_000;
    const { user, calculator } = pair();

    calculator.use(
      new (class extends provider.CalculatorRouter {
        override onAdd(message: provider.Add): bigint {
          return message.a + message.b;
        }
      })()
    );
    await vi.waitFor(() => expect(user.connection).toBeDefined());

    const sums = Array.from({ length: BURST }, (_, a) => user.send(new client.Add({ a: BigInt(a), b: 1n })));

    expect(await Promise.all(sums)).toEqual(Array.from({ length: BURST }, (_, a) => BigInt(a + 1)));
  });
});

describe('health checks', () => {
  it('should tell its health listeners but removed ones how its health changes', async () => {
    const { driver, user, healths, removed } = await watchedClient();

    pongEach(driver, 10);

    vi.advanceTimersByTime(400);
    driver.deliver(Role.Client, CONNECTION, { kind: FrameKind.Pong });
    expect(user.health).toBe(Health.Healthy);
    expect(healths).toEqual([Health.Unhealthy, Health.Healthy]);
    expect(removed).toEqual([]);
  });
});

describe('cancelling', () => {
  it("should cancel a request whose signal aborts, aborting its handler's signal", async () => {
    const { user, calculator } = pair();
    const delayer = new Delayer();

    calculator.use(delayer);

    const controller = new AbortController();
    const sum = user.send(new client.Add({ a: 1n, b: 2n }), { signal: controller.signal });

    await delayer.arrived();

    const [delayed] = delayer.delayed;
    const aborted = new Promise((resolve) => delayed!.signal.addEventListener('abort', resolve));

    controller.abort();
    await expect(sum).rejects.toEqual(ChannelError.reject(ErrorCode.Cancellation));
    await aborted;
    expect(delayed!.signal.aborted).toBe(true);
  });

  it('should let no handler respond to a message once its sender cancelled it', async () => {
    const { driver, calculator } = await connectedProvider();
    let sent: Promise<void> | undefined;

    calculator.use(
      new (class extends provider.CalculatorRouter {
        override onCount(
          _message: provider.Count,
          stream: ResponseWriter<Phase, number>,
          _next: Next<bigint>,
          context: RouteContext
        ): Promise<bigint> {
          stream.start(Phase.Active);
          context.signal.addEventListener('abort', () => {
            sent = stream.send(1);
            sent.catch(() => undefined);
          });

          return new Promise(() => undefined);
        }
      })()
    );
    driver.deliver(Role.Provider, CONNECTION, {
      kind: FrameKind.Message,
      typeId: client.Count.id,
      id: 0,
      body: body(new client.Count({ to: 3 }))
    });
    await settle();
    driver.take();
    driver.deliver(Role.Provider, CONNECTION, { kind: FrameKind.Cancel, id: 0 });
    await settle();
    await expect(sent).rejects.toEqual(ChannelError.reject(ErrorCode.Cancellation));

    const kinds = driver.take().flatMap(({ frames }) => frames.map((frame) => frame.kind));

    expect(kinds).toEqual([FrameKind.Failure]);
  });

  it('should send nothing of a request its signal aborts before it is sent', async () => {
    const driver = new MemoryDriver();
    const user = new client.UserChannel(driver, NO_CHECKS);
    const controller = new AbortController();
    const sum = user.send(new client.Add({ a: 1n, b: 2n }), { signal: controller.signal });

    controller.abort();
    driver.deliver(Role.Client, NO_CONNECTION, ack());
    await settle();
    await expect(sum).rejects.toEqual(ChannelError.reject(ErrorCode.Cancellation));

    const kinds = driver.take().flatMap(({ frames }) => frames.map((frame) => frame.kind));

    expect(kinds).not.toContain(FrameKind.Message);
  });
});

describe('queued messages', () => {
  it('should send what a message was when sent, though its fields change before there is a connection', async () => {
    const driver = new MemoryDriver();
    const user = new client.UserChannel(driver, NO_CHECKS);
    const fields = { text: 'as sent' };

    user.send(new client.Log(fields));
    fields.text = 'changed';
    driver.deliver(Role.Client, NO_CONNECTION, ack());
    await settle();

    const messages = driver.take().flatMap(({ frames }) => frames.filter((frame) => frame.kind === FrameKind.Message));

    expect(messages).toEqual([logFrame('as sent')]);
  });
});
