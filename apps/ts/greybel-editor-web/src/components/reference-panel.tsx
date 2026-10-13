import { MessageTargetDriver } from 'duplex-driver-worker';
import { openApiReference } from 'greybel-api-reference-web';
import { EditorChannel, EditorRouter, type OpenDocument } from 'greybel-api-reference-web/editor';
import referenceWasm from 'greybel-api-reference-web/greybel-api-reference-web.wasm?url';
import { useEffect, useRef } from 'preact/hooks';

import type { ProjectEditor } from '../editor/view.js';

/** Inserts the examples the reference opens into the file shown, at its selection. */
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

/** The GreyScript API reference, whose examples go into the file shown. */
export function ReferencePanel({ editor }: ReferencePanelConfiguration) {
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = rootRef.current;

    if (root === null) return;

    const { port1, port2 } = new MessageChannel();
    const examples = new EditorChannel(new MessageTargetDriver(port1)).use(new ExampleRouter(editor));
    const reference = openApiReference(root, {
      wasm: referenceWasm,
      editor: port2,
      documentationUrl: 'https://documentation.greyscript.org'
    });

    return () => {
      reference.close();
      examples.close();
    };
  }, [editor]);

  return <aside class="reference-panel" ref={rootRef} />;
}
