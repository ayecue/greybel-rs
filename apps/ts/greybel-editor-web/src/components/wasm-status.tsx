import { useEffect, useReducer } from 'preact/hooks';

import { useServices } from './services.js';

/** Renders the component again when a WebAssembly part starts, finishes or fails loading. */
export function usePartChanges(): void {
  const { wasm } = useServices();
  const [, refresh] = useReducer((count: number) => count + 1, 0);

  useEffect(() => wasm.subscribe(() => refresh(undefined)), [wasm]);
}

export function WasmStatus() {
  const { wasm } = useServices();

  usePartChanges();

  return (
    <>
      {wasm.incompleteParts.length > 0 && (
        <span class="loading-status" role="status">
          <span class="spinner" aria-hidden="true" />
          Loading the {wasm.describeIncompleteParts()}...
        </span>
      )}
      {wasm.failures.map(({ part, message }) => (
        <span key={part} class="loading-failure" role="alert">
          Couldn't load the {part}: {message}
        </span>
      ))}
    </>
  );
}
