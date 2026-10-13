import { Import } from 'greybel-builder';
import vscode, { ExtensionContext, Uri } from 'vscode';

import { getIngameDirectory } from './helper/get-ingame-directory';
import type { Launcher } from './launcher';
import { showBuildFailure } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import type { ProjectReader } from './project';
import { Configuration } from './settings';

/** Files imported from `uri` and the root for their game-relative paths. */
async function collectFiles(uri: Uri): Promise<{ root: Uri; files: Uri[] }> {
  const stat = await vscode.workspace.fs.stat(uri);

  if (stat.type === vscode.FileType.Directory) {
    return { root: uri, files: await vscode.workspace.findFiles(new vscode.RelativePattern(uri, '**/*')) };
  }

  return { root: Uri.joinPath(uri, '..'), files: stat.type === vscode.FileType.File ? [uri] : [] };
}

export function activate(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
  const tasks = new BuilderTaskRunner(launcher);

  async function importIntoGame(uri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri) {
    if (!uri) {
      vscode.window.showErrorMessage('Choose a file or a folder to import.');

      return;
    }

    const { root, files } = await collectFiles(uri);

    if (files.length === 0) {
      vscode.window.showInformationMessage('No files found to import.');

      return;
    }

    const config = new Configuration();

    try {
      const project = await projects.of(files[0]!);
      const count = await tasks.import(
        new Import({
          root: root.toString(),
          files: files.map((file) => file.toString()),
          ingameDirectory: (await getIngameDirectory(config, project.manifest.build.ingameDirectory)).path,
          port: config.get('createIngame.port')
        })
      );

      vscode.window.showInformationMessage(`Imported ${count} ${count === 1 ? 'file' : 'files'} into Grey Hack.`);
    } catch (error) {
      showBuildFailure(error);
    }
  }

  context.subscriptions.push(vscode.commands.registerCommand('greybel.import', importIntoGame));
}
