import { PageRouter } from '../page.generated.js';
import { Key } from '../protocol.generated.js';
import { KeyMap } from './key-map.js';

export class KeyboardInput extends PageRouter {
  readonly #target: EventTarget;

  constructor(target: EventTarget) {
    super();
    this.#target = target;
  }

  override onReadKey(): Promise<Key> {
    return this.#readUntil((key) => key);
  }

  override onReadLine(): Promise<string> {
    let line: string[] = [];

    return this.#readUntil((key) => {
      if (key instanceof Key.Enter) return line.join('');
      if (key instanceof Key.Char) line.push(key.value);
      if (key instanceof Key.Backspace) line = line.slice(0, -1);

      return undefined;
    });
  }

  #readUntil<T>(until: (key: Key) => T | undefined): Promise<T> {
    return new Promise((resolve) => {
      const listener = (event: Event) => {
        const key = KeyMap.of(event as KeyboardEvent);

        if (key === undefined) return;
        event.preventDefault();

        const value = until(key);

        if (value === undefined) return;
        this.#target.removeEventListener('keydown', listener);
        resolve(value);
      };

      this.#target.addEventListener('keydown', listener);
    });
  }
}
