use hebi::prelude::*;

fn main() {
  let mut hebi = Hebi::new();
  let chunk = hebi.compile("1 + 1").unwrap();
  println!("{}", chunk.disassemble());
  println!("Result: {}", hebi.run(chunk).unwrap());
}
