import { Key } from 'greybel-io-web';

const SEQUENCES: ReadonlyMap<string, () => Key> = new Map([
  ['\x1b[A', () => new Key.Up()],
  ['\x1b[B', () => new Key.Down()],
  ['\x1b[C', () => new Key.Right()],
  ['\x1b[D', () => new Key.Left()],
  ['\x1b[H', () => new Key.Home()],
  ['\x1b[F', () => new Key.End()],
  ['\x1b[1~', () => new Key.Home()],
  ['\x1b[4~', () => new Key.End()],
  ['\x1b[2~', () => new Key.Insert()],
  ['\x1b[3~', () => new Key.Delete()],
  ['\x1b[5~', () => new Key.PageUp()],
  ['\x1b[6~', () => new Key.PageDown()],
  ['\x1bOP', () => new Key.F({ value: 1 })],
  ['\x1bOQ', () => new Key.F({ value: 2 })],
  ['\x1bOR', () => new Key.F({ value: 3 })],
  ['\x1bOS', () => new Key.F({ value: 4 })],
  ['\x1b[15~', () => new Key.F({ value: 5 })],
  ['\x1b[17~', () => new Key.F({ value: 6 })],
  ['\x1b[18~', () => new Key.F({ value: 7 })],
  ['\x1b[19~', () => new Key.F({ value: 8 })],
  ['\x1b[20~', () => new Key.F({ value: 9 })],
  ['\x1b[21~', () => new Key.F({ value: 10 })],
  ['\x1b[23~', () => new Key.F({ value: 11 })],
  ['\x1b[24~', () => new Key.F({ value: 12 })],
  ['\x1b', () => new Key.Escape()],
  ['\r', () => new Key.Enter()],
  ['\n', () => new Key.Enter()],
  ['\t', () => new Key.Tab()],
  ['\x7f', () => new Key.Backspace()],
  ['\b', () => new Key.Backspace()]
]);

export function toKey(input: string): Key {
  const key = SEQUENCES.get(input);

  if (key) return key();

  const character = String.fromCodePoint(input.codePointAt(0) ?? 0x20);

  return new Key.Char({ value: character });
}
