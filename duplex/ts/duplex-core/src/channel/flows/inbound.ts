import type { BinaryReader } from '../../binary/reader.js';
import { ErrorCode, type Violation } from '../../definition.generated.js';
import { ChannelError, ChannelErrorKind } from '../../errors.js';
import type { Session } from '../../migration/session.js';
import { MessageSegment, type AnyMessage, type IMessage } from '../../protocol/message.js';
import { RouteContext } from '../router.js';
import { FinalEventKind } from '../streams/inbox.js';
import { INITIAL_CREDIT } from '../../const.js';
import type { Work } from '../../transport/work-queue.js';
import { Validation } from './validation.js';
import {
  ReceiveWindow,
  Inbox,
  IncomingStream,
  Outbox,
  ResponseWriter,
  type ItemSink,
  type ItemSource
} from '../streams/index.js';

export interface IncomingHost {
  sessionOf(connection: number): Session | undefined;
  readonly streamWindow: number;
  readonly validatesInput: boolean;
  readonly validatesOutput: boolean;
  now(): number;
  scheduleDeadline(at: number): void;
  route(typeId: number, message: AnyMessage, context: RouteContext): unknown;
  writeResponseSegment(
    connection: number,
    exchangeId: number,
    type: IMessage,
    segment: MessageSegment,
    value: unknown
  ): void;
  writeRejection(connection: number, exchangeId: number, code: ErrorCode, violation?: Violation): void;
  writeMessageCredit(connection: number, exchangeId: number, count: number): void;
  canHandle(): boolean;
  onBusy(flow: IncomingFlow): void;
  onFinish(flow: IncomingFlow): void;
}

export interface IncomingState {
  proceed(flow: IncomingFlow): void;
  onSegment(flow: IncomingFlow, segment: MessageSegment, body: BinaryReader, session: Session): void;
  onCredit(flow: IncomingFlow, count: number): void;
  cancel(flow: IncomingFlow): void;
  end(flow: IncomingFlow, reason: ErrorCode): void;
  onReturn(flow: IncomingFlow, segment: MessageSegment, value: unknown): void;
  onReject(flow: IncomingFlow, code: ErrorCode): void;
}

class ReadyState implements IncomingState {
  proceed(flow: IncomingFlow): void {
    if (!flow.tryHandle()) {
      flow.rejectAndCancel(ErrorCode.Overload);
      flow.enter(COMPLETE_STATE);

      return;
    }

    flow.readMessage();
  }

  onSegment(flow: IncomingFlow, segment: MessageSegment, body: BinaryReader): void {
    flow.refreshDeadline();
    flow.storeEarlySegment(segment, body);
  }

  onCredit(flow: IncomingFlow, count: number): void {
    flow.outbox?.grant(count);
  }

  cancel(flow: IncomingFlow): void {
    flow.rejectAndCancel(ErrorCode.Cancellation);
    flow.enter(COMPLETE_STATE);
  }

  end(flow: IncomingFlow, reason: ErrorCode): void {
    flow.enter(COMPLETE_STATE);
    flow.abort(ChannelError.channelEnd(reason));
  }

  onReturn(): void {}

  onReject(): void {}
}

class BusyState implements IncomingState {
  proceed(): void {}

  onSegment(flow: IncomingFlow, segment: MessageSegment, body: BinaryReader, session: Session): void {
    flow.refreshDeadline();
    flow.readSegment(segment, body, session);
  }

  onCredit(flow: IncomingFlow, count: number): void {
    flow.outbox?.grant(count);
  }

  cancel(flow: IncomingFlow): void {
    flow.rejectAndCancel(ErrorCode.Cancellation);
  }

  end(flow: IncomingFlow, reason: ErrorCode): void {
    flow.enter(COMPLETE_STATE);
    flow.abort(ChannelError.channelEnd(reason));
  }

  onReturn(flow: IncomingFlow, segment: MessageSegment, value: unknown): void {
    flow.enter(COMPLETE_STATE);
    flow.finish();
    flow.respond(segment, value);
  }

  onReject(flow: IncomingFlow, code: ErrorCode): void {
    flow.enter(COMPLETE_STATE);
    flow.finish();
    flow.reject(code);
  }
}

class CancellationState implements IncomingState {
  proceed(): void {}

  onSegment(): void {}

  onCredit(): void {}

  cancel(): void {}

  end(flow: IncomingFlow): void {
    flow.enter(COMPLETE_STATE);
  }

  onReturn(flow: IncomingFlow): void {
    flow.enter(COMPLETE_STATE);
  }

  onReject(flow: IncomingFlow): void {
    flow.enter(COMPLETE_STATE);
  }
}

class CompleteState implements IncomingState {
  proceed(): void {}

  onSegment(): void {}

  onCredit(): void {}

  cancel(): void {}

  end(): void {}

  onReturn(): void {}

  onReject(): void {}
}

const READY_STATE = new ReadyState();
const BUSY_STATE = new BusyState();
const CANCELLATION_STATE = new CancellationState();
const COMPLETE_STATE = new CompleteState();

export class IncomingFlow implements ItemSource, ItemSink, Work {
  readonly exchangeId: number;
  readonly connection: number;
  readonly type: IMessage;
  readonly remoteType: IMessage;
  readonly inbox: Inbox | undefined;
  readonly outbox: Outbox | undefined;
  readonly routeContext: RouteContext;
  deadline: number | undefined;
  readonly #timeout: number | undefined;
  readonly #host: IncomingHost;
  readonly #receiveWindow: ReceiveWindow | undefined;
  #state: IncomingState = READY_STATE;
  #body: BinaryReader | undefined;
  #earlySegments: [MessageSegment, BinaryReader][] | undefined;
  #sentItemCount = 0;
  #receivedItemCount = 0;
  #abortController: AbortController | undefined;
  #aborted = false;

  constructor(
    host: IncomingHost,
    connection: number,
    exchangeId: number,
    type: IMessage,
    remoteType: IMessage,
    timeout: number | undefined,
    body: BinaryReader
  ) {
    this.#host = host;
    this.#body = body;
    this.#timeout = timeout;
    if (timeout !== undefined) {
      this.deadline = host.now() + timeout;
      host.scheduleDeadline(this.deadline);
    }

    this.connection = connection;
    this.exchangeId = exchangeId;
    this.type = type;
    this.remoteType = remoteType;
    this.inbox = type.streams ? new Inbox() : undefined;
    this.#receiveWindow = type.streams ? new ReceiveWindow(host.streamWindow, type.messageItemSize) : undefined;
    this.outbox = type.streamsResponse ? new Outbox(this, MessageSegment.ResponseItem) : undefined;
    this.outbox?.grant(INITIAL_CREDIT);

    const incomingStream = this.inbox === undefined ? undefined : new IncomingStream(this, this.inbox);
    const responseWriter =
      this.outbox === undefined ? undefined : new ResponseWriter(this, this.outbox, type.startsResponse);

    this.routeContext = new RouteContext(incomingStream, responseWriter, this);
  }

  get signal(): AbortSignal {
    if (this.#abortController === undefined) {
      this.#abortController = new AbortController();
      if (this.#aborted) this.#abortController.abort();
    }

    return this.#abortController.signal;
  }

  proceed(): void {
    this.#state.proceed(this);
  }

  readMessage(): void {
    const body = this.#body!;

    this.#body = undefined;

    const session = this.#host.sessionOf(this.connection);

    if (session === undefined) return;

    let message: AnyMessage;

    try {
      Validation.prepareBody(this.#host.validatesInput, this.remoteType.limit, body);
      message = session.readMessage(this.remoteType, body);
    } catch (error) {
      return this.rejectWith(error);
    }

    this.#route(message);

    const earlySegment = this.#earlySegments;

    if (earlySegment === undefined) return;
    this.#earlySegments = undefined;
    for (const [segment, body] of earlySegment)
      if (this.#state === BUSY_STATE) this.readSegment(segment, body, session);
  }

  readSegment(segment: MessageSegment, body: BinaryReader, session: Session): void {
    const inbox = this.inbox;

    if (inbox === undefined) return;

    let value: unknown;

    try {
      this.validateSegment(segment, body);
      value = session.readSegment(segment, this.remoteType, body);
    } catch (error) {
      return this.rejectWith(error);
    }

    if (segment === MessageSegment.MessageEnd) inbox.end({ kind: FinalEventKind.Value, value });
    else inbox.push(value);
  }

  storeEarlySegment(segment: MessageSegment, body: BinaryReader): void {
    (this.#earlySegments ??= []).push([segment, body.getRemainderReader()]);
  }

  #route(message: AnyMessage): void {
    if (this.#receiveWindow !== undefined) {
      const count = this.#receiveWindow.initialGrant();

      if (count > 0) this.#host.writeMessageCredit(this.connection, this.exchangeId, count);
    }

    let routeResult: unknown;

    try {
      routeResult = this.#host.route(this.type.id, message, this.routeContext);
    } catch (error) {
      return this.#onFailure(error);
    }

    if (typeof (routeResult as PromiseLike<unknown> | undefined)?.then === 'function') {
      (routeResult as PromiseLike<unknown>).then(
        (value) => this.#onReturn(value),
        (error: unknown) => this.#onFailure(error)
      );
    } else {
      this.#onReturn(routeResult);
    }
  }

  onSegment(segment: MessageSegment, body: BinaryReader, session: Session): void {
    this.#state.onSegment(this, segment, body, session);
  }

  onCredit(count: number): void {
    this.#state.onCredit(this, count);
  }

  cancel(): void {
    this.#state.cancel(this);
  }

  end(reason: ErrorCode): void {
    this.#state.end(this, reason);
  }

  enter(state: IncomingState): void {
    if (this.#state === READY_STATE && state !== READY_STATE) this.#host.onBusy(this);
    this.#state = state;
  }

  tryHandle(): boolean {
    if (!this.#host.canHandle()) return false;
    this.enter(BUSY_STATE);

    return true;
  }

  ackItem(): void {
    const count = this.#receiveWindow?.ackItem() ?? 0;

    if (count === 0) return;
    if (this.#state !== BUSY_STATE) return;
    this.#host.writeMessageCredit(this.connection, this.exchangeId, count);
  }

  refreshDeadline(): void {
    if (this.#timeout === undefined) return;
    this.deadline = this.#host.now() + this.#timeout;
  }

  onDeadline(now: number): void {
    if (this.outbox?.isWaitingForCredit === true) {
      this.deadline = now + this.#timeout!;

      return;
    }

    if (this.#state === BUSY_STATE) this.rejectAndCancel(ErrorCode.Timeout);
    else if (this.#state === READY_STATE) {
      this.rejectAndCancel(ErrorCode.Timeout);
      this.enter(COMPLETE_STATE);
    }
  }

  writeSegment(segment: MessageSegment, value: unknown): void {
    if (this.#state !== BUSY_STATE) throw ChannelError.reject(ErrorCode.Cancellation);
    this.refreshDeadline();
    try {
      this.#validateOutput(segment);
      this.#host.writeResponseSegment(this.connection, this.exchangeId, this.type, segment, value);
    } catch (error) {
      this.rejectWith(error);
      throw error;
    }

    if (segment !== MessageSegment.ResponseItem) return;
    this.#sentItemCount++;
  }

  #validateOutput(segment: MessageSegment): void {
    const validating = this.#host.validatesOutput;

    if (segment === MessageSegment.ResponseItem)
      Validation.validateItem(validating, this.type.responseItemRange, this.#sentItemCount + 1);
    else if (segment === MessageSegment.ResponseEnd && this.type.streamsResponse)
      Validation.validateEnd(validating, this.type.responseItemRange, this.#sentItemCount);
  }

  rejectAndCancel(code: ErrorCode, violation?: Violation): void {
    this.enter(CANCELLATION_STATE);
    this.abort(violation === undefined ? ChannelError.reject(code) : ChannelError.invalid(violation));
    this.finish();
    this.reject(code, violation);
  }

  rejectWith(error: unknown): void {
    if (!(error instanceof ChannelError)) return this.rejectAndCancel(ErrorCode.MalformedMessage);
    this.rejectAndCancel(error.code ?? ErrorCode.MalformedMessage, error.violation);
  }

  validateSegment(segment: MessageSegment, body: BinaryReader): void {
    const validating = this.#host.validatesInput;

    Validation.prepareBody(validating, this.type.limit, body);
    if (segment !== MessageSegment.MessageItem)
      return Validation.validateEnd(validating, this.type.messageItemRange, this.#receivedItemCount);
    Validation.validateItem(validating, this.type.messageItemRange, ++this.#receivedItemCount);
    this.#receiveWindow?.onReceive(body.unreadLength);
  }

  abort(error: ChannelError): void {
    this.#aborted = true;
    this.#abortController?.abort();
    this.outbox?.fail(error);
    this.inbox?.end({ kind: FinalEventKind.Error, error });
  }

  finish(): void {
    this.outbox?.close();
    this.#host.onFinish(this);
  }

  respond(segment: MessageSegment, value: unknown): void {
    try {
      this.#validateOutput(segment);
      this.#host.writeResponseSegment(this.connection, this.exchangeId, this.type, segment, value);
    } catch (error) {
      if (!(error instanceof ChannelError)) return this.reject(ErrorCode.MalformedMessage);
      this.reject(error.code ?? ErrorCode.MalformedMessage, error.violation);
    }
  }

  reject(code: ErrorCode, violation?: Violation): void {
    this.#host.writeRejection(this.connection, this.exchangeId, code, violation);
  }

  #onReturn(value: unknown): void {
    const responseWriter = this.routeContext.responseWriter;

    if (responseWriter === undefined) {
      this.#state.onReturn(this, MessageSegment.ResponseEnd, value);

      return;
    }

    responseWriter.finish().then(
      () => this.#state.onReturn(this, MessageSegment.ResponseEnd, value),
      (error: unknown) => this.#onFailure(error)
    );
  }

  #onFailure(error: unknown): void {
    if (error instanceof ChannelError && error.kind === ChannelErrorKind.Failure) {
      this.#state.onReturn(this, MessageSegment.Failure, error.failure);
    } else {
      this.#state.onReject(
        this,
        error instanceof ChannelError ? (error.code ?? ErrorCode.HandlerFailure) : ErrorCode.HandlerFailure
      );
    }
  }
}
