export type Emitted = [event: string | symbol, error: unknown, origin: unknown];

export interface FakeProcess {
  readonly process: {
    emit(event: string | symbol, error: unknown, origin: unknown): boolean;
  };
  readonly emitted: Emitted[];
}


export function fakeProcess(
  onEmit?: (event: string | symbol) => void,
): FakeProcess {
  const emitted: Emitted[] = [];
  return {
    emitted,
    process: {
      emit(event, error, origin) {
        emitted.push([event, error, origin]);
        onEmit?.(event);
        return true;
      },
    },
  };
}
