#[macro_use]
mod macros;

mod state;

use state::StateRef;

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
  let s = StateRef::new(lex);
  grammar::module(lex, &s)?;
  Ok(s.into_inner().module)
}

peg::parser! {
  grammar grammar<'src, 'lex>(s: &StateRef<'input, 'lex>) for Lexer<'src> {
    pub rule module()
      = top_level_stmt()*

    rule top_level_stmt()
      = __ import_stmt() // one `import_stmt` may produce multiple imports
      / __ stmt:simple_stmt() { s.push_stmt(stmt) }
      / __ stmt:block_stmt()  { s.push_stmt(stmt) }

    rule import_stmt()
      = [Kw_Use] import_path_inner(&vec![])

      rule import_path_inner(path: &Vec<ast::Ident<'input>>)
        = import_path_list(path)
        / import_path(path)

      rule import_path_list( path: &Vec<ast::Ident<'input>>)
        = [Brk_CurlyL] import_path(path) ([Tok_Comma] import_path(path))* [Tok_Comma]? [Brk_CurlyR]

      rule import_path( path: &Vec<ast::Ident<'input>>)
        = name:ident() [Op_Dot] import_path_inner(&path.with(name))
        / name:ident() [Kw_As] alias:ident() { s.push_import(ast::Import::alias(path.with(name), alias)); }
        / name:ident() { s.push_import(ast::Import::normal(path.with(name))); }

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
        = [Kw_While] { todo!() }

        rule inf_loop_stmt() -> ast::Stmt<'input>
        = [Kw_Loop] { todo!() }

      rule fn_stmt() -> ast::Stmt<'input>
        = [Kw_Fn]  { todo!() }

      rule class_stmt() -> ast::Stmt<'input>
        = [Kw_Class] { todo!() }

    rule block_body() -> Vec<ast::Stmt<'input>>
      = // one un-indented simple stmt
        _ stmt:simple_stmt() {
          vec![stmt]
        }
        // or any number of statements in a freshly indented block
      / expect_indent()
        first:stmt()
        other:(__ stmt:stmt() { stmt })*
        expect_dedent()
        {
          let mut other = other;
          other.splice(0..0, [first]);
          other
        }

    // this should parse a stmt without caring for whitespace
    rule stmt() -> ast::Stmt<'input>
      = [_] { todo!() }

    // TODO: assignment is an expression, but it may not be present in other expressions
    // *AND* it may only actually exist in the context of a statement.
    // TODO: expr_range is not a thing, it only exists in the context of `for` loops.

    // exprs care about whitespace unless they are within parentheses
    rule expr() -> ast::Expr<'input>
      = precedence! {
        left:(@) _ [Op_QuestionQuestion] _ right:@ { binary!(left (??) right) }
        --
        left:(@) _ [Op_PipePipe] _ right:@ { binary!(left (||) right) }
        --
        left:(@) _ [Op_AndAnd] _ right:@ { binary!(left (&&) right) }
        --
        left:(@) _ [Op_EqualEqual] _ right:@ { binary!(left (==) right) }
        left:(@) _ [Op_BangEqual] _ right:@ { binary!(left (!=) right) }
        --
        left:(@) _ [Op_More] _ right:@ { binary!(left (>) right) }
        left:(@) _ [Op_MoreEqual] _ right:@ { binary!(left (>=) right) }
        left:(@) _ [Op_Less] _ right:@ { binary!(left (<) right) }
        left:(@) _ [Op_LessEqual] _ right:@ { binary!(left (<=) right) }
        --
        left:(@) _ [Op_Plus] _ right:@ { binary!(left (+) right) }
        left:(@) _ [Op_Minus] _ right:@ { binary!(left (-) right) }
        --
        left:(@) _ [Op_Star] _ right:@ { binary!(left (*) right) }
        left:(@) _ [Op_Slash] _ right:@ { binary!(left (/) right) }
        left:(@) _ [Op_Percent] _ right:@ { binary!(left (%) right) }
        --
        left:(@) _ [Op_StarStar] _ right:@ { binary!(left (**) right) }
        --
        p:position!() [Op_Minus] _ right:(@) { unary!(p (-) right) }
        p:position!() [Op_Not] _ right:(@) { unary!(p (!) right) }
        --
        left:(@) _ [Brk_ParenL] args:ignore_indent(<call_args()>) [Brk_ParenR] end:position!()
          { ast::expr_call(left.span.start..end, left, args) }
        left:(@) _ [Brk_SquareL] key:expr() [Brk_SquareR] end:position!()
          { ast::expr_index(left.span.start..end, left, key) }
        left:(@) _ [Op_Dot] key:ident()
          { ast::expr_field(left.span.start..key.span.end, left, key) }
        --
        pos:position!() [Lit_Null] { literal!(s @ pos, null) }
        pos:position!() [Lit_Bool] { literal!(s @ pos, bool) }
        e:(pos:position!() [Lit_Number] {? literal!(s @ pos, num?) }) { e }
        pos:position!() [Lit_String] { literal!(s @ pos, str) }
        e:expr_array() { e }
        e:expr_object() { e }
      }

        rule expr_array() -> ast::Expr<'input>
          = l:position!()
            [Brk_SquareL] (array_item() ([Tok_Comma] array_item())* [Tok_Comma]?)? [Brk_SquareR]
            r:position!()
            { ast::expr_array(l..r, take!(s.array_items)) }

          rule array_item()
            = e:expr() { temp!(s.array_items).push(e); }

        rule expr_object() -> ast::Expr<'input>
          = [_] {todo!()}

      rule call_args() -> ast::Args<'input>
        = call_args_inner(&mut false) { take!(s.call_args) }

        rule call_args_inner(parsing_kw: &mut bool)
          = &[Brk_ParenR]
          / call_arg_one(parsing_kw) ([Tok_Comma] call_arg_one(parsing_kw))* [Tok_Comma]?

        rule call_arg_one(parsing_kw: &mut bool)
          = name:ident() [Op_Equal] value:expr() {
              *parsing_kw = true;
              temp!(s.call_args).kw(name, value);
            }
          / value:expr() {?
            if *parsing_kw { return Err("keyword argument") }
            temp!(s.call_args).pos(value);
            Ok(())
          }

    rule ident() -> ast::Ident<'input>
      = pos:position!() [Lit_Ident]
      {
        let t = s.get_token(pos);
        ast::Ident::new(t.span, s.get_lexeme(t).into())
      }

    // indentation rules

    /// indent == None
    rule _()
      = pos:position!() &[_] {?
        if s.is_indent_ignored() {
          return Ok(());
        }
        match s.get_token(pos).indent() {
          Some(_) => Ok(()),
          None => Err("invalid indentation"),
        }
      }

    /// indent == current_indentation_level
    rule __()
      = pos:position!() &[_] {?
        if s.is_indent_ignored() {
          return Ok(());
        }
        let t = s.get_token(pos);
        let Some(n) = t.indent() else {
          return Err("invalid indentation")
        };
        if !s.is_indent_eq(n) {
          return Err("invalid indentation")
        }
        Ok(())
      }

    /// indent > current_indentation_level
    rule expect_indent()
      = pos:position!() &[_] {?
        if s.is_indent_ignored() {
          return Ok(());
        }
        let t = s.get_token(pos);
        let Some(n) = t.indent() else {
          return Err("invalid indentation")
        };
        if !s.is_indent_gt(n) {
          return Err("invalid indentation")
        }
        s.push_indent(n);
        Ok(())
      }

    rule expect_dedent()
      = pos:position!() &[_] {?
        if s.is_indent_ignored() {
          return Ok(());
        }
        let t = s.get_token(pos);
        let Some(n) = t.indent() else {
          return Err("invalid indentation")
        };
        if !s.is_indent_lt(n) {
          return Err("invalid indentation");
        }
        s.pop_indent();
        Ok(())
      }

    rule ignore_indent<I>(inner: rule<I>) -> I
      = _ignore_indent_start() v:inner() _ignore_indent_end() { v }

      rule _ignore_indent_start()
        = { s.ignore_indent(true) }

      rule _ignore_indent_end()
        = { s.ignore_indent(false) }

    // rule list<I, S>(item: rule<I>, sep: rule<S>) -> (Option<I>, Vec<(S, I)>)
    //   = first:item() items:(s:sep() i:item() { (s, i) })* sep()? { (Some(first), items) }
    //   / { (None, vec![]) }
  }
}

#[cfg(test)]
mod tests;
