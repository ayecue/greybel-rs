# Greybel for VS Code

Greybel VS adds GreyScript editing, builds, imports, execution, and debugging to VS Code. It supports `.src`, `.gs`, and `.ms` files in desktop VS Code and compatible web editors.

For command-line use, see the [CLI guide](cli.md). For the Grey Hack message hook and game setup, see [Grey Hack integration](greyhack-integration.md).

## Install

1. Install **Greybel VS** from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=ayecue.greybel-vs) or [Open VSX](https://open-vsx.org/extension/ayecue/greybel-vs). VS Code 1.90 or later is required.
2. Open the folder containing your GreyScript project.
3. Open a `.src`, `.gs`, or `.ms` file to activate the extension.

Desktop packages contain the required native binaries. The web extension runs the language server, builder, runtime, and mock world in WebAssembly workers. Importing into Grey Hack and the in-game run environment require the desktop extension.

## Quick start

1. Create `main.src`:

   ```js
   print("Hello from Greybel")
   ```

2. Run **Greybel: Run file from context** from the Command Palette or editor context menu.
3. Enter program arguments when prompted, or leave the prompt empty.
4. Use **Greybel: Build file from context** to write the build to `build/`.

Add a [`greybel.json`](#project-configuration) when several files share an entry point or build settings.

## Features

- Syntax highlighting, completion, signature help, hover documentation, diagnostics, formatting, symbols, definitions, and references.
- Rename, which also renames a parameter in its function's `@param` tags and also works from the name in a `@param` tag.
- Hover on the path of `#include`, `#include type`, `#import` or `import_code`: "Import statement for" the file's path, and what it contains at its top level, for example "Contains 3 imports, 1 namespace, 12 functions". A namespace is a map literal assigned to a name or member path (`Proxima.utils = {}`). To describe a file yourself, start it with a comment; its text is shown below the counts. Ctrl+click (Cmd+click on macOS) or Go to Definition on the path opens the file.
- Hover on a type name, in code (`port: number`, `type Id = ...`, `as T`, `implements T`) or in a doc comment (`{T}`, `@type T`, `@vtype T`, `@typedef {...} T`, `@extends T`): the type's members, or what an alias, `@typedef` or `enum` stands for.
- Hover on class fields in the class body, including instance fields and `private` or `static` ones.
- `@deprecated [text]` in a function's doc comment: hovers start with "Deprecated", completions and uses show struck through, and each use gets a `deprecated` hint. Unused locals and parameters and unreachable code show faded.
- Call hierarchy (Show Call Hierarchy): the callers of a function, including the top level of files, and the script functions it calls. A read of a function variable counts as a call, as in MiniScript; `@name` does not.
- Document and workspace symbols include `@type`/`@vtype` declarations, with their `@property` and `@function` tags as children. Member assignments such as `App.utils.format = ...` are listed under the symbol of `App.utils` (or `App`) when the file assigns it.
- Go to Type Definition: from a value to the `@type`/`@vtype` tag of its custom type, the map literal that created a map, or the class an instance was made from with `new`.
- Keyword completion: block snippets (`if`, `if else`, `for`, `while`, `function`) and statement keywords at the start of a line, including `break`/`continue` inside loops and the `end` of the innermost open block; `true`, `false`, `null`, `new`, `not` and a `function` snippet in expressions; `and`, `or`, `isa` after an operand.
- Completion of the string keys of a map inside `m["..."]`, and inside `//` comments: tag names after `@`, type names inside `{...}`, and the parameter names of the documented function after `@param`.
- Color swatches and a color picker for the values of `<color=...>`, `<mark=...>` and `<#...>` tags in strings (hex forms and the TextMesh color names); a picked color is written as `#RRGGBB` or `#RRGGBBAA`.
- Folding of function bodies, `if`/`else` clauses, loops, multi-line lists and maps, comment lines and import lines, and Expand/Shrink Selection by syntax node.
- Inlay hints: the type of a variable at its first assignment, parameter names before arguments that don't have the parameter's name, the types of parameters without a written type, and return types of functions without a `@return` tag. `greybel.typeAnalyzer.inlayHints.variableTypes`, `.parameterNames`, `.parameterTypes` and `.returnTypes` turn each off; the editor's `editor.inlayHints.enabled` turns all of them off.
- Quick fixes: add an `#include` or `import_code` for a name another workspace file defines; change a name that isn't found to a close name in scope, a close member, `self.name` inside a method, or `return`/`break`/`continue`; remove or rename (`_name`) an unused local; rename an unused parameter; remove unreachable statements; change a misspelled code in an `@ignore` comment to a close one; ignore a diagnostic on its line or in the file.
- Default, minified, and formatted transformations.
- Project builds with dependency bundling, installer generation, and optional deployment to Grey Hack.
- A generated mock Grey Hack world or an in-game run environment.
- Breakpoints, conditional breakpoints, hit conditions, logpoints, stepping, variable inspection and editing, and paused evaluation.
- A pseudo-terminal for program input and output, plus a TextMesh Pro output preview.
- API documentation, game-script snippets, comment-based type documentation, sharing, and navigation to the next error.

## Commands

Open the Command Palette with `Ctrl+Shift+P` (`Cmd+Shift+P` on macOS). Most commands also appear in the editor or Explorer context menu.

| Command | Action |
|---|---|
| **Greybel: Build root file** | Build the first path in `greybel.json`'s `entries`. |
| **Greybel: Build file from context** | Build the active or selected file. |
| **Greybel: Run root file** / **Debug root file** | Run or debug the first configured entry. |
| **Greybel: Run file from context** / **Debug file from context** | Run or debug the active or selected file. |
| **Greybel: Import file into the game** | Import the active file or a file/folder selected in the Explorer without building it. Desktop only. |
| **Greybel: Preview output** | Open the TextMesh Pro output preview. |
| **Greybel: API documentation** | Open the searchable GreyScript API reference. |
| **Greybel: Snippets** | Open a game script snippet in a new document. |
| **Greybel: Share** | Save the active file with every file it imports and the project's `greybel.json` through the editor service, and copy the web editor link. |
| **Greybel: Select profile** | Select the active project's build/run profile. |
| **Greybel: Reset the mock world** | Discard the mock world, including its saved state, and generate it again. |

The editor's **Greybel: Transform** submenu contains **Transform**, **Uglify**, **Beautify**, and **Transform to clipboard**. **Greybel: Go to error** in the editor context menu selects the next error in the current file.

## Project configuration

The CLI, MCP server, language server, and VS Code extension read the same optional `greybel.json`. Put it in the workspace folder root so VS Code can synchronize it with workspace settings. Paths are relative to the project folder.

```json
{
  "entries": ["src/main.src"],
  "extensions": ["src", "gs", "ms"],
  "build": {
    "mode": "Default",
    "output": "build",
    "variables": {
      "files": [".env"],
      "values": { "NAME": "value" }
    },
    "installer": {
      "active": true,
      "maxChars": 160000,
      "allowImport": false
    }
  },
  "run": {
    "seed": "test",
    "params": [],
    "programName": "myprogram",
    "memoryLimit": 1024,
    "variables": {
      "files": [".env"],
      "values": {}
    }
  },
  "profiles": {
    "release": {
      "build": { "mode": "Uglify" }
    },
    "release-obfuscated": {
      "extends": "release",
      "build": { "obfuscation": true }
    }
  }
}
```

Every field is optional. VS Code provides validation and completion for this file.

### Main fields

| Field | Meaning |
|---|---|
| `entries` | Entry files for builds and runs. VS Code's root-file commands use the first entry. |
| `extensions` | Extensions an import may omit, tried in order. Default: `src`, `gs`, `ms`. |
| `strictMode` | Reject Greybel conveniences that Grey Hack's compiler does not accept. Default: `false`. |
| `syntax` | Greybel syntax features turned on or off by name, for example `{ "ternary": false }`; a feature not listed is on. Synced with `greybel.syntax`. The names are listed in the [CLI guide](cli.md). |
| `exclude` | Glob excluded from workspace type analysis. Default: `**/{node_modules,.*}/**`. |
| `diagnostics` | Diagnostic levels by code, for example `{ "used-before-defined": "off", "shadowing": "error" }`: `off` drops a diagnostic, and `error`, `warning`, `information`, or `hint` replaces its default severity. Synced with `greybel.typeAnalyzer.diagnostics`. The codes are listed under [Diagnostics](#diagnostics). |
| `format` | Formatting for the editor and beautified builds: `indentation`, `indentationSpaces`, and `retainParentheses`. |
| `build` | Transformation, output, deployment, installer, and build-variable settings. |
| `run` | Mock-world, argument, program-name, memory-limit, and run-variable settings. |
| `profiles` | Named overrides of `build` and `run`. |

### Diagnostics

Each diagnostic has a code, shown in the Problems panel, and a default severity. Outside `strictMode`, `call-signature-mismatch` is off unless `diagnostics` sets it.

| Code | Default | Reports |
|---|---|---|
| `syntax-error` | error | Code that doesn't parse. |
| `unresolved-reference` | error | A name that is neither a variable nor a native. |
| `invalid-annotation` | warning | A comment tag the analyzer can't read. |
| `call-arity-mismatch` | warning | A call with more arguments than parameters. |
| `missing-import` | error | An import of a file that doesn't exist. |
| `cyclic-import` | error | Imports leading back to the importing file. |
| `dead-code-after-terminator` | warning | Code after `return`, `break`, `continue`, a call of `exit`, or branches that all end. |
| `map-literal-duplicate-keys` | error | A map literal giving a key twice. |
| `call-of-non-callable` | error | Calling a value that is never a function. |
| `index-of-non-indexable` | error | Indexing a value that can't be indexed. |
| `call-signature-mismatch` | warning | An argument its annotated parameter doesn't accept. |
| `type-mismatch` | error | A value its template bound doesn't accept. |
| `annotation-signature-mismatch` | error | A parameter or return value its tag doesn't accept. |
| `unused-local` | warning | A local variable that is never read. |
| `unused-parameter` | warning | A parameter that is never read. |
| `shadowing` | warning | A local hiding an outer variable. |
| `self-assignment` | warning | Assigning a variable to itself. |
| `constant-condition` | warning | A condition that is always true or always false. |
| `used-before-defined` | warning | Reading a variable before any assignment to it can have run. |
| `cyclic-inheritance` | error | An `isa` chain leading back to itself. |
| `cyclic-type-definition` | error | Type tags extending themselves. |
| `ignored-import-code` | warning | `import_code` in a comment or string, which Grey Hack still imports. |
| `misplaced-import-code` | warning | `import_code` after code in a file the build moves it out of. |
| `native-override` | warning | Assigning a name of a native function, such as `print`. |
| `unknown-member` | warning | Reading a member that a map never has: the map is made in the script by a map literal or `new`, its keys are only set by name, and it is never passed to a function the analyzer can't see into. |
| `private-member` | warning | Reading or assigning a `private` class member outside the methods of its class and of the classes extending it. |
| `missing-implementation` | warning | A class's `implements` type has a member the class (or a class it extends) doesn't define, defines with another type, or isn't a known type. |
| `missing-case` | warning | A `switch` without `else` whose subject is a union of literal types (such as an `enum` type) has no case for some of them. |
| `impossible-case` | warning | A `case` value the `switch` subject never equals: a literal missing from a subject whose type is a union of literal types, or a value of another kind than the subject. |
| `optional-member` | warning | Reading an optional member of a type (`timeout?: number` in a `type`, `@property {number} [timeout]`) where neither `value.hasIndex("timeout")` is true nor the member was assigned on every path before. |
| `readonly-member` | warning | Assigning a `readonly` member (`readonly id: T` in a `type`, `@property {readonly T} id`) through a value of its type. |
| `invalid-operation` | warning | An operator whose operand types never work together: it always gives `null` (such as `number - string`, `null + 1` or `-"x"`) or raises a type mismatch (such as `list + number` or `string * string`). Reported only when no combination of the operand types works. |
| `deprecated` | hint | A use of a function whose doc comment has a `@deprecated` tag; the tag's text follows the message. Editors show the use struck through. |
| `const-assignment` | error | An assignment to a variable declared with `const`, including `+=` and `globals.NAME =`. |
| `duplicate-declaration` | error | A second `let` or `const` for the same variable. |
| `missing-let` | warning, off by default | The first assignment to a variable in a file without `let` or `const`. Parameters, `for` variables and `#import` names count as declared. Turn it on with a level in `diagnostics`, for example `{"missing-let": "warning"}`. |
| `implicit-any` | off | A parameter without a type (in code or `@param`) or default, a function whose return type isn't written and is inferred as `any`, or a variable assigned an `any` value without a type. |

### Profiles

Use **Greybel: Select profile** or the profile item in the status bar. The selection is stored as `greybel.profile`, not in `greybel.json`.

A profile replaces the base values it specifies. Objects merge key by key; arrays and scalar values replace the base value. `extends` applies another profile before the current profile's overrides.

### Settings synchronization

Project-backed settings synchronize between `greybel.json` and VS Code's workspace/workspace-folder settings:

- On startup, values in `greybel.json` take precedence.
- Changing a project-backed setting at workspace or folder scope writes it to `greybel.json`.
- Editing `greybel.json` updates the corresponding folder settings.
- User settings apply when no project manifest provides the configuration.
- Invalid JSON is left unchanged and is not synchronized.

Editor features and user-specific integration settings are not written to the manifest. These include `greybel.formatter`, `greybel.autocomplete`, `greybel.hoverdocs`, `greybel.diagnostic`, `greybel.interpreter.environmentType`, both message-hook ports, `greybel.interpreter.useDefaultArgs`, `greybel.interpreter.silenceErrorPopups`, `greybel.transpiler.watch`, `greybel.transpiler.ingameDirectoryPrompt`, and `greybel.createIngame.active`.

## Transform and build

### Transform a file

The transform commands operate on the active editor's text. They do not bundle dependencies.

- **Transform** uses `build.mode` and replaces the editor contents.
- **Uglify** or **Beautify** overrides the configured mode for that transformation.
- **Transform to clipboard** uses the configured mode without changing the file.

`Default` writes the code without comments or minification. `Uglify` minifies code and optimizes literals and namespaces. `Beautify` formats code using `format`.

Uglify settings include `disableLiteralsOptimization`, `disableNamespacesOptimization`, `excludedNamespaces`, and `obfuscation`. The namespaces optimization also calls general API functions through short global aliases when that makes the build shorter. `multilineStrings` (`greybel.transpiler.multilineStrings`) sets how builds and runs write strings with line breaks: `Replace` (default), which writes the string on one line with a marker character replaced by `char(10)`, `Concatenation` or `Join`. `Replace` keeps a file about the size of its source; the other two can push an imported file past Grey Hack's 160,000 characters per file. Obfuscation also shortens names read as strings, such as map keys; enable it only when the program does not depend on those names remaining unchanged.

Both builds and runs read `#envar` values from `variables.files` first and `variables.values` second. Later values override earlier values. Variable files contain `NAME=value` lines and are relative to the project folder; missing files are ignored.

### Build a project

**Build file from context** starts at the selected file. **Build root file** starts at the first configured entry. A build saves open files, resolves dependencies, and writes the result to `build.output` (default: `build`). The output folder is replaced on each build.

For a single-file build, `build.outputName` overrides the built entry file's name. Include its extension if the build will be auto-compiled in Grey Hack.

Enable `greybel.transpiler.watch` to rebuild the root file when project sources change. Files under the build output folder do not trigger another build.

![Greybel build output](../apps/ts/greybel-vs/assets/build.png)

### Dependencies

Greybel supports relative imports and resolves omitted extensions in the order given by `extensions`. Absolute import paths start at the project folder.

- `#include "path/to/file.src"` inserts the file's contents at each include. Its declarations share the including file's global scope.
- `#import utils from "path/to/file.src"` evaluates the module once, caches it, and binds `utils` to its `module.exports` map. The module is bundled into the root file.
- `import_code("path/to/file.src")` keeps code in separate in-game files and supports nested imports. Prefer it when a bundled root file would exceed Grey Hack's character limit. Nested imports move into the entry file, so keep `import_code` at the head of a file; other placement cannot be guaranteed.

Builds report dependency cycles. At runtime, the interpreter loads dependencies dynamically and supports cyclic dependencies.

For compile-time directives and syntax extensions, see [Source imports and directives](cli.md#source-imports-and-directives) and [Syntax extensions](cli.md#syntax-extensions).

### Installers

Installer generation is enabled by default. A build writes `installer0.src` and additional numbered installers when `build.installer.maxChars` requires splitting. Copy the installer code into Grey Hack, then compile and run it to recreate the build there. Grey Hack allows at most 160,000 characters per file.

- `build.installer.maxChars` sets the split limit; the default is `160000`.
- `build.autoCompile` makes the installer compile the program and delete its source files. Default: `true`.
- `build.installer.allowImport` permits the auto-compiled program to import code at runtime.
- `build.ingameDirectory` sets the destination and run folder in Grey Hack. Default: `/root/`.

### Import or deploy to Grey Hack

Set up the message hook as described in [Grey Hack integration](greyhack-integration.md) before using these desktop-only features.

- Set `greybel.createIngame.active` to deploy each build; local build output is also written. `greybel.createIngame.port` selects the message-hook port (default: `7777`). Deployment uses `build.deploy.allowImport` when auto-compile is enabled.
- Use **Greybel: Import file into the game** to copy a file or folder without transforming or bundling it. Folder contents retain their paths relative to the selected folder.

Both features use `build.ingameDirectory`. Enable `greybel.transpiler.ingameDirectoryPrompt` to choose the destination for each build or import.

## Run and debug

### Mock environment

The default `Mock` environment generates computers, networks, file systems, services, and other world state from `run.seed`. The same seed generates the same initial world. `get_shell("root", "test")` returns the local root shell. Programs run as the player's user `test`; `run.user` (setting `greybel.interpreter.user`) set to `Root` runs them as `root`, as after `sudo -s` in the game.

Debug sessions use the world server that the VS Code windows and `greybel` commands of the machine share, on the port of `greybel.world.port` (default `41740`). The first session starts it when none runs; it shuts down after `greybel.world.idleTimeout` seconds without connections (default `600`, `0` never stops it). When it runs another seed, the window starts a world server of its own for its seed; `greybel.world.port` `0` always does. Changes persist between runs, and world servers save each seed's world in `greybel/worlds` of the user's data folder, or in `greybel.world.folder`, so it continues after restarts; see the [CLI guide](cli.md#mock-and-shared-worlds). **Greybel: Reset the mock world** generates the world again and interrupts the programs running in it, also those of other windows. The web extension keeps its world in memory. `run.memoryLimit` limits a program's memory in MiB; `run.programName` sets the binary name used inside the world. `run.lineLimits` (settings `greybel.interpreter.lineLimits.textPerLine` and `.tagsPerLine`) replaces the game's rule for cutting long printed lines with the one of the greyhack-customizable-output mod, in the terminal and the output preview.

The mock environment is local and does not require Grey Hack. Some game-specific behavior can differ from the real game.

### In-game environment

Set `greybel.interpreter.environmentType` to `In-Game` to execute in Grey Hack through the message hook. `greybel.interpreter.port` selects its port (default: `7777`). Execution and debugging require **single-player**, the desktop extension, and the setup in [Grey Hack integration](greyhack-integration.md).

### Arguments and environment variables

By default, each run prompts for arguments and splits the entered text on spaces. Set `greybel.interpreter.useDefaultArgs` to `true` to skip the prompt and use `run.params` instead.

`run.variables` supplies `#envar` values during execution, independently of `build.variables`.

### Debugging

Use a debug command to enable breakpoints. The debugger supports stepping, pausing, stack traces, variable inspection and editing, conditional and hit-count breakpoints, and logpoints. Evaluate code in the Debug Console while the program is stopped at a breakpoint or paused.

Set breakpoints on executable, non-empty lines. You can also create a `launch.json` configuration:

```json
{
  "type": "greyscript",
  "request": "launch",
  "name": "Debug current entry",
  "program": "${workspaceFolder}/src/main.src"
}
```

![A GreyScript breakpoint](../apps/ts/greybel-vs/assets/breakpoint.png)

### Terminal and output preview

Program input and output use the `greybel` pseudo-terminal. TextMesh Pro rich text is converted to ANSI where possible, so some formatting can be lost.

Open **Greybel: Preview output** before a run to see the same output rendered with TextMesh Pro formatting in a webview.

![TextMesh Pro output preview](../apps/ts/greybel-vs/assets/preview.gif)

## Comment documentation

Comment annotations add signatures and types to hover information, completion, and diagnostics.

```js
// Greet a user.
// @example greet("Ada", 1)
// @param {string} name - Display name.
// @param {string|number} id - User identifier.
// @returns {string} - The greeting.
greet = function(name, id)
  return "Hello, " + name
end function
```

A function type is written `(name: number, string) => string` or `function(name: number, string): string`; parameter names are optional, and without a return type the function returns `any`:

```js
// @param {(item: map) => string} format
printAll = function(items, format)
  for item in items
    print format(item)
  end for
end function
```

Strings and numbers can be types (`"tcp"|"udp"`, `22|80`); a literal argument is checked against them, and other strings and numbers fit them. An `enum` name is the union of its values. `[string, number]` is a list of those types. A return type `value is Dog` makes a function a type guard: `if isDog(a) then` narrows `a` to `Dog`, and its `else` removes `Dog`.

Use `@type` and `@property` to document a runtime object type. Use `@define` to give a value a documented type.

```js
// @type User
// @property {string} name
User = { "name": "" }

// @define {User}
currentUser = new User
```

Virtual types describe tooling-only interfaces without requiring a runtime implementation:

```js
// @vtype Repository
// @extends map
// @property {string} name
// @function find
  // @params {string} id
  // @returns {map}

// @define {Repository}
users = {}
```

Supported documentation annotations include `@description`, `@example`, `@param`/`@params`, `@returns`, `@deprecated`, `@type`, `@vtype`, `@typedef`, `@extends`, `@property`, `@function`, and `@define`. `@typedef {string|number} Id` names a type, like `type Id = string|number`; `@typedef {[T, T]} Pair<T>` takes type parameters.

### Ignoring diagnostics

`// @ignore` on its own line ignores the diagnostics of the next line with code; after code, `x = 1 // @ignore` ignores its own line. `// @ignore-file` anywhere in a file ignores the entire file. Each takes optional [diagnostic codes](#diagnostics), separated by commas or spaces, and then ignores only those:

```greyscript
// @ignore native-override
print = function(text)
  globals.log.push(text)
end function

result = undocumented_call // @ignore unresolved-reference
```

A code that names no diagnostic is reported as `invalid-annotation`.

The quick fixes "Ignore ... on this line" and "Ignore ... in this file" write these comments, adding the code to an existing comment that already names codes.

## Other tools

### API reference

**Greybel: API documentation** opens the GreyScript API reference beside the editor. Examples can be inserted into the active document.

![GreyScript API reference](../apps/ts/greybel-vs/assets/api-browser.png)

### Snippets, sharing, and errors

- **Greybel: Snippets** opens a selected game script in a new GreyScript document.
- **Greybel: Share** uploads the active file, the files it imports (`#include`, `#include type`, `#import`, `import_code`, also through other imported files) and the project's `greybel.json` to the editor service as one workspace, and copies the link that opens it in the web editor. Paths are relative to the project folder; unsaved changes are included.
- **Greybel: Go to error** moves to the next diagnostic error, wrapping to the first error when necessary.

## Troubleshooting

### Greybel commands are missing

Open a `.src`, `.gs`, or `.ms` file. The extension activates for the GreyScript language, and many context commands are visible only for that language.

### The root-file command cannot find an entry

Set `entries` in `greybel.json` or use a file-from-context command. Entry paths are relative to the project folder; VS Code uses the first entry.

### An import cannot be resolved

Check the path's letter case and the extension order in `extensions`. Relative paths start at the importing file; absolute paths start at the project folder. Use a build rather than a transform when dependencies must be bundled.

### `greybel.json` changes do not apply

Confirm that the file is a valid JSON object in the workspace folder root. VS Code warns when invalid JSON prevents synchronization. Check for a selected profile that overrides the changed setting.

### A mock run sees old files or world state

Programs share one saved mock world, also with other windows and `greybel` commands. Run **Greybel: Reset the mock world** to discard its changes. This does not delete source files on disk.

### Grey Hack import, deployment, or in-game execution fails

Use the desktop extension, start Grey Hack, and verify the message-hook installation. Match `greybel.createIngame.port` or `greybel.interpreter.port` to the hook's port. See [Grey Hack integration](greyhack-integration.md) for installation and connection troubleshooting.

### Execution error popups are distracting

Set `greybel.interpreter.silenceErrorPopups` to `true`. Errors remain visible in the pseudo-terminal.

## Related documentation

- [CLI guide](cli.md)
- [Grey Hack integration](greyhack-integration.md)
- [Greybel VS changelog](../apps/ts/greybel-vs/CHANGELOG.md)
- [GreyScript API documentation](https://documentation.greyscript.org)
