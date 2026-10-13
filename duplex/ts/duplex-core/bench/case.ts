export interface BenchCase {
  readonly name: string;
  readonly setup: () => (() => unknown) | Promise<() => unknown>;
}

export interface BenchGroup {
  readonly name: string;
  readonly cases: readonly BenchCase[];
}
