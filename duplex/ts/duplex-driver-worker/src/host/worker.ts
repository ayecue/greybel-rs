export type WorkerEventType = 'message' | 'error' | 'messageerror';

export interface WorkerLike {
  postMessage(message: unknown, transfer?: Transferable[]): void;
  addEventListener(type: WorkerEventType, listener: (event: Event) => void): void;
  removeEventListener(type: WorkerEventType, listener: (event: Event) => void): void;
  terminate(): void;
}
