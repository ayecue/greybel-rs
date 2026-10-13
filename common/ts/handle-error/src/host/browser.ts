import { invoke, add, remove } from "../utils/methods.js";
import { reporter as defaultReporter } from "./default.js";
import { ERROR_HANDLER_CALLBACK, type HostCallback, type Ctx } from "../types.js";

enum Mode {
  ReportError = 1
}

interface Host {
  readonly reportError?: (throwable: unknown) => void;
}

function tryReportError(host: Host, error: unknown): void {
  try {
    host.reportError?.(error);
  } catch {
    invoke(defaultReporter, error);
  }
}

function evaluateCompatibility(host: Host): number {
  if (typeof host?.reportError === "function") {
    return Mode.ReportError;
  }

  return 0;
}

function invokeReportError(host: Host, error: unknown): boolean {
  return tryReportError(host, error), true;
}

function createCallback(host: Host, mode: number): HostCallback {
  switch (mode) {
    case Mode.ReportError:
      return (error: unknown): boolean => invokeReportError(host, error);
    default:
      return (): boolean => false;
  }
}

function buildCallback(this: Ctx): Ctx {
  const self = this;
  const host = globalThis as Host;

  add(self, function (error: unknown): boolean {
    const mode = evaluateCompatibility(host);

    if (mode !== Mode.ReportError) {
      remove(self);
      return false;
    }

    const callback = createCallback(host, mode);
    add(self, callback);
    return callback(error);
  });

  return self;
}

export const provider = () => buildCallback.call({});
export const reporter = provider();