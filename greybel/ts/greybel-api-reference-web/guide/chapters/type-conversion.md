# Types and conversion

GreyScript is dynamically typed: a variable can hold values of different types over its lifetime. Values still have concrete runtime types, and operations may require particular types.

## Runtime types

[typeof](#GENERAL_TYPEOF) returns a type name as a string. The language values report `number`, `string`, `list`, `map`, `function`, or `null`. Game objects report names such as `shell`, `computer`, and `file`.

Use `@` when inspecting a function; otherwise the function is implicitly called and `typeof` receives its return value.

```
print typeof(3.14)     // number
print typeof("text")   // string
print typeof([1, 2])   // list
print typeof({"a": 1}) // map
print typeof(null)     // null
print typeof(@print)   // function
```

A map with a `classID` entry reports that value as its type, which can tell kinds of records apart. Game objects use the same entry. The lookup follows `__isa`, so a prototype can supply the `classID`.

```
Record = {"classID": "record"}
value = new Record
print typeof(value) // record
```

The `isa` operator tests against the intrinsic maps `number`, `string`, `list`, `map`, and `funcRef`. It also walks an object's `__isa` chain, which makes it suitable for class checks. Every class or object is also a `map`.

```
print 3 isa number       // 1
print "x" isa string     // 1
print [] isa list        // 1
print {} isa map         // 1
print @print isa funcRef // 1
```

See [Operators](#guide-operators) and [Classes](#guide-classes).

## To numbers

[val](#GENERAL_VAL), usually written as the string method `.val`, parses a complete numeric string and returns a number. It accepts signs, decimal fractions, surrounding whitespace, and scientific notation. Invalid numeric text returns `0`; check the input first if invalid text must be distinguished from an actual zero.

```
print "12.5".val + 0.5 // 13
print "1e3".val        // 1000
print " 12 ".val       // 12
print "invalid".val    // 0
```

[to_int](#GENERAL_TO_INT) parses the same invariant-culture numeric forms, including thousands separators and parenthesized negatives, but returns non-numeric text unchanged instead of zero. A number is returned as-is, other types give `null`, and the global function defaults to zero.

```
print to_int("1.5")       // 1.5
print to_int(" 1e3 ")     // 1000
print to_int("1,000")     // 1000
print to_int("(5)")       // -5
print to_int("invalid")   // invalid
```

Operators do not generally parse numeric strings implicitly. For example, `"2" * 3` replicates a string rather than multiplying two numbers. Convert input explicitly with `val` or `to_int` before numeric arithmetic.

## To strings

[str](#GENERAL_STR) converts a value to its string representation. [print](#GENERAL_PRINT) performs the same conversion for display. Concatenation with `+` also converts values when one operand is a string, but explicit `str` can make intent clearer.

```
print str(42) + "!"          // 42!
print str([1, "x"])           // [1, "x"]
print "level " + 3            // level 3
print "value=" + str(true)    // value=1
```

A string converted from `null` is empty. Collection formatting is intended for display and debugging, not as a serialization format.

## Characters

[code](#GENERAL_CODE) returns the numeric code of a string's first character, and [char](#GENERAL_CHAR) constructs a character from a numeric code.

```
print "A".code // 65
print char(65) // A
```

## Truth conversion

Conditions and logical operators apply truthiness rather than requiring a separate Boolean type. The constants `false` and `true` are the numbers `0` and `1`. Zero, `null`, and empty strings, lists, and maps are false; nonzero numbers and nonempty collections are true. See [Truthiness](#guide-operators).
