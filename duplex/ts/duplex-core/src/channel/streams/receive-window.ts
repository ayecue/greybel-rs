import { ASSUMED_ITEM_SIZE, INITIAL_CREDIT, RECEIVE_WINDOW_BYTES } from '../../const.js';

export class ReceiveWindow {
  readonly #maxWindow: number;
  readonly #itemSize: number | undefined;
  #largestItemSize = 0;
  #window: number;
  #creditLeft = INITIAL_CREDIT;

  constructor(maxWindow: number, itemSize: number | undefined) {
    this.#maxWindow = Math.max(maxWindow, 1);
    this.#itemSize = itemSize;
    this.#window = this.#getWindow(itemSize ?? ASSUMED_ITEM_SIZE);
  }

  onReceive(bytes: number): void {
    if (this.#itemSize !== undefined || bytes <= this.#largestItemSize) return;
    this.#largestItemSize = bytes;
  }

  initialGrant(): number {
    return this.#grant();
  }

  ackItem(): number {
    if (this.#creditLeft > 0) this.#creditLeft--;
    if (this.#itemSize === undefined && this.#largestItemSize > 0)
      this.#window = this.#getWindow(this.#largestItemSize);
    if (this.#creditLeft > Math.floor(this.#window / 2)) return 0;

    return this.#grant();
  }

  #grant(): number {
    const count = this.#window - this.#creditLeft;

    if (count <= 0) return 0;
    this.#creditLeft = this.#window;

    return count;
  }

  #getWindow(itemSize: number): number {
    return Math.min(Math.max(Math.floor(RECEIVE_WINDOW_BYTES / Math.max(itemSize, 1)), 1), this.#maxWindow);
  }
}
