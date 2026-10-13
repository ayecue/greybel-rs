import { spawn, type ChildProcessByStdio } from 'node:child_process';
import { join } from 'node:path';
import type { Readable, Writable } from 'node:stream';

import { Transport } from 'duplex-core';
import { StreamDriver } from 'duplex-driver-stdio';
import { EditorChannel as DocumentsChannel, MemoryEditor } from 'greybel-documents-host-provider';
import { inject } from 'vitest';

import { EditorChannel, EditorRouter, TranspileSettings, type TranspileMode } from '../../src/index.js';

export const BINARY = 'greybel-builder';

export const BUILD_TIMEOUT = 600_000;

export const MAIN = 'file:///project/main.src';

export const LIBRARY = 'file:///project/lib.src';

export type Child = ChildProcessByStdio<Writable, Readable, null>;

class SilentEditor extends EditorRouter {}

export function settings(mode: TranspileMode): TranspileSettings {
  return new TranspileSettings({ mode, obfuscation: false, strictMode: false, variables: {}, excludedNames: [] });
}

export const children: Child[] = [];

export function binaryPath(): string {
  const target = process.env.CARGO_TARGET_DIR ?? inject('target');

  return join(target, 'debug', BINARY);
}

export function start(files: Array<[string, string]>): EditorChannel {
  const child = spawn(binaryPath(), [], { stdio: ['pipe', 'pipe', 'inherit'] });

  children.push(child);

  const transport = Transport.of(StreamDriver.child(child));
  const editor = new EditorChannel(transport).use(new SilentEditor());

  new DocumentsChannel(transport).use(new MemoryEditor(files));

  return editor;
}
