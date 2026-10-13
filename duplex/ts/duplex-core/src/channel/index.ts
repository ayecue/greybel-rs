import { Emitter, type Callback, type Disposable } from 'emitter';

import { BinaryWriter } from '../binary/writer.js';
import { ID_BYTES } from '../const.js';
import { ErrorCode } from '../definition.generated.js';
import { Capabilities, type Driver } from '../driver/index.js';
import { ChannelError, ChannelErrorKind } from '../errors.js';
import type { Control } from '../frame/control.js';
import { Transport } from '../transport/index.js';
import type { Migration } from '../migration/index.js';
import type { Session } from '../migration/session.js';
import type { Exchange, StreamResponse, StreamExchange, ValueResponse } from '../protocol/exchange.js';
import {
  CORE,
  Role,
  type MessageOf,
  type OpenOf,
  type PeerOf,
  type Protocol,
  type VersionNumber
} from '../protocol/index.js';
import type { ExchangeOf, Message } from '../protocol/message.js';
import type { ConnectionActive } from './connection/active.js';
import type { QueueItem } from './connection/context.js';
import type { ConnectionEnd, ConnectionFailure } from './connection/end.js';
import type { Health } from './connection/health.js';
import { VersionTable } from './connection/version-table.js';
import { Connection } from './connection/index.js';
import { ConnectionInit } from './connection/init.js';
import {
  ConnectionStateKind,
  OpenOutcomeKind,
  type ConnectionListener,
  type ConnectionState,
  type OpenOutcome
} from './connection/state.js';
import { MessageExchange, type SendOptions } from './message-exchange.js';
import { ChannelSettings, type ChannelOptions } from './options.js';
import type { Router } from './router.js';
import { RouterChain } from './routes.js';
import type { ResponseStream, StreamWriter } from './streams/index.js';

export type { SendOptions } from './message-exchange.js';

export type HandleOf<X> =
  X extends ValueResponse<infer R, unknown>
    ? Promise<R>
    : X extends StreamResponse<infer S, infer E, infer I, unknown>
      ? ResponseStream<S, E, I>
      : X extends StreamExchange<infer E, infer I, infer Y>
        ? { readonly stream: StreamWriter<E, I>; readonly response: HandleOf<Y> }
        : void;

export type Handle<M> = HandleOf<ExchangeOf<M>>;

export type ChannelConstructor<C> = new (transport: Transport | Driver, options?: ChannelOptions) => C;

export abstract class Channel<P extends Protocol, L extends PeerOf<P>> {
  readonly closeEvent: Promise<ErrorCode>;
  readonly #protocol: P;
  readonly #transport: Transport;
  readonly #settings: ChannelSettings;
  readonly #routerChain = new RouterChain<P, L>();
  #openEmitter: Emitter<Callback<[ConnectionInit]>> | undefined;
  #ackEmitter: Emitter<Callback<[ConnectionActive]>> | undefined;
  #failureEmitter: Emitter<Callback<[ConnectionFailure]>> | undefined;
  #closeEmitter: Emitter<Callback<[ConnectionEnd]>> | undefined;
  #healthEmitter: Emitter<Callback<[Health]>> | undefined;
  readonly #connection: Connection;
  readonly #exchange: MessageExchange<P, L>;
  readonly #detach: () => void;
  #stopHealthChecks: (() => void) | undefined;
  #resolveCloseEvent!: (reason: ErrorCode) => void;

  protected constructor(
    transport: Transport | Driver,
    protocol: P,
    peer: L,
    options: ChannelOptions = {},
    open?: OpenOf<P>
  ) {
    if (!protocol.peers.includes(peer))
      throw new TypeError(`${peer} isn't a peer of the protocol "${protocol.channel}"`);
    if (protocol.core.major !== CORE.major || protocol.core.minor > CORE.minor) {
      throw new TypeError(
        `"${protocol.channel}" builds on the core ${protocol.core.major}.${protocol.core.minor}, which this duplex-core doesn't speak`
      );
    }

    const role: Role = peer;
    const driver = transport instanceof Transport ? transport.driver : transport;

    if (!Capabilities.supports(driver.capabilities, role, protocol.requiredCapabilities ?? Capabilities.NONE)) {
      throw new TypeError(`the transports of "${driver.transport}" can't carry the channels of "${protocol.channel}"`);
    }

    this.#protocol = protocol;
    this.#transport = transport instanceof Transport ? transport : Transport.of(transport);
    this.#settings = new ChannelSettings(options, protocol.flavours?.[driver.transport], driver.streamWindow);
    this.#connection = Channel.#createConnection(
      driver,
      protocol,
      role,
      this.#settings,
      this.#connectionListener,
      open
    );
    this.#exchange = new MessageExchange(
      protocol,
      role,
      this.#transport,
      this.#settings,
      this.#connection,
      this.#routerChain
    );
    this.closeEvent = new Promise((resolve) => (this.#resolveCloseEvent = resolve));

    const connection = this.#connection;
    const exchange = this.#exchange;

    this.#detach = this.#transport.attach(protocol.channelId, role, {
      receive: (inbound) => exchange.receive(inbound),
      onDisconnect: () => connection.close(ErrorCode.ConnectionLoss, false),
      close: (reason) => this.close(reason)
    });
    this.#transport.driver.defer(() => this.#start());
  }

  get state(): ConnectionState {
    return this.#connection.state;
  }

  get connection(): number | undefined {
    return this.#connection.activeId;
  }

  get version(): VersionNumber | undefined {
    return this.#connection.session?.protocol;
  }

  get health(): Health {
    return this.#connection.health;
  }

  get closeReason(): ErrorCode | undefined {
    return this.#connection.reason;
  }

  use(...routers: Router<P, L>[]): this {
    this.#routerChain.add(routers);

    return this;
  }

  onOpen(hook: Callback<[ConnectionInit]>): Disposable {
    return (this.#openEmitter ??= new Emitter()).emitter.addDisposable(hook);
  }

  onAck(hook: Callback<[ConnectionActive]>): Disposable {
    return (this.#ackEmitter ??= new Emitter()).emitter.addDisposable(hook);
  }

  onFailure(hook: Callback<[ConnectionFailure]>): Disposable {
    return (this.#failureEmitter ??= new Emitter()).emitter.addDisposable(hook);
  }

  onClose(hook: Callback<[ConnectionEnd]>): Disposable {
    return (this.#closeEmitter ??= new Emitter()).emitter.addDisposable(hook);
  }

  onHealth(hook: Callback<[Health]>): Disposable {
    return (this.#healthEmitter ??= new Emitter()).emitter.addDisposable(hook);
  }

  addMigration(migration: Migration): this {
    this.#connection.addMigration(migration);

    return this;
  }

  close(reason = ErrorCode.ClosedChannel): void {
    this.#connection.close(reason);
  }

  runHealthCheck(): void {
    this.#connection.runHealthCheck();
  }

  send<M extends MessageOf<P> & Message<L, object, Exchange>>(message: M, options: SendOptions = {}): Handle<M> {
    return this.#exchange.send(message, options) as Handle<M>;
  }

  readonly #connectionListener: ConnectionListener = {
    onControlOut: (control: Control, connection: number | undefined) =>
      this.#exchange.writeControl(control, connection),
    onConnect: (connection: number, session: Session, queue: readonly QueueItem[]) =>
      this.#exchange.onConnect(connection, session, queue),
    onOpen: (id: number, open: unknown) => this.#routeOpen(id, open as OpenOf<P>),
    onEnd: (connection: number | undefined, reason: ErrorCode) => this.#exchange.end(connection, reason),
    onClose: (reason: ErrorCode) => this.#finishClose(reason),
    onEnter: (state: ConnectionState) => this.#emitState(state),
    onHealth: (health: Health) => this.#healthEmitter?.emit(health)
  };

  #start(): void {
    if (this.closeReason !== undefined) return;

    const transport = this.#transport;
    const protocol = this.#protocol;
    const endReason = transport.isAsSecureAs(protocol.security, protocol.signed)
      ? transport.endReason
      : ErrorCode.Insecure;

    if (endReason !== undefined) return this.#connection.close(endReason, false);

    const { healthCheck } = this.#settings;

    if (healthCheck !== undefined)
      this.#stopHealthChecks = transport.driver.interval(healthCheck.interval, () => this.runHealthCheck());
    this.#connection.start();
  }

  #routeOpen(id: number, open: OpenOf<P>): void {
    const applyOutcome = (outcome: OpenOutcome): void =>
      this.#connection.onOpenResult(id, outcome, this.#transport.driver.random());

    if (this.#protocol.handshake === undefined)
      return applyOutcome({ kind: OpenOutcomeKind.Acceptance, ack: undefined });
    this.#routerChain.routeOpen(open).then(
      (ack) => applyOutcome({ kind: OpenOutcomeKind.Acceptance, ack }),
      (error: unknown) => {
        if (!(error instanceof ChannelError))
          applyOutcome({ kind: OpenOutcomeKind.Failure, reason: ErrorCode.HandlerFailure });
        else if (error.kind === ChannelErrorKind.Denial)
          applyOutcome({ kind: OpenOutcomeKind.Denial, refusal: error.failure });
        else applyOutcome({ kind: OpenOutcomeKind.Failure, reason: error.code ?? ErrorCode.HandlerFailure });
      }
    );
  }

  #emitState(state: ConnectionState): void {
    switch (state.kind) {
      case ConnectionStateKind.Init:
        return this.#openEmitter?.emit(state);
      case ConnectionStateKind.Active:
        return this.#ackEmitter?.emit(state);
      case ConnectionStateKind.Failure:
        return this.#failureEmitter?.emit(state);
      case ConnectionStateKind.End:
        return this.#closeEmitter?.emit(state);
    }
  }

  #finishClose(reason: ErrorCode): void {
    this.#transport.flushAll();
    this.#stopHealthChecks?.();
    this.#detach();
    this.#resolveCloseEvent(reason);
  }

  static #createConnection(
    driver: Driver,
    protocol: Protocol,
    role: Role,
    settings: ChannelSettings,
    events: ConnectionListener,
    open: unknown
  ): Connection {
    const flavour = new VersionTable(protocol.version, protocol.versions);
    const handshake = protocol.handshake;

    if (role !== Role.Client)
      return new Connection(new ConnectionInit(0), flavour, handshake, settings.healthCheck, undefined, events);

    const random = driver.random();
    const id = new DataView(random.buffer, random.byteOffset, random.byteLength).getUint32(
      random.byteLength - ID_BYTES,
      true
    );
    const writer = new BinaryWriter();

    handshake?.writeOpen(writer, open as never);

    return new Connection(new ConnectionInit(id), flavour, handshake, settings.healthCheck, writer.asBytes(), events);
  }
}
