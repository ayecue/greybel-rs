import { EditorChannel as DocumentsChannel } from 'greybel-documents-host-provider';
import { PageChannel } from 'greybel-io-web';
import { EditorChannel, type EditorRouter } from 'greybel-runtime';
import { TerminalChannel } from 'greyhack-terminal-text';

import type { Peer } from '../launcher';
import { TerminalPage } from './terminal-page';
import { WorkspaceDocumentProvider } from '../workspace-document-provider';

export class RuntimeConnection {
  readonly runtime: EditorChannel;
  readonly page: TerminalPage;
  readonly #peer: Peer;

  constructor(peer: Peer, editor: EditorRouter) {
    this.#peer = peer;
    this.page = new TerminalPage(new TerminalChannel(peer.transport));
    this.runtime = new EditorChannel(peer.transport).use(editor);
    new PageChannel(peer.transport).use(this.page);
    new DocumentsChannel(peer.transport).use(new WorkspaceDocumentProvider());
  }

  close(): void {
    this.page.end();
    this.runtime.close();
    this.#peer.stop();
  }
}
