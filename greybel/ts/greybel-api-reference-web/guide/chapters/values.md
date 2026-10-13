# Values

The core value types are numbers, strings, lists, maps, functions, and `null`. Numbers and strings are immutable. Lists and maps are mutable objects with reference semantics. Classes, instances, and many game objects extend these basic representations.

## Numbers

Every number is a double-precision floating-point value; there is no distinct integer type. Integer-valued numbers support the same operations as fractional numbers. Division therefore does not truncate.

```
print 7 / 2  // 3.5
print 7 % 3  // 1
print 2 ^ 10 // 1024
```

Use [floor](#GENERAL_FLOOR), [ceil](#GENERAL_CEIL), or [round](#GENERAL_ROUND) when an integer result is required. [range](#GENERAL_RANGE) constructs inclusive numeric sequences, and [abs](#GENERAL_ABS), [sqrt](#GENERAL_SQRT), [log](#GENERAL_LOG), and the trigonometric functions provide common mathematical operations.

```
print floor(7 / 2)      // 3
print ceil(7 / 2)       // 4
print round(7 / 2)      // 4
print range(1, 5, 2)    // [1, 3, 5]
```

GreyScript represents Boolean results numerically: `true` is `1`, `false` is `0`, and comparisons return one of these values.

Floating-point arithmetic is inexact: `0.1 + 0.2 == 0.3` is false, although both sides print as `0.3`, because `print` rounds to six decimal places. Compare fractional results with a tolerance. Very small and very large numbers print in exponent notation.

```
print 0.1 + 0.2 == 0.3            // 0
print abs(0.1 + 0.2 - 0.3) < 1e-9 // 1
print 1 / 3                       // 0.333333
print 1e-7                        // 1.000000E-07
```

[rnd](#GENERAL_RND) returns a number from `0` up to but excluding `1`. Called with a seed, it starts a repeatable sequence, which suits tests and generated content. `floor(rnd * (max - min + 1)) + min` gives a whole number from `min` to `max`.

```
print rnd(42) == rnd(42) // 1
roll = floor(rnd * 6) + 1
print roll >= 1 and roll <= 6 // 1
```

## Strings

A string is an immutable sequence of text positions. String literals and escaping are described in [Syntax](#guide-syntax). Indexing starts at zero, and a negative index counts backward from the end.

```
text = "GreyScript"
print text[0]  // G
print text[-1] // t
print text.len // 10
```

The `+` operator concatenates strings and converts the other operand to text when necessary. The `-` operator removes a suffix when the left operand ends with the right operand. The `*` operator replicates a string, including a fractional final repetition; `/` is equivalent to multiplication by the reciprocal.

```
print "Grey" + "Hack" // GreyHack
print "level " + 3    // level 3
print "hello" - "lo"  // hel
print "hello" - "x"   // hello
print "ab" * 2.5      // ababa
print "Spam" / 2      // Sp
```

Strings cannot be mutated by index. Construct and assign a new string instead.

```
word = "spin"
word = word[:2] + "u" + word[3:]
print word // spun
```

Common string intrinsics include:

| Member                                                                             | Result                                                        |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| [len](#STRING_LEN)                                                                 | Number of text positions.                                     |
| [upper](#STRING_UPPER), [lower](#STRING_LOWER)                                     | Uppercase or lowercase copy.                                  |
| [trim](#STRING_TRIM)                                                               | Copy without surrounding whitespace.                          |
| [indexOf](#STRING_INDEXOF), [lastIndexOf](#STRING_LASTINDEXOF)                     | Position of a substring, or `null` when absent for `indexOf`. |
| [split](#STRING_SPLIT)                                                             | List of substrings.                                           |
| [replace](#STRING_REPLACE)                                                         | Copy with matches replaced.                                   |
| [insert](#STRING_INSERT), [remove](#STRING_REMOVE)                                 | Copy with text inserted or removed.                           |
| [hasIndex](#STRING_HASINDEX), [indexes](#STRING_INDEXES), [values](#STRING_VALUES) | Index tests, valid indices, or individual characters.         |
| [code](#STRING_CODE), [val](#STRING_VAL)                                           | Numeric character code or parsed numeric value.               |

These operations return new values where appropriate; none modifies the original string.

```
line = " Grey,Script "
print line.trim             // Grey,Script
print "GreyScript".upper    // GREYSCRIPT
print "abc".indexOf("b")    // 1
print "abc".indexOf("z") == null // 1
print "a,b,c".split(",")    // ["a", "b", "c"]
print "abc".split("")       // ["a", "b", "c"]
print "abc".replace("b", "X") // aXc
print "abc".hasIndex("x")   // 0
```

[split](#STRING_SPLIT) and [replace](#STRING_REPLACE) treat their pattern as a regular expression. Escape characters that have a meaning in regular expressions, such as `.`, `*`, `+`, `?`, `(`, `[` and `|`, to match them literally; splitting on an unescaped `"."` matches every character and also prints a warning.

```
print "a.b.c".split("\.")        // ["a", "b", "c"]
print "a.b.c".replace("\.", "-") // a-b-c
```

The reference also documents regular-expression operations such as [is_match](#STRING_IS_MATCH) and [matches](#STRING_MATCHES).

## Lists

A list is an ordered, mutable sequence. Elements can have different types, including lists and maps. Indices start at zero; negative indices count from the end.

```
scores = [10, 20, 30, 40]
print scores[0]  // 10
print scores[-1] // 40
scores[1] = 25
print scores // [10, 25, 30, 40]
```

The `+` operator concatenates two lists into a new list. `*` replicates a list and `/` reduces it using the same rules as strings.

```
print [1, 2] + [3]       // [1, 2, 3]
print [1, 2] * 2         // [1, 2, 1, 2]
print [1, 2, 3, 4] / 2   // [1, 2]
```

Lists have reference semantics. Assignment copies the reference, not the elements, so mutations through one variable are visible through every alias. A full slice creates a shallow copy: the new list contains the same element references.

```
original = [1, 2, 3]
alias = original
copy = original[0:]
original[-1] = 5
print alias // [1, 2, 5]
print copy  // [1, 2, 3]
```

Common list intrinsics include:

| Member                                                                       | Operation                                                 |
| ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| [len](#LIST_LEN)                                                             | Returns the element count.                                |
| [push](#LIST_PUSH)                                                           | Appends an element.                                       |
| [pop](#LIST_POP)                                                             | Removes and returns the last element.                     |
| [pull](#LIST_PULL)                                                           | Removes and returns the first element.                    |
| [insert](#LIST_INSERT)                                                       | Inserts an element at an index.                           |
| [remove](#LIST_REMOVE)                                                       | Removes the element at an index.                          |
| [sort](#LIST_SORT)                                                           | Sorts the list in place, optionally by a key.             |
| [join](#LIST_JOIN)                                                           | Concatenates the elements into a string with a delimiter. |
| [indexOf](#LIST_INDEXOF)                                                     | Finds an element's index, or returns `null`.              |
| [hasIndex](#LIST_HASINDEX), [indexes](#LIST_INDEXES), [values](#LIST_VALUES) | Tests indices or returns indices and elements.            |
| [replace](#LIST_REPLACE)                                                     | Replaces matching elements in place.                      |
| [sum](#LIST_SUM), [reverse](#LIST_REVERSE), [shuffle](#LIST_SHUFFLE)         | Sums numbers or rearranges elements.                      |

```
numbers = [3, 1, 2]
numbers.sort
numbers.push(4)
print numbers       // [1, 2, 3, 4]
print numbers.pop   // 4
print numbers.pull  // 1
numbers.insert(1, 9)
print numbers.join(",") // 2,9,3
numbers.remove(1)
print numbers       // [2, 3]
```

Use `push` with `pop` for stack behavior and `push` with `pull` for queue behavior. The `join` delimiter defaults to one space, so `numbers.join` is equivalent to `numbers.join(" ")`.

`values` on a list returns the same list, not a copy; use a full slice for a copy. `sort` changes the list itself. Its key sorts a list of maps by the value under that key, or a list of lists by the element at that index; sort a copy to keep the original order.

```
items = [1, 2]
same = items.values
same.push(3)
print items // [1, 2, 3]

records = [{"name": "b", "rank": 10}, {"name": "a", "rank": 2}]
records.sort("rank")
print records[0].name // a
pairs = [[10, "b"], [2, "a"]]
pairs.sort(0)
print pairs // [[2, "a"], [10, "b"]]
```

## Maps

A map associates unique string or number keys with values. Values can have any type. Reassigning an existing key replaces its value. Do not rely on a particular iteration order for a map with multiple entries.

```
player = {"name": "Ada", "level": 3}
print player["name"]  // Ada
print player.level    // 3
player.level = 4
player["score"] = 20
print player.len      // 3
```

Bracket indexing accepts a computed key. Dot syntax is shorthand for a string key that is a valid identifier: `player.level` is equivalent to `player["level"]`.

The `+` operator returns a merged map. Entries from the right operand replace entries with equal keys in the left operand.

```
base = {"level": 1, "active": 0}
updated = base + {"level": 2}
print updated.level      // 2
print updated.hasIndex("active") // 1
```

Maps are mutable and have reference semantics, like lists.

```
settings = {"flags": [1]}
alias = settings
alias.flags.push(2)
print settings.flags // [1, 2]
```

Common map intrinsics include:

| Member                                         | Operation                                               |
| ---------------------------------------------- | ------------------------------------------------------- |
| [len](#MAP_LEN)                                | Returns the entry count.                                |
| [hasIndex](#MAP_HASINDEX)                      | Tests whether a key exists.                             |
| [indexes](#MAP_INDEXES), [values](#MAP_VALUES) | Returns the keys or values as lists.                    |
| [indexOf](#MAP_INDEXOF)                        | Finds a key associated with a value, or returns `null`. |
| [remove](#MAP_REMOVE)                          | Removes a key-value pair.                               |
| [push](#MAP_PUSH)                              | Inserts a key with value `1`, supporting set-like use.  |
| [pop](#MAP_POP)                                | Removes and returns a key.                              |
| [replace](#MAP_REPLACE)                        | Replaces matching values in place.                      |
| [sum](#MAP_SUM), [shuffle](#MAP_SHUFFLE)       | Sums numeric values or redistributes values among keys. |

```
player = {"name": "Ada", "level": 3}
print player.hasIndex("name") // 1
print player.indexOf(3)       // level
player.remove("level")
print player.hasIndex("level") // 0
```

## Indexing

Strings and lists support slicing as well as single-element indexing. A slice `[from:to]` includes `from` and excludes `to`. An omitted lower bound defaults to the start; an omitted upper bound extends to the end. Negative bounds count from the end, and out-of-range slice bounds are clipped.

```
items = [10, 20, 30, 40]
print items[1:3] // [20, 30]
print items[:2]  // [10, 20]
print items[2:]  // [30, 40]
print items[:]   // [10, 20, 30, 40]
name = "GreyScript"
print name[:-6]  // Grey
print name[-6:]  // Script
```

[slice](#GENERAL_SLICE) provides the same operation in function form. Slicing a list creates a new list but does not recursively copy its elements. A slice cannot be assigned to: `items[1:3] = [8, 9]` is a syntax error. Change a list element by element, or with `insert` and `remove`.

## null

`null` represents the absence of a value. A function without an explicit result returns `null`, and searches such as `indexOf` use it when no result exists. It is false in a condition and distinct from zero or an empty string.

```
result = "abc".indexOf("z")
print result == null // 1
print null == 0      // 0
print not null       // 1
```

Test for `null` explicitly when `0` or an empty collection is a valid result.

## Functions

A function is a first-class value of type `function`. Evaluating a function reference normally invokes it; prefix the reference with `@` to store, pass, return, or inspect the function itself.

```
answer = function()
    return 42
end function
callback = @answer
print typeof(@callback) // function
print callback()        // 42
```

[Functions](#guide-functions) covers definitions, parameters, calls, closures, and higher-order functions.
