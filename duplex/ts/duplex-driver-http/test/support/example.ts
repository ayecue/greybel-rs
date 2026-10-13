import { execFileSync, spawn, type ChildProcessByStdio } from 'node:child_process';
import { join } from 'node:path';
import type { Readable, Writable } from 'node:stream';

import { inject } from 'vitest';

import type { Downstream } from '../../src/index.js';

const EXAMPLE = 'http-calculator';
const LISTENING_LINE = /^listening (\S+)$/m;

export const BUILD_TIMEOUT = 600_000;

const EXAMPLE_MODES: Record<Downstream, string> = { 'long-poll': 'http', events: 'sse', stream: 'stream' };

export class CalculatorExample {
  private constructor(
    readonly url: string,
    readonly origin: string,
    private readonly child: ChildProcessByStdio<Writable, Readable, null>
  ) {}

  static build(): void {
    execFileSync('cargo', ['build', '--quiet', '--package', 'duplex-driver-http', '--example', EXAMPLE], {
      cwd: inject('rust'),
      stdio: 'inherit'
    });
    execFileSync(CalculatorExample.#path(), { input: '', stdio: ['pipe', 'ignore', 'inherit'] });
  }

  static async start(downstream: Downstream, allowOrigin?: string): Promise<CalculatorExample> {
    const env = allowOrigin === undefined ? process.env : { ...process.env, ALLOW_ORIGIN: allowOrigin };
    const child = spawn(CalculatorExample.#path(), [EXAMPLE_MODES[downstream]], {
      stdio: ['pipe', 'pipe', 'inherit'],
      env
    });
    const url = await new Promise<string>((resolve, reject) => {
      let printed = '';

      child.stdout.on('data', (bytes: Buffer) => {
        printed += bytes.toString();

        const listeningLine = LISTENING_LINE.exec(printed);

        if (listeningLine === null) return;
        resolve(listeningLine[1]!);
      });
      child.once('exit', () => reject(new Error('the calculator exited')));
    });

    return new CalculatorExample(url, new URL(url).origin, child);
  }

  stop(): void {
    this.child.kill();
  }

  static #path(): string {
    const target = process.env.CARGO_TARGET_DIR ?? inject('target');

    return join(target, 'debug', 'examples', EXAMPLE);
  }
}
