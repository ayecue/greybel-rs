import type { BinaryReader } from '../../binary/reader.js';
import { ErrorCode, type Violation } from '../../definition.generated.js';
import { ChannelError } from '../../errors.js';
import type { Session } from '../../migration/session.js';
import { MessageSegment, type IMessage } from '../../protocol/message.js';
import { FinalEventKind } from '../streams/inbox.js';
import { INITIAL_CREDIT } from '../../const.js';
import { Validation } from './validation.js';
import {
  ReceiveWindow,
  Inbox,
  Outbox,
  ResponseStream,
  StreamWriter,
  type ResponseSource,
  type StreamSink
} from '../streams/index.js';

export interface OutgoingHost {
  readonly streamWindow: number;
  readonly validatesInput: boolean;
  readonly validatesOutput: boolean;
  writeMessageSegment(exchangeId: number, type: IMessage, segment: MessageSegment, value: unknown): void;
  writeCancel(exchangeId: number): boolean;
  writeResponseCredit(exchangeId: number, count: number): void;
  withdraw(flow: OutgoingFlow): boolean;
  release(flow: OutgoingFlow): void;
  deferSettlement(flow: OutgoingFlow): boolean;
}

export interface OutgoingState {
  startDispatch(flow: OutgoingFlow): void;
  finishDispatch(flow: OutgoingFlow): void;
  onResponse(flow: OutgoingFlow, segment: MessageSegment, body: BinaryReader, session: Session): void;
  onRejection(flow: OutgoingFlow, code: ErrorCode, violation: Violation | undefined): void;
  onCredit(flow: OutgoingFlow, count: number): void;
  cancel(flow: OutgoingFlow, code: ErrorCode): void;
  end(flow: OutgoingFlow, error: Error): void;
}

class PendingState implements OutgoingState {
  startDispatch(flow: OutgoingFlow): void {
    flow.enter(UNSENT_STATE);
  }

  finishDispatch(flow: OutgoingFlow): void {
    UNSENT_STATE.finishDispatch(flow);
  }

  onResponse(): void {}

  onRejection(): void {}

  onCredit(): void {}

  cancel(flow: OutgoingFlow, code: ErrorCode): void {
    flow.withdraw();
    flow.failWith(ChannelError.reject(code));
  }

  end(flow: OutgoingFlow, error: Error): void {
    flow.failWith(error);
  }
}

class UnsentState implements OutgoingState {
  startDispatch(): void {}

  finishDispatch(flow: OutgoingFlow): void {
    flow.enter(RESPONSE_PENDING_STATE);
    flow.outbox?.grant(INITIAL_CREDIT);
    flow.grantInitialCredit();
  }

  onResponse(): void {}

  onRejection(): void {}

  onCredit(): void {}

  cancel(flow: OutgoingFlow, code: ErrorCode): void {
    flow.cancelWith(ChannelError.reject(code));
  }

  end(flow: OutgoingFlow, error: Error): void {
    flow.failWith(error);
  }
}

class ResponsePendingState implements OutgoingState {
  startDispatch(): void {}

  finishDispatch(): void {}

  onResponse(flow: OutgoingFlow, segment: MessageSegment, body: BinaryReader, session: Session): void {
    let value: unknown;

    try {
      flow.validateSegment(segment, body);
      value = session.readSegment(segment, flow.type, body);
    } catch (error) {
      const failure = error instanceof ChannelError ? error : ChannelError.reject(ErrorCode.MalformedMessage);

      if (segment === MessageSegment.ResponseEnd || segment === MessageSegment.Failure)
        return flow.receiveFinal(() => flow.failWith(failure));

      return flow.receiveItem(() => flow.cancelWith(failure));
    }

    switch (segment) {
      case MessageSegment.ResponseStart:
        return flow.inbox?.start(value);
      case MessageSegment.ResponseItem:
        return flow.receiveItem(() => flow.inbox?.push(value));
      case MessageSegment.Failure:
        return flow.receiveFinal(() => flow.failWith(ChannelError.fail(value)));
      default:
        return flow.receiveFinal(() => flow.completeWith(value));
    }
  }

  onRejection(flow: OutgoingFlow, code: ErrorCode, violation: Violation | undefined): void {
    const error = violation === undefined ? ChannelError.reject(code) : ChannelError.invalid(violation);

    flow.receiveFinal(() => flow.failWith(error));
  }

  onCredit(flow: OutgoingFlow, count: number): void {
    flow.outbox?.grant(count);
  }

  cancel(flow: OutgoingFlow, code: ErrorCode): void {
    flow.cancelWith(ChannelError.reject(code));
  }

  end(flow: OutgoingFlow, error: Error): void {
    flow.failWith(error);
  }
}

class CancelPendingState implements OutgoingState {
  startDispatch(): void {}

  finishDispatch(): void {}

  onResponse(flow: OutgoingFlow, segment: MessageSegment): void {
    if (segment !== MessageSegment.ResponseEnd && segment !== MessageSegment.Failure) return;
    flow.receiveFinal(() => {
      flow.enter(OUTCOME_STATE);
      flow.release();
    });
  }

  onRejection(flow: OutgoingFlow): void {
    flow.receiveFinal(() => {
      flow.enter(OUTCOME_STATE);
      flow.release();
    });
  }

  onCredit(): void {}

  cancel(): void {}

  end(flow: OutgoingFlow): void {
    flow.enter(OUTCOME_STATE);
    flow.release();
  }
}

class DeliveryPendingState implements OutgoingState {
  constructor(private readonly delivery?: () => void) {}

  get isFinal(): boolean {
    return this.delivery !== undefined;
  }

  startDispatch(): void {}

  finishDispatch(): void {}

  onResponse(flow: OutgoingFlow, segment: MessageSegment, body: BinaryReader, session: Session): void {
    if (this.delivery !== undefined) return;
    RESPONSE_PENDING_STATE.onResponse(flow, segment, body, session);
  }

  onRejection(flow: OutgoingFlow, code: ErrorCode, violation: Violation | undefined): void {
    if (this.delivery !== undefined) return;
    RESPONSE_PENDING_STATE.onRejection(flow, code, violation);
  }

  onCredit(flow: OutgoingFlow, count: number): void {
    if (this.delivery !== undefined) return;
    flow.outbox?.grant(count);
  }

  cancel(flow: OutgoingFlow, code: ErrorCode): void {
    if (this.delivery !== undefined) return;
    flow.cancelWith(ChannelError.reject(code));
  }

  end(flow: OutgoingFlow, error: Error): void {
    flow.failWith(error);
  }

  settle(): void {
    this.delivery?.();
  }
}

class OutcomeState implements OutgoingState {
  startDispatch(): void {}

  finishDispatch(): void {}

  onResponse(): void {}

  onRejection(): void {}

  onCredit(): void {}

  cancel(): void {}

  end(): void {}
}

const PENDING_STATE = new PendingState();
const UNSENT_STATE = new UnsentState();
const RESPONSE_PENDING_STATE = new ResponsePendingState();
const CANCEL_PENDING_STATE = new CancelPendingState();
const OUTCOME_STATE = new OutcomeState();

export class OutgoingFlow implements ResponseSource, StreamSink, EventListenerObject {
  readonly exchangeId: number;
  readonly type: IMessage;
  readonly inbox: Inbox | undefined;
  readonly outbox: Outbox | undefined;
  readonly #host: OutgoingHost;
  readonly #receiveWindow: ReceiveWindow | undefined;
  #state: OutgoingState = PENDING_STATE;
  #sentItemCount = 0;
  #pendingDeliveriesItemCount = 0;
  #resolveResponse: ((value: unknown) => void) | undefined;
  #rejectResponse: ((error: Error) => void) | undefined;
  #signal: AbortSignal | undefined;
  readonly #pendingDeliveries: (() => void)[] = [];

  constructor(host: OutgoingHost, exchangeId: number, type: IMessage) {
    this.#host = host;
    this.exchangeId = exchangeId;
    this.type = type;
    this.inbox = type.streamsResponse ? new Inbox() : undefined;
    this.outbox = type.streams ? new Outbox(this, MessageSegment.MessageItem) : undefined;
    this.#receiveWindow = type.streamsResponse
      ? new ReceiveWindow(host.streamWindow, type.responseItemSize)
      : undefined;
  }

  grantInitialCredit(): void {
    if (this.#receiveWindow === undefined) return;

    const count = this.#receiveWindow.initialGrant();

    if (!(count > 0)) return;
    this.#host.writeResponseCredit(this.exchangeId, count);
  }

  getSenderHandle(): unknown {
    const response =
      this.inbox === undefined
        ? new Promise((resolve, reject) => ((this.#resolveResponse = resolve), (this.#rejectResponse = reject)))
        : new ResponseStream(this, this.inbox);

    return this.outbox === undefined ? response : { stream: new StreamWriter(this, this.outbox), response };
  }

  attachSignal(signal: AbortSignal): void {
    if (this.#state === OUTCOME_STATE) return;
    this.#signal = signal;
    signal.addEventListener('abort', this, { once: true });
  }

  startDispatch(): void {
    this.#state.startDispatch(this);
  }

  finishDispatch(): void {
    this.#state.finishDispatch(this);
  }

  onResponse(segment: MessageSegment, body: BinaryReader, session: Session): void {
    this.#state.onResponse(this, segment, body, session);
  }

  onRejection(code: ErrorCode, violation?: Violation): void {
    this.#state.onRejection(this, code, violation);
  }

  receiveItem(delivery: () => void): void {
    if (!this.#host.deferSettlement(this)) return delivery();
    this.#pendingDeliveries.push(delivery);
    if (this.#state !== RESPONSE_PENDING_STATE) return;
    this.enter(new DeliveryPendingState());
  }

  receiveFinal(delivery: () => void): void {
    if (!this.#host.deferSettlement(this)) return delivery();
    this.enter(new DeliveryPendingState(delivery));
  }

  settle(): void {
    if (this.#state === OUTCOME_STATE) {
      this.#pendingDeliveries.length = 0;

      return;
    }

    const delivery = this.#pendingDeliveries.shift();

    if (delivery !== undefined) {
      delivery();
      if (this.#state instanceof DeliveryPendingState && !this.#state.isFinal && this.#pendingDeliveries.length === 0)
        this.enter(RESPONSE_PENDING_STATE);

      return;
    }

    if (!(this.#state instanceof DeliveryPendingState)) return;
    this.#state.settle();
  }

  validateSegment(segment: MessageSegment, body: BinaryReader): void {
    const validating = this.#host.validatesInput;

    Validation.prepareBody(validating, this.type.limit, body);
    if (segment === MessageSegment.ResponseItem) {
      Validation.validateItem(validating, this.type.responseItemRange, ++this.#pendingDeliveriesItemCount);
      this.#receiveWindow?.onReceive(body.unreadLength);
    } else if (segment === MessageSegment.ResponseEnd && this.type.streamsResponse)
      Validation.validateEnd(validating, this.type.responseItemRange, this.#pendingDeliveriesItemCount);
  }

  onCredit(count: number): void {
    this.#state.onCredit(this, count);
  }

  cancel(code = ErrorCode.Cancellation): void {
    this.#state.cancel(this, code);
  }

  end(error: Error): void {
    this.#state.end(this, error);
  }

  handleEvent(): void {
    this.cancel();
  }

  ackItem(): void {
    const count = this.#receiveWindow?.ackItem() ?? 0;

    if (count === 0) return;
    this.#host.writeResponseCredit(this.exchangeId, count);
  }

  writeSegment(segment: MessageSegment, value: unknown): void {
    const receivesItems = this.#state instanceof DeliveryPendingState && !this.#state.isFinal;

    if (this.#state !== RESPONSE_PENDING_STATE && this.#state !== UNSENT_STATE && !receivesItems)
      throw ChannelError.channelEnd(ErrorCode.ClosedChannel);

    const validating = this.#host.validatesOutput;

    if (segment === MessageSegment.MessageItem)
      Validation.validateItem(validating, this.type.messageItemRange, this.#sentItemCount + 1);
    else Validation.validateEnd(validating, this.type.messageItemRange, this.#sentItemCount);
    try {
      this.#host.writeMessageSegment(this.exchangeId, this.type, segment, value);
    } catch (error) {
      if (error instanceof ChannelError && error.violation !== undefined) throw error;
      this.cancelWith(ChannelError.reject(ErrorCode.Cancellation));
      throw error;
    }

    if (segment !== MessageSegment.MessageItem) return;
    this.#sentItemCount++;
  }

  cancelWith(error: ChannelError): void {
    this.enter(CANCEL_PENDING_STATE);
    this.deliverError(error);
    if (this.writeCancel()) return;
    this.enter(OUTCOME_STATE);
    this.release();
  }

  enter(state: OutgoingState): void {
    this.#state = state;
  }

  completeWith(value: unknown): void {
    this.#state = OUTCOME_STATE;
    this.release();
    this.outbox?.close();
    if (this.inbox === undefined) this.#resolveResponse?.(value);
    else this.inbox.end({ kind: FinalEventKind.Value, value });
  }

  failWith(error: Error): void {
    this.#state = OUTCOME_STATE;
    this.release();
    this.deliverError(error);
  }

  deliverError(error: Error): void {
    this.outbox?.fail(error instanceof ChannelError ? error : ChannelError.reject(ErrorCode.MalformedMessage));
    if (this.inbox === undefined) this.#rejectResponse?.(error);
    else this.inbox.end({ kind: FinalEventKind.Error, error });
  }

  release(): void {
    this.#signal?.removeEventListener('abort', this);
    this.#signal = undefined;
    this.#host.release(this);
  }

  withdraw(): void {
    this.#host.withdraw(this);
  }

  writeCancel(): boolean {
    return this.#host.writeCancel(this.exchangeId);
  }
}
