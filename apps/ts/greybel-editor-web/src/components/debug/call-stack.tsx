import type { StackFrame } from 'greybel-runtime';

import type { RuntimeSession } from '../../runtime/session.js';

export interface CallStackConfiguration {
  readonly session: RuntimeSession;
  /** Innermost first. */
  readonly frames: readonly StackFrame[];
  readonly frameIndex: number;
  readonly onSelect: (index: number) => void;
}

export function CallStack({ session, frames, frameIndex, onSelect }: CallStackConfiguration) {
  return (
    <ol class="call-stack">
      {frames.map((frame, index) => (
        <li key={frame.id}>
          <button
            type="button"
            class={`call-frame${index === frameIndex ? ' current' : ''}`}
            aria-current={index === frameIndex}
            onClick={() => onSelect(index)}
          >
            <span class="frame-name">{frame.name}</span>
            {frame.location && <span class="frame-location">{session.describeLine(frame.location)}</span>}
          </button>
        </li>
      ))}
    </ol>
  );
}
