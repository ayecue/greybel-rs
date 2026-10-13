// Where C# MiniScript differs from the TypeScript intrinsics.
print round(2.5)
print round(3.5)
print round(-2.5)
print round(2.675, 2)
print round(12345, -3)
print [0].hasIndex(0)
print [0].hasIndex(-1)
print [0].hasIndex(1)
print "abc".hasIndex(-3)
print {"a": 1}.hasIndex("a")
m = {"a": 1, "b": 2, "c": 3}
print m.pop
print m
print m.pull
print m
m = {}
m.push "x"
m.push "x"
print m
l = [1, 2, 1, 2, 1]
l.replace 1, 9, 2
print l
m = {"a": 1, "b": 1, "c": 1}
m.replace 1, 0, 2
print m
print "aXbXcX".replace("X", "-", 2)
print bitAnd(-6, -3)
print bitOr(-6, 3)
print bitXor(6, -3)
print bitAnd(2^40 + 5, 2^40 + 3)
print code(char(128512))
print char(128512).len
print val("1,000")
print val("(5)")
print val("5-")
print val("$5")
print val(" 1e3 ")
print atan(1, -1)
print atan(1)
print log(100)
print log(8, 2)
print log(2.718282, 2.718282)
print @abs
print @insert
print @sort
print @replace
print funcRef == @funcRef
print list.hasIndex("push")
print string.hasIndex("upper")
print map.hasIndex("pop")
print number.len
print intrinsics.hasIndex("print")
print intrinsics.hasIndex("hasIndex")
print [1, 2, 3].insert(-1, 9)
print "abc".insert(-1, "X")
print "a,b,c,d".split(",", 2)
print "a,b,".split(",")
print "abc".split("")
print slice([1, 2, 3, 4], -2)
print slice("hello", 1, -1)
print [{"k": 3}, {"k": 1}, {"k": 2}].sort("k")
print [[3, "c"], [1, "a"], [2, "b"]].sort(0, false)
print [3, null, 1, "b", "a"].sort
a = "same"
b = a
print refEquals(a, b)
print refEquals([1], [1])
print refEquals(1, 1)
print [1, "2", [3]].sum
print str(null) + "|" + str(1.5) + "|" + str([1, "a"])
print [1, null, "a"].join("-")
print "aé😀b".len
print "aé😀b".indexOf("b")
print "aé😀b".values
print range(1, 10, 3)
print range(3, 1)
print range(1, 2, 0.5)
print sign(-3) + sign(0) + sign(9)
print yield
print stackTrace.len
print rnd(1) == rnd(1)
x = rnd(7)
print rnd(7) == x
