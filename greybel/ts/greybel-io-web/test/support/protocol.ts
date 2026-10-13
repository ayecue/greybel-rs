import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { inject, onTestFinished } from 'vitest';

import { BinaryReader } from 'duplex-core';
import { MessageTargetDriver } from 'duplex-driver-worker';

import {
  Echo,
  Key,
  KeyboardInput,
  PageChannel,
  PageRouter,
  ReadKey,
  ReadLine,
  ScreenOutput,
  type TextScreen
} from '../../src/index.js';
import * as programProtocol from '../../src/program.generated.js';

interface FixtureSet {
  output: string;
  readLine: string;
  readKey: string;
  line: string;
  keys: Record<string, string>;
}

export const fixtures = JSON.parse(
  readFileSync(join(inject('rust'), 'greybel-io-web/tests/fixtures/messages.json'), 'utf8')
) as FixtureSet;

export function read<T>(layout: { read(reader: BinaryReader): T }, hex: string): T {
  const reader = new BinaryReader(Uint8Array.from(hex.match(/../g) ?? [], (byte) => Number.parseInt(byte, 16)));
  const value = layout.read(reader);

  reader.finish();

  return value;
}

export const messages = {
  output: read(programProtocol.OutputBatch, fixtures.output),
  readLine: read(programProtocol.ReadLine, fixtures.readLine),
  readKey: read(programProtocol.ReadKey, fixtures.readKey),
  line: read({ read: (reader) => reader.string() }, fixtures.line)
};

export const keys = Object.fromEntries(Object.entries(fixtures.keys).map(([name, key]) => [name, read(Key, key)]));

export function connect(...routers: PageRouter[]): programProtocol.ProgramChannel {
  const { port1, port2 } = new MessageChannel();

  onTestFinished(() => {
    port1.close();
    port2.close();
  });
  new PageChannel(new MessageTargetDriver(port1), { healthCheck: false }).use(...routers);

  return new programProtocol.ProgramChannel(new MessageTargetDriver(port2), { healthCheck: false });
}

export function keydown(key: string, code = key): Event {
  return Object.assign(new Event('keydown', { cancelable: true }), { key, code });
}

export function renderObserver(lines: string[]): { render: (screen: TextScreen) => void; shown: Promise<void> } {
  let resolve!: () => void;
  const shown = new Promise<void>((done) => (resolve = done));
  const render = (screen: TextScreen) => {
    if (JSON.stringify(screen.lines) !== JSON.stringify(lines)) return;
    resolve();
  };

  return { render, shown };
}

export class FixtureInputs extends PageRouter {
  override onReadLine(request: ReadLine): string {
    return request.prompt === 'password: ' && request.echo === Echo.Masked ? messages.line : '';
  }

  override onReadKey(): Key {
    return keys.up!;
  }
}

export const PASSWORD_KEYS = ['h', 'x', 'Backspace', 'i', 'Shift', 'Enter'];

export function typeKeys(keyboard: EventTarget, keys: string[]): void {
  for (const key of keys) {
    keyboard.dispatchEvent(keydown(key, key === 'Shift' ? 'ShiftLeft' : key));
  }
}

export function passwordFixture() {
  const keyboard = new EventTarget();
  const promptRender = renderObserver(['password: ']);
  const maskedRender = renderObserver(['password: **', '']);
  const program = connect(
    new ScreenOutput((screen) => {
      promptRender.render(screen);
      maskedRender.render(screen);
    }),
    new KeyboardInput(keyboard)
  );

  return { keyboard, program, promptRender, maskedRender };
}

export async function requestedPassword() {
  const { keyboard, program, promptRender, maskedRender } = passwordFixture();
  const line = program.send(new programProtocol.ReadLine({ prompt: 'password: ', echo: Echo.Masked }));

  await promptRender.shown;

  return { keyboard, line, maskedRender };
}
