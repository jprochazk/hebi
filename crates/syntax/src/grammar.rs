use crate::ast;
use crate::lexer::{Lexer, Token};

macro_rules! t {
  ($v:tt) => {
    Token {
      kind: crate::lexer::TokenKind::$v,
      ..
    }
  };
}

trait WithElem<T> {
  /// Clone `Self` and append `elem` to it
  fn with(&self, elem: T) -> Self;
}

impl<T: Clone> WithElem<T> for Vec<T> {
  #[inline]
  fn with(&self, elem: T) -> Self {
    let mut out = Vec::<T>::with_capacity(self.len() + 1);
    out.extend(self.iter().cloned());
    out.extend([elem]);
    out
  }
}

pub fn parse<'lex, 'src>(
  lex: &'lex Lexer<'src>,
) -> Result<ast::Module<'src>, peg::error::ParseError<span::Span>>
where
  'lex: 'src,
{
  let mut m = ast::Module {
    imports: vec![],
    body: vec![],
  };

  grammar::module(lex, &mut m)?;

  Ok(m)
}

peg::parser! {
  grammar grammar<'src>() for Lexer<'src> {
    pub rule module(m: &mut ast::Module<'input>)
      = top_level_stmt(m)*

    rule top_level_stmt(m: &mut ast::Module<'input>)
      = import_stmt(m)
      // / func_stmt(m)
      // / class_stmt(m)
      // / expr_stmt(m)

    rule import_stmt(m: &mut ast::Module<'input>)
      = [t!(Kw_Use)] import_path_inner(m, &vec![])

      rule import_path_inner(m: &mut ast::Module<'input>, path: &Vec<ast::Ident<'input>>)
        = import_path_list(m, path)
        / import_path(m, path)

      rule import_path_list(m: &mut ast::Module<'input>, path: &Vec<ast::Ident<'input>>)
        = [t!(Brk_CurlyL)] import_path(m, path) ([t!(Tok_Comma)] import_path(m, path))* [t!(Tok_Comma)]? [t!(Brk_CurlyR)]

      rule import_path(m: &mut ast::Module<'input>, path: &Vec<ast::Ident<'input>>)
        = name:ident() [t!(Op_Dot)] import_path_inner(m, &path.with(name))
        / name:ident() [t!(Kw_As)] alias:ident() { m.imports.push(ast::Import::alias(path.with(name), alias)); }
        / name:ident() { m.imports.push(ast::Import::normal(path.with(name))); }

    rule ident() -> ast::Ident<'input>
      = token:[t!(Lit_Ident)]
      { ast::Ident::new(token.span, token.lexeme.clone()) }

    // rule list<I, S>(item: rule<I>, sep: rule<S>) -> (Option<I>, Vec<(S, I)>)
    //   = first:item() items:(s:sep() i:item() { (s, i) })* sep()? { (Some(first), items) }
    //   / { (None, vec![]) }
  }
}

#[cfg(test)]
mod tests;
