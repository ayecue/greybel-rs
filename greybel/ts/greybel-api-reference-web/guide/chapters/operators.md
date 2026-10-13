# Operators

An operator combines one or two operands into an expression. Parentheses override precedence. GreyScript operators return values; assignment is a statement rather than an expression.

## Arithmetic

Numeric operators are `+` for addition, `-` for subtraction, `*` for multiplication, `/` for division, `%` for remainder, and `^` for exponentiation. Unary `-` negates a number. There are no increment or decrement operators in core GreyScript; use `x = x + 1`, or Greybel's `x += 1`.

```
print 7 + 2  // 9
print 7 - 2  // 5
print 7 * 2  // 14
print 7 / 2  // 3.5
print 7 % 3  // 1
print 2 ^ 8  // 256
```

Some operators are overloaded: `+` concatenates strings, lists, and maps; `-` removes a string suffix; `*` and `/` replicate or reduce strings and lists. See [Values](#guide-values). Greybel adds compound assignment and bitwise operators, documented in [Greybel](#guide-greybel).

## Comparison

The comparison operators are `==`, `!=`, `<`, `<=`, `>`, and `>=`. They return `1` or `0` when an ordering or equality is defined. Numbers compare numerically, and strings compare lexicographically with case sensitivity.

```
print 2 < 10       // 1
print "abc" < "abd" // 1
print "B" == "b"   // 0
```

Equality between values of different types is false rather than an implicit conversion. Ordered comparisons are defined for two numbers or two strings. Unsupported operand combinations yield `null`; for example, comparing a number with a string does not produce a numeric ordering.

```
print 5 == "5"             // 0
print 5 != "5"             // 1
print (1 < "2") == null    // 1
```

List and map equality compares contents recursively; it does not test whether two operands are the same object. Two distinct lists can therefore be equal even though mutations to one are not visible through the other.

```
left = [1, 2]
right = left[0:]
print left == right // 1
left.push(3)
print left  // [1, 2, 3]
print right // [1, 2]
print {"a": [1]} == {"a": [1]} // 1
```

Game objects such as shells, computers, files, routers, ports, and libraries are different: `==` and `hash` use object identity rather than recursively comparing their map contents. The game returns the same object again for the same underlying entity, so repeated retrieval compares equal. There is no separate identity-comparison operator in the core API.

## Truthiness

Conditions and logical operators use truthiness. The false values are numeric zero, `null`, `""`, `[]`, and `{}`. Nonzero numbers and nonempty strings, lists, and maps are true. The constants `false` and `true` are exactly `0` and `1`.

```
if 0 then print "not reached" else print "zero is false"
if "" then print "not reached" else print "empty string is false"
if [] then print "not reached" else print "empty list is false"
if {} then print "not reached" else print "empty map is false"
if null then print "not reached" else print "null is false"
if -1 then print "nonzero is true"
```

Use an explicit comparison such as `value != 0` or `value != null` when a condition should accept only a particular state rather than general truthiness.

## Logic

`not` returns `1` when its operand is false and `0` otherwise. `and` and `or` always return numbers and use short-circuit evaluation: the right operand is not evaluated when the left operand determines the result. They do not select and return one of their operands.

```
calls = 0
probe = function(value)
    globals.calls = globals.calls + 1
    return value
end function
print 0 and probe(1) // 0
print 1 or probe(1)  // 1
print calls          // 0
print not ""         // 1
print 0 or "ready"   // 1
```

With numeric operands, `not`, `and` and `or` implement fuzzy logic. `not a` computes `1 - a`, `a and b` computes `a * b`, and `a or b` computes `a + b - a * b`; operands and results are taken as absolute values clamped to the interval from `0` to `1`. Non-numeric operands are converted according to truthiness.

```
print 0.5 and 0.5 // 0.25
print 0.5 or 0.5  // 0.75
print 2 and 3     // 1
print 1 and 0.5   // 0.5
print not 0.25    // 0.75
print not -2      // 0
```

Use fractional operands deliberately. For ordinary Boolean conditions, keep values at `0` or `1`.

## Type tests

`value isa type` tests runtime type and inheritance. The intrinsic right-hand operands include `number`, `string`, `list`, `map`, and `funcRef`. For class instances, `isa` walks the `__isa` chain. See [Types and conversion](#guide-type-conversion) and [Classes](#guide-classes).

```
print 5 isa number        // 1
print "x" isa string      // 1
print [] isa list         // 1
print {} isa map          // 1
print @print isa funcRef  // 1
```

## Precedence

The table lists precedence from lowest to highest. Operators on the same row have equal precedence. Binary operators at the same level associate left to right, including exponentiation.

| Precedence | Operators                                                   |
| ---------- | ----------------------------------------------------------- |
| 1          | `or`                                                        |
| 2          | `and`                                                       |
| 3          | `not`                                                       |
| 4          | `isa`                                                       |
| 5          | `\|` (Greybel only)                                         |
| 6          | `&` (Greybel only)                                          |
| 7          | `==`, `!=`, `<`, `<=`, `>`, `>=`                            |
| 8          | `+`, `-`                                                    |
| 9          | `<<`, `>>`, `>>>` (Greybel only)                            |
| 10         | `*`, `/`, `%`                                               |
| 11         | Unary `-`                                                   |
| 12         | `new`                                                       |
| 13         | `@`                                                         |
| 14         | `^`                                                         |
| 15         | Indexing `[]`, slicing `[:]`, calls `()`, member access `.` |

```
print 2 + 3 * 4   // 14
print (2 + 3) * 4 // 20
print -2 ^ 2      // -4
print (-2) ^ 2    // 4
print 2 ^ 3 ^ 2   // 64
print not 1 == 0  // 1
```

Use parentheses when an expression mixes unfamiliar operators, especially Greybel bitwise operators and comparisons.
