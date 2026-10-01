use rishi_ast::*;
use rishi_lexer::{Token, TokenKind};
use rishi_span::Span;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token `{0:?}` at span {1:?}")]
    UnexpectedToken(TokenKind, Span),
    #[error("Expected token `{0:?}`, but found `{1:?}` at span {2:?}")]
    ExpectedToken(String, TokenKind, Span),
    #[error("Unexpected end of file")]
    UnexpectedEof,
    #[error("Invalid assignment target at span {0:?}")]
    InvalidAssignmentTarget(Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Pipeline,     // |>
    NullCoalesce, // ??
    Or,           // or, ||
    And,          // and, &&
    Equality,     // ==, !=
    Relational,   // <, <=, >, >=
    Range,        // .., ..=
    Term,         // +, -
    Factor,       // *, /, %
    Power,        // **
    Unary,        // -, !, not
    Postfix,      // (), [], ., ?., ?
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    continuation_indents: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
            continuation_indents: 0,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut stmts = Vec::new();
        let start_span = self.current_span();

        while !self.is_at_end() {
            // Skip stray newlines between statements
            if self.match_kind(&TokenKind::Newline) {
                continue;
            }
            if self.check(&TokenKind::Eof) {
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }

        let end_span = self.previous_span();
        Ok(Program {
            statements: stmts,
            span: start_span.merge(end_span),
        })
    }

    // -------------------------------------------------------------
    // STATEMENTS
    // -------------------------------------------------------------

    pub fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();

        if self.match_kind(&TokenKind::Let) {
            self.parse_let_stmt(start_span)
        } else if self.match_kind(&TokenKind::Mut) {
            self.parse_mut_stmt(start_span)
        } else if self.match_kind(&TokenKind::Fn) {
            self.parse_fn_decl(start_span)
        } else if self.match_kind(&TokenKind::Struct) {
            self.parse_struct_decl(start_span)
        } else if self.match_kind(&TokenKind::Return) {
            self.parse_return_stmt(start_span)
        } else if self.match_kind(&TokenKind::If) {
            self.parse_if_stmt(start_span)
        } else if self.match_kind(&TokenKind::While) {
            self.parse_while_stmt(start_span)
        } else if self.match_kind(&TokenKind::For) {
            self.parse_for_stmt(start_span)
        } else if self.match_kind(&TokenKind::Break) {
            self.consume_optional_newline();
            Ok(Stmt::new(StmtKind::Break, start_span))
        } else if self.match_kind(&TokenKind::Continue) {
            self.consume_optional_newline();
            Ok(Stmt::new(StmtKind::Continue, start_span))
        } else if self.match_kind(&TokenKind::Import) {
            self.parse_import_stmt(start_span)
        } else {
            self.parse_expr_or_assign_stmt()
        }
    }

    fn parse_let_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let name = self.consume_ident()?;
        let type_ann = if self.match_kind(&TokenKind::Colon) {
            Some(self.parse_type_ann()?)
        } else {
            None
        };

        self.consume(&TokenKind::Eq, "expected `=` after variable name in let statement")?;
        let init = self.parse_expr()?;
        self.consume_stmt_terminator();
        let span = start_span.merge(init.span);
        Ok(Stmt::new(
            StmtKind::Let {
                name,
                type_ann,
                init,
            },
            span,
        ))
    }

    fn parse_mut_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let name = self.consume_ident()?;
        let type_ann = if self.match_kind(&TokenKind::Colon) {
            Some(self.parse_type_ann()?)
        } else {
            None
        };

        self.consume(&TokenKind::Eq, "expected `=` after variable name in mut statement")?;
        let init = self.parse_expr()?;
        self.consume_stmt_terminator();
        let span = start_span.merge(init.span);
        Ok(Stmt::new(
            StmtKind::Mut {
                name,
                type_ann,
                init,
            },
            span,
        ))
    }

    fn parse_fn_decl(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let name = self.consume_ident()?;
        self.consume(&TokenKind::LParen, "expected `(` after function name")?;
        let params = self.parse_param_list()?;
        self.consume(&TokenKind::RParen, "expected `)` after parameter list")?;

        let return_type = if self.match_kind(&TokenKind::Arrow) {
            Some(self.parse_type_ann()?)
        } else {
            None
        };

        self.consume(&TokenKind::Colon, "expected `:` before function body")?;
        let body = self.parse_block()?;
        let span = start_span.merge(self.previous_span());

        Ok(Stmt::new(
            StmtKind::FnDecl {
                name,
                params,
                return_type,
                body,
            },
            span,
        ))
    }

    fn parse_struct_decl(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let name = self.consume_ident()?;
        self.consume(&TokenKind::Colon, "expected `:` before struct body")?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        self.consume_optional_newline();
        if self.match_kind(&TokenKind::Indent) {
            while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                if self.match_kind(&TokenKind::Fn) {
                    let fn_stmt = self.parse_fn_decl(self.previous_span())?;
                    methods.push(fn_stmt);
                } else {
                    let field_start = self.current_span();
                    let field_name = self.consume_ident()?;
                    let type_ann = if self.match_kind(&TokenKind::Colon) {
                        Some(self.parse_type_ann()?)
                    } else {
                        None
                    };
                    let default_val = if self.match_kind(&TokenKind::Eq) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    self.consume_optional_newline();
                    let span = field_start.merge(self.previous_span());
                    fields.push(StructField {
                        name: field_name,
                        type_ann,
                        default_val,
                        span,
                    });
                }
            }
            self.consume(&TokenKind::Dedent, "expected dedent at end of struct block")?;
        }

        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(
            StmtKind::StructDecl {
                name,
                fields,
                methods,
            },
            span,
        ))
    }

    fn parse_return_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let val = if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::Dedent) && !self.check(&TokenKind::Eof) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume_optional_newline();
        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(StmtKind::Return(val), span))
    }

    fn parse_if_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let condition = self.parse_expr()?;
        self.consume(&TokenKind::Colon, "expected `:` after if condition")?;
        let then_body = self.parse_block()?;

        let mut elif_branches = Vec::new();
        loop {
            self.consume_optional_newline();
            if self.match_kind(&TokenKind::Elif) {
                let elif_cond = self.parse_expr()?;
                self.consume(&TokenKind::Colon, "expected `:` after elif condition")?;
                let elif_body = self.parse_block()?;
                elif_branches.push((elif_cond, elif_body));
            } else {
                break;
            }
        }

        self.consume_optional_newline();
        let else_body = if self.match_kind(&TokenKind::Else) {
            self.consume(&TokenKind::Colon, "expected `:` after else")?;
            Some(self.parse_block()?)
        } else {
            None
        };

        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(
            StmtKind::If {
                condition,
                then_body,
                elif_branches,
                else_body,
            },
            span,
        ))
    }

    fn parse_while_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let condition = self.parse_expr()?;
        self.consume(&TokenKind::Colon, "expected `:` after while condition")?;
        let body = self.parse_block()?;
        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(StmtKind::While { condition, body }, span))
    }

    fn parse_for_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let var = self.consume_ident()?;
        self.consume(&TokenKind::In, "expected `in` after loop variable")?;
        let iter = self.parse_expr()?;
        self.consume(&TokenKind::Colon, "expected `:` after for iteration expression")?;
        let body = self.parse_block()?;
        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(StmtKind::ForIn { var, iter, body }, span))
    }

    fn parse_import_stmt(&mut self, start_span: Span) -> Result<Stmt, ParseError> {
        let module = self.consume_ident()?;
        let alias = if self.match_kind(&TokenKind::As) {
            Some(self.consume_ident()?)
        } else {
            None
        };
        self.consume_optional_newline();
        let span = start_span.merge(self.previous_span());
        Ok(Stmt::new(StmtKind::Import { module, alias }, span))
    }

    fn parse_expr_or_assign_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;

        // Check for assignments: =, +=, -=, *=, /=
        if self.match_kind(&TokenKind::Eq) {
            let target = expr_to_assign_target(expr)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            let span = self.previous_span();
            return Ok(Stmt::new(
                StmtKind::Assign {
                    target,
                    op: None,
                    value,
                },
                span,
            ));
        } else if self.match_kind(&TokenKind::PlusEq) {
            let target = expr_to_assign_target(expr)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            let span = self.previous_span();
            return Ok(Stmt::new(
                StmtKind::Assign {
                    target,
                    op: Some(BinOp::Add),
                    value,
                },
                span,
            ));
        } else if self.match_kind(&TokenKind::MinusEq) {
            let target = expr_to_assign_target(expr)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            let span = self.previous_span();
            return Ok(Stmt::new(
                StmtKind::Assign {
                    target,
                    op: Some(BinOp::Sub),
                    value,
                },
                span,
            ));
        } else if self.match_kind(&TokenKind::StarEq) {
            let target = expr_to_assign_target(expr)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            let span = self.previous_span();
            return Ok(Stmt::new(
                StmtKind::Assign {
                    target,
                    op: Some(BinOp::Mul),
                    value,
                },
                span,
            ));
        } else if self.match_kind(&TokenKind::SlashEq) {
            let target = expr_to_assign_target(expr)?;
            let value = self.parse_expr()?;
            self.consume_optional_newline();
            let span = self.previous_span();
            return Ok(Stmt::new(
                StmtKind::Assign {
                    target,
                    op: Some(BinOp::Div),
                    value,
                },
                span,
            ));
        }

        self.consume_stmt_terminator();
        let span = expr.span;
        Ok(Stmt::new(StmtKind::Expr(expr), span))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        self.consume_optional_newline();

        if self.match_kind(&TokenKind::Indent) {
            while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                stmts.push(self.parse_statement()?);
            }
            self.consume(&TokenKind::Dedent, "expected dedent at end of block")?;
        } else {
            // Single-line body (e.g. `if condition: return 42`)
            stmts.push(self.parse_statement()?);
        }

        Ok(stmts)
    }

    // -------------------------------------------------------------
    // EXPRESSIONS (Pratt Parsing)
    // -------------------------------------------------------------

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_precedence(Precedence::None)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            self.skip_line_continuations_if_operator();
            if precedence >= self.current_precedence() {
                break;
            }
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.current_span();

        if self.match_kind(&TokenKind::Minus) {
            let expr = self.parse_precedence(Precedence::Unary)?;
            let span = start_span.merge(expr.span);
            Ok(Expr::new(ExprKind::Unary { op: UnOp::Neg, expr: Box::new(expr) }, span))
        } else if self.match_kind(&TokenKind::Not) {
            let expr = self.parse_precedence(Precedence::Unary)?;
            let span = start_span.merge(expr.span);
            Ok(Expr::new(ExprKind::Unary { op: UnOp::Not, expr: Box::new(expr) }, span))
        } else if self.match_kind(&TokenKind::LParen) {
            // Check for lambda `(a, b) => expr` or simple grouping `(expr)`
            if self.is_lambda_ahead() {
                self.parse_lambda_after_lparen(start_span)
            } else {
                let expr = self.parse_expr()?;
                self.consume(&TokenKind::RParen, "expected `)` after expression")?;
                Ok(expr)
            }
        } else if self.match_kind(&TokenKind::LBracket) {
            self.parse_list_literal(start_span)
        } else if self.match_kind(&TokenKind::LBrace) {
            self.parse_map_literal(start_span)
        } else if self.match_kind(&TokenKind::If) {
            self.parse_if_expr(start_span)
        } else if self.match_kind(&TokenKind::SelfKw) {
            Ok(Expr::new(ExprKind::Identifier("self".to_string()), start_span))
        } else if self.check_ident() {
            let ident = self.consume_ident()?;
            Ok(Expr::new(ExprKind::Identifier(ident), start_span))
        } else if let Some(lit) = self.match_literal() {
            Ok(Expr::new(ExprKind::Literal(lit), start_span))
        } else {
            Err(ParseError::UnexpectedToken(self.peek().kind.clone(), start_span))
        }
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, ParseError> {
        let op_tok = self.advance();
        let op_span = op_tok.span;

        match op_tok.kind {
            TokenKind::LParen => {
                // Function call
                let args = self.parse_call_args()?;
                self.consume(&TokenKind::RParen, "expected `)` after arguments")?;
                let span = left.span.merge(self.previous_span());
                Ok(Expr::new(ExprKind::Call { callee: Box::new(left), args }, span))
            }
            TokenKind::LBracket => {
                // Index access `obj[index]`
                let index = self.parse_expr()?;
                self.consume(&TokenKind::RBracket, "expected `]` after index")?;
                let span = left.span.merge(self.previous_span());
                Ok(Expr::new(ExprKind::Index { object: Box::new(left), index: Box::new(index) }, span))
            }
            TokenKind::Dot => {
                // Member access `obj.field`
                let member = self.consume_ident()?;
                let span = left.span.merge(self.previous_span());
                Ok(Expr::new(ExprKind::MemberAccess { object: Box::new(left), member, is_safe: false }, span))
            }
            TokenKind::SafeDot => {
                // Safe navigation `obj?.field`
                let member = self.consume_ident()?;
                let span = left.span.merge(self.previous_span());
                Ok(Expr::new(ExprKind::MemberAccess { object: Box::new(left), member, is_safe: true }, span))
            }
            TokenKind::Question => {
                // Try operator `expr?`
                let span = left.span.merge(op_span);
                Ok(Expr::new(ExprKind::Try(Box::new(left)), span))
            }
            TokenKind::Pipeline => {
                // `a |> f(b)`
                let right = self.parse_precedence(Precedence::Pipeline)?;
                let span = left.span.merge(right.span);
                Ok(Expr::new(ExprKind::Binary { op: BinOp::Pipeline, left: Box::new(left), right: Box::new(right) }, span))
            }
            TokenKind::DotDot => {
                let right = self.parse_precedence(Precedence::Range)?;
                let span = left.span.merge(right.span);
                Ok(Expr::new(ExprKind::Range { start: Box::new(left), end: Box::new(right), inclusive: false }, span))
            }
            TokenKind::DotDotEq => {
                let right = self.parse_precedence(Precedence::Range)?;
                let span = left.span.merge(right.span);
                Ok(Expr::new(ExprKind::Range { start: Box::new(left), end: Box::new(right), inclusive: true }, span))
            }
            tok_kind => {
                let bin_op = token_to_bin_op(&tok_kind)
                    .ok_or_else(|| ParseError::UnexpectedToken(tok_kind.clone(), op_span))?;
                let prec = token_precedence(&tok_kind);
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                Ok(Expr::new(ExprKind::Binary { op: bin_op, left: Box::new(left), right: Box::new(right) }, span))
            }
        }
    }

    fn current_precedence(&self) -> Precedence {
        if self.is_at_end() {
            return Precedence::None;
        }
        token_precedence(&self.peek().kind)
    }

    fn is_lambda_ahead(&self) -> bool {
        let mut idx = self.cursor;
        let mut depth = 0;
        while idx < self.tokens.len() {
            if self.tokens[idx].kind == TokenKind::LParen {
                depth += 1;
            } else if self.tokens[idx].kind == TokenKind::RParen {
                if depth == 0 {
                    return idx + 1 < self.tokens.len() && self.tokens[idx + 1].kind == TokenKind::FatArrow;
                }
                depth -= 1;
            }
            if depth == 0 && (self.tokens[idx].kind == TokenKind::Newline || self.tokens[idx].kind == TokenKind::Eof) {
                return false;
            }
            idx += 1;
        }
        false
    }

    fn parse_lambda_after_lparen(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let params = self.parse_param_list()?;
        self.consume(&TokenKind::RParen, "expected `)` after lambda parameters")?;
        self.consume(&TokenKind::FatArrow, "expected `=>` after lambda parameter list")?;
        let body = self.parse_expr()?;
        let span = start_span.merge(body.span);
        Ok(Expr::new(ExprKind::Lambda { params, body: Box::new(body) }, span))
    }

    fn parse_if_expr(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let condition = self.parse_expr()?;
        self.consume(&TokenKind::Colon, "expected `:` after if expression condition")?;
        let then_branch = self.parse_expr()?;
        self.consume(&TokenKind::Else, "expected `else` in if expression")?;
        self.consume(&TokenKind::Colon, "expected `:` after else in if expression")?;
        let else_branch = self.parse_expr()?;
        let span = start_span.merge(else_branch.span);
        Ok(Expr::new(
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Some(Box::new(else_branch)),
            },
            span,
        ))
    }

    fn parse_list_literal(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let mut items = Vec::new();
        if !self.check(&TokenKind::RBracket) {
            loop {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                items.push(self.parse_expr()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(&TokenKind::RBracket, "expected `]` to close list")?;
        let span = start_span.merge(self.previous_span());
        Ok(Expr::new(ExprKind::List(items), span))
    }

    fn parse_map_literal(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let mut entries = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                let key = self.parse_expr()?;
                self.consume(&TokenKind::Colon, "expected `:` between key and value in map")?;
                let val = self.parse_expr()?;
                entries.push((key, val));
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(&TokenKind::RBrace, "expected `}` to close map")?;
        let span = start_span.merge(self.previous_span());
        Ok(Expr::new(ExprKind::Map(entries), span))
    }

    fn parse_call_args(&mut self) -> Result<Vec<CallArg>, ParseError> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                let arg_start = self.current_span();
                let expr = self.parse_expr()?;

                // Check for named argument `name=value`
                if let ExprKind::Identifier(ref name) = expr.kind {
                    if self.match_kind(&TokenKind::Eq) {
                        let val = self.parse_expr()?;
                        let span = arg_start.merge(val.span);
                        args.push(CallArg {
                            name: Some(name.clone()),
                            value: val,
                            span,
                        });
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                        continue;
                    }
                }

                let span = expr.span;
                args.push(CallArg {
                    name: None,
                    value: expr,
                    span,
                });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if self.match_kind(&TokenKind::Newline) {
                    continue;
                }
                let start_span = self.current_span();
                let name = if self.match_kind(&TokenKind::SelfKw) {
                    "self".to_string()
                } else {
                    self.consume_ident()?
                };

                let type_ann = if self.match_kind(&TokenKind::Colon) {
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };

                let default_val = if self.match_kind(&TokenKind::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };

                let span = start_span.merge(self.previous_span());
                params.push(Param {
                    name,
                    type_ann,
                    default_val,
                    span,
                });

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        Ok(params)
    }

    fn parse_type_ann(&mut self) -> Result<TypeAnn, ParseError> {
        let start_span = self.current_span();
        let name = self.consume_ident()?;
        let mut generics = Vec::new();

        if self.match_kind(&TokenKind::LBracket) {
            loop {
                generics.push(self.parse_type_ann()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.consume(&TokenKind::RBracket, "expected `]` after type parameters")?;
        }

        let span = start_span.merge(self.previous_span());
        Ok(TypeAnn {
            name,
            generics,
            span,
        })
    }

    // -------------------------------------------------------------
    // HELPER FUNCTIONS
    // -------------------------------------------------------------

    fn consume_ident(&mut self) -> Result<String, ParseError> {
        let span = self.current_span();
        if let TokenKind::Ident(s) = &self.peek().kind {
            let val = s.clone();
            self.advance();
            Ok(val)
        } else {
            Err(ParseError::ExpectedToken(
                "identifier".to_string(),
                self.peek().kind.clone(),
                span,
            ))
        }
    }

    fn check_ident(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Ident(_))
    }

    fn match_literal(&mut self) -> Option<Literal> {
        let lit = match &self.peek().kind {
            TokenKind::Int(v) => Some(Literal::Int(*v)),
            TokenKind::Float(v) => Some(Literal::Float(*v)),
            TokenKind::String(v) => Some(Literal::String(v.clone())),
            TokenKind::Bool(v) => Some(Literal::Bool(*v)),
            TokenKind::None => Some(Literal::None),
            _ => None,
        };
        if lit.is_some() {
            self.advance();
        }
        lit
    }

    fn consume(&mut self, expected: &TokenKind, msg: &str) -> Result<Token, ParseError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            Err(ParseError::ExpectedToken(
                msg.to_string(),
                self.peek().kind.clone(),
                self.current_span(),
            ))
        }
    }

    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            *kind == TokenKind::Eof
        } else {
            self.peek().kind == *kind
        }
    }

    fn skip_line_continuations_if_operator(&mut self) {
        let mut i = self.cursor;
        let mut indents = 0;
        while i < self.tokens.len() {
            match self.tokens[i].kind {
                TokenKind::Newline => {
                    i += 1;
                }
                TokenKind::Indent => {
                    indents += 1;
                    i += 1;
                }
                _ => break,
            }
        }
        if i < self.tokens.len() && is_infix_operator(&self.tokens[i].kind) {
            self.cursor = i;
            self.continuation_indents += indents;
        }
    }

    fn consume_stmt_terminator(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
        while self.continuation_indents > 0 && self.match_kind(&TokenKind::Dedent) {
            self.continuation_indents -= 1;
        }
        while self.match_kind(&TokenKind::Newline) {}
    }

    fn consume_optional_newline(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.cursor += 1;
        }
        self.previous()
    }

    fn peek(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else {
            self.tokens.last().unwrap()
        }
    }

    fn previous(&self) -> Token {
        self.tokens[self.cursor - 1].clone()
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn previous_span(&self) -> Span {
        self.previous().span
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len() || self.peek().kind == TokenKind::Eof
    }
}

fn expr_to_assign_target(expr: Expr) -> Result<AssignTarget, ParseError> {
    match expr.kind {
        ExprKind::Identifier(name) => Ok(AssignTarget::Variable(name)),
        ExprKind::MemberAccess { object, member, .. } => Ok(AssignTarget::Member { object, member }),
        ExprKind::Index { object, index } => Ok(AssignTarget::Index { object, index }),
        _ => Err(ParseError::InvalidAssignmentTarget(expr.span)),
    }
}

fn token_precedence(kind: &TokenKind) -> Precedence {
    match kind {
        TokenKind::Pipeline => Precedence::Pipeline,
        TokenKind::NullCoalesce => Precedence::NullCoalesce,
        TokenKind::Or => Precedence::Or,
        TokenKind::And => Precedence::And,
        TokenKind::EqEq | TokenKind::NotEq => Precedence::Equality,
        TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Precedence::Relational,
        TokenKind::DotDot | TokenKind::DotDotEq => Precedence::Range,
        TokenKind::Plus | TokenKind::Minus => Precedence::Term,
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
        TokenKind::Power => Precedence::Power,
        TokenKind::LParen
        | TokenKind::LBracket
        | TokenKind::Dot
        | TokenKind::SafeDot
        | TokenKind::Question => Precedence::Postfix,
        _ => Precedence::None,
    }
}

fn token_to_bin_op(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::Plus => Some(BinOp::Add),
        TokenKind::Minus => Some(BinOp::Sub),
        TokenKind::Star => Some(BinOp::Mul),
        TokenKind::Slash => Some(BinOp::Div),
        TokenKind::Percent => Some(BinOp::Mod),
        TokenKind::Power => Some(BinOp::Pow),
        TokenKind::EqEq => Some(BinOp::Eq),
        TokenKind::NotEq => Some(BinOp::NotEq),
        TokenKind::Lt => Some(BinOp::Lt),
        TokenKind::LtEq => Some(BinOp::LtEq),
        TokenKind::Gt => Some(BinOp::Gt),
        TokenKind::GtEq => Some(BinOp::GtEq),
        TokenKind::And => Some(BinOp::And),
        TokenKind::Or => Some(BinOp::Or),
        TokenKind::Pipeline => Some(BinOp::Pipeline),
        TokenKind::NullCoalesce => Some(BinOp::NullCoalesce),
        _ => None,
    }
}

fn is_infix_operator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Pipeline
            | TokenKind::NullCoalesce
            | TokenKind::Or
            | TokenKind::And
            | TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::LtEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::DotDot
            | TokenKind::DotDotEq
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Power
            | TokenKind::Dot
            | TokenKind::SafeDot
    )
}
