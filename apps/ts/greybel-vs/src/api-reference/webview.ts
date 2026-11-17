import { openApiReference } from 'greybel-api-reference-web';

import { ExtensionApi, ExtensionTarget } from '../webview/extension-target';
import { loadPageWasm } from '../webview/wasm';

interface PageState {
  readonly excludedTags?: readonly string[];
}

const vscode = new ExtensionApi();
const state = (vscode.getState() ?? {}) as PageState;

openApiReference(document.getElementById('reference')!, {
  wasm: loadPageWasm(),
  editor: new ExtensionTarget(vscode),
  documentationUrl: process.env.GREYBEL_DOCUMENTATION_URL,
  excludedTags: state.excludedTags,
  onExcludedTagsChange: (excludedTags) => vscode.setState({ excludedTags } satisfies PageState)
});
