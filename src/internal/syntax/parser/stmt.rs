use std::collections::HashSet;

use super::*;

impl<'src> Parser<'src> {
  pub(super) fn top_level_stmt(&mut self) -> Result<(), SpannedError> {
    self.indent_eq()?;
    let stmt = self.stmt()?;
    self.module.body.push(stmt);
    Ok(())
  }

  fn stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    match self.scoped_stmt()? {
      Some(stmt) => Ok(stmt),
      None => self.simple_stmt(),
    }
    .map(|v| {
      self.bump_if(Tok_Semicolon);
      v
    })
  }

  fn scoped_stmt(&mut self) -> Result<Option<ast::Stmt<'src>>, SpannedError> {
    Ok(match self.current().kind {
      Kw_If => Some(self.if_stmt()?),
      Kw_For => Some(self.for_loop_stmt()?),
      Kw_While => Some(self.while_loop_stmt()?),
      Kw_Loop => Some(self.loop_stmt()?),
      Kw_Fn => Some(self.func_stmt()?),
      Kw_Class => Some(self.class_stmt()?),
      Kw_Import | Kw_From => Some(self.import_stmt()?),
      _ => None,
    })
  }

  fn import_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    if self.bump_if(Kw_Import) {
      // import <module>
      let start = self.previous().span.start;
      let module = self.import_module_path()?;
      let alias = if self.no_indent().is_ok() && self.bump_if(Kw_As) {
        self.no_indent()?;
        Some(self.ident()?)
      } else {
        None
      };
      let end = self.previous().span.end;
      Ok(ast::import_module_stmt(start..end, module, alias))
    } else if self.bump_if(Kw_From) {
      // from <module> import <stuff>
      let start = self.previous().span.start;
      let module = self.import_module_path()?;
      self.no_indent()?;
      self.expect(Kw_Import)?;
      let symbols = self.import_symbol_list()?;
      let end = self.previous().span.end;
      Ok(ast::import_symbols_stmt(start..end, module, symbols))
    } else {
      Err(SpannedError::new(
        "expected `from` or `import`",
        self.previous().span,
      ))
    }
  }

  fn import_module_path(&mut self) -> Result<Vec<ast::Ident<'src>>, SpannedError> {
    self.no_indent()?;
    let mut path = vec![self.ident()?];
    while self.no_indent().is_ok() && self.bump_if(Op_Dot) {
      path.push(self.ident()?);
    }
    Ok(path)
  }

  fn import_symbol_list(&mut self) -> Result<Vec<ast::ImportSymbol<'src>>, SpannedError> {
    let mut symbols = vec![self.import_symbol()?];
    while self.no_indent().is_ok() && self.bump_if(Tok_Comma) {
      symbols.push(self.import_symbol()?);
    }
    Ok(symbols)
  }

  fn import_symbol(&mut self) -> Result<ast::ImportSymbol<'src>, SpannedError> {
    self.no_indent()?;
    let name = self.ident()?;
    let alias = if self.no_indent().is_ok() && self.bump_if(Kw_As) {
      Some(self.ident()?)
    } else {
      None
    };
    Ok(ast::ImportSymbol { name, alias })
  }

  fn if_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_If)?;
    let start = self.previous().span.start;

    let mut branches = vec![self.if_branch()?];
    let mut default = None;

    while self.current().is(Kw_Elif) {
      self.indent_eq()?; // `elif` on same indentation level
      self.bump(); // bump `elif`
      branches.push(self.if_branch()?);
    }
    if self.current().is(Kw_Else) {
      self.indent_eq()?; // `else` on same indentation level
      self.bump(); // bump `else`
      self.no_indent()?;
      self.expect(Tok_Colon)?;
      default = Some(self.body()?);
    }

    let end = self.previous().span.end;

    Ok(ast::if_stmt(start..end, branches, default))
  }

  fn if_branch(&mut self) -> Result<ast::Branch<'src>, SpannedError> {
    self.no_indent()?;
    let cond = self.expr()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.body()?;
    Ok(ast::branch(cond, body))
  }

  fn for_loop_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_For)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let item = self.ident()?;
    self.no_indent()?;
    self.expect(Kw_In)?;
    self.no_indent()?;
    let iter = self.for_iter()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.loop_body()?;
    let end = self.previous().span.end;
    Ok(ast::for_loop_stmt(start..end, item, iter, body))
  }

  fn for_iter(&mut self) -> Result<ast::ForIter<'src>, SpannedError> {
    let start = self.expr()?;
    let inclusive = match self.current().kind {
      Op_Range => false,
      Op_RangeInc => true,
      _ => return Ok(ast::ForIter::Expr(start)),
    };
    self.no_indent()?; // range op must be unindented
    self.bump(); // bump op
    self.no_indent()?;
    let end = self.expr()?;
    Ok(ast::ForIter::Range(ast::IterRange {
      start,
      end,
      inclusive,
    }))
  }

  fn while_loop_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_While)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let cond = self.expr()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.loop_body()?;
    let end = self.previous().span.end;
    Ok(ast::while_loop_stmt(start..end, cond, body))
  }

  fn loop_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_Loop)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.loop_body()?;
    let end = self.previous().span.end;
    Ok(ast::loop_stmt(start..end, body))
  }

  fn loop_body(&mut self) -> Result<Vec<ast::Stmt<'src>>, SpannedError> {
    let state = State::with_loop(&self.state);
    let (state, body) = self.with_state2(state, Self::body)?;
    // yield may appear in loop, in which case we have to propagate it upwards here
    self.state.current_func = state.current_func;

    Ok(body)
  }

  fn func_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_Fn)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let name = self.ident()?;
    self.no_indent()?; // func's opening paren must be unindented
    let func = self.func(name)?;
    let end = self.previous().span.end;
    Ok(ast::func_stmt(start..end, func))
  }

  fn func(&mut self, name: ast::Ident<'src>) -> Result<ast::Func<'src>, SpannedError> {
    let params = self.func_params()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let state = self.state.with_func(name.lexeme(), params.has_self);
    let (state, body) = self.with_state2(state, Self::body)?;
    let has_yield = state
      .current_func
      // TODO: improve `state` API to make this impossible?
      .expect("`state.current_func` set to `None` by a mysterious force outside of `Parser::func`")
      .has_yield;
    Ok(ast::func(name, params, body, has_yield))
  }

  fn func_params(&mut self) -> Result<ast::Params<'src>, SpannedError> {
    self.expect(Brk_ParenL)?;

    let has_self = self.bump_if(Kw_Self);
    if has_self {
      let span = self.previous().span;
      self.bump_if(Tok_Comma);
      if self.state.current_class.is_none() {
        fail!(@span, "cannot access `self` outside of class");
      }
    }

    let mut params = ast::Params {
      has_self,
      ..Default::default()
    };
    if !self.current().is(Brk_ParenR) {
      let mut state = ParamState::Positional;
      self.param(&mut params, &mut state)?;
      while self.bump_if(Tok_Comma) && !self.current().is(Brk_ParenR) {
        self.param(&mut params, &mut state)?;
      }
    }
    self.expect(Brk_ParenR)?;

    Ok(params)
  }

  fn param(
    &mut self,
    params: &mut ast::Params<'src>,
    state: &mut ParamState,
  ) -> Result<(), SpannedError> {
    let name = self.ident()?;
    if params.contains(&name) {
      fail!(@name.span, "duplicate argument `{name}`");
    }
    let default = if self.bump_if(Op_Equal) {
      *state = ParamState::Default;
      Some(self.expr()?)
    } else {
      if *state == ParamState::Default {
        fail!(
          @self.previous().span,
          "non-default argument follows default argument",
        );
      }

      None
    };

    params.pos.push(ast::Param { name, default });

    Ok(())
  }

  fn class_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_Class)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let name = self.ident()?;
    let parent = if self.current().is(Brk_ParenL) {
      self.no_indent()?; // opening paren must be unindented
      self.bump(); // bump opening paren
      let parent = self.ident()?;
      self.expect(Brk_ParenR)?;
      Some(parent)
    } else {
      None
    };
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let state = State::with_class(parent.is_some());
    let members = self.with_state(state, Self::class_members)?;
    let end = self.previous().span.end;
    Ok(ast::class_stmt(start..end, name, parent, members))
  }

  fn class_members(&mut self) -> Result<ast::ClassMembers<'src>, SpannedError> {
    let mut members = ast::ClassMembers::new();

    if self.no_indent().is_ok() {
      // empty class (single line)
      self
        .expect(Kw_Pass)
        .map_err(|e| SpannedError::new("invalid indentation", e.span))?;
      return Ok(members);
    }

    self.indent_gt()?;
    if self.bump_if(Kw_Pass) {
      // empty class (indented)
      self.dedent()?;
      return Ok(members);
    }

    let mut names = HashSet::new();

    while self.current().is(Lit_Ident) && self.indent_eq().is_ok() {
      let name = self.ident()?;

      if name == "init" {
        if members.init.is_some() {
          self
            .errors
            .push(SpannedError::new("duplicate initializer", name.span));
        }

        self.no_indent()?; // func's opening paren must be unindented
        members.init = Some(self.func(name)?);
      } else {
        if names.contains(&name) {
          self.errors.push(SpannedError::new(
            format!("duplicate field {name}"),
            name.span,
          ));
        } else {
          names.insert(name.clone());
        }

        self.no_indent()?; // op_equal must be unindented
        self.expect(Op_Equal)?;
        self.no_indent()?;
        let default = self.expr()?;
        members.fields.push(ast::Field { name, default });
      }
    }

    while self.current().is(Kw_Fn) && self.indent_eq().is_ok() {
      self.expect(Kw_Fn)?;

      let name = self.ident()?;
      if names.contains(&name) {
        self.errors.push(SpannedError::new(
          format!("duplicate field {name}"),
          name.span,
        ));
      } else if name == "init" {
        self.errors.push(SpannedError::new(
          "\
            a method may not be named `init`. \
            if you meant to write an initializer, \
            write it without the `fn` keyword\
            ",
          name.span,
        ));
      } else {
        names.insert(name.clone());
      }
      self.no_indent()?; // func's opening paren must be unindented
      let f = self.func(name)?;
      members.methods.push(f);
    }

    if self.current().is(Lit_Ident) && self.indent_eq().is_ok() {
      fail!(@self.current().span, "fields may not appear after methods",);
    }

    self.dedent()?;

    Ok(members)
  }

  fn body(&mut self) -> Result<Vec<ast::Stmt<'src>>, SpannedError> {
    self.check_recursion_limit(self.current().span)?;
    if self.no_indent().is_ok() {
      let mut body = vec![self.stmt()?];

      while self.previous().is(Tok_Semicolon)
        && !self.current().is(Tok_SemicolonSemicolon)
        && !self.current().is(Tok_Eof)
      {
        self.no_indent()?;
        body.push(self.stmt()?);
      }

      self.bump_if(Tok_SemicolonSemicolon);

      Ok(body)
    } else {
      self.indent_gt()?;

      let mut body = vec![self.stmt()?];
      while self.indent_eq().is_ok() && !self.current().is(Tok_Eof) {
        body.push(self.stmt()?);
      }

      self.dedent()?;
      Ok(body)
    }
  }

  fn simple_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    match self.current().kind {
      Kw_Pass => self.pass_stmt(),
      Kw_Return => self.return_stmt(),
      Kw_Continue => self.continue_stmt(),
      Kw_Break => self.break_stmt(),
      Kw_Yield => self.yield_().map(ast::yield_stmt),
      Kw_Print => self.print_stmt(),
      _ => self.expr_stmt(),
    }
  }

  fn pass_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_Pass)?;
    Ok(ast::pass_stmt(self.previous().span))
  }

  fn return_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    let Some(current_fn_name) = self.state.current_func.as_ref().map(|f| f.name.clone()) else {
      fail!(@self.current().span, "return outside of function");
    };

    self.expect(Kw_Return)?;
    let start = self.previous().span.start;
    if current_fn_name == "init" && self.state.current_class.is_some() && self.no_indent().is_ok() {
      fail!(@self.current().span, "return in `init` may not return a value");
    }
    let value = self.no_indent().ok().map(|_| self.expr()).transpose()?;
    let end = self.previous().span.end;
    Ok(ast::return_stmt(start..end, value))
  }

  fn continue_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    if self.state.current_loop.is_none() {
      fail!(@self.current().span, "continue outside of loop");
    }

    self.expect(Kw_Continue)?;
    Ok(ast::continue_stmt(self.previous().span))
  }

  fn break_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    if self.state.current_loop.is_none() {
      fail!(@self.current().span, "break outside of loop");
    }

    self.expect(Kw_Break)?;
    Ok(ast::break_stmt(self.previous().span))
  }

  fn print_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.expect(Kw_Print)?;
    let start = self.previous().span;
    self.no_indent()?;
    let has_parens = self.bump_if(Brk_ParenL);
    let mut values = vec![self.expr()?];
    while self.bump_if(Tok_Comma) {
      if !has_parens {
        self.no_indent()?;
      }
      values.push(self.expr()?);
    }
    if has_parens {
      self.expect(Brk_ParenR)?;
    }
    let end = self.previous().span;
    Ok(ast::print_stmt(start.join(end), values))
  }

  fn expr_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    self.assign_stmt()
  }

  fn assign_stmt(&mut self) -> Result<ast::Stmt<'src>, SpannedError> {
    let target = self.expr()?;

    'assign: {
      if self.no_indent().is_ok() {
        let Some(kind) = self.assign_kind() else {
          break 'assign;
        };
        let error_span = target.span.start..self.previous().span.end;
        self.no_indent()?;
        let value = self.expr()?;
        let Some(stmt) = ast::assign(target, kind, value) else {
          let msg = match kind {
            ast::AssignKind::Decl => "invalid variable declaration",
            ast::AssignKind::Op(_) => "invalid assignment target",
          };
          fail!(@error_span, "{msg}");
        };
        return Ok(stmt);
      }
    }

    Ok(ast::expr_stmt(target))
  }

  fn assign_kind(&mut self) -> Option<ast::AssignKind> {
    let kind = match self.current().kind {
      Op_ColonEqual => ast::AssignKind::Decl,
      Op_Equal => ast::AssignKind::Op(None),
      Op_PlusEqual => ast::AssignKind::Op(Some(ast::AssignOp::Add)),
      Op_MinusEqual => ast::AssignKind::Op(Some(ast::AssignOp::Sub)),
      Op_SlashEqual => ast::AssignKind::Op(Some(ast::AssignOp::Div)),
      Op_StarEqual => ast::AssignKind::Op(Some(ast::AssignOp::Mul)),
      Op_PercentEqual => ast::AssignKind::Op(Some(ast::AssignOp::Rem)),
      Op_StarStarEqual => ast::AssignKind::Op(Some(ast::AssignOp::Pow)),
      Op_QuestionQuestionEqual => ast::AssignKind::Op(Some(ast::AssignOp::Maybe)),
      _ => return None,
    };
    self.bump(); // bump operator
    Some(kind)
  }
}

#[allow(clippy::ptr_arg)]
fn extend_path<'src>(p: &Vec<ast::Ident<'src>>, v: ast::Ident<'src>) -> Vec<ast::Ident<'src>> {
  let mut p = p.clone();
  p.push(v);
  p
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamState {
  Positional,
  Default,
}
