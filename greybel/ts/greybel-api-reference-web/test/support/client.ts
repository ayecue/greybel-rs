import { MessageTargetDriver } from 'duplex-driver-worker';

import type { ApiReference } from '../../src/index.js';
import { EditorChannel, EditorRouter, type OpenDocument } from '../../src/workbench-editor.generated.js';

class EditorRecorder extends EditorRouter {
  readonly #record: (message: OpenDocument) => void;

  constructor(record: (message: OpenDocument) => void) {
    super();
    this.#record = record;
  }

  override onOpenDocument(message: OpenDocument): void {
    this.#record(message);
  }
}

class ReferenceClient {
  readonly #documents: OpenDocument[] = [];

  #onOpen: (() => void) | undefined;
  #reference: ApiReference | undefined;

  /** Opens the reference, its WebAssembly the bytes `wasm` encodes in base64. */
  async open(wasm: string, repositoryUrl?: string): Promise<void> {
    const { port1, port2 } = new MessageChannel();

    new EditorChannel(new MessageTargetDriver(port2)).use(new EditorRecorder((message) => this.#record(message)));
    this.#reference = greybelApiReferenceWeb.openApiReference(document.getElementById('reference')!, {
      wasm: Uint8Array.from(atob(wasm), (character) => character.charCodeAt(0)),
      editor: port1,
      repositoryUrl
    });
    await this.#reference.ready;
  }

  async getOpenedDocument(): Promise<{ content: string }> {
    if (this.#documents.length === 0) await new Promise<void>((resolve) => (this.#onOpen = resolve));

    const { content } = this.#documents.at(-1)!;

    return { content };
  }

  #record(message: OpenDocument): void {
    this.#documents.push(message);
    this.#onOpen?.();
  }
}

export const client = new ReferenceClient();
