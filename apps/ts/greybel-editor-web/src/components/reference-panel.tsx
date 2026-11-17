import { MessageTargetDriver } from 'duplex-driver-worker';
import { openApiReference } from 'greybel-api-reference-web';
import { EditorChannel, EditorRouter, type OpenDocument } from 'greybel-api-reference-web/editor';
import referenceWasm from 'greybel-api-reference-web/greybel-api-reference-web.wasm?url';
import { useEffect, useRef } from 'preact/hooks';

import type { ProjectEditor } from '../editor/view.js';
import { useServices } from './services.js';

/** Inserts examples opened in the reference at the selection. */
class ExampleRouter extends EditorRouter {
  constructor(private readonly editor: ProjectEditor) {
    super();
  }

  override onOpenDocument(message: OpenDocument): void {
    this.editor.insertCode(message.content);
  }
}

export interface ReferencePanelConfiguration {
  readonly editor: ProjectEditor;
}

export function ReferencePanel({ editor }: ReferencePanelConfiguration) {
  const rootRef = useRef<HTMLDivElement>(null);
  const { wasm } = useServices();

  useEffect(() => {
    const root = rootRef.current;

    if (root === null) return;

    const closeReference = wasm.load('API reference', referenceWasm).then((module) => {
      const { port1, port2 } = new MessageChannel();
      const examples = new EditorChannel(new MessageTargetDriver(port1)).use(new ExampleRouter(editor));
      const reference = openApiReference(root, {
        wasm: module,
        editor: port2,
        documentationUrl: 'https://documentation.greyscript.org'
      });

      return () => {
        reference.close();
        examples.close();
      };
    });

    return () => void closeReference.then((close) => close());
  }, [editor]);

  return <aside class="reference-panel" ref={rootRef} />;
}
