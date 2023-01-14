use std::hash::Hash;

use beef::lean::Cow;

use crate::disassembly::disassemble;

pub type BytecodeArray = Vec<u8>;

pub struct Chunk<Value: Hash + Eq> {
  pub name: Cow<'static, str>,
  pub bytecode: BytecodeArray,
  /// Pool of constants referenced in the bytecode.
  pub const_pool: Vec<Value>,
}

impl<Value: std::fmt::Display + Hash + Eq> Chunk<Value> {
  pub fn disassemble(&self) -> String {
    use std::fmt::Write;

    let mut f = String::new();

    {
      let f = &mut f;

      // name
      writeln!(f, "function <{}>:", self.name).unwrap();
      writeln!(f, "length = {}", self.bytecode.len()).unwrap();

      // constants
      if self.const_pool.is_empty() {
        writeln!(f, "const pool = <empty>").unwrap();
      } else {
        writeln!(f, "const pool = (length={}) {{", self.const_pool.len()).unwrap();
        for (i, value) in self.const_pool.iter().enumerate() {
          writeln!(f, "  {i} = {value}").unwrap();
        }
        writeln!(f, "}}").unwrap();
      }

      // bytecode
      writeln!(f, "bytecode:").unwrap();
      let offset_align = self.bytecode.len().to_string().len();
      let mut pc = 0;
      while pc < self.bytecode.len() {
        let instr = disassemble(&self.bytecode[..], pc);
        let size = instr.size();

        let bytes = {
          let mut out = String::new();
          // print bytes
          for byte in self.bytecode[pc..pc + size].iter() {
            write!(&mut out, "{byte:02x} ").unwrap();
          }
          if size < 6 {
            for _ in 0..(6 - size) {
              write!(&mut out, "   ").unwrap();
            }
          }
          out
        };

        writeln!(f, " {pc:offset_align$} | {bytes}{instr}").unwrap();

        pc += size;
      }
    }

    f
  }
}
