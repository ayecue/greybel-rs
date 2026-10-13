import { describe } from 'vitest';

import earlyExit from '../eslint/early-exit.js';
import { tester } from './support/rule-tester.js';

describe('early-exit', () => {
  tester.run('early-exit', earlyExit, {
    valid: [
      'function start() { if (!ready) return; run(); }',
      'function start() { if (ready) { run(); } else { wait(); } }',
      'function start() { if (ready) { run(); } log(); }',
      'function value() { if (ready) { return 1; } }',
      'function value() { if (ready) throw new Error(); }',
      'for (const item of items) { if (item.done) break; }',
      'const value = () => (ready ? 1 : 2);',
      'function start() {}'
    ],
    invalid: [
      {
        code: [
          'class Channel {',
          '  close(): void {',
          '    if (this.closeReason === undefined) this.send(Code.Closed);',
          '  }',
          '}'
        ].join('\n'),
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: [
          'function start() {',
          '  prepare();',
          '  if (!ready) {',
          '    const step = next();',
          '    run(step);',
          '  }',
          '}'
        ].join('\n'),
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: 'const start = () => { if (ready && open) run(); };',
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: 'function start() { if (item !== undefined && (item.ready || count > 1)) run(); }',
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: ['for (const item of items) {', '  if (item.ready) {', '    run(item);', '  }', '}'].join('\n'),
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: ['function show() {', '  if (open) {', '    print(`a', 'b`);', '  }', '}'].join('\n'),
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: 'function start() { log(step); if (ready) { const step = 1; run(step); } }',
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: 'function start() { const step = 0; if (ready) { const step = 1; run(step); } }',
        errors: [{ messageId: 'earlyExit' }]
      },
      {
        code: 'function start() { if (ready) /* why */ { run(); } }',
        errors: [{ messageId: 'earlyExit' }]
      }
    ]
  });
});
