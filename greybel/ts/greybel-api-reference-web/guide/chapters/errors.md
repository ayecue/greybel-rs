# Errors and pitfalls

GreyScript reports syntax errors before execution and runtime errors when an operation cannot be completed. Typical runtime errors include reading an absent name or map key, indexing outside a collection, calling a non-function, and applying an operator to unsupported operand types.

## Runtime errors

An unhandled runtime error terminates the current program and reports its source location. Core GreyScript has no exception-catching statement and no error-value type. Prevent errors by validating types, indices, keys, and API results before performing an operation.

Game API calls often report an expected failure by returning `null` or an error string instead of raising a runtime error. This includes argument-validation failures such as `md5: Invalid arguments`; check the returned value rather than assuming invalid arguments stop the program. Other failures, such as `is_match` with an invalid regex option, still raise. Consult each function's reference entry and handle all documented return variants.

```
print md5(42)                   // md5: Invalid arguments
print "a,b".split(",", "bad") // split: Invalid regex option
```

The terminal prints a runtime error in red as `Runtime Error: <message> [<file> line <n>]`. Messages name the kind of failure, for example:

| Code              | Message                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------- |
| `x = [1][5]`      | `Index Error: list index (5) out of range (-1 to 0)`                                        |
| `x = {}.name`     | `Key Not Found: 'name' not found in map`                                                    |
| `print missing`   | `Undefined Identifier: 'missing' is unknown in this context`                                |
| `x = null; x.n`   | `Type Error (while attempting to look up n)`                                                |
| `print "a" * "b"` | `got a Miniscript.ValString where a Miniscript.ValNumber was required (string replication)` |

## Reads that give null

Several operations that look like errors give `null` instead, so a mistake can pass silently:

```
x = null
print x[0]       // null
print 5[0]       // null
print "abc" - 1  // null
print -"a"       // null
for item in x
    print item   // never runs
end for
loop = {}
loop.__isa = loop
print loop.name  // null
```

Reading a member with `.` from a variable or result that holds `null` is still an error (`Type Error (while attempting to look up name)`), although the same read from the literal `null` gives `null`. Indexing a list or string with `null` is an error too. A list takes any index that is not a number as `0`, so `items["1"] = 5` replaces the first element.

## Interpreter quirks

Grey Hack's interpreter descends from MiniScript 1.6 and keeps behaviour that later MiniScript versions changed:

- `x isa null` is `1` for every value whose type map the program has not read yet through `number`, `string`, `list`, `map` or `funcRef`. Compare with `== null` instead.
- `@` inside parentheses does not stop a call when it takes a member or an element: `(@m.f)` calls `m.f`. Write `@m.f` without parentheses to refer to the function.
- `new` accepts any value: `new 42` is a map whose `__isa` is `42`.

```
print 1 isa null // 1
x = number
print 1 isa null // 0
f = function
    return 5
end function
m = {"f": @f}
print (@m.f) // 5
print typeof(@m.f) // function
print typeof(new 42) // map
```

## Isolating failures

A runtime error ends only the program in which it occurs. [launch](#SHELL_LAUNCH) runs another program and blocks until that program ends; when the launched program fails, its error is printed and the launching program continues with the next statement. Combined with [get_custom_object](#GENERAL_GET_CUSTOM_OBJECT), whose map is shared by a program and the programs it launches, this provides a limited substitute for try and catch.

The launched program records its completion as its last statement. A failure resets the custom object, so after a failed launch the marker is absent and `get_custom_object` returns a new, empty map. The following function builds `code` into a program, launches it, and reports whether it ran to completion:

```
attempt = function(code)
    shell = get_shell
    computer = shell.host_computer
    computer.touch(home_dir, "attempt.src")
    computer.File(home_dir + "/attempt.src").set_content(code + char(10) + "get_custom_object.finished = 1")
    if shell.build(home_dir + "/attempt.src", home_dir) != "" then return 0
    get_custom_object.remove("finished")
    shell.launch(home_dir + "/attempt")
    return get_custom_object.hasIndex("finished")
end function

print attempt("x = [1][0]") // 1
print attempt("x = [1][5]") // 0
```

Limitations:

- Everything stored in the custom object is lost when a launched program fails. A variable that already refers to the old map still holds its values.
- [build](#SHELL_BUILD) returns an error message instead of `""` when the code does not parse, so syntax errors are detected before the launch.
- `launch` itself returns `1` for a program that started and then failed, so only the marker tells the outcomes apart.
- Programs launched more than 32 levels deep are interrupted.
- Launching is unavailable in an SSH encryption process.

## Calls

Implicit function calls are a common source of type errors. Evaluating `answer` calls the function; `@answer` yields the function reference. This distinction also applies when passing a function to `typeof` or another function.

```
answer = function()
    return 42
end function
result = answer
callback = @answer
print result             // 42
print typeof(result)     // number
print typeof(@callback)  // function
```

A runtime error results from calling a value that is not a function. Check callbacks with `value isa funcRef` when their origin is uncertain, and use `@` whenever the function itself is the intended value.

## Bounds and keys

Check an index with `hasIndex` before indexing user-controlled data. Check map keys in the same way. Searches such as `indexOf` return `null` when no match exists; do not use that result as an index without checking it. The conventions differ between functions: string `lastIndexOf` returns `-1` instead, `pop` and `pull` return `null` for an empty list, and map `remove` returns `1` when the key existed and `0` otherwise.

```
print "abc".lastIndexOf("z") // -1
print [].pop == null         // 1
settings = {"a": 1}
print settings.remove("a")   // 1
print settings.remove("a")   // 0
```

```
itemAt = function(items, index)
    if not items.hasIndex(index) then return null
    return items[index]
end function
print itemAt([10, 20], 1)       // 20
print itemAt([10, 20], 5) == null // 1

config = {"debug": 1}
if config.hasIndex("debug") then print config.debug // 1
```

Writing `items[5]` when that index is absent, or `config.name` when the key is absent, raises a runtime error. In contrast, out-of-range slice bounds are clipped.

## Mutation

Do not add or remove elements while iterating directly over the same list or map. Structural mutation can invalidate iteration or skip entries. Iterate over a shallow snapshot when the original collection must change.

```
numbers = [1, 2, 3, 4]
for number in numbers[0:]
    if number % 2 == 0 then numbers.remove(numbers.indexOf(number))
end for
print numbers // [1, 3]
```

For a map, iterate over `map.indexes`, which is a separate list of keys, and check that a key still exists before using it.

A full slice is only a shallow copy. Nested lists and maps remain shared, so mutating a nested value through the copy also changes the value visible through the original. Construct nested copies explicitly when independent objects are required.

## Numbers and strings

All numbers use floating-point representation, and `/` never performs integer division. Apply [floor](#GENERAL_FLOOR), [ceil](#GENERAL_CEIL), or [round](#GENERAL_ROUND) explicitly when needed.

```
print 7 / 2        // 3.5
print floor(7 / 2) // 3
```

Strings are immutable. An assignment such as `word[0] = "H"` is invalid; build a replacement with slicing and concatenation instead. String-returning methods likewise leave the original string unchanged, so retain their return value.

```
word = "hello"
word = "H" + word[1:]
print word // Hello
```

Numeric text is not implicitly parsed for arithmetic. Use [val](#GENERAL_VAL), remembering that invalid text produces `0`.

## Missing values

Do not use truthiness alone to detect an omitted argument or missing result when `0`, `""`, `[]`, or `{}` is valid. Compare explicitly with `null`.

```
describe = function(value = null)
    if value == null then return "missing"
    return "value: " + value
end function
print describe()  // missing
print describe(0) // value: 0
```

## API results

A robust program checks both the type and value of an API result before using it. A string may contain an error message rather than requested text, while a successful object can have a type such as `shell` or `computer`.

```
resultName = function(result)
    if result == null then return "unavailable"
    if result isa string then return result
    return "received " + typeof(result)
end function
print resultName(null)       // unavailable
print resultName("denied")   // denied
print resultName({"id": 1})  // received map
```

See [Types and conversion](#guide-type-conversion) and the return variants in each API entry.
