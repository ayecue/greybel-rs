import { Transform } from 'greybel-builder';
import vscode, { ExtensionContext, Range, TextEditor } from 'vscode';

import type { Launcher } from './launcher';
import { showBuildFailure } from './builder/error-message';
import { BuilderTaskRunner } from './builder/task-runner';
import { transpileSettingsOf, type BuildMode, type ProjectReader } from './project';

enum TransformOutput {
  Editor,
  Clipboard
}

export function activate(context: ExtensionContext, launcher: Launcher, projects: ProjectReader) {
  const tasks = new BuilderTaskRunner(launcher);

  async function transform(editor: TextEditor, mode: BuildMode | undefined, output: TransformOutput) {
    let code: string;

    try {
      const project = await projects.of(editor.document.uri);

      code = await tasks.transform(
        new Transform({
          code: editor.document.getText(),
          transpile: transpileSettingsOf(project, mode)
        })
      );
    } catch (error) {
      showBuildFailure(error);

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
    vscode.window.showInformationMessage('Transform done.');
  }

  context.subscriptions.push(
    vscode.commands.registerTextEditorCommand('greybel.transform.clipboard', (editor) =>
      transform(editor, undefined, TransformOutput.Clipboard)
    ),
    vscode.commands.registerTextEditorCommand('greybel.transform.write', (editor) =>
      transform(editor, undefined, TransformOutput.Editor)
    ),
    vscode.commands.registerTextEditorCommand('greybel.uglify.write', (editor) =>
      transform(editor, 'Uglify', TransformOutput.Editor)
    ),
    vscode.commands.registerTextEditorCommand('greybel.beautify.write', (editor) =>
      transform(editor, 'Beautify', TransformOutput.Editor)
    )
  );
}
