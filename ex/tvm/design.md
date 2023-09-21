
## Variables

```swift
var v: Int = 0 // mutable

// type inference
var v = 0

// bi-directional type inference
var v = #[] // type of `v` is `Array<_>`, `_` meaning "unknown"
v.push(10)  // type of `v` is `Array<Int>`
```

## Literals

```swift
var v: Int = 1
var v: Float = 1.0
var v: Bool = true
var v: String = "yo"
var v: (Int, Float) = #(1, 1.0)
var v: [Int] = #[1, 2, 3] // or just `Array<Int>`
var v: Map<String, Int> = #{"a": 0, "b": 1}
```

## Operators

```rust
2 + 2
2 - 2
2 / 2
2 * 2
2 % 2
2 ** 2
2 == 2
2 != 2
2 > 2
2 >= 2
2 < 2
2 <= 2
-2
!true
not true
true && true
true and true
false || true
false or true
a ?? b
```

```rust
// can only re-assign variable bindings (`var`)
name = 1
name += 1
name -= 1
name /= 1
name *= 1
name %= 1
name **= 1
// assigns to `name` only if `name` is none, and asserts that `name` is no longer none
name ??= 1
```

## Control flow

```rust
// block stmt
{
  let v = 0
}

// block expr (substitute for normal grouping expr)
let v = {1}

// if stmt (`else` is optional)
if true {}
else if true {}
else {}

// if expr (`else` is required)
let v = if true {0} else {1}
```

```rust
loop {}
:label loop {}

while true {}
:label while true {}

for v in it {}
:label for v in it {}
```

```rust
return
return value
break :label
continue :label

// all of the above are expr with type `!`
let v = return
let v = break
let v = continue

yield
yield value
yield from generator

// `yield` produces whatever `resume` is called with
let v = yield
```

## Functions

```rust
fn name {}
fn name() {}
fn name(a: A) {}
fn name(label a: A) {}
fn name(a: A, b: B, c: C) {}
fn name<T>(v: T) {}
fn name<T>(v: T) where T: I & J {}
fn name -> R {}
fn name() -> R {}
fn name(a: A) -> R {}
fn name(label a: A) -> R {}
fn name(a: A, b: B, c: C) -> R {}
fn name<T>(v: T) -> R {}
fn name<T>(v: T) -> R where T: I & J {}

// all of the above are also expressions
// name is optional in this case
let name = fn {}
let name = fn () {}
let name = fn (a: A) {}
let name = fn (a: A, b: B, c: C) {}
let name = fn <T>(v: T) {}
let name = fn <T>(v: T) where T: I & J {}
let name = fn -> R {}
let name = fn () -> R {}
let name = fn (a: A) -> R {}
let name = fn (a: A, b: B, c: C) -> R {}
let name = fn <T>(v: T) -> R {}
let name = fn <T>(v: T) -> R where T: I & J {}

// function arguments may be labelled
let name = fn (label a: A) {}
let name = fn (label a: A) -> R {}

// `fn` in expr position can also have its argument and return types inferred
fn adder(a: Int) -> fn (Int) -> Int {
  fn [a](b) { a + b }
}

fn map<T>(x: Array<T>, f: fn (T) -> U) -> Array<U> {
  var o = Array(len: x.len)
  for i, v in x {
    o[i] = v
  }
  o
}

// functions do not have the ability to implicitly capture their environment
// only globals and other declarations are accessible
// if you want to create a closure, variables must be captured explicitly:
let a = 10
let f = fn [a]() { a + 10 }
// closures may only be created in expression position,
// such as in the initializer of a variable or the value in `return`

// captures bindings copy the original binding, and are always immutable.
// this would be an error:
//   let f = fn [a]() { a = 10 }
```

## Function calls

```rust
print(adder(10)(50)) // 60

let square = fn (v) {v*2}
print(map(#[1,2,3], square)) // #[1, 4, 6]

fn greet(person name: String) {
  print(f"Hi, {name}!")
}

greet(person: "Jan")
```

## Classes

```rust
// all visibility is private by default
class Name<T, U> : Parent<U> where T: I & J, U: I & J {
  // note: all fields must be initialized somehow
  // either explicitly when the class is instantiated,
  // or with a default value.
  // note: classes are not closures. they may not capture anything.
  // note: if a class has one or more private fields, then
  // its default constructor is also private.
  var a: Int = 1 // mutable field
  var b: Int
  static factor: Int = 3 // static field

  // a constructor is a function which returns an instance of the class.
  // this is only a convention.
  pub fn new() -> Self {
    Self()
  }

  // instance method
  fn foo(self, v: Int) -> Int {
    self.a = v

    // `Self` = `Name<T>`
    self.a + self.b + Self.bar(v)
  }

  // static method
  fn bar(v: Int) -> Int {
    v + Self.factor
  }
}

type Foo = Name<Int, Float>
```

## Interfaces

```rust
inter Widget {
  fn view(self, ctx: Context) -> Element
}

class IconButton {
  var icon: Icon
  var text: String
}

impl Widget for IconButton {
  fn view(self, ctx: Context) -> Element {
    Button(
      padding: 6px,
      Row(
        gap: 6px,
        [self.icon, self.text]
      )
    )
  }
}
```

```rust
type new UserId = Int

class NewUser {
  var name: String
  var age: Int
}

class User : NewUser {
  var id: UserId
}

inter UserDb {
  fn by_id(self, id: UserId) -> User?
  fn by_name(self, name: String) -> User?
  fn create(self, user: NewUser) -> User
  fn update(self, user: User) -> User
}

type new PostId = Int

class NewPost {
  var title: String
  var thumbnail_url: String
  var content: String
}

class Post : NewPost {
  var id: PostId
}

inter PostDb {
  fn by_id(self, id: PostId) -> Post?
  fn by_author(self, id: UserId) -> [Post]
  fn create(self, post: NewPost) -> Post
  fn update(self, post: Post) -> Post
}

inter Db {
  fn users(self) -> UserDb
  fn posts(self) -> PostDb
}

class UserWithPosts {
  var user: User
  var posts: [Post]
}

fn get_user_by_id_with_posts(db: Db, id: UserId) -> UserWithPosts? {
  UserWithPosts(
    user: db.users().by_id(id)
    posts: db.posts().by_author(id)
  )
}
```

## Code examples

```rust
import bot.Context
import rng
import parsing

class UserData {
  var fish: [String]
  var coins: Int
  var worms: Int
}

class Fish {
  var id: String
  var description: String
  var emoji: String
  var price: Int
}

var FISH = [
  Fish(id: "fish",             description: "a fish",             emoji: "🐟", price: 10  ),
  Fish(id: "tropical_fish",    description: "a tropical fish",    emoji: "🐠", price: 30  ),
  Fish(id: "blowfish",         description: "a blowfish",         emoji: "🐡", price: 25  ),
  Fish(id: "shark",            description: "a shark",            emoji: "🦈", price: 50  ),
  Fish(id: "shrimp",           description: "a shrimp",           emoji: "🦐", price: 5   ),
  Fish(id: "shoe",             description: "a shoe",             emoji: "👞", price: 1   ),
  Fish(id: "cd",               description: "a broken CD",        emoji: "💿", price: 2   ),
  Fish(id: "four_leaf_clover", description: "a four leaf clover", emoji: "🍀", price: 100 ),
]
var FISH_BY_ID = Map.from_pairs(FISH.map(fn(fish) {(fish.id, fish)}))
var FISH_BY_EMOJI = Map.from_pars(FISH.map(fn(fish) {(fish.emoji, fish)}))
var FISH_IDS = FISH.map(fn(fish) {fish.id})
var FAILURE_MESSAGES = [
  "No luck!",
  "Your hook got snagged on a tree branch and your fishing line broke!",
  "You caught some worthless scrap.",
]

fn fish(ctx: Context) -> String {
  ctx.parse_args(none)

  var ud: UserData = ctx.kv.get(ctx.user.id)
  if rng.bool(15pct) {
    var fish = FISH_BY_ID[rng.choice(FISH_IDS)]
    f"You fished up {fish.description}! +1 {fish.emoji}"
  } else {
    rng.choice(FAILURE_MESSAGES)
  }
}

fn pockets(ctx: Context) -> String {
  ctx.parse_args(none)

  var ud: UserData = ctx.kv.get(ctx.user.id)
  if ud.fish.is_empty() {
    "Your pockets are empty."
  } else {
    ud.fish.join(", ")
  }
}

fn buy(ctx: Context) -> String {
  var args = ctx.parse_args("<#count> <what>")
  var count = parsing.int(args[0])
  var what = args[1]

  var ud: UserData = ctx.kv.get(ctx.user.id)
  var response = "I don't recognize that item. Available items are: worms"
  if what == "worms" {
    var cost = count * 4
    if ud.coins < cost {
      var missing = cost - ud.coins
      f"You don't have enough coins! You need {missing} more."
    } else {
      ud.coins -= cost
      ud.worms += count
      f"You bought {count} worms for {cost} coins."
    }
  }

  ctx.kv.set(ctx.user.id, ud)

  response
}

fn sell(ctx: Context) -> String {
  var args = parse_args(ctx.args(), "<#count> <what>")
  var count = parsing.int(args[0])
  var what = args[1]

  var ud: UserData = ctx.kv.get(ctx.user.id)
  var fish = FISH_BY_EMOJI.get(what)
  if !fish { return "I don't recognize that item." }
  var id = fish.id

  var available = 0
  for fish in ud.fish {
    if fish.id == id {
      available += 1
    }
  }

  if available < count {
    return f"You only have {available} of those items."
  }

  var response = f"You sold {count} {fish.emoji} for {count * fish.price} coins."

  ud.coins += count * fish.price
  for i in 0..ud.fish.len() {
    if count == 0 { break }
    var index = ud.fish.len() - 1 - i
    if ud.fish[index].id == id {
      ud.fish.swap_remove(index)
    }
    count -= 1
  }

  ctx.kv.set(ctx.user.id, ud)

  response
}

register({
  "__default": fish,
  "pockets": pockets,
  "buy": buy,
  "sell": sell,
})
```

```rust
import {parsing, io.input}

class TicTacToe {
  board = [
    "-","-","-",
    "-","-","-",
    "-","-","-"
  ]
  current_player = "X"

  fn display_board(self) {
    for i in 0..3 {
      let line = ""
      let map = ""
      for j in 0..3 {
        let row = i * 3
        line += self.board[row + j]
        map += str(row + j + 1)
        if j < 2 {
          line += " | "
          map += "|"
        }
      }
      print(f"{line}      {map}")
    }
  }

  fn read_move(self) -> int {
    print(f"> Playing as {self.current_player}")

    loop {
      let pos = input("> Enter the position number (1-9): ")
      let pos: int? = parsing.int(pos)

      if !pos || pos < 1 || pos > 9 {
        print("Invalid position, try again.")
        continue
      }

      if self.board[pos - 1] != "-" {
        print(f"Position {pos} already taken, try again.")
        continue
      }

      return pos
    }
  }

  fn swap_player(self) {
    self.current_player =
      if self.current_player == "X" {
        "O"
      } else {
        "X"
      }
  }

  fn check_board(self) -> str? {
    if self.check_rows_and_cols() || self.check_diagonals() {
      return self.current_player
    }

    if self.board.contains("-") {
      return nil
    }

    return "draw"
  }

  fn check_rows_and_cols(self) -> bool {
    for i in 0..3 {
      let same_in_row_or_col = self.check_row_or_col(i)
      if same_in_row_or_col {
        return true
      }
    }

    return false
  }

  fn check_row_or_col(self, i: int) -> bool {
    let same_in_row = true
    let same_in_col = true
    for j in 1..3 {
      let row = i * 3
      let col = j * 3
      let prev_col = (j - 1) * 3

      same_in_row = same_in_row && (
        self.board[row + j] == self.board[row + j - 1]
        && self.board[row + j] != "-"
      )
      same_in_col = same_in_col && (
        self.board[col + i] == self.board[prev_col + i]
        && self.board[col + i] != "-"
      )
    }
    return same_in_row || same_in_col
  }

  fn check_diagonals(self) -> bool {
    let left_diag_player = self.board[0]
    let right_diag_plaer = self.board[2]
    let left_diag =
         left_diag_player != "-"
      && left_diag_player == self.board[4]
      && left_diag_player == self.board[8]
    let right_diag =
         right_diag_player != "-"
      && right_diag_player == self.board[4]
      && right_diag_player == self.board[6]
    return left_diag || right_diag
  }

  fn play(self) {
    loop {
      self.display_board()
      let pos = self.read_move()
      self.board[pos - 1] = self.current_player

      let result = self.check_board()
      self.display_board()
      if result == "draw" {
        print("It's a draw!")
        return
      } else if result == self.current_player {
        print(f"{self.current_player} wins!")
        return
      }

      self.swap_player()
    }
  }
}

TicTacToe().play()
```

## Grammar

```ebnf
stmt =
  | var
  | loop
  | while
  | for
  | fn
  | class
  | inter
  | impl
  | type
  (* all of these are exprs: *)
  | return
  | yield
  | break
  | continue
  | assign
  | block
  | if
  | expr

expr =
  | binary (* +, -, *, /, %, &&, ?? *)
  | unary (* +, -, !, ? *)
  | literal
  | get_var
  | set_var
  | get_field
  | set_field
  | get_index
  | set_index
  | call
  | self
  | super
```
