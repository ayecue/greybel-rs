import type { ErrorCode } from '../../definition.generated.js';
import type { Control } from '../../frame/control.js';
import type { Session } from '../../migration/session.js';
import type { ConnectionActive } from './active.js';
import type { ConnectionContext, QueueItem } from './context.js';
import type { ConnectionEnd, ConnectionFailure } from './end.js';
import type { Health } from './health.js';
import type { ConnectionInit } from './init.js';

export enum ConnectionStateKind {
  Init,
  Active,
  Failure,
  End
}

export enum OpenOutcomeKind {
  Acceptance,
  Denial,
  Failure
}

export type OpenOutcome =
  | { readonly kind: OpenOutcomeKind.Acceptance; readonly ack: unknown }
  | { readonly kind: OpenOutcomeKind.Denial; readonly refusal: unknown }
  | { readonly kind: OpenOutcomeKind.Failure; readonly reason: ErrorCode };

export type ConnectionState = ConnectionInit | ConnectionActive | ConnectionFailure | ConnectionEnd;

export interface ConnectionListener {
  onControlOut(control: Control, connection: number | undefined): void;
  onConnect(connection: number, session: Session, queue: readonly QueueItem[]): void;
  onOpen(id: number, open: unknown): void;
  onEnd(connection: number | undefined, reason: ErrorCode): void;
  onClose(reason: ErrorCode): void;
  onEnter(state: ConnectionState): void;
  onHealth(health: Health): void;
}

export interface ConnectionHandler {
  readonly reason: ErrorCode | undefined;
  getActiveRoute(): { connection: number; session: Session } | undefined;
  onControl(control: Control, connection: number | undefined, context: ConnectionContext): void;
  onHealthCheck(context: ConnectionContext): void;
  onOpenResult(id: number, outcome: OpenOutcome, random: Uint8Array, context: ConnectionContext): void;
  close(reason: ErrorCode, notifyRemote: boolean, context: ConnectionContext): void;
}
