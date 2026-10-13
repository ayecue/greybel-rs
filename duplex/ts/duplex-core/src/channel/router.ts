import type { AckOf, Inbound, OpenOf, PeerOf, Protocol } from '../protocol/index.js';
import type { Reply } from '../protocol/message.js';
import type { IncomingStream, ResponseWriter } from './streams/index.js';

declare const ROUTER_PROTOCOL: unique symbol;

export type Next<Output> = () => Promise<Output>;

export type Routed<M> = Reply<M> | Promise<Reply<M>>;

interface SignalHandle {
  readonly signal: AbortSignal;
}

const INERT_SIGNAL: SignalHandle = {
  get signal(): AbortSignal {
    return new AbortController().signal;
  }
};

export class RouteContext {
  readonly incomingStream: IncomingStream<unknown, unknown> | undefined;
  readonly responseWriter: ResponseWriter<unknown, unknown> | undefined;
  readonly #signal: SignalHandle;

  constructor(
    incomingStream?: IncomingStream<unknown, unknown>,
    responseWriter?: ResponseWriter<unknown, unknown>,
    signal: SignalHandle = INERT_SIGNAL
  ) {
    this.incomingStream = incomingStream;
    this.responseWriter = responseWriter;
    this.#signal = signal;
  }

  get signal(): AbortSignal {
    return this.#signal.signal;
  }
}

export abstract class Router<P extends Protocol, L extends PeerOf<P>> {
  declare readonly [ROUTER_PROTOCOL]?: [P, L];
  abstract route<M extends Inbound<P, L>>(
    typeId: number,
    message: M,
    next: Next<Reply<M>>,
    context: RouteContext
  ): Routed<M>;

  routeOpen(_open: OpenOf<P>, next: Next<AckOf<P>>): AckOf<P> | Promise<AckOf<P>> {
    return next();
  }
}
