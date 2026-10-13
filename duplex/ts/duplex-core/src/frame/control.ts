import type { ErrorCode, FrameKind } from '../definition.generated.js';
import type { VersionNumber } from '../protocol/index.js';

export type Control =
  | { kind: FrameKind.Listen }
  | {
      kind: FrameKind.Open;
      id: number;
      core: VersionNumber;
      protocol: VersionNumber;
      fingerprint: number;
      payload: Uint8Array;
    }
  | {
      kind: FrameKind.Ack;
      id: number;
      connection: number;
      core: VersionNumber;
      protocol: VersionNumber;
      fingerprint: number;
      deprecation: string | null;
      payload: Uint8Array;
    }
  | {
      kind: FrameKind.Refuse;
      id: number;
      reason: ErrorCode;
      core: VersionNumber;
      protocol: VersionNumber;
      majors: number[];
      payload: Uint8Array;
    }
  | { kind: FrameKind.Ping }
  | { kind: FrameKind.Pong }
  | { kind: FrameKind.Close; reason: ErrorCode };
