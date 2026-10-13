export enum ExchangeKind {
  None,
  Value,
  ResponseStream,
  MessageStream
}

export interface NoResponse {
  readonly kind: ExchangeKind.None;
}

export interface ValueResponse<R, F = never> {
  readonly kind: ExchangeKind.Value;
  readonly response: R;
  readonly failure: F;
}

export interface StreamResponse<S, E, I, F = never> {
  readonly kind: ExchangeKind.ResponseStream;
  readonly start: S;
  readonly end: E;
  readonly item: I;
  readonly failure: F;
}

export type ResponseExchange = ValueResponse<unknown, unknown> | StreamResponse<unknown, unknown, unknown, unknown>;

export interface StreamExchange<E, I, X extends ResponseExchange> {
  readonly kind: ExchangeKind.MessageStream;
  readonly end: E;
  readonly item: I;
  readonly response: X;
}

export type Exchange = NoResponse | ResponseExchange | StreamExchange<unknown, unknown, ResponseExchange>;

export type ReplyOf<X> =
  X extends ValueResponse<infer R, unknown>
    ? R
    : X extends StreamResponse<unknown, infer E, unknown, unknown>
      ? E
      : X extends StreamExchange<unknown, unknown, infer Y>
        ? ReplyOf<Y>
        : void;
