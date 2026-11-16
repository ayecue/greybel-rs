import vscode, { ExtensionContext, Uri } from 'vscode';

import { postJson } from './helper/request';

export function activate(context: ExtensionContext) {
  async function share(documentUri: Uri = vscode.window.activeTextEditor?.document?.uri) {
    try {
      const document = await vscode.workspace.openTextDocument(documentUri.toString());
      const content = document.getText();
      const response = await postJson<{ id: string }>(`${process.env.EDITOR_SERVICE_URL}/code`, { content });
      const link = `${process.env.GREYBEL_EDITOR_URL}?id=${response.id}`;

      vscode.env.clipboard.writeText(link);
      vscode.window
        .showInformationMessage('Editor link created and copied to the clipboard.', { modal: false }, 'Open link')
        .then(async () => {
          const uri = Uri.parse(link);

          vscode.env.openExternal(uri);
        });
    } catch (error: any) {
      vscode.window.showErrorMessage(
        `Couldn't share the file: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  context.subscriptions.push(vscode.commands.registerCommand('greybel.share', share));
}
