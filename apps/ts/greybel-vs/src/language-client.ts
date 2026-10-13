import { workspace, ExtensionContext } from 'vscode';

import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';
import { binaryPath } from './launcher/process';
import { SECTION } from './settings';

let client: LanguageClient;

/** Starts the language server and resolves with its client when ready. */
export async function activate(context: ExtensionContext): Promise<LanguageClient> {
  const serverOptions: ServerOptions = {
    command: binaryPath(context, 'greybel-lsp'),
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

  client = new LanguageClient('greyscript-language-server', 'GreyScript Language Server', serverOptions, clientOptions);

  client.registerProposedFeatures();
  await client.start();

  return client;
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }

  return client.stop();
}
