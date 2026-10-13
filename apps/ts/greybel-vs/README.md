# Greybel VS

[![Visual Studio Marketplace](https://img.shields.io/visual-studio-marketplace/v/ayecue.greybel-vs?style=for-the-badge&label=VS%20Marketplace&logo=visual-studio-code)](https://marketplace.visualstudio.com/items?itemName=ayecue.greybel-vs)
[![Open VSX](https://img.shields.io/open-vsx/dt/ayecue/greybel-vs?color=purple&label=OpenVSX%20Downloads&style=for-the-badge)](https://open-vsx.org/extension/ayecue/greybel-vs)

GreyScript editing, building, execution, and debugging for [Grey Hack](https://greyhackgame.com) in VS Code.

## Features

- Syntax highlighting, completion, hover documentation, diagnostics, formatting, symbols, definitions, and references for `.src`, `.gs`, and `.ms` files.
- Shared `greybel.json` project settings and selectable build/run profiles.
- Default, minified, and formatted output, dependency bundling, installer generation, and file imports.
- A persistent mock Grey Hack world or execution in the game through its message hook.
- Breakpoints, stepping, variable inspection, paused evaluation, terminal input/output, and TextMesh Pro output preview.
- Integrated API documentation, comment-based type documentation, snippets, and sharing.

## Install

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=ayecue.greybel-vs) or [Open VSX](https://open-vsx.org/extension/ayecue/greybel-vs). VS Code 1.90 or later is required.

## Quick start

1. Open a folder, then open a `.src`, `.gs`, or `.ms` file.
2. Run **Greybel: Run file from context** from the Command Palette or editor context menu.
3. Run **Greybel: Build file from context** to write a build to `build/`.
4. Add `greybel.json` to configure a project entry, build options, run options, and profiles:

   ```json
   {
     "entries": ["src/main.src"],
     "build": { "mode": "Uglify" },
     "run": { "seed": "test" }
   }
   ```

In-game imports and execution require the desktop extension and the message-hook setup described in the integration guide.

## Documentation

- [Complete VS Code guide](https://github.com/ayecue/greybel-rs/blob/main/docs/vscode.md): commands, project settings, builds, debugging, comment documentation, and troubleshooting.
- [CLI guide](https://github.com/ayecue/greybel-rs/blob/main/docs/cli.md)
- [Grey Hack integration](https://github.com/ayecue/greybel-rs/blob/main/docs/greyhack-integration.md)
- [GreyScript API documentation](https://documentation.greyscript.org)
- [Changelog](CHANGELOG.md)
