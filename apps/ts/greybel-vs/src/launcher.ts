import type { Transport } from 'duplex-core';

export interface Peer {
  readonly transport: Transport;
  stop(): void;
}

export type Launcher = () => Peer;

/**
 * Starts a runtime connected to the window game server's `seed` world for mock programs, or directly to Grey
 * Hack when `seed` is absent.
 */
export type RuntimeLauncher = (seed: string | undefined) => Promise<Peer>;

/** Window game server shared by all runtime programs; `C` is the runtime connection type. */
export interface WorldHost<C> {
  connect(seed: string): Promise<C>;
  reset(): void;
}
