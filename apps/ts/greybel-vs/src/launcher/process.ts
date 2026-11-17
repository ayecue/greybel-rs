import { spawn } from 'child_process';
import { Transport } from 'duplex-core';
import { StreamDriver } from 'duplex-driver-stdio';
import { ExtensionContext, Uri } from 'vscode';

import type { Launcher, Peer } from '../launcher';

export class ProcessLauncher implements Launcher {
  readonly #context: ExtensionContext;
  readonly #binary: string;
  readonly #args: readonly string[];

  constructor(context: ExtensionContext, binary: string, args: readonly string[] = []) {
    this.#context = context;
    this.#binary = binary;
    this.#args = args;
  }

  /** Returns the path to this extension's native binary. */
  getBinaryPath(): string {
    return Uri.joinPath(
      this.#context.extensionUri,
      'dist',
      'bin',
      process.platform === 'win32' ? `${this.#binary}.exe` : this.#binary
    ).fsPath;
  }

  launch(): Peer {
    const child = spawn(this.getBinaryPath(), this.#args, { stdio: ['pipe', 'pipe', 'inherit'] });

    return {
      transport: Transport.of(StreamDriver.child(child)),
      stop: () => child.kill()
    };
  }
}
