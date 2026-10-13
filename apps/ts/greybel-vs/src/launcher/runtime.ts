import type { Peer, RuntimeLauncher, WorldHost } from '../launcher';

/**
 * Creates runtimes through `launch`. Mock runtimes share the window game server `world` across debug
 * sessions; game runtimes omit it.
 */
export function createRuntimeLauncher<C>(world: WorldHost<C>, launch: (world: C | undefined) => Peer): RuntimeLauncher {
  return async (seed) => launch(seed === undefined ? undefined : await world.connect(seed));
}
