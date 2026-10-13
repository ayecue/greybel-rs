import { add } from "../utils/methods.js";
import {
  ERROR_HANDLER_CALLBACK,
  type HostCallback,
  type Ctx
} from "../types.js";

enum Mode {
  Microtask = 1,
  NextTick = 2,
  Timeout = 3
}

interface Host {
  readonly process?: { nextTick: (cb: () => void) => void };
  readonly queueMicrotask?: (cb: () => void) => void;
  readonly setTimeout: (cb: () => void, timeout?: number) => void;
}

function evaluateCompatibility(host: Host): number {
  if (typeof host?.queueMicrotask === "function") {
    return Mode.Microtask;
  } else if (typeof host.process?.nextTick === "function") {
    return Mode.NextTick;
  }

  return Mode.Timeout;
}

function invokeMicrotask(host: Host, error: unknown): boolean {
  return host.queueMicrotask!(() => {
    throw error;
  }), true;
}

function invokeNextTick(host: Host, error: unknown): boolean {
  return host.process!.nextTick(() => {
    throw error;
  }), true;
}

function invokeTimeout(host: Host, error: unknown): boolean {
  return host.setTimeout(() => {
    throw error;
  }), true;
}

function createCallback(host: Host, mode: number): HostCallback {
  switch (mode) {
    case Mode.Microtask:
      return (error: unknown): boolean => invokeMicrotask(host, error);
    case Mode.NextTick:
      return (error: unknown): boolean => invokeNextTick(host, error);
    case Mode.Timeout:
      return (error: unknown): boolean => invokeTimeout(host, error);
    default:
      return (): boolean => false;
  }
}

function buildCallback(this: Ctx): Ctx {
  const self = this;
  const host = globalThis as Host;

  add(self, function (error: unknown): boolean {
    const mode = evaluateCompatibility(host);
    const callback = createCallback(host, mode);
    add(self, callback);
    return callback(error);
  });

  return self;
}

export const provider = () => buildCallback.call({});
export const reporter = provider();