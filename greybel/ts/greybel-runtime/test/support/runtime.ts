import { spawn, type ChildProcessByStdio } from 'node:child_process';
import { join } from 'node:path';
import type { Readable, Writable } from 'node:stream';

import { Transport } from 'duplex-core';
import { StreamDriver } from 'duplex-driver-stdio';
import { EditorChannel as DocumentsChannel, MemoryEditor } from 'greybel-documents-host-provider';
import { Output, PageChannel, PageRouter, type OutputBatch } from 'greybel-io-web';
import { inject } from 'vitest';

import { EditorChannel, EditorRouter, Environment, Exit, Launch, type StopNotice } from '../../src/index.js';

/** The Rust example serving the runtime over standard input and output (greybel/rs/greybel-runtime/examples). */
export const EXAMPLE = 'stdio-runtime';

export const BUILD_TIMEOUT = 600_000;

/** How much longer than its wait a test waiting for the runtime may take. */
export const TIMEOUT_MARGIN = 10_000;

export const IDLE_TIME = 25_000;

export const MAIN = 'file:///project/main.src';

export const PROGRAM =
  'f = function(x)\n  y = x * 2\n  return y\nend function\na = 1\nb = f(a)\nprint(b)\nprint(a + b)';

export type Child = ChildProcessByStdio<Writable, Readable, null>;

export class StopRecorder extends EditorRouter {
  readonly stops: StopNotice[] = [];
  #resolve: (() => void) | undefined;

  override onStopNotice(message: StopNotice): void {
    this.stops.push(message);
    this.#resolve?.();
  }

  next(): Promise<void> {
    return new Promise((resolve) => {
      this.#resolve = resolve;
    });
  }
}

export class Screen extends PageRouter {
  readonly printedTexts: string[] = [];

  override onOutputBatch(message: OutputBatch): void {
    for (const entry of message.entries) {
      if (!(entry instanceof Output.Print)) continue;
      this.printedTexts.push(entry.text);
    }
  }
}

export const children: Child[] = [];

export function binaryPath(): string {
  const target = process.env.CARGO_TARGET_DIR ?? inject('target');

  return join(target, 'debug', 'examples', EXAMPLE);
}

export function start(files: Array<[string, string]>) {
  const child = spawn(binaryPath(), [], { stdio: ['pipe', 'pipe', 'inherit'] });

  children.push(child);

  const transport = Transport.of(StreamDriver.child(child));
  const recorder = new StopRecorder();
  const screen = new Screen();
  const editor = new EditorChannel(transport).use(recorder);

  new PageChannel(transport).use(screen);
  new DocumentsChannel(transport).use(new MemoryEditor(files));

  return { editor, recorder, screen };
}

export function launch(editor: EditorChannel, debug: boolean): Promise<Exit> {
  return editor.send(
    new Launch({
      entry: MAIN,
      extensions: ['src'],
      environment: new Environment.Mock({ seed: 'test', memoryLimit: 1024 }),
      debug,
      stopOnEntry: false,
      params: [],
      variables: {},
      strictMode: false,
      programName: 'myprogram'
    })
  );
}
