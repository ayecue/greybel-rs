import vscode from 'vscode';

const SET_UP = 'Set up Grey Hack';

/** Offers `Greybel: Set up Grey Hack` with errors about an unreachable game, where the command exists. */
export class GameSetupOffer {
  static readonly COMMAND = 'greybel.setupGame';

  /** Shows `message` with a button that opens the setup command, or alone in VS Code for the Web. */
  static async show(message: string): Promise<void> {
    const commands = await vscode.commands.getCommands(true);

    if (!commands.includes(GameSetupOffer.COMMAND)) {
      vscode.window.showErrorMessage(message);

      return;
    }

    if ((await vscode.window.showErrorMessage(message, SET_UP)) !== SET_UP) return;

    await vscode.commands.executeCommand(GameSetupOffer.COMMAND);
  }
}
