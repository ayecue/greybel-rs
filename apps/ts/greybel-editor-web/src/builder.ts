import { WorkerHost } from 'duplex-driver-worker';
import { EditorChannel as DocumentsChannel, MemoryEditor } from 'greybel-documents-host-provider';
import { EditorChannel, EditorRouter, Transform, TranspileMode, TranspileSettings } from 'greybel-builder';
import builderScript from 'greybel-builder-web/dist/index.js?url';
import builderWasm from 'greybel-builder-web/dist/greybel-builder-web.wasm?url';
import { serveWasm } from 'wasm-host/worker';

/** Result of transforming source code. */
export type TransformModeName = 'default' | 'uglify' | 'beautify';

export interface TransformOptions {
  readonly mode: TransformModeName;
  readonly obfuscation: boolean;
  readonly disableLiteralsOptimization: boolean;
  readonly disableNamespacesOptimization: boolean;
  readonly retainParentheses: boolean;
  /** Spaces per beautified indentation level; uses tabs when omitted. */
  readonly indentationSpaces?: number;
  readonly excludedNames: readonly string[];
}

class SilentEditor extends EditorRouter {}

/** Runs greybel-builder-web in a worker to transform page code independently. */
export class Builder {
  readonly #host: WorkerHost<{ builder: EditorChannel }>;

  constructor() {
    const worker = new Worker(builderScript);

    serveWasm(worker, builderWasm);
    this.#host = new WorkerHost(worker, {
      builder: { channelClass: EditorChannel, routers: [new SilentEditor()] }
    });
    new DocumentsChannel(this.#host.transport).use(new MemoryEditor());
  }

  /** Transforms `code` using `options`. */
  transform(code: string, options: TransformOptions): Promise<string> {
    return this.#host.channels.builder.send(
      new Transform({
        code,
        transpile: new TranspileSettings({
          mode: transformMode(options),
          obfuscation: options.obfuscation,
          strictMode: false,
          variables: {},
          excludedNames: [...options.excludedNames]
        })
      })
    );
  }

  close(): void {
    this.#host.close();
  }
}

function transformMode(options: TransformOptions): TranspileMode {
  switch (options.mode) {
    case 'uglify':
      return new TranspileMode.Uglify({
        disableLiteralsOptimization: options.disableLiteralsOptimization,
        disableNamespacesOptimization: options.disableNamespacesOptimization
      });
    case 'beautify':
      return new TranspileMode.Beautify({
        retainParentheses: options.retainParentheses,
        indentationSpaces: options.indentationSpaces
      });
    default:
      return new TranspileMode.Default();
  }
}
