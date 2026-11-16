# Syntax features

Outside strict mode, Greybel accepts the syntax in this chapter in addition to GreyScript. A build writes each feature as plain GreyScript, so the built program runs in Grey Hack; the formatter keeps the source as written. Strict mode rejects all of them.

Each feature can be turned off in `greybel.json`. A feature that isn't listed stays on:

```json
{
  "syntax": {
    "ternary": false,
    "classes": false
  }
}
```

The names are `number-literals`, `templates`, `declarations`, `enum`, `spread`, `map-shorthand`, `assert`, `short-functions`, `destructuring-assignments`, `for-index`, `switch`, `export`, `ternary`, `coalesce`, `optional-chains`, `classes` and `types`. Builds, runs, the language server and the editor use the setting.

## Number literals

Numbers can be written in hexadecimal (`0x`), octal (`0o`) and binary (`0b`), and `_` can separate digits. A build writes them as decimal numbers.

```
print 0xFF      // 255
print 0o17      // 15
print 0b1010    // 10
print 1_000_000 // 1000000
```

## String templates

A string starting with `#"` is a template: each `{...}` contains an expression whose value is inserted. `{{` and `}}` write braces, and `""` writes a quote. A build writes the template as string concatenation, so the result is always a string.

```
name = "Ada"
items = [1, 2, 3]
print #"Hello {name}, {items.len} items" // Hello Ada, 3 items
print #"{{literal}} and ""quotes"""      // {literal} and "quotes"
```

## let and const

`let` and `const` can start an assignment to a name. A build writes the plain assignment. The type analyzer reports assignments to a `const` and a second `let` of the same name in one function. With the `missing-let` diagnostic switched on, it also reports assignments that create a variable without `let` or `const`.

```
let count = 0
const LIMIT = 3
count += LIMIT
print count // 3
```

Variables are still scoped to the function, as in GreyScript; `let` doesn't create a block scope. `let` and `const` remain valid names, as in `let = 1`.

## Enums

`enum Name` lists one entry per line up to `end enum`. An entry without a value gets the previous number plus 1, starting at 0; values are number or string literals, and an entry after a string value needs its own value. A build writes the enum as a map.

```
enum Color
    Red
    Green
    Blue = 10
    Custom = "custom"
end enum
print Color.Green  // 1
print Color.Blue   // 10
print Color.Custom // custom
```

With `const enum`, a build also writes the value in place of every `Name.Entry` read in the files of the build. A file that uses the enum's name as a variable or parameter keeps its reads. The map is still written, so `Name[key]` keeps working.

## Spread and map shorthand

`...` inserts the items of a list or the entries of a map into a literal; for maps, a later entry replaces an earlier one with the same key. `{name, port}` is short for `{"name": name, "port": port}`.

```
defaults = {"port": 22, "verbose": 0}
options = {...defaults, "verbose": 1}
print options.port    // 22
print options.verbose // 1
numbers = [...[1, 2], 3]
print numbers.len     // 3
name = "web"
port = 80
service = {name, port}
print service.name    // web
```

A key written as a plain name in GreyScript, such as `{name: 1}`, keeps its meaning: the key is the value of `name`. Shorthand only applies to an entry without `:`.

## Destructuring

A list pattern `[a, b] = value` assigns the items of a list by position, and `...name` takes the remaining items. A map pattern `{key} = value` assigns the entries with those keys; `key as name` assigns to another name and `key = default` gives a default when the entry is missing or `null`. Patterns can be nested and can follow `let` or `const`.

```
[user, host] = "root@10.0.0.1".split("@")
print user       // root
print host       // 10.0.0.1
[first, ...others] = [1, 2, 3]
print others.len // 2
config = {"name": "web", "port": 80}
{name, port as p, timeout = 30} = config
print p          // 80
print timeout    // 30
[a, b] = [1, 2]
[a, b] = [b, a]
print a          // 2
```

A build writes one assignment per name, through a temporary when the right side isn't a variable.

## Loop index and patterns

`for i, item in items` also assigns the item's index to `i`. A pattern after `for` destructures each item: `for [x, y] in points` and `for {key, value} in map`. Both can be combined, as in `for i, [x, y] in points`.

```
for i, name in ["a", "b"]
    print i + ":" + name // 0:a, then 1:b
end for
for [x, y] in [[1, 2], [3, 4]]
    print x + y          // 3, then 7
end for
for {key, value} in {"a": 1}
    print key + value    // a1
end for
```

With two names, the first one is always the index, for maps too; use `{key, value}` for a map's keys and values.

## Short functions

`(x) => x * 2` is a function returning the expression after `=>`. With `=>` at the end of a line, the body continues on the following lines up to `end function`. Parameters can have defaults and types, as in `function`.

```
double = (x) => x * 2
print double(4)               // 8
apply = function(f, value)
    return f(value)
end function
print apply((x) => x + 1, 2)  // 3
describe = (count) =>
    if count == 1 then return "one item"
    return count + " items"
end function
print describe(3)             // 3 items
```

In call arguments only the one-line form is accepted. A build writes it as the function form GreyScript accepts there, `items.map(function(x))` with the body on the following lines.

## switch

`switch subject` compares the subject with each `case`, which can list several values, and runs the first matching case; `case is Type` checks the subject with `isa`, and `else` runs when no case matches. The statement ends with `end switch`. There is no fall-through, and `break` keeps its meaning for an enclosing loop.

```
describe = function(value)
    switch value
        case 0, 1
            return "small"
        case is string
            return "text"
        else
            return "other"
    end switch
end function
print describe(1)    // small
print describe("x")  // text
print describe([])   // other
```

A build writes an `if`/`else if` chain.

## Ternary, ?? and optional chains

`condition ? a : b` is `a` when the condition is true and `b` otherwise; only one of the two is evaluated. `a ?? b` is `a` unless `a` is `null`, and `b` is only evaluated in that case. An optional chain `a?.b`, `a?.[i]` or `f?.(x)` is `null` for the whole chain when the value before `?.` is `null`.

```
enabled = 1
print enabled ? "on" : "off"      // on
port = null
print port ?? 22                  // 22
user = {"profile": {"name": "Ada"}}
print user?.profile?.name         // Ada
nobody = null
print nobody?.profile.name ?? "-" // -
callback = null
print callback?.(1) == null       // 1
```

A `?.` only checks for `null`: reading a key that doesn't exist still raises `Key Not Found`. A build computes these values in temporaries before the statement that uses them, keeping the order in which the statement evaluates its parts. On the right side of `and`/`or`, in a parameter default and in an `assert` message they are a build error, because there the value is only evaluated sometimes.

## assert

`assert condition` exits the program when the condition is false, with "Assertion failed at" and the file and line; `assert condition, message` exits with the message.

```
port = 22
assert port > 0, "port must be positive"
print "checked" // checked
```

Setting the build variable `NO_ASSERTS`, for example with `--env-vars NO_ASSERTS=1`, removes all asserts from builds and runs.

## Modules: export and named imports

`export name = value` at the top level of a module assigns the value and adds it to `module.exports`. `#import {a, b as c} from "path"` imports single exports by name; the module still runs once, and each name is a copy of the export at the time of the import. See [Greybel](#guide-greybel) for `#import`.

The module `lib/format.src`:

```
export pad = function(text, size)
    while text.len < size
        text = " " + text
    end while
    return text
end function
export VERSION = 2
```

A program using it:

```
#import {pad, VERSION as version} from "lib/format.src"
print "[" + pad("7", 3) + "]" // [  7]
print version     // 2
```

## Classes

`class Name` up to `end class` declares a class with fields and methods. A method is written as `name(parameters)` up to `end function`; the method named `constructor` sets up a new instance and returns `self`. `Name.create(arguments)` makes an instance and calls its constructor with the arguments.

```
class Animal
    sound = "..."

    constructor(name)
        self.name = name
    end function

    speak()
        return self.name + " says " + self.sound
    end function
end class

class Dog extends Animal
    static kind = "dog"
    tricks = []

    constructor(name)
        super.constructor(name)
        self.sound = "woof"
    end function
end class

rex = Dog.create("Rex")
fido = Dog.create("Fido")
rex.tricks.push("sit")
print rex.speak     // Rex says woof
print fido.tricks   // []
print Dog.kind      // dog
print rex isa Animal // 1
```

- **Instance fields** (`tricks = []`) are assigned in the constructor, so every instance gets its own value. A class without a constructor gets one. A class that extends another and has instance fields needs its own constructor, which calls `super.constructor(...)`.
- **`static` fields** are assigned on the class, so all instances share them through `__isa`.
- **`private` members** may only be used in the methods of the class and of classes extending it; the type analyzer reports other uses.
- **`implements Type`** after the class name makes the type analyzer check that the class has the type's members.

A build writes `Dog = new Animal` (or `Dog = {}` without `extends`) and one assignment per method and `static` field. See [Classes and objects](#guide-classes) for how classes work in GreyScript.

## Types

Types can be written in code with the same type language as doc comments. They only inform the type analyzer and editor: a build removes them, and they don't change what the program does.

```
first = function<T>(items: list<T>): T
    return items[0]
end function
connect = function(ip: string, port: number = 22): map|null
    return {"ip": ip, "port": port}
end function
let retries: number = 3
value = connect("1.2.3.4") as map
print value.port   // 22
print first([5, 6]) // 5
print retries       // 3
```

- **Annotations:** `name: Type` on parameters and `let` variables, `): Type` for the return type and `function<T>` for type parameters. They count like `@param`, `@returns`, `@template` and `@define` in doc comments; a type written both ways is reported.
- **Assertions:** `value as Type` tells the analyzer the value's type, and `value!` that it isn't `null`.
- **Type statements:** `type Id = string|number` and `type Pair<T> = [T, T]` declare aliases. `type Repository extends map` up to `end type` declares a type with members, like `@vtype`: `name: string`, an optional member `timeout?: number`, a `readonly id: Id` and methods such as `find(id: Id): map|null`. A type statement writes no code.
- **Literal types:** `"tcp"|"udp"` or `22|80` accept only those values; comparisons and `switch` cases narrow them, and a `switch` that misses one of them without `else` is reported.
- **Tuple types:** `[string, number]` is a list with those item types.
- **Function types:** `(name: string, count: number) => string`.
- **Type guards:** a function with the return type `value is Dog` narrows its argument where it's called in a condition, such as `if isDog(pet) then`.

An `enum` name used as a type means the union of its values.
