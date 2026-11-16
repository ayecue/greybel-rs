import vscode, { ExtensionContext, Uri, workspace } from 'vscode';

import type { RuntimeLauncher, WorldHost } from '../launcher';
import { entriesOf, type ProjectReader } from '../project';
import { Configuration } from '../settings';
import { RuntimeDebugSession, toUri } from './session';

export function activate(
  context: ExtensionContext,
  launcher: RuntimeLauncher,
  world: WorldHost<unknown>,
  projects: ProjectReader
) {
  const runFile = (sourceUri: Uri | undefined, isDebug: boolean): Thenable<boolean> => {
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
  };

  const runFileFromContext = (sourceUri: Uri = vscode.window.activeTextEditor.document.uri): Thenable<boolean> => {
    return runFile(sourceUri, false);
  };

  const debugFileFromContext = (sourceUri: Uri = vscode.window.activeTextEditor.document.uri): Thenable<boolean> => {
    return runFile(sourceUri, true);
  };

  /** Project entry URI, or `sourceUri` when the project has no entry. */
  const entryOf = async (sourceUri: Uri | undefined): Promise<Uri | undefined> => {
    if (!sourceUri) return undefined;

    return entriesOf(await projects.of(sourceUri), sourceUri)[0] ?? sourceUri;
  };

  const runRootFile = async (sourceUri: Uri = vscode.window.activeTextEditor?.document.uri): Promise<boolean> => {
    return runFile(await entryOf(sourceUri), false);
  };

  const debugRootFile = async (sourceUri: Uri = vscode.window.activeTextEditor?.document.uri): Promise<boolean> => {
    return runFile(await entryOf(sourceUri), true);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand('greybel.debug.runFileFromContext', runFileFromContext),
    vscode.commands.registerCommand('greybel.debug.debugFileFromContext', debugFileFromContext),
    vscode.commands.registerCommand('greybel.debug.runRootFile', runRootFile),
    vscode.commands.registerCommand('greybel.debug.debugRootFile', debugRootFile)
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('greybel.debug.getProgramName', async (_config) => {
      const target = vscode.window.activeTextEditor?.document.uri.toString();

      if (target) {
        return target;
      }

      const rootPath = vscode.workspace.rootPath;

      if (!rootPath) {
        const value = await vscode.window.showInputBox({
          placeHolder: 'Enter the full file path',
          value: 'test.src'
        });

        if (!value) {
          return;
        }

        return value;
      }

      const value = await vscode.window.showInputBox({
        placeHolder: 'Enter the name of a .src, .gs, or .ms file in the workspace folder',
        value: 'test.src'
      });

      if (!value) {
        return;
      }

      return Uri.joinPath(Uri.file(rootPath), value).toString();
    })
  );

  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory('greyscript', new InlineDebugAdapterFactory(launcher, projects)),
    vscode.commands.registerCommand('greybel.resetWorld', () => world.reset()),
    { dispose: () => world.reset() }
  );
}

class InlineDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
  constructor(
    private readonly launcher: RuntimeLauncher,
    private readonly projects: ProjectReader
  ) {}

  async createDebugAdapterDescriptor(session: vscode.DebugSession): Promise<vscode.DebugAdapterDescriptor> {
    const program: string | undefined = session.configuration.program;
    const document = program === undefined ? session.workspaceFolder?.uri : toUri(program);
    let seed: string | undefined;

    if (new Configuration().get('interpreter.environmentType') === 'Mock' && document !== undefined) {
      seed = (await this.projects.of(document)).manifest.run.seed;
    }

    return new vscode.DebugAdapterInlineImplementation(
      new RuntimeDebugSession(await this.launcher(seed), this.projects)
    );
  }
}
