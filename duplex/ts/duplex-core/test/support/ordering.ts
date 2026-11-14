import { FrameKind } from '../../src/definition.generated.js';
import * as client from '../calculator.client.generated.js';
import { progress } from './channel.js';
import { body, written, type TestFrame } from './frames.js';

export function progressFrame(): TestFrame {
  const notification = progress();

  return { kind: FrameKind.Message, typeId: client.Progress.id, id: undefined, body: body(notification) };
}

export function addResponse(): TestFrame {
  return {
    kind: FrameKind.ResponseEnd,
    id: 0,
    body: written((writer) => client.Add.writeResponseEnd(writer, 3n))
  };
}

export function notificationsThen(count: number, ...last: TestFrame[]): TestFrame[] {
  const frame = progressFrame();

  return [...Array.from({ length: count }, () => frame), ...last];
}
