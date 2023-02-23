### Variables, Values, Basic Operators

```python
# Unlike Python, Mu has explicit declarations
v := 0

# Variables may be reassigned at will
v = 10

# Mu supports a whole host of different kinds of values
v = none # none, equivalent to Python's None or JavaScript's null
v = 0.1 # number, which may be represented as either an integer or a float, but that is an implementation detail.
v = true # bool, used to represent true or false

# Along with the primitive types above, Mu also supports various object types:
# Strings - may be multiline, also supports common escapes
v = "\tas\\df\x2800\n"

# Lists - heterogenous, meaning they may hold multiple types of values at the same time
v = [none, 0.1, true, "\tas\\df\x2800\n"]

# Dictionaries - associative key->value maps
# Only integers and strings may be used as keys
v = {a: none, b: 0.1, c: true, d: "\tas\\df\x2800\n"}
v = {["a"]: none, ["b"]: 0.1, ["c"]: true, ["d"]: "\tas\\df\x2800\n"}
v = {[0]: none, [1]: 0.1, [2]: true, [3]: "\tas\\df\x2800\n"}

# Mu supports many different kinds of operators:
# Common arithmetic
2 + 2
2 - 2
2 / 2
2 * 2

# Remainder, exponent
2 % 2
2 ** 2

# Comparisons
2 == 2
2 != 2
2 > 2
2 >= 2
2 < 2
2 <= 2

# Negation
-2
!true

# Logical
true && true
false || true

# None coalescing
# if `a` is not `none`, yields it, otherwise yields `b`
a ?? b

# Postfix operators
v.a    # field access
v["a"] # index access
v(a)   # function call
```

### Control Flow

```python
# Mu supports a few different kinds of control flow
# Note that indentation is used to denote blocks, just like in Python

# 1. If statements
if v == 1:
  print "`v` is exactly one"
elif v == 2:
  print "`v` is exactly two"
else:
  print "`v` is something else... not sure what, though."

# 2. Infinite loops
loop:
  print "I will not terminate unless I encounter a `break`"
  # With break/continue
  break

# 3. While loops
v = 0
while v < 10:
  print v
  v += 1

# 4. For loops
for i in 0..10: # this syntax denotes a range
  print i
```

### Functions

```python
# A basic function is declared using the `fn` keyword
fn add(a, b):
  return a + b

v := add(0, 1) # calling the above function
print v # prints `1`

# Functions may refer to themselves
fn factorial(n):
  if n < 2:
    return n
  else:
    return n * factorial(n - 1)


# Mu supports keyword arguments, but they are not enabled by default:
fn a(v): ...
a(v=20) # error: unknown keyword param `v`
a(20) # ok!

# To enable them, add a `*` before the first parameter which you want
# to turn into a keyword parameter:
fn b(*, v): ...
b(v=20) # ok!
# This however turns off its ability to be passed as a positional parameter:
b(20)

# You have to make a choice if you want your parameter
# on the right or left side of the `*`. This is because
# function calls in Python are quite expensive, and by
# having the developer declare their intent like this,
# we can make positional-only calls much easier to optimize.

# They're still very useful for adding options to your functions!
fn transform(weight, *, bamboozle_factor=0.3, discombobulator=none):
  if discombobulator is not none:
    return discombobulator.discombobulate(weight * bamboozle_factor)
  else:
    return weight * bamboozle_factor / 6.28


# Here's the full range of syntax for parameters on a single function:
fn c(first, second=2, *argv, third, fourth=4, **kwargs):
  print first, second, argv, third, fourth, kwargs

c(1, third=3) # prints `1 2 [] 3 4 {}`
c(10, 20, 40, third=30, fourth=0, other=-1) # prints `10 20 [40] 30 0 {other: -1}`
```

### Classes, Inheritance

```python
# Mu supports declaring classes
class Test:
  # The runtime will call the `init` method,
  # if present, after initializing the object
  fn init(self, *, n=0):
    self.n = n

  fn get_n(self):
    return self.n

  fn test1(self):
    print("instance", self)

  fn test0():
    print("static", Test)

# By "calling" the class, you create an instance of it:
v := Test()
print(v.get_n() == Test.get_n(v)) # true

# You may declare your classes using this simpler syntax
# if all you want is a few fields:
class A:
  a = 100

print(A().a)     # 100

# The field becomes a keyword parameter on the constructor:
print(A(a=10).a) # 10

# You can of course freely mix fields with the initializer:
class B:
  a = 100
  fn init(self):
    pass

# But if you do that, the fields will no longer be configurable,
# unless you put them in your initializer:
print(B().a)   # 100
print(B(a=10)) # error: unknown keyword param `a`

# The initializer may be used to add fields dynamically:
class C:
  fn init(self, include):
    if include:
      self.a = 10

print(C(true).a) # 10
print(C(false).a) # error: cannot get field `a` of `<class C>`

# But after the initializer runs, no more fields may be added
# to the class. In Mu, we refer to that as a frozen class:
C(true).b = 10 # error: cannot add field `b` to frozen class


# Classes also support inheritance, but only with one parent,
# to prevent the diamond problem.
class A:
  fn inherited(self):
    print("test 0")

class B(A): pass

B().inherited() # prints `test 0`

class C(B):
  # Methods may be overriden by redeclaring them on the child class:
  fn inherited(self):
    print("test 1")

C().inherited() # prints `test 1`

class D(C):
  # You can still refer to the method in the parent class
  # if you need to, using the `super` keyword:
  fn inherited(self):
    super.inherited()
    print("test 2")

D().inherited() # prints:
                #   `test 1`
                #   `test 2`
```

### Unfinished

This document is not finished, as some features are not yet fully implemented:
- [Generators and Yield](https://github.com/jprochazk/mu/issues/2)
- [Modules and Imports](https://github.com/jprochazk/mu/issues/3)
