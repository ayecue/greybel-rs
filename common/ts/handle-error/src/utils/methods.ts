import { ERROR_HANDLER_CALLBACK, type Ctx, type CtxCallback } from "../types.js";

function isTargetCompatible(target: unknown): target is Ctx {
  if (target === null) return false;
  const targetType = typeof target;

  switch (targetType) {
    case "object":
    case "function":
      return true;
    default:
      return false;
  }
}

export function add(target: unknown, handler: CtxCallback): void {
  if (!isTargetCompatible(target)) {
    throw new TypeError("Target must be an object or function");
  }

  Object.defineProperty(target, ERROR_HANDLER_CALLBACK, {
    value: handler,
    writable: true,
    enumerable: false,
    configurable: true,
  });
}

export function invoke(target: unknown, error: unknown): boolean {
  if (!isTargetCompatible(target)) {
    return false;
  }

  return target[ERROR_HANDLER_CALLBACK]?.(error) ?? false;
}

export function remove(target: unknown): void {
  if (!isTargetCompatible(target)) {
    throw new TypeError("Target must be an object or function");
  }

  target[ERROR_HANDLER_CALLBACK] = undefined;
  delete target[ERROR_HANDLER_CALLBACK];
}

export function has(target: unknown): boolean {
  if (!isTargetCompatible(target)) {
    return false;
  }

  return typeof target[ERROR_HANDLER_CALLBACK] === "function";
}

