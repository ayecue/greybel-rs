import { BinaryReader } from '../binary/reader.js';
import { ErrorCode } from '../definition.generated.js';
import { ChannelError } from '../errors.js';
import type { FrameEncoder } from '../frame/encoder.js';
import type { Role, VersionNumber } from '../protocol/index.js';
import { MessageSegment, SegmentCodec, type AnyMessage, type IMessage } from '../protocol/message.js';
import type { MigrationPath } from './path.js';

export class Session {
  constructor(
    readonly core: VersionNumber,
    readonly protocol: VersionNumber,
    readonly migrationPath?: MigrationPath
  ) {}

  get minor(): number {
    return this.protocol.minor;
  }

  writeMessage(frames: FrameEncoder, message: AnyMessage, exchangeId: number | undefined): void {
    const path = this.migrationPath;
    const migratedMessage =
      path === undefined
        ? message
        : (path.migrateDown(MessageSegment.Message, message.constructor as IMessage, message) as AnyMessage);
    const type = migratedMessage.constructor as IMessage;

    if (type.since > this.minor) throw ChannelError.unavailable();
    frames.message(type.id, exchangeId, migratedMessage);
  }

  writePendingBody(
    frames: FrameEncoder,
    type: IMessage,
    body: Uint8Array,
    exchangeId: number | undefined,
    newest: number
  ): void {
    if (this.migrationPath === undefined && this.minor >= newest) frames.encodedMessage(type.id, exchangeId, body);
    else this.writeMessage(frames, type.read(new BinaryReader(body)), exchangeId);
  }

  writeSegment(frames: FrameEncoder, segment: MessageSegment, exchangeId: number, type: IMessage, value: unknown): void {
    const path = this.migrationPath;
    const migratedType = path === undefined ? type : path.getOldestType(type);
    const rewritten = path === undefined ? value : path.migrateDown(segment, type, value);

    frames.segment(segment, exchangeId, migratedType, rewritten);
  }

  getRemoteType(inboundMessages: readonly (readonly IMessage[])[], role: Role, typeId: number): IMessage {
    const type = (this.migrationPath?.oldestProtocol.messages ?? inboundMessages)[role]?.[typeId];

    if (type === undefined) throw ChannelError.reject(ErrorCode.UnhandledMessage);

    return type;
  }

  getLocalType(remoteType: IMessage): IMessage {
    return this.migrationPath === undefined ? remoteType : this.migrationPath.getNewestType(remoteType);
  }

  readMessage(remoteType: IMessage, body: BinaryReader): AnyMessage {
    const path = this.migrationPath;
    const message = remoteType.read(body);

    body.finish();

    return path === undefined ? message : (path.migrateUp(MessageSegment.Message, remoteType, message) as AnyMessage);
  }

  readSegment(segment: MessageSegment, type: IMessage, body: BinaryReader): unknown {
    const path = this.migrationPath;
    const codecType = path === undefined || !SegmentCodec.isResponse(segment) ? type : path.getOldestType(type);
    const value = SegmentCodec.read(codecType, segment, body);

    body.finish();

    return path === undefined ? value : path.migrateUp(segment, codecType, value);
  }
}
