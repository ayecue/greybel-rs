const STORAGE_KEY = 'greybel-editor-layout';

/** Sizes the splitters set, as CSS variables `--<name>-size` on the page, stored in `storage`. */
export class Layout {
  readonly #element: HTMLElement;
  readonly #storage: Storage;

  constructor(element: HTMLElement, storage: Storage) {
    this.#element = element;
    this.#storage = storage;
  }

  /** Call before the first render. */
  restore(): void {
    for (const [name, size] of Object.entries(this.#readSizes())) this.show(name, size);
  }

  show(name: string, size: number): void {
    this.#element.style.setProperty(this.#getPropertyName(name), `${size}px`);
  }

  store(name: string, size: number): void {
    this.#storage.setItem(STORAGE_KEY, JSON.stringify({ ...this.#readSizes(), [name]: size }));
  }

  reset(name: string): void {
    const { [name]: _, ...others } = this.#readSizes();

    this.#element.style.removeProperty(this.#getPropertyName(name));
    this.#storage.setItem(STORAGE_KEY, JSON.stringify(others));
  }

  #getPropertyName(name: string): string {
    return `--${name}-size`;
  }

  #readSizes(): Readonly<Record<string, number>> {
    try {
      const value = JSON.parse(this.#storage.getItem(STORAGE_KEY) ?? '{}') as unknown;

      if (typeof value !== 'object' || value === null) return {};

      return Object.fromEntries(
        Object.entries(value).filter((entry): entry is [string, number] => typeof entry[1] === 'number')
      );
    } catch {
      return {};
    }
  }
}
