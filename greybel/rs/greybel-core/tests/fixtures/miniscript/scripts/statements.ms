f = function(a, b = 1, c = "x")
	if a then
		return b
	else if b then
		return c
	else
		return null
	end if
end function
if x then y = 1 else y = 2
while i < 10
	i += 1
	if i == 5 then continue
	if i == 8 then break
end while
for item in [1, 2, 3]
	print item
end for
print 1, 2
m = {"a": 1, "b": [1, 2]}
m.a -= 1
self.x = super.x
