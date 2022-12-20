mod state;

use crate::ast;
use crate::lexer::Lexer;
use crate::lexer::TokenKind::*;

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
  let mut s = State::new(lex);
  grammar::module(lex, &mut s)?;
  Ok(s.module)
}

use state::State;

peg::parser! {
  grammar grammar<'src, 'lex>(s: &mut State<'input, 'lex>) for Lexer<'src> {
    pub rule module()
      = top_level_stmt()*

    rule top_level_stmt()
      = __ import_stmt() // one `import_stmt` may produce multiple imports
      / __ stmt:simple_stmt() { s.module.body.push(stmt) }
      / __ stmt:block_stmt()  { s.module.body.push(stmt) }

    rule import_stmt()
      = [Kw_Use] import_path_inner(&vec![])

      rule import_path_inner( path: &Vec<ast::Ident<'input>>)
        = import_path_list(path)
        / import_path(path)

      rule import_path_list( path: &Vec<ast::Ident<'input>>)
        = [Brk_CurlyL] import_path(path) ([Tok_Comma] import_path(path))* [Tok_Comma]? [Brk_CurlyR]

      rule import_path( path: &Vec<ast::Ident<'input>>)
        = name:ident() [Op_Dot] import_path_inner(&path.with(name))
        / name:ident() [Kw_As] alias:ident() { s.module.imports.push(ast::Import::alias(path.with(name), alias)); }
        / name:ident() { s.module.imports.push(ast::Import::normal(path.with(name))); }

    // statements that don't introduce blocks must not be indented
    rule simple_stmt() -> ast::Stmt<'input>
      = ctrl_stmt()
      / expr_stmt()

      rule ctrl_stmt() -> ast::Stmt<'input>
        = l:position!() [Kw_Return] v:expr()? r:position!() { ast::stmt_return(l..r, v) }
        / l:position!() [Kw_Yield] v:expr() r:position!() { ast::stmt_yield(l..r, v) }
        / l:position!() [Kw_Continue] r:position!() { ast::stmt_continue(l..r) }
        / l:position!() [Kw_Break] r:position!() { ast::stmt_break(l..r) }

      rule expr_stmt() -> ast::Stmt<'input>
        = v:expr() { ast::stmt_expr(v) }

    // statements that introduce blocks must be indented
    rule block_stmt() -> ast::Stmt<'input>
      = if_stmt()
      / loop_stmt()
      / fn_stmt()
      / class_stmt()

      rule if_stmt() -> ast::Stmt<'input>
        =
        l:position!()
        first:([Kw_If] cond:expr() [Tok_Colon] body:block_body() { ast::branch(cond, body) })
        other:(__ [Kw_Elif] cond:expr() [Tok_Colon] body:block_body() { ast::branch(cond, body) })*
        default:(__ [Kw_Else] body:block_body() { body })?
        r:position!()
        {
          let mut other = other;
          other.splice(0..0, [first]);
          ast::stmt_if(l..r, other, default)
        }

      rule loop_stmt() -> ast::Stmt<'input>
        = for_loop_stmt()
        / while_loop_stmt()
        / inf_loop_stmt()

        rule for_loop_stmt() -> ast::Stmt<'input>
        = [Kw_For] { todo!() }

        rule while_loop_stmt() -> ast::Stmt<'input>
        = [Kw_For] { todo!() }

        rule inf_loop_stmt() -> ast::Stmt<'input>
        = [Kw_For] { todo!() }

      rule fn_stmt() -> ast::Stmt<'input>
        = [Kw_For]  { todo!() }

      rule class_stmt() -> ast::Stmt<'input>
        = [Kw_For] { todo!() }

    rule block_body() -> Vec<ast::Stmt<'input>>
      = // one un-indented simple stmt
        _ stmt:simple_stmt() {
          vec![stmt]
        }
        // or any number of statements in a freshly indented block
      / ___ first:stmt()
        other:(__ stmt:stmt() { stmt })*
        {
          let mut other = other;
          other.splice(0..0, [first]);
          other
        }

    // this should parse a stmt without caring for whitespace
    rule stmt() -> ast::Stmt<'input>
      = [_] { todo!() }

    // exprs care about whitespace unless they are within parentheses
    rule expr() -> ast::Expr<'input>
      = [_] { todo!() }

    rule ident() -> ast::Ident<'input>
      = pos:position!() [Lit_Ident]
      {
        let t = s.lexer.get(pos).unwrap();
        ast::Ident::new(t.span, s.lexer.lexeme(t).into())
      }

    // indentation rules

    /// indent == None
    rule _()
      = &[_] pos:position!() {?
        let t = s.lexer.get(pos).unwrap();
        match t.indent() {
          Some(_) => Ok(()),
          None => Err("invalid indentation"),
        }
      }

    /// indent == current_indentation_level
    rule __()
      = &[_] pos:position!() {?
        let t = s.lexer.get(pos).unwrap();
        let Some(n) = t.indent() else {
          return Err("invalid indentation")
        };
        if !s.indent.is_indent_eq(n) {
          return Err("invalid indentation")
        }
        Ok(())
      }

    /// indent > current_indentation_level
    rule ___()
      = &[_] pos:position!() {?
        let t = s.lexer.get(pos).unwrap();
        let Some(n) = t.indent() else {
          return Err("invalid indentation")
        };
        if !s.indent.is_indent_gt(n) {
          return Err("invalid indentation")
        }
        Ok(())
      }

    rule ignore_indent<I>(inner: rule<I>) -> I
      = _ignore_indent_start() v:inner() _ignore_indent_end() { v }

      rule _ignore_indent_start()
        = { s.indent.ignore(true) }

      rule _ignore_indent_end()
        = { s.indent.ignore(false) }

    // rule list<I, S>(item: rule<I>, sep: rule<S>) -> (Option<I>, Vec<(S, I)>)
    //   = first:item() items:(s:sep() i:item() { (s, i) })* sep()? { (Some(first), items) }
    //   / { (None, vec![]) }
  }
}

#[cfg(test)]
mod tests;
