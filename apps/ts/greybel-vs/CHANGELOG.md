# greybel-vs

Releases before 3.0.0: [greybel-vs's changelog](https://github.com/ayecue/greybel-rs/blob/02aaff4745e345ea2ca0d125be2fe0883b6e4b51/apps/ts/greybel-vs/CHANGELOG.md).

## 3.0.0 · 2026-11-18

### Major changes

- Greybel v3: greybel is rewritten in Rust. The language server, the runtime and debugger, the builder and the game server are Rust programs now, shipped in the extension as a binary per platform and, on the web, as WebAssembly workers.
- Programs run in the mock world or in Grey Hack through the message hook. Mock runs use the world server that the VS Code windows and `greybel` commands of a machine share (`greybel.world.port`, `greybel.world.idleTimeout`); a window whose seed differs starts its own. World servers save the world of each seed (`greybel.world.folder`), so it continues after restarts; **Greybel: Reset the mock world** generates it again.
- `greybel.interpreter.user` runs mock programs as the player's user or as `root`, as after `sudo -s` in the game.
- GreyScript syntax features outside strict mode: number literals, `#"..."` templates, `let`/`const`, `enum`, spread and map shorthand, destructuring, `for` index, `switch`, `assert`, `export` and named imports, short functions, ternaries, `??` and optional chains, `class`, and type syntax. Builds write plain GreyScript; `syntax` in `greybel.json` turns single features off.
- The type analyzer follows control flow: types narrow on assignments, `isa`, `typeof`, null checks, truthiness and literal comparisons, also on members such as `self.shell`; unreachable code, use before assignment and functions that never return are detected.

### Editor

- New diagnostics, among them unknown members, invalid operations, native overrides, private members, missing implementations and impossible `case` values. Each code's severity is set in `greybel.json` or `greybel.typeAnalyzer.diagnostics`; `// @ignore` and `// @ignore-file` turn diagnostics off in code.
- Quick fixes: add an `#include` or `import_code`, change to a close name or member, add `self.`, remove or rename an unused local, remove unreachable code, ignore a diagnostic.
- Inlay hints for variable types, parameter names and types, and return types (`greybel.typeAnalyzer.inlayHints.*`).
- Go to type definition, call hierarchy, folding and selection ranges, a color picker for `<color=...>` tags, keyword and snippet completion, map key and doc-comment completion, `@deprecated`, and hover and go to definition on import paths.
- **Greybel: Share** saves the file with every file it imports and the project's `greybel.json`, and the link opens them in the web editor.

### Builds

- Strings with line breaks are written as one line with a marker replaced by `char(10)` (`greybel.transpiler.multilineStrings`: `Replace`, `Concatenation` or `Join`), so files stay within Grey Hack's 160,000 characters.
- Builds refuse to import a file Grey Hack can't save, naming its size, and the completion message names the in-game folder.
- The **Grey Hack** color theme uses the game's code editor colors.
