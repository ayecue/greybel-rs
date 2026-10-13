import { spawn } from 'child_process';
import { Transport } from 'duplex-core';
import { StreamDriver } from 'duplex-driver-stdio';
import { ExtensionContext, Uri } from 'vscode';

import type { Launcher } from '../launcher';

/** Path to the extension's native `binary`. */
export function binaryPath(context: ExtensionContext, binary: string): string {
  return Uri.joinPath(context.extensionUri, 'dist', 'bin', process.platform === 'win32' ? `${binary}.exe` : binary)
    .fsPath;
}

export function createProcessLauncher(
  context: ExtensionContext,
  binary: string,
  args: readonly string[] = []
): Launcher {
  const program = binaryPath(context, binary);

  return () => {
    const child = spawn(program, args, { stdio: ['pipe', 'pipe', 'inherit'] });

    return {
      transport: Transport.of(StreamDriver.child(child)),
      stop: () => child.kill()
    };
  };
}
