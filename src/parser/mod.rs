use crate::parser::{
    ast::{Binary, BinaryOp},
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
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

type ParseResult = Result<Option<ast::Expression>, ParseError>;

fn require_expr(res: ParseResult) -> Result<ast::Expression, ParseError> {
    match res {
        Ok(Some(expr)) => Ok(expr),
        Ok(None) => Err(ParseError::ExpectedExpression),
        Err(err) => Err(err),
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
                res = Ok(Some(
                    Binary {
                        left: Box::new(expr),
                        operator,
                        right: Box::new(right),
                    }
                    .into(),
                ));
            } else {
                res = Ok(Some(expr));
                break;
            }
        }
        res
    }

    // Grammar at this point looks like:
    //
    // expression -> equality;
    // equality -> comparison (("!=" | "==") comparison)*;
    // comparison -> term ((">" | ">=" | "<" | "<=") term)*;
    // term -> factor (("-" | "+") factor)*;
    // factor -> unary (("/" | "*") unary)*;
    // unary -> ("!" | "-") unary | primary;
    // primary -> NUMBER | STRING | "true" | "false" | "nil" | "(" expression ")";

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
        match self.peek_type() {
            Some(TokenType::Bang) => {
                self.advance();
                let expr = require_expr(self.parse_unary())?;
                Ok(Some(ast::Unary::not(expr).into()))
            }
            Some(TokenType::Minus) => {
                self.advance();
                let expr = require_expr(self.parse_unary())?;
                Ok(Some(ast::Unary::negate(expr).into()))
            }
            _ => self.parse_primary(),
        }
    }

    pub fn parse_primary(&mut self) -> ParseResult {
        if let Some(next) = self.advance() {
            Ok(Some(match &next.token_type {
                TokenType::True => true.into(),
                TokenType::False => false.into(),
                TokenType::Nil => ast::Literal::Nil.into(),
                TokenType::Number(num) => ast::Literal::Number(*num).into(),
                TokenType::String(str) => ast::Literal::String(str.clone()).into(),
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
        ast::{Binary, BinaryOp, Expression, Literal, Unary},
    };

    fn parse(str: &str) -> Expression {
        Parser::from_str(str).parse_expression().unwrap().unwrap()
    }

    #[test]
    fn parse_unary() {
        let mut parser = Parser::from_str("!(-10)");
        let expr1 = parser.parse_unary().unwrap().unwrap();
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
