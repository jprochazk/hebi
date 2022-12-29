```python

# variable declaration
v := 0

# values
v = null # null
v = 0.1 # number
v = true # bool
v = "\tas\\df\x2800\n" # string
v = [null, 0.1, true, "\tas\\df\x2800\n"] # array
# object
v = {a: null, b: 0.1, c: true, d: "\tas\\df\x2800\n"}
v = {["a"]: null, ["b"]: 0.1, ["c"]: true, ["d"]: "\tas\\df\x2800\n"}
v = {[0]: null, [1]: 0.1, [2]: true, [3]: "\tas\\df\x2800\n"}

# operators
v = 2 + 2
v = 2 - 2
v = 2 / 2
v = 2 * 2
v = 2 % 2
v = 2 ** 2
v = 2 == 2
v = 2 != 2
v = 2 > 2
v = 2 >= 2
v = 2 < 2
v = 2 <= 2
v = -2
v = !true
v = true && true
v = false || true
v = a ?? b

# assignment
v = 1 # first ocurrence in a scope creates a new variable
v += 1
v -= 1
v /= 1
v *= 1
v %= 1
v **= 1
v ??= 1

# postfix
v.a
v["a"]
v(a)

# functions
fn add(a, b):
  return a + b

v = add(0, 1)

fn fib(n):
  if n < 2:
    return n
  else:
    return n * fib(n - 1)

fn print_fib(n):
  print(fib(n))

# loops
# range is an object
for i in 0..10:
  print(i)

# `yield` inside `fn` makes it an iterator
# when called, iterators return an object with a `next` method
# an iterator is done when its `next` method returns null
fn counter(start, step, end):
  n := start
  loop:
    yield n
    n += step
    if end && n > end:
      return

for n in counter(0, 10, 100):
  print(n)

v = 0
while v < 10:
  print(v)
  v += 1

v = 0
loop:
  if v >= 10:
    break
  print(v)
  v += 1

if v < 10:
  print("less than 10")
elif v < 20:
  print("less than 20")
else:
  print("very large")

class Test:
  init(self, n):
    self.n = n

  get_n(self):
    return self.n

  test1(self):
    print("instance", self)

  test0():
    print("static", Test)

v = Test()
print(v.get_n() == Test.get_n(v)) # true

v = Test(n: 10)

Test.test0()
v.test1()

# errors
# no exceptions, panic = abort
panic("asdf")

# modules
# json_test.t
use json
# other ways to import:
# use json.parse
# use json.{parse}
# use {json}
# use {json.parse}
# use {json.{parse}}

v = json.parse("{\"a\":0, \"b\":1}")
print(v) # { a: 0, b: 1 }

# data class, implicit initializer
class A:
  a = 100
  # init(self, a = 100):
  #   self.a = a

print(A().a)     # 100
print(A(a=10).a) # 10

class B:
  a = 100
  init(self): # override the implicit initializer
    pass

print(B().a)   # 100
# `a` is ignored
print(B(a=10)) # 100

class C:
  # fields do not have to be declared
  # and may be added in the initializer
  # after `init` is called, the class is frozen
  # no fields/methods may be added or removed
  init(self):
    self.a = 10

print(C().a) # 10
C().b = 10 # error: cannot add new field `b` to class `C`

class A:
  inherited(self):
    print("test 0")

class B(A): pass

A().inherited() # test 0
B().inherited() # test 0

class C(B):
  inherited(self): # override
    print("test 1")

C().inherited() # test 1

class D(C):
  inherited(self): # override with call to super
    super.inherited()
    print("test 2")

D().inherited() # test 1
                # test 2

class X:
  init(self):
    self.v = 10

class Y(X):
  init(self): # error: `super.init` must be called before accessing `self` or returning in derived constructor
    self.v = 10

class Z(X):
  init(self, v):
    super.init(self)
    self.v += v

print(Z(v=15).v) # 25
```

