export interface TextureImage {
  readonly width: number;
  readonly height: number;
  readonly data: Uint8Array;
}

export enum TextureFormat {
  Distance,
  Color
}

export function createTexture(gl: WebGL2RenderingContext, format: TextureFormat, pixels: TextureImage): WebGLTexture {
  const texture = gl.createTexture();
  const [internal, layout] = format === TextureFormat.Distance ? [gl.R8, gl.RED] : [gl.RGBA8, gl.RGBA];

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texImage2D(gl.TEXTURE_2D, 0, internal, pixels.width, pixels.height, 0, layout, gl.UNSIGNED_BYTE, pixels.data);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

  return texture;
}
