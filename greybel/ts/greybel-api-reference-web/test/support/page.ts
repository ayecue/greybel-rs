import { MessageTargetDriver } from 'duplex-driver-worker';

import type { ApiReference } from '../../src/index.js';
import { EditorChannel, EditorRouter, type OpenDocument } from '../../src/workbench-editor.generated.js';

const openedDocuments: OpenDocument[] = [];

let onOpen: (() => void) | undefined;

let reference: ApiReference | undefined;

class RecordingEditor extends EditorRouter {
  override onOpenDocument(message: OpenDocument): void {
    openedDocuments.push(message);
    onOpen?.();
  }
}

/** Opens the reference, its WebAssembly the bytes `wasm` encodes in base64. */
export async function open(wasm: string): Promise<void> {
  const { port1, port2 } = new MessageChannel();

  new EditorChannel(new MessageTargetDriver(port2)).use(new RecordingEditor());
  reference = greybelApiReferenceWeb.openApiReference(document.getElementById('reference')!, {
    wasm: Uint8Array.from(atob(wasm), (character) => character.charCodeAt(0)),
    editor: port1
  });
  await reference.ready;
}

export async function openedDocument(): Promise<{ content: string }> {
  if (openedDocuments.length === 0) await new Promise<void>((resolve) => (onOpen = resolve));

  const { content } = openedDocuments.at(-1)!;

  return { content };
}
