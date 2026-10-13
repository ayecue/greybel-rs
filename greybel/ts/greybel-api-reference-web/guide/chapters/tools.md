# Tools

Greybel's tools parse GreyScript with [Greybel extensions](#guide-greybel), execute programs in a simulated Grey Hack environment, and build programs for the game. They share the parser, interpreter, and type analyzer, providing consistent checks and behavior across interfaces.

## Visual Studio Code

The [greybel-vs](https://marketplace.visualstudio.com/items?itemName=ayecue.greybel-vs) extension is available from the Visual Studio Marketplace and [Open VSX](https://open-vsx.org/extension/ayecue/greybel-vs). It recognizes `.src`, `.gs`, and `.ms` files.

Its language server provides syntax highlighting, completion, inline documentation, diagnostics, type-analysis results, definition and reference navigation, and formatting. Commands in the command palette (`Ctrl+Shift+P`) and file context menus run or debug programs, build or transform source, import files into the game, and open API documentation, command snippets, and a terminal preview. Debugging supports breakpoints, stepping, and variable inspection in the simulated environment or in a single-player game. The extension runs in desktop and web editors.

## Command line

The `greybel` command provides project builds, execution, debugging, imports, and related tools:

| Command                         | Purpose                                                                                                                                                                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `greybel build [file] [output]` | Builds one file or files matched by a glob such as `"src/*.src"`, including imports, into `build/`. `-u` uglifies, `-b` beautifies, `--no-installer` skips the installer files written by default, and `--create-ingame` creates the build in a single-player or multiplayer game. |
| `greybel execute [file]`        | Executes a program in the simulated environment (`-s` sets the seed and `-p` sets params), in a single-player game (`--env-type in-game`), or with the terminal debugger (`-d`, `--break FILE:LINE`).                                                                              |
| `greybel repl`                  | Evaluates code interactively in the simulated environment.                                                                                                                                                                                                                         |
| `greybel import <path>`         | Imports a file or folder into a single-player or multiplayer game without building it.                                                                                                                                                                                             |
| `greybel ui`                    | Opens the [web editor](https://editor.greyscript.org).                                                                                                                                                                                                                             |
| `greybel mcp`                   | Starts the Model Context Protocol server described below.                                                                                                                                                                                                                          |

Run `greybel <command> --help` for all options. Other editors can use the standalone `greybel-lsp` language server over standard input and output.

## Configuration

A project stores shared settings in `greybel.json`. The CLI, MCP server, language server, and VS Code extension read this file; the extension's project settings remain synchronized with it. Command-line options override corresponding configuration values, while unspecified settings retain their defaults.

```json
{
  "entries": ["src/main.src"],
  "extensions": ["src", "gs", "ms"],
  "build": { "mode": "Uglify", "installer": { "active": true } },
  "run": { "seed": "test", "params": ["--verbose"], "memoryLimit": 1024, "variables": { "files": [".env"] } }
}
```

`entries` specifies the default starting files for `greybel build` and `greybel execute` when no file argument is supplied. Editor tooling uses the configuration's JSON Schema for completion and validation.

## Running a program

A run builds the program, with the files it includes, imports, and loads through `import_code`, into a binary in its user's home folder: `~/myprogram`, named by `run.programName` (`--program-name` on the command line). The program runs as that binary, so `program_path` names it and `launch(program_path)` starts the program again. The binary keeps a source map of where each of its lines came from, so a runtime error names the file and line it was written on, and breakpoints stop in every file. In the simulated world the binary remains like one built with `build`; in the game it exists only while the run lasts, and a file of that name that isn't a binary is left alone, in which case the program can't launch itself.

## Profiles

Profiles define named overrides for `build` and `run` settings. Profile values replace base values object key by object key. The `extends` property derives a profile from another profile.

```json
{
  "build": { "installer": { "active": false } },
  "profiles": {
    "release": { "build": { "mode": "Uglify", "obfuscation": true } },
    "ingame": { "extends": "release", "build": { "ingameDirectory": "/home/user/" } }
  }
}
```

Select a profile with `--profile` or `GREYBEL_PROFILE` on the command line, `profile` in an MCP tool request, or **Greybel: Select profile** and the status bar in VS Code.

## Game connection

Building into the game with `--create-ingame` and importing files with `greybel import` or the extension work in both single-player and multiplayer through the message hook. The message hook is a BepInEx plugin that listens on port 7777 by default. The setup wizard installs BepInEx, the plugin, and its configuration through either a window or its command-line interface.

Executing a program in the game with `greybel execute --env-type in-game`, including in-game debugging, is available only in single-player. In multiplayer, the game's interpreter runs on the server.

A build created in the game with `--create-ingame`, or the corresponding extension setting, is written to an in-game folder. With `autoCompile`, the build is also compiled and its source files are deleted afterward.

## MCP server

`greybel mcp` exposes Greybel tools over the Model Context Protocol for assistants that develop GreyScript. Available operations include searching and reading this API reference, checking and transforming code, building programs, and running them in the simulated environment with the project's `greybel.json`. The `--allow-game` option additionally permits running programs in the game and importing files into it.
