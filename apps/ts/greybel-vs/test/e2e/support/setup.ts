import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { dirname, join } from 'path';
import * as vscode from 'vscode';

/** A macOS Grey Hack folder with the given files, which `GREY_HACK_PATH` names until it is removed. */
export class FakeGame {
  readonly folder = mkdtempSync(join(tmpdir(), 'greybel-vs-setup-'));
  readonly #previous = process.env.GREY_HACK_PATH;

  constructor(files: readonly string[]) {
    for (const file of ['Grey Hack.app/Contents/Resources/Data/Managed/.keep', ...files]) {
      mkdirSync(dirname(this.path(file)), { recursive: true });
      writeFileSync(this.path(file), '');
    }

    process.env.GREY_HACK_PATH = this.folder;
  }

  path(file: string): string {
    return join(this.folder, file);
  }

  remove(): void {
    if (this.#previous === undefined) {
      delete process.env.GREY_HACK_PATH;
    } else {
      process.env.GREY_HACK_PATH = this.#previous;
    }

    rmSync(this.folder, { recursive: true, force: true });
  }
}

type Window = typeof vscode.window;

/**
 * Replaces the quick pick, the modal review and the information message of `vscode.window`: picks the item labelled
 * `pick`, responds `review` to the review, and records what they show.
 */
export class WindowStub {
  placeHolder: string | undefined;
  readonly reviews: string[] = [];
  readonly information: string[] = [];
  readonly #window = vscode.window as { -readonly [K in keyof Window]: Window[K] };
  readonly #original = {
    showQuickPick: vscode.window.showQuickPick,
    showWarningMessage: vscode.window.showWarningMessage,
    showInformationMessage: vscode.window.showInformationMessage
  };

  constructor(pick: string, review: string) {
    this.#window.showQuickPick = (async (
      items: readonly vscode.QuickPickItem[] | Thenable<readonly vscode.QuickPickItem[]>,
      options?: vscode.QuickPickOptions
    ) => {
      this.placeHolder = options?.placeHolder;

      return (await items).find((item) => item.label === pick);
    }) as Window['showQuickPick'];
    this.#window.showWarningMessage = (async (_message: string, options: vscode.MessageOptions) => {
      this.reviews.push(options.detail ?? '');

      return review;
    }) as Window['showWarningMessage'];
    this.#window.showInformationMessage = (async (message: string) => {
      this.information.push(message);

      return undefined;
    }) as Window['showInformationMessage'];
  }

  restore(): void {
    Object.assign(this.#window, this.#original);
  }
}
