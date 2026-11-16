/**
 * Complete project `greybel.json` returned by greybel-lsp's `custom/project`: greybel-project's `Manifest`,
 * described by `schemas/greybel.schema.json`.
 */
export interface Manifest {
  readonly entries: readonly string[];
  readonly extensions: readonly string[];
  readonly strictMode: boolean;
  /** Greybel syntax features turned on or off by name; a feature not listed is on. */
  readonly syntax: Readonly<Record<string, boolean>>;
  readonly exclude: string;
  readonly format: Format;
  readonly build: BuildSettings;
  readonly run: RunSettings;
  readonly profiles?: Readonly<Record<string, unknown>>;
}

export interface Format {
  readonly retainParentheses: boolean;
  readonly indentation: 'Tab' | 'Whitespace';
  readonly indentationSpaces: number;
}

export interface BuildSettings {
  readonly output: string;
  readonly outputName: string | null;
  readonly mode: BuildMode;
  readonly obfuscation: boolean;
  readonly disableLiteralsOptimization: boolean;
  readonly disableNamespacesOptimization: boolean;
  readonly excludedNamespaces: readonly string[];
  readonly ingameDirectory: string;
  readonly autoCompile: boolean;
  readonly installer: { readonly active: boolean; readonly maxChars: number; readonly allowImport: boolean };
  readonly deploy: { readonly allowImport: boolean };
  readonly variables: VariableSettings;
}

export type BuildMode = 'Default' | 'Uglify' | 'Beautify';

export interface RunSettings {
  readonly seed: string;
  readonly params: readonly string[];
  readonly programName: string;
  readonly memoryLimit: number;
  readonly variables: VariableSettings;
}

export interface VariableSettings {
  readonly files: readonly string[];
  readonly values: Readonly<Record<string, string>>;
}

export interface Project {
  readonly root: string | null;
  readonly manifest: Manifest;
  readonly buildVariables: Readonly<Record<string, string>>;
  readonly runVariables: Readonly<Record<string, string>>;
}
