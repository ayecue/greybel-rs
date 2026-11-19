import { ChannelError } from 'duplex-core';
import { BuildFailure } from 'greybel-builder';
import vscode from 'vscode';

import { ErrorReporter } from '../runtime/error-message';
import { GameSetupOffer } from '../setup/offer';

export class BuildFailureReporter {
  readonly #errors = new ErrorReporter();

  show(error: unknown): void {
    const failure = error instanceof ChannelError ? error.failure : undefined;

    if (failure instanceof BuildFailure.Invalid) {
      this.#errors.show(failure.message, failure.location ?? undefined);
    } else if (failure instanceof BuildFailure.Unreachable) {
      void GameSetupOffer.show(failure.message);
    } else if (failure instanceof BuildFailure.Unavailable) {
      vscode.window.showErrorMessage('Grey Hack import is not available here.');
    } else {
      vscode.window.showErrorMessage(error instanceof Error ? error.message : String(error));
    }
  }
}
