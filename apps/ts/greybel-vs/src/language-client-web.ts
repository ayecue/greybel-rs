import { WorkerHost } from 'duplex-driver-worker';
import { ClientChannel, LanguageServerConnection } from 'greybel-lsp-web';
import { workspace, ExtensionContext, Uri } from 'vscode';
import { BaseLanguageClient, LanguageClientOptions, MessageTransports } from 'vscode-languageclient/lib/browser/main';

import { globalFileSystem } from './helper/fs';
import { WorkerLauncher } from './launcher/worker';
import { SECTION } from './settings';

class WorkerLanguageClient extends BaseLanguageClient {
  readonly #startWorker: () => Worker;

  constructor(id: string, name: string, clientOptions: LanguageClientOptions, startWorker: () => Worker) {
    super(id, name, clientOptions);
    this.#startWorker = startWorker;
  }

  protected async createMessageTransports(_encoding: string): Promise<MessageTransports> {
    const host = new WorkerHost(this.#startWorker(), { server: { channelClass: ClientChannel } });

    void host.channels.server.closeEvent.then(() => host.close());

    const { reader, writer } = new LanguageServerConnection(host.channels.server);

    return { reader, writer };
  }
}

export class LanguageServer {
  readonly #context: ExtensionContext;
  #client: BaseLanguageClient | undefined;

  /** The client stops when the extension deactivates. */
  constructor(context: ExtensionContext) {
    this.#context = context;
    context.subscriptions.push({ dispose: () => void this.#client?.stop() });
  }

  /** Starts the language server and resolves with its client when ready. */
  async start(): Promise<BaseLanguageClient> {
    this.#client = this.#createClient();

    this.#client.registerProposedFeatures();
    await this.#client.start();

    return this.#client;
  }

  #createClient(): BaseLanguageClient {
    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ language: 'greyscript' }],
      synchronize: {
        configurationSection: SECTION,
        fileEvents: workspace.createFileSystemWatcher('**/*')
      },
      diagnosticCollectionName: 'greyscript',
      // The server validates all documents in this dialect (greybel-lsp's InitializationOptions).
      initializationOptions: { dialect: 'greyscript' },
      middleware: {
        provideDocumentSemanticTokens: () => {
          return undefined;
        },
        provideFoldingRanges: () => {
          return undefined;
        }
      }
    };
    const client = new WorkerLanguageClient(
      'greyscript-language-server',
      'GreyScript Language Server',
      clientOptions,
      () => new WorkerLauncher(this.#context, 'lsp-web').start()
    );

    client.onRequest('custom/read-file', async ({ uri }: { uri: string }) => {
      return await globalFileSystem.readText(Uri.parse(uri), true);
    });

    client.onRequest('custom/find-files', async ({ include, exclude }: { include: string; exclude?: string }) => {
      const fileUris = await workspace.findFiles(include, exclude);

      return fileUris.map((uri) => uri.toString());
    });

    return client;
  }
}
