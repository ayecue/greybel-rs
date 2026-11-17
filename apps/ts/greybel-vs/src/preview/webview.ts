import { openTerminal } from 'greyhack-terminal-web';

import { ExtensionApi, ExtensionTarget } from '../webview/extension-target';
import { loadPageWasm } from '../webview/wasm';

const canvas = document.getElementById('terminal') as HTMLCanvasElement;

try {
  const page = openTerminal(canvas, new ExtensionTarget(new ExtensionApi()), loadPageWasm());

  window.addEventListener('message', ({ data }: MessageEvent<unknown>) => {
    if (typeof data !== 'object' || data === null || !('lineLimits' in data)) return;

    const limits = data.lineLimits as { textPerLine: number; tagsPerLine?: number } | null;

    page.setLineLimits(limits ?? undefined);
  });
} catch (error) {
  canvas.remove();
  document.body.style.color = '#00f0dd';
  document.body.style.font = '14px monospace';
  document.body.textContent = `Cannot render the terminal preview: ${error instanceof Error ? error.message : String(error)}`;
}
