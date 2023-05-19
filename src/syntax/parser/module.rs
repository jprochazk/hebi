use super::*;

impl<'src> Parser<'src> {
  pub(super) fn module(mut self) -> Result<ast::Module<'src>, Vec<SpannedError>> {
    while !self.current().is(Tok_Eof) {
      if let Err(e) = self.top_level_stmt() {
        self.errors.push(e);
        self.sync();
      }
    }

    if !self.errors.is_empty() {
      return Err(self.errors);
    }

    Ok(self.module)
  }
}
