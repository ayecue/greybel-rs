# Classes and objects

GreyScript implements object-oriented programming through prototype-based inheritance. A class or object is a map; the terms describe how those maps are used rather than distinct runtime types.

## Prototypes

`new Prototype` creates a map and sets its `__isa` entry to `Prototype`, the new map's parent (prototype). Member lookup walks the `__isa` chain until it finds the requested key. The `isa` operator walks the same chain.

```
Animal = {"sound": "..."}
Animal.speak = function()
    return self.sound
end function
Dog = new Animal
Dog.sound = "Woof"
rex = new Dog
print rex.sound      // Woof
print rex.speak      // Woof
print rex isa Dog    // 1
print rex isa Animal // 1
print rex isa map    // 1
```

Here `rex` inherits from `Dog`, which inherits from `Animal`. Reading `rex.sound` finds the override in `Dog`; reading `rex.speak` continues to `Animal`.

Reading searches the `__isa` chain, but assignment always writes the key on the receiver itself. It does not modify the prototype where an inherited value was found.

```
Base = {"name": "base"}
item = new Base
item.name = "item"
print item.name // item
print Base.name // base
```

`new` sets `__isa` but does not automatically invoke a constructor.

## Methods

A method is a function invoked through map member syntax. The implicit call receives `self`, bound to the original receiver even when the method was found on a prototype. Methods without explicit arguments therefore commonly use `self` for instance state.

```
Point = {"x": 0, "y": 0}
Point.length = function()
    return sqrt(self.x ^ 2 + self.y ^ 2)
end function
point = new Point
point.x = 3
point.y = 4
print point.length // 5
```

`self` is bound only by the call through the dot. A method stored with `@pet.name` and called later has no receiver: `self` is `null`, and so is every member read from it, without a runtime error. Wrap the call in a function to pass a method on.

```
Pet = {"label": "Rex"}
Pet.name = function()
    return self.label
end function
pet = new Pet
detached = @pet.name
print detached == null // 1
bound = function()
    return pet.name
end function
print bound // Rex
```

## super

Within a method, `super` starts lookup in the `__isa` chain above the prototype on which the current method was found. It allows an override to invoke the inherited implementation while preserving the same `self`.

```
Animal = {"sound": "..."}
Animal.speak = function()
    return self.sound
end function
Dog = new Animal
Dog.sound = "Woof"
Dog.speak = function()
    return super.speak + "!"
end function
rex = new Dog
print rex.speak // Woof!
```

Without `super`, calling `speak` from `Dog.speak` through `self` would recurse into the override.

## Construction

Define a factory method when instances require initialization. A factory creates an instance with `new`, sets its state, and returns it. Instance methods and defaults remain shared through the `__isa` chain.

```
Point = {"x": 0, "y": 0}
Point.create = function(x, y)
    point = new Point
    point.x = x
    point.y = y
    return point
end function
Point.length = function()
    return sqrt(self.x ^ 2 + self.y ^ 2)
end function
point = Point.create(3, 4)
print point.length // 5
print point isa Point // 1
```

Avoid storing mutable per-instance lists or maps only on a class prototype unless sharing them is intentional. Create those values in the factory so each instance owns a separate object.

## Built-in types

`number`, `string`, `list`, and `map` are intrinsic maps containing the methods available through dot syntax. Adding a function to one of these maps extends that type for subsequent calls. As with other methods, `self` is the value before the dot.

```
string.capitalized = function()
    if self.len < 2 then return self.upper
    return self[0].upper + self[1:]
end function
print "greyScript".capitalized // GreyScript
```

Numeric literals cannot be followed directly by dot syntax. Store a number in a variable or use parentheses before invoking an added numeric method.

```
number.twice = function()
    return self * 2
end function
value = 21
print value.twice // 42
```

Extensions modify shared intrinsic maps and can affect unrelated code, so use unique method names and add them deliberately.
