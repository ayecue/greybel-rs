const PIXEL_LIMIT = 8;

import { MessageTargetDriver } from 'duplex-driver-worker';
import { Output, OutputBatch, ProgramChannel } from 'greybel-io-web/program';

import type { LineLimits, Scene, TerminalPage } from '../../src/index.js';

export interface Difference {
  readonly mean: number;
  readonly max: number;
  readonly overLimit: number;
}

let openedConnection: { readonly terminal: TerminalPage; readonly program: ProgramChannel } | undefined;

let expectedDraw: Promise<unknown> | undefined;

function frame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function connection(): NonNullable<typeof openedConnection> {
  if (openedConnection === undefined) throw new Error('the terminal is not open');

  return openedConnection;
}

let lastScene: Scene | undefined;

async function nextDrawWithMeshes(): Promise<Scene> {
  for (;;) {
    const scene = await connection().terminal.nextDraw();

    if (scene.meshes.length > 0) return scene;
  }
}

/** Size of the vertices of the last scene drawn. */
export function getVertexBytes(): number {
  return lastScene?.meshes.reduce((sum, mesh) => sum + mesh.vertices.length, 0) ?? 0;
}

export function expectDraw(): void {
  expectedDraw = connection().terminal.nextDraw();
}

export async function awaitDraw(): Promise<void> {
  await expectedDraw;
  await frame();
}

/** Opens the terminal, its WebAssembly the bytes `wasm` encodes in base64. */
export async function open(wasm: string): Promise<void> {
  const { port1, port2 } = new MessageChannel();
  const bytes = Uint8Array.from(atob(wasm), (character) => character.charCodeAt(0));

  const page = greyhackTerminalWeb.openTerminal(document.getElementById('terminal') as HTMLCanvasElement, port1, bytes);
  const recordDraws = (): void => {
    void page.nextDraw().then((scene) => {
      lastScene = scene;
      recordDraws();
    });
  };

  recordDraws();

  const channel = new ProgramChannel(new MessageTargetDriver(port2));

  await new Promise<void>((resolve) => channel.onAck(() => resolve()));
  openedConnection = { terminal: page, program: channel };
}

export async function print(text: string): Promise<void> {
  const nextDraw = nextDrawWithMeshes();

  await connection().program.send(
    new OutputBatch({ entries: [new Output.Print({ text, newLine: false, replace: false })] })
  );
  await nextDraw;
  await frame();
}

async function pixels(image: string): Promise<Uint8ClampedArray> {
  const decoded = new Image();

  decoded.src = `data:image/png;base64,${image}`;
  await decoded.decode();

  const context = new OffscreenCanvas(decoded.width, decoded.height).getContext('2d')!;

  context.drawImage(decoded, 0, 0);

  return context.getImageData(0, 0, decoded.width, decoded.height).data;
}

export function setLineLimits(limits: LineLimits.Fields | undefined): void {
  connection().terminal.setLineLimits(limits);
}

export async function compare(shot: string, capture: string): Promise<Difference> {
  const [ours, theirs] = await Promise.all([pixels(shot), pixels(capture)]);
  let sum = 0;
  let max = 0;
  let overLimit = 0;

  for (let index = 0; index < ours.length; index += 4) {
    let pixel = 0;

    for (let channel = 0; channel < 3; channel++) {
      const difference = Math.abs(ours[index + channel]! - theirs[index + channel]!);

      sum += difference;
      pixel = Math.max(pixel, difference);
    }

    max = Math.max(max, pixel);
    if (!(pixel > PIXEL_LIMIT)) continue;
    overLimit++;
  }

  const count = ours.length / 4;

  return { mean: sum / (count * 3), max, overLimit: overLimit / count };
}
