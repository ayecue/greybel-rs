import type { Transport } from '@codemirror/lsp-client';
import { ClientMessage, ClientRouter, type ClientChannel, type ServerMessage } from 'greybel-lsp-web';
import picomatch from 'picomatch';

import type { Project } from '../project.js';

interface JsonRpcMessage {
  readonly id?: number | string;
  readonly method?: string;
  readonly params?: unknown;
}

interface FileQuery {
  readonly uri: string;
}

interface FilePattern {
  readonly include: string;
  readonly exclude?: string;
}

/** Answers greybel-lsp's file read and search requests from the project. */
export class ServerTransport extends ClientRouter implements Transport {
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
    const path = this.#project.paths.toPath(uri);

    return (path === undefined ? undefined : this.#project.read(path)) ?? null;
  }

  #findFiles({ include, exclude }: FilePattern): string[] {
    const isIncluded = picomatch(include, { dot: true });
    const isExcluded = exclude === undefined ? () => false : picomatch(exclude, { dot: true });

    return this.#project.filePaths
      .filter((path) => isIncluded(path) && !isExcluded(path))
      .map((path) => this.#project.paths.toUri(path));
  }
}
