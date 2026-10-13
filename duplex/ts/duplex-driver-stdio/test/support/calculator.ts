import { spawn, type ChildProcessByStdio } from 'node:child_process';
import { join } from 'node:path';
import type { Readable, Writable } from 'node:stream';

import { inject } from 'vitest';

import { StreamDriver } from '../../src/index.js';
import { Progress, UserChannel, UserRouter } from '../calculator.client.generated.js';

export const EXAMPLE = 'stdio-calculator';

export const BUILD_TIMEOUT = 600_000;

export type CalculatorProcess = ChildProcessByStdio<Writable, Readable, null>;

export class Confirmer extends UserRouter {
  readonly notes: string[] = [];
  #nextNote: (() => void) | undefined;

  constructor(readonly confirm: boolean) {
    super();
  }

  override onConfirm(): boolean {
    return this.confirm;
  }

  override onProgress(message: Progress): void {
    if (typeof message.note === 'string') this.notes.push(message.note);
    this.#nextNote?.();
  }

  nextNote(): Promise<void> {
    return new Promise((resolve) => {
      this.#nextNote = resolve;
    });
  }
}

export const calculators: CalculatorProcess[] = [];

export function path(): string {
  const target = process.env.CARGO_TARGET_DIR ?? inject('target');

  return join(target, 'debug', 'examples', EXAMPLE);
}

export function calculate(confirm = true): { user: UserChannel; confirmer: Confirmer; calculator: CalculatorProcess } {
  const calculator = spawn(path(), [], { stdio: ['pipe', 'pipe', 'inherit'] });

  calculators.push(calculator);

  const confirmer = new Confirmer(confirm);
  const user = new UserChannel(StreamDriver.child(calculator), { healthCheck: false }).use(confirmer);

  return { user, confirmer, calculator };
}
