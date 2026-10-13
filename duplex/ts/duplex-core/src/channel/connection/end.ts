import { FrameKind, type ErrorCode } from '../../definition.generated.js';
import { ChannelError } from '../../errors.js';
import type { Session } from '../../migration/session.js';
import type { ConnectionContext } from './context.js';
import { ConnectionStateKind, type ConnectionHandler } from './state.js';

export class ConnectionFailure implements ConnectionHandler {
  readonly kind = ConnectionStateKind.Failure;

  constructor(
    readonly reason: ErrorCode,
    readonly refusal: unknown
  ) {}

  getActiveRoute(): { connection: number; session: Session } | undefined {
    throw ChannelError.channelEnd(this.reason);
  }

  onControl(): void {}

  onHealthCheck(): void {}

  onOpenResult(): void {}

  close(): void {}
}

export class ConnectionEnd implements ConnectionHandler {
  readonly kind = ConnectionStateKind.End;

  constructor(readonly reason: ErrorCode) {}

  getActiveRoute(): { connection: number; session: Session } | undefined {
    throw ChannelError.channelEnd(this.reason);
  }

  onControl(): void {}

  onHealthCheck(): void {}

  onOpenResult(): void {}

  close(): void {}

  static enter(
    connection: number | undefined,
    reason: ErrorCode,
    context: ConnectionContext,
    notifyRemote: boolean
  ): void {
    const finalState = context.enter(new ConnectionEnd(reason));

    if (notifyRemote) context.post({ kind: FrameKind.Close, reason }, connection);
    context.listener.onEnd(connection, reason);
    context.listener.onClose(reason);
    context.listener.onEnter(finalState);
  }
}
