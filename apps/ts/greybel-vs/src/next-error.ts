import vscode, { DiagnosticSeverity, ExtensionContext, TextEditor } from 'vscode';

export function activate(context: ExtensionContext) {
  function goToNextError(editor: TextEditor) {
    const errors = vscode.languages
      .getDiagnostics(editor.document.uri)
      .filter((diagnostic) => diagnostic.severity === DiagnosticSeverity.Error)
      .sort((a, b) => a.range.start.compareTo(b.range.start));

    if (errors.length === 0) {
      vscode.window.showInformationMessage('No errors found.');

      return;
    }

    const cursor = editor.selection.start;
    const next = errors.find((error) => error.range.start.isAfter(cursor)) ?? errors[0];

    editor.selection = new vscode.Selection(next.range.start, next.range.end);
    editor.revealRange(next.range);
    vscode.window.showErrorMessage(next.message);
  }

  context.subscriptions.push(vscode.commands.registerTextEditorCommand('greybel.gotoError', goToNextError));
}
