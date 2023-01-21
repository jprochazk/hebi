# Calls

Caller frame stores the callee in a register, followed by N arguments.

The `call` opcode creates a new call frame for the callee, ensuring sufficient stack space and copying arguments.

The `call_kw` opcode does the same, but it also expects the `kwargs` dictionary to be on the stack after the callee.

### Positional-only

```
# code:
fn f(a, b, c):
  print a, b, c

f(0, 1, 2)

# bytecode:
<func f>:
  create_empty_list
  store_reg r3
  load_reg r0
  push_to_list r3
  load_reg r1
  push_to_list r3
  load_reg r2
  push_to_list r3
  print_list r3

<func main>:
  load_const [0] # <func f>
  store_reg r0
  store_small_int 0, r1
  store_small_int 1, r2
  store_small_int 2, r3
  call r0, 3


# call frame for `f`:
start ->  f    <func a> # callee
          r0   0        # params
          r1   1
          r2   2   
          r3   []       # locals

# stack at the time of call to `f`:

0: <func a> <-- `main` locals
1: 0 
2: 1
3: 2
4: 0        <-- `f` locals
5: 1
6: 2
7: none

```

### Positional-only with defaults

```
# code:
fn f(a, b, c=10):
  print a, b, c

f(0, 1, 2)
f(0, 1)

# bytecode:
<func f>:
  load_param a2
  is_some
  jump_if_false @a2_is_set
  push_small_int 10
  store_param a2
@a2_is_set:
  create_empty_list
  store_reg r0
  load_param a0
  push_to_list r0
  load_param a1
  push_to_list r0
  load_param a2
  push_to_list r0
  print_list r0

<func main>:
  load_global [0] # <func f>
  store_reg r0
  store_small_int 0, r1
  store_small_int 1, r2
  store_small_int 2, r3
  call r0, 3
  
  load_global [0] # <func f>
  store_reg r0
  store_small_int 0, r1
  store_small_int 1, r2
  call r0, 2


# call frame for `f` (n=3):
start ->  f    <func a> # callee
          a1   0        # params
          a2   1
          a3   2   
base  ->  r0   []       # locals

# call frame for `f` (n=2):
start ->  f    <func a> # callee
          a1   0        # params
          a2   1
          a3   none   
base  ->  r0   []       # locals
```

### Positional as keyword

```
# code:
fn f(a, b, c=10):
  print a, b, c

f(0, b=2)

# pseudo-code for `f`
  if !is_set(reg(a)):
    if kw.has("a"):
      reg(a) = kw.get("a")
  if !is_set(reg(b)):
    if kw.has("b"):
      reg(b) = kw.get("b")
  if !is_set(reg(c)):
    if kw.has("c"):
      reg(c) = kw.get("c")
    else:
      reg(c) = default(c) # 10
  temp = []
  temp.push(reg(a))
  temp.push(reg(b))
  temp.push(reg(c))
  print_list(temp)

# bytecode:
<func f>:
  

<func main>:
  load_global [0]   # loads <func f>
  store_reg r0      # callee
  create_empty_dict 
  store_reg r1      # kw dict
  push_small_int 0  #
  store_reg r2      # a (0)
  load_const [1]    #
  store_reg r3      #
  push_small_int 2  #
  insert_to_dict r1 # b=2
  call_kw r0, 1 # `call_kw` implies callee is followed by kw dict

```
