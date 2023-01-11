use std::collections::HashSet;
use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::ext::IdentExt;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Token, Type};

// TODO: see if you can get rid of some `.collect()` calls, because quote!
// handles iterators

struct VariableWidthOpcode {
  meta: Vec<Attribute>,
  name: Ident,
  flags: HashSet<String>,
  operands: Vec<Ident>,
}

struct FixedWidthOpcode {
  meta: Vec<Attribute>,
  name: Ident,
  flags: HashSet<String>,
  operands: Vec<TypedOperand>,
}

#[derive(Clone, Copy)]
enum FixedOperandType {
  U8,
  U16,
  U32,
  I8,
  I16,
  I32,
}

impl FixedOperandType {
  fn size(&self) -> usize {
    match self {
      FixedOperandType::U8 => 1,
      FixedOperandType::U16 => 2,
      FixedOperandType::U32 => 4,
      FixedOperandType::I8 => 1,
      FixedOperandType::I16 => 2,
      FixedOperandType::I32 => 4,
    }
  }

  fn width(&self) -> Width {
    match self {
      FixedOperandType::U8 => Width::_1,
      FixedOperandType::U16 => Width::_2,
      FixedOperandType::U32 => Width::_4,
      FixedOperandType::I8 => Width::_1,
      FixedOperandType::I16 => Width::_2,
      FixedOperandType::I32 => Width::_4,
    }
  }

  fn unsigned(&self) -> FixedOperandType {
    match self {
      FixedOperandType::U8 => FixedOperandType::U8,
      FixedOperandType::U16 => FixedOperandType::U16,
      FixedOperandType::U32 => FixedOperandType::U32,
      FixedOperandType::I8 => FixedOperandType::U8,
      FixedOperandType::I16 => FixedOperandType::U16,
      FixedOperandType::I32 => FixedOperandType::U32,
    }
  }

  fn is_signed(&self) -> bool {
    match self {
      FixedOperandType::U8 => false,
      FixedOperandType::U16 => false,
      FixedOperandType::U32 => false,
      FixedOperandType::I8 => true,
      FixedOperandType::I16 => true,
      FixedOperandType::I32 => true,
    }
  }

  fn fetch(&self, offset: usize) -> TokenStream2 {
    match self {
      FixedOperandType::U8 => quote!(unsafe { *bc.inner.get_unchecked(*pc + #offset) }),
      FixedOperandType::U16 => quote!(unsafe {
        u16::from_le_bytes([
          *bc.inner.get_unchecked(*pc + #offset),
          *bc.inner.get_unchecked(*pc + #offset + 1)
        ])
      }),
      FixedOperandType::U32 => quote!(unsafe {
        u32::from_le_bytes([
          *bc.inner.get_unchecked(*pc + #offset),
          *bc.inner.get_unchecked(*pc + #offset + 1),
          *bc.inner.get_unchecked(*pc + #offset + 2),
          *bc.inner.get_unchecked(*pc + #offset + 3)
        ])
      }),
      FixedOperandType::I8 => quote!(unsafe {
        ::std::mem::transmute::<_, i8>(
          *bc.inner.get_unchecked(*pc + #offset)
        )
      }),
      FixedOperandType::I16 => quote!(unsafe {
        i16::from_le_bytes([
          *bc.inner.get_unchecked(*pc + #offset),
          *bc.inner.get_unchecked(*pc + #offset + 1)
        ])
      }),
      FixedOperandType::I32 => quote!(unsafe {
        i32::from_le_bytes([
          *bc.inner.get_unchecked(*pc + #offset),
          *bc.inner.get_unchecked(*pc + #offset + 1),
          *bc.inner.get_unchecked(*pc + #offset + 2),
          *bc.inner.get_unchecked(*pc + #offset + 3)
        ])
      }),
    }
  }
}

struct Raw(FixedOperandType);
impl ToTokens for Raw {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    match self.0 {
      FixedOperandType::U8 => tokens.extend(TokenStream2::from_str("OperandType::U8")),
      FixedOperandType::U16 => tokens.extend(TokenStream2::from_str("OperandType::U16")),
      FixedOperandType::U32 => tokens.extend(TokenStream2::from_str("OperandType::U32")),
      FixedOperandType::I8 => tokens.extend(TokenStream2::from_str("OperandType::I8")),
      FixedOperandType::I16 => tokens.extend(TokenStream2::from_str("OperandType::I16")),
      FixedOperandType::I32 => tokens.extend(TokenStream2::from_str("OperandType::I32")),
    }
  }
}

impl ToTokens for FixedOperandType {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    match self {
      FixedOperandType::U8 => tokens.extend(TokenStream2::from_str("u8")),
      FixedOperandType::U16 => tokens.extend(TokenStream2::from_str("u16")),
      FixedOperandType::U32 => tokens.extend(TokenStream2::from_str("u32")),
      FixedOperandType::I8 => tokens.extend(TokenStream2::from_str("i8")),
      FixedOperandType::I16 => tokens.extend(TokenStream2::from_str("i16")),
      FixedOperandType::I32 => tokens.extend(TokenStream2::from_str("i32")),
    }
  }
}

struct TypedOperand {
  name: Ident,
  ty: FixedOperandType,
}

impl ToTokens for TypedOperand {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { name, ty } = self;
    tokens.extend(quote!(#name : #ty))
  }
}

/// Opcodes have the following format:
///
/// ```text
/// $name $(:$flag)* $(<$operand $(:$type)?>)*
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
enum Opcode {
  Variable(VariableWidthOpcode),
  Fixed(FixedWidthOpcode),
}

impl Opcode {
  fn new(
    meta: Vec<Attribute>,
    name: Ident,
    flags: HashSet<String>,
    operands: Vec<(Ident, Option<FixedOperandType>)>,
  ) -> Self {
    let is_fixed_width = operands.is_empty() || operands.iter().any(|o| o.1.is_some());

    if is_fixed_width {
      let operands = operands
        .into_iter()
        .map(|(name, ty)| TypedOperand {
          name,
          ty: ty.unwrap_or(FixedOperandType::U8),
        })
        .collect();
      Opcode::Fixed(FixedWidthOpcode {
        meta,
        name,
        flags,
        operands,
      })
    } else {
      let operands = operands.into_iter().map(|(name, _)| name).collect();
      Opcode::Variable(VariableWidthOpcode {
        meta,
        name,
        flags,
        operands,
      })
    }
  }

  fn name(&self) -> &Ident {
    match self {
      Opcode::Variable(v) => &v.name,
      Opcode::Fixed(v) => &v.name,
    }
  }

  fn meta(&self) -> &Vec<Attribute> {
    match self {
      Opcode::Variable(v) => &v.meta,
      Opcode::Fixed(v) => &v.meta,
    }
  }

  fn flags(&self) -> &HashSet<String> {
    match self {
      Opcode::Variable(v) => &v.flags,
      Opcode::Fixed(v) => &v.flags,
    }
  }

  fn is_jump_op(&self) -> bool {
    self.flags().contains("jump")
  }

  fn is_fixed_width(&self) -> bool {
    match self {
      Opcode::Fixed(_) => true,
      Opcode::Variable(_) => false,
    }
  }

  fn operands(&self) -> Vec<Ident> {
    match self {
      Opcode::Variable(v) => v.operands.clone(),
      Opcode::Fixed(v) => v
        .operands
        .iter()
        .map(|operand| operand.name.clone())
        .collect(),
    }
  }

  fn num_operands(&self) -> usize {
    match self {
      Opcode::Variable(v) => v.operands.len(),
      Opcode::Fixed(v) => v.operands.len(),
    }
  }
}

impl Parse for FixedOperandType {
  fn parse(input: ParseStream) -> Result<Self> {
    use syn::spanned::Spanned;

    let ty = Type::parse(input)?;

    'bad: {
      let Type::Path(p) = &ty else { break 'bad };
      if p.qself.is_some() {
        break 'bad;
      };
      if p.path.leading_colon.is_some() {
        break 'bad;
      };
      if p.path.segments.len() != 1 {
        break 'bad;
      };
      let Some(s) = p.path.segments.first() else { break 'bad };
      if !s.arguments.is_empty() {
        break 'bad;
      };
      let ty = match s.ident.to_string().as_str() {
        "u8" => FixedOperandType::U8,
        "u16" => FixedOperandType::U16,
        "u32" => FixedOperandType::U32,
        "i8" => FixedOperandType::I8,
        "i16" => FixedOperandType::I16,
        "i32" => FixedOperandType::I32,
        _ => break 'bad,
      };
      return Ok(ty);
    }

    Err(Error::new(
      ty.span(),
      "invalid operand type. valid operand types are: u8, u16, u32, i8, i16, i32",
    ))
  }
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
      let ty = if input.peek(Token![:]) {
        let _ = <Token![:]>::parse(input)?;
        Some(FixedOperandType::parse(input)?)
      } else {
        None
      };
      let _ = <Token![>]>::parse(input)?;
      operands.push((operand, ty));
    }

    if !input.peek(Token![,]) && !input.cursor().eof() {
      return Err(Error::new(input.span(), "unexpected token"));
    }

    Ok(Opcode::new(meta, name, flags, operands))
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
    .map(|op| {
      let name = op.name();
      let meta = op.meta();

      let name = quote::format_ident!("op_{name}");

      let result = if op.is_jump_op() {
        quote!(Result<Jump, Self::Error>)
      } else {
        quote!(Result<(), Self::Error>)
      };

      let operands = match op {
        Opcode::Variable(op) => {
          let operands = &op.operands;
          quote!(#(#operands : u32),*)
        }
        Opcode::Fixed(op) => {
          let operands = &op.operands;
          quote!(#(#operands),*)
        }
      };

      quote! {
        #(#meta)*
        fn #name (&mut self, #operands) -> #result;
      }
    })
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
  consts.push(quote! {pub const nop: u8 = 0;});
  consts.push(quote! {pub const wide: u8 = 1;});
  consts.push(quote! {pub const xwide: u8 = 2;});
  consts.extend(ops.iter().enumerate().map(|(i, op)| {
    let name = op.name();
    let value = TokenStream2::from_str(&format!("{}", 3 + i)).unwrap();
    quote! {pub const #name : u8 = #value;}
  }));
  consts.push(quote! {pub const suspend: u8 = 255;});

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
  let emit_one = |op: &Opcode, skip_vm_call: bool, next_width: Width| {
    let op_name = quote::format_ident!("op_{}", op.name());
    let is_jump = op.is_jump_op();
    let (get_args, vm_call) = match op {
      Opcode::Variable(op) => {
        let operands = &op.operands;
        let argc = operands.len();
        let get_args = if argc != 0 {
          quote! {
            let [#(#operands),*] = bc.get_operands_u32::<#argc>(*opcode, *pc, *width);
          }
        } else {
          quote! {}
        };

        let vm_call = if skip_vm_call {
          quote! {
            *pc += 1 + #argc * (*width) as usize;
          }
        } else if is_jump {
          quote! {
            let _jump = match vm. #op_name (#(#operands),*) {
              Ok(jump) => jump,
              Err(e) => {
                *result = Err(e);
                Jump::Skip
              },
            };
            match _jump {
              Jump::Skip => *pc += 1 + #argc * (*width) as usize,
              Jump::Goto { offset } => *pc = offset as usize,
            }
          }
        } else {
          quote! {
            *result = vm. #op_name (#(#operands),*);
            *pc += 1 + #argc * (*width) as usize;
          }
        };

        (get_args, vm_call)
      }
      Opcode::Fixed(op) => {
        let operands_size = op.operands.iter().map(|op| op.ty.size()).sum::<usize>();

        let get_args = if !skip_vm_call {
          let mut fetch = vec![];
          let mut offset = 1usize;
          for operand in op.operands.iter() {
            let name = &operand.name;
            let get = operand.ty.fetch(offset);
            fetch.push(quote! {
              let #name = #get;
            });
            offset += operand.ty.size();
          }

          quote! {
            let start = 1 + *pc;
            if start + #operands_size >= bc.len() {
              panic!(
                "malformed bytecode: missing operands for opcode {} (pc={}, w={})",
                *opcode,
                *pc,
                #operands_size,
              );
            }
            #(#fetch)*
          }
        } else {
          quote! {}
        };

        let vm_call = if !skip_vm_call {
          let operands = op
            .operands
            .iter()
            .map(|op| op.name.clone())
            .collect::<Vec<_>>();
          quote! {
            *result = vm. #op_name (#(#operands),*);
            *pc += 1 + #operands_size;
          }
        } else {
          quote! {
            *pc += 1 + #operands_size;
          }
        };

        (get_args, vm_call)
      }
    };

    quote! {
      #[inline]
      fn #op_name <H: Handler> (
        vm: &mut H,
        bc: &mut BytecodeArray,
        pc: &mut usize,
        opcode: &mut u8,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        if cfg!(feature = "disassembly") {
          println!("{}", bc.disassemble(*pc).unwrap());
        }
        #get_args
        #vm_call
        *width = #next_width;
        *opcode = bc.fetch(*pc);
      }
    }
  };

  let mut handlers = vec![];
  handlers.push(emit_one(
    &Opcode::Fixed(FixedWidthOpcode {
      meta: vec![],
      name: Ident::new("nop", Span::call_site()),
      flags: HashSet::new(),
      operands: vec![],
    }),
    true,
    Width::_1,
  ));
  handlers.push(emit_one(
    &Opcode::Fixed(FixedWidthOpcode {
      meta: vec![],
      name: Ident::new("wide", Span::call_site()),
      flags: HashSet::new(),
      operands: vec![],
    }),
    true,
    Width::_2,
  ));
  handlers.push(emit_one(
    &Opcode::Fixed(FixedWidthOpcode {
      meta: vec![],
      name: Ident::new("xwide", Span::call_site()),
      flags: HashSet::new(),
      operands: vec![],
    }),
    true,
    Width::_4,
  ));
  for op in ops {
    handlers.push(emit_one(op, false, Width::_1));
  }

  quote! {
    #(#handlers)*
  }
}

/// Emit the main dispatch loop function
fn emit_run_fn(ops: &[Opcode]) -> TokenStream2 {
  let mut arms = vec![];
  arms.push(quote! {op::nop => op_nop(vm, bc, pc, opcode, width, result),});
  arms.push(quote! {op::wide => op_wide(vm, bc, pc, opcode, width, result),});
  arms.push(quote! {op::xwide => op_xwide(vm, bc, pc, opcode, width, result),});
  arms.extend(ops.iter().map(|op| {
    let name = op.name();
    let f = quote::format_ident!("op_{name}");
    quote! {op::#name => #f(vm, bc, pc, opcode, width, result),}
  }));
  arms.push(quote! {op::suspend => break,});

  quote! {
    #[inline(never)]
    pub fn run<H: Handler>(
      vm: &mut H,
      bc: &mut BytecodeArray,
      pc: &mut usize
    ) -> Result<(), H::Error> {
      let opcode = &mut bc.fetch(*pc);
      let width = &mut Width::_1;
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
  let emit_one = |op: &Opcode| {
    let is_jump = op.is_jump_op();
    match op {
      Opcode::Variable(op) => {
        let op_name = &op.name;
        let fn_name = quote::format_ident!("op_{}", op_name);
        let operands = &op.operands;
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
      }
      Opcode::Fixed(op) => {
        let op_name = &op.name;
        let fn_name = quote::format_ident!("op_{}", op_name);
        let operands = &op.operands;
        let operand_names = operands.iter().map(|op| op.name.clone());

        quote! {
          pub fn #fn_name (&mut self, #(#operands),*) -> &mut Self {
            self.bytecode.inner.push(op::#op_name);
            #(
              self.bytecode.inner.extend_from_slice(&#operand_names.to_le_bytes()[..]);
            )*
            self
          }
        }
      }
    }
  };

  let mut methods = vec![];
  methods.push(emit_one(&Opcode::Fixed(FixedWidthOpcode {
    meta: vec![],
    name: Ident::new("nop", Span::call_site()),
    flags: HashSet::new(),
    operands: vec![],
  })));
  methods.push(emit_one(&Opcode::Fixed(FixedWidthOpcode {
    meta: vec![],
    name: Ident::new("suspend", Span::call_site()),
    flags: HashSet::new(),
    operands: vec![],
  })));

  for op in ops {
    methods.push(emit_one(op));
  }

  quote! {
    impl<Value: Hash + Eq> BytecodeBuilder<Value> {
      #(#methods)*
    }
  }
}

/// Emits utilities used to patch jump instructions.
fn emit_jump_patching(ops: &[Opcode]) -> TokenStream2 {
  let jump_ops = ops
    .iter()
    .filter(|op| op.is_jump_op())
    .map(|op| op.name())
    .collect::<Vec<_>>();
  quote! {
    fn is_jump_op(op: u8) -> bool {
      [#(op::#jump_ops),*].contains(&op)
    }

    fn patch_jump_op(bc: &mut [u8], op: u8, pc: usize, offset: u32) {
      bc[pc..pc+2+std::mem::size_of::<u32>()].copy_from_slice(&[0; 6]);
      if offset < u8::MAX as u32 {
        bc[pc] = op;
        bc[pc+1] = offset as u8;
      } else if offset < u16::MAX as u32 {
        let offset = (unsafe { u16::try_from(offset).unwrap_unchecked() });
        bc[pc] = op::wide;
        bc[pc+1] = op;
        bc[pc+2..pc+2+std::mem::size_of::<u16>()].copy_from_slice(&offset.to_le_bytes());
      } else {
        bc[pc] = op::xwide;
        bc[pc+1] = op;
        bc[pc+2..pc+2+std::mem::size_of::<u32>()].copy_from_slice(&offset.to_le_bytes());
      }
    }
  }
}

fn emit_disassembler(ops: &[Opcode]) -> TokenStream2 {
  let emit_arm = |op: &Opcode, name_align: usize| {
    let op_name = op.name();
    let print_name = op.name().to_string();
    match op {
      Opcode::Variable(op) => {
        let operands = op
          .operands
          .iter()
          .map(|v| v.to_string())
          .collect::<Vec<_>>();
        quote! {
          op::#op_name => Some(Disassembly::new(
            #print_name,
            self,
            pc,
            width as usize > 1,
            DisassemblyOperands::Variable(&[#(#operands),*], width),
            #name_align
          )),
        }
      }
      Opcode::Fixed(op) => {
        let operands = op
          .operands
          .iter()
          .map(|o| {
            let name = o.name.to_string();
            let ty = Raw(o.ty);
            quote! {(#name, #ty)}
          })
          .collect::<Vec<_>>();
        quote! {
          op::#op_name => Some(Disassembly::new(
            #print_name,
            self,
            pc,
            width as usize > 1,
            DisassemblyOperands::Fixed(&[#(#operands),*]),
            #name_align
          )),
        }
      }
    }
  };

  let name_align = ops
    .iter()
    .map(|v| v.name().to_string().len())
    .chain(["suspend".len()])
    .max()
    .unwrap()
    + ".xwide".len();

  let mut arms = vec![];
  arms.push(emit_arm(
    &Opcode::Fixed(FixedWidthOpcode {
      meta: vec![],
      name: Ident::new("nop", Span::call_site()),
      flags: HashSet::new(),
      operands: vec![],
    }),
    name_align,
  ));
  arms.extend(ops.iter().map(|op| emit_arm(op, name_align)));
  arms.push(emit_arm(
    &Opcode::Fixed(FixedWidthOpcode {
      meta: vec![],
      name: Ident::new("suspend", Span::call_site()),
      flags: HashSet::new(),
      operands: vec![],
    }),
    name_align,
  ));

  quote! {
    impl BytecodeArray {
      pub fn disassemble(
        &self,
        pc: usize,
      ) -> Option<Disassembly<'_>> {
        let mut width = Width::_1;
        match self.fetch(pc) {
          op::wide => self.disassemble_inner(self.fetch(pc+1), pc+1, Width::_2),
          op::xwide => self.disassemble_inner(self.fetch(pc+1), pc+1, Width::_4),
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

fn check_input(input: &Input) -> Result<()> {
  // TODO: check that nothing uses reserved opcode names
  // check for duplicates

  // check max number of opcodes
  // an opcode is u8, so there are 255 possible values
  // we use 4 of them for predefined opcodes:
  //   0x00 = nop
  //   0x01 = wide
  //   0x02 = xwide
  //   0xFF = suspend
  const MAX_OPCODES: usize = (u8::MAX - 4) as usize;
  if input.opcodes.len() > MAX_OPCODES {
    return Err(Error::new(
      Span::call_site(),
      format!("too many opcodes, maximum is {MAX_OPCODES}"),
    ));
  }

  // check max number of operands (3)
  const MAX_OPERANDS: usize = 3;
  if let Some(op) = input
    .opcodes
    .iter()
    .find(|o| o.num_operands() > MAX_OPERANDS)
  {
    return Err(Error::new(
      op.name().span(),
      format!("too many operands, maximum is {MAX_OPERANDS}"),
    ));
  }

  // jump instructions may not be fixed width
  let bad_jump_ops = input
    .opcodes
    .iter()
    .filter(|op| op.is_jump_op() && op.is_fixed_width())
    .collect::<Vec<_>>();
  if !bad_jump_ops.is_empty() {
    if bad_jump_ops.len() == 1 {
      return Err(Error::new(
        bad_jump_ops[0].name().span(),
        "an opcode flagged :jump may not have fixed-width operands".to_string(),
      ));
    } else {
      let name_list = bad_jump_ops
        .iter()
        .map(|op| format!("- {}", op.name()))
        .collect::<Vec<_>>()
        .join("\n");
      return Err(Error::new(
        Span::call_site(),
        format!("opcodes flagged :jump may not have fixed-width operands, but the following opcodes do:\n{name_list}")
      ));
    }
  }

  Ok(())
}

#[proc_macro]
pub fn define_bytecode(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as Input);

  if let Err(e) = check_input(&input) {
    return e.into_compile_error().into();
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
