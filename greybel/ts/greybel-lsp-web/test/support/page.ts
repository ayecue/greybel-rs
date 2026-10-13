import { ErrorCode, type Next } from 'duplex-core';
import { WorkerHost } from 'duplex-driver-worker';
import { serveWasm } from 'wasm-host/worker';
import { createMessageConnection, type MessageConnection } from 'vscode-jsonrpc/browser.js';

import {
  ClientChannel,
  ClientMessage,
  ClientRouter,
  LanguageServerConnection,
  type ServerMessage
} from '../../src/index.js';

export const ROOT = 'file:///project';

export interface Diagnostic {
  readonly code?: string | number;
  readonly message: string;
}

export interface Hover {
  readonly contents: { readonly kind: string; readonly value: string };
}

export interface OpenResult {
  readonly diagnostics: readonly Diagnostic[];
  readonly reads: readonly string[];
}

export type Project = Readonly<Record<string, string>>;

const INITIALIZE_PARAMS = {
  processId: null,
  rootUri: null,
  capabilities: {
    workspace: { configuration: true },
    textDocument: { hover: { contentFormat: ['markdown', 'plaintext'] } }
  },
  workspaceFolders: [{ uri: ROOT, name: 'project' }]
};

const uri = (path: string): string => `${ROOT}/${path}`;

let workerScript: string | undefined;
let workerWasm: Uint8Array | undefined;

/** The worker's script and its WebAssembly, the bytes `wasm` encodes in base64. */
export function setWorker(script: string, wasm: string): void {
  workerScript = script;
  workerWasm = Uint8Array.from(atob(wasm), (character) => character.charCodeAt(0));
}

function serverChannel(): ClientChannel {
  if (workerScript === undefined || workerWasm === undefined) throw new Error('the page has no worker');

  const worker = new Worker(URL.createObjectURL(new Blob([workerScript], { type: 'text/javascript' })));

  serveWasm(worker, workerWasm);

  const host = new WorkerHost(worker, { server: { channelClass: ClientChannel } });

  void host.channels.server.closeEvent.then(() => host.close());

  return host.channels.server;
}

class EditorClient {
  readonly reads: string[] = [];
  readonly server: LanguageServerConnection;
  readonly #rpc: MessageConnection;
  readonly #files: Map<string, string>;
  readonly #diagnosticsResolvers = new Map<string, (diagnostics: Diagnostic[]) => void>();

  constructor(project: Project) {
    this.#files = new Map(Object.entries(project).map(([path, text]): [string, string] => [uri(path), text]));
    this.server = new LanguageServerConnection(serverChannel());
    this.#rpc = createMessageConnection(this.server.reader, this.server.writer);
    this.#rpc.onRequest('custom/read-file', ({ uri: read }: { uri: string }) => {
      this.reads.push(read);

      return this.#files.get(read) ?? null;
    });
    this.#rpc.onRequest('custom/find-files', () => [...this.#files.keys()]);
    this.#rpc.onRequest('workspace/configuration', () => [{}]);
    this.#rpc.onNotification(
      'textDocument/publishDiagnostics',
      (params: { uri: string; diagnostics: Diagnostic[] }) => {
        this.#diagnosticsResolvers.get(params.uri)?.(params.diagnostics);
        this.#diagnosticsResolvers.delete(params.uri);
      }
    );
    this.#rpc.listen();
  }

  async initialize(): Promise<{ serverInfo?: { name: string } }> {
    const result = await this.#rpc.sendRequest<{ serverInfo?: { name: string } }>('initialize', INITIALIZE_PARAMS);

    await this.#rpc.sendNotification('initialized', {});

    return result;
  }

  open(path: string): Promise<Diagnostic[]> {
    const documentUri = uri(path);
    const diagnostics = new Promise<Diagnostic[]>((resolve) => this.#diagnosticsResolvers.set(documentUri, resolve));

    void this.#rpc.sendNotification('textDocument/didOpen', {
      textDocument: { uri: documentUri, languageId: 'greyscript', version: 1, text: this.#files.get(documentUri) }
    });

    return diagnostics;
  }

  hover(path: string, line: number, character: number): Promise<Hover | null> {
    return this.#rpc.sendRequest('textDocument/hover', {
      textDocument: { uri: uri(path) },
      position: { line, character }
    });
  }

  async exit(): Promise<ErrorCode> {
    await this.#rpc.sendRequest('shutdown');
    await this.#rpc.sendNotification('exit');

    return this.server.channel.closeEvent;
  }

  dispose(): void {
    this.#rpc.dispose();
    this.server.close();
  }
}

async function withClient<T>(project: Project, use: (client: EditorClient) => Promise<T>): Promise<T> {
  const client = new EditorClient(project);

  try {
    await client.initialize();

    return await use(client);
  } finally {
    client.dispose();
  }
}

export async function serverName(project: Project): Promise<string | undefined> {
  const client = new EditorClient(project);

  try {
    return (await client.initialize()).serverInfo?.name;
  } finally {
    client.dispose();
  }
}

export function open(project: Project, path: string): Promise<OpenResult> {
  return withClient(project, async (client) => ({ diagnostics: await client.open(path), reads: client.reads }));
}

export function hover(project: Project, path: string, line: number, character: number): Promise<Hover | null> {
  return withClient(project, async (client) => {
    await client.open(path);

    return client.hover(path, line, character);
  });
}

export function exit(project: Project): Promise<string> {
  return withClient(project, async (client) => ErrorCode[await client.exit()]);
}

class FirstMessageRouter extends ClientRouter {
  readonly receivedMessage: Promise<unknown>;
  #receive!: (message: unknown) => void;

  constructor() {
    super();
    this.receivedMessage = new Promise((resolve) => (this.#receive = resolve));
  }

  override onServerMessage(message: ServerMessage, _next: Next<void>): void {
    this.#receive(JSON.parse(message.json));
  }
}

export async function reply(json: string): Promise<unknown> {
  const first = new FirstMessageRouter();
  const channel = serverChannel().use(first);

  try {
    await channel.send(new ClientMessage({ json }));

    return await first.receivedMessage;
  } finally {
    channel.close(ErrorCode.ClosedChannel);
  }
}
