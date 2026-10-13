import { spawn, type ChildProcessByStdio } from 'node:child_process';
import { join } from 'node:path';
import type { Readable, Writable } from 'node:stream';

import { StreamDriver } from 'duplex-driver-stdio';
import { inject } from 'vitest';

import { TerminalChannel } from '../../src/index.js';

export const BUILD_TIMEOUT = 600_000;

export const processes: ChildProcessByStdio<Writable, Readable, null>[] = [];

export function start(): TerminalChannel {
  const target = process.env.CARGO_TARGET_DIR ?? inject('target');
  const child = spawn(join(target, 'debug', 'greybel-runtime'), [], { stdio: ['pipe', 'pipe', 'inherit'] });

  processes.push(child);

  return new TerminalChannel(StreamDriver.child(child));
}
