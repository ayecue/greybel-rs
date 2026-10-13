import { ErrorCode, type Violation } from './definition.generated.js';

const FAILURE_MESSAGE = 'the message failed';

let errorMessages: Record<ErrorCode, string> | undefined;

const buildErrorMessages = (): Record<ErrorCode, string> => ({
  [ErrorCode.ClosedChannel]: 'the channel closed',
  [ErrorCode.Timeout]: 'the connection timed out',
  [ErrorCode.VersionMismatch]: "the client and the provider speak versions that can't read each other",
  [ErrorCode.Busy]: 'the channel is already connected',
  [ErrorCode.ConnectionLoss]: 'the connection to the other peer was lost',
  [ErrorCode.Panic]: 'a peer failed',
  [ErrorCode.UnhandledMessage]: 'no handler took the message',
  [ErrorCode.DroppedMessage]: 'the message got no response',
  [ErrorCode.HandlerFailure]: 'the handler of the message failed',
  [ErrorCode.MalformedMessage]: "a message couldn't be written or read",
  [ErrorCode.UnsupportedVersion]: "the version isn't supported",
  [ErrorCode.Denial]: 'the provider denied the client',
  [ErrorCode.Cancellation]: 'the sender cancelled the message',
  [ErrorCode.Insecure]: "the transport isn't secure enough",
  [ErrorCode.Overload]: 'a peer reached its limits',
  [ErrorCode.Invalid]: "a value doesn't meet what its definition requires"
});

export enum ChannelErrorKind {
  Failure,
  Denial,
  Rejection,
  ChannelEnd,
  Unavailable,
  Invalid
}

export class ChannelError<F = unknown> extends Error {
  override readonly name = 'ChannelError';

  private constructor(
    readonly kind: ChannelErrorKind,
    readonly code: ErrorCode | undefined,
    readonly failure: F | undefined,
    readonly violation: Violation | undefined = undefined
  ) {
    super(code === undefined ? FAILURE_MESSAGE : ChannelError.message(code));
  }

  static message(code: ErrorCode): string {
    return (errorMessages ??= buildErrorMessages())[code];
  }

  static fail<F>(failure: F): ChannelError<F> {
    return new ChannelError(ChannelErrorKind.Failure, undefined, failure);
  }

  static deny<F>(refusal: F): ChannelError<F> {
    return new ChannelError(ChannelErrorKind.Denial, ErrorCode.Denial, refusal);
  }

  static unavailable(): ChannelError<never> {
    return new ChannelError<never>(ChannelErrorKind.Unavailable, ErrorCode.UnsupportedVersion, undefined);
  }

  static reject(code: ErrorCode): ChannelError<never> {
    return new ChannelError<never>(ChannelErrorKind.Rejection, code, undefined);
  }

  static invalid(violation: Violation): ChannelError<never> {
    return new ChannelError<never>(ChannelErrorKind.Invalid, ErrorCode.Invalid, undefined, violation);
  }

  static channelEnd(code: ErrorCode): ChannelError<never> {
    return new ChannelError<never>(ChannelErrorKind.ChannelEnd, code, undefined);
  }
}

export class EntropyUnavailable extends Error {
  override readonly name = 'EntropyUnavailable';

  constructor() {
    super('no secret randomness to secure the transport with');
  }
}
