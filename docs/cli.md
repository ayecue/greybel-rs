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

Build settings also include `outputName` for a single entry, `disableLiteralsOptimization`, `disableNamespacesOptimization`, and `excludedNamespaces`. `memoryLimit` is in MiB and applies to mock execution. Unknown configuration fields are rejected. The complete editor schema is [greybel.schema.json](../apps/ts/greybel-vs/schemas/greybel.schema.json).

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

Uglify builds support `--obfuscation` / `-o` to shorten names even where code reads them as strings, `--disable-literals-optimization`, `--disable-namespaces-optimization`, and `--exclude-namespaces NAME...` to preserve selected names. Beautify builds support `--beautify-keep-parentheses`, `--beautify-indentation tab|whitespace`, and `--beautify-indentation-spaces N`.

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
- the `debugger` statement.
- string templates: `#"Hello {name}, {items.len} items"`. Each `{...}` holds an expression; `{{` and `}}` write braces and `""` a quote. Builds write `"Hello " + (name) + ", " + (items.len) + " items"`, starting with `""` when the template starts with an expression, so the result is always a string;
- `enum Name` with one `Entry` or `Entry = value` per line and `end enum`: builds write `Name = {"Entry": 0, ...}`. An entry without a value gets the previous number plus 1 (0 first); values are number or string literals, and an entry after a string value needs its own value. With `const enum`, builds also write the value in place of `Name.Entry` reads in every file of the build, except in a file that also uses `Name` as a variable or parameter;
- spread in list and map literals and map shorthand: `[...a, 1]`, `{...defaults, "verbose": 1}`, `{name, port}`. Builds write `([] + a + [1])`, `({} + defaults + {"verbose": 1})` and `{"name": name, "port": port}`; later map entries win;
- `assert condition` and `assert condition, message`: builds write `if not (condition) then exit(message)`; without a message the program exits with "Assertion failed at file:line". Setting the build variable `NO_ASSERTS` (for example `--env-vars NO_ASSERTS=1`) removes asserts from builds and from `greybel execute`;
- short functions: `(x) => x * 2`, or `(line) =>` at the end of a line with the body up to `end function`. Builds write `function(x)`, `return x * 2`, `end function`; in call arguments and literals they use the form GreyScript accepts there (`items.map(function(x))` with the body on the following lines);
- destructuring statements: `[user, host] = address.split("@")`, `[first, ...others] = items`, `{name, port as p, timeout = 30} = config`, nested patterns and `let`/`const` before them. Builds write one assignment per name (`user = @__d0[0]`, `name = @config.name`), through a `__d` temporary when the right side isn't a variable; a default is used when the element or key is missing or `null`;
- `for` headers with an index or a pattern: `for i, item in items` (`i` is the item's index, `__item_idx`), `for [x, y] in points` and `for {key, value} in map` (each item destructured, like a pattern assignment), or both (`for i, [x, y] in points`);
- `switch subject` with `case value, ...`, `case is Type` and `else` clauses, closed by `end switch`. Builds write an `if`/`else if` chain comparing `@subject` (a `__s` temporary when the subject isn't a variable); there is no fall-through, and `break` keeps its meaning for an enclosing loop;
- `#import {parse, format as fmt} from "path"`: builds write `__m = __REQUIRE(...)` followed by `parse = @__m.parse` and `fmt = @__m.format`. The module still runs once; the names are copies of the exports when the import runs;
- `export name = value` at the top of a file loaded with `#import`: builds write the assignment and `module.exports.name = @name`, with `module.exports = {}` before the file's first export;
- types in code: `function(ip: string, port: number = 22): map|null`, `function<T>(items: list<T>): T`, `(x: number) => x * 2`, `let port: number = 22`, `value as list<string>` and `value!` (not `null`). They use the doc comment type language, count like `@param`/`@returns`/`@template`/`@define` in the type analyzer, and builds remove them. `type Id = string|number` declares an alias and `type Repository extends map` with `name: string`, `timeout?: number`, `readonly id: Id` and `find(id: Id): map|null` lines up to `end type` declares a type like `@vtype`;
- classes: `class Dog extends Animal` with instance fields (`name = value`), `static` fields shared by all instances (`static sound = "woof"`) and methods (`bark(times)` up to `end function`) up to `end class`; `Dog.create(arguments)` makes an instance with the `constructor` method, which assigns every instance field (`self.name = value`). Builds write `Dog = new Animal` and `Dog.member = value` for methods and `static` fields;
- ternary expressions `enabled ? "on" : "off"`, `a ?? b` (`a` unless it is `null`) and optional chains `a?.b`, `a?.[i]`, `f?.(x)` (`null` for the whole chain when the base is `null`). Builds compute them into `__tN` temporaries before their statement; on the right side of `and`/`or`, in a parameter default or an `assert` message they are a build error;
- `let` and `const` before an assignment to a name: `let count = 0`, `const LIMIT = 10`. Builds write the plain assignment; the type analyzer reports assignments to constants and, when the `missing-let` diagnostic is on, assignments that create a variable without them. `let` and `const` are still valid names, as in `let = 1`;
- hex, octal and binary number literals and `_` between digits: `0xFF`, `0o17`, `0b1010`, `1_000_000`. Builds write them as decimal numbers.

```greyscript
/* Greybel extensions are transformed by a build. */
total = 0
for n in range(1, 3) total += n
flags = 6 | 3
items = [1, 2,]
```

Enable `strictMode` or `--strict-mode` when source must be accepted directly by Grey Hack without a Greybel build.

To turn off single syntax features instead, list them under `syntax` in `greybel.json`, for example `"syntax": { "ternary": false, "classes": false }`. A feature not listed is on. The names are `number-literals`, `templates`, `declarations`, `enum`, `spread`, `map-shorthand`, `assert`, `short-functions`, `destructuring-assignments`, `for-index`, `switch`, `export`, `ternary`, `coalesce`, `optional-chains`, `classes` and `types`. Builds, runs, the language server and the MCP check tool use the setting.

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

The default `--env-type mock` generates a local simulated Grey Hack world. `--seed` / `-s` changes the default `test` seed; the same seed generates the same world. The mock world approximates Grey Hack; use an in-game run when exact game behavior matters. The local root shell is available as `get_shell("root", "test")`.

To share one simulated world among separate CLI processes, connect to a running `greyhack-world-server` with:

```sh
greybel execute src/main.src --world ADDRESS
```

`--world` replaces seed-generated world creation and cannot be combined with `--seed`.

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

The server exposes `api_search`, `api_reference`, `check`, `transform`, `build`, and `run`. These tools use `greybel.json` where applicable, and build, transform, and run requests can select a project profile. The server uses its process working directory for standalone code and relative paths.

Game access is disabled by default. To permit in-game `run` requests and add the `import_into_game` tool, start it with:

```sh
greybel mcp --allow-game --port 7777
```

Only enable `--allow-game` for a client that should be allowed to execute programs and import files in Grey Hack. In-game MCP execution also requires single-player and the message hook.
