use crate::parser::{
    ast::{
        Binary, BinaryOp, Declaration, DeclarationKind, Expression, ExpressionKind, Statement,
        StatementKind,
    },
    diagnostic::Diagnostic,
    lexing::scan,
    span::{Span, Spanned},
    token::{Token, TokenKind},
};

pub mod ast;
pub mod diagnostic;
pub mod lexing;
pub mod span;
pub mod token;

struct Parser {
    tokens: Vec<Token>,
    decls: Vec<Declaration>,
    diagnostics: Vec<Diagnostic>,
    current: usize,
}

type ParseExpressionResult = Result<Expression, Diagnostic>;
type ParseStatementResult = Result<Statement, Diagnostic>;
type ParseDeclarationResult = Result<Declaration, Diagnostic>;

impl Parser {
    pub fn new(tokens: Vec<Token>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            tokens,
            diagnostics,
            current: 0,
            decls: Vec::new(),
        }
    }

    /// Retrieve the next token to consider (by copying it, unfortunately), and advance
    /// the pointer to the current token
    fn advance(&mut self) -> Result<Token, Diagnostic> {
        let res = self
            .tokens
            .get(self.current)
            .cloned()
            .ok_or_else(|| Diagnostic::error(self.current_span(), "unexpected end of input"));
        if res.is_ok() {
            self.current += 1;
        }
        res
    }

    /// Get the span of the current token - or, if we are at the end of tokens,
    /// the last token. Or, if there are no tokens, a span for a hypothetical first token
    fn current_span(&self) -> Span {
        self.tokens
            .get(self.current)
            .map(|tok| tok.span)
            .unwrap_or_else(|| {
                let mut pos = 0;
                if let Some(tok) = self.tokens.last() {
                    pos = tok.span.start + tok.span.end
                }
                Span {
                    start: pos,
                    end: pos,
                }
            })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn peek_type(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.node)
    }

    fn expect_token(&mut self, expected: TokenKind) -> Result<Span, Diagnostic> {
        let tok = self.advance()?;
        if tok.node == expected {
            Ok(tok.span)
        } else {
            let msg = format!("expected token {:?}, found {:?}", expected, tok.node);
            Err(Diagnostic::error(&tok, msg))
        }
    }

    fn try_binary<P, Op>(&mut self, mut parse_fn: P, op: Op) -> ParseExpressionResult
    where
        P: FnMut(&mut Self) -> ParseExpressionResult,
        Op: Fn(&TokenKind) -> Option<BinaryOp>,
    {
        let mut res = parse_fn(self);
        while let Ok(expr) = res {
            if let Some(tok) = self.peek_type()
                && let Some(operator) = op(tok)
            {
                self.advance()?;
                let right = parse_fn(self)?;
                let binary = Binary {
                    left: Box::new(expr),
                    operator,
                    right: Box::new(right),
                };

                res = Ok(binary.into())
            } else {
                res = Ok(expr);
                break;
            }
        }
        res
    }

    fn expect_identifier(&mut self) -> Result<(Span, String), Diagnostic> {
        let tok = self.advance()?;
        if let TokenKind::Identifier(s) = &tok.node {
            Ok((tok.span, s.clone()))
        } else {
            Err(Diagnostic::error(tok.span, "expected an identifier"))
        }
    }

    fn synchronize(&mut self) {
        loop {
            match self.peek_type() {
                None => break,
                Some(TokenKind::Semicolon) => {
                    self.current += 1;
                    break;
                }
                Some(
                    TokenKind::Class
                    | TokenKind::Fun
                    | TokenKind::Var
                    | TokenKind::For
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::Print
                    | TokenKind::Return,
                ) => break,
                _ => {
                    self.current += 1;
                }
            }
        }
    }

    pub fn parse_declaration(&mut self) -> ParseDeclarationResult {
        match self.peek_type() {
            Some(TokenKind::Var) => {
                let start = self.advance()?.span.start;

                let (_, identifier) = self.expect_identifier()?;

                let next_tok = self.advance()?;
                match next_tok.node {
                    TokenKind::Equal => {
                        let expr = self.parse_expression()?;
                        let semi = self.expect_token(TokenKind::Semicolon)?;
                        let end = semi.end;
                        Ok(Declaration {
                            span: Span { start, end },
                            node: DeclarationKind::Var {
                                identifier,
                                initial: Some(expr),
                            },
                        })
                    }
                    TokenKind::Semicolon => {
                        let end = next_tok.span.end;
                        Ok(Declaration {
                            span: Span { start, end },
                            node: DeclarationKind::Var {
                                identifier,
                                initial: None,
                            },
                        })
                    }
                    _ => Err(Diagnostic::error(
                        &next_tok,
                        "expected '=' or ';' when parsing declaration",
                    )),
                }
            }
            _ => self.parse_statement().map(|stmt| Declaration {
                span: stmt.span,
                node: DeclarationKind::Statement(stmt),
            }),
        }
    }

    pub fn parse_statement(&mut self) -> ParseStatementResult {
        match self.peek_type() {
            Some(TokenKind::Print) => {
                let start = self.advance()?.span.start;
                let expr = self.parse_expression()?;
                let semi = self.expect_token(TokenKind::Semicolon)?;
                let end = semi.end;
                Ok(Spanned {
                    span: Span { start, end },
                    node: StatementKind::Print(expr),
                })
            }
            Some(TokenKind::LeftBrace) => {
                let start = self.advance()?.span.start;
                let mut decls = Vec::<Declaration>::new();
                loop {
                    match self.peek_type() {
                        Some(TokenKind::RightBrace) | None => {
                            break;
                        }
                        _ => {
                            decls.push(self.parse_declaration()?);
                        }
                    }
                }

                let semi = self.expect_token(TokenKind::RightBrace)?;
                let end = semi.end;
                Ok(Spanned {
                    span: Span { start, end },
                    node: StatementKind::Block(decls),
                })
            }
            _ => {
                let expr = self.parse_expression()?;
                let expr_span = expr.span;
                let semi = self.expect_token(TokenKind::Semicolon)?;
                Ok(Statement::encapsulating(
                    expr_span,
                    semi,
                    StatementKind::Expression(expr),
                ))
            }
        }
    }

    pub fn parse_expression(&mut self) -> ParseExpressionResult {
        self.parse_assignment()
    }

    pub fn parse_assignment(&mut self) -> ParseExpressionResult {
        let expr = self.parse_equality()?;

        if self.peek_type() == Some(&TokenKind::Equal) {
            self.advance()?;
            let value = self.parse_assignment()?;
            if let ExpressionKind::Variable(ident) = expr.node {
                Ok(Expression::encapsulating(
                    expr.span,
                    value.span,
                    ExpressionKind::Assign(ident, Box::new(value)),
                ))
            } else {
                Err(Diagnostic::error(expr.span, "Invalid assignment target"))
            }
        } else {
            Ok(expr)
        }
    }

    pub fn parse_equality(&mut self) -> ParseExpressionResult {
        self.try_binary(Self::parse_comparison, |tok_type| match tok_type {
            TokenKind::BangEqual => Some(BinaryOp::NotEqual),
            TokenKind::EqualEqual => Some(BinaryOp::Equal),
            _ => None,
        })
    }

    pub fn parse_comparison(&mut self) -> ParseExpressionResult {
        self.try_binary(Self::parse_term, |tok_type| match tok_type {
            TokenKind::Greater => Some(BinaryOp::Greater),
            TokenKind::GreaterEqual => Some(BinaryOp::GreaterEqual),
            TokenKind::Less => Some(BinaryOp::Less),
            TokenKind::LessEqual => Some(BinaryOp::LessEqual),
            _ => None,
        })
    }
    pub fn parse_term(&mut self) -> ParseExpressionResult {
        self.try_binary(Self::parse_factor, |tok_type| match tok_type {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Subtract),
            _ => None,
        })
    }
    pub fn parse_factor(&mut self) -> ParseExpressionResult {
        self.try_binary(Self::parse_unary, |tok_type| match tok_type {
            TokenKind::Slash => Some(BinaryOp::Divide),
            TokenKind::Star => Some(BinaryOp::Multiply),
            _ => None,
        })
    }

    pub fn parse_unary(&mut self) -> ParseExpressionResult {
        let build: fn(Box<Expression>) -> ast::Unary = match self.peek_type() {
            Some(TokenKind::Bang) => ast::Unary::Not,
            Some(TokenKind::Minus) => ast::Unary::Negate,
            _ => return self.parse_primary(),
        };
        let start = self.advance()?.span.start;
        let inner = self.parse_unary()?;
        let end = inner.span.end;
        Ok(Expression {
            span: Span { start, end },
            node: ast::ExpressionKind::Unary(build(Box::new(inner))),
        })
    }

    pub fn parse_primary(&mut self) -> ParseExpressionResult {
        let next = self.advance()?;
        let wrap = |node: ast::ExpressionKind| {
            Ok(Expression {
                span: next.span,
                node,
            })
        };
        match next.node {
            TokenKind::True => wrap(true.into()),
            TokenKind::False => wrap(false.into()),
            TokenKind::Nil => wrap(ast::Literal::Nil.into()),
            TokenKind::Number(num) => wrap(ast::Literal::Number(num).into()),
            TokenKind::String(str) => wrap(ast::Literal::String(str).into()),
            TokenKind::LeftParen => {
                let inner = self.parse_expression()?;
                let right_paren = self.expect_token(TokenKind::RightParen)?;
                Ok(Expression::encapsulating(&next, right_paren, inner.node))
            }
            TokenKind::Identifier(ident) => wrap(ast::ExpressionKind::Variable(ident)),
            _ => Err(Diagnostic::error(&next, "unexpected token")),
        }
    }

    fn parse(&mut self) {
        loop {
            if self.peek().is_none() {
                break;
            }

            match self.parse_declaration() {
                Ok(decl) => self.decls.push(decl),
                Err(diag) => {
                    self.diagnostics.push(diag);
                    self.synchronize();
                }
            }
        }
    }
}

pub type ParseResult = (Vec<Declaration>, Vec<Diagnostic>);
pub fn parse_str(source: &str) -> ParseResult {
    let (tokens, diag) = scan(source);
    let mut parser = Parser::new(tokens, diag);
    parser.parse();
    (parser.decls, parser.diagnostics)
}

#[cfg(test)]
mod tests {
    use crate::parser::{
        Parser,
        ast::{Binary, BinaryOp, Expression, Literal, Unary},
        lexing::scan,
        parse_str,
    };

    fn parser_from_str(str: &str) -> Parser {
        let (tokens, diagnostics) = scan(str);
        Parser::new(tokens, diagnostics)
    }

    fn parse_expr(src: &str) -> Expression {
        let mut parser = parser_from_str(src);
        parser.parse_expression().unwrap()
    }

    #[test]
    fn parse_empty() {
        let (decls, diag) = parse_str("");
        assert_eq!(decls.len(), 0);
        assert_eq!(diag.len(), 0);
    }

    #[test]
    fn parse_unary() {
        let mut parser = parser_from_str("!(-10)");
        let expr1 = parser.parse_unary().unwrap().strip_spans();
        assert_eq!(
            expr1,
            Unary::not(Unary::negate(Literal::Number(10.0).into()).into()).into()
        )
    }

    #[test]
    fn parse_not_bool() {
        let expr = parse_expr("!!false");
        assert_eq!(expr, Unary::not(Unary::not(false.into()).into()).into())
    }

    #[test]
    fn parse_factor() {
        assert_eq!(
            parse_expr("10 / 10"),
            Binary {
                left: Box::new(Literal::Number(10.0).into()),
                operator: BinaryOp::Divide,
                right: Box::new(Literal::Number(10.0).into()),
            }
            .into()
        );
    }

    #[test]
    fn parse_complex() {
        assert_eq!(
            parse_expr("-(\"hello\" >= 4)"),
            Unary::negate(
                Binary::new(
                    Literal::String("hello".to_owned()).into(),
                    BinaryOp::GreaterEqual,
                    Literal::Number(4.0).into()
                )
                .into()
            )
            .into()
        );

        assert_eq!(
            parse_expr("-10 - 4 - - 12 / 64 * 9 + 2"),
            Binary::new(
                Binary::new(
                    Binary::new(
                        Unary::negate(Literal::Number(10.0).into()).into(),
                        BinaryOp::Subtract,
                        Literal::Number(4.0).into()
                    )
                    .into(),
                    BinaryOp::Subtract,
                    Binary::new(
                        Binary::new(
                            Unary::negate(Literal::Number(12.0).into()).into(),
                            BinaryOp::Divide,
                            Literal::Number(64.0).into()
                        )
                        .into(),
                        BinaryOp::Multiply,
                        Literal::Number(9.0).into()
                    )
                    .into(),
                )
                .into(),
                BinaryOp::Add,
                Literal::Number(2.0).into()
            )
            .into()
        );
    }
}
