import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { Ajv2020 } from 'ajv/dist/2020.js';
import { inject, vi } from 'vitest';

import { BinaryWriter } from '../../src/binary/writer.js';
import { Health } from '../../src/channel/connection/health.js';
import { ErrorCode, FrameKind, Requirement, Violation } from '../../src/definition.generated.js';
import { ChannelErrorKind } from '../../src/errors.js';
import {
  ChannelError,
  ConnectionActive,
  ConnectionFailure,
  CORE,
  DEFAULT_HEALTH_CHECK,
  Transport,
  Role,
  Security,
  type ChannelOptions,
  type IncomingStream,
  type Next,
  type ResponseStream,
  type ResponseWriter,
  type StreamWriter,
  type VersionNumber
} from '../../src/index.js';
import { SecurityRole } from '../../src/security/handshake.js';
import * as client from '../calculator.client.generated.js';
import { DivisionFailure, Phase, Quotient } from '../calculator.generated.js';
import { CalculationMigration1 } from '../calculator.migration.generated.js';
import * as provider from '../calculator.provider.generated.js';
import * as v2 from '../calculator.v2.provider.generated.js';
import * as login from '../login.generated.js';
import * as loginMember from '../login.member.generated.js';
import * as service from '../login.service.generated.js';
import * as asker from '../vault.asker.generated.js';
import * as vault from '../vault.generated.js';
import * as keeper from '../vault.keeper.generated.js';
import { MemoryDriver, NO_CHECKS, zeroRandomValues } from './channel.js';
import { decode, encode, encodedFrame, hex, unhex, type TestBatch, type TestFrame } from './frames.js';

export interface ScenarioFile {
  description: string;
  peer: 'client' | 'provider';
  protocol?: 'calculator' | 'login' | 'vault';
  major?: 1 | 2;
  migrates?: boolean;
  open?: { user: string; token: string };
  connected?: boolean;
  healthCheck?: boolean;
  unhandled?: string[];
  limits?: { queued?: number; pending?: number; responding?: number; waiting?: number; message?: number };
  streamWindow?: number;
  unvalidated?: boolean;
  security?: { channel: SecuredEnd; remote: SecuredEnd };
  steps: Json[];
}

interface SecuredEnd {
  identity?: number;
  trusts?: number[];
  signsBatches?: boolean;
}

type Json = any;

const CORE_OFFSET = 0;

type Subject = typeof client.Calculation | typeof login.Login | typeof vault.Vault;

function roleOf(peer: 'client' | 'provider'): Role {
  return peer === 'client' ? Role.Client : Role.Provider;
}

function entry(value: Json): [string, Json] {
  const entries = Object.entries(value as object);

  if (entries.length !== 1) throw new TypeError(`not one entry: ${JSON.stringify(value)}`);

  return entries[0]!;
}

function member<E extends number>(enumeration: object, name: string): E {
  const value = (enumeration as Record<string, unknown>)[name];

  if (typeof value !== 'number') throw new TypeError(`no member ${name}`);

  return value as E;
}

function code(name: string): ErrorCode {
  return member<ErrorCode>(ErrorCode, name);
}

function idOf(message: object): number {
  return (message.constructor as unknown as { id: number }).id;
}

type CalculatorMessage =
  | client.Log
  | client.Add
  | client.Divide
  | client.Reset
  | client.Count
  | client.Sum
  | provider.Confirm
  | provider.Progress;

function message(value: Json, subject: Subject = client.Calculation): CalculatorMessage | login.Whoami {
  const [name, fields] = entry(value);

  if (subject === login.Login) {
    if (name === 'Whoami') return new login.Whoami();
    throw new TypeError(`no message ${name}`);
  }

  switch (name) {
    case 'Log':
      return new client.Log({ text: fields.text });
    case 'Add':
      return new client.Add({ a: BigInt(fields.a), b: BigInt(fields.b) });
    case 'Divide':
      return new client.Divide({
        dividend: BigInt(fields.dividend),
        divisor: BigInt(fields.divisor),
        precision: fields.precision
      });
    case 'Reset':
      return new client.Reset();
    case 'Count':
      return new client.Count({ to: fields.to });
    case 'Sum':
      return new client.Sum();
    case 'Confirm':
      return new provider.Confirm({ question: fields.question });
    case 'Progress':
      return new provider.Progress({
        phase: member<Phase>(Phase, fields.phase),
        doneShare: null,
        steps: [],
        counts: {},
        eta: fields.eta
      });
    default:
      throw new TypeError(`no message ${name}`);
  }
}

const MESSAGES: Record<string, { readonly id: number }> = {
  Log: client.Log,
  Add: client.Add,
  Divide: client.Divide,
  Reset: client.Reset,
  Count: client.Count,
  Sum: client.Sum,
  Confirm: provider.Confirm,
  Progress: provider.Progress
};

function quotient(value: Json): Quotient {
  const [name, fields] = entry(value);

  if (name === 'Exact') return new Quotient.Exact({ value: BigInt(fields.value) });
  if (name === 'WithRemainder')
    return new Quotient.WithRemainder({ value: BigInt(fields.value), remainder: BigInt(fields.remainder) });
  throw new TypeError(`no quotient ${name}`);
}

function quotientJson(value: Quotient): Json {
  if (value instanceof Quotient.WithRemainder)
    return { WithRemainder: { value: Number(value.value), remainder: Number(value.remainder) } };
  if (value instanceof Quotient.Exact) return { Exact: { value: Number(value.value) } };
  throw new TypeError('no quotient');
}

function responseJson(value: unknown): Json {
  return typeof value === 'bigint' ? Number(value) : value;
}

function progressJson(progress: client.Progress): Json {
  const fields: Json = { phase: Phase[progress.phase] };

  if (progress.eta !== undefined) fields.eta = progress.eta;

  return { Progress: fields };
}

function written(minor: number, write: (writer: BinaryWriter) => void): string {
  const writer = new BinaryWriter();

  writer.minor = minor;
  write(writer);

  return hex(writer.asBytes());
}

function versionText(version: VersionNumber): string {
  return `${version.major}.${version.minor}`;
}

function messageOf(fields: Json): [string, Json] {
  const found = Object.entries(fields as object).find(([key]) => key !== 'id');

  if (found === undefined) throw new TypeError(`no message in ${JSON.stringify(fields)}`);

  return found;
}

function frame(value: Json, minor: number, subject: Subject): TestFrame {
  const [kind, fields] = entry(value);

  switch (kind) {
    case 'Message': {
      if (fields.body !== undefined)
        return { kind: FrameKind.Message, typeId: MESSAGES[fields.message]!.id, id: fields.id, body: fields.body };

      const sent = message(Object.fromEntries([messageOf(fields)]), subject);

      return {
        kind: FrameKind.Message,
        typeId: idOf(sent),
        id: fields.id,
        body: written(minor, (writer) => sent.write(writer))
      };
    }

    case 'ResponseEnd': {
      const [name, response] = messageOf(fields);
      const body = written(minor, (writer) => {
        if (name === 'Whoami') login.Whoami.writeResponseEnd(writer, response);
        else if (name === 'Add') client.Add.writeResponseEnd(writer, BigInt(response));
        else if (name === 'Divide') client.Divide.writeResponseEnd(writer, quotient(response));
        else if (name === 'Confirm') provider.Confirm.writeResponseEnd(writer, response);
        else if (name === 'Count') client.Count.writeResponseEnd(writer, BigInt(response));
        else if (name === 'Sum') client.Sum.writeResponseEnd(writer, BigInt(response));
        else throw new TypeError(`${name} has no response`);
      });

      return { kind: FrameKind.ResponseEnd, id: fields.id, body };
    }

    case 'ResponseStart':
      return {
        kind: FrameKind.ResponseStart,
        id: fields.id,
        body: written(minor, (writer) => client.Count.writeResponseStart(writer, member<Phase>(Phase, fields.Count)))
      };
    case 'ResponseItem':
      return {
        kind: FrameKind.ResponseItem,
        id: fields.id,
        body: written(minor, (writer) => client.Count.writeResponseItem(writer, fields.Count))
      };
    case 'MessageItem':
      return {
        kind: FrameKind.MessageItem,
        id: fields.id,
        body: fields.body ?? written(minor, (writer) => client.Sum.writeMessageItem(writer, BigInt(fields.Sum)))
      };
    case 'MessageEnd':
      return { kind: FrameKind.MessageEnd, id: fields.id, body: '' };
    case 'ResponseCredit':
      return { kind: FrameKind.ResponseCredit, id: fields.id, count: fields.count };
    case 'MessageCredit':
      return { kind: FrameKind.MessageCredit, id: fields.id, count: fields.count };
    case 'Cancel':
      return { kind: FrameKind.Cancel, id: fields.id };
    case 'Fragment': {
      const encoded = encodedFrame(frame(fields.of, minor, subject));
      const to = fields.to ?? encoded.length;

      return {
        kind: FrameKind.Fragment,
        ends: to === encoded.length,
        bytes: hex(encoded.subarray(fields.from ?? 0, to))
      };
    }

    case 'Failure': {
      if (fields.rejected !== undefined) {
        const violation =
          fields.violation === undefined ? '' : written(minor, (writer) => violationOf(fields.violation).write(writer));

        return { kind: FrameKind.Failure, id: fields.id, code: code(fields.rejected), body: violation };
      }

      const [name, failure] = messageOf(fields);

      if (name !== 'Divide') throw new TypeError(`${name} declares no failure`);

      const declared = member<DivisionFailure>(DivisionFailure, failure);

      return {
        kind: FrameKind.Failure,
        id: fields.id,
        code: undefined,
        body: written(minor, (writer) => client.Divide.writeFailure(writer, declared))
      };
    }

    case 'Listen':
      return { kind: FrameKind.Listen };
    case 'Ping':
      return { kind: FrameKind.Ping };
    case 'Pong':
      return { kind: FrameKind.Pong };
    case 'Open': {
      const protocol = version(fields.protocol, subject.version);

      return {
        kind: FrameKind.Open,
        id: fields.id,
        core: version(fields.core, CORE),
        protocol,
        fingerprint: fields.fingerprint ?? fingerprint(protocol, subject),
        payload: payload(kind, fields)
      };
    }

    case 'Ack': {
      const protocol = version(fields.protocol, subject.version);

      return {
        kind: FrameKind.Ack,
        id: fields.id,
        connection: fields.connection,
        core: version(fields.core, CORE),
        protocol,
        fingerprint: fields.fingerprint ?? fingerprint(protocol, subject),
        deprecation: fields.deprecation ?? null,
        payload: payload(kind, fields)
      };
    }

    case 'Refuse':
      return {
        kind: FrameKind.Refuse,
        id: fields.id,
        reason: code(fields.reason),
        core: version(fields.core, CORE),
        protocol: version(fields.protocol, subject.version),
        majors: fields.majors ?? [subject.version.major],
        payload: payload(kind, fields)
      };
    case 'Close':
      return { kind: FrameKind.Close, reason: code(fields.reason) };
    case 'Unknown':
      return { kind: fields.kind, payload: fields.payload } as unknown as TestFrame;
    default:
      throw new TypeError(`no frame ${kind}`);
  }
}

function version(text: string | undefined, fallback: VersionNumber): VersionNumber {
  if (text === undefined) return fallback;

  const [major, minor] = text.split('.').map(Number);

  if (major === undefined || minor === undefined) throw new TypeError(`\`${text}\` is no version`);

  return { major, minor };
}

function fingerprint(version: VersionNumber, subject: Subject): number {
  if (version.major !== subject.version.major) return 0;

  return subject.versions[version.minor]?.fingerprint ?? 0;
}

function payload(kind: string, fields: Json): Uint8Array {
  if (fields.payloadBytes !== undefined) return unhex(fields.payloadBytes);
  if (fields.payload === undefined) return new Uint8Array(0);

  const writer = new BinaryWriter();

  if (kind === 'Open') login.LoginHandshake.writeOpen(writer, new login.Credentials(fields.payload));
  else if (kind === 'Ack') login.LoginHandshake.writeAck(writer, new login.Session(fields.payload));
  else login.LoginHandshake.writeRefusal(writer, member<login.Denial>(login.Denial, fields.payload));

  return writer.asBytes();
}

function batch(value: Json, from: Role, subject: Subject): Uint8Array {
  if (value.bytes !== undefined) return unhex(value.bytes);

  const known: TestFrame[] = [];
  const unknown: { at: number; kind: number; payload: string }[] = [];

  for (const item of value.frames as Json[]) {
    const [kind] = entry(item);

    if (kind === 'Unknown') unknown.push({ at: known.length, kind: item.Unknown.kind, payload: item.Unknown.payload });
    else known.push(frame(item, value.minor ?? BinaryWriter.NEWEST, subject));
  }

  const test: TestBatch = {
    channel: value.channel ?? subject.channelId,
    from: value.from === undefined ? from : roleOf(value.from),
    connection: value.connection,
    frames: known
  };
  let bytes = encode(test, unknown);

  if (value.core !== undefined) bytes[CORE_OFFSET] = value.core;
  if (value.truncate !== undefined) bytes = bytes.slice(0, bytes.length - value.truncate);

  return bytes;
}

function outcome(settled: Promise<unknown>, response: (value: any) => Json): () => Json {
  let result: Json = 'pending';

  settled.then(
    (value) => (result = { response: response(value) }),
    (error: unknown) => (result = errorJson(error))
  );

  return () => result;
}

enum PendingKind {
  Value,
  Count,
  Sum
}

type Pending =
  | { kind: PendingKind.Value; outcome: () => Json; abort: AbortController }
  | {
      kind: PendingKind.Count;
      stream: ResponseStream<Phase, bigint, number>;
      start?: Json;
      items: Json[];
      ended?: Json;
    }
  | { kind: PendingKind.Sum; terms: StreamWriter<void, bigint>; outcome: () => Json };

function countingJson(pending: Extract<Pending, { kind: PendingKind.Count }>): Json {
  return {
    ...(pending.start === undefined ? {} : { start: pending.start }),
    items: pending.items,
    ...(pending.ended ?? {})
  };
}

function violationOf(value: Json): Violation {
  return new Violation({
    path: value.path,
    requirement: (Requirement as unknown as Record<string, Requirement>)[value.requirement]!
  });
}

function errorJson(error: unknown): Json {
  if (!(error instanceof ChannelError)) return { error: String(error) };
  if (error.violation !== undefined) {
    return { invalid: { path: [...error.violation.path], requirement: Requirement[error.violation.requirement] } };
  }

  if (error.kind === ChannelErrorKind.Failure)
    return { failed: DivisionFailure[error.failure as DivisionFailure] ?? String(error.failure) };
  if (error.kind === ChannelErrorKind.Unavailable) return 'unsupported';

  const code = ErrorCode[error.code!];

  if (error.kind === ChannelErrorKind.Rejection) return { rejected: code };
  if (error.kind === ChannelErrorKind.ChannelEnd) return { closed: code };

  return { denied: code };
}

async function drain(): Promise<void> {
  for (let round = 0; round < 3; round++) await new Promise((resolve) => setImmediate(resolve));
}

class CalculatorHandlers extends provider.CalculatorRouter {
  constructor(
    private readonly unhandled: string[],
    private readonly received: Json[]
  ) {
    super();
  }

  override onLog(message: provider.Log, next: Next<void>): void {
    if (this.unhandled.includes('Log')) return next() as unknown as void;
    this.received.push({ Log: { text: message.text } });
  }

  override onAdd(message: provider.Add, next: Next<bigint>): bigint | Promise<bigint> {
    if (this.unhandled.includes('Add')) return next();

    return message.a + message.b;
  }

  override async onCount(
    message: provider.Count,
    stream: ResponseWriter<Phase, number>,
    next: Next<bigint>
  ): Promise<bigint> {
    if (this.unhandled.includes('Count')) return next();
    stream.start(Phase.Active);
    for (let number = 1; number <= message.to; number++) await stream.send(number);

    return BigInt((message.to * (message.to + 1)) / 2);
  }

  override async onSum(
    _message: provider.Sum,
    terms: IncomingStream<void, bigint>,
    next: Next<bigint>
  ): Promise<bigint> {
    if (this.unhandled.includes('Sum')) return next();

    let sum = 0n;

    for await (const term of terms) sum += term;

    return sum;
  }

  override onDivide({ dividend, divisor }: provider.Divide, next: Next<Quotient>): Quotient | Promise<Quotient> {
    if (this.unhandled.includes('Divide')) return next();
    if (divisor === 0n) throw ChannelError.fail(DivisionFailure.ByZero);

    const value = dividend / divisor;
    const remainder = dividend % divisor;

    return remainder === 0n ? new Quotient.Exact({ value }) : new Quotient.WithRemainder({ value, remainder });
  }
}

class UserHandlers extends client.UserRouter {
  constructor(
    private readonly unhandled: string[],
    private readonly received: Json[]
  ) {
    super();
  }

  override onProgress(message: client.Progress, next: Next<void>): void {
    if (this.unhandled.includes('Progress')) return next() as unknown as void;
    this.received.push(progressJson(message));
  }

  override onConfirm(message: client.Confirm, next: Next<boolean>): boolean | Promise<boolean> {
    if (this.unhandled.includes('Confirm')) return next();

    return message.question.endsWith('?');
  }
}

class Directory extends service.ServiceRouter {
  constructor(private readonly unhandled: string[]) {
    super();
  }

  override onConnect(open: login.Credentials): login.Session | Promise<login.Session> {
    if (this.unhandled.includes('Open')) throw ChannelError.reject(ErrorCode.UnhandledMessage);
    if (open.user !== 'ada') throw ChannelError.deny(login.Denial.UnknownUser);
    if (open.token !== 't0k') throw ChannelError.deny(login.Denial.WrongToken);

    return new login.Session({ greeting: `hello, ${open.user}` });
  }

  override onWhoami(_message: login.Whoami, next: Next<string>): string | Promise<string> {
    if (this.unhandled.includes('Whoami')) return next();

    return 'ada';
  }
}

class Upgrade extends CalculationMigration1 {
  override migrateAdd(message: client.Add): v2.Add {
    return new v2.Add({ terms: [message.a, message.b] });
  }
}

class CalculatorHandlersOf2 extends v2.CalculatorRouter {
  constructor(
    private readonly unhandled: string[],
    private readonly received: Json[]
  ) {
    super();
  }

  override onLog(message: v2.Log, next: Next<void>): void {
    if (this.unhandled.includes('Log')) return next() as unknown as void;
    this.received.push({ Log: { text: message.text } });
  }

  override onAdd(message: v2.Add, next: Next<bigint>): bigint | Promise<bigint> {
    if (this.unhandled.includes('Add')) return next();

    return message.terms.reduce((sum, term) => sum + term, 0n);
  }

  override async onCount(
    message: v2.Count,
    stream: ResponseWriter<v2.Phase, number>,
    next: Next<bigint>
  ): Promise<bigint> {
    if (this.unhandled.includes('Count')) return next();
    stream.start(v2.Phase.Active);
    for (let number = 1; number <= message.to; number++) await stream.send(number);

    return BigInt((message.to * (message.to + 1)) / 2);
  }

  override async onSum(_message: v2.Sum, terms: IncomingStream<void, bigint>, next: Next<bigint>): Promise<bigint> {
    if (this.unhandled.includes('Sum')) return next();

    let sum = 0n;

    for await (const term of terms) sum += term;

    return sum;
  }

  override onDivide({ dividend, divisor }: v2.Divide, next: Next<v2.Quotient>): v2.Quotient | Promise<v2.Quotient> {
    if (this.unhandled.includes('Divide')) return next();
    if (divisor === 0n) throw ChannelError.fail(v2.DivisionFailure.ByZero);

    const value = dividend / divisor;
    const remainder = dividend % divisor;

    return remainder === 0n ? new v2.Quotient.Exact({ value }) : new v2.Quotient.WithRemainder({ value, remainder });
  }
}

type AnyChannel =
  | client.UserChannel
  | provider.CalculatorChannel
  | v2.CalculatorChannel
  | loginMember.MemberChannel
  | service.ServiceChannel
  | asker.AskerChannel
  | keeper.KeeperChannel;

class Keeper extends keeper.KeeperRouter {
  override onReveal(message: keeper.Reveal): string {
    return `the ${message.name}`;
  }
}

interface Paired {
  readonly driver: TaskDriver;
  readonly channel: AnyChannel;
  forge: boolean;
}

function zeros(bytes: Uint8Array): void {
  bytes.fill(0);
}

function key(byte: number): Uint8Array {
  return new Uint8Array(32).fill(byte);
}

class TaskDriver extends MemoryDriver {
  override defer(task: () => void): void {
    setImmediate(task);
  }
}

export class Scenario {
  readonly #driver = new TaskDriver();
  readonly #role: Role;
  readonly #subject: Subject;
  readonly #channel: AnyChannel;
  readonly #received: Json[] = [];
  readonly #deprecated: Json[] = [];
  readonly #sent = new Map<string, Pending>();

  readonly #paired: Paired | undefined;

  private constructor(file: ScenarioFile) {
    this.#role = roleOf(file.peer);
    this.#subject = Scenario.#subjectOf(file.protocol);
    this.#channel = Scenario.#channelOf(
      file,
      this.#role,
      Scenario.#transport(file, this.#role, 'channel', this.#driver),
      this.#received
    );
    this.#channel.onAck((active) => {
      if (active.deprecation === undefined) return;
      this.#deprecated.push({ version: versionText(active.deprecation.version), note: active.deprecation.note });
    });
    if (file.security === undefined) return;

    const remote = this.#role === Role.Client ? Role.Provider : Role.Client;
    const driver = new TaskDriver();
    const channel = Scenario.#channelOf(file, remote, Scenario.#transport(file, remote, 'remote', driver), []);

    this.#paired = { driver, channel, forge: false };
  }

  static #channelOf(file: ScenarioFile, role: Role, transport: Transport | TaskDriver, received: Json[]): AnyChannel {
    const unhandled = file.unhandled ?? [];
    const checks: ChannelOptions = file.healthCheck === true ? { healthCheck: DEFAULT_HEALTH_CHECK } : NO_CHECKS;
    const { queued, pending, responding, waiting, message: maxMessage } = file.limits ?? {};
    const limits = Object.fromEntries(
      Object.entries({
        queuedItems: queued,
        pendingItems: pending,
        activeResponses: responding,
        readyItems: waiting
      }).filter(([, value]) => value !== undefined)
    );
    const options: ChannelOptions = {
      ...checks,
      limits,
      streamWindow: file.streamWindow,
      maxMessage,
      unvalidated: file.unvalidated
    };
    const isClient = role === Role.Client;

    if (file.protocol === 'login') {
      return isClient
        ? new loginMember.MemberChannel(transport, new login.Credentials(file.open!), options)
        : new service.ServiceChannel(transport, options).use(new Directory(unhandled));
    }

    if (file.protocol === 'vault')
      return isClient
        ? new asker.AskerChannel(transport, options)
        : new keeper.KeeperChannel(transport, options).use(new Keeper());
    if (file.major === 2) {
      const channel = new v2.CalculatorChannel(transport, options).use(new CalculatorHandlersOf2(unhandled, received));

      if (file.migrates !== false) channel.addMigration(new Upgrade());

      return channel;
    }

    return isClient
      ? new client.UserChannel(transport, options).use(new UserHandlers(unhandled, received))
      : new provider.CalculatorChannel(transport, options).use(new CalculatorHandlers(unhandled, received));
  }

  static #transport(
    file: ScenarioFile,
    role: Role,
    end: 'channel' | 'remote',
    driver: TaskDriver
  ): Transport | TaskDriver {
    if (file.security === undefined) return driver;

    const { identity, trusts, signsBatches } = file.security[end];
    const security = new Security({
      role: role === Role.Client ? SecurityRole.Initiator : SecurityRole.Responder,
      identity: identity === undefined ? undefined : key(identity),
      verify:
        trusts === undefined
          ? undefined
          : Security.pinnedKeyVerifier(trusts.map((trusted) => Security.publicKey(key(trusted)))),
      signBatches: signsBatches === true,
      entropy: zeros
    });

    return new Transport(driver, { security });
  }

  static async run(file: ScenarioFile): Promise<void> {
    zeroRandomValues();
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout', 'performance'] });
    try {
      const scenario = new Scenario(file);

      await new Promise((resolve) => setImmediate(resolve));
      if (file.connected === true) await scenario.#connect();
      for (const [index, step] of file.steps.entries()) {
        try {
          await scenario.#perform(step);
        } catch (error) {
          throw new Error(`step ${index}, ${JSON.stringify(step)}:\n${(error as Error).message}`, { cause: error });
        }
      }
    } finally {
      vi.useRealTimers();
      vi.restoreAllMocks();
    }
  }

  static #subjectOf(protocol: ScenarioFile['protocol']): Subject {
    if (protocol === 'login') return login.Login;
    if (protocol === 'vault') return vault.Vault;

    return client.Calculation;
  }

  async #perform(step: Json): Promise<void> {
    if (step.expect !== undefined) return this.#expect(step.expect);
    if (step.take !== undefined) return this.#take(step.of, step.take);
    this.#step(step);
  }

  get #remote(): Role {
    return this.#role === Role.Client ? Role.Provider : Role.Client;
  }

  async #connect(): Promise<void> {
    if (this.#paired !== undefined) {
      await this.#pass();
      if (this.#channel.connection === undefined) throw new Error("didn't connect");

      return;
    }

    await drain();
    this.#driver.take();

    const login = this.#subject !== client.Calculation;
    const frames =
      this.#role === Role.Client
        ? [{ Ack: { id: 0, connection: 1, ...(login ? { payload: { greeting: 'hello, ada' } } : {}) } }]
        : [{ Open: { id: 0, ...(login ? { payload: { user: 'ada', token: 't0k' } } : {}) } }];

    this.#receive({ connection: 0, frames });
    await drain();
    this.#driver.take();
    if (this.#channel.connection === undefined) throw new Error("didn't connect");
  }

  async #pass(): Promise<void> {
    const paired = this.#paired;

    if (paired === undefined) return;
    for (;;) {
      await drain();

      const sent = this.#driver.posted.splice(0);

      for (const batch of sent) paired.driver.dispatch(batch);
      await drain();

      const back = paired.driver.posted.splice(0);

      if (paired.forge && back.length > 0) {
        const forged = back[0]!.slice();

        forged[forged.length - 1]! ^= 1;
        back[0] = forged;
        paired.forge = false;
      }

      for (const batch of back) this.#driver.dispatch(batch);
      if (sent.length === 0 && back.length === 0) return;
    }
  }

  #receive(value: Json): void {
    this.#driver.dispatch(batch(value, this.#remote, this.#subject));
  }

  #step(step: Json): void {
    if (step.receive !== undefined) return this.#receive(step.receive);
    if (step.send !== undefined) {
      if (step.as === undefined) return this.#notify(step.send, step.fails);

      return this.#request(step.send, step.as);
    }

    if (step.drop !== undefined) return this.#drop(step.drop);
    if (step.item !== undefined) return this.#item(step.into, step.item);
    if (step.end !== undefined) return this.#end(step.end);
    if (step.check !== undefined) {
      for (let check = 0; check < step.check; check++) vi.advanceTimersByTime(DEFAULT_HEALTH_CHECK.interval);

      return;
    }

    if (step.wait !== undefined) {
      vi.advanceTimersByTime(step.wait);

      return;
    }

    if (step.close !== undefined) return this.#channel.close(code(step.close));
    if (step.forge !== undefined) {
      if (this.#paired === undefined) throw new Error('no remote channel to forge for');
      this.#paired.forge = true;

      return;
    }

    if (step.disconnect !== undefined) return this.#driver.disconnect();
    throw new TypeError('no step');
  }

  #notify(value: Json, fails: Json): void {
    let outcome: Json;

    try {
      const notification = message(value, this.#subject);

      if (this.#channel instanceof v2.CalculatorChannel)
        this.#channel.send(Scenario.#progressOf2(notification as provider.Progress));
      else (this.#channel as client.UserChannel).send(notification as client.Log | client.Reset);
    } catch (error) {
      outcome = errorJson(error);
    }

    if (JSON.stringify(outcome) !== JSON.stringify(fails)) {
      throw new Error(`notified: ${JSON.stringify(outcome)}, expected ${JSON.stringify(fails)}`);
    }
  }

  static #progressOf2(progress: provider.Progress): v2.Progress {
    return new v2.Progress({
      phase: progress.phase as unknown as v2.Phase,
      doneShare: null,
      steps: [],
      counts: {},
      eta: progress.eta
    });
  }

  #request(value: Json, name: string): void {
    if (this.#channel instanceof asker.AskerChannel) {
      const [kind, fields] = entry(value);

      if (kind !== 'Reveal') throw new TypeError(`no message ${kind}`);

      const abort = new AbortController();
      const response = this.#channel.send(new asker.Reveal({ name: fields.name }), { signal: abort.signal });

      this.#sent.set(name, { kind: PendingKind.Value, outcome: outcome(response, (value) => value), abort });

      return;
    }

    const sent = message(value, this.#subject);
    const [kind] = entry(value);
    const abort = new AbortController();
    const options = { signal: abort.signal };

    if (this.#channel instanceof v2.CalculatorChannel) {
      const response = this.#channel.send(new v2.Confirm({ question: (sent as provider.Confirm).question }), options);

      this.#sent.set(name, { kind: PendingKind.Value, outcome: outcome(response, (value) => value), abort });

      return;
    }

    if (sent instanceof client.Count) {
      const stream = (this.#channel as client.UserChannel).send(sent);

      this.#sent.set(name, { kind: PendingKind.Count, stream, items: [] });

      return;
    }

    if (sent instanceof client.Sum) {
      const { stream, response } = (this.#channel as client.UserChannel).send(sent);

      this.#sent.set(name, { kind: PendingKind.Sum, terms: stream, outcome: outcome(response, responseJson) });

      return;
    }

    const response = (this.#channel as client.UserChannel).send(sent as client.Add, options) as Promise<unknown>;
    const json = kind === 'Divide' ? quotientJson : responseJson;

    this.#sent.set(name, { kind: PendingKind.Value, outcome: outcome(response, json), abort });
  }

  async #take(name: string, count: number): Promise<void> {
    const pending = this.#sent.get(name);

    if (pending?.kind !== PendingKind.Count) throw new Error(`\`${name}\` is no stream`);

    const ready = <T>(promise: Promise<T>, what: string) =>
      Promise.race([promise, drain().then(() => Promise.reject(new Error(`no ${what} yet`)))]);

    try {
      if (pending.start === undefined) pending.start = Phase[await ready(pending.stream.start(), 'start')];

      const items = pending.stream[Symbol.asyncIterator]();

      for (let taken = 0; taken < count; taken++) {
        const next = await ready(items.next(), `item ${pending.items.length}`);

        if (next.done === true) {
          pending.ended = { end: responseJson(await ready(pending.stream.end(), 'end')) };

          return;
        }

        pending.items.push(next.value);
      }
    } catch (error) {
      if (error instanceof ChannelError) pending.ended = errorJson(error);
      else throw error;
    }
  }

  #drop(name: string): void {
    const pending = this.#sent.get(name);

    if (pending === undefined) throw new Error(`no message ${name}`);
    this.#sent.delete(name);
    if (pending.kind === PendingKind.Count) pending.stream.cancel();
    else if (pending.kind === PendingKind.Sum) pending.terms.cancel();
    else pending.abort.abort();
  }

  #item(name: string, item: Json): void {
    const pending = this.#sent.get(name);

    if (pending?.kind !== PendingKind.Sum) throw new Error(`\`${name}\` streams nothing`);
    void pending.terms.send(BigInt(item.Sum));
  }

  #end(name: string): void {
    const pending = this.#sent.get(name);

    if (pending?.kind !== PendingKind.Sum) throw new Error(`\`${name}\` streams nothing`);
    pending.terms.end(undefined).catch(() => undefined);
  }

  async #expect(expectation: Json): Promise<void> {
    await drain();
    await this.#pass();
    if (this.#paired !== undefined && expectation.sent !== undefined)
      throw new Error("what a secure transport sends can't be read");

    const failures: string[] = [];
    const differ = (label: string, actual: unknown, expected: unknown) => {
      if (JSON.stringify(actual) === JSON.stringify(expected)) return;
      failures.push(`${label} ${JSON.stringify(actual, null, 1)}, expected ${JSON.stringify(expected, null, 1)}`);
    };

    if (expectation.sent !== undefined) {
      const expected = (expectation.sent as Json[]).map((value) => decode(batch(value, this.#role, this.#subject)));

      differ('sent', this.#driver.take(), expected);
    }

    if (expectation.received !== undefined) differ('received', this.#received.splice(0), expectation.received);
    for (const [name, expected] of Object.entries((expectation.settled ?? {}) as object)) {
      const pending = this.#sent.get(name);

      if (pending === undefined) throw new Error(`no message ${name}`);
      differ(
        `\`${name}\` settled`,
        pending.kind === PendingKind.Count ? countingJson(pending) : pending.outcome(),
        expected
      );
    }

    if (expectation.health !== undefined)
      differ('health', Health[this.#channel.health].toLowerCase(), expectation.health);
    if (expectation.deprecated !== undefined) differ('deprecated', this.#deprecated.splice(0), expectation.deprecated);
    if (expectation.version !== undefined) {
      const version = this.#channel.version;

      differ('version', version === undefined ? null : versionText(version), expectation.version);
    }

    if (expectation.connection !== undefined)
      differ('connection', this.#channel.connection ?? null, expectation.connection);
    if (expectation.ack !== undefined) {
      const state = this.#channel instanceof loginMember.MemberChannel ? this.#channel.state : undefined;
      const ack = state instanceof ConnectionActive ? (state.ack as login.Session | undefined) : undefined;

      differ('ack', ack === undefined ? null : { greeting: ack.greeting }, expectation.ack);
    }

    if (expectation.refusal !== undefined) {
      const state = this.#channel instanceof loginMember.MemberChannel ? this.#channel.state : undefined;
      const refusal = state instanceof ConnectionFailure ? (state.refusal as login.Denial | undefined) : undefined;

      differ('refusal', refusal === undefined ? null : login.Denial[refusal], expectation.refusal);
    }

    if (expectation.closed !== undefined) {
      const reason = this.#channel.closeReason;

      differ('closed', reason === undefined ? null : ErrorCode[reason], expectation.closed);
    }

    if (failures.length > 0) throw new Error(failures.join('\n'));
  }
}

export const SCENARIOS = join(inject('duplex'), 'scenarios');

export const SCHEMA = join(inject('duplex'), 'schema/scenario.schema.json');

export const validate = new Ajv2020({ allErrors: true }).compile(JSON.parse(readFileSync(SCHEMA, 'utf8')));

export const scenarios: [string, ScenarioFile][] = readdirSync(SCENARIOS)
  .filter((file) => file.endsWith('.json'))
  .sort()
  .map((file) => [
    file.slice(0, -'.json'.length),
    JSON.parse(readFileSync(join(SCENARIOS, file), 'utf8')) as ScenarioFile
  ]);
