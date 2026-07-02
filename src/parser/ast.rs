use crate::parser::{node::Node, span::Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
    Variable(Span),
    Assign(Assign),
    Logical(Logical),
    Call(Call),
    Get(Get),
    Set(Set),
    This,
    Super(Span),
}

pub type Expression = Node<ExpressionKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub target: Span,
    pub value: Box<Expression>,
}

impl From<Assign> for ExpressionKind {
    fn from(a: Assign) -> ExpressionKind {
        ExpressionKind::Assign(a)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

impl From<Call> for ExpressionKind {
    fn from(c: Call) -> ExpressionKind {
        ExpressionKind::Call(c)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Get {
    pub object: Box<Expression>,
    pub name: Span,
}

impl From<Get> for ExpressionKind {
    fn from(g: Get) -> ExpressionKind {
        ExpressionKind::Get(g)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub object: Box<Expression>,
    pub name: Span,
    pub value: Box<Expression>,
}

impl From<Set> for ExpressionKind {
    fn from(s: Set) -> ExpressionKind {
        ExpressionKind::Set(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Expression(Expression),
    Print(Expression),
    Return(Option<Expression>),
    Block(Vec<Declaration>),
    If(If),
    While(While),
}

#[derive(Debug, Clone, PartialEq)]
pub struct If {
    pub condition: Expression,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
}

impl From<If> for StatementKind {
    fn from(i: If) -> StatementKind {
        StatementKind::If(i)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct While {
    pub condition: Expression,
    pub body: Box<Statement>,
}

impl From<While> for StatementKind {
    fn from(w: While) -> StatementKind {
        StatementKind::While(w)
    }
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
    Var(Var),
    Function(Function),
    Class(Class),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub identifier: Span,
    pub initial: Option<Expression>,
}

impl From<Var> for DeclarationKind {
    fn from(v: Var) -> DeclarationKind {
        DeclarationKind::Var(v)
    }
}

pub type Declaration = Node<DeclarationKind>;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Span,
    pub parameter_names: Vec<Span>,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: Span,
    pub superclass: Option<Expression>,
    pub methods: Vec<Node<Function>>,
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
        Literal::Bool(b).into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unary {
    /// negate the sign of a number
    Negate(Box<Expression>),
    /// logical (boolean) not operation
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
