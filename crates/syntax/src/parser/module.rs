
use super::*;

impl<'src> Parser<'src> {
  pub(super) fn module(mut self) -> Result<ast::Module<'src>, Vec<Error>> {
    let mut module = ast::Module::new();

    while !self.current().is(Tok_Eof) {
      eprintln!("{:?}", self.current());
      if let Err(e) = self.top_level_stmt(&mut module) {
        self.indent.reset();
        self.errors.push(e);
        self.sync();
      }
    }

    if !self.errors.is_empty() {
      return Err(self.errors);
    }

    Ok(module)
  }
}
