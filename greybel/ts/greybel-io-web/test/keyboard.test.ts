import { describe, expect, it } from 'vitest';

import { Key, KeyMap, Side } from '../src/index.js';

describe('KeyMap', () => {
  it.each([
    [{ key: 'a', code: 'KeyA' }, new Key.Char({ value: 'a' })],
    [{ key: ' ', code: 'Space' }, new Key.Char({ value: ' ' })],
    [{ key: 'é', code: 'KeyE' }, new Key.Char({ value: 'é' })],
    [{ key: 'ArrowUp', code: 'ArrowUp' }, new Key.Up()],
    [{ key: 'Enter', code: 'NumpadEnter' }, new Key.Enter()],
    [{ key: 'PageDown', code: 'PageDown' }, new Key.PageDown()],
    [{ key: 'F12', code: 'F12' }, new Key.F({ value: 12 })],
    [{ key: 'Shift', code: 'ShiftLeft' }, new Key.Shift({ value: Side.Left })],
    [{ key: 'Control', code: 'ControlRight' }, new Key.Control({ value: Side.Right })],
    [{ key: 'AltGraph', code: 'AltRight' }, new Key.Alt({ value: Side.Right })]
  ])('should read %o as its key', (event, key) => {
    const input = event;
    const mapped = KeyMap.of(input);

    expect(mapped).toStrictEqual(key);
  });

  it.each(['Dead', 'Meta', 'CapsLock', 'Unidentified'])('should skip %s', (key) => {
    const input = { key, code: '' };
    const mapped = KeyMap.of(input);

    expect(mapped).toBeUndefined();
  });
});
