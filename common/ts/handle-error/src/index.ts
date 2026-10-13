import { reporter as browserReporter } from "./host/browser.js";
import { reporter as nodeReporter } from "./host/node.js";
import { reporter as defaultReporter } from "./host/default.js";
import { invoke, add, remove } from "./utils/methods.js";

const globalReporters = [browserReporter, nodeReporter, defaultReporter];

export function handleError(error: unknown, context: unknown = globalThis): void {
  if (invoke(context, error)) return;
  if (globalReporters.some(r => invoke(r, error))) return;
  Promise.reject(error);
}

handleError.add = add;
handleError.remove = remove;