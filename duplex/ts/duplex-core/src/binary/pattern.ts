const START_STATE_OFFSET = 0;
const STATE_COUNT_OFFSET = 2;
const SLOT_COUNT_OFFSET = 6;
const CLASS_TABLE_OFFSET = 10;
const CLASS_TABLE_LENGTH = 256;

export class Pattern {
  private constructor(
    private readonly byteClasses: Uint8Array,
    private readonly start: number,
    private readonly base: Uint32Array,
    private readonly next: Uint16Array,
    private readonly owner: Uint16Array,
    private readonly accepting: Uint8Array
  ) {}

  static decode(base64: string): Pattern {
    const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
    const view = new DataView(bytes.buffer);
    const states = view.getUint32(STATE_COUNT_OFFSET, true);
    const slots = view.getUint32(SLOT_COUNT_OFFSET, true);
    let offset = CLASS_TABLE_OFFSET + CLASS_TABLE_LENGTH;
    const readTable = (count: number, width: number, read: (at: number) => number): number[] => {
      const values = Array.from({ length: count }, (_, index) => read(offset + index * width));

      offset += count * width;

      return values;
    };

    const base = Uint32Array.from(readTable(states, 4, (at) => view.getUint32(at, true)));
    const next = Uint16Array.from(readTable(slots, 2, (at) => view.getUint16(at, true)));
    const owner = Uint16Array.from(readTable(slots, 2, (at) => view.getUint16(at, true)));

    return new Pattern(
      bytes.slice(CLASS_TABLE_OFFSET, CLASS_TABLE_OFFSET + CLASS_TABLE_LENGTH),
      view.getUint16(START_STATE_OFFSET, true),
      base,
      next,
      owner,
      bytes.slice(offset, offset + states)
    );
  }

  matches(bytes: Uint8Array, start: number, end: number): boolean {
    let state = this.start;

    for (let index = start; index < end; index++) {
      const slot = this.base[state]! + this.byteClasses[bytes[index]!]!;

      if (this.owner[slot] !== state) return false;
      state = this.next[slot]!;
    }

    return this.accepting[state] === 1;
  }
}
