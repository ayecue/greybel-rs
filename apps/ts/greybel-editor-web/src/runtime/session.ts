import { ChannelError } from 'duplex-core';
import { WorkerHost } from 'duplex-driver-worker';
import { EditorChannel as DocumentsChannel, MemoryEditor } from 'greybel-documents-host-provider';
import { PageChannel } from 'greybel-io-web';
import {
  Continue,
  EditorChannel,
  EditorRouter,
  Environment,
  Evaluate,
  EvaluationFailure,
  Exit,
  Launch,
  LaunchFailure,
  MultilineStringStyle,
  Pause,
  Scopes,
  SetBreakpoints,
  SourceBreakpoint,
  StackTrace,
  Step,
  Terminate,
  Variables,
  type Evaluation,
  type LogMessage,
  type Scope,
  type SourceLocation,
  type StackFrame,
  type StepMode,
  type StopNotice,
  type Variable
} from 'greybel-runtime';
import runtimeScript from 'greybel-runtime-web/dist/index.js?url';
import runtimeWasm from 'greybel-runtime-web/dist/greybel-runtime-web.wasm?url';
import { Render, RichText, TerminalChannel } from 'greyhack-terminal-text';
import { World } from 'greyhack-world-operations';

import type { ResolvedSettings } from '../project/manifest.js';
import type { PathResolver } from '../project/paths.js';
import type { WasmLoader } from '../wasm.js';
import type { Terminal } from './terminal.js';

const ANSI_COLOR = /\x1b\[[\d;]*m/g;

interface RuntimeConnection {
  readonly host: WorkerHost<Record<string, never>>;
  readonly runtime: EditorChannel;
  readonly text: TerminalChannel;
}

export interface RunOptions {
  readonly documents: ReadonlyMap<string, string>;
  readonly entry: string;
  readonly settings: ResolvedSettings;
  readonly seed: string;
  readonly debug: boolean;
}

export interface SessionListener {
  onStop(notice: StopNotice): void;
  onLog(message: LogMessage): void;
}

class ListenerRouter extends EditorRouter {
  constructor(private readonly listener: SessionListener) {
    super();
  }

  override onStopNotice(message: StopNotice): void {
    this.listener.onStop(message);
  }

  override onLogMessage(message: LogMessage): void {
    this.listener.onLog(message);
  }
}

export class RuntimeSession {
  readonly #channels: Promise<RuntimeConnection>;
  readonly #documents = new MemoryEditor();
  readonly #paths: PathResolver;

  constructor(
    terminal: Terminal,
    listener: SessionListener,
    world: Promise<MessagePort>,
    wasm: WasmLoader,
    paths: PathResolver
  ) {
    this.#paths = paths;
    this.#channels = Promise.all([wasm.startWorker('runtime', runtimeScript, runtimeWasm), world]).then(
      ([worker, port]) => {
        const host = new WorkerHost<Record<string, never>>(worker, {}, { ports: [{ protocol: World, port }] });

        new PageChannel(host.transport).use(terminal);
        new DocumentsChannel(host.transport).use(this.#documents);

        return {
          host,
          runtime: new EditorChannel(host.transport).use(new ListenerRouter(listener)),
          text: new TerminalChannel(host.transport)
        };
      }
    );
  }

  async run(options: RunOptions): Promise<Exit> {
    const { runtime } = await this.#channels;

    this.#documents.files.clear();
    for (const [uri, code] of options.documents) this.#documents.files.set(uri, code);

    const { manifest, runVariables } = options.settings;

    return runtime.send(
      new Launch({
        entry: this.#paths.toUri(options.entry),
        root: this.#paths.rootUri,
        extensions: [...manifest.extensions],
        environment: new Environment.Mock({ seed: options.seed, memoryLimit: manifest.run.memoryLimit }),
        debug: options.debug,
        stopOnEntry: false,
        params: [...manifest.run.params],
        variables: { ...runVariables },
        strictMode: manifest.strictMode,
        programName: manifest.run.programName,
        disabledSyntax: options.settings.disabledSyntax,
        multilineStrings:
          manifest.build.multilineStrings === 'Join' ? MultilineStringStyle.Join : MultilineStringStyle.Concatenation
      })
    );
  }

  /** Removes tags with the runtime's terminal-text renderer, without wrapping, then the ANSI color codes. */
  async renderPlainText(text: string): Promise<string> {
    const { text: renderer } = await this.#channels;
    const { texts } = await renderer.send(new Render({ texts: [new RichText({ text, columns: 0 })] }));

    return (texts[0] ?? '').replace(ANSI_COLOR, '');
  }

  /** One-based lines. */
  async setBreakpoints(uri: string, lines: readonly number[]): Promise<void> {
    await (
      await this.#channels
    ).runtime.send(
      new SetBreakpoints({
        uri,
        breakpoints: lines.map((line) => new SourceBreakpoint({ line }))
      })
    );
  }

  resume(): void {
    void this.#channels.then(({ runtime }) => runtime.send(new Continue()));
  }

  step(mode: StepMode): void {
    void this.#channels.then(({ runtime }) => runtime.send(new Step({ mode })));
  }

  pause(): void {
    void this.#channels.then(({ runtime }) => runtime.send(new Pause()));
  }

  terminate(): void {
    void this.#channels.then(({ runtime }) => runtime.send(new Terminate()));
  }

  getStackTrace(): Promise<StackFrame[]> {
    return this.#channels.then(({ runtime }) => runtime.send(new StackTrace()));
  }

  getScopes(frame: number): Promise<Scope[]> {
    return this.#channels.then(({ runtime }) => runtime.send(new Scopes({ frame })));
  }

  getVariables(reference: number): Promise<Variable[]> {
    return this.#channels.then(({ runtime }) => runtime.send(new Variables({ reference })));
  }

  /** Returns undefined for statements. */
  evaluate(code: string): Promise<Evaluation | null> {
    return this.#channels.then(({ runtime }) => runtime.send(new Evaluate({ code })));
  }

  close(): void {
    void this.#channels.then(({ host }) => host.close());
  }

  describeLocation(location: SourceLocation): string {
    const file = this.#paths.toPath(location.uri) ?? location.uri;

    return `${file}:${location.line}:${location.character}`;
  }

  /** Undefined when the program completed. */
  describeExit(exit: Exit): string | undefined {
    if (exit instanceof Exit.Crash) {
      const stack = exit.stack.map((location) => `\n  at ${this.describeLocation(location)}`).join('');

      return `Runtime error: ${exit.message}${stack}`;
    }

    if (exit instanceof Exit.Termination) return 'The program was stopped.';

    return undefined;
  }

  describeFailure(error: unknown): string {
    const failure = error instanceof ChannelError ? (error.failure as unknown) : undefined;

    if (failure instanceof LaunchFailure.Invalid) {
      return `Prepare error: ${failure.message}${failure.location ? ` at ${this.describeLocation(failure.location)}` : ''}`;
    }

    if (failure instanceof LaunchFailure.Busy) return 'A program runs already.';
    if (failure instanceof LaunchFailure.Unavailable || failure instanceof LaunchFailure.Unreachable) {
      return "Grey Hack can't be reached from the browser.";
    }

    if (failure instanceof EvaluationFailure.CodeError) return failure.message;
    if (failure instanceof EvaluationFailure.ActiveProgram) return 'The program runs: pause it first.';

    return error instanceof Error ? error.message : String(error);
  }
}
