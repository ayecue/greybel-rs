import vscode, { ExtensionContext, QuickPickItem } from 'vscode';

import { Configuration } from '../settings';
import { GameSetupOffer } from './offer';
import { SetupTool, type StatusReport } from './tool';

/** One choice of the setup command: its greyhack-setup steps and what they download or change, for the review. */
interface SetupChoice extends QuickPickItem {
  readonly steps: readonly (readonly string[])[];
  readonly changes: readonly string[];
  readonly chooseFolder?: boolean;
}

const COPY_LAUNCH_OPTION = 'Copy launch option';

/**
 * `Greybel: Set up Grey Hack`: shows the game's setup, offers BepInEx, the message hook and its port, reviews the
 * downloads and changes, then runs them with the extension's greyhack-setup.
 */
export class GameSetupCommand {
  readonly #tool: SetupTool;
  #folder: string | undefined;

  constructor(context: ExtensionContext, binary: string) {
    this.#tool = new SetupTool(binary);

    context.subscriptions.push(vscode.commands.registerCommand(GameSetupOffer.COMMAND, () => this.#run()));
  }

  async #run(): Promise<void> {
    let report: StatusReport;

    try {
      report = await this.#tool.status(this.#folder);
    } catch (error) {
      if (await this.#chooseFolder(String(error instanceof Error ? error.message : error))) await this.#run();

      return;
    }

    const choice = await vscode.window.showQuickPick(this.#createChoices(report), {
      title: `Grey Hack in ${report.status.folder}`,
      placeHolder: `${this.#describe(report)}: ${report.readinessText}`,
      ignoreFocusOut: true
    });

    if (choice === undefined) return;

    if (choice.chooseFolder === true) {
      if (await this.#chooseFolder()) await this.#run();

      return;
    }

    const review = await vscode.window.showWarningMessage(
      'Apply these changes to Grey Hack?',
      { modal: true, detail: choice.changes.map((change) => `• ${change}`).join('\n') },
      'Apply'
    );

    if (review !== 'Apply') return;

    try {
      const result = await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title: 'Setting up Grey Hack' },
        async (progress) => {
          let last = report;

          for (const step of choice.steps) {
            last = await this.#tool.run(step, report.status.folder, (message) => progress.report({ message }));
          }

          return last;
        }
      );

      await this.#showResult(result);
    } catch (error) {
      vscode.window.showErrorMessage(
        `Couldn't set up Grey Hack: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /** The choices for the game's setup, the one that completes it first. */
  #createChoices(report: StatusReport): SetupChoice[] {
    const { status } = report;
    const port = new Configuration().get('createIngame.port');
    const stable = report.bepinexReleases.find((release) => release.major === 5);
    const beta = report.bepinexReleases.find((release) => release.major === 6);
    const hookChange = `Download the message hook of ${report.hookRelease} from GitHub and install it`;
    const portStep = ['configure', '--port', String(port)];
    const choices: SetupChoice[] = [];

    if (status.readiness.kind !== 'ready' || (status.port ?? 7777) !== port) {
      const steps: string[][] = [];
      const changes: string[] = [];

      if (status.bepinex === null && stable !== undefined) {
        steps.push(['bepinex', 'install', '--bepinex', '5']);
        changes.push(`Download BepInEx ${stable.version} from ${stable.url} and install it`);
      }

      if (status.readiness.kind !== 'ready') {
        steps.push(['hook', 'install', ...(status.bepinex === null ? ['--bepinex', '5'] : [])]);
        changes.push(hookChange);
      }

      steps.push(portStep);
      changes.push(`Set the port to ${port} (the setting greybel.createIngame.port)`);
      choices.push({
        label: 'Complete the setup',
        description: 'recommended',
        detail: changes.join(', '),
        steps,
        changes
      });
    }

    for (const release of [stable, beta]) {
      if (release === undefined) continue;

      const name = `BepInEx ${release.major} (${release.version}${release.major === 5 ? ', stable' : ', beta'})`;

      choices.push({
        label: `Install ${name}`,
        detail:
          status.bepinex === null ? undefined : 'Replaces the installed BepInEx; plugins and settings stay unchanged',
        steps: [['bepinex', 'install', '--bepinex', String(release.major)]],
        changes: [`Download ${name} from ${release.url} and install it`]
      });
    }

    if (status.bepinex !== null) {
      choices.push({
        label: 'Install the message hook',
        detail: `For BepInEx ${status.bepinex.major}, from ${report.hookRelease}`,
        steps: [['hook', 'install']],
        changes: [hookChange]
      });
      choices.push({
        label: 'Remove BepInEx',
        detail: 'Plugins and settings stay unchanged',
        steps: [['bepinex', 'remove']],
        changes: ['Remove BepInEx']
      });
    }

    if (status.hook !== null) {
      choices.push({
        label: 'Remove the message hook',
        steps: [['hook', 'remove']],
        changes: ['Remove the message hook']
      });
    }

    if ((status.port ?? 7777) !== port) {
      choices.push({ label: `Set the port to ${port}`, steps: [portStep], changes: [`Set the port to ${port}`] });
    }

    if (status.duplicateSteamApi !== null) {
      choices.push({
        label: 'Remove the duplicate libsteam_api.dylib',
        detail: 'It crashes Grey Hack under BepInEx, because Steam\u2019s library is then found twice',
        steps: [['remove-duplicate-steam-api']],
        changes: [`Remove ${status.duplicateSteamApi}`]
      });
    }

    choices.push({ label: 'Choose another game folder…', steps: [], changes: [], chooseFolder: true });

    return choices;
  }

  #describe(report: StatusReport): string {
    const { bepinex, hook, port } = report.status;

    return [
      bepinex === null
        ? 'no BepInEx'
        : `BepInEx ${bepinex.major}${bepinex.version === '' ? '' : ` (${bepinex.version})`}`,
      hook ?? 'no message hook',
      `port ${port ?? 7777}`
    ].join(', ');
  }

  async #chooseFolder(problem?: string): Promise<boolean> {
    if (problem !== undefined) {
      const choice = await vscode.window.showErrorMessage(problem, 'Choose folder…');

      if (choice === undefined) return false;
    }

    const folders = await vscode.window.showOpenDialog({
      canSelectFiles: false,
      canSelectFolders: true,
      title: 'Choose the Grey Hack folder'
    });

    if (folders === undefined || folders.length === 0) return false;

    this.#folder = folders[0]!.fsPath;

    return true;
  }

  async #showResult(report: StatusReport): Promise<void> {
    const launchOption = report.status.launchOption;

    if (launchOption === null) {
      vscode.window.showInformationMessage(report.readinessText);

      return;
    }

    const choice = await vscode.window.showInformationMessage(
      `${report.readinessText} Start the game from Steam with the launch option ${launchOption}`,
      COPY_LAUNCH_OPTION
    );

    if (choice !== COPY_LAUNCH_OPTION) return;

    await vscode.env.clipboard.writeText(launchOption);
  }
}
