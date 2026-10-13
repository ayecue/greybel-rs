# Variables and scope

Assignment with `=` binds a value to a name; no declaration statement is required. Variables are dynamically typed and can be rebound to values of different types.

```
count = 3
count = "three"
print count // three
```

Assignment to a list or map element mutates that object. Assignment to a variable only rebinds the variable and does not mutate its previous value. See [Values](#guide-values) for reference semantics.

## Lexical scope

Each function call has a local scope. A bare assignment inside a function creates or updates a local variable in that call, even if an outer function or global variable has the same name. It never implicitly updates an outer binding.

```
count = 0
reset = function()
    count = 5
    print count // 5
end function
reset
print count // 0
```

Name resolution is lexical. A read searches the current function's locals, then the locals of lexically enclosing functions, then globals, and finally runtime intrinsics. A nearer binding shadows a binding with the same name farther out.

Top-level code has no enclosing function. Its local scope is the global scope, so a top-level assignment creates a global variable.

Only functions create scopes. A `for` loop's variable and every assignment inside a loop or an `if` block remain in the enclosing scope afterwards.

```
for n in [1, 2]
    last = n * 10
end for
print n    // 2
print last // 20
```

## Globals

`globals` is the map of global variables. Use explicit member assignment to change a global from inside a function. `locals` is the current scope's map and can likewise disambiguate a local name.

```
count = 0
increment = function()
    globals.count = globals.count + 1
end function
increment
print count // 1

state = "global"
visit = function()
    locals.state = "local"
    print state        // local
    print globals.state // global
end function
visit
print state // global
```

Prefer parameters and return values over mutable global state. Explicit `globals` access is appropriate when shared state is intentional.

## Outer

A nested function can read variables from its lexically enclosing function. The `outer` map refers to the immediately enclosing function's scope and is required to assign those captured variables.

```
makeCounter = function()
    count = 0
    next = function()
        outer.count = outer.count + 1
        return count
    end function
    return @next
end function

counter = makeCounter()
print counter() // 1
print counter() // 2
```

The returned function retains its lexical environment after `makeCounter` returns; this is a closure. Each call to `makeCounter` creates an independent `count` and therefore an independent counter.

[Functions](#guide-functions) provides more examples of nested and returned functions.
