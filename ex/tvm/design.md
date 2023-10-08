
## Variables

```rust
let v: int = 0;

// type inference
let v = 0;

// bi-directional type inference
let v = []; // type of `v` is `[_]`, `_` meaning "empty"
v.push(10);  // type of `v` is `[int]`
```

## Literals

```rust
let v: _; // the empty type
let v: int = 1;
let v: num = 1.0;
let v: bool = true;
let v: str = "yo";
let v: [int] = [1, 2, 3];
// TODO: set type?
let v: {str} = {"a", "b", "c"};
let v: {str -> int} = {"a": 0, "b": 1};

// anu type can be made optional with a `?`
// `?` types support a very limited form of subtyping:
// - `T` can be assigned to both `T` and `T?`
// - `T?` can be assigned to only `T?`
let opt: int? = 1;
let opt: int? = none;
let v: int = opt; // type error
let v: int? = opt; // ok
```

## Operators

```rust
2 + 2;
2 - 2;
2 / 2;
2 * 2;
2 % 2;
2 ** 2;
2 == 2;
2 != 2;
2 > 2;
2 >= 2;
2 < 2;
2 <= 2;
-2;
!true;
not true;
true && true;
true and true;
false || true;
false or true;
a ?? b;
```

```rust
name = 1;
name += 1;
name -= 1;
name /= 1;
name *= 1;
name %= 1;
name **= 1;

// assigns to `name` only if `name` is `none`,
// and asserts that `name` is no longer `none`
name ??= 1;
```

## Control flow

```rust
// block stmt
{
  let v = 0;
}

// block expr
let v = do { 1 };

// if stmt
if true {}
else if true {}
else {}

// if expr
// evaluates to:
// - `0` if `true`
// - `none` if `false`
let v: int? = if true {0};
let v: int = if true {0} else {1};

// bare loop
loop {}
```

```rust
return value;
yield value;
break;
continue;

// all of the above are expr with type `!`
let v = return none;
let v = break;
let v = continue;
// `yield` produces whatever `resume` is called with
let v = yield none;
```

## Functions

```rust
// function declarations:
fn name() {}
fn name(a: A) {}
fn name(a: A, b: B, c: C) {}
fn name() -> R {}
fn name(a: A) -> R {}
fn name(a: A, b: B, c: C) -> R {}
```

## Function calls

```rust
fn adder(a: int) -> (int) -> int {
  fn inner(b: int) { a + b }
  inner
}

print(adder(10)(50)); // 60
```

## Composite types

```rust
// Records
record Foo {
  a: int,
  b: str,
}

// record constructor, field access
let v = Foo(a: 100, b: "test");
print(v.b);

// Unions
union Bar {
  Lorem
  Ipsum { a: int, b: str }
  Dolor
}

// union constructor, match
let v = Bar.Lorem
let v = Bar.Ipsum(a: 100, b: "test")

match v {
  Bar.Lorem -> print("A")
  Bar.Ipsum -> print("B")
  _ -> print("?")
}

// unions enable another limited form of subtyping
// each variant is a type, and the union is their supertype
fn test(v: Bar) {/*...*/} // accepts any variant of `Bar`
fn test(v: Bar.Lorem) {/*...*/} // accepts only `Lorem`
```

## UFCS

```rust
// given a record like
record User {
  name: str,
  age: int,
}

// and a function like
fn greet(user: User) {
  print("Hi, {user.name}!");
}

let v = User(name: "😂", age: 100);

// both of these are equivalent function calls:
v.greet();
greet(v);

// the above also applies for unions
union Animal {
  Cat
  Dog
}

fn sound(a: Animal) -> str {
  match a {
    Bar.Cat -> "meow",
    Bar.Dog -> "woof",
  }
}

let v = Animal.Cat;
print(v.sound());
print(sound(v));
```

## Code examples

### Tic Tac Toe

```rust
union Player { X, O }

union Cell {
  Empty,
  With { player: Player }
}


record TicTacToe {
  board: [Cell] = [Cell.Empty; 9],
  player: Player = Player.X,
}

fn to_str(v: Cell) -> str {
  match v {
    Cell.Empty -> "-"
    Cell.Player(player) -> player.to_str()
  }
}

fn to_str(v: Player) -> str {
  match v {
    Player.X -> "X"
    Player.O -> "O"
  }
}

fn to_str(game: TicTacToe) -> str {
  let out = [];
  for i in 0..3 {
    let line = "";
    let map = "";
    for j in 0..3 {
      let row = i * 3;
      line += self.board[row + j].to_str();
      map += str(row + j + 1);
      if j < 2 {
        line += " | ";
        map += "|";
      }
    }
    out.push("{line}      {map}");
  }
  out.join("\n");
}

fn move(game: TicTacToe) -> int {
  print("> Playing as {self.current_player}");

  loop {
    let pos: int =
      input("> Enter the position number (1-9): ")
        .parse_int();
    if pos < 1 || pos > 9 {
      print("Invalid position, please try again.");
      continue
    }

    if self.board[pos - 1] != Cell.Empty {
      print("Position {pos} already taken, please try again.");
      continue
    }

    return pos
  }
}

fn swap(game: TicTacToe) {
  game.player = match game.player {
    Player.X -> Player.O
    Player.O -> Player.X
  }
}

fn win(game: TicTacToe) -> str? {
  let scan = [
    [0, 1, 2], [3, 4, 5], [6, 7, 8], // rows
    [0, 3, 6], [1, 4, 7], [2, 5, 8], // cols
    [0, 4, 8], [2, 4, 6],             // diag
  ];

  for line in scan {
    let p0 = game.board[line[0]];
    let p1 = game.board[line[1]];
    let p2 = game.board[line[2]];
    match p0 {
      Cell.Empty -> {
        continue // not a full line
      }
      Cell.With { player } -> {
        if p0 == p1 && p1 == p2 {
          return "{player.to_str()} wins!"
        }
      }
    }
  }

  if Cell.Empty in game.board {
    return none // game is not over yet
  }

  return "It's a draw!"
}

fn main() {
  let game = TicTacToe();
  print(game.to_str());

  loop {
    let pos = game.move();
    game.board[pos - 1] = game.player;

    let result = game.win();
    if !result {
      game.swap();
      continue
    } else {
      print(result);
      break
    }
  }
}

main();
```

