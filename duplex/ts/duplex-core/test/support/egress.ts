import { FrameReader, type RawFrame } from '../../src/frame/reader.js';
import { Egress } from '../../src/transport/egress.js';
import { FrameKind } from '../../src/definition.generated.js';
import { Bundle, Role } from '../../src/index.js';
import { BatchReader } from '../../src/frame/batch.js';
import { InboundAliasTable } from '../../src/transport/aliases.js';
import type { Route } from '../../src/frame/route.js';

export const ROUTE = { from: Role.Client, channel: 1, connection: undefined };

export const OTHER = { from: Role.Client, channel: 2, connection: undefined };

export const BATCH_BYTES = 64 * 1024;

export function write(
  egress: Egress,
  route: Route,
  payload: Uint8Array,
  maxFrame = Number.POSITIVE_INFINITY,
  bundle = Bundle.COMMON
): void {
  const frames = egress.begin(route, bundle, 0, maxFrame);

  try {
    frames.rawFrame(0, payload);
  } finally {
    egress.commitFrame();
  }
}

export function flushAll(egress: Egress): Uint8Array[] {
  const sent: Uint8Array[] = [];

  while (!egress.isEmpty) egress.flush((batch) => sent.push(batch));

  return sent;
}

function* framesOf(sent: readonly Uint8Array[], channel?: number): Generator<{ bytes: Uint8Array; frame: RawFrame }> {
  const aliases = new InboundAliasTable();

  for (const bytes of sent) {
    const batch = BatchReader.of(bytes)!;

    for (let slice = batch.next(); slice !== undefined; slice = batch.next()) {
      const route = aliases.getRoute(slice.key);

      if (channel !== undefined && route?.channel !== channel) continue;

      const frames = new FrameReader(bytes, slice.start, slice.end);

      for (let frame = frames.nextRaw(); frame !== undefined; frame = frames.nextRaw()) yield { bytes, frame };
    }
  }
}

export function payloadsOf(sent: readonly Uint8Array[], channel?: number): Uint8Array[] {
  const payloads: Uint8Array[] = [];
  let fragments: number[] = [];

  for (const { bytes, frame } of framesOf(sent, channel)) {
    if (frame.kind !== FrameKind.Fragment) {
      payloads.push(bytes.subarray(frame.start, frame.end));
      continue;
    }

    fragments.push(...bytes.subarray(frame.start + 1, frame.end));
    if (bytes[frame.start] !== 1) continue;

    const reassembled = Uint8Array.from(fragments);
    const raw = new FrameReader(reassembled, 0, reassembled.length).nextRaw()!;

    payloads.push(reassembled.subarray(raw.start, raw.end));
    fragments = [];
  }

  return payloads;
}
