import { WorkerHost, type WorkerHostOptions } from 'duplex-driver-worker';
import { ExtensionContext, Uri } from 'vscode';
import { serveWasm } from 'wasm-host/worker';

import type { Launcher } from '../launcher';

/** URL for extension file `file` under dist/. */
export function distUrl(context: ExtensionContext, file: string): string {
  return Uri.joinPath(context.extensionUri, 'dist', file).toString(true);
}

/** Starts dist/`name`.js in a worker with `parameters` in its URL query and supplies `name`.wasm. */
export function startWorker(context: ExtensionContext, name: string, parameters: Record<string, string> = {}): Worker {
  const query = new URLSearchParams(parameters).toString();
  const worker = new Worker(`${distUrl(context, `${name}.js`)}${query === '' ? '' : `?${query}`}`);

  serveWasm(worker, distUrl(context, `${name}.wasm`));

  return worker;
}

export function createWorkerLauncher(
  context: ExtensionContext,
  name: string,
  options: WorkerHostOptions = {}
): Launcher {
  return () => {
    const host = new WorkerHost(startWorker(context, name), {}, options);

    return {
      transport: host.transport,
      stop: () => host.close()
    };
  };
}
