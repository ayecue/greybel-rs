# Syntax sugar proposals

Status: proposal. Implemented so far: number literals, `let` and `const` (without destructuring), string interpolation, `enum`, spread in literals, map shorthand, `assert`, short functions, destructuring statements, `for` headers and `#import` patterns, `switch` (with `case is Type`), `export`, the ternary expression, `??` and optional chains, `class`, and types in code (parameter, return and `let` types, `function<T>`, `as`, `x!`), literal types, tuple types, type guards, `type` statements (aliases, generic aliases, members with `?` and `readonly`, methods), `implicit-any` and `const enum`.

## Rules for every feature

- Greybel syntax only: rejected in strict mode, like the existing extensions in [cli.md](../cli.md#syntax-extensions).
- Builds lower it to plain GreyScript; dev mode (the formatter) keeps the source as written.
- Each feature needs parser, transpiler (default, beautify, uglify), interpreter, type analyzer and TextMate support.
- Temporaries use reserved `__` names (like `__MSTR`), unique per function; uglify renames them.
- A lowering that copies a value reads it with `@`, so a function value is not called. Example: `a = @__d[0]`.
- A lowering that has to insert statements before an expression only works where statements can be inserted. `while` conditions and the right side of `and`/`or` need the statement rewritten (see optional chaining).
- A rule that only some projects want (`let`, types) is a diagnostic that is off by default and enabled through the `diagnostics` map in `greybel.json`, like every other diagnostic. No separate option.

Facts checked with `greybel execute` (probe files, 2026-11-16):

| Probe | Result |
|---|---|
| `m = {}; print m.cache` | runtime error `Key Not Found` |
| `for v in l` then `__v_idx` | the index is readable in the loop body |

## Accepted

### String interpolation

```greyscript
msg = #"Hello {name}, {items.len} items"
```

Becomes `msg = "Hello " + (name) + ", " + (items.len) + " items"`. If the string starts with an expression, `"" +` comes first so the result is always a string. `{{` and `}}` write literal braces. Code inside braces is lexed as code, so `#"{m["k"]}"` works. A function value inside braces is called, like any read. Works with multiline strings.

### `let` and `const`

```greyscript
let count = 0
const LIMIT = 10
```

Both become plain assignments.

- Scope is the function (or the file's globals at the top level), as for any MiniScript variable. There is no block scope at runtime, so `let` inside an `if` declares the variable for the whole function.
- `let` or `const` twice for one name in one function is reported.
- `const`: any later assignment is reported, including `LIMIT += 1`, `globals.LIMIT = ...` and assignments in other files. Contents can still change (`const L = []` then `L.push(1)` is allowed). No runtime check.
- New diagnostic `missing-let`, off by default: an assignment that creates a variable without `let` or `const`. Parameters, `for` variables and `#import` names count as declared. `outer.x =`, `globals.x =` and member assignments are not affected.
- Destructuring takes the prefix too: `let [user, host] = ...`.

### Destructuring

```greyscript
[user, host] = address.split("@")
[first, ...others] = items
{name, port as p, timeout = 30} = config
for {key, value} in settings
for [x, y] in points
#import {parse, format as fmt} from "lib/util.src"
```

- `[user, host] = expr` becomes `__d = expr`, `user = @__d[0]`, `host = @__d[1]`. A missing element raises the usual index error.
- `...others` takes the remaining elements: `others = __d[1:]`. Lists only, last position only.
- `{name, port as p} = expr` becomes `name = @__d.name`, `p = @__d.port`. A missing key raises `Key Not Found`.
- A default (`timeout = 30`) is used when the key is missing or `null`: `if __d.hasIndex("timeout") and @__d.timeout != null then timeout = @__d.timeout else timeout = 30`. For lists the check is the length.
- `for {key, value} in m` becomes `for __e in m` with `key = @__e.key` and `value = @__e.value` at the start of the body.
- `for [x, y] in points` destructures each item the same way.
- `#import {parse, format as fmt} from "path"` becomes `__m = __REQUIRE("lib/util")`, `parse = @__m.parse`, `fmt = @__m.format`. `__REQUIRE` caches the module, so it still runs once. The names are copies taken when the import runs: a later change to the module's `exports` isn't seen. A missing export raises `Key Not Found` at the import; the analyzer reports it as `unknown-member` when the exports map is fully known.
- The temporary is skipped when the right side is a plain variable.
- Statements only.

### `enum`

```greyscript
enum Color
	Red
	Green = 5
	Blue
	Name = "n"
end enum
```

Becomes `Color = {"Red": 0, "Green": 5, "Blue": 6, "Name": "n"}`.

- Values are number or string literals, negative numbers included.
- Without a value: 0 for the first entry, otherwise the previous number plus 1. After a string value, the next entry needs a value (parse error).
- A repeated name is a parse error.
- The analyzer sees a fully known map, so `Color.Redd` is reported as `unknown-member`. As a type, `Color` means any of its values.

### Short functions

```greyscript
double = (x) => x * 2
names = items.map((x) => x.name)
handler = (line) =>
	parts = line.split(" ")
	return parts[0]
end function
```

- `(x) => expr` becomes `function(x)`, `return expr`, `end function`.
- A block form starts with `=>` at the end of the line and closes with `end function`.
- In call arguments, list and map literals it lowers to the form GreyScript already accepts there, `items.map(function(x))` with the body on the following lines; the transpiler already emits that form (`deferred_function` in `greybel-transpiler/src/emit/nodes.rs`).
- Parameters take defaults and types like `function` parameters.
- Like any MiniScript function it reads the enclosing function's locals only through `outer`, one level up.
- Telling `(x) =>` apart from a parenthesized expression needs lookahead to the `=>`.

### `switch` / `case`

```greyscript
switch command
	case "ls", "dir"
		list_files
	case "exit"
		exit
	else
		print "unknown"
end switch
```

Becomes `__s = @command` plus an `if __s == "ls" or __s == "dir" then` / `else if` chain. No fall-through. `break` inside a `case` keeps its meaning for an enclosing loop. The temporary is skipped when the subject is a plain variable. Cases narrow the subject in the analyzer like the equivalent `if`.

### `class`

```greyscript
class Dog extends Animal implements Pet
	static sound = "woof"
	private tricks = []

	constructor(name)
		super.constructor(name)
	end function

	bark(times)
		print self.sound * times
	end function
end class

rex = Dog.create("rex")
```

- Becomes `Dog = new Animal` (or `Dog = {}` without `extends`), then `Dog.member = value` for each method and `static` field in order. `name(parameters)` up to `end function` is a method; `name = function ... end function` works too.
- A field without `static` is an instance field: it is only assigned in the constructor (`self.tricks = []`), so each instance has its own value and the class map doesn't have it. A `static` field is assigned on the class and shared by all instances through `__isa`. `private` and `static` can be combined in either order.
- The constructor is the `constructor` method. It starts with the instance fields' assignments, in order, and returns `self` (a `return` without a value returns `self` too). A class with instance fields and no constructor gets one; a class that extends another needs to write one, as a generated one couldn't pass arguments on to the base's.
- `Dog.create("rex")` is `(new Dog).constructor("rex")`: each class with a constructor, or without a base, gets `create = function(<the constructor's parameters>)` with `return (new self).constructor(...)` (or `return new self`), unless it defines `create` itself. It creates `new self`, so a class that inherits `create` creates instances of itself. `(new Dog).constructor("rex")` works as well. `new Dog("rex")` isn't used: it already means `new (Dog("rex"))`.
- Dev mode keeps the class as written.
- `implements Pet` (type names, after the name or `extends`): removed by the build; the analyzer reports members of the type the class doesn't define, or defines with another type (`missing-implementation`).
- `private` before a member: removed by the build; the analyzer reports reads and assignments of it outside the class's own methods, the methods of classes extending it and functions inside them (`private-member`).

### Type syntax

The same type language as the doc comments (`string|number`, `list<number>`, `map<string,number>`, custom type names), written in code. The build removes all of it. Comments and syntax are two ways to write the same thing: the analyzer gives both the same meaning, and a parameter typed both ways is reported.

```greyscript
let port: number = 22

connect = function(ip: string, port: number = 22): shell|null
	...
end function

type Id = string|number

type Repository extends map
	name: string
	find(id: Id): map|null
end type

first = function<T>(items: list<T>): T
	return items[0]
end function

isDog = function(value): value is Dog
	return value isa Dog
end function

shell = get_shell as shell
```

- Variable, parameter and return types; types for short-function parameters.
- `type Name = ...` is a type alias; `type Pair<T> = [T, T]` takes type parameters. The comment form is `// @typedef {string|number} Id` or `// @typedef {[T, T]} Pair<T>` (without `<...>`, the block's `@template` names are the type parameters).
- `type Name ... end type` declares a type without runtime code, like `@vtype`; `extends` and methods as in `@extends`/`@function`.
- `function<T>` is `@template T`.
- `value is Dog` as a return type is a type guard: `if isDog(a) then` narrows `a` to `Dog`, and the `else` branch removes it (new: the comment form gets `@returns {value is Dog}`).
- `expr as T` gives an expression a type, like `@define` does for a value.
- New diagnostic `implicit-any`, off by default: a parameter, return type or variable whose type the analyzer can't infer and that has no type written.
- Function types: `(x: number) => string` or `function(x: number): string`, in comments too.

### Optional chaining and null-coalescing

```greyscript
computer = shell?.host_computer
path = params[0] ?? "/home"
first = list?.[0]
result = callback?.(x)
```

- `a ?? b` is `a` unless it is `null`, then `b`; `b` runs only when needed. It binds below `or`: `a ?? b or c` is `a ?? (b or c)`.
- `a?.b.c` is `null` for the whole chain when `a` is `null`, as in JavaScript; parentheses end the chain (`(a?.b).c`). `a?.f(x)` skips the call and `x`. `f?.(x)` reads `f` without calling it; `a.f?.(x)` calls `f` with `a` as `self` (builds: `__t0 = a`, `if @__t0.f != null then __t0 = __t0.f(x) else __t0 = null`), while `a[k]?.(x)` calls the function value without `self`, since GreyScript has no `a[k](x)` call. `?.` can't be assigned to (`a?.b = 1` is an error).
- `?.` only checks for `null`: a missing key still raises `Key Not Found`.
- Builds compute the value into a `__tN` temporary in statements before the statement that uses it, then read `@__tN`: `__t0 = a`, `if @__t0 != null then __t0 = __t0.b.c`. Operands that come earlier in evaluation order are computed into temporaries before it, so the order stays: `print(f(), a ?? b)` becomes `__t0 = f()`, the `??` into `__t1`, then `print(@__t0, @__t1)`. An assignment target's base and index come first, as in MiniScript (`m[f()] = a ?? b` calls `f` first).
- A `while` condition becomes `while 1` with the values and `if not (condition) then break` at the start of the body; an `else if` condition becomes an `else` holding an `if`. On the right side of `and`/`or`, in a parameter default or an `assert` message the value only runs sometimes, so builds report it (and `greybel execute`, which runs the build). The interpreter itself and the type analyzer have no such limit.
- The analyzer removes `null` from the base of a `?.` link and the links after it, and adds `null` to the chain's type when the base could be `null`.

### `export`

```greyscript
export parse = function(text)
	...
end function
```

In a file loaded with `#import`: becomes `parse = function(text) ... end function` plus `module.exports.parse = @parse` after it, with `module.exports = {}` before the file's first export (exports are only allowed at the top level, so that one runs first). The analyzer reports `export` in an entry file or an `#include`d file (there is no `module` there), and a file that uses `export` and also assigns `module.exports`. Pairs with `#import` destructuring.

### List index in `for`

```greyscript
for i, item in items
for {key, value} in settings
```

`for i, item in items` becomes `for item in items` with `i = __item_idx` at the start of the body, the loop's own index variable. It is the index form for any sequence, maps included (where `item` is the `{"key": ..., "value": ...}` map). Key and value of a map come from destructuring each item: `for {key, value} in settings`. Decided while implementing: one meaning per syntax, so builds need no runtime check of the sequence's type.

### Non-null assertion

```greyscript
computer = get_shell.host_computer!
```

`x!` is removed by the build. The analyzer drops `null` from the type of `x`. Postfix only; `x != y` keeps its meaning.

### `assert`

```greyscript
assert port > 0, "port must be positive"
```

Becomes `if not (port > 0) then exit("port must be positive")`. Without a message: `exit("Assertion failed at main.src:12")`. The build variable `NO_ASSERTS` removes all asserts from the output. The analyzer narrows by the condition after the statement, like `if not cond then exit`.

### Spread in list and map literals

```greyscript
all = [...defaults, 1, ...extra]
options = {...defaults, "verbose": 1}
```

Becomes `defaults + [1] + extra` and `defaults + {"verbose": 1}`. Both create new values; for maps the later entry wins, as in JavaScript. A non-list or non-map operand raises the usual type error; the analyzer reports it.

### Map shorthand

```greyscript
user = {name, port}
```

Becomes `{"name": name, "port": port}`, the mirror of map destructuring. Only plain identifiers. In plain MiniScript `{name: 1}` uses the value of `name` as the key; that form keeps its meaning.

### Number literals

`0xFF`, `0b1010`, `0o17` and `1_000_000` become decimal numbers at build time. Exact up to 2^53.

### Ternary expression

```greyscript
label = enabled ? "on" : "off"
print "Port " + (open ? "open" : "closed")
```

`condition ? if_true : if_false`, lowest precedence and right-associative (`a ? b : c ? d : e` is `a ? b : (c ? d : e)`). Only the chosen branch runs. Inside a map literal or a slice the conditional takes its `:` first (`{"k": a ? b : c}`). `c?.5:1` is `c ? .5 : 1`, as in JavaScript.

Builds: as the whole right side of an assignment or `return`, an `if` statement (`if enabled then label = "on" else label = "off"`); elsewhere a `__tN` temporary set by an `if` statement before the statement, as for `??`.

### Literal types

```greyscript
type Protocol = "tcp"|"udp"

open = function(protocol: Protocol)
```

- String and number literals are types. `open("tpc")` is reported as an argument mismatch; a string or number that isn't a literal in the call fits, since values are inferred as `string` or `number`.
- `enum` values are literal types: `Color` as a type is the union of its values.
- `==`, `!=` and `switch` cases narrow a literal union: after `if protocol == "tcp"` the branch has `"tcp"` and the `else` branch the other literals.
- New diagnostic `missing-case`: a `switch` over an enum or literal union that misses a value and has no `else`.
- Comment form: `@param {"tcp"|"udp"} protocol`.

### Optional members

```greyscript
type Options extends map
	name: string
	timeout?: number
end type
```

`timeout` may be missing from the map: a class that `implements` the type doesn't need it. A read of `options.timeout` is reported (`optional-member`) unless `options.hasIndex("timeout")` is true there or the member was assigned on every path before it. Optional parameters (`port?: number`) are not needed: defaults already make a parameter optional. Comment form: `@property {number} [timeout]`, as in JSDoc.

### Further type and pattern additions

| Idea | Example | Becomes |
|---|---|---|
| Type cases in `switch` | `case is Dog` / `case is string` | `isa` checks; the analyzer narrows inside each case |
| Nested destructuring and swap | `[a, [b, c]] = x`, `[a, b] = [b, a]` | follows from destructuring through the temporary |
| Optional index and call | `list?.[0]`, `callback?.(x)` | part of optional chaining |
| Tuple types | `: [string, number]` | type-only; implemented as a list of the union of the element types (the length and positions are not tracked) |
| Generic type aliases | `type Pair<T> = [T, T]` | type-only |
| `readonly` members in `type` | `readonly id: string` | the analyzer reports assignments outside the class's methods |
| `const enum` | `const enum Port` ... `end enum` | implemented: builds write the value of `Port.Ssh` reads in every file of the build (not in a file that also uses `Port` as a variable or parameter) and still write the map (for `Port[key]`); assigning an entry is reported as `const-assignment` |

## Not planned

| Idea | Reason |
|---|---|
| `x \|> f(y)` | Not wanted. |
| `repeat ... until` | Not wanted. |
| `defer` | Little use: GreyScript has nothing to close on return. |
| Labeled `break outer` / `continue outer` | Not wanted. |
| `??=` | Not wanted. |
| `x in items` | Too much hidden behaviour: a helper that picks `hasIndex` or `indexOf` breaks when either is replaced on the type. |
| Range literals `0..10` | Not wanted. |
| `++` / `--` | Not wanted; `a--b` already means `a - (-b)`. |
| Named arguments `f(count: 3)` | The transpiler doesn't know `f`'s parameter order, and calls through variables can't be resolved. |
| Spread in calls `f(...args)` | MiniScript can't call a function with a list of arguments. |
| `try` / `catch` | MiniScript runtime errors can't be caught. |
| Block-scoped `let` | No block scope at runtime; it would need renaming every shadowed variable. |
