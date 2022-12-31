
use super::*;

impl<'src> Parser<'src> {
  pub(super) fn ident(&mut self) -> Result<ast::Ident<'src>> {
    self.expect(Lit_Ident)?;
    Ok(Spanned::new(
      self.previous().span,
      Cow::from(self.lex.lexeme(self.previous())),
    ))
  }
}
