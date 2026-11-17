import { StopReason, type Evaluation, type SourceLocation } from 'greybel-runtime';

export interface ConsoleEntry {
  readonly kind: 'input' | 'output' | 'error' | 'log';
  readonly text: string;
  /** An expandable result while `stop` is the current stop. */
  readonly evaluation?: Evaluation;
  readonly stop: number;
}

const STOP_REASONS: Readonly<Record<StopReason, string>> = {
  [StopReason.Entry]: 'Paused on entry',
  [StopReason.Breakpoint]: 'Paused on breakpoint',
  [StopReason.Debugger]: 'Paused on debugger statement',
  [StopReason.Step]: 'Paused after step',
  [StopReason.Pause]: 'Paused'
};

/** A terminal's debugger state for one debug run: the last stop, the open variables and the console. */
export class DebugState {
  isActive = false;
  stopReason: StopReason | undefined;
  stopLocation: SourceLocation | undefined;
  /** Incremented on each stop; variable and evaluation references are valid only until the next one. */
  stop = 0;
  /** Paths of the open variable nodes, retained across stops. */
  readonly openPaths = new Set<string>();
  /** Inputs, oldest first. */
  readonly history: string[] = [];
  entries: readonly ConsoleEntry[] = [];

  start(): void {
    this.isActive = true;
    this.clearStop();
    this.entries = [];
  }

  finish(): void {
    this.isActive = false;
    this.clearStop();
  }

  clearStop(): void {
    this.stopReason = undefined;
    this.stopLocation = undefined;
  }

  recordStop(reason: StopReason, location: SourceLocation | undefined): void {
    this.stopReason = reason;
    this.stopLocation = location;
    this.stop += 1;
  }

  addEntry(kind: ConsoleEntry['kind'], text: string, evaluation?: Evaluation): void {
    this.entries = [...this.entries, { kind, text, evaluation, stop: this.stop }];
  }

  addInput(code: string): void {
    if (this.history[this.history.length - 1] !== code) this.history.push(code);
    this.addEntry('input', code);
  }

  /** Undefined while the program runs. */
  describeStop(describeLocation: (location: SourceLocation) => string): string | undefined {
    if (this.stopReason === undefined) return undefined;

    const at = this.stopLocation === undefined ? '' : ` at ${describeLocation(this.stopLocation)}`;

    return `${STOP_REASONS[this.stopReason]}${at}`;
  }
}
