import { LSPClient, languageServerExtensions } from '@codemirror/lsp-client';
import { WorkerHost } from 'duplex-driver-worker';
import { ClientChannel } from 'greybel-lsp-web';
import serverScript from 'greybel-lsp-web/worker.js?url';
import serverWasm from 'greybel-lsp-web/greybel-lsp-web.wasm?url';

import type { Project } from '../project.js';
import type { FileChange } from '../project/event.js';
import type { WasmLoader } from '../wasm.js';
import { ServerTransport } from './server-transport.js';
import type { TooltipHighlighter } from './tooltip/highlighter.js';

const FILE_CHANGE_TYPES: Readonly<Record<FileChange['kind'], number>> = { creation: 1, deletion: 3 };

/** greybel-lsp-web in a worker. Sends file creations and deletions as `workspace/didChangeWatchedFiles`. */
export class LanguageServer {
  readonly client: LSPClient;

  readonly #host: Promise<WorkerHost<{ server: ClientChannel }>>;
  readonly #unsubscribe: () => void;

  #isClosed = false;

  constructor(project: Project, wasm: WasmLoader, highlighter: TooltipHighlighter) {
    this.client = new LSPClient({
      rootUri: project.paths.rootUri,
      initializationOptions: { dialect: 'greyscript' },
      extensions: languageServerExtensions(),
      highlightLanguage: (name) => highlighter.findLanguage(name)
    });
    this.#host = wasm
      .startWorker('language server', serverScript, serverWasm)
      .then((worker) => new WorkerHost(worker, { server: { channelClass: ClientChannel } }));
    wasm.track('language server', this.client.initializing);
    void this.#host.then((host) => {
      if (this.#isClosed) return;
      this.client.connect(new ServerTransport(host.channels.server, project));
    });
    this.#unsubscribe = project.subscribe((event) => {
      if (event.kind !== 'files') return;
      this.client.notification('workspace/didChangeWatchedFiles', {
        changes: event.changes.map(({ path, kind }) => ({
          uri: project.paths.toUri(path),
          type: FILE_CHANGE_TYPES[kind]
        }))
      });
    });
  }

  close(): void {
    this.#isClosed = true;
    this.#unsubscribe();
    this.client.disconnect();
    void this.#host.then((host) => host.close());
  }
}
