use std::collections::HashSet;
use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Token};

/// Opcodes have the following format:
///
/// ```text
/// $name $(:$flag)* $(<$operand>)*
/// ```
///
/// - `name` is the name of the opcode, sometimes prefixed by `op_`
/// - each `flag` represents some modifier on the opcode
///   - for example, jump instructions have the `:jump` flag, which among other
///     things causes the jump to be delegated to the user VM implementation
/// - each `operand` represents one slot in the bytecode which is passed to the
///   user VM implementation when the opcode is dispatched.
///
/// For example:
///
/// ```text
/// jump_if_false :jump <offset>
/// ```
///
/// Defines an instruction called `jump_if_false` that is treated as a jump
/// instruction. The `offset` is an operand which is passed to the user VM
/// implementation, and the VM returns back the control-flow `Jump` enum, which
/// determines if the jump should be skipped, or if it should not be skipped,
/// what the final offset should be.
struct Opcode {
  meta: Vec<Attribute>,
  name: Ident,
  flags: HashSet<String>,
  operands: Vec<Ident>,
}

impl Parse for Opcode {
  fn parse(input: ParseStream) -> Result<Self> {
    let meta = Attribute::parse_outer(input)?;

    let mut flags = HashSet::new();
    let mut operands = Vec::new();

    let name = Ident::parse_any(input)?;
    while input.peek(Token![:]) {
      let _ = <Token![:]>::parse(input)?;
      let flag = Ident::parse_any(input)?.to_string();
      flags.insert(flag);
    }

    while input.peek(Token![<]) {
      let _ = <Token![<]>::parse(input)?;
      let operand = Ident::parse_any(input)?;
      let _ = <Token![>]>::parse(input)?;
      operands.push(operand);
    }

    if !input.peek(Token![,]) && !input.cursor().eof() {
      return Err(Error::new(input.span(), "unexpected token"));
    }

    Ok(Opcode {
      meta,
      name,
      flags,
      operands,
    })
  }
}

struct Input {
  opcodes: Vec<Opcode>,
}

impl Parse for Input {
  fn parse(input: ParseStream) -> Result<Self> {
    let opcodes = <Punctuated<Opcode, Token![,]>>::parse_terminated_with(input, Opcode::parse)?;
    let opcodes = opcodes.into_iter().collect::<Vec<_>>();
    Ok(Self { opcodes })
  }
}

/// Emit the `Handler` trait, which the user of the `op` crate
/// implements for their VM.
fn emit_handler_trait(ops: &[Opcode]) -> TokenStream2 {
  let methods: Vec<_> = ops
    .iter()
    .map(
      |Opcode {
         meta,
         name,
         flags,
         operands,
       }| {
        let name = quote::format_ident!("op_{name}");

        if flags.contains("jump") {
          quote! {
            #(#meta)*
            fn #name (&mut self, #(#operands : u32),*) -> Result<Jump, Self::Error>;
          }
        } else {
          quote! {
            #(#meta)*
            fn #name (&mut self, #(#operands : u32),*) -> Result<(), Self::Error>;
          }
        }
      },
    )
    .collect();

  quote! {
    pub trait Handler {
      type Error;
      #(#methods)*
    }
  }
}

/// Emit the module containing opcode constants.
fn emit_op_mod(ops: &[Opcode]) -> TokenStream2 {
  let mut consts = vec![];
  consts.push(quote! {pub const __nop: u8 = 0;});
  consts.push(quote! {pub const __wide: u8 = 1;});
  consts.push(quote! {pub const __xwide: u8 = 2;});
  consts.extend(ops.iter().enumerate().map(|(i, Opcode { name, .. })| {
    let value = TokenStream2::from_str(&format!("{}", 3 + i)).unwrap();
    quote! {pub const #name : u8 = #value;}
  }));
  consts.push(quote! {pub const __suspend: u8 = 255;});

  quote! {
    pub mod op {
      #(#consts)*
    }
  }
}

enum Width {
  _1,
  _2,
  _4,
}

impl quote::ToTokens for Width {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    tokens.extend(match self {
      Width::_1 => quote! {Width::_1},
      Width::_2 => quote! {Width::_2},
      Width::_4 => quote! {Width::_4},
    })
  }
}

/// Emit dispatch handlers, which are used in place of inline blocks in the main
/// dispatch loop match arms. A dispatch handler's job is to:
/// - Fetch operands
/// - Call the VM's opcode handler
/// - Update the program counter
///   - If the instruction is a jump, this step is also delegated to the VM
/// - Fetch the next opcode
fn emit_dispatch_handlers(ops: &[Opcode]) -> TokenStream2 {
  let emit_one = |name: &Ident,
                  operands: &[Ident],
                  skip_vm_call: bool,
                  is_jump: bool,
                  next_operand_size: Width| {
    let name = quote::format_ident!("op_{name}");
    let argc = operands.len();
    let get_args = if operands.is_empty() {
      quote! {}
    } else {
      quote! {
        let [#(#operands),*] = bc.get_args::<#argc>(*opcode, *pc, *operand_size);
      }
    };

    let action = if skip_vm_call {
      quote! {}
    } else if is_jump {
      quote! {
        let _jump = match vm. #name (#(#operands),*) {
          Ok(jump) => jump,
          Err(e) => {
            *result = Err(e);
            Jump::Skip
          },
        };
        match _jump {
          Jump::Skip => *pc += 1 + #argc * (*operand_size) as usize,
          Jump::Goto { offset } => *pc = offset as usize,
        }
      }
    } else {
      quote! {
        *result = vm. #name (#(#operands),*);
        *pc += 1 + #argc * (*operand_size) as usize;
      }
    };

    quote! {
      #[inline]
      fn #name <H: Handler> (
        vm: &mut H,
        bc: &mut BytecodeArray,
        pc: &mut usize,
        opcode: &mut u8,
        operand_size: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        #get_args
        #action
        *operand_size = #next_operand_size;
        *opcode = bc.fetch(*pc);
      }
    }
  };

  let mut handlers = vec![];
  handlers.push(emit_one(
    &Ident::new("nop", Span::call_site()),
    &[],
    true,
    false,
    Width::_1,
  ));
  handlers.push(emit_one(
    &Ident::new("wide", Span::call_site()),
    &[],
    true,
    false,
    Width::_2,
  ));
  handlers.push(emit_one(
    &Ident::new("xwide", Span::call_site()),
    &[],
    true,
    false,
    Width::_4,
  ));
  for op in ops {
    handlers.push(emit_one(
      &op.name,
      &op.operands,
      false,
      op.flags.contains("jump"),
      Width::_1,
    ));
  }

  quote! {
    #(#handlers)*
  }
}

/// Emit the main dispatch loop function
fn emit_run_fn(ops: &[Opcode]) -> TokenStream2 {
  let mut arms = vec![];
  arms.push(quote! {op::__nop => op_nop(vm, bc, pc, opcode, operand_size, result),});
  arms.push(quote! {op::__wide => op_wide(vm, bc, pc, opcode, operand_size, result),});
  arms.push(quote! {op::__xwide => op_xwide(vm, bc, pc, opcode, operand_size, result),});
  arms.extend(ops.iter().map(|Opcode { name, .. }| {
    let f = quote::format_ident!("op_{name}");
    quote! {op::#name => #f(vm, bc, pc, opcode, operand_size, result),}
  }));
  arms.push(quote! {op::__suspend => break,});

  quote! {
    #[inline(never)]
    pub fn run<H: Handler>(
      vm: &mut H,
      bc: &mut BytecodeArray,
      pc: &mut usize
    ) -> Result<(), H::Error> {
      let opcode = &mut bc.fetch(*pc);
      let operand_size = &mut Width::_1;
      let mut result = Ok(());
      while !result.is_err() {
        let result = &mut result;
        match *opcode {
          #(#arms)*
          _ => panic!("malformed bytecode: invalid opcode {}", *opcode),
        }
      }
      result
    }
  }
}

/// Emits bytecode builder methods for each opcode.
///
/// The methods accept operands which are written into the bytecode using
/// variable-width encoding.
fn emit_bytecode_builder_impl(ops: &[Opcode]) -> TokenStream2 {
  let emit_one = |fn_name: &Ident, op_name: &Ident, operands: &[Ident], is_jump: bool| {
    quote! {
      pub fn #fn_name (&mut self, #(#operands : u32),*) -> &mut Self {
        let values = [#(#operands),*];
        let max_value = values.iter().fold(0, |a,b| a.max(*b));
        let width = Self::_width_of(max_value);

        self._push_op_prefix(width, #is_jump);
        self.bytecode.inner.push(op::#op_name);
        unsafe { self._push_values(&values[..], width, #is_jump) };
        self
      }
    }
  };

  let mut methods = vec![];
  methods.push(emit_one(
    &Ident::new("op_nop", Span::call_site()),
    &Ident::new("__nop", Span::call_site()),
    &[],
    false,
  ));
  methods.push(emit_one(
    &Ident::new("op_suspend", Span::call_site()),
    &Ident::new("__suspend", Span::call_site()),
    &[],
    false,
  ));
  for op in ops {
    methods.push(emit_one(
      &quote::format_ident!("op_{}", op.name),
      &op.name,
      &op.operands,
      op.flags.contains("jump"),
    ));
  }

  quote! {
    impl<Value> BytecodeBuilder<Value> {
      #[inline]
      fn _width_of(value: u32) -> Width {
        if value < u8::MAX as u32 {
          Width::_1
        } else if value < u16::MAX as u32 {
          Width::_2
        } else {
          Width::_4
        }
      }

      #[inline]
      fn _push_op_prefix(&mut self, width: Width, is_jump: bool) {
        if is_jump {
          self.bytecode.inner.push(op::__xwide);
        } else {
          match width {
            Width::_1 => {},
            Width::_2 => self.bytecode.inner.push(op::__wide),
            Width::_4 => self.bytecode.inner.push(op::__xwide),
          }
        }
      }

      unsafe fn _push_values(&mut self, values: &[u32], width: Width, is_jump: bool) {
        if is_jump {
          for value in values {
            self.bytecode.inner.extend_from_slice(&value.to_le_bytes())
          }
        } else {
          match width {
            Width::_1 => {
              for value in values {
                let value = unsafe { u8::try_from(*value).unwrap_unchecked() };
                self.bytecode.inner.push(value);
              }
            }
            Width::_2 => {
              for value in values {
                let value = unsafe { u16::try_from(*value).unwrap_unchecked() };
                self.bytecode.inner.extend_from_slice(&value.to_le_bytes());
              }
            }
            Width::_4 => {
              for value in values {
                self.bytecode.inner.extend_from_slice(&value.to_le_bytes());
              }
            }
          }
        }
      }

      #(#methods)*
    }
  }
}

/// Emits utilities used to patch jump instructions.
fn emit_jump_patching(ops: &[Opcode]) -> TokenStream2 {
  let jump_ops = ops
    .iter()
    .filter(|op| op.flags.contains("jump"))
    .map(|op| &op.name)
    .collect::<Vec<_>>();
  quote! {
    fn is_jump_op(op: u8) -> bool {
      [#(op::#jump_ops),*].contains(&op)
    }

    fn patch_jump_op(bc: &mut [u8], op: u8, pc: usize, offset: u32) {
      if offset < u8::MAX as u32 {
        bc[pc] = op;
        bc[pc+1] = offset as u8;
      } else if offset < u16::MAX as u32 {
        let offset = (unsafe { u16::try_from(offset).unwrap_unchecked() });
        bc[pc] = op::__wide;
        bc[pc+1] = op;
        bc[pc+2..pc+2+std::mem::size_of::<u16>()].copy_from_slice(&offset.to_le_bytes());
      } else {
        bc[pc] = op::__xwide;
        bc[pc+1] = op;
        bc[pc+2..pc+2+std::mem::size_of::<u32>()].copy_from_slice(&offset.to_le_bytes());
      }
    }
  }
}

fn emit_disassembler(ops: &[Opcode]) -> TokenStream2 {
  let emit_arm = |op_name: &Ident, print_name: &str, operands: &[Ident], name_align: usize| {
    let name_align = name_align + ".xwide".len();
    let operands = operands.iter().map(|v| v.to_string()).collect::<Vec<_>>();
    quote! {
      op::#op_name => Some(Disassembly::new(#print_name, self, pc, &[#(#operands),*], width, #name_align)),
    }
  };

  let name_align = ops
    .iter()
    .map(|v| v.name.to_string().len())
    .chain(["suspend".len()])
    .max()
    .unwrap();

  let mut arms = vec![];
  arms.push(emit_arm(
    &Ident::new("__nop", Span::call_site()),
    "nop",
    &[],
    name_align,
  ));
  arms.extend(
    ops
      .iter()
      .map(|op| emit_arm(&op.name, &op.name.to_string(), &op.operands, name_align)),
  );
  arms.push(emit_arm(
    &Ident::new("__suspend", Span::call_site()),
    "suspend",
    &[],
    name_align,
  ));

  quote! {
    pub struct Disassembly<'bc> {
      name: &'static str,
      bc: &'bc BytecodeArray,
      pc: usize,
      operands: &'static [&'static str],
      width: Width,
      align: usize,
    }
    impl<'bc> Disassembly<'bc> {
      fn new(
        name: &'static str,
        bc: &'bc BytecodeArray,
        pc: usize,
        operands: &'static [&'static str],
        width: Width,
        align: usize,
      ) -> Self {
        Self {
          name,
          bc,
          pc,
          operands,
          width,
          align,
        }
      }
      pub fn size(&self) -> usize {
        let prefix = if self.width as usize > 1 {
          1
        } else {
          0
        };
        prefix + 1 + self.operands.len() * self.width as usize
      }
    }
    impl<'bc> ::std::fmt::Display for Disassembly<'bc> {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        let Self { name, bc, pc, operands, width, align } = self;
        write!(f, "{}", name)?;
        let mod_align = match width {
          Width::_1 => { 0 },
          Width::_2 => {
            write!(f, ".wide")?;
            ".wide".len()
          },
          Width::_4 => {
            write!(f, ".xwide")?;
            ".xwide".len()
          },
        };
        write!(f, "{:w$}", "", w = align - name.len() - mod_align)?;
        for (i, operand) in operands.iter().enumerate() {
          write!(f, " {}={}", operand, bc.get_arg(*pc, i, *width))?;
        }
        Ok(())
      }
    }

    impl BytecodeArray {
      pub fn disassemble(
        &self,
        pc: usize,
      ) -> Option<Disassembly<'_>> {
        let mut width = Width::_1;
        match self.fetch(pc) {
          op::__wide => self.disassemble_inner(self.fetch(pc+1), pc+1, Width::_2),
          op::__xwide => self.disassemble_inner(self.fetch(pc+1), pc+1, Width::_4),
          op => self.disassemble_inner(op, pc, Width::_1),
        }
      }
      fn disassemble_inner(
        &self,
        op: u8,
        pc: usize,
        width: Width,
      ) -> Option<Disassembly<'_>> {
        match op {
          #(#arms)*
          _ => None,
        }
      }
    }
  }
}

#[proc_macro]
pub fn define_bytecode(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as Input);

  // 4 predefined opcodes (nop, wide, xwide, suspend)
  const MAX_OPCODES: usize = (u8::MAX - 4) as usize;
  if input.opcodes.len() > MAX_OPCODES {
    return Error::new(
      Span::call_site(),
      format!("too many opcodes, maximum is {MAX_OPCODES}"),
    )
    .to_compile_error()
    .into();
  }

  const MAX_OPERANDS: usize = 3;
  if let Some(op) = input
    .opcodes
    .iter()
    .find(|o| o.operands.len() > MAX_OPERANDS)
  {
    return Error::new(
      op.name.span(),
      format!("too many operands, maximum is {MAX_OPERANDS}"),
    )
    .to_compile_error()
    .into();
  }

  let handler_trait = emit_handler_trait(&input.opcodes);
  let op_mod = emit_op_mod(&input.opcodes);
  let dispatch_handlers = emit_dispatch_handlers(&input.opcodes);
  let run_fn = emit_run_fn(&input.opcodes);
  let bytecode_builder_impl = emit_bytecode_builder_impl(&input.opcodes);
  let jump_patching = emit_jump_patching(&input.opcodes);
  let disassembler = emit_disassembler(&input.opcodes);

  proc_macro::TokenStream::from(quote! {
    #handler_trait
    #op_mod
    #dispatch_handlers
    #run_fn
    #bytecode_builder_impl
    #jump_patching
    #disassembler
  })
}
