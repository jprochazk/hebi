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
