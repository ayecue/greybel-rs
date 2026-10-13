// range as a value
print range(3)
print range(1, 3)
print range(3, 1)
print range(0, 1, 0.25)
print range(0, 2, 0.7)
print range(5, 0, -2)
print range(0, 0)
r = range(2)
r.push(7)
print r

// a for loop over range takes the same numbers, added up step by step
items = []
for x in range(0, 1, 0.1)
  items.push(x)
end for
print items == range(0, 1, 0.1)
print items.len
print items[-1] == 0.9999999999999999

// setting the loop index back and on
seen = []
for x in range(10, 20)
  seen.push(x)
  if x == 13 and seen.len < 6 then __x_idx = 0
  if x == 15 then __x_idx = 7
end for
print seen

// nested, continue and break; in a function too
f = function(n)
  out = []
  for i in range(1, n)
    for j in range(i, 1, -1)
      if j == 2 then continue
      out.push(i * 10 + j)
    end for
    if i == 3 then break
  end for
  return out
end function
print f(5)

// a redefined range is called as itself
original = @range
range = function(a, b)
  return ["mine", a, b]
end function
for m in range(1, 2)
  print m
end for
range = @original

print range(0, 1, 0)
