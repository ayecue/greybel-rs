// Empties dist/ before a compile, so no file of an earlier build stays; dist/bin stays, as build-binaries.cjs writes
// it for the platform being packaged before vsce compiles.
const { existsSync, readdirSync, rmSync } = require('node:fs');
const { join } = require('node:path');

const DIST = join(__dirname, '..', 'dist');

for (const entry of existsSync(DIST) ? readdirSync(DIST, { withFileTypes: true }) : []) {
  if (entry.name !== 'bin') rmSync(join(DIST, entry.name), { recursive: true, force: true });
}
