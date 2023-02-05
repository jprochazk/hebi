use std::ptr::NonNull;

use crate::instruction::*;

macro_rules! check {
  ($chunk:ident) => {
    insta::assert_snapshot!($chunk.disassemble(false));
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

  let mut b = Builder::<Value>::new("test");

  let [start, end] = b.labels(["start", "end"]);

  b.constant(Value::String("test".into()));
  b.constant(Value::Number(123_456_789));
  b.constant(Value::Bool(true));

  b.finish_label(start);
  b.op(Nop {});
  b.op(LoadConst { slot: 0u32 });
  b.op(LoadConst {
    slot: u8::MAX as u32 + 1,
  });
  b.op(LoadConst {
    slot: u16::MAX as u32 + 1,
  });
  b.op(LoadReg { reg: 0 });
  b.op(LoadReg {
    reg: u8::MAX as u32 + 1,
  });
  b.op(LoadReg {
    reg: u16::MAX as u32 + 1,
  });
  b.op(StoreReg { reg: 0 });
  b.op(StoreReg {
    reg: u8::MAX as u32 + 1,
  });
  b.op(StoreReg {
    reg: u16::MAX as u32 + 1,
  });
  b.op(Jump { offset: start.id() });
  b.op(Jump { offset: end.id() });
  b.op(JumpIfFalse { offset: end.id() });
  b.op(PushSmallInt { value: 0 });
  b.op(PushSmallInt { value: i32::MAX });
  b.op(PushSmallInt { value: i32::MIN });
  b.finish_label(end);
  b.op(Ret {});
  b.op(Suspend {});

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
    pc: usize,
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

    fn op_jump(&mut self, offset: u32) -> Result<ControlFlow, Self::Error> {
      Ok(ControlFlow::Jump(offset))
    }

    fn op_jump_back(&mut self, offset: u32) -> Result<ControlFlow, Self::Error> {
      Ok(ControlFlow::Loop(offset))
    }

    fn op_jump_if_false(&mut self, offset: u32) -> Result<ControlFlow, Self::Error> {
      let value = *self.a.as_number().ok_or(())?;
      Ok(if value == 0 {
        ControlFlow::Jump(offset)
      } else {
        ControlFlow::Next
      })
    }

    fn op_print(&mut self) -> Result<(), Self::Error> {
      use std::io::Write;
      writeln!(&mut self.stdout, "{}", self.a).map_err(|_| ())?;
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

    fn op_create_empty_list(&mut self) -> Result<(), Self::Error> {
      self.a = Value::List(vec![]);
      Ok(())
    }

    fn op_push_to_list(
      &mut self,
      list: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      let list = self.r[list as usize].as_list_mut().ok_or(())?;
      list.push(self.a.clone());
      Ok(())
    }

    fn op_ret(&mut self) -> Result<(), Self::Error> {
      self.pc += 1;

      Ok(())
    }

    fn op_load_capture(
      &mut self,
      slot: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_store_capture(
      &mut self,
      slot: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_global(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_store_global(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_named(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_named_opt(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_store_named(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
      obj: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_keyed(&mut self, key: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_keyed_opt(
      &mut self,
      key: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_store_keyed(
      &mut self,
      key: <ty::Reg as ty::Operand>::Decoded,
      obj: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_module(
      &mut self,
      path: <ty::Const as ty::Operand>::Decoded,
      dest: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_self(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_super(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_push_none(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_push_true(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_push_false(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_create_empty_dict(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_insert_to_dict(
      &mut self,
      key: <ty::Reg as ty::Operand>::Decoded,
      dict: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_insert_to_dict_named(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
      dict: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_create_closure(
      &mut self,
      desc: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_capture_reg(
      &mut self,
      reg: <ty::Reg as ty::Operand>::Decoded,
      slot: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_capture_slot(
      &mut self,
      parent_slot: <ty::uv as ty::Operand>::Decoded,
      self_slot: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_add(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_mul(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_div(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_rem(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_pow(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_unary_plus(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_unary_minus(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_unary_not(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_eq(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_neq(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_gt(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_ge(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_lt(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_cmp_le(&mut self, lhs: <ty::Reg as ty::Operand>::Decoded) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_is_none(&mut self) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_print_list(
      &mut self,
      list: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_call0(&mut self, return_address: usize) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_call(
      &mut self,
      start: <ty::Reg as ty::Operand>::Decoded,
      args: <ty::uv as ty::Operand>::Decoded,
      return_address: usize,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_call_kw(
      &mut self,
      start: <ty::Reg as ty::Operand>::Decoded,
      args: <ty::uv as ty::Operand>::Decoded,
      return_address: usize,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_is_pos_param_not_set(
      &mut self,
      index: <ty::uv as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_is_kw_param_not_set(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_load_kw_param(
      &mut self,
      name: <ty::Const as ty::Operand>::Decoded,
      param: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_create_class_empty(
      &mut self,
      desc: <ty::Const as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }

    fn op_create_class(
      &mut self,
      desc: <ty::Const as ty::Operand>::Decoded,
      start: <ty::Reg as ty::Operand>::Decoded,
    ) -> Result<(), Self::Error> {
      todo!()
    }
  }

  let mut b = Builder::<Value>::new("test");

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
  let [r0] = [0];

  //   push_small_int    value=10       //
  //   store_reg         reg=r0         // v := 10
  // @loop:                             // loop:
  //   load_reg          reg=r0         //
  //   jump_if_false     @break         //   if (i == 0): break
  //   push_small_int    value=123      //
  //   print             reg=r1         //   print 123
  //   push_small_int    value=1        //
  //   sub               lhs=r0         //
  //   store_reg         reg=r0         //   v -= 1
  //   jump              @loop          //
  // @break:                            //
  //   ret                              //
  //   suspend                          //

  b.op(PushSmallInt { value: 10 });
  b.op(StoreReg { reg: r0 });
  b.finish_label(l_loop);
  b.op(LoadReg { reg: r0 });
  b.op(JumpIfFalse {
    offset: l_break.id(),
  });
  b.op(PushSmallInt { value: 123 });
  b.op(Print);
  b.op(PushSmallInt { value: 1 });
  b.op(Sub { lhs: r0 });
  b.op(StoreReg { reg: r0 });
  b.op(Jump {
    offset: l_loop.id(),
  });
  b.finish_label(l_break);
  b.op(Ret);

  let chunk = b.build();
  check!(chunk);

  let Chunk {
    mut bytecode,
    const_pool,
    ..
  } = chunk;
  let mut vm = VM {
    pc: 0,
    stdout: Vec::new(),
    a: Value::Number(0),
    r: vec![Value::Number(0); 2],
    c: const_pool,
  };

  let bc = NonNull::from(&mut bytecode[..]);
  let pc = NonNull::from(&mut vm.pc);
  unsafe { run(&mut vm, bc, pc) }.unwrap();

  let stdout = String::from_utf8(vm.stdout).unwrap();
  insta::assert_snapshot!(stdout);
}

/* #[test]
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

    fn op_jump(&mut self, _: u32) -> Result<ControlFlow, Self::Error> {
      Err("test")
    }

    fn op_jump_if_false(&mut self, _: u32) -> Result<ControlFlow, Self::Error> {
      Err("test")
    }

    fn op_sub(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_print(&mut self) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_push_small_int(&mut self, _: i32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_create_empty_list(&mut self) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_push_to_list(&mut self, _: u32) -> Result<(), Self::Error> {
      Err("test")
    }

    fn op_ret(&mut self) -> Result<(), Self::Error> {
      Err("test")
    }
  }

  let mut b = Builder::<()>::new("test");
  b.op(Ret);
  let Chunk { mut bytecode, .. } = b.build();
  let Err(e) = run(&mut VM, &mut bytecode, &mut 0) else {
    panic!("VM did not return error");
  };

  assert_eq!(e, "test");
} */
