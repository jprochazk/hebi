use super::*;

impl<'src> Parser<'src> {
  pub(super) fn top_level_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
    self.indent_eq()?;

    if self.bump_if(Kw_Use) {
      self.import_stmt(module)?;
    } else {
      module.body.push(self.stmt()?)
    }

    Ok(())
  }

  fn import_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
    #[allow(clippy::ptr_arg)]
    fn extend_path<'src>(p: &Vec<ast::Ident<'src>>, v: ast::Ident<'src>) -> Vec<ast::Ident<'src>> {
      let mut p = p.clone();
      p.push(v);
      p
    }

    fn import_stmt_inner<'src>(
      p: &mut Parser<'src>,
      path: &Vec<ast::Ident<'src>>,
      module: &mut ast::Module<'src>,
    ) -> Result<()> {
      check_recursion_limit(p.current().span)?;

      let path = extend_path(path, p.ident()?);
      if p.bump_if(Kw_As) {
        let alias = Some(p.ident()?);
        module.imports.push(ast::Import { path, alias });
        return Ok(());
      }

      if p.bump_if(Op_Dot) {
        if p.bump_if(Brk_CurlyL) {
          import_stmt_inner(p, &path, module)?;
          while p.bump_if(Tok_Comma) && !p.current().is(Brk_CurlyR) {
            import_stmt_inner(p, &path, module)?;
          }
          p.expect(Brk_CurlyR)?;
          return Ok(());
        }

        import_stmt_inner(p, &path, module)?;
        return Ok(());
      }

      module.imports.push(ast::Import { path, alias: None });
      Ok(())
    }

    let path = vec![];
    if self.bump_if(Brk_CurlyL) {
      import_stmt_inner(self, &path, module)?;
      while self.bump_if(Tok_Comma) && !self.current().is(Brk_CurlyR) {
        import_stmt_inner(self, &path, module)?;
      }
      self.expect(Brk_CurlyR)?;
    } else {
      import_stmt_inner(self, &path, module)?;
    }

    Ok(())
  }

  fn stmt(&mut self) -> Result<ast::Stmt<'src>> {
    match self.scoped_stmt()? {
      Some(stmt) => Ok(stmt),
      None => self.simple_stmt(),
    }
  }

  fn scoped_stmt(&mut self) -> Result<Option<ast::Stmt<'src>>> {
    Ok(match self.current().kind {
      Kw_If => Some(self.if_stmt()?),
      Kw_For => Some(self.for_loop_stmt()?),
      Kw_While => Some(self.while_loop_stmt()?),
      Kw_Loop => Some(self.loop_stmt()?),
      Kw_Fn => Some(self.func_stmt()?),
      Kw_Class => Some(self.class_stmt()?),
      _ => None,
    })
  }

  fn if_stmt(&mut self) -> Result<ast::Stmt<'src>> {
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

  fn if_branch(&mut self) -> Result<ast::Branch<'src>> {
    let cond = self.expr()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.body()?;
    Ok(ast::branch(cond, body))
  }

  fn for_loop_stmt(&mut self) -> Result<ast::Stmt<'src>> {
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

  fn for_iter(&mut self) -> Result<ast::ForIter<'src>> {
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

  fn while_loop_stmt(&mut self) -> Result<ast::Stmt<'src>> {
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

  fn loop_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    self.expect(Kw_Loop)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let body = self.loop_body()?;
    let end = self.previous().span.end;
    Ok(ast::loop_stmt(start..end, body))
  }

  fn loop_body(&mut self) -> Result<Vec<ast::Stmt<'src>>> {
    let ctx = Context {
      current_loop: Some(()),
      current_func: self.ctx.current_func,
      ..Default::default()
    };
    let (ctx, body) = self.with_ctx2(ctx, Self::body)?;
    self.ctx.current_func = ctx.current_func;

    Ok(body)
  }

  fn func_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    self.expect(Kw_Fn)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let name = self.ident()?;
    self.no_indent()?; // func's opening paren must be unindented
    let func = self.func(name)?;
    let end = self.previous().span.end;
    Ok(ast::func_stmt(start..end, func))
  }

  fn func(&mut self, name: ast::Ident<'src>) -> Result<ast::Func<'src>> {
    let params = self.func_params()?;
    self.no_indent()?;
    self.expect(Tok_Colon)?;
    let ctx = Context {
      current_func: Some(Func::default()),
      ..Default::default()
    };
    let (ctx, body) = self.with_ctx2(ctx, Self::body)?;
    let has_yield = ctx
      .current_func
      // TODO: improve `ctx` API to make this impossible?
      .expect("`ctx.current_func` set to `None` by a mysterious force outside of `Parser::func`")
      .has_yield;
    Ok(ast::func(name, params, body, has_yield))
  }

  // TODO: default params

  fn func_params(&mut self) -> Result<Vec<ast::Ident<'src>>> {
    self.expect(Brk_ParenL)?;
    let mut params = vec![];
    if !self.current().is(Brk_ParenR) {
      params.push(self.ident()?);
      while self.bump_if(Tok_Comma) && !self.current().is(Brk_ParenR) {
        params.push(self.ident()?);
      }
    }
    self.expect(Brk_ParenR)?;
    Ok(params)
  }

  fn class_stmt(&mut self) -> Result<ast::Stmt<'src>> {
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
    let mut fields = vec![];
    let mut funcs = vec![];
    self.class_members(&mut fields, &mut funcs)?;
    let end = self.previous().span.end;
    Ok(ast::class_stmt(start..end, name, parent, fields, funcs))
  }

  fn class_members(
    &mut self,
    fields: &mut Vec<ast::Field<'src>>,
    funcs: &mut Vec<ast::Func<'src>>,
  ) -> Result<()> {
    if self.no_indent().is_ok() {
      // empty class (single line)
      self.expect(Kw_Pass)?;
      return Ok(());
    }

    self.indent_gt()?;
    if self.bump_if(Kw_Pass) {
      // empty class (indented)
      self.dedent()?;
      return Ok(());
    }

    while self.current().is(Lit_Ident) && self.indent_eq().is_ok() {
      let name = self.ident()?;
      if self.current().is(Op_Equal) {
        self.no_indent()?; // op_equal must be unindented
        self.bump(); // bump op_equal
        self.no_indent()?;
        let default = Some(self.expr()?);
        fields.push(ast::Field { name, default });
      } else if self.current().is(Brk_ParenL) {
        self.no_indent()?; // func's opening paren must be unindented
        let f = self.func(name)?;
        funcs.push(f);
      } else {
        fields.push(ast::Field {
          name,
          default: None,
        });
      }
    }

    self.dedent()?;

    Ok(())
  }

  fn body(&mut self) -> Result<Vec<ast::Stmt<'src>>> {
    check_recursion_limit(self.current().span)?;
    if self.no_indent().is_ok() {
      Ok(vec![self.simple_stmt()?])
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

  fn simple_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    match self.current().kind {
      Kw_Pass => self.pass_stmt(),
      Kw_Return => self.return_stmt(),
      Kw_Continue => self.continue_stmt(),
      Kw_Break => self.break_stmt(),
      Kw_Yield => self.yield_stmt(),
      _ => self.expr_stmt(),
    }
  }

  fn pass_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    self.expect(Kw_Pass)?;
    Ok(ast::pass_stmt(self.previous().span))
  }

  fn return_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    if self.ctx.current_func.is_none() {
      return Err(Error::new(
        "return outside of function",
        self.current().span,
      ));
    }

    self.expect(Kw_Return)?;
    let start = self.previous().span.start;
    let value = self.no_indent().ok().map(|_| self.expr()).transpose()?;
    let end = self.previous().span.end;
    Ok(ast::return_stmt(start..end, value))
  }

  fn continue_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    if self.ctx.current_loop.is_none() {
      return Err(Error::new("continue outside of loop", self.current().span));
    }

    self.expect(Kw_Continue)?;
    Ok(ast::continue_stmt(self.previous().span))
  }

  fn break_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    if self.ctx.current_loop.is_none() {
      return Err(Error::new("break outside of loop", self.current().span));
    }

    self.expect(Kw_Break)?;
    Ok(ast::break_stmt(self.previous().span))
  }

  fn yield_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    if self.ctx.current_func.is_none() {
      return Err(Error::new("yield outside of function", self.current().span));
    }

    self.expect(Kw_Yield)?;
    let start = self.previous().span.start;
    self.no_indent()?;
    let value = self.expr()?;
    let current_func = self
      .ctx
      .current_func
      .as_mut()
      // TODO: improve `ctx` API to make this impossible?
      .expect("`ctx.current_func` set to `None` by a mysterious force outside of `Parser::func`");
    current_func.has_yield = true;
    Ok(ast::yield_stmt(start..value.span.end, value))
  }

  fn expr_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    self.assign_stmt()
  }

  fn assign_stmt(&mut self) -> Result<ast::Stmt<'src>> {
    let target = self.expr()?;

    'assign: {
      if self.no_indent().is_ok() {
        let Some(kind) = self.assign_kind() else { break 'assign };
        let error_span = target.span.start..self.previous().span.end;
        self.no_indent()?;
        let value = self.expr()?;
        let Some(stmt) = ast::assign(target, kind, value) else {
            let msg = match kind {
              ast::AssignKind::Decl => "invalid variable declaration",
              ast::AssignKind::Op(_) => "invalid assignment target",
            };
            return Err(Error::new(msg, error_span));
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
