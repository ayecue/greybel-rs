import { BinaryReader } from '../../binary/reader.js';
import { BinaryWriter } from '../../binary/writer.js';
import { NO_BYTES } from '../../const.js';
import { ErrorCode, FrameKind } from '../../definition.generated.js';
import type { Control } from '../../frame/control.js';
import { MigrationTable } from '../../migration/path.js';
import { Session } from '../../migration/session.js';
import { CORE, VersionNumber, type IHandshake } from '../../protocol/index.js';
import type { IMessage } from '../../protocol/message.js';
import type { OutgoingFlow } from '../flows/outbound.js';
import type { Bundle, HealthCheck } from '../options.js';
import { VersionTable } from './version-table.js';
import type { ConnectionListener, ConnectionState } from './state.js';

export type Ack = Extract<Control, { kind: FrameKind.Ack }>;

type Open = Extract<Control, { kind: FrameKind.Open }>;

export interface NegotiatedOpen {
  readonly id: number;
  readonly session: Session;
  readonly fingerprint: number;
  readonly deprecation: { version: VersionNumber; note: string } | undefined;
}

export interface QueueItem {
  readonly type: IMessage;
  readonly body: Uint8Array;
  readonly bundle: Bundle;
  readonly flow: OutgoingFlow | undefined;
}

export class ConnectionContext {
  readonly flavour: VersionTable;
  readonly handshake: IHandshake | undefined;
  readonly healthCheck: HealthCheck | undefined;
  readonly openPayload: Uint8Array | undefined;
  readonly listener: ConnectionListener;
  readonly migrations = new MigrationTable();
  queue: QueueItem[] = [];
  missedChecks = 0;
  activitySinceCheck = false;
  ack: Ack | undefined;
  negotiatedOpen: NegotiatedOpen | undefined;
  readonly #enterState: (state: ConnectionState) => void;

  constructor(
    flavour: VersionTable,
    handshake: IHandshake | undefined,
    healthCheck: HealthCheck | undefined,
    openPayload: Uint8Array | undefined,
    events: ConnectionListener,
    commit: (state: ConnectionState) => void
  ) {
    this.flavour = flavour;
    this.handshake = handshake;
    this.healthCheck = healthCheck;
    this.openPayload = openPayload;
    this.listener = events;
    this.#enterState = commit;
  }

  recordActivity(): void {
    this.missedChecks = 0;
    this.activitySinceCheck = true;
  }

  resetChecks(): void {
    this.missedChecks = 0;
    this.activitySinceCheck = false;
  }

  get isClient(): boolean {
    return this.openPayload !== undefined;
  }

  post(control: Control, connection?: number): void {
    this.listener.onControlOut(control, connection);
  }

  enter<S extends ConnectionState>(state: S): S {
    this.#enterState(state);

    return state;
  }

  drainQueue(): QueueItem[] {
    const queue = this.queue;

    this.queue = [];

    return queue;
  }

  negotiate(open: Open, openId: number | undefined): void {
    const nativeMajor = this.flavour.version.major;
    const path =
      open.protocol.major === nativeMajor ? undefined : this.migrations.path(open.protocol.major, nativeMajor);

    if (open.protocol.major !== nativeMajor && path === undefined)
      return this.refuse(open.id, ErrorCode.VersionMismatch);

    const flavour =
      path === undefined ? this.flavour : new VersionTable(path.oldestProtocol.version, path.oldestProtocol.versions);
    const refusal = ConnectionContext.#refusalReason(open, flavour);

    if (refusal !== undefined) return this.refuse(open.id, refusal);
    if (openId !== undefined) {
      if (openId !== open.id) return this.refuse(open.id, ErrorCode.Busy);
      if (this.ack !== undefined) this.post(this.ack);

      return;
    }

    if (this.negotiatedOpen !== undefined) return;

    const status = flavour.getVersionInfo(open.protocol.minor);
    const protocol = VersionNumber.lowerMinorVersion(open.protocol, flavour.version);

    this.negotiatedOpen = {
      id: open.id,
      fingerprint: flavour.getFingerprint(protocol.minor) ?? 0,
      session: new Session(VersionNumber.lowerMinorVersion(CORE, open.core), protocol, path),
      deprecation:
        status?.deprecationNote === undefined ? undefined : { version: open.protocol, note: status.deprecationNote }
    };

    let value: unknown;

    try {
      const handshake = this.handshake;

      value =
        handshake === undefined
          ? undefined
          : ConnectionContext.decodePayload(open.payload, (reader) => handshake.readOpen(reader));
    } catch {
      this.negotiatedOpen = undefined;

      return this.refuse(open.id, ErrorCode.MalformedMessage);
    }

    this.listener.onOpen(open.id, value);
  }

  refuse(id: number, reason: ErrorCode, payload: Uint8Array = NO_BYTES): void {
    const version = this.flavour.version;

    this.post({
      kind: FrameKind.Refuse,
      id,
      reason,
      core: CORE,
      protocol: version,
      majors: this.migrations.getReachableMajors(version.major),
      payload
    });
  }

  static #refusalReason(open: Open, flavour: VersionTable): ErrorCode | undefined {
    if (open.core.major !== CORE.major) return ErrorCode.VersionMismatch;
    if (
      open.protocol.minor <= flavour.version.minor &&
      flavour.getFingerprint(open.protocol.minor) !== open.fingerprint
    )
      return ErrorCode.VersionMismatch;
    if (flavour.getVersionInfo(open.protocol.minor)?.unsupported === true) return ErrorCode.UnsupportedVersion;

    return undefined;
  }

  static decodePayload(payload: Uint8Array, read: (reader: BinaryReader) => unknown): unknown {
    const reader = new BinaryReader(payload);
    const value = read(reader);

    reader.finish();

    return value;
  }

  static encodePayload(write: ((writer: BinaryWriter) => void) | undefined): Uint8Array {
    if (write === undefined) return NO_BYTES;

    const writer = new BinaryWriter();

    write(writer);

    return writer.asBytes();
  }
}
