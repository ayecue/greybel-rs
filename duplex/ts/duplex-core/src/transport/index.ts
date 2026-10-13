import { Bundle } from '../channel/options.js';
import { SETTLE_ROUNDS, NO_LIMIT } from '../const.js';
import { ErrorCode, type Role } from '../definition.generated.js';
import type { Driver } from '../driver/index.js';
import { EntropyUnavailable } from '../errors.js';
import type { FrameEncoder } from '../frame/encoder.js';
import type { Route } from '../frame/route.js';
import type { PeerOf, Protocol, SecurityLevel } from '../protocol/index.js';
import { ReceiveOutcomeKind, type Security } from '../security/secure.js';
import { Egress } from './egress.js';
import { EndpointTable, type Endpoint } from './endpoints.js';
import { TransportSecurity } from './security.js';
import { Hub } from '../hub/index.js';
import { WorkQueue, type Work } from './work-queue.js';
import { Relay } from './relay.js';

export interface TransportOptions {
  readonly security?: Security;
}

export class Transport {
  static readonly #transports = new WeakMap<Driver, Transport>();

  readonly driver: Driver;
  readonly #endpoints = new EndpointTable();
  readonly #egress = new Egress();
  readonly #workQueue: WorkQueue;
  readonly #security: TransportSecurity;
  readonly #flushTask = (): void => this.#settleThenFlush();
  readonly #deferFlushTask = (): void => {
    this.#isFlushAfterEvent = false;
    this.#scheduleFlush();
  };

  readonly #sealAndSend = (batch: Uint8Array): void => {
    const sealedBatch = this.#security.seal(batch);

    if (sealedBatch === undefined) return;
    this.driver.send(sealedBatch);
  };

  readonly #hub = Hub.current;
  #scheduled = false;
  #isFlushAfterEvent = false;
  #writeCount = 0;
  #settledWriteCount = 0;
  #settleRounds = 0;
  #closeReceiver: (() => void) | undefined;
  #endReason: ErrorCode | undefined;

  constructor(driver: Driver, options: TransportOptions = {}) {
    this.driver = driver;
    this.#workQueue = new WorkQueue(driver, this.#hub);

    let guard: TransportSecurity | undefined;

    try {
      guard = TransportSecurity.of(options.security);
    } catch (error) {
      if (!(error instanceof EntropyUnavailable)) throw error;
    }

    this.#security = guard ?? TransportSecurity.NONE;
    Transport.#transports.set(driver, this);
    if (guard === undefined) {
      this.#end(ErrorCode.Insecure);

      return;
    }

    if (!guard.isSecured) return;
    this.#openReceiver();

    const firstHandshakeMessage = guard.startHandshake();

    if (firstHandshakeMessage === undefined) return;
    driver.send(firstHandshakeMessage);
  }

  static of(driver: Driver): Transport {
    return Transport.#transports.get(driver) ?? new Transport(driver);
  }

  get endReason(): ErrorCode | undefined {
    return this.#endReason;
  }

  isAsSecureAs(level: SecurityLevel | undefined, signed = false): boolean {
    return this.#security.isAsSecureAs(level, signed);
  }

  attach(channel: number, role: Role, endpoint: Endpoint): () => void {
    const detach = this.#endpoints.attach(channel, role, endpoint);

    this.#openReceiver();

    return () => {
      if (!detach()) return;
      this.#onDetach();
    };
  }

  begin(route: Route, bundle: Bundle, minor: number, maxFrame = NO_LIMIT): FrameEncoder {
    return this.#egress.begin(route, bundle, minor, maxFrame);
  }

  commit(): void {
    if (!this.#egress.commitFrame()) return;
    this.#onWrite();
  }

  enqueue(work: Work): void {
    this.#workQueue.enqueue(work);
  }

  release(route: Route): void {
    this.#egress.aliases.release(route);
  }

  flush(): void {
    this.#scheduled = false;
    if (!this.#security.canSend) return;
    this.#egress.flush(this.#sealAndSend);
    if (this.#egress.isEmpty) return;
    this.#scheduleFlush();
  }

  flushAll(): void {
    while (this.#security.canSend && !this.#egress.isEmpty) this.flush();
  }

  closeAll(reason: ErrorCode): void {
    for (const endpoint of this.#endpoints.toArray()) endpoint.close(reason);
    this.flushAll();
  }

  relay<P extends Protocol>(protocol: P, peer: PeerOf<P>, bundle: Bundle = Bundle.COMMON): Relay {
    return new Relay(this, protocol.channelId, peer, bundle);
  }

  #onDetach(): void {
    this.flushAll();
    if (this.#endpoints.size > 0 || this.#security.isSecured) return;
    this.#closeReceiver?.();
    this.#closeReceiver = undefined;
  }

  #onWrite(): void {
    this.#writeCount++;
    if (this.#egress.isFull) this.flush();
    else if (this.#hub.inEvent) {
      if (this.#scheduled || this.#isFlushAfterEvent) return;
      this.#isFlushAfterEvent = true;
      this.#hub.afterEvent(this.#deferFlushTask);
    } else this.#scheduleFlush();
  }

  #scheduleFlush(): void {
    if (this.#scheduled) return;
    this.#scheduled = true;
    this.#settledWriteCount = this.#writeCount;
    this.#settleRounds = 0;
    this.driver.defer(this.#flushTask);
  }

  #settleThenFlush(): void {
    if (!this.#scheduled) return;
    if (this.#writeCount !== this.#settledWriteCount && this.#settleRounds < SETTLE_ROUNDS) {
      this.#settledWriteCount = this.#writeCount;
      this.#settleRounds++;
      queueMicrotask(this.#flushTask);

      return;
    }

    this.flush();
  }

  #openReceiver(): void {
    if (this.#endReason !== undefined) return;
    this.#closeReceiver ??= this.driver.open({
      receive: (batch) => this.#receive(batch),
      onDisconnect: () => this.#end(ErrorCode.ConnectionLoss)
    });
  }

  #routeBatch(batch: Uint8Array): void {
    const hub = this.#hub;

    hub.beginEvent();
    try {
      this.#endpoints.routeBatch(batch);
      hub.processAfterBatch(this.#workQueue);
    } finally {
      hub.endEvent();
    }
  }

  #receive(bytes: Uint8Array): void {
    const guard = this.#security;

    if (!guard.isSecured) return this.#routeBatch(bytes);

    const outcome = guard.unseal(bytes);

    switch (outcome.kind) {
      case ReceiveOutcomeKind.Batch:
        return this.#routeBatch(outcome.batch);
      case ReceiveOutcomeKind.Reply:
        this.driver.send(outcome.reply);
        break;
      case ReceiveOutcomeKind.Failure:
        return this.#end(ErrorCode.Insecure);
      case ReceiveOutcomeKind.Nothing:
        break;
    }

    this.flushAll();
  }

  #end(reason: ErrorCode): void {
    if (this.#endReason !== undefined) return;
    this.#endReason = reason;
    this.#endpoints.clearAliases();
    this.#egress.clear();
    this.#workQueue.clear();
    this.#closeReceiver?.();
    this.#closeReceiver = undefined;
    for (const endpoint of this.#endpoints.toArray()) {
      if (reason === ErrorCode.ConnectionLoss) endpoint.onDisconnect();
      else endpoint.close(reason);
    }
  }
}
