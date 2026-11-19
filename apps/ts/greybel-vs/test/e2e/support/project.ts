import * as vscode from 'vscode';

import { fixture } from './workspace';

/** The fixture folder's `greybel.json`. */
export const MANIFEST = 'greybel.json';

/** The fixture folder's `greybel.json`, or `undefined` while there is none or it is being written. */
export async function readManifest(): Promise<unknown> {
  const text = await vscode.workspace.fs.readFile(fixture(MANIFEST)).then(
    (bytes) => new TextDecoder().decode(bytes),
    () => undefined
  );

  try {
    return text === undefined ? undefined : JSON.parse(text);
  } catch {
    return undefined;
  }
}

export async function writeManifest(manifest: unknown): Promise<void> {
  await vscode.workspace.fs.writeFile(fixture(MANIFEST), new TextEncoder().encode(JSON.stringify(manifest)));
}

export async function removeManifest(): Promise<void> {
  await vscode.workspace.fs.delete(fixture(MANIFEST)).then(undefined, () => undefined);
}

/** The `build.obfuscation` of a greybel.json's `manifest`. */
export function obfuscation(manifest: unknown): unknown {
  return (manifest as { build?: { obfuscation?: unknown } } | undefined)?.build?.obfuscation;
}
