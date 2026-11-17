import { Import } from 'greybel-builder';
import vscode, { ExtensionContext, Uri } from 'vscode';

import { BuildFailureReporter } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import { IngameDirectoryPicker } from './helper/get-ingame-directory';
import type { Launcher } from './launcher';
import type { ProjectReader } from './project';
import { Configuration } from './settings';

export class ImportCommand {
  readonly #projects: ProjectReader;
  readonly #tasks: BuilderTaskRunner;
  readonly #errors = new BuildFailureReporter();

  constructor(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
    this.#projects = projects;
    this.#tasks = new BuilderTaskRunner(launcher);

    context.subscriptions.push(vscode.commands.registerCommand('greybel.import', (uri?: Uri) => this.#import(uri)));
  }

  async #import(uri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri): Promise<void> {
    if (!uri) {
      vscode.window.showErrorMessage('Choose a file or a folder to import.');

      return;
    }

    const { root, files } = await this.#collectFiles(uri);

    if (files.length === 0) {
      vscode.window.showInformationMessage('No files found to import.');

      return;
    }

    const config = new Configuration();

    try {
      const project = await this.#projects.read(files[0]!);
      const picker = new IngameDirectoryPicker(config);
      const count = await this.#tasks.import(
        new Import({
          root: root.toString(),
          files: files.map((file) => file.toString()),
          ingameDirectory: (await picker.get(project.manifest.build.ingameDirectory)).path,
          port: config.get('createIngame.port')
        })
      );

      vscode.window.showInformationMessage(`Imported ${count} ${count === 1 ? 'file' : 'files'} into Grey Hack.`);
    } catch (error) {
      this.#errors.show(error);
    }
  }

  /** Returns files imported from `uri` and the root for their game-relative paths. */
  async #collectFiles(uri: Uri): Promise<{ root: Uri; files: Uri[] }> {
    const stat = await vscode.workspace.fs.stat(uri);

    if (stat.type === vscode.FileType.Directory) {
      return { root: uri, files: await vscode.workspace.findFiles(new vscode.RelativePattern(uri, '**/*')) };
    }

    return { root: Uri.joinPath(uri, '..'), files: stat.type === vscode.FileType.File ? [uri] : [] };
  }
}
