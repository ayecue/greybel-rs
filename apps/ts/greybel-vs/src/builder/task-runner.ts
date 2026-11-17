import { EditorChannel as DocumentsChannel } from 'greybel-documents-host-provider';
import {
  type Build,
  type BuildResult,
  EditorChannel,
  EditorRouter,
  type Import,
  type Transform
} from 'greybel-builder';

import type { Launcher } from '../launcher';
import { WorkspaceDocumentProvider } from '../workspace-document-provider';

class SilentEditor extends EditorRouter {}

export class BuilderTaskRunner {
  readonly #launcher: Launcher;

  constructor(launcher: Launcher) {
    this.#launcher = launcher;
  }

  build(message: Build): Promise<BuildResult> {
    return this.#run((builder) => builder.send(message));
  }

  import(message: Import): Promise<number> {
    return this.#run((builder) => builder.send(message));
  }

  transform(message: Transform): Promise<string> {
    return this.#run((builder) => builder.send(message));
  }

  async #run<T>(task: (builder: EditorChannel) => Promise<T>): Promise<T> {
    const peer = this.#launcher.launch();
    const builder = new EditorChannel(peer.transport).use(new SilentEditor());

    new DocumentsChannel(peer.transport).use(new WorkspaceDocumentProvider());
    try {
      return await task(builder);
    } finally {
      builder.close();
      peer.stop();
    }
  }
}
