export type Callback<TArgs extends unknown[] = unknown[]> = (
  ...args: TArgs
) => void;
export type Disposable = () => void;
export type Handle<TCallback extends Callback<any[]> = Callback> = [
  idx: number,
  callback: TCallback,
  origin: Array<TCallback | undefined> | undefined,
];
