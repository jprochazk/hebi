use crate::prelude::*;

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
  b.op_push_small_int(0);
  b.op_push_small_int(i32::MAX);
  b.op_push_small_int(i32::MIN);
  b.op_ret();
  b.finish_label(end);
  b.op_suspend();

  let chunk = b.build();
  check!(chunk);
}

#[test]
fn dispatch() {
  #[derive(Clone, Hash, PartialEq, Eq, Debug)]
  enum Value {
    Number(i32),
    List(Vec<Value>),
  }

  impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Value::Number(v) => write!(f, "{v}"),
        Value::List(v) => f.debug_list().entries(v).finish(),
      }
    }
  }

  impl Value {
    fn as_number(&self) -> Option<&i32> {
      if let Self::Number(v) = self {
        Some(v)
      } else {
        None
      }
    }

    fn as_list(&self) -> Option<&Vec<Value>> {
      if let Self::List(v) = self {
        Some(v)
      } else {
        None
      }
    }

    fn as_list_mut(&mut self) -> Option<&mut Vec<Value>> {
      if let Self::List(v) = self {
        Some(v)
      } else {
        None
      }
    }
  }

  struct VM {
    stdout: Vec<u8>,
    a: Value,
    r: Vec<Value>,
    c: Vec<Value>,
  }

  impl Handler for VM {
    type Error = ();
    fn op_load_const(&mut self, slot: u32) -> Result<(), Self::Error> {
      self.a = self.c[slot as usize].clone();
      Ok(())
    }

    fn op_load_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
      self.a = self.r[reg as usize].clone();
      Ok(())
    }

    fn op_store_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
      self.r[reg as usize] = self.a.clone();
      Ok(())
    }

    fn op_jump(&mut self, offset: u32) -> Result<Jump, Self::Error> {
      Ok(Jump::Goto { offset })
    }

    fn op_jump_if_false(&mut self, offset: u32) -> Result<Jump, Self::Error> {
      let value = *self.a.as_number().ok_or(())?;
      Ok(if value > 0 {
        Jump::Skip
      } else {
        Jump::Goto { offset }
      })
    }

    fn op_print(&mut self, reg: u32) -> Result<(), Self::Error> {
      use std::io::Write;

      let mut list = self.r[reg as usize].as_list().ok_or(())?.iter().peekable();

      while let Some(v) = list.next() {
        write!(&mut self.stdout, "{v}").map_err(|_| ())?;
        if list.peek().is_some() {
          write!(&mut self.stdout, " ").map_err(|_| ())?;
        }
      }
      writeln!(&mut self.stdout).map_err(|_| ())?;
      Ok(())
    }

    fn op_sub(&mut self, lhs: u32) -> Result<(), Self::Error> {
      let lhs = *self.r[lhs as usize].as_number().ok_or(())?;
      let rhs = *self.a.as_number().ok_or(())?;
      self.a = Value::Number(lhs - rhs);
      Ok(())
    }

    fn op_push_small_int(&mut self, value: i32) -> Result<(), Self::Error> {
      self.a = Value::Number(value);
      Ok(())
    }

    fn op_create_empty_list(&mut self, _: ()) -> Result<(), Self::Error> {
      self.a = Value::List(vec![]);
      Ok(())
    }

    fn op_list_push(&mut self, list: u32) -> Result<(), Self::Error> {
      let list = self.r[list as usize].as_list_mut().ok_or(())?;
      list.push(self.a.clone());
      Ok(())
    }

    fn op_ret(&mut self, _: ()) -> Result<(), Self::Error> {
      Ok(())
    }
  }

  let mut b = BytecodeBuilder::<Value>::new("test");

  // v := 10
  // loop:
  //   if v == 0: break
  //   print 123
  //   v -= 1

  //
  // c0 = 123 (v)
  // c1 = 10  (start)
  // r0 = printed value (v)
  // r1 = loop index (i)
  //
  let [l_loop, l_break] = b.labels(["loop", "break"]);
  let [r0, r1] = [0, 1];

  //   push_small_int    value=10       //
  //   store_reg         reg=r0         // v := 10
  // @loop:                             // loop:
  //   load_reg          reg=r0         //
  //   jump_if_false     @break         //   if (i == 0): break
  //   create_empty_list                //
  //   store_reg         reg=1          //
  //   push_small_int    value=123      //
  //   list_push         list=r1        //
  //   print             reg=r1         //   print 123
  //   push_small_int    value=1        //
  //   sub               lhs=r0         //
  //   store_reg         reg=r0         //   v -= 1
  //   jump              @loop          //
  // @break:                            //
  //   ret                              //
  //   suspend                          //

  b.op_push_small_int(10);
  b.op_store_reg(r0);
  b.finish_label(l_loop);
  b.op_load_reg(r0);
  b.op_jump_if_false(l_break);
  b.op_create_empty_list();
  b.op_store_reg(r1);
  b.op_push_small_int(123);
  b.op_list_push(r1);
  b.op_print(r1);
  b.op_push_small_int(1);
  b.op_sub(r0);
  b.op_store_reg(r0);
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
    a: Value::Number(0),
    r: vec![Value::Number(0); 2],
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

    fn op_sub(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_print(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_push_small_int(&mut self, _: i32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_create_empty_list(&mut self, _: ()) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_list_push(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_ret(&mut self, _: ()) -> Result<(), Self::Error> {
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
