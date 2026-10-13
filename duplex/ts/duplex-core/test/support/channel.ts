import { onTestFinished, vi } from 'vitest';

import { Health } from '../../src/channel/connection/health.js';
import { FrameKind } from '../../src/definition.generated.js';
import type { Control } from '../../src/frame/control.js';
import {
  ChannelError,
  CORE,
  Driver,
  Role,
  type ChannelOptions,
  type Next,
  type Receiver,
  type RouteContext
} from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { DivisionFailure, Phase, Quotient, Step } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { body, encode, TestAliases, type TestBatch, type TestFrame } from './frames.js';

export const HEALTH_CHECK = { healthCheck: { interval: 100, unhealthyThreshold: 2, closeThreshold: 3 } } as const;
export const NO_CHECKS: ChannelOptions = { healthCheck: false };
export const CONNECTION = 1;
export const CLIENT = 0;
export const NO_CONNECTION = 0;
export const CHANNEL = client.Calculation.channelId;

export function zeroRandomValues(): void {
  vi.spyOn(crypto, 'getRandomValues').mockImplementation(<T extends ArrayBufferView | null>(array: T): T => {
    new Uint8Array(array!.buffer, array!.byteOffset, array!.byteLength).fill(0);

    return array;
  });
}

export function restoreMocksAndTimers(): void {
  vi.restoreAllMocks();
  vi.useRealTimers();
}

export async function settled(): Promise<void> {
  for (let round = 0; round < 8; round++) await new Promise((resolve) => setTimeout(resolve, 0));
}

export function settle(): Promise<void> {
  return new Promise((resolve) => queueMicrotask(resolve));
}

export class MemoryDriver extends Driver {
  readonly transport = 'memory';
  readonly posted: Uint8Array[] = [];
  readonly #aliases = new TestAliases();
  #receiver: Receiver | undefined;

  get isOpen(): boolean {
    return this.#receiver !== undefined;
  }

  open(receiver: Receiver): () => void {
    this.#receiver = receiver;

    return () => {
      this.#receiver = undefined;
    };
  }

  send(batch: Uint8Array): void {
    this.posted.push(batch);
  }

  dispatch(batch: Uint8Array): void {
    this.#receiver?.receive(batch);
  }

  deliver(to: Role, connection: number, ...frames: TestFrame[]): void {
    this.dispatch(encode(batch(to === Role.Client ? Role.Provider : Role.Client, connection, ...frames)));
  }

  disconnect(): void {
    this.#receiver?.onDisconnect();
  }

  take(): TestBatch[] {
    return this.posted.splice(0).flatMap((batch) => this.#aliases.read(batch));
  }
}

export class PortDriver extends Driver {
  readonly transport = 'port';
  readonly #port: MessagePort;
  #receiver: Receiver | undefined;

  constructor(port: MessagePort) {
    super();
    this.#port = port;
  }

  open(receiver: Receiver): () => void {
    const listener = ({ data }: MessageEvent) => receiver.receive(data as Uint8Array);

    this.#receiver = receiver;

    this.#port.addEventListener('message', listener);
    this.#port.start();

    return () => this.#port.removeEventListener('message', listener);
  }

  send(batch: Uint8Array): void {
    this.#port.postMessage(batch, [batch.buffer as ArrayBuffer]);
  }

  disconnect(): void {
    this.#receiver?.onDisconnect();
  }
}

export function batch(from: Role, connection: number, ...frames: TestFrame[]): TestBatch {
  return { channel: CHANNEL, from, connection, frames };
}

const VERSION = client.Calculation.version;
const FINGERPRINT = client.Calculation.versions[VERSION.minor]!.fingerprint;

export function open(id = CLIENT): Control {
  return {
    kind: FrameKind.Open,
    id,
    core: CORE,
    protocol: VERSION,
    fingerprint: FINGERPRINT,
    payload: new Uint8Array(0)
  };
}

export function ack(id = CLIENT): Control {
  return {
    kind: FrameKind.Ack,
    id,
    connection: CONNECTION,
    core: CORE,
    protocol: VERSION,
    fingerprint: FINGERPRINT,
    deprecation: null,
    payload: new Uint8Array(0)
  };
}

export function logFrame(text: string): TestFrame {
  return { kind: FrameKind.Message, typeId: client.Log.id, id: undefined, body: body(new client.Log({ text })) };
}

export function addFrame(id: number, a: bigint, b: bigint): TestFrame {
  return { kind: FrameKind.Message, typeId: client.Add.id, id, body: body(new client.Add({ a, b })) };
}

export class Adder extends provider.CalculatorRouter {
  override onAdd(message: provider.Add): bigint {
    return message.a + message.b;
  }
}

export class Divider extends provider.CalculatorRouter {
  override onDivide({ dividend, divisor }: provider.Divide): Quotient {
    const value = dividend / divisor;
    const remainder = dividend % divisor;

    return remainder === 0n ? new Quotient.Exact({ value }) : new Quotient.WithRemainder({ value, remainder });
  }
}

export class Logger extends provider.CalculatorRouter {
  readonly logged: string[] = [];
  #wake?: () => void;

  override onLog(message: provider.Log): void {
    this.logged.push(message.text);
    this.#wake?.();
  }

  next(): Promise<void> {
    return new Promise((resolve) => (this.#wake = resolve));
  }
}

export class Confirmer extends client.UserRouter {
  override async onConfirm(request: client.Confirm): Promise<boolean> {
    return request.question.endsWith('?');
  }
}

export class Doubler extends provider.CalculatorRouter {
  override async onAdd(_message: provider.Add, next: Next<bigint>): Promise<bigint> {
    return 2n * (await next());
  }
}

export class Failer extends provider.CalculatorRouter {
  override onAdd(): bigint {
    throw new Error('overflow');
  }

  override async onDivide(): Promise<Quotient> {
    throw ChannelError.fail(DivisionFailure.ByZero);
  }
}

export class Wrong extends provider.CalculatorRouter {
  // @ts-expect-error: `Add` replies with a number
  override onAdd(): string {
    return 'three';
  }
}

export async function listeningProvider(options: ChannelOptions = NO_CHECKS) {
  const driver = new MemoryDriver();
  const calculator = new provider.CalculatorChannel(driver, options);

  await settle();
  driver.take();

  return { driver, calculator };
}

export async function connectedClient(options: ChannelOptions = NO_CHECKS) {
  const driver = new MemoryDriver();
  const user = new client.UserChannel(driver, options);

  driver.deliver(Role.Client, NO_CONNECTION, ack());
  await settle();
  driver.take();

  return { driver, user };
}

export async function connectedProvider(options: ChannelOptions = NO_CHECKS) {
  const { driver, calculator } = await listeningProvider(options);

  driver.deliver(Role.Provider, NO_CONNECTION, open());
  await settle();
  driver.take();

  return { driver, calculator };
}

export function pair() {
  const ports = new MessageChannel();
  const calculator = new provider.CalculatorChannel(new PortDriver(ports.port2), NO_CHECKS);
  const user = new client.UserChannel(new PortDriver(ports.port1), NO_CHECKS);

  onTestFinished(() => {
    user.close();
    calculator.close();
    ports.port1.close();
    ports.port2.close();
  });

  return { user, calculator };
}

export function routedPair() {
  const { user, calculator } = pair();
  const logger = new Logger();

  calculator.use(logger, new Doubler(), new Adder(), new Divider());

  return { user, logger };
}

export function nextLog(calculator: provider.CalculatorChannel): Promise<provider.Log> {
  return new Promise<provider.Log>((resolve) =>
    calculator.use(
      new (class extends provider.CalculatorRouter {
        override onLog(message: provider.Log): void {
          resolve(message);
        }
      })()
    )
  );
}

export function nextProgress(user: client.UserChannel): Promise<client.Progress> {
  return new Promise<client.Progress>((resolve) =>
    user.use(
      new (class extends client.UserRouter {
        override onProgress(message: client.Progress): void {
          resolve(message);
        }
      })()
    )
  );
}

export function progress(note?: string | null): provider.Progress {
  return new provider.Progress({
    phase: Phase.Active,
    doneShare: null,
    steps: [
      new Step({ name: 'divide', result: new Quotient.WithRemainder({ value: 2n, remainder: 1n }) }),
      new Step({ name: 'wait', result: null })
    ],
    counts: { divisions: 1 },
    note
  });
}

export async function watchedClient() {
  vi.useFakeTimers();

  const { driver, user } = await connectedClient(HEALTH_CHECK);
  const healths: Health[] = [];
  const removed: Health[] = [];

  user.onHealth((health) => healths.push(health));

  const remove = user.onHealth((health) => removed.push(health));

  remove();

  return { driver, user, healths, removed };
}

export function pongEach(driver: MemoryDriver, checks: number): void {
  for (let check = 0; check < checks; check++) {
    vi.advanceTimersByTime(100);
    driver.deliver(Role.Client, CONNECTION, { kind: FrameKind.Pong });
  }
}

export class MemoryLink {
  readonly delivered: Uint8Array[] = [];
  readonly #queue: { to: LinkedDriver; batch: Uint8Array }[] = [];
  readonly #tasks: (() => void)[] = [];
  readonly client = new LinkedDriver(this);
  readonly provider = new LinkedDriver(this);

  post(from: LinkedDriver, batch: Uint8Array): void {
    this.#queue.push({ to: from === this.client ? this.provider : this.client, batch: batch.slice() });
  }

  defer(task: () => void): void {
    this.#tasks.push(task);
  }

  deliver(): void {
    for (;;) {
      while (this.#tasks.length > 0) this.#tasks.shift()!();

      const next = this.#queue.shift();

      if (next === undefined) return;
      this.delivered.push(next.batch);
      next.to.receiver?.receive(next.batch);
    }
  }
}

export class LinkedDriver extends Driver {
  readonly transport = 'memory';
  receiver: Receiver | undefined;
  readonly #link: MemoryLink;

  constructor(link: MemoryLink) {
    super();
    this.#link = link;
  }

  open(receiver: Receiver): () => void {
    this.receiver = receiver;

    return () => {
      this.receiver = undefined;
    };
  }

  send(batch: Uint8Array): void {
    this.#link.post(this, batch);
  }

  override defer(task: () => void): void {
    this.#link.defer(task);
  }
}

export class Delayer extends provider.CalculatorRouter {
  readonly delayed: { message: provider.Add; signal: AbortSignal; respond(sum: bigint): void }[] = [];
  #arrived?: () => void;

  override onAdd(message: provider.Add, _next: unknown, context: RouteContext): Promise<bigint> {
    return new Promise((respond) => {
      this.delayed.push({ message, signal: context.signal, respond });
      this.#arrived?.();
    });
  }

  arrived(): Promise<void> {
    return this.delayed.length > 0 ? Promise.resolve() : new Promise((resolve) => (this.#arrived = resolve));
  }
}
