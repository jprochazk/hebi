use super::*;

#[test]
fn lex_design_file() {
  let input = r#"
# values
v = none # none
v = 0.1 # number
v = true # bool
v = "\tas\\df\x2800\n" # string
v = [none, 0.1, true, "\tas\\df\x2800\n"] # array
# object
v = {a=none, b=0.1, c=true, d="\tas\\df\x2800\n"}
v = {["a"]=none, ["b"]=0.1, ["c"]=true, ["d"]="\tas\\df\x2800\n"}
v = {[0]=none, [1]=0.1, [2]=true, [3]="\tas\\df\x2800\n"}

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
# last expr return
fn add(a, b):
  a + b

v = add(0, 1)

fn fib(n):
  if n < 2: n
  else: n * fib(n - 1)
fn print_fib(n):
  print(fib(n))

# loops
# range is an object
for i in 0..10:
  print(i)

# `yield` inside `fn` makes it an iterator
# when called, iterators return an object with a `next` method
# an iterator is done when its `next` method returns none
fn counter(start, step, end):
  n = start
  loop:
    yield n
    n += step
    if end && n > end: return

for n in counter(0, 10, 100):
  print(n)

v = 0
while v < 10:
  print(v)
  v += 1

v = 0
loop:
  if v >= 10: break
  print(v)
  v += 1

if v >= 10: print("larger than 10", v)
else: print("not larger than 10", v)

if v < 10:
  print("less than 10")
else if v < 20:
  print("less than 20")
else:
  print("very large")

# `if` is an expression
fn clamp(v, min, max):
  if v < min: min
  elif v > max: max
  else: v

class Test:
  init(self, n):
    self.n = n

  get_n(self):
    self.n

  test0():
    print("static", Test)

  test1(self):
    print("instance", self)

v = Test()
print(v.get_n() == Test.get_n(v)) # true

# errors
# no exceptions, panic = abort
panic("asdf")

# modules
# json_test.t
import json
# other ways to import:
# import json.parse
# import json.{parse}
# import {json}
# import {json.parse}
# import {json.{parse}}

v = json.parse("{\"a\":0, \"b\":1}")
print(v) # { a: 0, b: 1 }
"#;

  let lexer = Lexer::new(input);
  let tokens = Tokens(lexer)
    .map(|(s, t)| DebugToken(t, s))
    .collect::<Vec<_>>();

  insta::assert_debug_snapshot!(tokens)
}
