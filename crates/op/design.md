Mu uses a register-based bytecode.

The destination of an opcode is always the accumulator.


``` rust
struct BytecodeArray {
  inner: Vec<u8>,
}

impl BytecodeArray {
  fn fetch(&self, pc: usize) -> u8 {
    self.inner[pc]
  }
  fn get_args<const N: u8>(&self, pc: usize, width: u8) -> [u32; N] {
    let mut args = [0u32; N];
    for i in 0..N {
      args[i] = self.inner[1 + pc + i * width];
    }
    args
  }
  fn get_buffer_mut(&mut self) -> &mut Vec<u8> {
    &mut self.inner
  }
}



define_bytecode! {
  /// Load a constant into the accumulator.
  load_const <slot>,
  /// Load a register into the accumulator.
  load_reg <reg>,
  /// Store a register in the accumulator.
  store_reg <reg>,
}

// `define_bytecode` above expands to:

mod op {
  pub const __nop: u8 = 0x00;
  pub const __wide: u8 = 0x01;
  pub const __xwide: u8 = 0x02;
  pub const load_const: u8 = 0x03;
  pub const load_reg: u8 = 0x04;
  pub const store_reg: u8 = 0x05;
  pub const __halt: u8 = 0xFF;
  pub const __num_opcodes: usize = 0x07;
}
pub trait Handler {
  type Error;
  /// Load a constant into the accumulator.
  fn op_load_const(&mut self, slot: u32) -> Result<(), Error>;
  /// Load a register into the accumulator.
  fn op_load_reg(&mut self, n: u32) -> Result<(), Error>;
  /// Store a register in the accumulator.
  fn op_store_reg(&mut self, n: u32) -> Result<(), Error>;
}

macro_rules! define_handlers {
  () => {};
  (
    $op:ident (
      $bc:ident,
      $pc:ident,
      $opcode:ident,
      $operand_size:ident,
      $result:ident,
      next_operand_size : $next_operand_size:expr,
      args : $num_args:expr$(,)?
    )
    $action:block
    ;
    $($tail:tt)*
  ) => {
    paste! {
      fn [<op_ $op>]<H: Handler>(
        vm: &mut H,
        $bc: &mut BytecodeArray,
        $pc: &mut usize,
        $opcode: &mut u8,
        $operand_size: &mut u8,
        $result: &mut Result<()>
      ) {
        {
          $body
        }

        *$pc += 1 + $num_args * (*$operand_size);
        *$operand_size = $next_operand_size;
        *$opcode = $bc.fetch(*$pc);
      }
    }
    define_handlers!($($tail)*);
  };

  (
    $op:ident (
      $bc:ident,
      $pc:ident,
      $opcode:ident,
      $operand_size:ident,
      $result:ident,
      $args:ident : $num_args:expr$(,)?
    )
    $action:block
    ;
    $($tail:tt)*
  ) => {
    paste! {
      fn [<op_ $op>]<H: Handler>(
        vm: &mut H,
        $bc: &mut BytecodeArray,
        $pc: &mut usize,
        $opcode: &mut u8,
        $operand_size: &mut u8,
        $result: &mut Result<()>
      ) {
        let $args = $bc.get_args::<$num_args>(*$operand_size);
        {
          $body
        }

        *$pc += 1 + $num_args * (*$operand_size);
        *$operand_size = 1;
        *$opcode = $bc.fetch(*$pc);
      }
    }
    define_handlers!($($tail)*);
  }
}

handler! {
  nop(bc, pc, opcode, operand_size, result, args:0) {}
}

fn nop<T: Handler>(vm: &mut T, bc: &mut BytecodeArray, pc: &mut usize, opcode: &mut u8, operand_size: &mut u8, result: &mut Result<()>) {
  *operand_size = 1;
  *pc += 1;
  *opcode = bc.fetch(pc);
}

#[inline(never)]
pub fn run<T: Handler>(vm: &mut T, bc: &mut BytecodeArray) -> Result<()> {
  let mut pc = 0;
  let mut opcode = bc.fetch(pc);
  let mut operand_size = 1u8;
  let mut result = Ok(());
  loop {
    match opcode {
      op::__nop => 
    }
  }
  result

  let mut jump_table = [label_addr!("__goto_op_nop"); op::__num_opcodes];

  jump_table[op::__nop] = label_addr!("__goto_op_nop");
  jump_table[op::__wide] = label_addr!("__goto_op_wide");
  jump_table[op::__xwide] = label_addr!("__goto_op_xwide");
  jump_table[op::load_const] = label_addr!("__goto_op_load_const");
  jump_table[op::load_reg] = label_addr!("__goto_op_load_reg");
  jump_table[op::store_reg] = label_addr!("__goto_op_store_reg");
  jump_table[op::__halt] = label_addr!("__goto_op_halt");

  let mut pc = 0;
  let mut opcode = bc.fetch(pc);
  let mut operand_size = 1u8;
  let mut result = Ok(());
  dispatch!(vm, jump_table, pc, opcode);

  handler!("__goto_op_nop", vm, jump_table, pc, opcode, {
    // action

    // setup dispatch
    operand_size = 1;
    pc += 1;
    opcode = bc.fetch(pc);
  });

  handler!("__goto_op_wide", vm, jump_table, pc, opcode, {
    // action

    // before dispatch
    operand_size = 2;
    pc += 1;
    opcode = bc.fetch(pc);
  });

  handler!("__goto_op_xwide", vm, jump_table, pc, opcode, {
    // action

    // before dispatch
    operand_size = 4;
    pc += 1;
    opcode = bc.fetch(pc);
  });

  handler!("__goto_op_load_const", vm, jump_table, pc, opcode, {
    // action
    let [__arg_slot] = bc.get_args::<1>(operand_size);
    result = vm.op_load_const(__arg_slot);
    if result.is_some() {
      goto!("__goto_op_halt");
    }

    // before dispatch
    operand_size = 1;
    pc += 1 + operand_size * 1;
    opcode = bc.fetch(pc);
  });

  handler!("__goto_op_load_reg", vm, jump_table, pc, opcode, operand_size, {
    // action
    let [__arg_reg] = bc.get_args::<1>(operand_size);
    result = vm.op_load_reg(__arg_reg);
    if result.is_some() {
      goto!("__goto_op_halt");
    }

    // before dispatch
    operand_size = 1;
    pc += 1 + operand_size * 1;
    opcode = bc.fetch(pc);
  });

  handler!("__goto_op_store_reg", vm, jump_table, pc, opcode, operand_size, {
    // action
    let [__arg_reg] = bc.get_args::<1>(operand_size);
    result = vm.op_store_reg(__arg_reg);
    if result.is_some() {
      goto!("__goto_op_halt");
    }

    // before dispatch
    operand_size = 1;
    pc += 1 + operand_size * 1;
    opcode = bc.fetch(pc);
  });

  handle!(vm, jump_table, pc, opcode, )

  label!("__goto_op_halt");
  result
}

```

```

self.v = 10

  load_small_int  dest=reg.1, value=10
  store_acc       src=reg.0
  store_field     src=reg.1, key=const.0

a = None
const = [0: "v"]
reg.0 = {} (#0)
reg.1 = None

> load_small_int dest=reg.1, value=10

a = None
const = [0: "v"]
reg.0 = {} (#0)
reg.1 = 10 (int)

> store_a src=reg.0

a = {} (#0)
const = [0: "v"]
reg.0 = {} (#0)
reg.1 = 10 (int)

> store_field_a src=reg.1, key=const.0

a = {"v":10} (#0)
const = [0: "v"]
reg.0 = {"v":10} (#0)
reg.1 = 10 (int)

```
