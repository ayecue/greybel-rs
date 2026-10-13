// Compares the TypeScript renderer's drawings with Unity captures, over the game's atlases and over rebuilt ones.
(() => {
  const data = JSON.parse(document.getElementById('meshes').textContent);
  const { WebGlRenderer, Scene, Frame, Mesh, Material, DistanceField, Vector2, Vector4, Color } = greyhackTerminalWeb;
  const results = [];
  const bytes = (base64) => Uint8Array.from(atob(base64), (character) => character.charCodeAt(0));
  const vector2 = ([x, y]) => new Vector2({ x, y });
  const vector4 = ([x, y, z, w]) => new Vector4({ x, y, z, w });
  const pixels = (image) => {
    const context = document.createElement('canvas').getContext('2d', { willReadFrequently: true });
    context.canvas.width = image.width;
    context.canvas.height = image.height;
    context.drawImage(image, 0, 0);
    return context.getImageData(0, 0, image.width, image.height);
  };
  /** A renderer on a canvas of its own, with one atlas source's atlases and the sprite sheet. */
  const makeRenderer = (target) => {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: true,
    });
    const renderer = new WebGlRenderer(gl);
    for (const { font, index, width, height, alpha } of data.atlases[target] ?? []) {
      renderer.setAtlas(font, index, { width, height, data: bytes(alpha) });
    }
    const sheet = data.spriteSheet;
    renderer.setSpriteSheet({ width: sheet.width, height: sheet.height, data: bytes(sheet.pixels) });
    return { canvas, renderer };
  };
  const material = (font) => {
    if (font === null) return new Material.Sprites();
    const uniforms = font.uniforms;
    return new Material.Font({
      font: font.font,
      atlas: font.atlas,
      uniforms:
        uniforms === null
          ? null
          : new DistanceField({
              ...uniforms,
              faceColor: vector4(uniforms.faceColor),
              outlineColor: vector4(uniforms.outlineColor),
              scale: vector2(uniforms.scale),
              vertexOffset: vector2(uniforms.vertexOffset),
            }),
    });
  };
  const scene = ({ frame, meshes }) =>
    new Scene({
      frame: new Frame({
        ...frame,
        samples: frame.samples.map(vector2),
        background: new Color(Object.fromEntries(['r', 'g', 'b', 'a'].map((key, index) => [key, frame.background[index]]))),
      }),
      meshes: meshes.map(({ font, vertices }) => new Mesh({ material: material(font), vertices: bytes(vertices) })),
    });
  const run = () => {
    const renderers = {};
    for (const entry of data.cases) {
      const { canvas, renderer } = (renderers[entry.target] ??= makeRenderer(entry.target));
      canvas.width = entry.frame.width;
      canvas.height = entry.frame.height;
      renderer.draw(scene(entry));
      const rendering = canvas.toDataURL('image/png');
      const section = document.getElementById(`case-${entry.section}`);
      section.querySelector(`img.${entry.target}`).src = rendering;
      results.push({ name: entry.name, target: entry.target, section: entry.section, rendering });
    }
    document.getElementById('renderings').textContent = JSON.stringify(results);
  };
  const compare = () => {
    for (const entry of data.cases) {
      const section = document.getElementById(`case-${entry.section}`);
      const unity = section.querySelector('.unity');
      const webgl = section.querySelector(`img.${entry.target}`);
      if (!unity || !unity.naturalWidth) continue;
      const a = pixels(unity);
      const b = pixels(webgl);
      const width = Math.min(a.width, b.width);
      const height = Math.min(a.height, b.height);
      const diff = new ImageData(width, height);
      let sum = 0;
      let max = 0;
      let over = 0;
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          const i = (y * a.width + x) * 4;
          const j = (y * b.width + x) * 4;
          let pixel = 0;
          for (let c = 0; c < 3; c++) {
            const d = Math.abs(a.data[i + c] - b.data[j + c]);
            sum += d;
            pixel = Math.max(pixel, d);
          }
          max = Math.max(max, pixel);
          if (pixel > 8) over++;
          const o = (y * width + x) * 4;
          const shade = pixel === 0 ? [0, 0, 0] : pixel > 8 ? [255, 64, 64] : Array(3).fill(64 + pixel * 16);
          diff.data.set([...shade, 255], o);
        }
      }
      const canvas = section.querySelector(`.diff-${entry.target}`);
      canvas.width = width;
      canvas.height = height;
      canvas.getContext('2d').putImageData(diff, 0, 0);
      section.querySelector(`.stats-${entry.target}`).textContent =
        `mean ${(sum / (width * height * 3)).toFixed(3)}, max ${max}, ${(100 * over / (width * height)).toFixed(3)}% > 8/255` +
        (a.width !== b.width || a.height !== b.height ? ` (sizes ${a.width}×${a.height} vs ${b.width}×${b.height})` : '');
    }
  };
  window.addEventListener('load', () => {
    try {
      run();
    } catch (error) {
      document.getElementById('renderings').textContent = JSON.stringify({ error: String(error) });
      throw error;
    }
    Promise.all([...document.querySelectorAll('img.webgl, img.built')].map((image) => image.decode())).then(compare);
  });
})();
