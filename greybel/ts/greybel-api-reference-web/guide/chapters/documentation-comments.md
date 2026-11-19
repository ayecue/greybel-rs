# Documentation comments

Comments directly above an assignment document it. The language server shows them on hover and in completion, and the type analyzer uses their types to check calls, members and values. Tags start with `@`; text before the first tag is the description. Documentation comments write no code, also in strict mode.

## Functions

```
// Greets a user.
// @example greet("Ada", 1)
// @param {string} name - The name to show.
// @param {string|number} id
// @returns {string} The greeting.
greet = function(name, id)
    return "Hello, " + name + " (" + id + ")"
end function
print greet("Ada", 1) // Hello, Ada (1)
```

- `@description` is the description, when the comment doesn't start with it.
- `@example` shows code, on its line and the lines after it.
- `@param {Type} name text` (also `@params`) types and describes a parameter.
- `@returns {Type} text` (also `@return`) types and describes the result.
- `@template T` names a type parameter, used in other tags as `T`: `@param {list<T>} items` and `@returns {T}` make the result's type follow the argument's.
- `@deprecated text` marks the function as no longer to be used. Each call is reported with the text, and editors show it struck through.
- `@mutates name` says the function adds to the list or map passed as `name`, so the caller's variable gets the added item types. Without a name it means `self`.

```
// @template T
// @param {list<T>} out
// @param {T} item
// @mutates out
add = function(out, item)
    out.push(item)
end function
names = []
add(names, "Ada")
print names.len // 1
```

## Types

The type language is the one of [Types in syntax features](#guide-syntax-features): `number`, `string`, `list<T>`, `map<K, V>`, `A|B` (or `A or B`), literal types such as `"tcp"|"udp"`, tuples such as `[string, number]`, function types such as `(item: map) => string`, `any`, `null` and the API's types such as `shell` or `computer`.

- `@type Name` before a map declares a type that exists at run time, and `@property {Type} name` its members.
- `@vtype Name` declares a type for the analyzer only, with members described by `@property` and `@function`. Each `@function name` is followed by indented `@param`, `@returns` and `@mutates` tags for that method.
- `@extends Type` gives a declared type the members of another type.
- `@property {readonly Type} name` can't be assigned; `[name]` is optional, read only after `hasIndex` or an assignment; `a.b` documents a nested member.
- `@typedef {Type} Name` names a type, like `type Name = Type`; `Name<T>` or `@template T` gives it type parameters.
- `@define {Type}` (or `@type {Type}`) above a value gives the value that type.

```
// @vtype Repository
// @extends map
// @property {string} name
// @property {number} [timeout]
// @function find
//   @param {string} id
//   @returns {map|null}

// @typedef {string|number} Id

// @define {Repository}
users = {"name": "users"}
print users.name // users
```

A file containing only such declarations can be shared with `#include type "path"`.

## Ignoring diagnostics

`// @ignore` on its own line ignores the diagnostics of the next line with code; after code, it ignores its own line. `// @ignore-file` anywhere in a file ignores all diagnostics of the file. Both take optional diagnostic codes, separated by commas or spaces, and then ignore only those. A code that names no diagnostic is itself reported as `invalid-annotation`.

```
// @ignore native-override
print = function(text)
    globals.messages.push(text)
end function
messages = []
retries = 3
retries = retries // @ignore self-assignment
```

Each diagnostic shows its code. `diagnostics` in `greybel.json` changes the severity of a code or turns it off for the whole project:

```json
{
  "diagnostics": {
    "missing-let": "warning",
    "unknown-member": "off"
  }
}
```
