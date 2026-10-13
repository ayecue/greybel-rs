import {
  FIRST_CONNECTION,
  FX_MULTIPLIER,
  FX_ROTATE,
  NO_BYTES,
  NO_CONNECTION,
  U32_BITS,
  WORD_BYTES
} from '../../const.js';
import { ErrorCode, FrameKind } from '../../definition.generated.js';
import type { Control } from '../../frame/control.js';
import { Session } from '../../migration/session.js';
import { CORE, VersionNumber } from '../../protocol/index.js';
import { ConnectionActive } from './active.js';
import { ConnectionContext, type Ack } from './context.js';
import { ConnectionEnd, ConnectionFailure } from './end.js';
import { ConnectionStateKind, OpenOutcomeKind, type ConnectionHandler, type OpenOutcome } from './state.js';

export class ConnectionInit implements ConnectionHandler {
  readonly kind = ConnectionStateKind.Init;
  readonly reason = undefined;

  constructor(readonly id: number) {}

  start(context: ConnectionContext): void {
    context.resetChecks();
    if (context.isClient) this.#postOpen(context);
    else context.post({ kind: FrameKind.Listen });
  }

  getActiveRoute(): undefined {
    return undefined;
  }

  onControl(control: Control, _connection: number | undefined, context: ConnectionContext): void {
    if (!context.isClient) {
      if (control.kind === FrameKind.Open) context.negotiate(control, undefined);

      return;
    }

    switch (control.kind) {
      case FrameKind.Listen:
        return this.#postOpen(context);
      case FrameKind.Ack:
        if (control.id === this.id) this.#onAck(control, context);

        return;
      case FrameKind.Refuse: {
        if (control.id !== this.id) return;

        let refusal: unknown;

        if (control.reason === ErrorCode.Denial && context.handshake !== undefined) {
          try {
            refusal = ConnectionContext.decodePayload(control.payload, (reader) =>
              context.handshake!.readRefusal(reader)
            );
          } catch {
            refusal = undefined;
          }
        }

        return ConnectionInit.enterFailure(control.reason, refusal, context);
      }

      default:
        return;
    }
  }

  onHealthCheck(context: ConnectionContext): void {
    const healthCheck = context.healthCheck;

    if (healthCheck === undefined || !context.isClient) return;
    context.missedChecks++;
    if (context.missedChecks > healthCheck.closeThreshold)
      return ConnectionInit.enterFailure(ErrorCode.Timeout, undefined, context);
    this.#postOpen(context);
  }

  onOpenResult(id: number, outcome: OpenOutcome, random: Uint8Array, context: ConnectionContext): void {
    const negotiatedOpen = context.negotiatedOpen;

    if (negotiatedOpen?.id !== id) return;
    context.negotiatedOpen = undefined;

    const handshake = context.handshake;
    let payload: Uint8Array;

    try {
      switch (outcome.kind) {
        case OpenOutcomeKind.Failure:
          return context.refuse(id, outcome.reason);
        case OpenOutcomeKind.Denial:
          return context.refuse(
            id,
            ErrorCode.Denial,
            ConnectionContext.encodePayload(
              handshake && ((writer) => handshake.writeRefusal(writer, outcome.refusal as never))
            )
          );
        case OpenOutcomeKind.Acceptance:
          payload = ConnectionContext.encodePayload(
            handshake && ((writer) => handshake.writeAck(writer, outcome.ack as never))
          );
      }
    } catch {
      return context.refuse(id, ErrorCode.MalformedMessage);
    }

    const { session, deprecation } = negotiatedOpen;
    const ack: Ack = {
      kind: FrameKind.Ack,
      id,
      connection: ConnectionInit.generateConnectionId(random),
      core: session.core,
      protocol: session.protocol,
      fingerprint: negotiatedOpen.fingerprint,
      deprecation: deprecation?.note ?? null,
      payload
    };

    context.ack = ack;
    ConnectionActive.connect(ack.connection, id, session, undefined, deprecation, context, ack);
  }

  close(reason: ErrorCode, _notifyRemote: boolean, context: ConnectionContext): void {
    ConnectionEnd.enter(undefined, reason, context, false);
  }

  static enterFailure(
    reason: ErrorCode,
    refusal: unknown,
    context: ConnectionContext,
    connectionToClose?: number
  ): void {
    const failure = context.enter(new ConnectionFailure(reason, refusal));

    if (connectionToClose !== undefined) context.post({ kind: FrameKind.Close, reason }, connectionToClose);
    context.listener.onEnd(undefined, reason);
    context.listener.onClose(reason);
    context.listener.onEnter(failure);
  }

  #postOpen(context: ConnectionContext): void {
    const version = context.flavour.version;

    context.post({
      kind: FrameKind.Open,
      id: this.id,
      core: CORE,
      protocol: version,
      fingerprint: context.flavour.getFingerprint(version.minor) ?? 0,
      payload: context.openPayload ?? NO_BYTES
    });
  }

  #onAck(ack: Ack, context: ConnectionContext): void {
    const flavour = context.flavour;
    const agrees =
      VersionNumber.isWithin(ack.core, CORE) &&
      VersionNumber.isWithin(ack.protocol, flavour.version) &&
      flavour.getFingerprint(ack.protocol.minor) === ack.fingerprint;
    let reason: ErrorCode | undefined = agrees ? undefined : ErrorCode.VersionMismatch;
    let value: unknown;

    if (reason === undefined && context.handshake !== undefined) {
      try {
        value = ConnectionContext.decodePayload(ack.payload, (reader) => context.handshake!.readAck(reader));
      } catch {
        reason = ErrorCode.MalformedMessage;
      }
    }

    if (reason !== undefined) return ConnectionInit.enterFailure(reason, undefined, context, ack.connection);

    const deprecation = ack.deprecation === null ? undefined : { version: flavour.version, note: ack.deprecation };

    ConnectionActive.connect(ack.connection, this.id, new Session(ack.core, ack.protocol), value, deprecation, context);
  }

  static generateConnectionId(random: Uint8Array): number {
    const view = new DataView(random.buffer, random.byteOffset, random.byteLength);
    let hash = 0;

    for (let offset = 0; offset < random.byteLength; offset += WORD_BYTES)
      hash = Math.imul((hash + view.getUint32(offset, true)) >>> 0, FX_MULTIPLIER) >>> 0;

    const id = ((hash << FX_ROTATE) | (hash >>> (U32_BITS - FX_ROTATE))) >>> 0;

    return id === NO_CONNECTION ? FIRST_CONNECTION : id;
  }
}
