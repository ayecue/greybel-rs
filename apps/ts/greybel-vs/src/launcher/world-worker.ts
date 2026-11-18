import { WorkerHost } from 'duplex-driver-worker';
import type { ExtensionContext } from 'vscode';

import type { WorldHost } from '../launcher';
import { WorkerLauncher } from './worker';

/**
 * Window greyhack-world-operations-web worker started on demand. Debug-session runtime workers connect world
 * channels to its ports and share its world.
 */
export class WorldWorker implements WorldHost<MessagePort> {
  readonly #context: ExtensionContext;
  #current: { readonly host: WorkerHost<Record<string, never>>; readonly seed: string } | undefined;

  constructor(context: ExtensionContext) {
    this.#context = context;
  }

  /** Starts the game server for `seed` if needed and returns a connection port. */
  connect(seed: string): Promise<MessagePort> {
    if (this.#current?.seed !== seed) {
      this.reset();

      const launcher = new WorkerLauncher(this.#context, 'world-web');

      this.#current = { host: new WorkerHost(launcher.start({ seed }), {}), seed };
    }

    return Promise.resolve(this.#current.host.open());
  }

  /** Stops the game server so the next program uses a new world. */
  reset(): Promise<void> {
    this.dispose();

    return Promise.resolve();
  }

  dispose(): void {
    this.#current?.host.close();
    this.#current = undefined;
  }
}
