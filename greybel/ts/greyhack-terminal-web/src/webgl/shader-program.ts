import { RenderError } from './render-error.js';

const ATTRIBUTES = ['a_position', 'a_uv', 'a_scale', 'a_color'] as const;

export class ShaderProgram {
  static readonly POSITION = 0;
  static readonly UV = 1;
  static readonly SCALE = 2;
  static readonly COLOR = 3;

  readonly program: WebGLProgram;
  readonly #gl: WebGL2RenderingContext;
  readonly #uniforms = new Map<string, WebGLUniformLocation | null>();

  constructor(gl: WebGL2RenderingContext, vertex: string, fragment: string) {
    this.#gl = gl;

    const program = gl.createProgram();
    const shaders = [ShaderProgram.#compile(gl, gl.VERTEX_SHADER, vertex)];

    try {
      shaders.push(ShaderProgram.#compile(gl, gl.FRAGMENT_SHADER, fragment));
      for (const shader of shaders) gl.attachShader(program, shader);
      ATTRIBUTES.forEach((name, location) => gl.bindAttribLocation(program, location, name));
      gl.linkProgram(program);
    } finally {
      for (const shader of shaders) gl.deleteShader(shader);
    }

    if (!(gl.getProgramParameter(program, gl.LINK_STATUS) as boolean)) {
      const log = gl.getProgramInfoLog(program) ?? '';

      gl.deleteProgram(program);
      throw new RenderError(`shader program linking failed: ${log}`);
    }

    this.program = program;
  }

  uniformLocation(name: string): WebGLUniformLocation | null {
    let location = this.#uniforms.get(name);

    if (location === undefined) {
      location = this.#gl.getUniformLocation(this.program, name);
      this.#uniforms.set(name, location);
    }

    return location;
  }

  delete(): void {
    this.#gl.deleteProgram(this.program);
  }

  static #compile(gl: WebGL2RenderingContext, kind: GLenum, source: string): WebGLShader {
    const shader = gl.createShader(kind);

    if (shader === null) throw new RenderError('WebGL could not create a shader');
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (gl.getShaderParameter(shader, gl.COMPILE_STATUS) as boolean) return shader;

    const log = gl.getShaderInfoLog(shader) ?? '';

    gl.deleteShader(shader);
    throw new RenderError(`shader compilation failed: ${log}`);
  }
}
