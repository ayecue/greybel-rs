import { invoke, remove, add } from "../utils/methods.js";
import { reporter as defaultReporter  } from "./default.js";
import { ERROR_HANDLER_CALLBACK, type HostCallback, type Ctx } from "../types.js";

const ORIGIN = "uncaughtException";

enum Mode {
  Emit = 1
}

interface ProcessLike {
  readonly emit: (eventName: string | symbol, ...args: any[]) => boolean;
  readonly listenerCount?: (eventName: string | symbol) => number;
}

interface Host {
  readonly process?: ProcessLike;
}

function tryEmit(process: ProcessLike, eventName: string | symbol, error: unknown, origin: string): void {
  try {
    process.emit(eventName, error, origin);
  } catch {
    invoke(defaultReporter, error);
  }
}

function evaluateCompatibility(host: Host): number {
  if (
    host?.process &&
    typeof host.process.emit === "function"
  ) {
    return Mode.Emit;
  }

  return 0;
}

function invokeEmit(host: Host, error: unknown): boolean {
  const process = host.process!;
  if (process.listenerCount?.(ORIGIN) === 0) return invoke(defaultReporter, error);
  tryEmit(process, "uncaughtExceptionMonitor", error, ORIGIN);
  tryEmit(process, "uncaughtException", error, ORIGIN);
  return true;
}

function createCallback(host: Host, mode: number): HostCallback {
  switch (mode) {
    case Mode.Emit:
      return (error: unknown): boolean => invokeEmit(host, error);
    default:
      return (): boolean => false;
  }
}

function buildCallback(this: Ctx): Ctx {
  const self = this;
  const host = globalThis as Host;
  
  self[ERROR_HANDLER_CALLBACK] = function (error: unknown): boolean {
    const mode = evaluateCompatibility(host);

    if (mode !== Mode.Emit) {
      remove(self);
      return false;
    }

    const callback = createCallback(host, mode);
    add(self, callback);
    return callback(error);
  };

  return self;
}

export const provider = () => buildCallback.call({});
export const reporter = provider();