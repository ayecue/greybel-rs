# Greybel

Greybel extends GreyScript with compile-time directives and syntax for development outside the game. Greybel's interpreter executes these extensions directly, and a build transforms them into source that Grey Hack accepts. Strict mode (`strictMode` in `greybel.json`) accepts only what Grey Hack's own compiler accepts, so the code also compiles in the game without a build.

## Modules

`#include "path"` inserts another source file at the directive's location. Included code shares the including file's scope and does not create a module boundary.

`#import name from "path"` evaluates a module once, caches it, and binds `name` to the map assigned to `module.exports` by that module. Repeated imports reuse the cached result. Relative paths are resolved from the importing source file.

```
#import math from "lib/math.src"
print math.square(5) // 25
#include "lib/version.src"
print VERSION // 1
```

The imported `lib/math.src` defines and exports its public interface:

```
square = function(n)
    return n * n
end function
module.exports = {"square": @square}
```

The included `lib/version.src` in this example contains `VERSION = 1`. A build incorporates included and imported code into its output. For code that should remain in separate files in the game, use [import_code](#GENERAL_IMPORT_CODE) as described in [GreyScript programs](#guide-greyscript).

`#include type "path"` reads only the type declarations of a file (`@type`, `@vtype`, `@typedef`, `type` statements; see [Documentation comments](#guide-documentation-comments)) for the type analyzer and editor. It writes no code and runs nothing, so a file can share types without including its values.

`//include "path"` and `//import name from "path"` work like `#include` and `#import`, but read as comments to Grey Hack's editor and compiler.

```
//include "lib/version.src"
print VERSION // 1
```

## Environment

`#envar NAME` reads a configured build or run variable as a string. An unset variable produces `null`. In the CLI, supply values with `--env-vars NAME=value` or `--env-files`.

`#if NAME`, `#else`, and `#endif` conditionally include source according to whether the variable is set to a value other than `0` or `false`. These directives control compilation; they are not runtime `if` statements.

With `GREETING=hello` and `DEBUG=1` configured, the following program selects the debug branch:

```
greeting = #envar GREETING
missing = #envar MISSING_GUIDE_VARIABLE
#if DEBUG
mode = "debug"
#else
mode = "release"
#endif
print greeting + ":" + mode // hello:debug
print missing == null       // 1
```

Use environment directives for build variants and deployment settings, not for values that should remain secret in distributed source.

## Source data

`#inject "path"` reads a file and inserts its contents as a string literal. `#line` evaluates to the directive's source line number, and `#filename` evaluates to the current source filename.

Given a `notes.txt` containing `compiled`, this example uses all three:

```
notes = #inject "notes.txt"
print notes.trim       // compiled
print #line > 0        // 1
print #filename.len > 0 // 1
```

Injection is useful for static text and data. Changes to an injected file require rebuilding or rerunning the source.

## Syntax

Greybel accepts `/* ... */` block comments, compound assignment, bitwise operators, single-line loop bodies, trailing commas in list and map literals, and string literals spanning multiple source lines. Grey Hack's lexer rejects those multiline strings, so a build writes each one on one line, with a character the string doesn't contain in place of each line break, and replaces it when the string is created: `"first line¶second line".replace("¶", char(10))`. The file stays about the size of the source, which matters because Grey Hack saves at most 160,000 characters per file. `multilineStrings` in `greybel.json` can select `Concatenation` (`"first line" + __NL + "second line"`) or `Join` (`__MSTR(["first line", "second line"])`) instead. A `\r\n` or `\r` in the string becomes a line break too. Outside strict mode, a parameter default can be any expression, such as `function(items = [], count = items.len)`. It is evaluated on each call that omits the argument or passes `null`, and a build writes it as `if @items == null then items = []` at the start of the function. In strict mode defaults must be literals (see [Functions](#guide-functions)).

Further syntax, such as string templates, destructuring, classes and types, is described in [Syntax features](#guide-syntax-features).

Compound assignment supports `+=`, `-=`, `*=`, `/=`, `%=`, and `^=`. It applies the corresponding binary operator and assigns the result to the target.

```
/* Multi-line comments do not produce code. */
x = 6
x += 2
x *= 3
print x // 24
```

The bitwise operators are `|` for OR, `&` for AND, `<<` for left shift, `>>` for signed right shift, and `>>>` for unsigned right shift. They operate on integer representations of their numeric operands. The `^` operator remains exponentiation; use [bitXor](#GENERAL_BITXOR) for exclusive OR.

```
print 6 | 3   // 7
print 6 & 3   // 2
print 1 << 4  // 16
print 16 >> 2 // 4
print 16 >>> 2 // 4
```

These operators use 64-bit integers. The intrinsics [bitAnd](#GENERAL_BITAND), [bitOr](#GENERAL_BITOR) and [bitXor](#GENERAL_BITXOR) use 32-bit integers, as in Grey Hack: an operand outside that range wraps around, so `bitAnd(2147483648, 2147483648)` is `-2147483648` while `2147483648 & 2147483648` is `2147483648`.

A `for` or `while` body can follow its header on the same line without `end for` or `end while`.

```
sum = 0
for n in range(1, 3) sum += n
print sum // 6
```

List and map literals may contain a trailing comma.

```
items = [10, 20,]
options = {"debug": 1,}
print items.len + options.len // 3
```

In strict mode, GreyScript is parsed exactly as Grey Hack parses it: no directives, `debugger` statements, bitwise operators, block comments, compound assignments, trailing commas in calls, line breaks before a closing parenthesis, or loop bodies on the loop's own line. Greybel's simulated world always compiles in-game code (`build`, launched programs) this way. See [Operators](#guide-operators) for bitwise precedence.

## Debugger

The `debugger` statement pauses execution at its source line and enters Greybel's debugger, equivalent to placing a breakpoint there. The CLI enables debugging with `greybel execute -d file.src`; the VS Code extension exposes breakpoints and stepping in its debugger.
