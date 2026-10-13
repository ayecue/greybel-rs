export type BenchCase = {
  readonly name: string;

  readonly setup: () => () => void;
};

export type BenchGroup = {
  readonly name: string;
  readonly cases: readonly BenchCase[];
};
