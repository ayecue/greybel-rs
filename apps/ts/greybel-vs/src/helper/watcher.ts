import vscode from 'vscode';

import { Configuration } from '../settings';
import { wait } from './wait';

const BUILD_TIMEOUT = 5 * 60 * 1000;

class BuildDispatcher {
  readonly #startEvent = new vscode.EventEmitter<void>();
  readonly #failureEvent = new vscode.EventEmitter<Error>();
  readonly #completionEvent = new vscode.EventEmitter<boolean>();
  readonly onStart = this.#startEvent.event;
  readonly onFailure = this.#failureEvent.event;
  readonly onCompletion = this.#completionEvent.event;

  #buildCommand: (() => Promise<void>) | null;
  readonly #delay: number;
  #timer: NodeJS.Timeout | null;
  #active: boolean;
  #runCount: number;

  constructor(buildCommand: () => Promise<void>, delay: number = 1000) {
    this.#buildCommand = buildCommand;
    this.#delay = delay;
    this.#active = false;
    this.#timer = null;
    this.#runCount = 0;
  }

  run(): void {
    if (this.#active) return;
    if (this.#runCount > 0) return;

    clearTimeout(this.#timer);

    this.#timer = setTimeout(async () => {
      let success = false;

      try {
        this.#active = true;
        this.#startEvent.fire();
        await this.#runWithTimeout(this.#buildCommand!());
        success = true;
      } catch (error) {
        this.#failureEvent.fire(error);
      } finally {
        this.#active = false;
        this.#runCount++;
        this.#completionEvent.fire(success);
      }
    }, this.#delay);
  }

  hasRun(): boolean {
    return this.#runCount > 0;
  }

  dispose(): void {
    this.#startEvent.dispose();
    this.#failureEvent.dispose();
    this.#completionEvent.dispose();
    this.#buildCommand = null;

    if (!this.#timer) return;
    clearTimeout(this.#timer);
    this.#timer = null;
  }

  #runWithTimeout<T>(promise: Promise<T>): Promise<T> {
    return Promise.race([
      wait(BUILD_TIMEOUT).then(() => Promise.reject(new Error(`the build timed out after ${BUILD_TIMEOUT} ms`))),
      promise
    ]);
  }
}

export class Watcher {
  #watcher: vscode.FileSystemWatcher | null;
  #watch: boolean;
  #buildCommand: () => Promise<void>;
  readonly #isSource: (uri: vscode.Uri) => Promise<boolean>;
  #dispatcher: BuildDispatcher | null;
  #changeCount: number;

  /** Debounces `buildCommand` after changes to files accepted by `isSource`. */
  constructor(buildCommand: () => Promise<void>, isSource: (uri: vscode.Uri) => Promise<boolean>) {
    this.#watcher = null;
    this.#watch = false;
    this.#dispatcher = null;
    this.#changeCount = 0;
    this.#buildCommand = buildCommand;
    this.#isSource = isSource;

    this.#followWatchSetting();
  }

  #followWatchSetting(): void {
    const onConfigurationChange = (event: vscode.ConfigurationChangeEvent) => {
      const watch = new Configuration().get('transpiler.watch');

      if (watch && watch != this.#watch) {
        this.#watch = true;
        this.start();
      } else if (!watch && watch != this.#watch) {
        this.#watch = false;
        this.dispose();
      }
    };

    vscode.workspace.onDidChangeConfiguration(onConfigurationChange);
    this.#watch = new Configuration().get('transpiler.watch');
  }

  start(): this {
    if (this.#watcher) return this;
    if (!this.#watch) return this;

    this.#watcher = vscode.workspace.createFileSystemWatcher('**/*');
    this.#watcher.onDidChange(this.#onDidChange.bind(this));
    this.#watcher.onDidCreate(this.#onDidChange.bind(this));
    this.#watcher.onDidDelete(this.#onDidChange.bind(this));

    return this;
  }

  async #onDidChange(uri: vscode.Uri): Promise<void> {
    if (!(await this.#isSource(uri))) return;

    this.#changeCount++;
    this.#scheduleBuild();
  }

  #scheduleBuild(): void {
    if (this.#dispatcher) {
      this.#dispatcher.run();
    } else if (this.#changeCount > 0) {
      this.#dispatcher = new BuildDispatcher(this.#buildCommand);
      this.#dispatcher.onStart(() => {
        this.#changeCount = 0;
      });
      this.#dispatcher.onFailure((error) => {
        console.error(`The build failed: ${error.message}`);
      });
      this.#dispatcher.onCompletion(() => {
        this.#dispatcher.dispose();
        this.#dispatcher = null;
        this.#scheduleBuild();
      });
      this.#dispatcher.run();
    }
  }

  dispose(): void {
    if (!this.#watcher) return;
    this.#watcher.dispose();
    this.#watcher = null;
  }
}
