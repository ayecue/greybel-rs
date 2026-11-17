import vscode, { ConfigurationTarget, ExtensionContext, StatusBarAlignment, Uri } from 'vscode';

import { ProjectReader } from '.';

const BASE = 'No profile';

/** Status bar item for the active GreyScript document's build/run profile. */
export class ProfilePicker {
  readonly #projects: ProjectReader;
  readonly #item = vscode.window.createStatusBarItem('greybel.profile', StatusBarAlignment.Left);

  constructor(context: ExtensionContext, projects: ProjectReader) {
    this.#projects = projects;
    this.#item.name = 'Greybel profile';
    this.#item.command = 'greybel.selectProfile';
    this.#item.tooltip = 'The greybel.json profile used by builds and runs';

    context.subscriptions.push(
      this.#item,
      vscode.window.onDidChangeActiveTextEditor(() => this.#show()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (!event.affectsConfiguration('greybel.profile')) return;
        this.#show();
      }),
      vscode.commands.registerCommand('greybel.selectProfile', (uri?: Uri) => this.#selectProfile(uri))
    );

    this.#show();
  }

  #show(): void {
    const document = vscode.window.activeTextEditor?.document;

    if (document?.languageId !== 'greyscript') {
      this.#item.hide();

      return;
    }

    this.#item.text = `$(versions) ${this.#projects.getProfile(document.uri) ?? BASE}`;
    this.#item.show();
  }

  async #selectProfile(uri?: Uri): Promise<void> {
    const document = uri ?? vscode.window.activeTextEditor?.document.uri;

    if (document === undefined) return;

    const profiles = await this.#projects.getProfiles(document);

    if (profiles.length === 0) {
      void vscode.window.showInformationMessage("The project's greybel.json has no profiles.");

      return;
    }

    const choice = await vscode.window.showQuickPick([BASE, ...profiles], {
      placeHolder: this.#projects.getProfile(document) ?? BASE,
      title: 'Profile used by builds and runs'
    });

    if (choice === undefined) return;

    await this.#select(document, choice === BASE ? undefined : choice);
  }

  async #select(document: Uri, profile: string | undefined): Promise<void> {
    const folder = vscode.workspace.getWorkspaceFolder(document);
    const configuration = vscode.workspace.getConfiguration('greybel', document);

    await configuration.update(
      'profile',
      profile,
      folder === undefined ? ConfigurationTarget.Global : ConfigurationTarget.WorkspaceFolder
    );
  }
}
