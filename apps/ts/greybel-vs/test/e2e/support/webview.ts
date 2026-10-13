import * as vscode from 'vscode';

const EXTENSION = 'ayecue.greybel-vs';

export function extensionUnderTest(): vscode.Extension<unknown> {
  const extension = vscode.extensions.getExtension(EXTENSION);

  if (!extension) {
    throw new Error(`the ${EXTENSION} extension is not installed`);
  }

  return extension;
}

function webviewTab(label: string): vscode.Tab | undefined {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .find((tab) => tab.input instanceof vscode.TabInputWebview && tab.label === label);
}

export function webviewTabNamed(label: string): Promise<vscode.Tab> {
  return new Promise((resolve) => {
    const open = webviewTab(label);

    if (open) {
      resolve(open);

      return;
    }

    const listener = vscode.window.tabGroups.onDidChangeTabs(() => {
      const tab = webviewTab(label);

      if (!tab) return;
      listener.dispose();
      resolve(tab);
    });
  });
}
