/** Project files keyed by paths such as `lib/util.src`, plus the entry file. */
export interface ProjectContent {
  readonly entry: string;
  readonly files: ReadonlyMap<string, string>;
}

/** Initial file for a new project. */
export const DEFAULT_PATH = 'main.src';
