import vscode, { Uri } from 'vscode';

export interface ErrorLocation {
  readonly uri: string;
  readonly line: number;
  readonly character: number;
}

export function describeLocation(location: ErrorLocation): string {
  const uri = Uri.parse(location.uri);

  return `${uri.scheme === 'file' ? uri.fsPath : uri.toString()}:${location.line}:${location.character}`;
}

export function showError(message: string, location?: ErrorLocation): void {
  if (!location) {
    vscode.window.showErrorMessage(message);

    return;
  }

  vscode.window
    .showErrorMessage(`${message} at ${describeLocation(location)}`, 'Go to error')
    .then(async (selection) => {
      if (selection !== 'Go to error') return;

      const position = new vscode.Position(location.line - 1, Math.max(location.character - 1, 0));
      const document = await vscode.workspace.openTextDocument(Uri.parse(location.uri));

      vscode.window.showTextDocument(document, { selection: new vscode.Range(position, position) });
    });
}
