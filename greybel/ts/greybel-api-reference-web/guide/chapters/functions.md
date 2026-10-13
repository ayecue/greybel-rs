# Functions

A function is a first-class value defined by a `function` expression. Functions can be assigned, passed as arguments, returned from other functions, and stored in collections.

## Definition

A definition contains a parameter list, a body, and `end function`. Parentheses are required after `function`, including when the parameter list is empty.

```
add = function(a, b)
    return a + b
end function
print add(2, 3) // 5
```

Assigning a function expression to a variable is the usual way to name a function. A function can also be defined directly inside a collection or another function.

## Parameters

Arguments are bound to parameters by position. A parameter can declare a literal default value with `name=default`. An omitted argument uses its default; without a default, the parameter receives `null`.

```
format = function(value, prefix = "value: ")
    if value == null then return "missing"
    return prefix + value
end function
print format(7)          // value: 7
print format(7, "n=")   // n=7
print format()           // missing
```

Grey Hack requires literal defaults: numbers, strings, `true`, `false` and `null`, also in parentheses. It compiles an `@name` default too, but the parameter then holds the unevaluated reference itself: it prints as `@name`, `typeof` gives `"undefined"`, and it isn't the function (`@x == @name` gives `null`), so GreyScript rejects it as a non-literal default. Only Greybel's own dialect accepts a function reference such as `@name`, outside strict mode, and there the parameter receives `null`. Lists, maps, calculations, and other non-literal defaults are invalid; default to `null` and create the value in the function instead, which also gives each call a new list or map. Parameters are local variables of the function call.

```
collect = function(item, items = null)
    if items == null then items = []
    items.push(item)
    return items
end function
print collect(1) // [1]
print collect(2) // [2]
```

## Returns

`return expression` exits the function and yields the expression's value. `return` without an expression yields `null`. Reaching `end function` without a `return` also yields `null`.

```
nothing = function()
end function
stop = function()
    return
    print "not reached"
end function
print nothing() == null // 1
print stop() == null    // 1
```

A function can return any value, including a list, map, or function reference.

## Calls

A call with arguments uses parentheses when it appears inside an expression. When a call is the entire statement, arguments can follow the function without parentheses. A zero-argument function can be called by naming it or with empty parentheses.

```
greet = function(name)
    print "Hello, " + name
end function
greet "Ada"

answer = function()
    return 42
end function
print answer   // 42
print answer() // 42
```

Evaluating a function value normally causes an implicit call. This applies in assignments, arguments, and expressions, not just in statement position. Consequently, `x = answer` stores `42`, not the function.

## References

Prefix a function reference with `@` to obtain the function value without invoking it. Use `@` when assigning, passing, returning, or inspecting the function itself.

```
double = function(n)
    return n * 2
end function
operation = @double
print typeof(@operation) // function
print operation(4)       // 8
```

Without `@`, `operation = double` would call `double` with its default arguments and store the result. See [Errors and pitfalls](#guide-errors).

[str](#GENERAL_STR) of a function reference shows its parameters and defaults, which helps to inspect a callback or a built-in function.

```
print str(@double)     // FUNCTION(n)
print str(@user_input) // FUNCTION(inputMsg="", isPassword=0, anyKey=0, addToHistory=0, echo=1)
```

## Higher-order

A higher-order function accepts or returns a function. Pass a callback with `@`; inside the receiving function, call the parameter normally.

```
applyAll = function(fn, items)
    result = []
    for item in items
        result.push(fn(item))
    end for
    return result
end function
double = function(n)
    return n * 2
end function
print applyAll(@double, [1, 2, 3]) // [2, 4, 6]
```

To return a function, use `@` before its name in the `return` statement.

## Recursion

A recursive function calls itself directly or indirectly. Define a base case that stops recursion and ensure that each recursive call approaches it.

```
factorial = function(n)
    if n <= 1 then return 1
    return n * factorial(n - 1)
end function
print factorial(5) // 120
```

Grey Hack keeps the call stack in memory with no depth limit, so recursion that never reaches its base case keeps running until it is stopped or the game runs out of memory, which interrupts it with "Max memory usage reached. Script execution interrupted". Greybel has no depth limit either: it interrupts the program with the same message once it uses more memory than `run.memoryLimit` in `greybel.json` allows (1024 MiB by default).

## Nested functions

A nested function is local to its enclosing definition and can capture the enclosing function's variables. Reads resolve lexically; writes to captured variables use `outer`. Return the nested function with `@` to create a closure.

```
makeAdder = function(amount)
    add = function(value)
        return value + amount
    end function
    return @add
end function
addFive = makeAdder(5)
print addFive(3) // 8
```

Each invocation of the outer function creates a new lexical environment. See [Variables and scope](#guide-variables) for name resolution, `locals`, `globals`, and mutable captured state.
