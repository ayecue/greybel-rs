import resolveFragment from 'text-mesh-shaders/resolve.frag?raw';
import resolveVertex from 'text-mesh-shaders/resolve.vert?raw';
import sdfFragment from 'text-mesh-shaders/sdf.frag?raw';
import sdfVertex from 'text-mesh-shaders/sdf.vert?raw';
import spriteFragment from 'text-mesh-shaders/sprite.frag?raw';
import spriteVertex from 'text-mesh-shaders/sprite.vert?raw';

import {
  Material,
  type DistanceField,
  type Frame,
  type Mesh,
  type Scene,
  type Vector4
} from '../protocol.generated.js';
import { RenderError } from './render-error.js';
import { SampleTargetContainer } from './sample-target-container.js';
import { ShaderProgram } from './shader-program.js';
import { TextureFormat, createTexture, type TextureImage } from './texture.js';

const STRIDE = 24;
const VERTICES_PER_QUAD = 4;
const INDICES_PER_QUAD = 6;

const DISTANCE_FIELD_FLOATS = [
  ['u_gradient_scale', 'gradientScale'],
  ['u_sharpness', 'sharpness'],
  ['u_weight_normal', 'weightNormal'],
  ['u_weight_bold', 'weightBold'],
  ['u_face_dilate', 'faceDilate'],
  ['u_scale_ratio_a', 'scaleRatioA'],
  ['u_outline_width', 'outlineWidth'],
  ['u_outline_softness', 'outlineSoftness']
] as const satisfies readonly (readonly [string, keyof DistanceField.Fields])[];

export class WebGlRenderer {
  readonly #gl: WebGL2RenderingContext;
  readonly #sdf: ShaderProgram;
  readonly #sprite: ShaderProgram;
  readonly #resolve: ShaderProgram;
  readonly #vertexArray: WebGLVertexArrayObject;
  readonly #vertices: WebGLBuffer;
  readonly #indices: WebGLBuffer;
  readonly #targets: SampleTargetContainer;
  readonly #atlases = new Map<string, WebGLTexture>();
  #spriteSheet: WebGLTexture | undefined;

  constructor(gl: WebGL2RenderingContext) {
    this.#gl = gl;
    this.#sdf = new ShaderProgram(gl, sdfVertex, sdfFragment);
    this.#sprite = new ShaderProgram(gl, spriteVertex, spriteFragment);
    this.#resolve = new ShaderProgram(gl, resolveVertex, resolveFragment);
    this.#vertexArray = gl.createVertexArray();
    this.#vertices = gl.createBuffer();
    this.#indices = gl.createBuffer();
    this.#targets = new SampleTargetContainer(gl);
    this.#bindAttributes();
  }

  setAtlas(font: number, atlas: number, pixels: TextureImage): void {
    const key = WebGlRenderer.#atlasKey(font, atlas);
    const previous = this.#atlases.get(key);

    if (previous !== undefined) this.#gl.deleteTexture(previous);
    this.#atlases.set(key, createTexture(this.#gl, TextureFormat.Distance, pixels));
  }

  setSpriteSheet(pixels: TextureImage): void {
    if (this.#spriteSheet !== undefined) this.#gl.deleteTexture(this.#spriteSheet);
    this.#spriteSheet = createTexture(this.#gl, TextureFormat.Color, pixels);
  }

  draw({ frame, meshes }: Scene): void {
    const gl = this.#gl;
    const samples = frame.samples;

    this.#targets.prepare(frame.width, frame.height, samples.length);
    gl.viewport(0, 0, frame.width, frame.height);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.CULL_FACE);

    const { r, g, b, a } = frame.background;

    samples.forEach((offset, layer) => {
      this.#targets.bind(layer);
      gl.clearColor(r / 255, g / 255, b / 255, a / 255);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.enable(gl.BLEND);
      gl.bindVertexArray(this.#vertexArray);
      for (const mesh of meshes) this.#drawMesh(frame, [offset.x, offset.y], mesh);
      gl.bindVertexArray(null);
    });
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.disable(gl.BLEND);
    gl.useProgram(this.#resolve.program);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.#targets.texture);
    gl.uniform1i(this.#resolve.uniformLocation('u_samples'), 0);
    gl.uniform1i(this.#resolve.uniformLocation('u_count'), samples.length);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  delete(): void {
    const gl = this.#gl;

    for (const texture of this.#atlases.values()) gl.deleteTexture(texture);
    if (this.#spriteSheet !== undefined) gl.deleteTexture(this.#spriteSheet);
    this.#targets.delete();
    gl.deleteBuffer(this.#vertices);
    gl.deleteBuffer(this.#indices);
    gl.deleteVertexArray(this.#vertexArray);
    for (const program of [this.#sdf, this.#sprite, this.#resolve]) program.delete();
  }

  #drawMesh(frame: Frame, [sampleX, sampleY]: readonly [number, number], { material, vertices }: Mesh): void {
    const gl = this.#gl;
    const [program, texture, samplerUniform] = this.#useMaterial(frame, material);
    const quads = vertices.length / STRIDE / VERTICES_PER_QUAD;

    gl.uniformMatrix4fv(program.uniformLocation('u_projection'), false, frame.projection);
    gl.uniform2f(program.uniformLocation('u_screen'), frame.width, frame.height);
    gl.uniform2f(program.uniformLocation('u_sample_offset'), sampleX, sampleY);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.uniform1i(program.uniformLocation(samplerUniform), 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.#vertices);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, WebGlRenderer.#quadIndices(quads), gl.STREAM_DRAW);
    gl.drawElements(gl.TRIANGLES, quads * INDICES_PER_QUAD, gl.UNSIGNED_INT, 0);
  }

  #useMaterial(frame: Frame, material: Material): [ShaderProgram, WebGLTexture, string] {
    const gl = this.#gl;

    if (material instanceof Material.Font) {
      const texture = this.#atlases.get(WebGlRenderer.#atlasKey(material.font, material.atlas));

      if (texture === undefined) throw new RenderError(`no atlas ${material.atlas} was set for font ${material.font}`);
      gl.useProgram(this.#sdf.program);
      gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
      if (material.uniforms !== null) this.#setDistanceField(material.uniforms);
      gl.uniform1f(this.#sdf.uniformLocation('u_anisotropy'), frame.anisotropy);

      return [this.#sdf, texture, 'u_atlas'];
    }

    if (this.#spriteSheet === undefined) throw new RenderError('no sprite sheet was set');
    gl.useProgram(this.#sprite.program);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.uniform4f(this.#sprite.uniformLocation('u_color'), 1, 1, 1, 1);
    gl.uniform4f(this.#sprite.uniformLocation('u_texture_sample_add'), 0, 0, 0, 0);

    return [this.#sprite, this.#spriteSheet, 'u_sprites'];
  }

  #setDistanceField(material: DistanceField): void {
    const gl = this.#gl;
    const setVector4 = (name: string, { x, y, z, w }: Vector4) =>
      gl.uniform4f(this.#sdf.uniformLocation(name), x, y, z, w);

    setVector4('u_face_color', material.faceColor);
    setVector4('u_outline_color', material.outlineColor);
    for (const [name, property] of DISTANCE_FIELD_FLOATS)
      gl.uniform1f(this.#sdf.uniformLocation(name), material[property]);
    gl.uniform2f(this.#sdf.uniformLocation('u_scale_xy'), material.scale.x, material.scale.y);
    gl.uniform2f(this.#sdf.uniformLocation('u_vertex_offset'), material.vertexOffset.x, material.vertexOffset.y);
  }

  #bindAttributes(): void {
    const gl = this.#gl;

    gl.bindVertexArray(this.#vertexArray);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.#vertices);
    for (const [location, size, kind, isNormalized, offset] of [
      [ShaderProgram.POSITION, 2, gl.FLOAT, false, 0],
      [ShaderProgram.UV, 2, gl.FLOAT, false, 8],
      [ShaderProgram.SCALE, 1, gl.FLOAT, false, 16],
      [ShaderProgram.COLOR, 4, gl.UNSIGNED_BYTE, true, 20]
    ] as const) {
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(location, size, kind, isNormalized, STRIDE, offset);
    }

    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.#indices);
    gl.bindVertexArray(null);
  }

  static #atlasKey(font: number, atlas: number): string {
    return `${font}:${atlas}`;
  }

  static #quadIndices(quads: number): Uint32Array {
    const indices = new Uint32Array(quads * INDICES_PER_QUAD);

    for (let quad = 0; quad < quads; quad++) {
      const first = quad * VERTICES_PER_QUAD;

      indices.set([first, first + 1, first + 2, first + 2, first + 3, first], quad * INDICES_PER_QUAD);
    }

    return indices;
  }
}
