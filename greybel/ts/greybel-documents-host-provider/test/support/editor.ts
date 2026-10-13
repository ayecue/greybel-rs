import { onTestFinished } from 'vitest';
import { MessageTargetDriver } from 'duplex-driver-worker';

import { EditorChannel, MemoryEditor } from '../../src/index.js';
import { ServerChannel } from '../server.generated.js';

export const FILES: Array<[string, string]> = [
  ['file:///work/main.src', 'print 1'],
  ['file:///work/lib/a.src', 'a = 1'],
  ['file:///work/notes.txt', 'notes']
];

export function connect(files: Array<[string, string]>): ServerChannel {
  const { port1, port2 } = new MessageChannel();

  onTestFinished(() => {
    port1.close();
    port2.close();
  });
  new EditorChannel(new MessageTargetDriver(port1), { healthCheck: false }).use(new MemoryEditor(files));

  return new ServerChannel(new MessageTargetDriver(port2), { healthCheck: false });
}
