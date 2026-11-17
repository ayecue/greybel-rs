import type { Peer, RuntimeLauncher, WorldHost } from '../launcher';

/**
 * Creates runtimes through a peer launcher. Mock runtimes share the window game server `world` across debug
 * sessions; game runtimes omit it.
 */
export class RuntimePeerLauncher<C> implements RuntimeLauncher {
  readonly #world: WorldHost<C>;
  readonly #launchPeer: (world: C | undefined) => Peer;

  constructor(world: WorldHost<C>, launchPeer: (world: C | undefined) => Peer) {
    this.#world = world;
    this.#launchPeer = launchPeer;
  }

  async launch(seed: string | undefined): Promise<Peer> {
    return this.#launchPeer(seed === undefined ? undefined : await this.#world.connect(seed));
  }
}
