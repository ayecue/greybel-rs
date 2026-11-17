const STORAGE_KEY = 'greybel-editor-watches';

/** The debugger's watch expressions, stored in `storage`. */
export class WatchList {
  readonly #storage: Storage;
  readonly #listeners = new Set<() => void>();
  #expressions: readonly string[];

  constructor(storage: Storage) {
    this.#storage = storage;
    this.#expressions = this.#read();
  }

  get expressions(): readonly string[] {
    return this.#expressions;
  }

  add(expression: string): void {
    const code = expression.trim();

    if (code === '' || this.#expressions.includes(code)) return;
    this.#write([...this.#expressions, code]);
  }

  remove(expression: string): void {
    this.#write(this.#expressions.filter((other) => other !== expression));
  }

  /** Returns the unsubscribe function. */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);

    return () => this.#listeners.delete(listener);
  }

  #write(expressions: readonly string[]): void {
    this.#expressions = expressions;
    this.#storage.setItem(STORAGE_KEY, JSON.stringify(expressions));
    for (const listener of this.#listeners) listener();
  }

  #read(): readonly string[] {
    try {
      const value = JSON.parse(this.#storage.getItem(STORAGE_KEY) ?? '[]') as unknown;

      return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
    } catch {
      return [];
    }
  }
}
