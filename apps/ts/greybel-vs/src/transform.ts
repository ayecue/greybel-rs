import { Transform } from 'greybel-builder';
import vscode, { ExtensionContext, Range, TextEditor } from 'vscode';

import { BuildFailureReporter } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import type { Launcher } from './launcher';
import type { BuildMode, ProjectReader } from './project';

enum TransformOutput {
  Editor,
  Clipboard
}

export class TransformCommand {
  readonly #projects: ProjectReader;
  readonly #tasks: BuilderTaskRunner;
  readonly #errors = new BuildFailureReporter();

  constructor(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
    this.#projects = projects;
    this.#tasks = new BuilderTaskRunner(launcher);

    context.subscriptions.push(
      vscode.commands.registerTextEditorCommand('greybel.transform.clipboard', (editor) =>
        this.#transform(editor, undefined, TransformOutput.Clipboard)
      ),
      vscode.commands.registerTextEditorCommand('greybel.transform.write', (editor) =>
        this.#transform(editor, undefined, TransformOutput.Editor)
      ),
      vscode.commands.registerTextEditorCommand('greybel.uglify.write', (editor) =>
        this.#transform(editor, 'Uglify', TransformOutput.Editor)
      ),
      vscode.commands.registerTextEditorCommand('greybel.beautify.write', (editor) =>
        this.#transform(editor, 'Beautify', TransformOutput.Editor)
      )
    );
  }

  async #transform(editor: TextEditor, mode: BuildMode | undefined, output: TransformOutput): Promise<void> {
    let code: string;

    try {
      const project = await this.#projects.read(editor.document.uri);

      code = await this.#tasks.transform(
        new Transform({
          code: editor.document.getText(),
          transpile: project.getTranspileSettings(mode)
        })
      );
    } catch (error) {
      this.#errors.show(error);

      return;
    }

    if (output === TransformOutput.Clipboard) {
      await vscode.env.clipboard.writeText(code);
      vscode.window.showInformationMessage('Transformed code copied to the clipboard.');

      return;
    }

    const document = editor.document;
    const entireRange = new Range(document.lineAt(0).range.start, document.lineAt(document.lineCount - 1).range.end);

    await editor.edit((edit) => edit.replace(entireRange, code));
    vscode.window.showInformationMessage('Transform complete.');
  }
}
