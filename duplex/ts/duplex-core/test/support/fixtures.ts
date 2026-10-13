import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { inject } from 'vitest';

import { BinaryReader, BinaryWriter, ChannelError } from '../../src/index.js';
import * as client from '../calculator.client.generated.js';
import { DivisionFailure, Phase, Quotient, Step } from '../calculator.generated.js';
import * as provider from '../calculator.provider.generated.js';
import {
  CharacterLayout,
  DefaultLayout,
  Example,
  Flags,
  Keyword,
  KeywordLayout,
  NumberLayout,
  PrintLayout,
  Raw,
  Shape,
  Signal,
  SignalLayout
} from '../layouts.generated.js';
import { unhex, written } from './frames.js';

export const fixtures = JSON.parse(readFileSync(join(inject('duplex'), 'conformance/fixtures.json'), 'utf8')) as Record<
  string,
  string
>;

export const BATCH = 'notificationBatch';

export const ALIASED_BATCH = 'aliasedBatch';

export const FAILURE_BATCH = 'failureBatch';

export const OPEN_BATCH = 'openBatch';

export interface Layout<T> {
  read(reader: BinaryReader): T;
}

export function read<T>(layout: Layout<T>, bytes: string): T {
  const reader = new BinaryReader(unhex(bytes));
  const value = layout.read(reader);

  reader.finish();

  return value;
}

export function progress(note?: string | null): provider.Progress {
  return new provider.Progress({
    phase: Phase.Active,
    doneShare: 0.75,
    steps: [
      new Step({ name: 'exact', result: new Quotient.Exact({ value: 2n }) }),
      new Step({ name: 'remainder', result: new Quotient.WithRemainder({ value: 3n, remainder: 1n }) }),
      new Step({ name: 'waiting', result: null })
    ],
    counts: { done: 2, total: 3 },
    note
  });
}

export const layouts: [string, { write(writer: BinaryWriter): void }, Layout<unknown>][] = [
  ['defaults', new DefaultLayout({}), DefaultLayout],
  ['example', new Example({ text: 'test', count: 300, flag: true }), Example],
  [
    'numbers',
    new NumberLayout({
      u8: 255,
      u16: 65_535,
      u32: 4_000_000_000,
      u64: BigInt(Number.MAX_SAFE_INTEGER),
      i8: -128,
      i16: -32_768,
      i32: -2_147_483_648,
      i64: -BigInt(Number.MAX_SAFE_INTEGER),
      f32: -1.25,
      f64: 0.1
    }),
    NumberLayout
  ],
  [
    'flags',
    new Flags({
      first: true,
      maybe: true,
      nullable: null,
      either: null,
      text: 't',
      second: false,
      third: true,
      fourth: true
    }),
    Flags
  ],
  [
    'otherFlags',
    new Flags({
      first: false,
      nullable: false,
      either: true,
      second: true,
      third: false,
      fourth: false
    }),
    Flags
  ],
  ['characters', new CharacterLayout({ letter: 'é', letters: ['a', null, '😀'] }), CharacterLayout],
  ['printed', new PrintLayout({ text: 'a', newLine: true, replace: false }), PrintLayout],
  ['point', new Shape.Point(), Shape],
  ['circle', new Shape.Circle({ value: 0.5 }), Shape],
  ['rectangle', new Shape.Rectangle({ width: 2, height: 3, filled: true }), Shape],
  ['keywords', new KeywordLayout({ type: Keyword.Match, match: [Keyword.Type] }), KeywordLayout],
  [
    'raw',
    new Raw({
      hash: 0xdeadbeef,
      seed: 2n ** 40n + 5n,
      data: Uint8Array.of(1, 2, 3),
      chunks: [new Uint8Array(), Uint8Array.of(255)]
    }),
    Raw
  ],
  ['signals', new SignalLayout({ first: Signal.Kill, all: [Signal.Hang, Signal.Kill] }), SignalLayout]
];

export const bodies: Record<string, () => string> = {
  logBody: () => written((writer) => new client.Log({ text: 'adding' }).write(writer)),
  progressBody: () => written((writer) => progress('almost').write(writer)),
  progressAbsentBody: () => written((writer) => progress().write(writer)),
  progressNullBody: () => written((writer) => progress(null).write(writer))
};

export const DEFAULTS_OF_MINOR_0 = 'defaultsOfMinor0';

export const MALFORMED_TEXT_BODY = 'malformedTextBody';

export const SECURE_FIXTURES = [
  'secureFirst',
  'secureSecond',
  'secureThird',
  'secureClientBatch',
  'secureProviderBatch'
];

export class Decliner extends provider.CalculatorRouter {
  override onDivide(): Quotient {
    throw ChannelError.fail(DivisionFailure.Refusal);
  }
}
