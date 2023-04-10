use super::*;
use crate::span::Spanned;

impl<'src> Parser<'src> {
  pub(super) fn ident(&mut self) -> Result<ast::Ident<'src>> {
    self.expect(Lit_Ident)?;
    Ok(Spanned::new(
      self.previous().span,
      Cow::from(self.lex.lexeme(self.previous())),
    ))
  }

  pub(super) fn yield_(&mut self) -> Result<Spanned<ast::Yield<'src>>> {
    if self.ctx.current_func.is_none() {
      return Err(Error::new("yield outside of function", self.current().span));
    }

    self.expect(Kw_Yield)?;
    let start = self.previous().span.start;
    let value = self.no_indent().ok().map(|_| self.expr()).transpose()?;
    let end = self.previous().span.end;

    let current_func = self
      .ctx
      .current_func
      .as_mut()
      // TODO: improve `ctx` API to make this impossible?
      .expect("`ctx.current_func` set to `None` by a mysterious force outside of `Parser::func`");
    current_func.has_yield = true;

    Ok(Spanned::new(start..end, ast::Yield { value }))
  }
}
