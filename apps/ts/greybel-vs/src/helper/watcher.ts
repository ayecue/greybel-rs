import vscode from 'vscode';

import { Configuration } from '../settings';
import { wait } from './wait';

function withTimeout<T>(promise: Promise<T>, timeout: number): Promise<T> {
  return Promise.race([
    wait(timeout).then(() => Promise.reject(new Error(`the build timed out after ${timeout} ms`))),
    promise
  ]);
}

class BuildDispatcher {
  static readonly BUILD_TIMEOUT = 5 * 60 * 1000;

  readonly #startEvent = new vscode.EventEmitter<void>();
  readonly #failureEvent = new vscode.EventEmitter<Error>();
  readonly #completionEvent = new vscode.EventEmitter<boolean>();
  readonly onStart = this.#startEvent.event;
  readonly onFailure = this.#failureEvent.event;
  readonly onCompletion = this.#completionEvent.event;

  private _buildCommand: () => Promise<void>;
  private _delay: number;
  private _timer: NodeJS.Timeout | null;
  private _pending: boolean;
  private _hasRun: boolean;

  constructor(buildCommand: () => Promise<void>, delay: number = 1000) {
    this._buildCommand = buildCommand;
    this._delay = delay;
    this._pending = false;
    this._timer = null;
    this._hasRun = false;
  }

  public run(): void {
    if (this._pending) {
      return;
    }

    if (this._hasRun) {
      return;
    }

    clearTimeout(this._timer);

    this._timer = setTimeout(async () => {
      let success = false;

      try {
        this._pending = true;
        this.#startEvent.fire();
        await withTimeout(this._buildCommand(), BuildDispatcher.BUILD_TIMEOUT);
        success = true;
      } catch (error) {
        this.#failureEvent.fire(error);
      } finally {
        this._pending = false;
        this._hasRun = true;
        this.#completionEvent.fire(success);
      }
    }, this._delay);
  }

  public hasRun(): boolean {
    return this._hasRun;
  }

  public dispose(): void {
    this.#startEvent.dispose();
    this.#failureEvent.dispose();
    this.#completionEvent.dispose();
    this._buildCommand = null;

    if (!this._timer) return;
    clearTimeout(this._timer);
    this._timer = null;
  }
}

export class Watcher {
  private _watcher: vscode.FileSystemWatcher | null;
  private _watchEnabled: boolean;
  private _buildCommand: () => Promise<void>;
  private readonly _isSource: (uri: vscode.Uri) => Promise<boolean>;
  private _delayedBuild: BuildDispatcher | null;
  private _pendingChanges: number;

  /** Debounces `buildCommand` after changes to files accepted by `isSource`. */
  constructor(buildCommand: () => Promise<void>, isSource: (uri: vscode.Uri) => Promise<boolean>) {
    this._watcher = null;
    this._watchEnabled = false;
    this._delayedBuild = null;
    this._pendingChanges = 0;
    this._buildCommand = buildCommand;
    this._isSource = isSource;

    this.followWatchSetting();
  }

  private followWatchSetting(): void {
    const onConfigurationChange = (event: vscode.ConfigurationChangeEvent) => {
      const isWatchEnabled = new Configuration().get('transpiler.watch');

      if (isWatchEnabled && isWatchEnabled != this._watchEnabled) {
        this._watchEnabled = true;
        this.start();
      } else if (!isWatchEnabled && isWatchEnabled != this._watchEnabled) {
        this._watchEnabled = false;
        this.dispose();
      }
    };

    vscode.workspace.onDidChangeConfiguration(onConfigurationChange);
    this._watchEnabled = new Configuration().get('transpiler.watch');
  }

  public start(): this {
    if (this._watcher) {
      return this;
    }

    if (!this._watchEnabled) {
      return this;
    }

    this._watcher = vscode.workspace.createFileSystemWatcher('**/*');
    this._watcher.onDidChange(this.onDidChange.bind(this));
    this._watcher.onDidCreate(this.onDidChange.bind(this));
    this._watcher.onDidDelete(this.onDidChange.bind(this));

    return this;
  }

  private async onDidChange(uri: vscode.Uri): Promise<void> {
    if (!(await this._isSource(uri))) {
      return;
    }

    this._pendingChanges++;
    this.scheduleBuild();
  }

  private scheduleBuild() {
    if (this._delayedBuild) {
      this._delayedBuild.run();
    } else if (this._pendingChanges > 0) {
      this._delayedBuild = new BuildDispatcher(this._buildCommand);
      this._delayedBuild.onStart(() => {
        this._pendingChanges = 0;
      });
      this._delayedBuild.onFailure((error) => {
        console.error(`The build failed: ${error.message}`);
      });
      this._delayedBuild.onCompletion(() => {
        this._delayedBuild.dispose();
        this._delayedBuild = null;
        this.scheduleBuild();
      });
      this._delayedBuild.run();
    }
  }

  public dispose(): void {
    if (!this._watcher) return;
    this._watcher.dispose();
    this._watcher = null;
  }
}
