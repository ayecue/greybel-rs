import vscode, { ExtensionContext, Uri, workspace } from 'vscode';

import type { RuntimeLauncher, WorldHost } from '../launcher';
import type { Preview } from '../preview';
import type { ProjectReader } from '../project';
import { Configuration } from '../settings';
import { RuntimeDebugSession, toUri } from './session';

export class DebugController {
  readonly #projects: ProjectReader;

  constructor(
    context: ExtensionContext,
    launcher: RuntimeLauncher,
    world: WorldHost<unknown>,
    projects: ProjectReader,
    preview: Preview
  ) {
    this.#projects = projects;

    context.subscriptions.push(
      vscode.commands.registerCommand('greybel.debug.runFileFromContext', (uri?: Uri) => this.#runFileFromContext(uri)),
      vscode.commands.registerCommand('greybel.debug.debugFileFromContext', (uri?: Uri) =>
        this.#debugFileFromContext(uri)
      ),
      vscode.commands.registerCommand('greybel.debug.runRootFile', (uri?: Uri) => this.#runRootFile(uri)),
      vscode.commands.registerCommand('greybel.debug.debugRootFile', (uri?: Uri) => this.#debugRootFile(uri)),
      vscode.commands.registerCommand('greybel.debug.getProgramName', () => this.#getProgramName()),
      vscode.debug.registerDebugAdapterDescriptorFactory(
        'greyscript',
        new InlineDebugAdapterFactory(launcher, projects, preview)
      ),
      vscode.commands.registerCommand('greybel.resetWorld', () => world.reset()),
      { dispose: () => world.reset() }
    );
  }

  #runFile(sourceUri: Uri | undefined, isDebug: boolean): Thenable<boolean> {
    if (!sourceUri) {
      vscode.window.showErrorMessage('Open a file to run.');

      return Promise.resolve(false);
    }

    return vscode.debug.startDebugging(
      workspace.getWorkspaceFolder(sourceUri),
      {
        type: 'greyscript',
        name: 'Run File',
        request: 'launch',

        program: sourceUri.fsPath
      },
      { noDebug: !isDebug }
    );
  }

  #runFileFromContext(sourceUri: Uri = vscode.window.activeTextEditor.document.uri): Thenable<boolean> {
    return this.#runFile(sourceUri, false);
  }

  #debugFileFromContext(sourceUri: Uri = vscode.window.activeTextEditor.document.uri): Thenable<boolean> {
    return this.#runFile(sourceUri, true);
  }

  /** Returns the project entry URI, or `sourceUri` when the project has no entry. */
  async #getEntry(sourceUri: Uri | undefined): Promise<Uri | undefined> {
    if (!sourceUri) return undefined;

    return (await this.#projects.read(sourceUri)).getEntries(sourceUri)[0] ?? sourceUri;
  }

  async #runRootFile(sourceUri: Uri = vscode.window.activeTextEditor?.document.uri): Promise<boolean> {
    return this.#runFile(await this.#getEntry(sourceUri), false);
  }

  async #debugRootFile(sourceUri: Uri = vscode.window.activeTextEditor?.document.uri): Promise<boolean> {
    return this.#runFile(await this.#getEntry(sourceUri), true);
  }

  async #getProgramName(): Promise<string | undefined> {
    const target = vscode.window.activeTextEditor?.document.uri.toString();

    if (target) return target;

    const rootPath = vscode.workspace.rootPath;

    if (!rootPath) {
      const value = await vscode.window.showInputBox({
        placeHolder: 'Enter the full file path',
        value: 'test.src'
      });

      if (!value) return;

      return value;
    }

    const value = await vscode.window.showInputBox({
      placeHolder: 'Enter the name of a .src, .gs, or .ms file in the workspace folder',
      value: 'test.src'
    });

    if (!value) return;

    return Uri.joinPath(Uri.file(rootPath), value).toString();
  }
}

class InlineDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
  readonly #launcher: RuntimeLauncher;
  readonly #projects: ProjectReader;
  readonly #preview: Preview;

  constructor(launcher: RuntimeLauncher, projects: ProjectReader, preview: Preview) {
    this.#launcher = launcher;
    this.#projects = projects;
    this.#preview = preview;
  }

  async createDebugAdapterDescriptor(session: vscode.DebugSession): Promise<vscode.DebugAdapterDescriptor> {
    const program: string | undefined = session.configuration.program;
    const document = program === undefined ? session.workspaceFolder?.uri : toUri(program);
    let seed: string | undefined;

    if (new Configuration().get('interpreter.environmentType') === 'Mock' && document !== undefined) {
      seed = (await this.#projects.read(document)).manifest.run.seed;
    }

    return new vscode.DebugAdapterInlineImplementation(
      new RuntimeDebugSession(await this.#launcher.launch(seed), this.#projects, this.#preview)
    );
  }
}
