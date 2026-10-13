import * as vscode from 'vscode';

import { DebugRecorder } from './debug';
import { fixture } from './workspace';

export const WRITE = 'world/write.src';

const READ = 'world/read.src';

/** Runs `path` to its end in a debug session of its own. */
export async function run(path: string): Promise<void> {
  const recorder = new DebugRecorder();

  try {
    await recorder.start(fixture(path), false);
    await recorder.terminated();
  } finally {
    recorder.dispose();
  }
}

/** The value of `content` where the reader stops, before printing it. */
export async function readContent(): Promise<string> {
  const program = fixture(READ);
  const recorder = new DebugRecorder();

  vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(program, new vscode.Position(3, 0)))]);

  try {
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

    return variables.find((variable: { name: string }) => variable.name === 'content').value;
  } finally {
    vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
    recorder.dispose();
  }
}
