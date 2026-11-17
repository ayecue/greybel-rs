# Syntax

GreyScript source is a sequence of statements. Line breaks terminate statements, keywords delimit blocks, and indentation is optional. Identifiers and keywords are case-sensitive.

## Statements

Write one statement per line. A semicolon may separate multiple statements on the same line, but no semicolon is required at the end of a statement.

```
x = 1; y = 2
print x + y // 3
```

Blocks begin with keywords such as `if`, `for`, `while`, or `function` and end with the matching `end if`, `end for`, `end while`, or `end function`. Braces delimit map literals, not statement blocks. Indent nested statements for readability; the parser does not assign meaning to indentation.

## Continuation

A statement continues on the next line when the current line ends with an opening parenthesis, bracket, or brace, a comma, or a binary operator. This permits long expressions and collection literals without an explicit continuation character.

```
total = 1 +
    2 +
    3
names = [
    "Ada",
    "Grace"
]
print total // 6
print names.join(", ") // Ada, Grace
```

## Comments

A line comment begins with `//` and extends to the end of the line. It can occupy an entire line or follow a statement.

```
// Compute the answer.
answer = 6 * 7
print answer // 42
```

Greybel also supports block comments; see [Greybel syntax](#guide-greybel).

## Identifiers

Identifiers contain letters, digits, and underscores and cannot begin with a digit. Case is significant: `print`, `Print`, and `PRINT` are different names. By convention, class names begin with an uppercase letter and other identifiers with a lowercase letter. Avoid leading underscores for program-defined globals because the host uses such names for internal facilities such as `__isa`.

## Keywords

The keywords are `and`, `break`, `continue`, `else`, `end`, `for`, `function`, `if`, `in`, `isa`, `new`, `not`, `or`, `repeat`, `return`, `then`, and `while`. `repeat` is reserved but has no statement; using it is a syntax error. A keyword cannot be used as an identifier.

`true`, `false`, and `null` are literals and cannot be assigned. `import_code` is parsed as a statement and requires its path in parentheses. Greybel additionally recognizes `debugger` and compile-time directives beginning with `#`.

`self`, `super`, `globals`, `locals`, and `outer` are predefined identifiers rather than keywords: they have special meaning, but the parser treats them as ordinary names. Names such as `print`, `range`, `params`, and `module` identify values supplied by the runtime.

## Literals

Number literals support integers, decimal fractions, and scientific notation. A decimal point may appear on either side of the significant digits. Negative numbers are formed with the unary `-` operator.

```
print 42       // 42
print 3.1415   // 3.1415
print .5       // 0.5
print 5.       // 5
print 1e3      // 1000
print 1.25e-2  // 0.0125
```

String literals use double quotes. Double a quote to include it in the string. Backslashes have no escape meaning; in particular, `\n` represents two characters rather than a newline. By default, [print](#GENERAL_PRINT) converts that two-character sequence to a line break; pass zero as its third argument to print it unchanged.

```
print "say ""hi"""       // say "hi"
print "a\nb".len         // 4
print("a\nb", 0, 0)      // a\nb
```

For an explicit line break in a value, concatenate [char(10)](#GENERAL_CHAR).

```
lines = "first" + char(10) + "second"
print lines.len // 12
```

Grey Hack's lexer does not accept a string literal spanning multiple source lines. Greybel accepts multiline strings; a build for the game writes each as its lines concatenated with a `char(10)` variable, or joined by a function (see [Greybel](#guide-greybel)).

Other literal forms are `true`, `false`, `null`, list literals such as `[1, 2]`, map literals such as `{"id": 1}`, and function expressions introduced by `function`. See [Values](#guide-values) and [Functions](#guide-functions).

## Parentheses

Parentheses group expressions, contain arguments in expression-position calls, and declare function parameters. `if` and `while` conditions need none, though parentheses around them are allowed. A call without arguments can omit them, and a statement-position call can omit them around its arguments.

```
print (2 + 4) * 7 // 42
print cos(0)      // 1
print len("abc") // 3
```

[Functions](#guide-functions) describes implicit calls and the `@` operator.
