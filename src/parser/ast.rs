use crate::parser::node::{Node, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    // TODO should this be Bool(bool) ?
    True,
    False,
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
    Variable(String),
    Assign(String, Box<Expression>),
    Logical(Logical),
    Call(Box<Expression>, Vec<Expression>),
}

pub type Expression = Node<ExpressionKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Expression(Expression),
    Print(Expression),
    Return(Option<Expression>),
    Block(Vec<Declaration>),
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
    },
}

impl From<Vec<Declaration>> for Statement {
    fn from(decls: Vec<Declaration>) -> Statement {
        Statement::new(
            Span {
                start: decls.first().map(|d| d.span.start).unwrap_or(0),
                end: decls.last().map(|d| d.span.end).unwrap_or(0),
            },
            StatementKind::Block(decls),
        )
    }
}

pub type Statement = Node<StatementKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum DeclarationKind {
    Statement(Statement),
    Var {
        identifier: String,
        initial: Option<Expression>,
    },
    Function(Function),
}

pub type Declaration = Node<DeclarationKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub name_span: Span,
    pub parameter_names: Vec<(Span, String)>,
    pub body: Box<Statement>,
}

impl From<Statement> for Declaration {
    fn from(stmt: Statement) -> Declaration {
        Declaration::new(stmt.span, DeclarationKind::Statement(stmt))
    }
}

impl From<Expression> for Declaration {
    fn from(expr: Expression) -> Declaration {
        Declaration::new(
            expr.span,
            DeclarationKind::Statement(Statement::new(expr.span, StatementKind::Expression(expr))),
        )
    }
}

impl From<Literal> for ExpressionKind {
    fn from(literal: Literal) -> ExpressionKind {
        ExpressionKind::Literal(literal)
    }
}

impl From<bool> for ExpressionKind {
    fn from(b: bool) -> ExpressionKind {
        match b {
            true => Literal::True,
            false => Literal::False,
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unary {
    Negate(Box<Expression>),
    Not(Box<Expression>),
}

impl From<Unary> for ExpressionKind {
    fn from(u: Unary) -> ExpressionKind {
        ExpressionKind::Unary(u)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub left: Box<Expression>,
    pub operator: BinaryOp,
    pub right: Box<Expression>,
}

impl From<Binary> for ExpressionKind {
    fn from(b: Binary) -> ExpressionKind {
        ExpressionKind::Binary(b)
    }
}

impl From<Binary> for Expression {
    fn from(b: Binary) -> Expression {
        Expression::encapsulating(b.left.span, b.right.span, b.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOp {
    Or,
    And,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Logical {
    pub left: Box<Expression>,
    pub operator: LogicalOp,
    pub right: Box<Expression>,
}

#[cfg(test)]
mod test_conversions {

    use super::*;

    impl Unary {
        pub fn negate(expr: Expression) -> Self {
            Self::Negate(Box::new(expr))
        }
        pub fn not(expr: Expression) -> Self {
            Self::Not(Box::new(expr))
        }
    }

    impl Binary {
        pub fn new(left: Expression, operator: BinaryOp, right: Expression) -> Self {
            Self {
                left: Box::new(left),
                right: Box::new(right),
                operator,
            }
        }
    }

    impl Expression {
        /// Recursively zeros out all spans, for comparison in tests.
        pub fn strip_spans(self) -> Self {
            let node = match self.node {
                ExpressionKind::Literal(l) => ExpressionKind::Literal(l),
                ExpressionKind::Unary(u) => ExpressionKind::Unary(match u {
                    Unary::Negate(inner) => Unary::Negate(Box::new(inner.strip_spans())),
                    Unary::Not(inner) => Unary::Not(Box::new(inner.strip_spans())),
                }),
                ExpressionKind::Binary(b) => ExpressionKind::Binary(Binary {
                    left: Box::new(b.left.strip_spans()),
                    operator: b.operator,
                    right: Box::new(b.right.strip_spans()),
                }),
                ExpressionKind::Logical(l) => ExpressionKind::Logical(Logical {
                    left: Box::new(l.left.strip_spans()),
                    operator: l.operator,
                    right: Box::new(l.right.strip_spans()),
                }),
                ExpressionKind::Variable(ident) => ExpressionKind::Variable(ident),
                ExpressionKind::Assign(ident, inner) => {
                    ExpressionKind::Assign(ident, Box::new(inner.strip_spans()))
                }
                ExpressionKind::Call(callee, args) => ExpressionKind::Call(
                    Box::new(callee.strip_spans()),
                    args.into_iter().map(|a| a.strip_spans()).collect(),
                ),
            };
            Node::untracked(node)
        }
    }

    impl From<Literal> for Expression {
        fn from(l: Literal) -> Self {
            Node::untracked(ExpressionKind::from(l))
        }
    }

    impl From<Unary> for Expression {
        fn from(u: Unary) -> Self {
            Node::untracked(ExpressionKind::from(u))
        }
    }

    impl From<bool> for Expression {
        fn from(b: bool) -> Self {
            Node::untracked(ExpressionKind::from(b))
        }
    }
}
