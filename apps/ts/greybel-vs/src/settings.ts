import vscode from 'vscode';

/** Extension configuration section. */
export const SECTION = 'greybel';

/**
 * User extension settings keyed as declared by `contributes.configuration` in package.json; VS Code supplies
 * declared defaults. Project settings come from `./project`, using `greybel.json` and values synchronized by
 * `ProjectSettings`.
 */
export interface Settings {
  formatter: boolean;
  autocomplete: boolean;
  hoverdocs: boolean;
  diagnostic: boolean;
  'interpreter.environmentType': 'Mock' | 'In-Game';
  'interpreter.port': number;
  'interpreter.useDefaultArgs': boolean;
  'interpreter.silenceErrorPopups': boolean;
  'transpiler.watch': boolean;
  'transpiler.ingameDirectoryPrompt': boolean;
  'createIngame.port': number;
  'createIngame.active': boolean;
}

/** Current extension settings. */
export class Configuration {
  readonly #configuration = vscode.workspace.getConfiguration(SECTION);

  get<K extends keyof Settings>(key: K): Settings[K] {
    return this.#configuration.get(key) as Settings[K];
  }
}
