import type { BinaryReader } from '../binary/reader.js';
import { BinaryWriter } from '../binary/writer.js';
import { FieldValidation } from '../binary/validations.js';
import { ErrorCode, FrameKind, Requirement, Role as DefinitionRole, type Violation } from '../definition.generated.js';
import { ChannelError } from '../errors.js';
import type { Control } from '../frame/control.js';
import type { FrameEncoder } from '../frame/encoder.js';
import { FrameReader, type Frame, type FrameHead } from '../frame/reader.js';
import type { Route } from '../frame/route.js';
import type { InboundSlice } from '../transport/endpoints.js';
import type { Transport } from '../transport/index.js';
import type { Work } from '../transport/work-queue.js';
import type { Session } from '../migration/session.js';
import { CORE, type Inbound, type PeerOf, type Protocol, type Role } from '../protocol/index.js';
import { MessageSegment, type AnyMessage, type IMessage } from '../protocol/message.js';
import type { QueueItem } from './connection/context.js';
import type { Connection } from './connection/index.js';
import { IncomingFlow, type IncomingHost } from './flows/inbound.js';
import { OutgoingFlow, type OutgoingHost } from './flows/outbound.js';
import { IncomingFlowTable, OutgoingFlowTable } from './flows/tables.js';
import { Validation } from './flows/validation.js';
import type { Bundle, ChannelSettings } from './options.js';
import { RouteContext } from './router.js';
import type { RouterChain } from './routes.js';
import { NO_LIMIT } from '../const.js';

export interface SendOptions {
  readonly signal?: AbortSignal;
  readonly bundle?: Bundle;
}

const NOTIFICATION_CONTEXT = new RouteContext();

interface NotificationHost {
  handleNotification(connection: number, remoteType: IMessage, body: BinaryReader): void;
}

class NotificationWork implements Work {
  constructor(
    private readonly host: NotificationHost,
    private readonly connection: number,
    private readonly remoteType: IMessage,
    private readonly body: BinaryReader
  ) {}

  proceed(): void {
    this.host.handleNotification(this.connection, this.remoteType, this.body);
  }
}

class Reassembly {
  readonly bytes: Uint8Array;
  #length = 0;
  #dropping: boolean;

  constructor(length: number) {
    this.bytes = new Uint8Array(length);
    this.#dropping = length === 0;
  }

  append(bytes: Uint8Array): boolean {
    if (this.#dropping) return false;
    if (this.#length + bytes.length > this.bytes.length) {
      this.#dropping = true;

      return false;
    }

    this.bytes.set(bytes, this.#length);
    this.#length += bytes.length;

    return true;
  }
}

export class MessageExchange<P extends Protocol, L extends PeerOf<P>> implements OutgoingHost, IncomingHost {
  readonly streamWindow: number;
  readonly #protocol: P;
  readonly #role: Role;
  readonly #transport: Transport;
  readonly #settings: ChannelSettings;
  readonly #connection: Connection;
  readonly #routerChain: RouterChain<P, L>;
  readonly #outgoingFlows = new OutgoingFlowTable();
  readonly #incomingFlows = new IncomingFlowTable();
  #cachedRoute: Route | undefined;
  #reassembly: Reassembly | undefined;
  #nextDeadline: number | undefined;
  #cancelDeadlineTimer: (() => void) | undefined;

  constructor(
    protocol: P,
    role: Role,
    transport: Transport,
    settings: ChannelSettings,
    connection: Connection,
    routes: RouterChain<P, L>
  ) {
    this.#protocol = protocol;
    this.#role = role;
    this.#transport = transport;
    this.#settings = settings;
    this.#connection = connection;
    this.#routerChain = routes;
    this.streamWindow = settings.streamWindow;
  }

  get validatesInput(): boolean {
    return this.#role === DefinitionRole.Provider;
  }

  get validatesOutput(): boolean {
    return this.#settings.validating;
  }

  send(message: AnyMessage, options: SendOptions): unknown {
    const type = message.constructor as IMessage;
    const bundle = options.bundle ?? this.#settings.bundle;

    if (!type.responds) {
      this.#sendMessage(message, type, undefined, bundle);

      return undefined;
    }

    const flow = this.#openOutgoingFlow(type);
    const handle = flow.getSenderHandle();
    const { signal } = options;

    try {
      if (signal?.aborted === true) throw ChannelError.reject(ErrorCode.Cancellation);
      if (this.#outgoingFlows.size > this.#settings.limits.pendingItems) throw ChannelError.reject(ErrorCode.Overload);
      this.#sendMessage(message, type, flow, bundle);
    } catch (error) {
      flow.end(error as Error);

      return handle;
    }

    if (signal !== undefined) flow.attachSignal(signal);

    return handle;
  }

  receive({ batch, slice, connection, core }: InboundSlice): void {
    this.#readFrames(batch, slice.start, slice.end, connection, core);
  }

  #readFrames(batch: Uint8Array, start: number, end: number, connection: number | undefined, core: number): void {
    const frameReader = new FrameReader(batch, start, end);
    const isOfThisCore = core === CORE.major;
    const maxMessage = this.#settings.maxMessage;

    try {
      for (;;) {
        const session = this.#connection.session;

        frameReader.coreMinor = session?.core.minor ?? BinaryWriter.NEWEST;
        frameReader.protocolMinor = session?.minor ?? BinaryWriter.NEWEST;

        const frame = frameReader.next();

        if (frame === undefined) return;
        if (frame.kind === FrameKind.Fragment) {
          this.#onFragment(connection, core, frame.bytes, frame.isLastPiece);
        } else if (frameReader.lastFrameLength > maxMessage && !FrameReader.isControl(frame.kind)) {
          this.#refuseOverLimit(
            connection,
            FrameReader.head(
              batch.subarray(frameReader.lastFrameStart, frameReader.lastFrameStart + frameReader.lastFrameLength)
            )
          );
        } else if (isOfThisCore || FrameReader.isFrozen(frame)) {
          this.#onFrame(connection, frame);
        }

        frameReader.skipFrame();
      }
    } catch {
      return;
    }
  }

  onConnect(connection: number, session: Session, queue: readonly QueueItem[]): void {
    const newest = this.#protocol.version.minor;

    for (const { type, body, bundle, flow } of queue) {
      if (flow === undefined) {
        const frames = this.#transport.begin(
          this.#routeOf(connection),
          bundle,
          session.minor,
          this.#settings.maxMessage
        );

        try {
          session.writePendingBody(frames, type, body, undefined, newest);
        } catch {
          continue;
        } finally {
          this.#transport.commit();
        }

        continue;
      }

      flow.startDispatch();
      try {
        const frames = this.#beginFrame(connection, bundle, session.minor);

        try {
          session.writePendingBody(frames, type, body, flow.exchangeId, newest);
        } finally {
          this.#transport.commit();
        }
      } catch (error) {
        flow.end(error instanceof ChannelError ? error : ChannelError.reject(ErrorCode.MalformedMessage));
        continue;
      }

      flow.finishDispatch();
    }
  }

  end(connection: number | undefined, reason: ErrorCode): void {
    if (connection !== undefined) this.#transport.release(this.#routeOf(connection));
    this.#reassembly = undefined;
    this.#cancelDeadlineTimer?.();
    this.#cancelDeadlineTimer = undefined;
    this.#nextDeadline = undefined;

    const error = ChannelError.channelEnd(reason);

    for (const flow of this.#outgoingFlows.toArray()) flow.end(error);
    for (const flow of this.#incomingFlows.drain()) flow.end(reason);
  }

  writeControl(control: Control, connection: number | undefined): void {
    const minor = this.#connection.session?.core.minor ?? BinaryWriter.NEWEST;
    const frames = this.#transport.begin(this.#routeOf(connection), this.#settings.bundle, minor, NO_LIMIT);

    try {
      frames.control(control);
    } finally {
      this.#transport.commit();
    }
  }

  now(): number {
    return this.#transport.driver.now();
  }

  scheduleDeadline(at: number): void {
    if (this.#nextDeadline !== undefined && this.#nextDeadline <= at) return;
    this.#cancelDeadlineTimer?.();

    const driver = this.#transport.driver;

    this.#nextDeadline = at;
    this.#cancelDeadlineTimer = driver.after(Math.max(at - driver.now(), 0), () => this.#onDeadline());
  }

  route(typeId: number, message: AnyMessage, context: RouteContext): unknown {
    return this.#routerChain.route(typeId, message as Inbound<P, L>, context);
  }

  writeMessageSegment(exchangeId: number, type: IMessage, segment: MessageSegment, value: unknown): void {
    const route = this.#connection.getActiveRoute();

    if (route === undefined) throw ChannelError.channelEnd(ErrorCode.ClosedChannel);
    this.#writeSegment(route.connection, route.session, exchangeId, type, segment, value);
  }

  writeResponseSegment(
    connection: number,
    exchangeId: number,
    type: IMessage,
    segment: MessageSegment,
    value: unknown
  ): void {
    const session = this.#sessionOf(connection);

    if (session === undefined) return;
    this.#writeSegment(connection, session, exchangeId, type, segment, value);
  }

  writeRejection(connection: number, exchangeId: number, code: ErrorCode, violation?: Violation): void {
    const session = this.#sessionOf(connection);

    if (session === undefined) return;
    try {
      const frames = this.#beginFrame(connection, this.#settings.bundle, session.minor);

      try {
        frames.rejection(exchangeId, code, violation);
      } finally {
        this.#transport.commit();
      }
    } catch {
      return;
    }
  }

  writeResponseCredit(exchangeId: number, count: number): void {
    const connection = this.#connection.activeId;

    if (connection === undefined) return;
    this.#writeCredit(connection, FrameKind.ResponseCredit, exchangeId, count);
  }

  writeMessageCredit(connection: number, exchangeId: number, count: number): void {
    this.#writeCredit(connection, FrameKind.MessageCredit, exchangeId, count);
  }

  writeCancel(exchangeId: number): boolean {
    const route = this.#connection.reason === undefined ? this.#connection.getActiveRoute() : undefined;

    if (route === undefined) return false;

    const frames = this.#beginFrame(route.connection, this.#settings.bundle, route.session.minor);

    try {
      frames.cancel(exchangeId);
    } finally {
      this.#transport.commit();
    }

    return true;
  }

  withdraw(flow: OutgoingFlow): boolean {
    return this.#connection.withdraw(flow);
  }

  release(flow: OutgoingFlow): void {
    this.#outgoingFlows.remove(flow);
  }

  canHandle(): boolean {
    return this.#incomingFlows.busyCount < this.#settings.limits.activeResponses;
  }

  onBusy(flow: IncomingFlow): void {
    this.#incomingFlows.onBusy(flow);
  }

  onFinish(flow: IncomingFlow): void {
    this.#incomingFlows.remove(flow);
  }

  #openOutgoingFlow(type: IMessage): OutgoingFlow {
    const flow = new OutgoingFlow(this, this.#outgoingFlows.nextId, type);

    this.#outgoingFlows.add(flow);

    return flow;
  }

  #sendMessage(message: AnyMessage, type: IMessage, flow: OutgoingFlow | undefined, bundle: Bundle): void {
    const connection = this.#connection;
    const route = connection.getActiveRoute();

    if (route === undefined) {
      if (connection.queuedCount >= this.#settings.limits.queuedItems) throw ChannelError.reject(ErrorCode.Overload);

      const writer = new BinaryWriter();

      writer.validate(this.#settings.validating);
      message.write(writer);
      if (writer.size > this.#getBodyLimit(type)) throw FieldValidation.invalidAt(undefined, Requirement.Limit);
      connection.queue({ type, body: writer.copyBytes(), bundle, flow });

      return;
    }

    const { session } = route;
    const validating = this.#settings.validating;
    const limit = this.#getBodyLimit(type);

    if (flow === undefined) {
      const frames = this.#transport.begin(
        this.#routeOf(route.connection),
        bundle,
        session.minor,
        this.#settings.maxMessage
      );

      try {
        frames.validate(validating, limit);
        session.writeMessage(frames, message, undefined);
      } finally {
        this.#transport.commit();
      }

      return;
    }

    flow.startDispatch();

    const frames = this.#beginFrame(route.connection, bundle, session.minor);

    try {
      frames.validate(validating, limit);
      session.writeMessage(frames, message, flow.exchangeId);
    } finally {
      this.#transport.commit();
    }

    flow.finishDispatch();
  }

  #beginFrame(connection: number, bundle: Bundle, minor: number): FrameEncoder {
    const frames = this.#transport.begin(this.#routeOf(connection), bundle, minor, this.#settings.maxMessage);

    frames.validate(false, NO_LIMIT);

    return frames;
  }

  #onFrame(connection: number | undefined, frame: Frame): void {
    switch (frame.kind) {
      case FrameKind.Listen:
      case FrameKind.Open:
      case FrameKind.Ack:
      case FrameKind.Refuse:
      case FrameKind.Ping:
      case FrameKind.Pong:
      case FrameKind.Close:
        return this.#connection.onControl(frame, connection);
      default:
        break;
    }

    const session = this.#connection.session;

    if (connection === undefined || connection !== this.#connection.activeId || session === undefined) return;
    this.#connection.onActivity();
    switch (frame.kind) {
      case FrameKind.Message:
        if (frame.id === undefined) this.#receiveNotification(connection, session, frame.typeId, frame.body);
        else this.#receiveRequest(connection, session, frame.typeId, frame.id, frame.body);

        return;
      case FrameKind.ResponseStart:
        return this.#outgoingFlows.get(frame.id)?.onResponse(MessageSegment.ResponseStart, frame.body, session);
      case FrameKind.ResponseItem:
        return this.#outgoingFlows.get(frame.id)?.onResponse(MessageSegment.ResponseItem, frame.body, session);
      case FrameKind.ResponseEnd:
        return this.#outgoingFlows.get(frame.id)?.onResponse(MessageSegment.ResponseEnd, frame.body, session);
      case FrameKind.Failure:
        if (frame.code !== undefined)
          return this.#outgoingFlows.get(frame.id)?.onRejection(frame.code, frame.violation);

        return this.#outgoingFlows.get(frame.id)?.onResponse(MessageSegment.Failure, frame.body, session);
      case FrameKind.MessageItem:
        return this.#incomingFlows.get(frame.id)?.onSegment(MessageSegment.MessageItem, frame.body, session);
      case FrameKind.MessageEnd:
        return this.#incomingFlows.get(frame.id)?.onSegment(MessageSegment.MessageEnd, frame.body, session);
      case FrameKind.ResponseCredit:
        return this.#incomingFlows.get(frame.id)?.onCredit(frame.count);
      case FrameKind.MessageCredit:
        return this.#outgoingFlows.get(frame.id)?.onCredit(frame.count);
      case FrameKind.Cancel:
        return this.#incomingFlows.get(frame.id)?.cancel();
    }
  }

  #receiveNotification(connection: number, session: Session, typeId: number, body: BinaryReader): void {
    let remoteType: IMessage;

    try {
      remoteType = session.getRemoteType(this.#protocol.messages, this.#role, typeId);
    } catch {
      return;
    }

    this.#transport.enqueue(new NotificationWork(this, connection, remoteType, body.getRemainderReader()));
  }

  handleNotification(connection: number, remoteType: IMessage, body: BinaryReader): void {
    const session = this.#sessionOf(connection);

    if (session === undefined) return;

    let message: AnyMessage;

    try {
      Validation.prepareBody(this.validatesInput, remoteType.limit, body);
      message = session.readMessage(remoteType, body);
    } catch {
      return;
    }

    const type = message.constructor as IMessage;

    if (type.responds) return;
    this.#routerChain.notify(type.id, message as Inbound<P, L>, NOTIFICATION_CONTEXT);
  }

  #receiveRequest(
    connection: number,
    session: Session,
    typeId: number,
    exchangeId: number,
    body: BinaryReader
  ): void {
    const incomingFlows = this.#incomingFlows;

    if (incomingFlows.get(exchangeId) !== undefined) return;
    if (incomingFlows.readyCount >= this.#settings.limits.readyItems)
      return this.writeRejection(connection, exchangeId, ErrorCode.Overload);

    let remoteType: IMessage;
    let type: IMessage;

    try {
      remoteType = session.getRemoteType(this.#protocol.messages, this.#role, typeId);
      type = session.getLocalType(remoteType);
    } catch (error) {
      if (!(error instanceof ChannelError)) return this.writeRejection(connection, exchangeId, ErrorCode.MalformedMessage);

      return this.writeRejection(connection, exchangeId, error.code ?? ErrorCode.MalformedMessage, error.violation);
    }

    if (!type.responds) return this.writeRejection(connection, exchangeId, ErrorCode.UnhandledMessage);

    const flow = new IncomingFlow(
      this,
      connection,
      exchangeId,
      type,
      remoteType,
      type.timeout ?? this.#settings.timeout,
      body.getRemainderReader()
    );

    incomingFlows.add(flow);
    this.#transport.enqueue(flow);
  }

  sessionOf(connection: number): Session | undefined {
    return this.#sessionOf(connection);
  }

  #writeSegment(
    connection: number,
    session: Session,
    exchangeId: number,
    type: IMessage,
    segment: MessageSegment,
    value: unknown
  ): void {
    const validating = this.#settings.validating;
    const limit = this.#getBodyLimit(type);
    const frames = this.#beginFrame(connection, this.#settings.bundle, session.minor);

    try {
      frames.validate(validating, limit);
      session.writeSegment(frames, segment, exchangeId, type, value);
    } finally {
      this.#transport.commit();
    }
  }

  #getBodyLimit(type: IMessage): number {
    return this.#settings.validating && this.#role === DefinitionRole.Client ? type.limit : NO_LIMIT;
  }

  #writeCredit(
    connection: number,
    kind: FrameKind.ResponseCredit | FrameKind.MessageCredit,
    exchangeId: number,
    count: number
  ): void {
    const session = this.#sessionOf(connection);

    if (session === undefined) return;

    const frames = this.#transport.begin(
      this.#routeOf(connection),
      this.#settings.bundle,
      session.minor,
      this.#settings.maxMessage
    );

    try {
      frames.credit(kind, exchangeId, count);
    } finally {
      this.#transport.commit();
    }
  }

  #onDeadline(): void {
    this.#cancelDeadlineTimer = undefined;
    this.#nextDeadline = undefined;

    const now = this.#transport.driver.now();
    let next: number | undefined;

    for (const flow of this.#incomingFlows.toArray()) {
      if (flow.deadline !== undefined && flow.deadline <= now) flow.onDeadline(now);
      if (!(
        this.#incomingFlows.get(flow.exchangeId) === flow &&
        flow.deadline !== undefined &&
        (next === undefined || flow.deadline < next)
      ))
        continue;
      next = flow.deadline;
    }

    if (next === undefined) return;
    this.scheduleDeadline(next);
  }

  #onFragment(connection: number | undefined, core: number, bytes: Uint8Array, isLastPiece: boolean): void {
    let fragments = this.#reassembly;

    if (fragments === undefined) {
      const head = FrameReader.head(bytes);
      const oversized =
        head === undefined || (head.length > this.#settings.maxMessage && !FrameReader.isControl(head.kind));

      fragments = this.#reassembly = new Reassembly(oversized ? 0 : head.length);
      if (oversized) this.#refuseOverLimit(connection, head);
    }

    if (!fragments.append(bytes)) {
      if (isLastPiece) this.#reassembly = undefined;

      return;
    }

    if (!isLastPiece) return;
    this.#reassembly = undefined;

    const frame = fragments.bytes;

    this.#readFrames(frame, 0, frame.length, connection, core);
  }

  #refuseOverLimit(connection: number | undefined, head: FrameHead | undefined): void {
    if (connection === undefined || head?.id === undefined) return;
    switch (head.kind) {
      case FrameKind.Message:
        return this.writeRejection(connection, head.id, ErrorCode.Overload);
      case FrameKind.MessageItem:
      case FrameKind.MessageEnd:
        return this.#incomingFlows.get(head.id)?.rejectAndCancel(ErrorCode.Overload);
      case FrameKind.ResponseStart:
      case FrameKind.ResponseItem:
      case FrameKind.ResponseEnd:
      case FrameKind.Failure:
        return this.#outgoingFlows.get(head.id)?.cancel(ErrorCode.Overload);
      default:
        return;
    }
  }

  #sessionOf(connection: number): Session | undefined {
    return this.#connection.activeId === connection ? this.#connection.session : undefined;
  }

  #routeOf(connection: number | undefined): Route {
    const route = this.#cachedRoute;

    if (route !== undefined && route.connection === connection) return route;

    return (this.#cachedRoute = { from: this.#role, channel: this.#protocol.channelId, connection });
  }
}
