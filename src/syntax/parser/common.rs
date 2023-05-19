use super::*;
use crate::span::Spanned;

impl<'src> Parser<'src> {
  pub(super) fn ident(&mut self) -> Result<ast::Ident<'src>, SpannedError> {
    self.expect(Lit_Ident)?;
    Ok(ast::Ident::new(
      self.previous().span,
      Cow::from(self.lex.lexeme(self.previous())),
    ))
  }

  pub(super) fn yield_(&mut self) -> Result<Spanned<ast::Yield<'src>>, SpannedError> {
    if self.state.current_func.is_none() {
      fail!(@self.current().span, "yield outside of function");
    }

    self.expect(Kw_Yield)?;
    let start = self.previous().span.start;
    let value = self.no_indent().ok().map(|_| self.expr()).transpose()?;
    let end = self.previous().span.end;

    let current_func = self
      .state
      .current_func
      .as_mut()
      // TODO: improve `state` API to make this impossible?
      .expect("`state.current_func` set to `None` by a mysterious force outside of `Parser::func`");
    current_func.has_yield = true;

    Ok(Spanned::new(start..end, ast::Yield { value }))
  }
}
