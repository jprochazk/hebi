use super::*;

macro_rules! check {
  ($chunk:ident) => {
    insta::assert_snapshot!($chunk.disassemble());
  };
}

#[test]
fn test_builder() {
  #[derive(Clone, Hash, PartialEq, Eq)]
  enum Value {
    String(String),
    Number(u64),
    Bool(bool),
  }

  impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Value::String(v) => write!(f, "\"{v}\""),
        Value::Number(v) => write!(f, "{v}"),
        Value::Bool(v) => write!(f, "{v}"),
      }
    }
  }

  let mut b = BytecodeBuilder::<Value>::new("test");

  let [start, end] = b.labels(["start", "end"]);

  b.constant(Value::String("test".into()));
  b.constant(Value::Number(123_456_789));
  b.constant(Value::Bool(true));

  b.finish_label(start);
  b.op_nop();
  b.op_load_const(0u32);
  b.op_load_const(u8::MAX as u32 + 1);
  b.op_load_const(u16::MAX as u32 + 1);
  b.op_load_reg(0);
  b.op_load_reg(u8::MAX as u32 + 1);
  b.op_load_reg(u16::MAX as u32 + 1);
  b.op_store_reg(0);
  b.op_store_reg(u8::MAX as u32 + 1);
  b.op_store_reg(u16::MAX as u32 + 1);
  b.op_jump(start);
  b.op_jump(end);
  b.op_jump_if_false(start);
  b.op_jump_if_false(end);
  b.op_ret();
  b.finish_label(end);
  b.op_suspend();

  let chunk = b.build();
  check!(chunk);
}

#[test]
fn dispatch() {
  type Value = u32;

  struct VM {
    stdout: Vec<u8>,
    a: Value,
    r: Vec<Value>,
    c: Vec<Value>,
  }

  impl Handler for VM {
    type Error = ();
    fn op_load_const(&mut self, slot: u32) -> Result<(), Self::Error> {
      self.a = self.c[slot as usize];
      Ok(())
    }

    fn op_load_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
      self.a = self.r[reg as usize];
      Ok(())
    }

    fn op_store_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
      self.r[reg as usize] = self.a;
      Ok(())
    }

    fn op_jump(&mut self, offset: u32) -> Result<Jump, Self::Error> {
      Ok(Jump::Goto { offset })
    }

    fn op_jump_if_false(&mut self, offset: u32) -> Result<Jump, Self::Error> {
      Ok(if self.a > 0 {
        Jump::Skip
      } else {
        Jump::Goto { offset }
      })
    }

    fn op_print(&mut self, reg: u32) -> Result<(), Self::Error> {
      use std::io::Write;
      writeln!(&mut self.stdout, "{}", self.r[reg as usize]).map_err(|_| ())?;
      Ok(())
    }

    fn op_sub(&mut self, dest: u32, a: u32, b: u32) -> Result<(), Self::Error> {
      println!(
        "r{dest} = r{a}({}) - r{b}({})",
        self.r[a as usize], self.r[b as usize]
      );
      self.r[dest as usize] = self.r[a as usize] - self.r[b as usize];
      Ok(())
    }

    fn op_ret(&mut self) -> Result<(), Self::Error> {
      Ok(())
    }
  }

  let mut b = BytecodeBuilder::<Value>::new("test");

  // loop:
  //   if v == 0: break
  //   print(123)

  //
  // c0 = 123 (v)
  // c1 = 10  (start)
  // c2 = 1   (decrement)
  // r0 = printed value (v)
  // r1 = loop index (i)
  // r2 = 1 (dec)
  //
  let [l_loop, l_break] = b.labels(["loop", "break"]);
  let [c0, c1, c2] = [b.constant(123), b.constant(10), b.constant(1)];
  let [r0, r1, r2] = [0, 1, 2];

  //   load_const offset=0       // a = v
  //   store_reg  reg=0          // r0 = a
  //   load_const offset=1       // a = start
  //   store_reg  reg=1          // r1 = a
  //   load_const offset=2       // a = 1
  //   load_reg   reg=2          // r2 = a
  // @loop:
  //   load_reg   reg=1          // a = i
  //   jump_if_false @break      // if (i == 0) goto @break
  //   print      reg=0          // print v
  //   sub        dest=1 a=1 b=2 // r1 = r1 - r2
  //   jump       @loop          // goto @loop
  // @break:
  //   ret                       // return
  //   suspend                   // suspend

  b.op_load_const(c0);
  b.op_store_reg(r0);
  b.op_load_const(c1);
  b.op_store_reg(r1);
  b.op_load_const(c2);
  b.op_store_reg(r2);

  b.finish_label(l_loop);

  b.op_load_reg(r1);
  b.op_jump_if_false(l_break);
  b.op_print(r0);
  b.op_sub(r1, r1, r2);
  b.op_jump(l_loop);

  b.finish_label(l_break);

  b.op_ret();

  let chunk = b.build();
  check!(chunk);

  let Chunk {
    mut bytecode,
    const_pool,
    ..
  } = chunk;
  let mut vm = VM {
    stdout: Vec::new(),
    a: 0,
    r: vec![0; 3],
    c: const_pool,
  };

  let mut pc = 0;
  run(&mut vm, &mut bytecode, &mut pc).unwrap();

  let stdout = String::from_utf8(vm.stdout).unwrap();
  insta::assert_snapshot!(stdout);
}

#[test]
fn vm_error() {
  struct VM;

  impl Handler for VM {
    type Error = &'static str;

    fn op_load_const(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_load_reg(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_store_reg(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_jump(&mut self, _: u32) -> Result<Jump, Self::Error> {
      Err("test")
    }

    fn op_jump_if_false(&mut self, _: u32) -> Result<Jump, Self::Error> {
      Err("test")
    }

    fn op_sub(&mut self, _: u32, _: u32, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_print(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_ret(&mut self) -> Result<(), Self::Error> {
      Err("test")
    }
  }

  let mut b = BytecodeBuilder::<()>::new("test");
  b.op_ret();
  let Chunk { mut bytecode, .. } = b.build();
  let Err(e) = run(&mut VM, &mut bytecode, &mut 0) else {
    panic!("VM did not return error");
  };

  assert_eq!(e, "test");
}
