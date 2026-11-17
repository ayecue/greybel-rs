import vscode, { ExtensionContext } from 'vscode';

import { SNIPPETS } from './snippets.generated';

export class SnippetCommand {
  constructor(context: ExtensionContext) {
    context.subscriptions.push(vscode.commands.registerCommand('greybel.snippets', () => this.#open()));
  }

  async #open(): Promise<void> {
    const names = Object.keys(SNIPPETS);
    const snippetName = await vscode.window.showQuickPick(names);

    if (snippetName && snippetName in SNIPPETS) {
      const document = await vscode.workspace.openTextDocument({
        language: 'greyscript',
        content: SNIPPETS[snippetName]
      });

      vscode.window.showTextDocument(document);

      return;
    }

    vscode.window.showErrorMessage(`Cannot find snippet. Available snippets: ${names.join(', ')}`);
  }
}
