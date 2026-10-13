import { ErrorCode, FrameKind } from '../../definition.generated.js';
import type { Control } from '../../frame/control.js';
import type { Session } from '../../migration/session.js';
import type { VersionNumber } from '../../protocol/index.js';
import type { Ack, ConnectionContext } from './context.js';
import { ConnectionEnd } from './end.js';
import { ConnectionInit } from './init.js';
import { ConnectionStateKind, type ConnectionHandler } from './state.js';

interface Route {
  readonly connection: number;
  readonly session: Session;
}

export class ConnectionActive implements ConnectionHandler {
  readonly kind = ConnectionStateKind.Active;
  readonly reason = undefined;
  readonly #route: Route;

  private constructor(
    readonly id: number,
    readonly openId: number,
    readonly session: Session,
    readonly ack: unknown,
    readonly deprecation: { version: VersionNumber; note: string } | undefined
  ) {
    this.#route = { connection: id, session };
  }

  get version(): VersionNumber {
    return this.session.protocol;
  }

  static connect(
    id: number,
    openId: number,
    session: Session,
    ack: unknown,
    deprecation: { version: VersionNumber; note: string } | undefined,
    context: ConnectionContext,
    ackFrame?: Ack
  ): void {
    context.resetChecks();

    const queuedItems = context.drainQueue();
    const active = context.enter(new ConnectionActive(id, openId, session, ack, deprecation));

    if (ackFrame !== undefined) context.post(ackFrame);
    context.listener.onConnect(id, session, queuedItems);
    context.listener.onEnter(active);
  }

  getActiveRoute(): Route {
    return this.#route;
  }

  onControl(control: Control, connection: number | undefined, context: ConnectionContext): void {
    const isCurrent = connection === this.id;

    switch (control.kind) {
      case FrameKind.Pong:
        if (isCurrent) context.recordActivity();

        return;
      case FrameKind.Ping:
        if (!isCurrent) return;
        context.recordActivity();

        return context.post({ kind: FrameKind.Pong }, connection);
      case FrameKind.Close:
        if (!isCurrent) return;
        if (context.isClient) return ConnectionEnd.enter(this.id, control.reason, context, false);

        return this.#endAndRelisten(control.reason, false, context);
      case FrameKind.Open:
        if (!context.isClient) context.negotiate(control, this.openId);

        return;
      default:
        return;
    }
  }

  onHealthCheck(context: ConnectionContext): void {
    const healthCheck = context.healthCheck;

    if (healthCheck === undefined) return;
    if (context.activitySinceCheck) {
      context.activitySinceCheck = false;

      return;
    }

    context.missedChecks++;
    if (context.missedChecks <= healthCheck.closeThreshold) return context.post({ kind: FrameKind.Ping }, this.id);

    if (context.isClient) this.close(ErrorCode.Timeout, true, context);
    else this.#endAndRelisten(ErrorCode.Timeout, true, context);
  }

  onOpenResult(): void {}

  close(reason: ErrorCode, notifyRemote: boolean, context: ConnectionContext): void {
    ConnectionEnd.enter(this.id, reason, context, notifyRemote);
  }

  #endAndRelisten(reason: ErrorCode, notifyRemote: boolean, context: ConnectionContext): void {
    context.ack = undefined;

    const initState = context.enter(new ConnectionInit(0));

    if (notifyRemote) context.post({ kind: FrameKind.Close, reason }, this.id);
    context.listener.onEnd(this.id, reason);
    context.listener.onEnter(new ConnectionEnd(reason));
    initState.start(context);
    context.listener.onEnter(initState);
  }
}
