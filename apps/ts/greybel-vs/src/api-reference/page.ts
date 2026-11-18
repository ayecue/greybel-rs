import { Uri, type Webview } from 'vscode';

const SCRIPT = 'dist/api-reference.js';
const WASM = 'dist/api-reference.wasm';
const MERMAID_ORIGIN = 'https://cdn.jsdelivr.net';

export function createApiReferencePage(webview: Webview, extensionUri: Uri): string {
  const script = webview.asWebviewUri(Uri.joinPath(extensionUri, SCRIPT));
  const wasm = webview.asWebviewUri(Uri.joinPath(extensionUri, WASM));
  const policy = [
    "default-src 'none'",
    `script-src ${webview.cspSource} 'wasm-unsafe-eval' ${MERMAID_ORIGIN}`,
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
    <title>GreyScript API</title>
    <style>
      body { margin: 0; padding: 0; }
    </style>
  </head>
  <body>
    <div id="reference"></div>
    <script src="${script}" data-wasm="${wasm}"></script>
  </body>
</html>`;
}
