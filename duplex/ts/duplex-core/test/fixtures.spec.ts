import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { Ajv2020 } from 'ajv/dist/2020.js';
import { afterEach, beforeEach, describe, expect, inject, it, vi } from 'vitest';

import { FrameKind } from '../src/definition.generated.js';
import { BinaryReader, BinaryWriter, ChannelError, ErrorCode, Role } from '../src/index.js';
import * as client from './calculator.client.generated.js';
import { DivisionFailure, Phase } from './calculator.generated.js';
import * as provider from './calculator.provider.generated.js';
import { DefaultLayout, Example, GREETING, PRECISION, Shape, Signal } from './layouts.generated.js';
import { Credentials } from './login.generated.js';
import * as member from './login.member.generated.js';
import {
  addFrame,
  connectedClient,
  connectedProvider,
  CONNECTION,
  Logger,
  MemoryDriver,
  NO_CHECKS,
  restoreMocksAndTimers,
  settle,
  zeroRandomValues
} from './support/channel.js';
import {
  ALIASED_BATCH,
  BATCH,
  bodies,
  Decliner,
  DEFAULTS_OF_MINOR_0,
  FAILURE_BATCH,
  fixtures,
  layouts,
  MALFORMED_TEXT_BODY,
  OPEN_BATCH,
  read,
  SECURE_FIXTURES
} from './support/fixtures.js';
import { decode, hex, unhex, written } from './support/frames.js';

describe('defaults', () => {
  it('should fill the defaults of a struct', () => {
    const defaults = new DefaultLayout({ offset: 5 });

    expect([defaults.precision, defaults.greeting, defaults.signal, defaults.ratio, defaults.offset]).toEqual([
      PRECISION,
      GREETING,
      Signal.Kill,
      0.5,
      5
    ]);
    expect([defaults.note, defaults.initial, defaults.loud, defaults.level]).toEqual([null, 'a', true, 7]);
  });

  it('should read a field a peer of an older minor left out as its default', () => {
    const writer = new BinaryWriter();

    writer.minor = 0;
    new DefaultLayout({ level: 1, precision: 4 }).write(writer);

    const reader = new BinaryReader(writer.asBytes());

    reader.minor = 0;

    const read = DefaultLayout.read(reader);

    expect([read.level, read.precision]).toEqual([7, 4]);
  });

  it('should write for minor 0 as the crate does', () => {
    const writer = new BinaryWriter();

    writer.minor = 0;
    new DefaultLayout({}).write(writer);
    expect(hex(writer.asBytes())).toBe(fixtures[DEFAULTS_OF_MINOR_0]);
  });
});

describe('binary layouts', () => {
  it.each(layouts)('should lay out %s as the crate does', (name, value, layout) => {
    const bytes = written((writer) => value.write(writer));

    expect(bytes).toBe(fixtures[name]);
    expect(read(layout, bytes)).toEqual(value);
  });

  it('should lay out message bodies as the crate does', () => {
    const written = Object.entries(bodies).map(([name, body]) => [name, body()]);

    for (const [name, body] of written) expect(body, name).toBe(fixtures[name!]);
  });

  it('should describe its fixtures as their schema says', () => {
    const schema = JSON.parse(readFileSync(join(inject('duplex'), 'schema/fixtures.schema.json'), 'utf8'));
    const validate = new Ajv2020().compile(schema);
    const valid = validate(fixtures);

    expect(validate.errors ?? []).toEqual([]);
    expect(valid).toBe(true);
  });

  it('should refuse to write a text with half a surrogate pair', () => {
    const writer = new BinaryWriter();
    const writing = () => writer.string('a\ud800b');

    expect(writing).toThrow(TypeError);
  });

  it('should refuse a text that is not UTF-8', () => {
    const reader = new BinaryReader(unhex(fixtures[MALFORMED_TEXT_BODY]!));
    const reading = () => client.Log.read(reader);

    expect(reading).toThrow();
  });

  it('should cover every conformance fixture', () => {
    const covered = [
      ...layouts.map(([name]) => name),
      DEFAULTS_OF_MINOR_0,
      ...Object.keys(bodies),
      BATCH,
      ALIASED_BATCH,
      FAILURE_BATCH,
      OPEN_BATCH,
      MALFORMED_TEXT_BODY,
      ...SECURE_FIXTURES
    ].sort();

    expect(covered).toEqual(Object.keys(fixtures).sort());
  });

  it('should read every optional state back', () => {
    const notes = ['progressBody', 'progressAbsentBody', 'progressNullBody'].map(
      (name) => read(provider.Progress, fixtures[name]!).note
    );

    expect(notes).toEqual(['almost', undefined, null]);
  });

  it('should refuse to read a record key named __proto__', () => {
    const counts: Record<string, number> = {};

    Object.defineProperty(counts, '__proto__', { value: 7, enumerable: true, writable: true, configurable: true });

    const sent = new provider.Progress({
      phase: Phase.Active,
      doneShare: null,
      steps: [],
      counts,
      note: undefined,
      eta: undefined
    });
    const writer = new BinaryWriter();

    sent.write(writer);

    const reading = () => provider.Progress.read(new BinaryReader(writer.asBytes()));

    expect(reading).toThrow('a record key is `__proto__`');
  });

  it('should reject flags no field uses', () => {
    const reading = () => read(Example, '020000');

    expect(reading).toThrow('invalid flags');
  });

  it('should reject an index that tags no variant or member', () => {
    const variant = () => read(Shape, '03');
    const member = () => read({ read: (reader) => Phase.read(reader) }, '03');

    expect(variant).toThrow('no variant 3 of `Shape`');
    expect(member).toThrow('no member 3 of `Phase`');
  });

  it('should write members as their given values, and reject a value no member has', () => {
    const bytes = written((writer) => Signal.write(writer, Signal.Kill));
    const unknown = () => read({ read: (reader) => Signal.read(reader) }, '03');

    expect(bytes).toBe('09');
    expect(unknown).toThrow('no member 3 of `Signal`');
  });

  it('should write and read every 64-bit integer, as Rust does', () => {
    const extremes: [string, (writer: BinaryWriter) => void, (reader: BinaryReader) => bigint, bigint][] = [
      ['ffffffffffffffffff01', (writer) => writer.u64(2n ** 64n - 1n), (reader) => reader.u64(), 2n ** 64n - 1n],
      ['ffffffffffffffffff01', (writer) => writer.i64(-(2n ** 63n)), (reader) => reader.i64(), -(2n ** 63n)],
      ['feffffffffffffffff01', (writer) => writer.i64(2n ** 63n - 1n), (reader) => reader.i64(), 2n ** 63n - 1n],
      ['ffffffffffffffff', (writer) => writer.fixed64(2n ** 64n - 1n), (reader) => reader.fixed64(), 2n ** 64n - 1n]
    ];

    for (const [bytes, write, readValue, value] of extremes) {
      expect(written(write)).toBe(bytes);
      expect(read({ read: readValue }, bytes)).toBe(value);
    }
  });

  it('should write and read 64-bit integers either side of where they stop fitting a number', () => {
    const zigzag = (value: bigint) => (value < 0n ? -value * 2n - 1n : value * 2n);
    const varuint = (value: bigint) => {
      let hex = '';

      for (; value >= 0x80n; value >>= 7n) hex += ((value & 0x7fn) | 0x80n).toString(16).padStart(2, '0');

      return hex + value.toString(16).padStart(2, '0');
    };

    const values = [2n ** 47n, 2n ** 48n, 2n ** 52n, 2n ** 53n, 2n ** 56n]
      .flatMap((edge) => [edge - 1n, edge, edge + 1n])
      .flatMap((value) => [value, -value]);

    for (const value of values) {
      const bytes = varuint(zigzag(value));

      expect(written((writer) => writer.i64(value))).toBe(bytes);
      expect(read({ read: (reader) => reader.i64() }, bytes)).toBe(value);
    }

    expect(() => new BinaryWriter().i64(5 as unknown as bigint)).toThrow(RangeError);
  });

  it('should refuse integers beyond their kind', () => {
    const writer = new BinaryWriter();

    expect(() => writer.u16(65_536)).toThrow(RangeError);
    expect(() => writer.u64(2n ** 64n)).toThrow(RangeError);
    expect(() => writer.i64(2n ** 63n)).toThrow(RangeError);
    expect(() => writer.fixed64(-1n)).toThrow(RangeError);
    expect(() => read({ read: (reader) => reader.u64() }, 'ffffffffffffffffff02')).toThrow(RangeError);
  });

  it('should zigzag signed integers', () => {
    const values = [-1n, 1n, -64n, 64n, -(2n ** 52n) - 3n].map((value) => written((writer) => writer.i64(value)));

    expect(values.slice(0, 4)).toEqual(['01', '02', '7f', '8001']);
    expect(read({ read: (reader) => reader.i64() }, values[4]!)).toBe(-(2n ** 52n) - 3n);
  });

  it('should refuse a text of more than one code point as a character', () => {
    expect(() => new BinaryWriter().char('ab')).toThrow(RangeError);
    expect(() => new BinaryWriter().char('\ud800')).toThrow(RangeError);
  });

  it('should write a length of several bytes in front of what it measures', () => {
    const writer = new BinaryWriter();

    writer.byte(0);
    writer.raw(new Uint8Array(300).fill(7));
    writer.finishLength(0);

    const bytes = writer.asBytes();

    expect(hex(bytes.subarray(0, 3))).toBe('ac0207');
    expect(bytes.length).toBe(302);
  });
});

describe('binary batches', () => {
  beforeEach(zeroRandomValues);

  afterEach(restoreMocksAndTimers);

  it('should post a notification as the crate does', async () => {
    const { driver, user } = await connectedClient();

    user.send(new client.Log({ text: 'adding' }));
    await settle();
    expect(driver.posted.map((posted) => hex(posted as Uint8Array))).toEqual([fixtures[BATCH]]);
  });

  it('should alias the next batch of a connection as the crate does', async () => {
    const { driver, user } = await connectedClient();

    user.send(new client.Log({ text: 'adding' }));
    await settle();
    driver.posted.splice(0);
    user.send(new client.Log({ text: 'adding' }));
    await settle();
    expect(driver.posted.map((posted) => hex(posted))).toEqual([fixtures[ALIASED_BATCH]]);
  });

  it('should read the alias a batch of the crate announced', async () => {
    const { driver, calculator } = await connectedProvider();
    const logger = new Logger();

    calculator.use(logger);
    driver.dispatch(unhex(fixtures[BATCH]!));
    driver.dispatch(unhex(fixtures[ALIASED_BATCH]!));
    await settle();
    expect(logger.logged).toEqual(['adding', 'adding']);
  });

  it('should read the frames of a batch of the crate', () => {
    const batch = decode(unhex(fixtures[BATCH]!));

    expect(batch.from).toBe(Role.Client);
    expect(batch.frames).toEqual([{ kind: 0, typeId: 0, id: undefined, body: fixtures.logBody }]);
  });

  it('should fail requests and close as the crate does', async () => {
    const { driver, calculator } = await connectedProvider();

    vi.spyOn(driver, 'defer').mockImplementation(() => undefined);
    calculator.use(new Decliner());

    const divide = written((writer) => new client.Divide({ dividend: 1n, divisor: 2n }).write(writer));

    driver.deliver(
      Role.Provider,
      CONNECTION,
      { kind: FrameKind.Message, typeId: 2, id: 0, body: divide },
      addFrame(1, 1n, 2n)
    );
    await new Promise((resolve) => setTimeout(resolve));
    calculator.close();
    expect(driver.posted.map((posted) => hex(posted))).toEqual([fixtures[FAILURE_BATCH]]);
  });

  it('should open as the crate does', async () => {
    const driver = new MemoryDriver();

    new member.MemberChannel(driver, new Credentials({ user: 'ada', token: 't0k' }), NO_CHECKS);
    await settle();
    expect(driver.posted.map((posted) => hex(posted))).toEqual([fixtures[OPEN_BATCH]]);
  });

  it('should read the failures of a batch of the crate', async () => {
    const { driver, user } = await connectedClient();
    const declined = user.send(new client.Divide({ dividend: 1n, divisor: 2n }));
    const unhandled = user.send(new client.Add({ a: 1n, b: 2n }));

    driver.dispatch(unhex(fixtures[FAILURE_BATCH]!));
    await expect(declined).rejects.toEqual(ChannelError.fail(DivisionFailure.Refusal));
    await expect(unhandled).rejects.toEqual(ChannelError.reject(ErrorCode.UnhandledMessage));
    expect(user.closeReason).toBe(ErrorCode.ClosedChannel);
  });
});
