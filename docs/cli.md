# Greybel CLI

The `greybel` command builds and transforms GreyScript, runs and debugs programs in a simulated Grey Hack world or in the game, imports files, and provides a REPL, web editor launcher, and MCP server.

For editor features, see [VS Code](vscode.md). Commands that access Grey Hack require the message hook; see [Grey Hack integration](greyhack-integration.md). Local builds, mock execution, and the REPL do not need the game or plugin.

## Installation

Download a platform binary from [GitHub Releases](https://github.com/ayecue/greybel-rs/releases) and put `greybel` on your `PATH`.

Alternatively, from the root of a `greybel-rs` checkout with Rust installed:

```sh
cargo install --path apps/rs/greybel-cli --locked
```

Choose the release artifact for your operating system and architecture. The source installation uses the checkout's locked dependencies.

```sh
greybel --help
greybel build --help
```

Running `greybel` without a subcommand also prints its command list.

## Projects and profiles

All settings are optional. When a command names a file, Greybel looks for the nearest `greybel.json` in that file's folder or a parent folder. Without a file argument, discovery starts in the current folder. Without a configuration file, defaults apply and paths are relative to the current folder.

A typical project is:

```json
{
  "entries": ["src/main.src"],
  "extensions": ["src", "gs", "ms"],
  "strictMode": false,
  "format": {
    "indentation": "Whitespace",
    "indentationSpaces": 2,
    "retainParentheses": false
  },
  "build": {
    "output": "build",
    "mode": "Uglify",
    "obfuscation": false,
    "ingameDirectory": "/root/",
    "autoCompile": true,
    "installer": {
      "active": true,
      "maxChars": 160000,
      "allowImport": false
    },
    "deploy": { "allowImport": false },
    "variables": { "files": [".env"], "values": {} }
  },
  "run": {
    "seed": "test",
    "params": [],
    "programName": "myprogram",
    "memoryLimit": 1024,
    "variables": { "files": [".env"], "values": {} }
  }
}
```

`entries` supplies the files for `greybel build` and the first entry for `greybel execute` when no file is named. Entry and environment-file paths are relative to the project folder. `extensions` lists suffixes an import may omit, in lookup order; its default is `src`, `gs`, `ms`. `strictMode` rejects Greybel syntax that Grey Hack's compiler does not accept. `exclude` is a separate type-analyzer exclusion glob, defaulting to `**/{node_modules,.*}/**`; it does not exclude build dependencies.

Build settings also include `outputName` for a single entry, `disableLiteralsOptimization`, `disableNamespacesOptimization`, `excludedNamespaces`, and `multilineStrings`. `multilineStrings` sets how builds and runs write strings with line breaks: `Replace` (default) writes `"a¶b".replace("¶",char(10))` with a marker character the string doesn't contain, which keeps the file about the size of the source and takes one pass at run time; `Concatenation` writes `"a"+__NL+"b"` with `__NL=char(10)`; `Join` writes `__MSTR(["a","b"])`. Grey Hack saves at most 160,000 characters per file, so the other two styles can make an imported file with a long string too large for the game. `memoryLimit` is in MiB and applies to mock execution. `run.lineLimits` (`{ "textPerLine": 256, "tagsPerLine": null }`) replaces the game's rule for cutting long printed lines with the one of the greyhack-customizable-output mod: a line whose visible text is longer than `textPerLine` characters, or whose tags take more than `tagsPerLine` characters, is cut into pieces of `textPerLine` characters; `null` for `tagsPerLine` means no tag limit. `greybel execute` and `greybel repl` take `--text-per-line N` and `--tags-per-line N` for the same. Unknown configuration fields are rejected. The complete editor schema is [greybel.schema.json](../apps/ts/greybel-vs/schemas/greybel.schema.json).

Profiles override `build` and `run`:

```json
{
  "build": { "installer": { "active": false } },
  "profiles": {
    "release": {
      "build": { "mode": "Uglify", "obfuscation": true }
    },
    "ingame": {
      "extends": "release",
      "build": { "ingameDirectory": "/home/user/" }
    }
  }
}
```

Objects merge key by key; arrays and scalar values replace the inherited value. `extends` applies another profile first. Select a profile with `--profile NAME` or the `GREYBEL_PROFILE` environment variable. Explicit command options override the selected project's corresponding settings.

## Build and transform

```sh
greybel build src/main.src
greybel build "src/*.src" dist --uglify
greybel build --profile release
greybel build src/main.src --output-filename app.src
```

The first argument is a file or glob; quote globs so Greybel, rather than the shell, expands them. With no file, every configured entry is built. `--output-filename` (`build.outputName`) is valid only for a single entry.

By default, output goes under `build/` in the project folder. A positional output folder is a base folder, so `greybel build src/main.src dist` writes to `dist/build/` unless `build.output` says otherwise. `--disable-build-folder` writes directly into the base folder. Unless that option is used, Greybel removes the previous configured build-output folder before writing; use a dedicated output folder.

There is no separate `greybel transform` subcommand. A build transforms each file **and resolves its dependencies**. Standalone transformation without resolving `#include`, `#import`, or `import_code` is available as the editor's transform operation and the MCP `transform` tool.

### Output modes

| Mode | Selection | Output |
|---|---|---|
| `Default` | `build.mode: "Default"` | Source as written, without comments; no special optimization or formatting. |
| `Uglify` | `--uglify` / `-u` | Minified code with literal and namespace optimizations. |
| `Beautify` | `--beautify` / `-b` | Formatted code using the `format` settings. |

Uglify builds support `--obfuscation` / `-o` to shorten names even where code reads them as strings, `--disable-literals-optimization`, `--disable-namespaces-optimization`, and `--exclude-namespaces NAME...` to preserve selected names. The namespaces optimization also calls general API functions through short global aliases (`globals.A=@get_shell`, then `A()`) when that makes the build shorter; a function the program assigns, takes as a parameter or writes through `globals` keeps its name, and a write through `globals`, `locals` or `outer` with a computed key turns the aliases off. `--multiline-strings concatenation|join` overrides `multilineStrings`. Beautify builds support `--beautify-keep-parentheses`, `--beautify-indentation tab|whitespace`, and `--beautify-indentation-spaces N`.

Use `--strict-mode` to reject syntax extensions and `--file-extensions EXT...` to replace the import suffix list. `--silent` suppresses build progress, not errors.

## Source imports and directives

Greybel supports three different dependency mechanisms:

```greyscript
#include "lib/constants.src"
#import math from "lib/math.src"
import_code("lib/runtime.src")
```

- **`#include "path"`** inserts the file at that location. Included code shares the including file's scope and is included each time the directive appears. `#include type "path"` is a type-only include: it emits no runtime code, and the language server only reads the file's type declarations (`@vtype`, `@type`, `@property`, `@function`), not its variables.
- **`#import name from "path"`** evaluates a module once, caches it, and binds `name` to its `module.exports` map. Later imports reuse the cached module.
- **`import_code("path")`** keeps code in separate built files for Grey Hack's compiler. It does not return a `module.exports` object. Greybel supports relative and nested imports, rewrites paths for their in-game locations, and moves nested imports into the entry file where the game requires it. Keep `import_code` at the head of a file; nested import locations cannot otherwise be guaranteed.

Relative paths start at the importing file. For `import_code`, an absolute path starts at the project folder, or the entry file's folder without a project. Builds include all three dependency kinds; standalone transforms process only the supplied source and do not load those dependencies.

Other compile-time directives are:

```greyscript
name = #envar NAME
text = #inject "data.txt"
print #line
print #filename

#if DEBUG
print "debug"
#else
print "release"
#endif
```

`#envar` produces a string or `null` when unset. `#if` selects source at compile time: unset, empty, `0`, and `false` values are false; other values are true. `#inject` inserts a file's contents as a string literal, while `#line` and `#filename` identify the directive's source location.

### Syntax extensions

Compound assignments `+=`, `-=`, `*=`, `/=`, `%=`, and `^=` are supported generally; strict mode accepts them for current GreyScript and MiniScript 1.6.2 or later. List and map literals may also have trailing commas in both modes.

Outside strict mode, the parser and transpiler additionally support:

- `/* ... */` block comments;
- bitwise operators `|`, `&`, `<<`, `>>`, and `>>>` (`^` remains exponentiation);
- single-line `for` and `while` bodies;
- trailing commas in call argument lists;
- multiline string literals, which builds convert to Grey Hack-compatible code;
- parameter defaults that aren't literals, such as `function(items = [], count = items.len)`. The default is evaluated in the function on each call that omits the argument or passes `null`; builds write it as `if @items == null then items = []` at the start of the function;
- the `debugger` statement;
- string templates, `let`/`const`, `enum`, spread and map shorthand, destructuring, loop indexes, short functions, `switch`, ternary expressions, `??` and optional chains, `assert`, `export` and named imports, classes, and types in code.

The guide chapter [Syntax features](../greybel/ts/greybel-api-reference-web/guide/chapters/syntax-features.md) describes each of them with examples and what builds write for them. It is also shown in the GreyScript API reference (the `Greybel: API documentation` command in VS Code).

```greyscript
/* Greybel extensions are transformed by a build. */
total = 0
for n in range(1, 3) total += n
flags = 6 | 3
items = [1, 2,]
```

Enable `strictMode` or `--strict-mode` when source must be accepted directly by Grey Hack without a Greybel build.

To turn off single syntax features instead, list them under `syntax` in `greybel.json`, for example `"syntax": { "ternary": false, "classes": false }`. A feature not listed is on; the names are listed in the guide chapter. Builds, runs, the language server and the MCP check tool use the setting.

### Environment values

Builds read `build.variables`; runs and the REPL read `run.variables`. Both default to reading `.env` in the project folder. Files contain one `NAME=value` per line, with blank lines and lines beginning with `#` ignored. Missing configured files contribute no values.

```sh
greybel build src/main.src --env-files .env.local --env-vars DEBUG=1 NAME=Ada
greybel execute src/main.src --env-vars DEBUG=1
```

Values are applied in this order, with later values winning:

1. `variables.files`, in listed order;
2. `variables.values` from `greybel.json`;
3. `--env-files`, in command-line order;
4. `--env-vars` assignments.

These directives read Greybel's configured values, not arbitrary process environment variables. Do not put secrets into values that will be embedded in distributed source.

## Installers and in-game builds

Installer generation and auto-compilation are enabled by default. A build writes `installer0.src`, `installer1.src`, and so on as needed. These scripts recreate all built files in the game and are split at `build.installer.maxChars` (160,000 by default).

```sh
greybel build src/main.src --no-installer
greybel build src/main.src --installer --max-chars 160000 --no-auto-compile
greybel build src/main.src --create-ingame --ingame-directory /root/myapp/
```

`--auto-compile` compiles an installed or deployed build and deletes its source files after compilation; `--no-auto-compile` leaves those sources in place. `--allow-import` enables `allowImport` for auto-compiled installers and deployments when the compiled program needs to import code. `--no-installer` affects generated installer files, not deployment.

`--create-ingame` additionally deploys the build through the message hook on port 7777 by default; `--port` selects another port. Local output is still written. With auto-compile enabled, deployed sources are deleted after compilation. See [Grey Hack integration](greyhack-integration.md) before using game deployment.

## Execute and debug

```sh
greybel execute src/main.src
greybel execute src/main.src --seed demo --params alpha beta
greybel execute src/main.src --interactive
greybel execute src/main.src --debug --break src/main.src:12
greybel execute src/main.src --env-type in-game
```

With no file, the first project entry runs. `--params` / `-p` supplies the program's `params`; `--interactive` / `-i` instead prompts for one whitespace-separated line of parameters. `--program-name` changes the default `myprogram` binary name.

### Mock and shared worlds

The default `--env-type mock` generates a local simulated Grey Hack world. `--seed` / `-s` changes the default `test` seed; the same seed generates the same world. The mock world approximates Grey Hack; use an in-game run when exact game behavior matters. The local root shell is available as `get_shell("root", "test")`. Programs run as the player's user `test`; `--user root` or `run.user: "Root"` in `greybel.json` runs them as `root`, as after `sudo -s` in the game.

`execute`, `repl` and `mcp` run programs in the world server that the commands and VS Code windows of the machine share, on port 41740 of `127.0.0.1`. When none runs there, the command starts one for its seed in the background; it shuts down after 600 seconds without connections. When the running server has another seed, the program gets a world of its own and the command says so.

| Option | Effect |
|---|---|
| `--world-port PORT` | Uses the shared server on another port. |
| `--world-idle-timeout SECONDS` | Sets how long a server the command starts stays up without connections; `0` keeps it running. |
| `--own-world` | Runs each program in a world of its own, which is discarded afterwards. The `GREYBEL_OWN_WORLD` environment variable does the same. |
| `--world ADDRESS` | Uses the world server at `ADDRESS` instead. |
| `--worlds FOLDER` | Sets the folder in which a server the command starts saves its worlds. The `GREYBEL_WORLDS` environment variable does the same. |

World servers save the world of each seed, so it continues where it stopped when a server starts again. The worlds are kept in `greybel/worlds` in the user's data folder (`~/Library/Application Support` on macOS, `%APPDATA%` on Windows, `$XDG_DATA_HOME` or `~/.local/share` elsewhere), listed by seed in its `worlds.json`. While one server uses a seed's saved world, a second server of the same seed runs its world in memory only.

`worlds.json` records the database layout of each saved world: the schema version of each part of the world and a hash of its tables. A world saved by an older Greybel version is migrated when its parts have newer versions with migrations. A world whose tables changed without a version step is renamed with an `.outdated` suffix and generated again, as is a world that fails to load (`.unreadable`). A world saved by a newer Greybel version is not changed; the server runs a world in memory instead.

```sh
greybel world reset                 # generate the shared world again; running programs are interrupted
greybel world serve --seed test     # serve a world yourself; see --help for --port, --idle-timeout, --memory
```

A run compiles the program and its dependencies into a binary in its user's home folder, normally `~/myprogram`, so `program_path` and `launch(program_path)` work. In the game this binary exists only for the run.

### In-game execution

`--env-type in-game` (alias `ingame`) runs in Grey Hack through the message hook; `--port` changes its default port 7777. Execution and debugging in the game require **single-player**. The message hook is not needed for mock runs.

### Terminal debugger

`--debug` / `-d` stops at `debugger` statements and breakpoints. Each `--break FILE:LINE` adds a breakpoint and implies debug mode. At a stop, enter `help` for commands. Common commands are `continue`, `next`, `step`, `out`, `stack`, `vars`, and `terminate`; any other input is evaluated as code in the current frame.

## Import files without building

```sh
greybel import scripts
greybel import src/main.src --ingame-directory /root/myapp/ --port 7777
```

`greybel import TARGET` uploads a file or a folder of files unchanged through the message hook. It does not transpile, bundle, or auto-compile them. The destination defaults to the project's `build.ingameDirectory` (`/root/` by default), and `--profile` is honored. Use `--silent` to suppress the completion message. See [Grey Hack integration](greyhack-integration.md) for setup.

## REPL

```sh
greybel repl
greybel repl --seed demo --profile development
```

The REPL evaluates GreyScript in one persistent mock program, retaining variables and functions between entries. Non-`null` expression results are printed. Incomplete blocks continue on following lines; an empty line submits accumulated multiline input. Relative imports resolve from the current directory, and the project's run variables and source settings apply.

Enter `exit` or send end-of-input to quit. Evaluation errors are reported without discarding the session. The standalone REPL runs only in the mock world.

## Web editor

```sh
greybel ui
greybel ui --url https://editor.greyscript.org
```

This opens the web editor in the system browser; it does not start a local web server. The default URL is `https://editor.greyscript.org`.

## MCP server

Configure an MCP client to launch this as a **stdio** server:

```sh
greybel mcp
```

The server exposes `api_search`, `api_reference`, `check`, `transform`, `build`, and `run`. These tools use `greybel.json` where applicable, and build, transform, and run requests can select a project profile.

For navigating a project's code it also exposes `document_symbols`, `find_symbol`, `find_references`, `describe`, `call_hierarchy`, and `rename`. They analyse every script of the project (the folder of the nearest `greybel.json`, else the given folder) as the language server does. Symbols are addressed by name path, such as `App.utils.format` or `main.count` for a local of `main`, or by `path`, `line`, and `column`; an ambiguous name returns the matching name paths. `rename` writes the changed files unless `dry_run` is set. The server uses its process working directory for standalone code and relative paths.

Game access is disabled by default. To permit in-game `run` requests and add the `import_into_game` tool, start it with:

```sh
greybel mcp --allow-game --port 7777
```

Only enable `--allow-game` for a client that should be allowed to execute programs and import files in Grey Hack. In-game MCP execution also requires single-player and the message hook.
