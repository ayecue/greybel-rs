import { AutoCompile, Build, MultilineStringStyle, TranspileMode, TranspileSettings } from 'greybel-builder';
import { Environment, Launch, MultilineStringStyle as LaunchedStyle, RunUser } from 'greybel-runtime';
import vscode, { Uri } from 'vscode';

import type { BuildMode, Manifest, ProjectJson, TerminalLineLimits } from './manifest';
import type { ProjectSettings } from './settings';

export type { BuildMode, Manifest, TerminalLineLimits } from './manifest';

export interface ProjectServer {
  sendRequest<R>(method: string, params: unknown): Promise<R>;
}

export class ProjectReader {
  readonly #server: Promise<ProjectServer>;
  readonly #settings: ProjectSettings;

  constructor(server: Promise<ProjectServer>, settings: ProjectSettings) {
    this.#server = server;
    this.#settings = settings;
  }

  async read(uri: Uri): Promise<Project> {
    return new Project(await this.#request(uri, this.getProfile(uri)));
  }

  getProfile(uri: Uri): string | undefined {
    const profile = vscode.workspace.getConfiguration('greybel', uri).get<string>('profile') ?? '';

    return profile === '' ? undefined : profile;
  }

  /** The files `uri` imports, directly or through other imported files, in import order. */
  async getImportedFiles(uri: Uri): Promise<Uri[]> {
    const server = await this.#server;
    const uris = await server.sendRequest<string[]>('custom/imported-files', { uri: uri.toString() });

    return uris.map((value) => Uri.parse(value));
  }

  async getProfiles(uri: Uri): Promise<string[]> {
    const project = await this.#request(uri, undefined);

    return Object.keys(project.manifest.profiles ?? {});
  }

  async #request(uri: Uri, profile: string | undefined): Promise<ProjectJson> {
    const server = await this.#server;

    return server.sendRequest<ProjectJson>('custom/project', {
      uri: uri.toString(),
      fallback: this.#settings.createManifest(uri),
      profile
    });
  }
}

/** A resolved project: its settings, files, and the builder and runtime requests made from them. */
export class Project {
  readonly #project: ProjectJson;

  constructor(project: ProjectJson) {
    this.#project = project;
  }

  get manifest(): Manifest {
    return this.#project.manifest;
  }

  /** The folder of the project's `greybel.json`; undefined without one. */
  get root(): Uri | undefined {
    return this.#project.root === null ? undefined : Uri.parse(this.#project.root);
  }

  getFolder(document: Uri): Uri {
    if (this.#project.root !== null) return Uri.parse(this.#project.root);

    return vscode.workspace.getWorkspaceFolder(document)?.uri ?? Uri.joinPath(document, '..');
  }

  getEntries(document: Uri): Uri[] {
    const folder = this.getFolder(document);

    return this.manifest.entries.map((entry) => Uri.joinPath(folder, entry));
  }

  getOutput(document: Uri): Uri {
    return Uri.joinPath(this.getFolder(document), this.manifest.build.output);
  }

  getTranspileSettings(mode: BuildMode = this.manifest.build.mode): TranspileSettings {
    const { build, format, strictMode } = this.manifest;

    return new TranspileSettings({
      mode: this.#getTranspileMode(mode, format),
      obfuscation: build.obfuscation,
      strictMode,
      variables: { ...this.#project.buildVariables },
      excludedNames: [...build.excludedNamespaces],
      disabledSyntax: this.#getDisabledSyntax(),
      multilineStrings: MultilineStringStyle[build.multilineStrings]
    });
  }

  /**
   * Builds `entries` from `ingameDirectory` and deploys through message-hook port `deployPort`. Resolves absolute
   * imports from the project folder, or the workspace folder without `greybel.json`.
   */
  createBuild(entries: readonly Uri[], ingameDirectory: string, deployPort?: number): Build {
    const { build, extensions } = this.manifest;

    return new Build({
      entries: entries.map((entry) => entry.toString()),
      root: entries[0] === undefined ? undefined : this.getFolder(entries[0]).toString(),
      extensions: [...extensions],
      transpile: this.getTranspileSettings(),
      ingameDirectory,
      outputName: build.outputName ?? undefined,
      autoCompile: build.autoCompile
        ? new AutoCompile({
            allowImport: deployPort === undefined ? build.installer.allowImport : build.deploy.allowImport
          })
        : undefined,
      installerLength: build.installer.active ? build.installer.maxChars : undefined,
      deployPort
    });
  }

  /**
   * Runs `entry` using `environment` and `params`. Resolves absolute imports from the project folder, or the
   * workspace folder without `greybel.json`.
   */
  createLaunch(
    entry: Uri,
    environment: Environment,
    params: readonly string[],
    options: { readonly debug: boolean; readonly stopOnEntry: boolean }
  ): Launch {
    const { build, extensions, strictMode, run } = this.manifest;

    return new Launch({
      entry: entry.toString(),
      root: this.getFolder(entry).toString(),
      extensions: [...extensions],
      environment,
      debug: options.debug,
      stopOnEntry: options.stopOnEntry,
      params: [...params],
      variables: { ...this.#project.runVariables },
      strictMode,
      programName: run.programName,
      disabledSyntax: this.#getDisabledSyntax(),
      multilineStrings: LaunchedStyle[build.multilineStrings]
    });
  }

  /** The terminal form of `run.lineLimits`; `undefined` splits long lines as the game does. */
  getLineLimits(): TerminalLineLimits | undefined {
    const limits = this.manifest.run.lineLimits;

    if (limits === null) return undefined;

    return { textPerLine: limits.textPerLine, tagsPerLine: limits.tagsPerLine ?? undefined };
  }

  getEnvironment(port?: number): Environment {
    const { run } = this.manifest;

    return port === undefined
      ? new Environment.Mock({ seed: run.seed, memoryLimit: run.memoryLimit, user: RunUser[run.user] })
      : new Environment.Game({ port });
  }

  #getTranspileMode(mode: BuildMode, format: Manifest['format']): TranspileMode {
    switch (mode) {
      case 'Uglify':
        return new TranspileMode.Uglify({
          disableLiteralsOptimization: this.manifest.build.disableLiteralsOptimization,
          disableNamespacesOptimization: this.manifest.build.disableNamespacesOptimization
        });
      case 'Beautify':
        return new TranspileMode.Beautify({
          retainParentheses: format.retainParentheses,
          indentationSpaces: format.indentation === 'Whitespace' ? format.indentationSpaces : undefined
        });
      default:
        return new TranspileMode.Default();
    }
  }

  /** The names of the syntax features this project turns off, or `undefined` when every feature is on. */
  #getDisabledSyntax(): string[] | undefined {
    const names = Object.entries(this.manifest.syntax)
      .filter(([, active]) => !active)
      .map(([name]) => name);

    return names.length === 0 ? undefined : names;
  }
}
