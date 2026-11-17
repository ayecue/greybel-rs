import vscode, { Uri } from 'vscode';

import type { Configuration } from '../settings';

/** Resolves game destination folders for builds and imports. */
export class IngameDirectoryPicker {
  readonly #config: Configuration;

  constructor(config: Configuration) {
    this.#config = config;
  }

  /** Prompts when configured to prompt, otherwise uses `directory`. */
  async get(directory: string): Promise<Uri> {
    if (this.#config.get('transpiler.ingameDirectoryPrompt')) {
      const customDirectory = await vscode.window.showInputBox({
        title: 'Enter the in-game directory',
        value: directory,
        ignoreFocusOut: true
      });

      if (customDirectory != null && customDirectory !== '') return Uri.file(customDirectory);
    }

    return Uri.file(directory);
  }
}
