import vscode from 'vscode';
import { EditorRouter, type OpenDocument } from 'greybel-api-reference-web/editor';

export class WorkbenchEditor extends EditorRouter {
  override async onOpenDocument(message: OpenDocument): Promise<void> {
    const document = await vscode.workspace.openTextDocument({ content: message.content, language: 'greyscript' });

    await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.Beside, preview: false });
  }
}
