import { AutoCompile, Build, TranspileMode, TranspileSettings } from 'greybel-builder';
import { Environment, Launch } from 'greybel-runtime';
import vscode, { Uri } from 'vscode';

import type { BuildMode, Project } from './manifest';
import type { ProjectSettings } from './settings';

export type { BuildMode, Manifest, Project } from './manifest';

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

  of(uri: Uri): Promise<Project> {
    return this.#request(uri, profileOf(uri));
  }

  async profilesOf(uri: Uri): Promise<string[]> {
    const project = await this.#request(uri, undefined);

    return Object.keys(project.manifest.profiles ?? {});
  }

  async #request(uri: Uri, profile: string | undefined): Promise<Project> {
    const server = await this.#server;

    return server.sendRequest<Project>('custom/project', {
      uri: uri.toString(),
      fallback: this.#settings.manifestOf(uri),
      profile
    });
  }
}

export function profileOf(uri: Uri): string | undefined {
  const profile = vscode.workspace.getConfiguration('greybel', uri).get<string>('profile') ?? '';

  return profile === '' ? undefined : profile;
}

export function folderOf(project: Project, document: Uri): Uri {
  if (project.root !== null) return Uri.parse(project.root);

  return vscode.workspace.getWorkspaceFolder(document)?.uri ?? Uri.joinPath(document, '..');
}

export function entriesOf(project: Project, document: Uri): Uri[] {
  const folder = folderOf(project, document);

  return project.manifest.entries.map((entry) => Uri.joinPath(folder, entry));
}

export function transpileSettingsOf(
  project: Project,
  mode: BuildMode = project.manifest.build.mode
): TranspileSettings {
  const { build, format, strictMode } = project.manifest;

  return new TranspileSettings({
    mode: transpileModeOf(project, mode),
    obfuscation: build.obfuscation,
    strictMode,
    variables: { ...project.buildVariables },
    excludedNames: [...build.excludedNamespaces]
  });

  function transpileModeOf({ manifest }: Project, mode: BuildMode): TranspileMode {
    switch (mode) {
      case 'Uglify':
        return new TranspileMode.Uglify({
          disableLiteralsOptimization: manifest.build.disableLiteralsOptimization,
          disableNamespacesOptimization: manifest.build.disableNamespacesOptimization
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
}

/**
 * Builds `entries` in `project` from `ingameDirectory` and deploys through message-hook port `deployPort`.
 * Resolves absolute imports from the project folder, or the workspace folder without `greybel.json`.
 */
export function buildOf(
  project: Project,
  entries: readonly Uri[],
  ingameDirectory: string,
  deployPort?: number
): Build {
  const { build, extensions } = project.manifest;

  return new Build({
    entries: entries.map((entry) => entry.toString()),
    root: entries[0] === undefined ? undefined : folderOf(project, entries[0]).toString(),
    extensions: [...extensions],
    transpile: transpileSettingsOf(project),
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
 * Runs `entry` in `project` using `environment` and `params`. Resolves absolute imports from the project
 * folder, or the workspace folder without `greybel.json`.
 */
export function launchOf(
  project: Project,
  entry: Uri,
  environment: Environment,
  params: readonly string[],
  options: { readonly debug: boolean; readonly stopOnEntry: boolean }
): Launch {
  const { extensions, strictMode, run } = project.manifest;

  return new Launch({
    entry: entry.toString(),
    root: folderOf(project, entry).toString(),
    extensions: [...extensions],
    environment,
    debug: options.debug,
    stopOnEntry: options.stopOnEntry,
    params: [...params],
    variables: { ...project.runVariables },
    strictMode,
    programName: run.programName
  });
}

export function environmentOf(project: Project, port?: number): Environment {
  const { run } = project.manifest;

  return port === undefined
    ? new Environment.Mock({ seed: run.seed, memoryLimit: run.memoryLimit })
    : new Environment.Game({ port });
}
