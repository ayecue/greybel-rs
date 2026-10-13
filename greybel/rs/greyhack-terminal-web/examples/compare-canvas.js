// Replays text-mesh-canvas' drawing commands on 2D canvases once the
// embedded fonts are loaded.
const families = new Set();
const pages = [...document.querySelectorAll('canvas[data-commands]')].map((canvas) => {
  const commands = JSON.parse(document.getElementById(canvas.dataset.commands).textContent);
  commands.forEach((command) => command.kind === 'glyph' && families.add(command.font));
  return [canvas, commands];
});
const paint = ({ r, g, b, a }) => `rgba(${r},${g},${b},${a / 255})`;
Promise.all([...families].map((family) => document.fonts.load(`16px '${family}'`))).then(() => {
  for (const [canvas, commands] of pages) {
    const ratio = window.devicePixelRatio || 1;
    const width = Number(canvas.dataset.width);
    const height = Number(canvas.dataset.height);
    canvas.width = Math.ceil(width * ratio);
    canvas.height = Math.ceil(height * ratio);
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    const context = canvas.getContext('2d');
    context.scale(ratio, ratio);
    context.fillStyle = canvas.dataset.background;
    context.fillRect(0, 0, width, height);
    context.textBaseline = 'alphabetic';
    for (const command of commands) {
      context.fillStyle = paint(command.paint);
      if (command.kind === 'rect') {
        context.fillRect(command.x, command.y, command.width, command.height);
        continue;
      }
      context.font = `${command.bold ? 'bold ' : ''}${command.size}px '${command.font}'`;
      if (command.transform) {
        const { a, b, c, d, e, f } = command.transform;
        context.save();
        context.transform(a, b, c, d, e, f);
        context.fillText(command.character, command.x, command.baseline);
        context.restore();
      } else {
        context.fillText(command.character, command.x, command.baseline);
      }
    }
  }
});
