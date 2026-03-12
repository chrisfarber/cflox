#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    True,
    False,
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub start: usize,
    pub end: usize,
    pub node: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
}

pub type SpannedExpression = Spanned<Expression>;

impl From<Literal> for Expression {
    fn from(literal: Literal) -> Expression {
        Expression::Literal(literal)
    }
}

impl From<bool> for Expression {
    fn from(b: bool) -> Expression {
        match b {
            true => Literal::True,
            false => Literal::False,
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unary {
    Negate(Box<SpannedExpression>),
    Not(Box<SpannedExpression>),
}

impl Unary {
    pub fn negate(expr: SpannedExpression) -> Self {
        Self::Negate(Box::new(expr))
    }

    pub fn not(expr: SpannedExpression) -> Self {
        Self::Not(Box::new(expr))
    }
}

impl From<Unary> for Expression {
    fn from(u: Unary) -> Expression {
        Expression::Unary(u)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Divide,
    Multiply,
    Add,
    Subtract,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub left: Box<SpannedExpression>,
    pub operator: BinaryOp,
    pub right: Box<SpannedExpression>,
}

impl Binary {
    pub fn new(left: SpannedExpression, operator: BinaryOp, right: SpannedExpression) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operator,
        }
    }
}

impl From<Binary> for Expression {
    fn from(b: Binary) -> Expression {
        Expression::Binary(b)
    }
}
