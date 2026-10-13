import type { Bundle, PeerOf, Protocol, Transport } from 'duplex-core';

export interface RelayDeclaration<P extends Protocol = Protocol> {
  readonly transport: Transport;
  readonly protocol: P;
  readonly peer: PeerOf<P>;
  readonly bundle?: Bundle;
}

export interface PortDeclaration {
  readonly protocol: Protocol;
  readonly port: MessagePort;
}

export interface WorkerHostOptions {
  readonly relays?: readonly RelayDeclaration[];
  readonly ports?: readonly PortDeclaration[];
}
