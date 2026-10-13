import { openApiReference } from 'greybel-api-reference-web';

import { ExtensionTarget, vscodeApi } from '../webview/extension-target';
import { pageWasm } from '../webview/wasm';

interface PageState {
  readonly excludedTags?: readonly string[];
}

const vscode = vscodeApi();
const state = (vscode.getState() ?? {}) as PageState;

openApiReference(document.getElementById('reference')!, {
  wasm: pageWasm(),
  editor: new ExtensionTarget(),
  documentationUrl: process.env.GREYBEL_DOCUMENTATION_URL,
  excludedTags: state.excludedTags,
  onExcludedTagsChange: (excludedTags) => vscode.setState({ excludedTags } satisfies PageState)
});
