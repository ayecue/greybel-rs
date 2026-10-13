export function withGlobal<T>(name: string, value: unknown, run: () => T): T {
  const original = Object.getOwnPropertyDescriptor(globalThis, name);
  Object.defineProperty(globalThis, name, {
    value,
    configurable: true,
    writable: true,
  });

  try {
    return run();
  } finally {
    if (original === undefined) Reflect.deleteProperty(globalThis, name);
    else Object.defineProperty(globalThis, name, original);
  }
}
