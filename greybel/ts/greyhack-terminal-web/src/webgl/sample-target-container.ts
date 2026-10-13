export class SampleTargetContainer {
  readonly texture: WebGLTexture;
  readonly #gl: WebGL2RenderingContext;
  readonly #framebuffer: WebGLFramebuffer;
  #size: readonly [number, number, number] = [0, 0, 0];

  constructor(gl: WebGL2RenderingContext) {
    this.#gl = gl;
    this.texture = gl.createTexture();
    this.#framebuffer = gl.createFramebuffer();
  }

  prepare(width: number, height: number, layers: number): void {
    const [currentWidth, currentHeight, currentLayers] = this.#size;

    if (currentWidth === width && currentHeight === height && currentLayers === layers) return;

    const gl = this.#gl;

    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.texture);
    gl.texImage3D(gl.TEXTURE_2D_ARRAY, 0, gl.RGBA8, width, height, layers, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    this.#size = [width, height, layers];
  }

  bind(layer: number): void {
    const gl = this.#gl;

    gl.bindFramebuffer(gl.FRAMEBUFFER, this.#framebuffer);
    gl.framebufferTextureLayer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, this.texture, 0, layer);
  }

  delete(): void {
    this.#gl.deleteTexture(this.texture);
    this.#gl.deleteFramebuffer(this.#framebuffer);
  }
}
