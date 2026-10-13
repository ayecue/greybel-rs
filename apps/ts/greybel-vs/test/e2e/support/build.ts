import * as vscode from 'vscode';

export const ENTRY = 'project/entry.src';

export const OUTPUT = 'build';

export async function editorWith(content: string): Promise<vscode.TextEditor> {
  const document = await vscode.workspace.openTextDocument({ language: 'greyscript', content });

  return vscode.window.showTextDocument(document);
}

export async function waitForFiles(folder: vscode.Uri, count: number): Promise<string[]> {
  for (;;) {
    const entries = await vscode.workspace.fs.readDirectory(folder).then(
      (found) => found.map(([name]) => name),
      () => [] as string[]
    );

    if (entries.length >= count) return entries;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}
