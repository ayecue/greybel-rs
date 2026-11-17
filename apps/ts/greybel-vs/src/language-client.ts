import { workspace, ExtensionContext } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';

import { ProcessLauncher } from './launcher/process';
import { SECTION } from './settings';

export class LanguageServer {
  readonly #context: ExtensionContext;
  #client: LanguageClient | undefined;

  /** The client stops when the extension deactivates. */
  constructor(context: ExtensionContext) {
    this.#context = context;
    context.subscriptions.push({ dispose: () => void this.#client?.stop() });
  }

  /** Starts the language server and resolves with its client when ready. */
  async start(): Promise<LanguageClient> {
    const serverOptions: ServerOptions = {
      command: new ProcessLauncher(this.#context, 'greybel-lsp').getBinaryPath(),
      transport: TransportKind.stdio
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: 'file', language: 'greyscript' }],
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

    this.#client = new LanguageClient(
      'greyscript-language-server',
      'GreyScript Language Server',
      serverOptions,
      clientOptions
    );

    this.#client.registerProposedFeatures();
    await this.#client.start();

    return this.#client;
  }
}
