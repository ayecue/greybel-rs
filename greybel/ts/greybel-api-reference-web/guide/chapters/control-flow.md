# Control flow

GreyScript provides conditional `if` statements and two loop forms, `while` and `for`. Block delimiters, not indentation, determine the statements in each branch or loop body.

## if

An `if` block executes when its condition is true according to [truthiness](#guide-operators). Add zero or more `else if` clauses and an optional final `else`. Conditions are evaluated in order, and at most one branch executes.

```
level = 7
if level > 5 then
    print "high"
else if level > 2 then
    print "middle"
else
    print "low"
end if
```

The block form requires `then` after each condition and a final `end if`. Each `else if`, `else`, and `end if` appears on its own line.

A single-line `if` contains one statement after `then` and optionally one statement after `else`; it has no `end if`. Use the block form for multiple statements or an `else if` chain.

```
level = 7
if level == 7 then print "seven" else print "other"
if level > 10 then level = 10
print level // 7
```

## while

A `while` loop evaluates its condition before every iteration. If the condition is initially false, the body does not execute. The loop ends when the condition becomes false or a `break` executes.

```
total = 0
n = 0
while n < 4
    n = n + 1
    total = total + n
end while
print total // 10
```

`while true` creates an intentionally unbounded loop; include a reachable exit condition or `break`.

## for

A `for` loop iterates over each element of a list, each position of a string, or each entry of a map. The loop variable is assigned in the current scope.

```
result = []
for item in [2, 4, 6]
    result.push(item * 2)
end for
print result // [4, 8, 12]

characters = []
for character in "abc"
    characters.push(character)
end for
print characters.join("") // abc
```

For a map, the loop variable is a small map with `key` and `value` members. Map iteration order is unspecified; sort derived output if a deterministic order is required.

```
entries = []
for entry in {"a": 1, "b": 2}
    entries.push(entry.key + "=" + entry.value)
end for
entries.sort
print entries.join(",") // a=1,b=2
```

Inside the loop, `__name_idx` holds the current position, where `name` is the loop variable's name.

```
for item in ["a", "b"]
    print __item_idx + ": " + item
end for
// 0: a
// 1: b
```

Avoid changing the structure of a collection while iterating over it. [Errors and pitfalls](#guide-errors) shows how to iterate over a snapshot instead.

## range

[range](#GENERAL_RANGE) returns a list of numbers and is commonly used for counted loops. Both endpoints are inclusive. The default step is `1` when the end is greater than the start and `-1` otherwise; an explicit third argument sets the step.

```
print range(1, 3)     // [1, 2, 3]
print range(3, 1)     // [3, 2, 1]
print range(1, 5, 2)  // [1, 3, 5]
```

With one argument, `range(n)` counts down from `n` to `0`, so it has `n + 1` elements. A step of `0` is a runtime error.

```
print range(3) // [3, 2, 1, 0]
```

Use [indexes](#LIST_INDEXES) or `range(0, items.len - 1)` when a loop needs list indices. Take care with an empty list: `items.indexes` directly expresses the intended empty sequence.

## break

`break` terminates the innermost enclosing loop. `continue` skips the remainder of the current iteration and proceeds to the next one. In a `while` loop, `continue` reevaluates the condition; in a `for` loop, it advances to the next element.

```
result = []
for n in [1, 2, 3, 4, 5]
    if n == 2 then continue
    if n == 4 then break
    result.push(n)
end for
print result // [1, 3]
```

## Nested loops

With nested loops, `break` and `continue` apply only to the innermost loop. There is no labeled break; use a flag or refactor the inner loop into a function when an outer loop must terminate.

```
result = []
for row in range(1, 3)
    for column in range(1, 3)
        if row == 2 and column == 2 then break
        result.push(row + ":" + column)
    end for
end for
print result.join(",") // 1:1,1:2,1:3,2:1,3:1,3:2,3:3
```

Greybel also permits a loop body on the same line as its header; see [Greybel syntax](#guide-greybel).
