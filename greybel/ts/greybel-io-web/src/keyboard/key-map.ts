import { Key, Side } from '../protocol.generated.js';

type ModifierKey = typeof Key.Shift | typeof Key.Control | typeof Key.Alt;

export class KeyMap {
  static readonly #NAMED_KEYS: Readonly<Record<string, Key>> = {
    Enter: new Key.Enter(),
    Escape: new Key.Escape(),
    Tab: new Key.Tab(),
    Backspace: new Key.Backspace(),
    Delete: new Key.Delete(),
    Insert: new Key.Insert(),
    Home: new Key.Home(),
    End: new Key.End(),
    PageUp: new Key.PageUp(),
    PageDown: new Key.PageDown(),
    ArrowUp: new Key.Up(),
    ArrowDown: new Key.Down(),
    ArrowLeft: new Key.Left(),
    ArrowRight: new Key.Right(),
    AltGraph: new Key.Alt({ value: Side.Right })
  };

  static readonly #MODIFIER_KEYS: Readonly<Record<string, ModifierKey>> = {
    Shift: Key.Shift,
    Control: Key.Control,
    Alt: Key.Alt
  };

  static readonly #FUNCTION_KEY_PATTERN = /^F([1-9]\d?)$/;

  static of(event: Pick<KeyboardEvent, 'key' | 'code'>): Key | undefined {
    const namedKey = KeyMap.#NAMED_KEYS[event.key];

    if (namedKey !== undefined) return namedKey;

    const modifierKey = KeyMap.#MODIFIER_KEYS[event.key];

    if (modifierKey !== undefined) return new modifierKey({ value: KeyMap.#getSide(event.code) });

    const functionKeyMatch = KeyMap.#FUNCTION_KEY_PATTERN.exec(event.key);

    if (functionKeyMatch !== null) return new Key.F({ value: Number(functionKeyMatch[1]) });
    if ([...event.key].length === 1) return new Key.Char({ value: event.key });

    return undefined;
  }

  static #getSide(code: string): Side {
    return code.endsWith('Right') ? Side.Right : Side.Left;
  }
}
