use super::*;

impl<'cx, 'src> Parser<'cx, 'src> {
  pub(super) fn expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
    self.maybe_expr()
  }

  fn maybe_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn or_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn and_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn eq_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn comp_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn add_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn mul_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn pow_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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

  fn unary_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
    let op = match self.current().kind {
      Op_Minus => ast::UnaryOp::Minus,
      Op_Plus => ast::UnaryOp::Plus,
      Op_Bang => ast::UnaryOp::Not,
      Tok_Question => ast::UnaryOp::Opt,
      _ => return self.postfix_expr(),
    };
    self.bump(); // bump operator
    let start = self.previous().span.start;
    self.no_indent()?;
    let right = self.unary_expr()?;
    Ok(ast::expr_unary(start..right.span.end, op, right))
  }

  fn postfix_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
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
          let name = self.ident()?;
          expr = ast::expr_get_field(expr.span.start..name.span.end, expr, name);
        }
        _ => break,
      }
    }
    Ok(expr)
  }

  fn primary_expr(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
    self.check_recursion_limit(self.current().span)?;

    if self.bump_if(Lit_None) {
      return Ok(ast::lit::none(self.cx, self.previous().span));
    }

    if self.bump_if(Lit_Bool) {
      let token = self.previous();
      return ast::lit::bool(self.cx, token.span, self.lex.lexeme(token));
    }

    if self.bump_if(Lit_Int) {
      let token = self.previous();
      return ast::lit::int(self.cx, token.span, self.lex.lexeme(token));
    }

    if self.bump_if(Lit_Float) {
      let token = self.previous();
      return ast::lit::float(self.cx, token.span, self.lex.lexeme(token));
    }

    if self.bump_if(Lit_String) {
      let token = self.previous();
      match ast::lit::str(token.span, self.lex.lexeme(token)) {
        Some(str) => return Ok(str),
        None => fail!(token.span, "invalid escape sequence"),
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
        items.push(self.table_field()?);
        while self.bump_if(Tok_Comma) && !self.current().is(Brk_CurlyR) {
          items.push(self.table_field()?);
        }
      }

      self.expect(Brk_CurlyR)?;
      let end = self.previous().span.end;
      return Ok(ast::expr_table(start..end, items));
    }

    if self.bump_if(Kw_Self) {
      if self.state.current_class.is_none()
        || !self.state.current_func.map(|f| f.has_self).unwrap_or(false)
      {
        fail!(
          self.previous().span,
          "cannot access `self` outside of class method",
        );
      }
      return Ok(ast::expr_get_self(self.previous().span));
    }

    if self.bump_if(Kw_Super) {
      if let Some(c) = &self.state.current_class {
        if !c.has_super {
          fail!(
            self.previous().span,
            "cannot access `super` in a class with no parent class",
          );
        }
        if !self.state.current_func.map(|f| f.has_self).unwrap_or(false) {
          fail!(
            self.previous().span,
            "cannot access `super` outside of a class method that takes `self`",
          );
        }
      } else {
        fail!(
          self.previous().span,
          "cannot access `super` outside of class method",
        )
      }
      return Ok(ast::expr_get_super(self.previous().span));
    }

    if self.current().is(Lit_Ident) {
      return Ok(ast::expr_get_var(self.ident()?));
    }

    if self.bump_if(Brk_ParenL) {
      let state = self.state.with_ignore_indent();
      let expr = self.with_state(state, |p| p.expr())?;
      self.expect(Brk_ParenR)?;
      return Ok(expr);
    }

    Err(SpannedError::new("unexpected token", self.current().span))
  }

  fn table_field(&mut self) -> Result<(ast::Expr<'src>, ast::Expr<'src>), SpannedError> {
    let key = self.table_key()?;
    self.expect(Tok_Colon)?;
    let value = self.expr()?;
    Ok((key, value))
  }

  fn table_key(&mut self) -> Result<ast::Expr<'src>, SpannedError> {
    if self.bump_if(Brk_SquareL) {
      let key = self.expr()?;
      self.expect(Brk_SquareR)?;
      Ok(key)
    } else {
      let key = ast::ident_key(self.ident()?);
      Ok(key)
    }
  }

  fn call_args(&mut self) -> Result<Vec<ast::Expr<'src>>, SpannedError> {
    let mut args = Vec::new();
    self.expect(Brk_ParenL)?;
    if !self.current().is(Brk_ParenR) {
      let state = self.state.with_ignore_indent();
      self.with_state(state, |p| {
        args.push(p.expr()?);
        while p.bump_if(Tok_Comma) && !p.current().is(Brk_ParenR) {
          args.push(p.expr()?);
        }
        Ok(())
      })?;
    }
    self.expect(Brk_ParenR)?;
    Ok(args)
  }
}
