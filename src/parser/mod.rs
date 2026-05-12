use crate::parser::{
    ast::{
        Binary, BinaryOp, Declaration, Spanned, SpannedDeclaration, SpannedExpression, Statement,
    },
    lexing::{Token, TokenType},
};

pub mod ast;
pub mod lexing;

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("unexpected token encountered")]
    UnexpectedToken {
        expected: Option<TokenType>,
        found: Option<Token>,
    },

    #[error("Expected expression")]
    ExpectedExpression,

    #[error("Expected identifier")]
    ExpectedIdentifier,

    #[error("Unexpected end of file")]
    UnexpectedEof,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

type ParseResult = Result<Option<ast::SpannedExpression>, ParseError>;
type ParseStatementResult = Result<Option<ast::SpannedStatement>, ParseError>;
type ParseDeclarationResult = Result<Option<ast::SpannedDeclaration>, ParseError>;

pub trait HasSpan {
    fn start(&self) -> usize;
    fn end(&self) -> usize;
}

impl<T> HasSpan for Spanned<T> {
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
}

impl HasSpan for Token {
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
}

impl<T> Spanned<T> {
    pub fn encapsulating(l: &impl HasSpan, r: &impl HasSpan, node: T) -> Spanned<T> {
        Self {
            start: l.start(),
            end: r.end(),
            node,
        }
    }
}

/// This function takes a parse result and pulls the expression out of it.
/// If there is no expression contained within, it yields an error.
fn require_expr(res: ParseResult) -> Result<ast::SpannedExpression, ParseError> {
    match res {
        Ok(Some(expr)) => Ok(expr),
        Ok(None) => Err(ParseError::ExpectedExpression),
        Err(err) => Err(err),
    }
}

fn require_identifier<'a>(tok: &'a Option<&Token>) -> Result<&'a String, ParseError> {
    let tok = tok.ok_or(ParseError::ExpectedIdentifier)?;
    if let TokenType::Identifier(str) = &tok.token_type {
        Ok(str)
    } else {
        Err(ParseError::ExpectedExpression)
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn from_str(str: &str) -> Self {
        let tokens = lexing::Scanner::new(str).collect();
        Self::new(tokens)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.current);
        self.current += 1;
        tok
    }

    fn try_advance<T, F>(&mut self, f: F) -> Option<T>
    where
        F: Fn(&Token) -> Option<T>,
    {
        let out = self.tokens.get(self.current).and_then(f);
        if out.is_some() {
            self.current += 1;
        }
        out
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn peek_type(&self) -> Option<&TokenType> {
        self.peek().map(|t| &t.token_type)
    }

    fn expect_token(&mut self, expected: TokenType) -> Result<&Token, ParseError> {
        let maybe_token = self.advance();
        if let Some(tok) = maybe_token
            && tok.token_type == expected
        {
            Ok(tok)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: Some(expected),
                found: maybe_token.cloned(),
            })
        }
    }

    // fn try_unary<Next, Op>(&mut self, mut next_parse_fn: Next, op: Op) -> ParseResult
    // where
    //     Next: FnMut(&mut Self) -> ParseResult,
    //     Op: Fn(&TokenType, ) -> SpannedExpression
    // {}

    fn try_binary<P, Op>(&mut self, mut parse_fn: P, op: Op) -> ParseResult
    where
        P: FnMut(&mut Self) -> ParseResult,
        Op: Fn(&TokenType) -> Option<BinaryOp>,
    {
        let mut res = parse_fn(self);
        while let Ok(Some(expr)) = res {
            if let Some(tok) = self.peek_type()
                && let Some(operator) = op(tok)
            {
                self.advance();
                let right = require_expr(parse_fn(self))?;
                let start = expr.start;
                let end = right.end;
                let binary = Binary {
                    left: Box::new(expr),
                    operator,
                    right: Box::new(right),
                };
                res = Ok(Some(ast::Spanned {
                    start,
                    end,
                    node: binary.into(),
                }))
            } else {
                res = Ok(Some(expr));
                break;
            }
        }
        res
    }

    fn synchronize(&mut self) {
        loop {
            let prev = self.advance().map(|t| &t.token_type);

            if prev == Some(&TokenType::Semicolon) {
                break;
            }

            let cur = self.peek_type();
            match cur {
                Some(TokenType::Class)
                | Some(TokenType::Fun)
                | Some(TokenType::Var)
                | Some(TokenType::For)
                | Some(TokenType::If)
                | Some(TokenType::While)
                | Some(TokenType::Print)
                | Some(TokenType::Return) => {
                    break;
                }
                _ => {}
            }
        }
    }

    pub fn parse_declaration(&mut self) -> ParseDeclarationResult {
        // What should we actually do about this error? Things are complicated.
        // We can still continue to parse useful AST fragments, so that we can do LSP
        // things or continue to find more syntax errors. However we can't surface the
        // parse error if we continue on.
        //
        // This makes me question my use of the result type. We actually want to report
        // the parsed AST _and_ a set of errors, where an empty set means successful
        // parsing.
        //
        // And then all of the errors should point to spans in the code.
        //
        // Similarly, I'm wondering if it was a mistake to even bother lexing and parsing
        // in "streams".
        //
        // I also wonder what should happen when lexing fails. Is it better to emit the
        // list of valid tokens alongside a set of errors (with spans)? When parsing,
        // the parser will run with the (probably invalid) tokens and report the errors
        // from lexing along with its additional parse errors?

        self.parse_declaration_inner()
        // loop {
        //     let res = self.parse_declaration_inner();
        //     if res.is_err() {
        //         self.synchronize();
        //     } else {
        //         return res;
        //     }
        // }
    }

    pub fn parse_declaration_inner(&mut self) -> ParseDeclarationResult {
        let Some(start_tok) = self.peek() else {
            return Ok(None);
        };

        match &start_tok.token_type {
            TokenType::Var => {
                let start = start_tok.start;
                self.advance();
                let identifier_token = self.advance();
                let identifier = require_identifier(&identifier_token)?.to_owned();

                let next_tok = self.advance().ok_or(ParseError::UnexpectedEof)?;
                match next_tok.token_type {
                    TokenType::Equal => {
                        let expr = self
                            .parse_expression()?
                            .ok_or(ParseError::ExpectedExpression)?;

                        let semi = self.expect_token(TokenType::Semicolon)?;
                        let end = semi.end;
                        Ok(Some(SpannedDeclaration {
                            start,
                            end,
                            node: Declaration::Var {
                                identifier,
                                initial: Some(expr),
                            },
                        }))
                    }
                    TokenType::Semicolon => {
                        let end = next_tok.end;
                        Ok(Some(SpannedDeclaration {
                            start,
                            end,
                            node: Declaration::Var {
                                identifier,
                                initial: None,
                            },
                        }))
                    }
                    _ => Err(ParseError::UnexpectedToken {
                        expected: None,
                        found: Some(next_tok.clone()),
                    }),
                }
            }
            _ => {
                let stmt = self.parse_statement()?.map(|stmt| {
                    let start = stmt.start;
                    let end = stmt.end;
                    SpannedDeclaration {
                        start,
                        end,
                        node: Declaration::Statement(stmt),
                    }
                });
                Ok(stmt)
            }
        }
    }

    pub fn parse_statement(&mut self) -> ParseStatementResult {
        let Some(start_tok) = self.peek() else {
            return Ok(None);
        };
        match &start_tok.token_type {
            TokenType::Print => {
                let start = start_tok.start;
                self.advance();
                let expr = require_expr(self.parse_expression())?;
                let semi = self.expect_token(TokenType::Semicolon)?;
                let end = semi.end;
                Ok(Some(Spanned {
                    start,
                    end,
                    node: Statement::Print(expr),
                }))
            }
            _ => {
                let expr = require_expr(self.parse_expression())?;
                let start = expr.start;
                let semi = self.expect_token(TokenType::Semicolon)?;
                let end = semi.end;
                Ok(Some(Spanned {
                    start,
                    end,
                    node: Statement::Expression(expr),
                }))
            }
        }
    }

    pub fn parse_expression(&mut self) -> ParseResult {
        self.parse_equality()
    }

    pub fn parse_equality(&mut self) -> ParseResult {
        self.try_binary(Self::parse_comparison, |tok_type| match tok_type {
            TokenType::BangEqual => Some(BinaryOp::NotEqual),
            TokenType::EqualEqual => Some(BinaryOp::Equal),
            _ => None,
        })
    }

    pub fn parse_comparison(&mut self) -> ParseResult {
        self.try_binary(Self::parse_term, |tok_type| match tok_type {
            TokenType::Greater => Some(BinaryOp::Greater),
            TokenType::GreaterEqual => Some(BinaryOp::GreaterEqual),
            TokenType::Less => Some(BinaryOp::Less),
            TokenType::LessEqual => Some(BinaryOp::LessEqual),
            _ => None,
        })
    }
    pub fn parse_term(&mut self) -> ParseResult {
        self.try_binary(Self::parse_factor, |tok_type| match tok_type {
            TokenType::Plus => Some(BinaryOp::Add),
            TokenType::Minus => Some(BinaryOp::Subtract),
            _ => None,
        })
    }
    pub fn parse_factor(&mut self) -> ParseResult {
        self.try_binary(Self::parse_unary, |tok_type| match tok_type {
            TokenType::Slash => Some(BinaryOp::Divide),
            TokenType::Star => Some(BinaryOp::Multiply),
            _ => None,
        })
    }

    pub fn parse_unary(&mut self) -> ParseResult {
        let unary = self.try_advance(|tok| {
            let build: Option<fn(Box<SpannedExpression>) -> ast::Unary> = match tok.token_type {
                TokenType::Bang => Some(ast::Unary::Not),
                TokenType::Minus => Some(ast::Unary::Negate),
                _ => None,
            };
            build.map(|b| (tok.start, b))
        });
        if let Some((start, build)) = unary {
            let inner = require_expr(self.parse_unary())?;
            let end = inner.end;
            let node = ast::Expression::Unary(build(Box::new(inner)));
            Ok(Some(SpannedExpression { start, end, node }))
        } else {
            self.parse_primary()
        }
    }

    pub fn parse_primary(&mut self) -> ParseResult {
        if let Some(next) = self.advance() {
            let Token { start, end, .. } = *next;
            let wrap = |node: ast::Expression| SpannedExpression { start, end, node };

            Ok(Some(match &next.token_type {
                TokenType::True => wrap(true.into()),
                TokenType::False => wrap(false.into()),
                TokenType::Nil => wrap(ast::Literal::Nil.into()),
                TokenType::Number(num) => wrap(ast::Literal::Number(*num).into()),
                TokenType::String(str) => wrap(ast::Literal::String(str.clone()).into()),
                TokenType::LeftParen => {
                    // should this be primary or expression?
                    // the book appeared to indicate just expression
                    // update: I think it must be expression so it can step
                    // through the precedence ladder again?
                    let inner = require_expr(self.parse_expression())?;
                    self.expect_token(TokenType::RightParen)?;
                    inner
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: None,
                    found: Some(next.clone()),
                })?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{
        Parser,
        ast::{Binary, BinaryOp, Expression, Literal, SpannedExpression, Unary},
    };

    fn parse(str: &str) -> SpannedExpression {
        Parser::from_str(str)
            .parse_expression()
            .unwrap()
            .unwrap()
            .strip_spans()
    }

    #[test]
    fn parse_empty() {
        let mut parser = Parser::from_str("");
        let expr = parser.parse_expression();
        assert!(expr.unwrap().is_none());
    }

    #[test]
    fn parse_unary() {
        let mut parser = Parser::from_str("!(-10)");
        let expr1 = parser.parse_unary().unwrap().unwrap().strip_spans();
        assert_eq!(
            expr1,
            Unary::not(Unary::negate(Literal::Number(10.0).into()).into()).into()
        )
    }

    #[test]
    fn parse_not_bool() {
        let expr = parse("!!false");
        assert_eq!(expr, Unary::not(Unary::not(false.into()).into()).into())
    }

    #[test]
    fn parse_factor() {
        assert_eq!(
            parse("10 / 10"),
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
            parse("-(\"hello\" >= 4)"),
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
            parse("-10 - 4 - - 12 / 64 * 9 + 2"),
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
