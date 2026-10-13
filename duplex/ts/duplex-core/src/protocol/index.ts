import type { BinaryReader } from '../binary/reader.js';
import type { BinaryWriter } from '../binary/writer.js';
import type { HealthCheck } from '../channel/options.js';
import type { Exchange } from './exchange.js';
import type { IMessage, Message } from './message.js';
import type { Tag } from './tags.js';

export { Role } from '../definition.generated.js';

export interface VersionNumber {
  readonly major: number;
  readonly minor: number;
}

export const VersionNumber = {
  isWithin(version: VersionNumber, newer: VersionNumber): boolean {
    return version.major === newer.major && version.minor <= newer.minor;
  },

  lowerMinorVersion(version: VersionNumber, other: VersionNumber): VersionNumber {
    return { major: version.major, minor: Math.min(version.minor, other.minor) };
  }
};

export const CORE: VersionNumber = { major: 1, minor: 0 };

export interface VersionInfo {
  readonly fingerprint: number;
  readonly deprecationNote?: string;
  readonly unsupported?: boolean;
}

export interface IHandshake {
  writeOpen(writer: BinaryWriter, open: never): void;
  readOpen(reader: BinaryReader): unknown;
  writeAck(writer: BinaryWriter, ack: never): void;
  readAck(reader: BinaryReader): unknown;
  writeRefusal(writer: BinaryWriter, refusal: never): void;
  readRefusal(reader: BinaryReader): unknown;
}

export interface TransportFlavour {
  readonly healthCheck?: Partial<HealthCheck> | false;
  readonly streamWindow?: number;
  readonly timeout?: number;
  readonly maxMessage?: number;
}

export type SecurityLevel = 'encrypted' | 'authenticated';

export interface Protocol {
  readonly channel: string;
  readonly channelId: number;
  readonly version: VersionNumber;
  readonly core: VersionNumber;
  readonly versions: readonly VersionInfo[];
  readonly peers: readonly [number, number];
  readonly messages: readonly [readonly IMessage[], readonly IMessage[]];
  readonly handshake?: IHandshake;
  readonly flavours?: Readonly<Record<string, TransportFlavour>>;
  readonly security?: SecurityLevel;
  readonly signed?: boolean;
  readonly requiredCapabilities?: number;

  readonly tags?: readonly Tag[];
}

export type PeerOf<P extends Protocol> = P['peers'][number];
export type RemoteOf<P extends Protocol, L extends PeerOf<P>> = Exclude<PeerOf<P>, L>;
export type MessageOf<P extends Protocol> = InstanceType<P['messages'][number][number]>;
export type Inbound<P extends Protocol, L extends PeerOf<P>> = Extract<
  MessageOf<P>,
  Message<RemoteOf<P, L>, object, Exchange>
>;
export type OpenOf<P extends Protocol> = P['handshake'] extends IHandshake
  ? ReturnType<P['handshake']['readOpen']>
  : undefined;
export type AckOf<P extends Protocol> = P['handshake'] extends IHandshake
  ? ReturnType<P['handshake']['readAck']>
  : undefined;
export type RefusalOf<P extends Protocol> = P['handshake'] extends IHandshake
  ? ReturnType<P['handshake']['readRefusal']>
  : undefined;
