import vscode, { Uri } from 'vscode';

import type { Configuration } from '../settings';

/**
 * Game destination folder for a build or import; prompts when configured to prompt, otherwise uses
 * `directory`.
 */
export const getIngameDirectory = async (config: Configuration, directory: string): Promise<Uri> => {
  if (config.get('transpiler.ingameDirectoryPrompt')) {
    const customDirectory = await vscode.window.showInputBox({
      title: 'Enter the in-game directory',
      value: directory,
      ignoreFocusOut: true
    });

    if (customDirectory != null && customDirectory !== '') return Uri.file(customDirectory);
  }

  return Uri.file(directory);
};
