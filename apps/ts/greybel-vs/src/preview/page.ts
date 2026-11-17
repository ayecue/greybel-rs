import { Uri, type Webview } from 'vscode';

const SCRIPT = 'dist/preview.js';
const WASM = 'dist/preview.wasm';

export function createPreviewPage(webview: Webview, extensionUri: Uri): string {
  const script = webview.asWebviewUri(Uri.joinPath(extensionUri, SCRIPT));
  const wasm = webview.asWebviewUri(Uri.joinPath(extensionUri, WASM));
  const policy = [
    "default-src 'none'",
    `script-src ${webview.cspSource} 'wasm-unsafe-eval'`,
    `connect-src ${webview.cspSource}`,

    'worker-src blob:',
    "style-src 'unsafe-inline'"
  ].join('; ');

  return `<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="Content-Security-Policy" content="${policy}">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Grey Hack Output Preview</title>
    <style>
      html, body { margin: 0; padding: 0; height: 100%; overflow: hidden; background: #04081a; }
      #terminal { display: block; width: 100vw; height: 100vh; }
    </style>
  </head>
  <body>
    <canvas id="terminal"></canvas>
    <script src="${script}" data-wasm="${wasm}"></script>
  </body>
</html>`;
}
