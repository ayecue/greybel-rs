import vscode, { ConfigurationTarget, ExtensionContext, StatusBarAlignment, Uri } from 'vscode';

import { ProjectReader, profileOf } from '.';

const BASE = 'No profile';

/**
 * Status bar item for the active GreyScript document's build/run profile. `Greybel: Select profile` selects a
 * project profile for its workspace folder.
 */
export function activate(context: ExtensionContext, projects: ProjectReader): void {
  const item = vscode.window.createStatusBarItem('greybel.profile', StatusBarAlignment.Left);

  item.name = 'Greybel profile';
  item.command = 'greybel.selectProfile';
  item.tooltip = 'The greybel.json profile used by builds and runs';

  const show = () => {
    const document = vscode.window.activeTextEditor?.document;

    if (document?.languageId !== 'greyscript') {
      item.hide();

      return;
    }

    item.text = `$(versions) ${profileOf(document.uri) ?? BASE}`;
    item.show();
  };

  context.subscriptions.push(
    item,
    vscode.window.onDidChangeActiveTextEditor(show),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration('greybel.profile')) return;
      show();
    }),
    vscode.commands.registerCommand('greybel.selectProfile', async (uri?: Uri) => {
      const document = uri ?? vscode.window.activeTextEditor?.document.uri;

      if (document === undefined) return;

      const profiles = await projects.profilesOf(document);

      if (profiles.length === 0) {
        void vscode.window.showInformationMessage("The project's greybel.json has no profiles.");

        return;
      }

      const choice = await vscode.window.showQuickPick([BASE, ...profiles], {
        placeHolder: profileOf(document) ?? BASE,
        title: 'Profile used by builds and runs'
      });

      if (choice === undefined) return;

      await select(document, choice === BASE ? undefined : choice);
    })
  );

  show();
}

async function select(document: Uri, profile: string | undefined): Promise<void> {
  const folder = vscode.workspace.getWorkspaceFolder(document);
  const configuration = vscode.workspace.getConfiguration('greybel', document);

  await configuration.update(
    'profile',
    profile,
    folder === undefined ? ConfigurationTarget.Global : ConfigurationTarget.WorkspaceFolder
  );
}
