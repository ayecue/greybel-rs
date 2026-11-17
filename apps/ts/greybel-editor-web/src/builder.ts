import { ChannelError } from 'duplex-core';
import { WorkerHost } from 'duplex-driver-worker';
import { EditorChannel as DocumentsChannel, MemoryEditor } from 'greybel-documents-host-provider';
import {
  BuildFailure,
  EditorChannel,
  EditorRouter,
  MultilineStringStyle,
  Transform,
  TranspileMode,
  TranspileSettings
} from 'greybel-builder';
import builderScript from 'greybel-builder-web/dist/index.js?url';
import builderWasm from 'greybel-builder-web/dist/greybel-builder-web.wasm?url';

import type { ResolvedSettings } from './project/manifest.js';
import type { WasmLoader } from './wasm.js';

class SilentEditor extends EditorRouter {}

export class Builder {
  readonly #host: Promise<WorkerHost<{ builder: EditorChannel }>>;

  constructor(wasm: WasmLoader) {
    this.#host = wasm.startWorker('transformer', builderScript, builderWasm).then((worker) => {
      const host = new WorkerHost(worker, {
        builder: { channelClass: EditorChannel, routers: [new SilentEditor()] }
      });

      new DocumentsChannel(host.transport).use(new MemoryEditor());

      return host;
    });
  }

  async transform(code: string, settings: ResolvedSettings): Promise<string> {
    const { build, strictMode } = settings.manifest;
    const host = await this.#host;

    return host.channels.builder.send(
      new Transform({
        code,
        transpile: new TranspileSettings({
          mode: this.#createTranspileMode(settings),
          obfuscation: build.obfuscation,
          strictMode,
          variables: { ...settings.buildVariables },
          excludedNames: [...build.excludedNamespaces],
          disabledSyntax: settings.disabledSyntax,
          multilineStrings:
            build.multilineStrings === 'Join' ? MultilineStringStyle.Join : MultilineStringStyle.Concatenation
        })
      })
    );
  }

  close(): void {
    void this.#host.then((host) => host.close());
  }

  describeFailure(error: unknown): string {
    const failure = error instanceof ChannelError ? (error.failure as unknown) : undefined;

    if (failure instanceof BuildFailure.Invalid) {
      const at = failure.location ? ` at line ${failure.location.line}, character ${failure.location.character}` : '';

      return `Transform error: ${failure.message}${at}`;
    }

    return error instanceof Error ? error.message : String(error);
  }

  #createTranspileMode({ manifest }: ResolvedSettings): TranspileMode {
    const { build, format } = manifest;

    switch (build.mode) {
      case 'Uglify':
        return new TranspileMode.Uglify({
          disableLiteralsOptimization: build.disableLiteralsOptimization,
          disableNamespacesOptimization: build.disableNamespacesOptimization
        });
      case 'Beautify':
        return new TranspileMode.Beautify({
          retainParentheses: format.retainParentheses,
          indentationSpaces: format.indentation === 'Whitespace' ? format.indentationSpaces : undefined
        });
      default:
        return new TranspileMode.Default();
    }
  }
}
