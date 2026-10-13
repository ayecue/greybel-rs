export interface MessageEventLike {
  readonly data: unknown;
  readonly ports?: readonly MessagePort[];
}

export type MessageListener = (event: MessageEventLike) => void;

export interface MessageTarget {
  postMessage(message: unknown, transfer?: Transferable[]): void;
  addEventListener(type: 'message', listener: MessageListener): void;
  removeEventListener(type: 'message', listener: MessageListener): void;
  start?(): void;
}
