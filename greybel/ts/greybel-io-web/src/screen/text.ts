import { Output } from '../protocol.generated.js';

enum LineCursor {
  Blank = 0,
  InLine = 1,
  BelowLine = 2
}

export class TextScreen {
  #lines: string[] = [''];
  #cursor = LineCursor.Blank;

  get lines(): readonly string[] {
    return this.#lines;
  }

  get text(): string {
    return this.#lines.join('\n');
  }

  apply(entries: readonly Output[]): void {
    for (const entry of entries) {
      if (entry instanceof Output.Clear) this.clear();
      if (!(entry instanceof Output.Print)) continue;
      this.print(entry);
    }
  }

  print({ text, newLine, replace }: Output.Print): void {
    if (replace) this.#eraseLine();

    const [first, ...remainder] = text.split('\n');

    this.#lines[this.#lines.length - 1] += first;
    this.#lines.push(...remainder);
    if (newLine) this.#lines.push('');
    if (newLine || text.endsWith('\n')) {
      this.#cursor = LineCursor.BelowLine;
    } else if (text !== '' || replace) {
      this.#cursor = LineCursor.InLine;
    }
  }

  clear(): void {
    this.#lines = [''];
    this.#cursor = LineCursor.Blank;
  }

  #eraseLine(): void {
    if (this.#cursor === LineCursor.Blank) return;
    if (this.#cursor === LineCursor.BelowLine && this.#lines.length > 1) this.#lines.pop();
    this.#lines[this.#lines.length - 1] = '';
  }
}
