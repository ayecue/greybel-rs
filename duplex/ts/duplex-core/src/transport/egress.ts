import { BinaryWriter } from '../binary/writer.js';
import { Bundle } from '../channel/options.js';
import { FLUSH_BYTES, MAX_BUFFERED_BYTES, MIN_FRAGMENT_BYTES, NO_LIMIT } from '../const.js';
import { Varuint } from '../binary/varint.js';
import { ErrorCode } from '../definition.generated.js';
import { ChannelError } from '../errors.js';
import { BatchWriter } from '../frame/batch.js';
import { FrameEncoder } from '../frame/encoder.js';
import { Route } from '../frame/route.js';
import { CORE } from '../protocol/index.js';
import { OutboundAliasTable } from './aliases.js';

interface Run {
  readonly route: Route;
  readonly bundle: number;
  end: number;
  lastFrameStart: number;
}

interface PlannedSlice {
  readonly outbox: Outbox;
  readonly route: Route;
  readonly start: number;
  readonly end: number;
  readonly lastFrameStart: number;
  readonly fragment: FragmentPiece;
}

enum FragmentPiece {
  None,
  Continues,
  LastPiece
}

class Outbox {
  readonly frames = new BinaryWriter();
  readonly encoder = new FrameEncoder(this.frames);
  readonly runs: Run[] = [];
  firstRunIndex = 0;
  unsentStart = 0;
  nextPieceStart: number | undefined;
  isReady = false;

  get isEmpty(): boolean {
    return this.firstRunIndex === this.runs.length;
  }

  get frontRun(): Run | undefined {
    return this.runs[this.firstRunIndex];
  }

  append(route: Route, bundle: number, end: number, lastFrameStart: number): void {
    const runs = this.runs;
    const run = runs.length > this.firstRunIndex ? runs[runs.length - 1]! : undefined;

    if (run !== undefined && run.bundle === bundle && Route.equals(run.route, route)) {
      run.end = end;
      run.lastFrameStart = lastFrameStart;

      return;
    }

    runs.push({ route, bundle, end, lastFrameStart: lastFrameStart });
  }

  plan(bundle: number, planner: Planner): void {
    const runs = this.runs;
    const front = runs[this.firstRunIndex];

    if (front === undefined || front.bundle !== bundle) return;
    planner.startOutbox(this);
    if (this.nextPieceStart !== undefined) {
      planner.planNextPiece(front, this.unsentStart, this.nextPieceStart);

      return;
    }

    let start = this.unsentStart;

    for (let index = this.firstRunIndex; index < runs.length; index++) {
      const run = runs[index]!;

      if (run.bundle !== bundle || !planner.planRun(run, start)) return;
      start = run.end;
    }
  }

  recordSentPiece(end: number, isLastPiece: boolean): void {
    if (!isLastPiece) {
      this.nextPieceStart = end;

      return;
    }

    this.nextPieceStart = undefined;
    this.advanceSendPosition(end);
  }

  advanceSendPosition(end: number): void {
    this.unsentStart = end;

    const runs = this.runs;

    while (this.firstRunIndex < runs.length && runs[this.firstRunIndex]!.end <= end) this.firstRunIndex++;
    if (this.firstRunIndex === runs.length) {
      runs.length = 0;
      this.firstRunIndex = 0;
      this.unsentStart = 0;
      this.frames.reset();

      return;
    }

    if (this.unsentStart < FLUSH_BYTES || this.unsentStart * 2 < this.frames.size) return;

    const shift = this.unsentStart;
    const bytes = this.frames.asBytes();

    bytes.copyWithin(0, shift);
    this.frames.truncate(bytes.length - shift);
    runs.splice(0, this.firstRunIndex);
    this.firstRunIndex = 0;
    for (const run of runs) {
      run.end -= shift;
      run.lastFrameStart -= shift;
    }

    if (this.nextPieceStart !== undefined) this.nextPieceStart -= shift;
    this.unsentStart = 0;
  }
}

class Planner {
  readonly slices: PlannedSlice[] = [];
  #sharePerOutbox = 0;
  #pieceSize = 0;
  #outbox: Outbox | undefined;
  #bytes: Uint8Array = new Uint8Array();
  #shareLeft = 0;
  #hasFrames = false;

  startBatch(share: number): void {
    this.slices.length = 0;
    this.#sharePerOutbox = share;
    this.#pieceSize = Math.max(share, MIN_FRAGMENT_BYTES);
  }

  startOutbox(outbox: Outbox): void {
    this.#outbox = outbox;
    this.#bytes = outbox.frames.asBytes();
    this.#shareLeft = this.#sharePerOutbox;
    this.#hasFrames = false;
  }

  planRun(run: Run, start: number): boolean {
    if (run.end - start <= this.#shareLeft) {
      this.#shareLeft -= run.end - start;
      this.#addFrames(run, start, run.end, run.lastFrameStart);

      return this.#shareLeft > 0;
    }

    let end = start;
    let lastFrameStart = -1;

    while (end < run.end) {
      const frameEnd = Varuint.frameEnd(this.#bytes, end);

      if (frameEnd === undefined || frameEnd > run.end) return false;

      const frameSize = frameEnd - end;

      if (frameSize <= this.#shareLeft) {
        this.#shareLeft -= frameSize;
        lastFrameStart = end;
        end = frameEnd;
        continue;
      }

      const isFirst = !this.#hasFrames && lastFrameStart < 0;

      if (frameSize > this.#pieceSize && (isFirst || this.#shareLeft >= MIN_FRAGMENT_BYTES)) {
        if (lastFrameStart >= 0) this.#addFrames(run, start, end, lastFrameStart);
        this.#addPiece(run, end, end + (isFirst ? this.#pieceSize : this.#shareLeft), false);

        return false;
      }

      if (isFirst && frameSize <= this.#pieceSize) {
        this.#shareLeft = 0;
        lastFrameStart = end;
        end = frameEnd;
      }

      break;
    }

    if (lastFrameStart >= 0) this.#addFrames(run, start, end, lastFrameStart);

    return end === run.end && this.#shareLeft > 0;
  }

  planNextPiece(run: Run, frameStart: number, pieceStart: number): void {
    const frameEnd = Varuint.frameEnd(this.#bytes, frameStart);

    if (frameEnd === undefined || frameEnd > run.end) return;

    const pieceEnd = Math.min(pieceStart + this.#pieceSize, frameEnd);

    this.#addPiece(run, pieceStart, pieceEnd, pieceEnd === frameEnd);
  }

  #addFrames(run: Run, start: number, end: number, lastFrameStart: number): void {
    this.slices.push({
      outbox: this.#outbox!,
      route: run.route,
      start,
      end,
      lastFrameStart: lastFrameStart,
      fragment: FragmentPiece.None
    });
    this.#hasFrames = true;
  }

  #addPiece(run: Run, start: number, end: number, isLastPiece: boolean): void {
    const fragment = isLastPiece ? FragmentPiece.LastPiece : FragmentPiece.Continues;

    this.slices.push({ outbox: this.#outbox!, route: run.route, start, end, lastFrameStart: start, fragment });
  }
}

export class Egress {
  readonly aliases = new OutboundAliasTable();
  readonly #outboxes = new Map<number, Outbox>();
  readonly #readyOutboxes: Outbox[] = [];
  readonly #batchWriter = new BinaryWriter();
  readonly #planner = new Planner();
  #pendingBytes = 0;
  #currentOutbox: Outbox | undefined;
  #currentRoute: Route | undefined;
  #currentBundle: number = Bundle.id(Bundle.COMMON);
  #frameStart = 0;
  #maxFrameLength = NO_LIMIT;

  get isFull(): boolean {
    return this.#pendingBytes >= FLUSH_BYTES;
  }

  get isEmpty(): boolean {
    return this.#pendingBytes === 0;
  }

  begin(route: Route, bundle: Bundle, minor: number, maxFrame: number): FrameEncoder {
    if (this.#pendingBytes >= MAX_BUFFERED_BYTES) throw ChannelError.reject(ErrorCode.Overload);

    const outbox = this.#outboxOf(route);

    this.#currentOutbox = outbox;
    this.#currentRoute = route;
    this.#currentBundle = Bundle.id(bundle);
    this.#frameStart = outbox.frames.size;
    this.#maxFrameLength = maxFrame;

    const encoder = outbox.encoder;

    encoder.minor = minor;
    encoder.lastFrameStart = undefined;

    return encoder;
  }

  commitFrame(): boolean {
    const outbox = this.#currentOutbox!;
    const frames = outbox.frames;
    const start = this.#frameStart;
    const frameLength = frames.size - start;

    if (frameLength > this.#maxFrameLength) {
      frames.truncate(start);
      throw ChannelError.reject(ErrorCode.Overload);
    }

    const lastFrameStart = outbox.encoder.lastFrameStart;

    if (lastFrameStart === undefined || lastFrameStart < start || frameLength <= 0) {
      frames.truncate(start);

      return false;
    }

    if (!outbox.isReady) {
      outbox.isReady = true;
      this.#readyOutboxes.push(outbox);
    }

    outbox.append(this.#currentRoute!, this.#currentBundle, frames.size, lastFrameStart);
    this.#pendingBytes += frameLength;

    return true;
  }

  flush(send: (batch: Uint8Array) => void): void {
    const ready = this.#readyOutboxes;

    if (ready.length === 0) return;

    const bundle = ready[0]!.frontRun!.bundle;
    let isSingleBundle = true;

    for (const outbox of ready) {
      for (let index = outbox.firstRunIndex; index < outbox.runs.length; index++) {
        if (outbox.runs[index]!.bundle === bundle) continue;
        isSingleBundle = false;
        break;
      }

      if (!isSingleBundle) break;
    }

    if (isSingleBundle) {
      if (ready.length !== 1 || !this.#sendOutboxInPlace(ready[0]!, bundle, send)) this.#sendBatch(bundle, ready, send);
    } else {
      this.#sendEachBundle(send);
    }

    let retainedLength = 0;

    for (const outbox of ready) {
      outbox.isReady = !outbox.isEmpty;
      if (!outbox.isReady) continue;
      ready[retainedLength++] = outbox;
    }

    ready.length = retainedLength;
  }

  clear(): void {
    this.#outboxes.clear();
    this.#readyOutboxes.length = 0;
    this.#pendingBytes = 0;
    this.aliases.clear();
  }

  #outboxOf(route: Route): Outbox {
    const peerKey = Route.getPeerKey(route);
    let outbox = this.#outboxes.get(peerKey);

    if (outbox === undefined) {
      outbox = new Outbox();
      this.#outboxes.set(peerKey, outbox);
    }

    return outbox;
  }

  #sendEachBundle(send: (batch: Uint8Array) => void): void {
    const bundles: number[] = [];
    const bundleOrder = new Map<number, number>();
    const outboxRank = new Map<Outbox, number>();
    const outboxesByFrontBundle = new Map<number, Outbox[]>();

    this.#readyOutboxes.forEach((outbox, at) => {
      outboxRank.set(outbox, at);
      for (let index = outbox.firstRunIndex; index < outbox.runs.length; index++) {
        const { bundle } = outbox.runs[index]!;

        if (bundleOrder.has(bundle)) continue;
        bundleOrder.set(bundle, bundles.length);
        bundles.push(bundle);
      }

      const front = outbox.frontRun;

      if (front === undefined) return;
      Egress.#addToBundle(outboxesByFrontBundle, front.bundle, outbox);
    });
    for (let at = 0; at < bundles.length; at++) {
      const bundle = bundles[at]!;
      const sharers = outboxesByFrontBundle.get(bundle);

      if (sharers === undefined) continue;
      outboxesByFrontBundle.delete(bundle);
      sharers.sort((a, b) => outboxRank.get(a)! - outboxRank.get(b)!);
      this.#sendBatch(bundle, sharers, send);
      for (const outbox of sharers) {
        const front = outbox.frontRun;

        if (front === undefined || bundleOrder.get(front.bundle)! <= at) continue;
        Egress.#addToBundle(outboxesByFrontBundle, front.bundle, outbox);
      }
    }
  }

  static #addToBundle(fronts: Map<number, Outbox[]>, bundle: number, outbox: Outbox): void {
    const bundleOutboxes = fronts.get(bundle);

    if (bundleOutboxes === undefined) fronts.set(bundle, [outbox]);
    else bundleOutboxes.push(outbox);
  }

  #sendBatch(bundle: number, sharers: readonly Outbox[], send: (batch: Uint8Array) => void): void {
    const planner = this.#planner;

    planner.startBatch(FLUSH_BYTES / sharers.length);
    for (const outbox of sharers) outbox.plan(bundle, planner);

    const slices = planner.slices;
    const batch = new BatchWriter(this.#batchWriter, CORE.major, slices.length);

    for (const slice of slices) {
      const key = this.aliases.assignKey(slice.route);
      const bytes = slice.outbox.frames.asBytes();

      if (slice.fragment === FragmentPiece.None) batch.slice(key, bytes, slice.start, slice.end, slice.lastFrameStart);
      else batch.fragment(key, bytes, slice.start, slice.end, slice.fragment === FragmentPiece.LastPiece);
    }

    for (const slice of slices) {
      this.#pendingBytes -= slice.end - slice.start;
      if (slice.fragment === FragmentPiece.None) slice.outbox.advanceSendPosition(slice.end);
      else slice.outbox.recordSentPiece(slice.end, slice.fragment === FragmentPiece.LastPiece);
    }

    slices.length = 0;
    this.aliases.reclaimAliases();
    send(this.#batchWriter.copyBytes());
  }

  #sendOutboxInPlace(outbox: Outbox, bundle: number, send: (batch: Uint8Array) => void): boolean {
    const run = outbox.frontRun!;
    const start = outbox.unsentStart;

    if (
      run.bundle !== bundle ||
      outbox.runs.length - outbox.firstRunIndex !== 1 ||
      run.end - start > FLUSH_BYTES ||
      outbox.nextPieceStart !== undefined
    ) {
      return false;
    }

    new BatchWriter(this.#batchWriter, CORE.major, 1).slice(
      this.aliases.assignKey(run.route),
      outbox.frames.asBytes(),
      start,
      run.end,
      run.lastFrameStart
    );
    this.#pendingBytes -= run.end - start;
    outbox.advanceSendPosition(run.end);
    this.aliases.reclaimAliases();
    send(this.#batchWriter.copyBytes());

    return true;
  }
}
