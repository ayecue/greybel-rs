import * as vscode from 'vscode';

const TYPE = 'greyscript';

export const PROGRAM = 'debug/main.src';

export const LOOP = 'debug/loop.src';

export interface Stop {
  readonly reason: string;
  readonly threadId: number;
}

export class DebugRecorder {
  readonly stops: Stop[] = [];

  readonly logs: string[] = [];
  readonly #unread: Stop[] = [];
  #session: vscode.DebugSession | undefined;
  #waitingForStop: ((stop: Stop) => void) | undefined;
  #waitingForEnd: (() => void) | undefined;
  #ended = false;
  readonly #disposables: vscode.Disposable[] = [];

  constructor() {
    this.#disposables.push(
      vscode.debug.registerDebugAdapterTrackerFactory(TYPE, {
        createDebugAdapterTracker: (session) => {
          this.#session = session;

          return {
            onDidSendMessage: (message: {
              type: string;
              event?: string;
              body?: Stop & { category?: string; output?: string };
            }) => {
              if (message.type !== 'event') return;
              if (message.event === 'output' && message.body?.category === 'console' && message.body.output) {
                this.logs.push(message.body.output.trimEnd());
              }

              if (message.event === 'stopped' && message.body) {
                this.stops.push(message.body);

                const waiting = this.#waitingForStop;

                this.#waitingForStop = undefined;
                if (waiting) waiting(message.body);
                else this.#unread.push(message.body);
              }

              if (message.event !== 'terminated') return;
              this.#ended = true;
              this.#waitingForEnd?.();
            }
          };
        }
      })
    );
  }

  async start(program: vscode.Uri, debug: boolean): Promise<void> {
    const started = await vscode.debug.startDebugging(
      vscode.workspace.getWorkspaceFolder(program),
      {
        type: TYPE,
        name: 'test',
        request: 'launch',
        program: program.scheme === 'file' ? program.fsPath : program.toString()
      },
      { noDebug: !debug }
    );

    if (!started) throw new Error("the debug session didn't start");
  }

  session(): vscode.DebugSession {
    if (!this.#session) throw new Error('no debug session started');

    return this.#session;
  }

  stopped(): Promise<Stop> {
    const unread = this.#unread.shift();

    if (unread) return Promise.resolve(unread);

    return new Promise((resolve) => {
      this.#waitingForStop = resolve;
    });
  }

  terminated(): Promise<void> {
    if (this.#ended) return Promise.resolve();

    return new Promise((resolve) => {
      this.#waitingForEnd = resolve;
    });
  }

  dispose(): void {
    for (const disposable of this.#disposables) disposable.dispose();
  }
}
