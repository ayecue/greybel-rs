import * as vscode from 'vscode';

import { waitFor } from './workspace';

export const ENTRY = 'project/entry.src';

export const OUTPUT = 'build';

export async function editorWith(content: string): Promise<vscode.TextEditor> {
  const document = await vscode.workspace.openTextDocument({ language: 'greyscript', content });

  return vscode.window.showTextDocument(document);
}

export async function waitForFiles(folder: vscode.Uri, count: number): Promise<string[]> {
  const read = () =>
    vscode.workspace.fs.readDirectory(folder).then(
      (found) => found.map(([name]) => name),
      () => [] as string[]
    );
  let entries: string[] = [];

  await waitFor(
    `${count} files in ${vscode.workspace.asRelativePath(folder)}`,
    async () => (entries = await read()).length >= count,
    () => `it holds ${entries.length === 0 ? 'nothing' : entries.join(', ')}`
  );

  return entries;
}
