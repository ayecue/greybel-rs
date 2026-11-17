import { CompletionContext, type CompletionResult, type CompletionSource } from '@codemirror/autocomplete';
import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';

import { ManifestHelp } from '../src/editor/manifest-help.js';
import schemaText from '../src/greybel.schema.json?raw';
import type { JsonObject } from '../src/project/json.js';

const help = new ManifestHelp(JSON.parse(schemaText) as JsonObject);

/** `|` marks the cursor. */
async function getCompletions(text: string): Promise<string[]> {
  const pos = text.indexOf('|');
  const state = EditorState.create({ doc: text.replace('|', ''), extensions: help.extension() });
  const sources = state.languageDataAt<CompletionSource>('autocomplete', pos);
  const results = await Promise.all(sources.map((source) => source(new CompletionContext(state, pos, true))));

  return results
    .filter((result): result is CompletionResult => result !== null)
    .flatMap((result) => result.options.map((option) => option.label));
}

describe('the editing help of greybel.json', () => {
  it('should complete the names of the top-level settings it has no value for', async () => {
    const names = await getCompletions('{\n  "run": {},\n  |\n}');

    expect(names).toContain('build');
    expect(names).toContain('strictMode');
    expect(names).not.toContain('run');
  });

  it('should complete the names of nested settings and the names of a profile', async () => {
    expect(await getCompletions('{ "run": { "seed": "x", "| } }')).toContain('params');
    expect(await getCompletions('{ "run": { "seed": "x", | } }')).not.toContain('seed');
    expect(await getCompletions('{ "run": { "|')).toContain('seed');
    expect(await getCompletions('{ "profiles": { "fast": { "build": { | } } } }')).toContain('obfuscation');
  });

  it('should complete the values a setting takes', async () => {
    expect(await getCompletions('{ "build": { "mode": | } }')).toEqual(['"Default"', '"Uglify"', '"Beautify"']);
    expect(await getCompletions('{ "strictMode": | }')).toEqual(['true', 'false']);
    expect(await getCompletions('{ "diagnostics": { "unused-local": "|" } }')).toContain('"off"');
  });

  it('should complete diagnostic codes and syntax features as names', async () => {
    expect(await getCompletions('{ "diagnostics": { | } }')).toContain('unused-local');
    expect(await getCompletions('{ "syntax": { "| } }')).toContain('ternary');
  });
});
