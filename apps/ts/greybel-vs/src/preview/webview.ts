import { openTerminal } from 'greyhack-terminal-web';

import { ExtensionTarget } from '../webview/extension-target';
import { pageWasm } from '../webview/wasm';

const canvas = document.getElementById('terminal') as HTMLCanvasElement;

try {
  openTerminal(canvas, new ExtensionTarget(), pageWasm());
} catch (error) {
  canvas.remove();
  document.body.style.color = '#00f0dd';
  document.body.style.font = '14px monospace';
  document.body.textContent = `The preview cannot draw the terminal: ${error instanceof Error ? error.message : String(error)}`;
}
