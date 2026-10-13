import vscode, { ExtensionContext, Uri } from 'vscode';

import { getIngameDirectory } from './helper/get-ingame-directory';
import { Watcher } from './helper/watcher';
import type { Launcher } from './launcher';
import { showBuildFailure } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import { buildOf, entriesOf, folderOf, type Project, type ProjectReader } from './project';
import { Configuration } from './settings';

/** Output folder for `project` builds. */
function outputOf(project: Project, document: Uri): Uri {
  return Uri.joinPath(folderOf(project, document), project.manifest.build.output);
}

export function activate(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
  const tasks = new BuilderTaskRunner(launcher);

  async function build(target: Uri) {
    const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === target.toString());

    if (document?.isDirty) await document.save();
    await vscode.workspace.saveAll(false);

    const config = new Configuration();
    const deploy = config.get('createIngame.active');

    try {
      const project = await projects.of(target);
      const ingameDirectory = (await getIngameDirectory(config, project.manifest.build.ingameDirectory)).path;
      const result = await tasks.build(
        buildOf(project, [target], ingameDirectory, deploy ? config.get('createIngame.port') : undefined)
      );
      const output = outputOf(project, target);

      try {
        await vscode.workspace.fs.delete(output, { recursive: true });
      } catch {}

      await vscode.workspace.fs.createDirectory(output);

      const encoder = new TextEncoder();
      const files = [...result.files, ...result.installer];

      await Promise.all(
        files.map((file) => vscode.workspace.fs.writeFile(Uri.joinPath(output, file.path), encoder.encode(file.code)))
      );

      const importNote = deploy ? ' and imported into Grey Hack' : '';

      vscode.window.showInformationMessage(`Build done${importNote}. Available [here](${output.toString()}).`);
    } catch (error) {
      showBuildFailure(error);
    }
  }

  async function buildRootFile(sourceUri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri) {
    if (!sourceUri) {
      vscode.window.showErrorMessage("Open a file, or set greybel.json's entries, to build.");

      return;
    }

    const [entry] = entriesOf(await projects.of(sourceUri), sourceUri);

    await build(entry ?? sourceUri);
  }

  /** Return whether changing `uri` should rebuild the project entry: it must be a source file outside the build output. */
  async function isSource(uri: Uri): Promise<boolean> {
    const project = await projects.of(uri);

    return (
      project.manifest.entries.length > 0 &&
      project.manifest.extensions.some((extension) => uri.path.endsWith(`.${extension}`)) &&
      !uri.path.startsWith(`${outputOf(project, uri).path}/`)
    );
  }

  async function buildFileFromContext(sourceUri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri) {
    if (!sourceUri) {
      vscode.window.showErrorMessage('Open a file to build.');

      return;
    }

    await build(sourceUri);
  }

  context.subscriptions.push(
    vscode.commands.registerCommand('greybel.buildFileFromContext', buildFileFromContext),
    vscode.commands.registerCommand('greybel.buildRootFile', buildRootFile),
    new Watcher(buildRootFile, isSource).start()
  );
}
