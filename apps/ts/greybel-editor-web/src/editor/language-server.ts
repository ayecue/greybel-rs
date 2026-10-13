import { LSPClient, languageServerExtensions, type Transport } from '@codemirror/lsp-client';
import { WorkerHost } from 'duplex-driver-worker';
import { ClientChannel, ClientMessage, ClientRouter, type ServerMessage } from 'greybel-lsp-web';
import serverScript from 'greybel-lsp-web/worker.js?url';
import serverWasm from 'greybel-lsp-web/greybel-lsp-web.wasm?url';
import picomatch from 'picomatch';
import { serveWasm } from 'wasm-host/worker';

import type { Project } from '../project.js';
import type { FileChange } from '../project/event.js';
import { filePath, fileUri, rootUri } from '../project/paths.js';

/** LSP `FileChangeType` values for file creation and deletion. */
const FILE_CHANGE_TYPES: Readonly<Record<FileChange['kind'], number>> = { creation: 1, deletion: 3 };

interface JsonRpcMessage {
  readonly id?: number | string;
  readonly method?: string;
  readonly params?: unknown;
}

/** File content returned to the language server. */
interface FileQuery {
  readonly uri: string;
}

/** File URIs returned to the language server. */
interface FilePattern {
  readonly include: string;
  readonly exclude?: string;
}

/**
 * Adapts a greybel-lsp-web channel to CodeMirror's language-client transport, carrying one JSON-RPC message
 * per channel message. Answers server requests for files outside the editor from the project.
 */
class ServerTransport extends ClientRouter implements Transport {
  readonly #channel: ClientChannel;
  readonly #project: Project;
  readonly #handlers = new Set<(message: string) => void>();

  constructor(channel: ClientChannel, project: Project) {
    super();
    this.#channel = channel.use(this);
    this.#project = project;
  }

  send(message: string): void {
    this.#channel.send(new ClientMessage({ json: message }));
  }

  subscribe(handler: (message: string) => void): void {
    this.#handlers.add(handler);
  }

  unsubscribe(handler: (message: string) => void): void {
    this.#handlers.delete(handler);
  }

  override onServerMessage(message: ServerMessage): void {
    const request = JSON.parse(message.json) as JsonRpcMessage;

    if (request.id !== undefined && request.method === 'custom/read-file') {
      this.#respond(request.id, this.#readFile(request.params as FileQuery));

      return;
    }

    if (request.id !== undefined && request.method === 'custom/find-files') {
      this.#respond(request.id, this.#findFiles(request.params as FilePattern));

      return;
    }

    for (const handler of this.#handlers) handler(message.json);
  }

  #respond(id: number | string, result: unknown): void {
    this.send(JSON.stringify({ jsonrpc: '2.0', id, result }));
  }

  #readFile({ uri }: FileQuery): string | null {
    const path = filePath(uri);

    return (path === undefined ? undefined : this.#project.read(path)) ?? null;
  }

  #findFiles({ include, exclude }: FilePattern): string[] {
    const isIncluded = picomatch(include, { dot: true });
    const isExcluded = exclude === undefined ? () => false : picomatch(exclude, { dot: true });

    return this.#project.paths.filter((path) => isIncluded(path) && !isExcluded(path)).map(fileUri);
  }
}

export interface LanguageServer {
  readonly client: LSPClient;

  close(): void;
}

/**
 * Starts greybel-lsp-web in a worker for `project` and returns the client used by editor plugins. Reports
 * project file creation and deletion as watched-file changes.
 */
export function startLanguageServer(project: Project): LanguageServer {
  const worker = new Worker(serverScript);

  serveWasm(worker, serverWasm);

  const host = new WorkerHost(worker, { server: { channelClass: ClientChannel } });
  const client = new LSPClient({
    rootUri,
    initializationOptions: { dialect: 'greyscript' },
    extensions: languageServerExtensions()
  }).connect(new ServerTransport(host.channels.server, project));
  const unsubscribe = project.subscribe((event) => {
    if (event.kind !== 'files') return;
    client.notification('workspace/didChangeWatchedFiles', {
      changes: event.changes.map(({ path, kind }) => ({ uri: fileUri(path), type: FILE_CHANGE_TYPES[kind] }))
    });
  });

  return {
    client,
    close: () => {
      unsubscribe();
      client.disconnect();
      host.close();
    }
  };
}
