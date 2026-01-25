use crate::parser::lexing::{Token, TokenType};

pub enum Literal {
    Number(f64),
    String(String),
    True,
    False,
    Nil,
}

pub enum Expression {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
}

pub enum Unary {
    Negate(Box<Expression>),
    Not(Box<Expression>),
}

struct Binary {
    left: Box<Expression>,
    operator: Option<()>,
    right: Box<Expression>,
}
