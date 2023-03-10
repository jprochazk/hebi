use super::*;

impl<'src> Parser<'src> {
  pub(super) fn expr(&mut self) -> Result<ast::Expr<'src>> {
    self.maybe_expr()
  }

  fn maybe_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.or_expr()?;
    while self.no_indent().is_ok() && self.bump_if(Op_QuestionQuestion) {
      self.no_indent()?;
      let right = self.or_expr()?;
      left = ast::expr_binary(
        left.span.start..right.span.end,
        ast::BinaryOp::Maybe,
        left,
        right,
      );
    }
    Ok(left)
  }

  fn or_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.and_expr()?;
    while self.no_indent().is_ok() && self.bump_if(Op_PipePipe) {
      self.no_indent()?;
      let right = self.and_expr()?;
      left = ast::expr_binary(
        left.span.start..right.span.end,
        ast::BinaryOp::Or,
        left,
        right,
      );
    }
    Ok(left)
  }

  fn and_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.eq_expr()?;
    while self.no_indent().is_ok() && self.bump_if(Op_AndAnd) {
      self.no_indent()?;
      let right = self.eq_expr()?;
      left = ast::expr_binary(
        left.span.start..right.span.end,
        ast::BinaryOp::And,
        left,
        right,
      );
    }
    Ok(left)
  }

  fn eq_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.comp_expr()?;
    while self.no_indent().is_ok() {
      let op = match self.current().kind {
        Op_EqualEqual => ast::BinaryOp::Eq,
        Op_BangEqual => ast::BinaryOp::Neq,
        _ => break,
      };
      self.bump(); // bump operator
      self.no_indent()?;
      let right = self.comp_expr()?;
      left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
    }
    Ok(left)
  }

  fn comp_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.add_expr()?;
    while self.no_indent().is_ok() {
      let op = match self.current().kind {
        Op_Less => ast::BinaryOp::Less,
        Op_LessEqual => ast::BinaryOp::LessEq,
        Op_More => ast::BinaryOp::More,
        Op_MoreEqual => ast::BinaryOp::MoreEq,
        _ => break,
      };
      self.bump(); // bump operator
      self.no_indent()?;
      let right = self.add_expr()?;
      left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
    }
    Ok(left)
  }

  fn add_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.mul_expr()?;
    while self.no_indent().is_ok() {
      let op = match self.current().kind {
        Op_Plus => ast::BinaryOp::Add,
        Op_Minus => ast::BinaryOp::Sub,
        _ => break,
      };
      self.bump(); // bump operator
      self.no_indent()?;
      let right = self.mul_expr()?;
      left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
    }
    Ok(left)
  }

  fn mul_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.pow_expr()?;
    while self.no_indent().is_ok() {
      let op = match self.current().kind {
        Op_Star => ast::BinaryOp::Mul,
        Op_Slash => ast::BinaryOp::Div,
        Op_Percent => ast::BinaryOp::Rem,
        _ => break,
      };
      self.bump(); // bump operator
      self.no_indent()?;
      let right = self.pow_expr()?;
      left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
    }
    Ok(left)
  }

  fn pow_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut left = self.unary_expr()?;
    while self.no_indent().is_ok() && self.bump_if(Op_StarStar) {
      self.no_indent()?;
      let right = self.unary_expr()?;
      left = ast::expr_binary(
        left.span.start..right.span.end,
        ast::BinaryOp::Pow,
        left,
        right,
      );
    }
    Ok(left)
  }

  fn unary_expr(&mut self) -> Result<ast::Expr<'src>> {
    let op = match self.current().kind {
      Op_Minus => ast::UnaryOp::Minus,
      Op_Plus => ast::UnaryOp::Plus,
      Op_Bang => ast::UnaryOp::Not,
      Tok_Question => ast::UnaryOp::Opt,
      // TODO: yield should probably not be a unary expr
      Kw_Yield => return self.yield_().map(ast::yield_expr),
      _ => return self.postfix_expr(),
    };
    self.bump(); // bump operator
    let start = self.previous().span.start;
    self.no_indent()?;
    let right = self.unary_expr()?;
    Ok(ast::expr_unary(start..right.span.end, op, right))
  }

  fn postfix_expr(&mut self) -> Result<ast::Expr<'src>> {
    let mut expr = self.primary_expr()?;
    while self.no_indent().is_ok() {
      match self.current().kind {
        Brk_ParenL => {
          let args = self.call_args()?; // bumps `(`
          expr = ast::expr_call(expr.span.start..self.previous().span.end, expr, args);
        }
        Brk_SquareL => {
          self.bump(); // bump `[`
          let key = self.expr()?;
          self.expect(Brk_SquareR)?;
          expr = ast::expr_get_index(expr.span.start..self.previous().span.end, expr, key);
        }
        Op_Dot => {
          self.bump(); // bump `.`
          let name = self.ident().context("field key")?;
          expr = ast::expr_get_field(expr.span.start..name.span.end, expr, name);
        }
        _ => break,
      }
    }
    Ok(expr)
  }

  fn primary_expr(&mut self) -> Result<ast::Expr<'src>> {
    check_recursion_limit(self.current().span)?;

    if self.bump_if(Lit_None) {
      return Ok(ast::lit::none(self.previous().span));
    }

    if self.bump_if(Lit_Bool) {
      let token = self.previous();
      return Ok(ast::lit::bool(token.span, self.lex.lexeme(token)));
    }

    if self.bump_if(Lit_Int) {
      let token = self.previous();
      return ast::lit::int(token.span, self.lex.lexeme(token));
    }

    if self.bump_if(Lit_Float) {
      let token = self.previous();
      return ast::lit::float(token.span, self.lex.lexeme(token));
    }

    if self.bump_if(Lit_String) {
      let token = self.previous();
      match ast::lit::str(token.span, self.lex.lexeme(token)) {
        Some(str) => return Ok(str),
        None => return Err(Error::new("invalid escape sequence", token.span)),
      }
    }

    if self.bump_if(Brk_SquareL) {
      let start = self.previous().span.start;

      let mut items = vec![];
      if !self.current().is(Brk_SquareR) {
        items.push(self.expr()?);
        while self.bump_if(Tok_Comma) && !self.current().is(Brk_SquareR) {
          items.push(self.expr()?);
        }
      }

      self.expect(Brk_SquareR)?;
      let end = self.previous().span.end;
      return Ok(ast::expr_list(start..end, items));
    }

    if self.bump_if(Brk_CurlyL) {
      let start = self.previous().span.start;

      let mut items = vec![];
      if !self.current().is(Brk_CurlyR) {
        items.push(self.dict_field()?);
        while self.bump_if(Tok_Comma) && !self.current().is(Brk_CurlyR) {
          items.push(self.dict_field()?);
        }
      }

      self.expect(Brk_CurlyR)?;
      let end = self.previous().span.end;
      return Ok(ast::expr_dict(start..end, items));
    }

    if self.bump_if(Kw_Self) {
      if self.ctx.current_class.is_none()
        || !self.ctx.current_func.map(|f| f.has_self).unwrap_or(false)
      {
        return Err(Error::new(
          "cannot access `self` outside of class method",
          self.previous().span,
        ));
      }
      return Ok(ast::expr_get_self(self.previous().span));
    }

    if self.bump_if(Kw_Super) {
      if let Some(c) = &self.ctx.current_class {
        if !c.has_super {
          return Err(Error::new(
            "cannot access `super` in a class with no parent class",
            self.previous().span,
          ));
        }
        if !self.ctx.current_func.map(|f| f.has_self).unwrap_or(false) {
          return Err(Error::new(
            "cannot access `super` outside of a class method that takes `self`",
            self.previous().span,
          ));
        }
      } else {
        return Err(Error::new(
          "cannot access `super` outside of class method",
          self.previous().span,
        ));
      }
      return Ok(ast::expr_get_super(self.previous().span));
    }

    if self.current().is(Lit_Ident) {
      return Ok(ast::expr_get_var(self.ident()?));
    }

    if self.bump_if(Brk_ParenL) {
      let ctx = self.ctx.with_ignore_indent();
      let expr = self.with_ctx(ctx, |p| p.expr())?;
      self.expect(Brk_ParenR)?;
      return Ok(expr);
    }

    Err(Error::new("unexpected token", self.current().span))
  }

  fn dict_field(&mut self) -> Result<(ast::Expr<'src>, ast::Expr<'src>)> {
    let key = self.dict_key()?;
    self.expect(Tok_Colon)?;
    let value = self.expr()?;
    Ok((key, value))
  }

  fn dict_key(&mut self) -> Result<ast::Expr<'src>> {
    if self.bump_if(Brk_SquareL) {
      let key = self.expr()?;
      self.expect(Brk_SquareR)?;
      Ok(key)
    } else {
      let key = ast::ident_key(self.ident()?);
      Ok(key)
    }
  }

  fn call_args(&mut self) -> Result<ast::Args<'src>> {
    let mut args = ast::Args::new();
    self.expect(Brk_ParenL)?;
    if !self.current().is(Brk_ParenR) {
      let ctx = self.ctx.with_ignore_indent();
      self.with_ctx(ctx, |p| {
        let mut parsing_kw = false;
        p.call_arg_one(&mut args, &mut parsing_kw)?;
        while p.bump_if(Tok_Comma) && !p.current().is(Brk_ParenR) {
          p.call_arg_one(&mut args, &mut parsing_kw)?;
        }
        Ok(())
      })?;
    }
    self.expect(Brk_ParenR)?;
    Ok(args)
  }

  fn call_arg_one(&mut self, args: &mut ast::Args<'src>, parsing_kw: &mut bool) -> Result<()> {
    // to avoid lookahead or backtracking, `GetVar` is treated as the identifier in
    // f(<ident> = <value>)
    let expr = self.expr()?;
    if *parsing_kw {
      let expr_span = expr.span;
      let ast::ExprKind::GetVar(ident) = expr.into_inner() else {
        return Err(Error::new("positional argument follows keyword argument", expr_span));
      };
      self.expect(Op_Equal)?;
      let value = self.expr()?;
      args.kw(ident.name, value);
    } else if self.bump_if(Op_Equal) {
      *parsing_kw = true;
      let expr_span = expr.span;
      let ast::ExprKind::GetVar(ident) = expr.into_inner() else {
        return Err(Error::new("positional argument follows keyword argument", expr_span));
      };
      let value = self.expr()?;
      args.kw(ident.name, value);
    } else {
      args.pos(expr);
    }

    Ok(())
  }
}
