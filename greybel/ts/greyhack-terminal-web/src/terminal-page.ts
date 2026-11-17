import { WorkerHost, type WorkerLike } from 'duplex-driver-worker';
import type { Transport } from 'duplex-core';
import { Io, IoPeer } from 'greybel-io-web';

import {
  Draw,
  LineLimits,
  PageChannel,
  PageRouter,
  Scroll,
  SetLineLimits,
  Textures,
  type Redraw,
  type Scene
} from './page.generated.js';
import { RenderError } from './webgl/render-error.js';
import { WebGlRenderer } from './webgl/renderer.js';

export class TerminalPage {
  readonly #canvas: HTMLCanvasElement;
  readonly #renderer: WebGlRenderer;
  readonly #host: WorkerHost<{ view: PageChannel }>;
  readonly #textures: Promise<void>;
  readonly #resizeObserver: ResizeObserver;

  #drawListeners: ((scene: Scene) => void)[] = [];

  #frame: number | undefined;
  #isDrawing = false;

  #isStale = false;
  #isClosed = false;

  readonly #onWheel = (event: WheelEvent) => {
    event.preventDefault();
    void this.#host.channels.view.send(new Scroll({ delta: event.deltaY }));
    this.#requestDraw();
  };

  constructor(canvas: HTMLCanvasElement, worker: WorkerLike, program: Transport) {
    const gl = canvas.getContext('webgl2', { alpha: false, antialias: false, premultipliedAlpha: false });

    if (gl === null) throw new RenderError('the canvas has no WebGL2 context');
    this.#canvas = canvas;
    this.#renderer = new WebGlRenderer(gl);
    this.#host = new WorkerHost(
      worker,
      { view: { channelClass: PageChannel, routers: [new RedrawRouter(() => this.#requestDraw())] } },
      { relays: [{ transport: program, protocol: Io, peer: IoPeer.Page }] }
    );
    this.#textures = this.#loadTextures();
    this.#resizeObserver = new ResizeObserver(() => this.#requestDraw());
    this.#resizeObserver.observe(canvas);
    canvas.addEventListener('wheel', this.#onWheel, { passive: false });
    this.#requestDraw();
  }

  /** Splits long output lines by `limits`, as greyhack-customizable-output does; `undefined` splits them as the game does. */
  setLineLimits(limits: LineLimits.Fields | undefined): void {
    void this.#host.channels.view.send(
      new SetLineLimits({ limits: limits === undefined ? undefined : new LineLimits(limits) })
    );
  }

  nextDraw(): Promise<Scene> {
    return new Promise((resolve) => this.#drawListeners.push(resolve));
  }

  close(): void {
    if (this.#isClosed) return;
    this.#isClosed = true;
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    this.#resizeObserver.disconnect();
    this.#canvas.removeEventListener('wheel', this.#onWheel);
    this.#host.close();
    this.#renderer.delete();
  }

  async #loadTextures(): Promise<void> {
    const { atlases, spriteSheet } = await this.#host.channels.view.send(new Textures());

    for (const { font, index, width, height, alpha } of atlases) {
      this.#renderer.setAtlas(font, index, { width, height, data: alpha });
    }

    this.#renderer.setSpriteSheet({ width: spriteSheet.width, height: spriteSheet.height, data: spriteSheet.rgba });
  }

  #requestDraw(): void {
    if (this.#isClosed || this.#frame !== undefined) return;
    if (this.#isDrawing) {
      this.#isStale = true;

      return;
    }

    this.#frame = requestAnimationFrame(() => {
      this.#frame = undefined;
      void this.#draw();
    });
  }

  async #draw(): Promise<void> {
    this.#isDrawing = true;
    try {
      const scene = await this.#host.channels.view.send(
        new Draw({
          width: Math.max(this.#canvas.clientWidth, 1),
          height: Math.max(this.#canvas.clientHeight, 1),
          pixelRatio: devicePixelRatio
        })
      );

      await this.#textures;
      if (this.#isClosed) return;
      this.#canvas.width = scene.frame.width;
      this.#canvas.height = scene.frame.height;
      this.#renderer.draw(scene);

      const listeners = this.#drawListeners;

      this.#drawListeners = [];
      for (const listener of listeners) listener(scene);
    } catch (error) {
      if (this.#isClosed) return;
      console.error('greyhack-terminal-web:', error);
    } finally {
      this.#isDrawing = false;
      if (this.#isStale) {
        this.#isStale = false;
        this.#requestDraw();
      }
    }
  }
}

class RedrawRouter extends PageRouter {
  readonly #redraw: () => void;

  constructor(redraw: () => void) {
    super();
    this.#redraw = redraw;
  }

  override onRedraw(_message: Redraw): void {
    this.#redraw();
  }
}
