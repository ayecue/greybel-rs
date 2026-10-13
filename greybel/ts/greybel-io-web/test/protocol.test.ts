import { describe, expect, it } from 'vitest';

import { KeyboardInput, KeyMap, ScreenOutput } from '../src/index.js';
import { Echo, ReadKey, ReadLine } from '../src/program.generated.js';
import {
  connect,
  FixtureInputs,
  keydown,
  keys,
  messages,
  PASSWORD_KEYS,
  passwordFixture,
  requestedPassword,
  renderObserver,
  typeKeys
} from './support/protocol.js';

describe('the Io protocol', () => {
  it('should show the output batch of the fixture', async () => {
    const { render, shown } = renderObserver(['name: ']);
    const program = connect(new ScreenOutput(render));

    program.send(messages.output);
    await expect(shown).resolves.toBeUndefined();
  });

  it('should reply to the line input of the fixture', async () => {
    const program = connect(new FixtureInputs());
    const line = program.send(messages.readLine);

    await expect(line).resolves.toBe(messages.line);
  });

  it('should reply to the key input of the fixture', async () => {
    const program = connect(new FixtureInputs());
    const key = program.send(messages.readKey);

    await expect(key).resolves.toEqual(keys.up);
  });

  it('should read keyboard events as the keys the crate reads', () => {
    const events = [
      { key: 'ArrowUp', code: 'ArrowUp' },
      { key: 'PageDown', code: 'PageDown' },
      { key: 'F5', code: 'F5' },
      { key: 'Shift', code: 'ShiftLeft' },
      { key: 'a', code: 'KeyA' }
    ] as KeyboardEvent[];
    const [up, pageDown, f5, leftShift, char] = events.map((event) => KeyMap.of(event));

    expect(up).toEqual(keys.up);
    expect(pageDown).toEqual(keys.pageDown);
    expect(f5).toEqual(keys.f5);
    expect(leftShift).toEqual(keys.leftShift);
    expect(char).toEqual(keys.char);
  });
});

describe('the page routers', () => {
  it('should show the prompt of a line to read', async () => {
    const { program, promptRender } = passwordFixture();

    void program.send(new ReadLine({ prompt: 'password: ', echo: Echo.Masked }));
    await expect(promptRender.shown).resolves.toBeUndefined();
  });

  it('should read the line from the keyboard', async () => {
    const { keyboard, line } = await requestedPassword();

    typeKeys(keyboard, PASSWORD_KEYS);
    await expect(line).resolves.toBe('hi');
  });

  it('should show the line masked', async () => {
    const { keyboard, maskedRender } = await requestedPassword();

    typeKeys(keyboard, PASSWORD_KEYS);
    await expect(maskedRender.shown).resolves.toBeUndefined();
  });

  it('should read the next key and prevent the page from handling it', async () => {
    const keyboard = new EventTarget();
    const promptRender = renderObserver(['> ']);
    const program = connect(new ScreenOutput(promptRender.render), new KeyboardInput(keyboard));
    const key = program.send(new ReadKey({ prompt: '> ' }));

    await promptRender.shown;

    const event = keydown('ArrowUp');

    keyboard.dispatchEvent(event);
    await expect(key).resolves.toEqual(keys.up);
    expect(event.defaultPrevented).toBe(true);
  });
});
