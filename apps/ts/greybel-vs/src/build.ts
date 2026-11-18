import vscode, { ExtensionContext, Uri } from 'vscode';

import { BuildFailureReporter } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import { IngameDirectoryPicker } from './helper/get-ingame-directory';
import { Watcher } from './helper/watcher';
import type { Launcher } from './launcher';
import type { ProjectReader, Project } from './project';
import { Configuration } from './settings';

export class BuildCommand {
  readonly #projects: ProjectReader;
  readonly #tasks: BuilderTaskRunner;
  readonly #errors = new BuildFailureReporter();

  constructor(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
    this.#projects = projects;
    this.#tasks = new BuilderTaskRunner(launcher);

    context.subscriptions.push(
      vscode.commands.registerCommand('greybel.buildFileFromContext', (uri?: Uri) => this.#buildFileFromContext(uri)),
      vscode.commands.registerCommand('greybel.buildRootFile', (uri?: Uri) => this.#buildRootFile(uri)),
      new Watcher(
        () => this.#buildRootFile(),
        (uri) => this.#isSource(uri)
      ).start()
    );
  }

  async #build(target: Uri): Promise<void> {
    const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === target.toString());

    if (document?.isDirty) await document.save();
    await vscode.workspace.saveAll(false);

    const config = new Configuration();
    const deploy = config.get('createIngame.active');

    try {
      const project = await this.#projects.read(target);
      const picker = new IngameDirectoryPicker(config);
      const ingameDirectory = (await picker.get(project.manifest.build.ingameDirectory)).path;
      const result = await this.#tasks.build(
        project.createBuild([target], ingameDirectory, deploy ? config.get('createIngame.port') : undefined)
      );
      const output = project.getOutput(target);

      try {
        await vscode.workspace.fs.delete(output, { recursive: true });
      } catch {}

      await vscode.workspace.fs.createDirectory(output);

      const encoder = new TextEncoder();
      const files = [...result.files, ...result.installer];

      await Promise.all(
        files.map((file) => vscode.workspace.fs.writeFile(Uri.joinPath(output, file.path), encoder.encode(file.code)))
      );

      const importNote = deploy ? ` and imported into Grey Hack at ${ingameDirectory}` : '';

      vscode.window.showInformationMessage(`Build complete${importNote}. [Open output](${output.toString()}).`);
    } catch (error) {
      this.#errors.show(error);
    }
  }

  async #buildRootFile(sourceUri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri): Promise<void> {
    if (!sourceUri) {
      vscode.window.showErrorMessage("Open a file, or set greybel.json's entries, to build.");

      return;
    }

    const [entry] = (await this.#projects.read(sourceUri)).getEntries(sourceUri);

    await this.#build(entry ?? sourceUri);
  }

  /** Returns whether changing `uri` should rebuild the project entry: it must be a source file outside the build output. */
  async #isSource(uri: Uri): Promise<boolean> {
    const project = await this.#projects.read(uri);

    return this.#isProjectSource(project, uri);
  }

  #isProjectSource(project: Project, uri: Uri): boolean {
    return (
      project.manifest.entries.length > 0 &&
      project.manifest.extensions.some((extension) => uri.path.endsWith(`.${extension}`)) &&
      !uri.path.startsWith(`${project.getOutput(uri).path}/`)
    );
  }

  async #buildFileFromContext(
    sourceUri: Uri | undefined = vscode.window.activeTextEditor?.document?.uri
  ): Promise<void> {
    if (!sourceUri) {
      vscode.window.showErrorMessage('Open a file to build.');

      return;
    }

    await this.#build(sourceUri);
  }
}
