import { ChannelError } from 'duplex-core';
import { BuildFailure } from 'greybel-builder';
import vscode from 'vscode';

import { showError } from '../runtime/error-message';

export function showBuildFailure(error: unknown): void {
  const failure = error instanceof ChannelError ? error.failure : undefined;

  if (failure instanceof BuildFailure.Invalid) {
    showError(failure.message, failure.location ?? undefined);
  } else if (failure instanceof BuildFailure.Unreachable) {
    vscode.window.showErrorMessage(failure.message);
  } else if (failure instanceof BuildFailure.Unavailable) {
    vscode.window.showErrorMessage("Importing into Grey Hack isn't available here.");
  } else {
    vscode.window.showErrorMessage(error instanceof Error ? error.message : String(error));
  }
}
