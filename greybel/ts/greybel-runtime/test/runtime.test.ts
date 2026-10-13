import { execFileSync } from 'node:child_process';

import { afterEach, beforeAll, describe, expect, inject, it } from 'vitest';

import {
  Continue,
  Exit,
  Scopes,
  SetBreakpoints,
  SourceBreakpoint,
  StackTrace,
  Step,
  StepMode,
  StopReason,
  Variables
} from '../src/index.js';
import {
  BUILD_TIMEOUT,
  EXAMPLE,
  IDLE_TIME,
  launch,
  MAIN,
  PROGRAM,
  children,
  start,
  TIMEOUT_MARGIN
} from './support/runtime.js';

beforeAll(() => {
  execFileSync('cargo', ['build', '--quiet', '--package', 'greybel-runtime', '--example', EXAMPLE], {
    cwd: inject('rust'),
    stdio: 'inherit'
  });
}, BUILD_TIMEOUT);

afterEach(() => {
  for (const child of children.splice(0)) child.kill();
});

describe('the Rust runtime over standard input and output', () => {
  it('should run a program and show what it prints', async () => {
    const { editor, screen } = start([[MAIN, PROGRAM]]);

    const exit = await launch(editor, false);

    expect(exit).toBeInstanceOf(Exit.Completion);
    // The program's prints reach the page in one batch, as the game sends them.
    expect(screen.printedTexts).toEqual(['2\n3']);
  });

  it('should stop at a breakpoint, step into a call and show its variables', async () => {
    const { editor, recorder, screen } = start([[MAIN, PROGRAM]]);

    await editor.send(new SetBreakpoints({ uri: MAIN, breakpoints: [new SourceBreakpoint({ line: 6 })] }));

    const exit = launch(editor, true);

    await recorder.next();
    editor.send(new Step({ mode: StepMode.Into }));
    await recorder.next();

    const frames = await editor.send(new StackTrace());
    const [locals] = await editor.send(new Scopes({ frame: frames[0].id }));
    const variables = await editor.send(new Variables({ reference: locals.reference }));

    editor.send(new Continue());

    expect(recorder.stops.map((stop) => [stop.reason, stop.location?.line])).toEqual([
      [StopReason.Breakpoint, 6],
      [StopReason.Step, 2]
    ]);
    expect(frames.map((frame) => frame.name)).toEqual(['function main.src:2', 'main.src']);
    expect(variables.map((variable) => [variable.name, variable.value])).toEqual([['x', '1']]);
    expect(await exit).toBeInstanceOf(Exit.Completion);
    expect(screen.printedTexts).toEqual(['2\n3']);
  });

  it(
    'should stay connected while the program is stopped for longer than the health checks allow silence',
    async () => {
      const { editor, recorder, screen } = start([[MAIN, PROGRAM]]);

      await editor.send(new SetBreakpoints({ uri: MAIN, breakpoints: [new SourceBreakpoint({ line: 6 })] }));

      const exit = launch(editor, true);

      await recorder.next();
      await new Promise((resolve) => setTimeout(resolve, IDLE_TIME));
      editor.send(new Continue());

      expect(await exit).toBeInstanceOf(Exit.Completion);
      expect(screen.printedTexts).toEqual(['2\n3']);
    },
    IDLE_TIME + TIMEOUT_MARGIN
  );
});
