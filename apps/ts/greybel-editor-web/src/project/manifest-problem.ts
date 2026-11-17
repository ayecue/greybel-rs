import { MANIFEST_PATH } from './content.js';
import type { PathResolver } from './paths.js';

const MESSAGE = /^(?<uri>[a-z]+:\/\/\S+): (?<text>.*?)(?: at line (?<line>\d+) column (?<column>\d+))?$/s;

export class ManifestProblem {
  readonly path: string;
  readonly message: string;
  readonly line: number;
  readonly column: number;

  constructor(path: string, message: string, line: number, column: number) {
    this.path = path;
    this.message = message;
    this.line = line;
    this.column = column;
  }

  /** Parses `<uri>: <message> at line <n> column <n>`; falls back to line 1 of the root `greybel.json`. */
  static parse(message: string, paths: PathResolver): ManifestProblem {
    const groups = MESSAGE.exec(message)?.groups;
    const uri = groups?.['uri'];

    return new ManifestProblem(
      (uri === undefined ? undefined : paths.toPath(uri)) ?? MANIFEST_PATH,
      groups?.['text'] ?? message,
      Number(groups?.['line'] ?? 1),
      Number(groups?.['column'] ?? 1)
    );
  }

  toString(): string {
    return `${this.path}: ${this.message} (line ${this.line}, column ${this.column})`;
  }
}
