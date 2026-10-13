import { ReceiveWindow } from '../../src/channel/streams/receive-window.js';

export function grants(receiveWindow: ReceiveWindow, taken: number, bytesEach: number): number[] {
  const granted: number[] = [];
  const started = receiveWindow.initialGrant();

  if (started > 0) granted.push(started);
  for (let index = 0; index < taken; index++) {
    receiveWindow.onReceive(bytesEach);

    const count = receiveWindow.ackItem();

    if (!(count > 0)) continue;
    granted.push(count);
  }

  return granted;
}
