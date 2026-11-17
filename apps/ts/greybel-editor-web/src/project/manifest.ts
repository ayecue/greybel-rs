/** The parts of greybel-project's `Manifest` the page uses, as `custom/project` returns it. */
export interface Manifest {
  readonly extensions: readonly string[];
  readonly strictMode: boolean;
  /** Features not listed are on. */
  readonly syntax: Readonly<Record<string, boolean>>;
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

export type BuildMode = 'Default' | 'Uglify' | 'Beautify';

export interface BuildSettings {
  readonly mode: BuildMode;
  readonly obfuscation: boolean;
  readonly disableLiteralsOptimization: boolean;
  readonly disableNamespacesOptimization: boolean;
  readonly excludedNamespaces: readonly string[];
  readonly multilineStrings: 'Concatenation' | 'Join';
}

export interface RunSettings {
  readonly seed: string;
  readonly params: readonly string[];
  readonly programName: string;
  readonly memoryLimit: number;
}

/** greybel-lsp's `custom/project` result. */
export interface ResolvedSettingsJson {
  readonly manifest: Manifest;
  readonly buildVariables: Readonly<Record<string, string>>;
  readonly runVariables: Readonly<Record<string, string>>;
}

/** The settings of a file's `greybel.json` with a profile applied. */
export class ResolvedSettings {
  readonly manifest: Manifest;
  readonly buildVariables: Readonly<Record<string, string>>;
  readonly runVariables: Readonly<Record<string, string>>;

  constructor({ manifest, buildVariables, runVariables }: ResolvedSettingsJson) {
    this.manifest = manifest;
    this.buildVariables = buildVariables;
    this.runVariables = runVariables;
  }

  /** Undefined when every feature is on. */
  get disabledSyntax(): string[] | undefined {
    const names = Object.entries(this.manifest.syntax)
      .filter(([, isEnabled]) => !isEnabled)
      .map(([name]) => name);

    return names.length === 0 ? undefined : names;
  }
}

/** Also applies while the project has no `greybel.json`. */
export const MANIFEST_TEMPLATE = `{
  "run": {
    "seed": "test",
    "params": []
  },
  "build": {
    "mode": "Uglify"
  },
  "profiles": {}
}
`;
