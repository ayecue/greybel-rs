import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { vi } from "vitest";


export type HandleError = typeof import("../../src/index.js").handleError;


export const cleanups: Array<() => void> = [];


export async function freshHandleError(): Promise<HandleError> {
  vi.resetModules();
  const module = await import("../../src/index.js");
  return module.handleError;
}


export function runFixture(name: string): ReturnType<typeof spawnSync> {
  const fixture = fileURLToPath(new URL(`../fixtures/${name}`, import.meta.url));
  return spawnSync(process.execPath, ["--import", "tsx", fixture], {
    encoding: "utf8",
  });
}
