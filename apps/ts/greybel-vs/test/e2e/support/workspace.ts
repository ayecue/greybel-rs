import * as vscode from 'vscode';

export const TIMEOUT = 60_000;

/** How long `waitFor` waits: below `TIMEOUT`, so a failed wait names what it waited for. */
const WAIT = 20_000;

/** Waits until `condition` holds; fails naming `description` (and `detail`'s text, if given) after 20 s. */
export async function waitFor(
  description: string,
  condition: () => Promise<boolean> | boolean,
  detail?: () => Promise<string> | string
): Promise<void> {
  const end = Date.now() + WAIT;

  while (!(await condition())) {
    if (Date.now() > end) {
      throw new Error(`waited ${WAIT / 1000} s for ${description}${detail ? `; ${await detail()}` : ''}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

export function fixture(path: string): vscode.Uri {
  const [folder] = vscode.workspace.workspaceFolders ?? [];

  if (!folder) {
    throw new Error('the tests run in the fixture workspace');
  }

  return vscode.Uri.joinPath(folder.uri, path);
}

export async function openFixture(path: string): Promise<vscode.TextDocument> {
  const document = await vscode.workspace.openTextDocument(fixture(path));

  await vscode.window.showTextDocument(document);

  return document;
}

export function diagnosticsOf(uri: vscode.Uri): Promise<vscode.Diagnostic[]> {
  return new Promise((resolve) => {
    const current = vscode.languages.getDiagnostics(uri);

    if (current.length > 0) {
      resolve(current);

      return;
    }

    const listener = vscode.languages.onDidChangeDiagnostics(({ uris }) => {
      if (!uris.some((changed) => changed.toString() === uri.toString())) return;
      listener.dispose();
      resolve(vscode.languages.getDiagnostics(uri));
    });
  });
}

export async function hoverAt(document: vscode.TextDocument, position: vscode.Position): Promise<string[]> {
  const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
    'vscode.executeHoverProvider',
    document.uri,
    position
  );

  return hovers.flatMap((hover) =>
    hover.contents.map((content) => (typeof content === 'string' ? content : content.value))
  );
}
