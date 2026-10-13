import * as assert from 'assert';
import * as vscode from 'vscode';

import { DebugRecorder, LOOP, PROGRAM } from './support/debug';
import { TIMEOUT, fixture } from './support/workspace';

describe('debugging in the runtime', function () {
  this.timeout(TIMEOUT);

  let recorder: DebugRecorder;

  before(async () => {
    await vscode.workspace
      .getConfiguration('greybel')
      .update('interpreter.useDefaultArgs', true, vscode.ConfigurationTarget.Workspace);
  });

  after(async () => {
    await vscode.workspace
      .getConfiguration('greybel')
      .update('interpreter.useDefaultArgs', undefined, vscode.ConfigurationTarget.Workspace);
  });

  beforeEach(() => {
    recorder = new DebugRecorder();
  });

  afterEach(() => {
    vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
    recorder.dispose();
  });

  it('should run a program to its end', async () => {
    await recorder.start(fixture(PROGRAM), false);

    await recorder.terminated();

    assert.deepStrictEqual(recorder.stops, []);
  });

  it('should take breakpoints set once the program has ended, unverified', async () => {
    const program = fixture(PROGRAM);

    await recorder.start(program, true);
    await recorder.terminated();

    const { breakpoints } = await recorder.session().customRequest('setBreakpoints', {
      source: { path: program.scheme === 'file' ? program.fsPath : program.toString() },
      breakpoints: [{ line: 3 }]
    });

    assert.deepStrictEqual(
      breakpoints.map((breakpoint: { verified: boolean; line: number }) => [breakpoint.verified, breakpoint.line]),
      [[false, 3]]
    );
  });

  it('should stop at a breakpoint and show the call stack and its variables', async () => {
    const program = fixture(PROGRAM);

    vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(program, new vscode.Position(2, 0)))]);
    await recorder.start(program, true);

    const stop = await recorder.stopped();
    const session = recorder.session();
    const { stackFrames } = await session.customRequest('stackTrace', { threadId: stop.threadId });
    const { scopes } = await session.customRequest('scopes', { frameId: stackFrames[0].id });
    const { variables } = await session.customRequest('variables', {
      variablesReference: scopes[0].variablesReference
    });

    await session.customRequest('continue', { threadId: stop.threadId });
    await recorder.terminated();

    assert.strictEqual(stop.reason, 'breakpoint');
    assert.deepStrictEqual(
      stackFrames.map((frame: { name: string; line: number }) => [frame.name, frame.line]),
      [
        ['function main.src:2', 3],
        ['main.src', 6]
      ]
    );
    assert.deepStrictEqual(
      variables.map((variable: { name: string; value: string }) => [variable.name, variable.value]),
      [
        ['x', '1'],
        ['y', '2']
      ]
    );
  });

  it('should step into a call and out of it', async () => {
    const program = fixture(PROGRAM);

    vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(program, new vscode.Position(5, 0)))]);
    await recorder.start(program, true);

    const stop = await recorder.stopped();
    const session = recorder.session();

    await session.customRequest('stepIn', { threadId: stop.threadId });

    const into = await recorder.stopped();
    const intoFrames = await session.customRequest('stackTrace', { threadId: stop.threadId });

    await session.customRequest('stepOut', { threadId: stop.threadId });

    const out = await recorder.stopped();
    const outFrames = await session.customRequest('stackTrace', { threadId: stop.threadId });

    await session.customRequest('continue', { threadId: stop.threadId });
    await recorder.terminated();

    assert.deepStrictEqual([into.reason, out.reason], ['step', 'step']);
    assert.strictEqual(intoFrames.stackFrames[0].line, 2);
    assert.strictEqual(outFrames.stackFrames[0].line, 7);
  });

  it('should evaluate an expression where the program stopped', async () => {
    const program = fixture(PROGRAM);

    vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(program, new vscode.Position(2, 0)))]);
    await recorder.start(program, true);

    const stop = await recorder.stopped();
    const session = recorder.session();
    const { stackFrames } = await session.customRequest('stackTrace', { threadId: stop.threadId });
    const evaluated = await session.customRequest('evaluate', {
      expression: 'y * 21',
      frameId: stackFrames[0].id,
      context: 'repl'
    });

    await session.customRequest('continue', { threadId: stop.threadId });
    await recorder.terminated();

    assert.strictEqual(evaluated.result, '42');
  });

  it('should pause a program running a loop, and terminate it', async () => {
    await recorder.start(fixture(LOOP), true);

    const session = recorder.session();

    await session.customRequest('pause', { threadId: 1 });

    const stop = await recorder.stopped();
    const { stackFrames } = await session.customRequest('stackTrace', { threadId: stop.threadId });

    await vscode.debug.stopDebugging(session);
    await recorder.terminated();

    assert.strictEqual(stop.reason, 'pause');
    assert.ok([1, 2, 3, 4].includes(stackFrames[0].line), `stopped at line ${stackFrames[0].line}`);
  });

  it('should stop by a condition, log a logpoint and set a variable', async () => {
    const program = fixture(PROGRAM);
    const at = (line: number) => new vscode.Location(program, new vscode.Position(line - 1, 0));

    vscode.debug.addBreakpoints([
      new vscode.SourceBreakpoint(at(2), true, 'x > 1'),
      new vscode.SourceBreakpoint(at(6), true, undefined, undefined, 'a is {a}'),
      new vscode.SourceBreakpoint(at(7))
    ]);
    await recorder.start(program, true);

    const stop = await recorder.stopped();
    const session = recorder.session();
    const { stackFrames } = await session.customRequest('stackTrace', { threadId: stop.threadId });
    const { scopes } = await session.customRequest('scopes', { frameId: stackFrames[0].id });
    const set = await session.customRequest('setVariable', {
      variablesReference: scopes[0].variablesReference,
      name: 'b',
      value: 'b * 10'
    });
    const evaluated = await session.customRequest('evaluate', {
      expression: '[a, b]',
      frameId: stackFrames[0].id,
      context: 'repl'
    });
    const { variables: items } = await session.customRequest('variables', {
      variablesReference: evaluated.variablesReference
    });

    await session.customRequest('continue', { threadId: stop.threadId });
    await recorder.terminated();

    assert.strictEqual(stackFrames[0].line, 7);
    assert.deepStrictEqual(recorder.stops.length, 1);
    assert.deepStrictEqual(recorder.logs, ['a is 1']);
    assert.strictEqual(set.value, '20');
    assert.deepStrictEqual(
      items.map((item: { name: string; value: string }) => [item.name, item.value]),
      [
        ['0', '1'],
        ['1', '20']
      ]
    );
  });
});
