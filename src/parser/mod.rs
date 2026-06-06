use crate::parser::{
    ast::{
        Binary, BinaryOp, Declaration, DeclarationKind, Expression, ExpressionKind, Function,
        Logical, Statement, StatementKind,
    },
    diagnostic::Diagnostic,
    lexing::scan,
    node::{Node, Span},
    token::{Token, TokenKind},
};

pub mod ast;
pub mod diagnostic;
pub mod lexing;
pub mod node;
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
            Some(TokenKind::Var) => self.parse_var_declaration(),
            Some(TokenKind::Fun) => self.parse_fun_declaration(),
            _ => self.parse_statement_declaration(),
        }
    }

    pub fn parse_statement_declaration(&mut self) -> ParseDeclarationResult {
        self.parse_statement()
            .map(|stmt| Declaration::new(stmt.span, DeclarationKind::Statement(stmt)))
    }

    pub fn parse_fun_declaration(&mut self) -> ParseDeclarationResult {
        let fun = self.expect_token(TokenKind::Fun)?;

        let (_, fun_name) = self.expect_identifier()?;
        let mut parameter_names = vec![];

        self.expect_token(TokenKind::LeftParen)?;
        while let Some(TokenKind::Identifier(_)) = self.peek_type() {
            let (_, param_name) = self.expect_identifier()?;
            parameter_names.push(param_name);

            if self.peek_type() == Some(&TokenKind::Comma) {
                self.advance()?;
            } else {
                break;
            }
        }
        self.expect_token(TokenKind::RightParen)?;

        let body = self.parse_block_statement()?;

        Ok(Declaration::encapsulating(
            fun,
            body.span,
            DeclarationKind::Function(Function {
                name: fun_name,
                parameter_names,
                body: Box::new(body),
            }),
        ))
    }

    pub fn parse_var_declaration(&mut self) -> ParseDeclarationResult {
        let start = self.expect_token(TokenKind::Var)?.start;

        let (_, identifier) = self.expect_identifier()?;

        let next_tok = self.advance()?;
        match next_tok.node {
            TokenKind::Equal => {
                let expr = self.parse_expression()?;
                let semi = self.expect_token(TokenKind::Semicolon)?;
                let end = semi.end;
                Ok(Declaration::new(
                    Span { start, end },
                    DeclarationKind::Var {
                        identifier,
                        initial: Some(expr),
                    },
                ))
            }
            TokenKind::Semicolon => {
                let end = next_tok.span.end;
                Ok(Declaration::new(
                    Span { start, end },
                    DeclarationKind::Var {
                        identifier,
                        initial: None,
                    },
                ))
            }
            _ => Err(Diagnostic::error(
                &next_tok,
                "expected '=' or ';' when parsing declaration",
            )),
        }
    }

    pub fn parse_statement(&mut self) -> ParseStatementResult {
        match self.peek_type() {
            Some(TokenKind::Print) => {
                let start = self.advance()?.span.start;
                let expr = self.parse_expression()?;
                let semi = self.expect_token(TokenKind::Semicolon)?;
                let end = semi.end;
                Ok(Node::new(Span { start, end }, StatementKind::Print(expr)))
            }
            Some(TokenKind::Return) => self.parse_return_statement(),
            Some(TokenKind::LeftBrace) => self.parse_block_statement(),
            Some(TokenKind::If) => self.parse_if_statement(),
            Some(TokenKind::While) => self.parse_while_statement(),
            Some(TokenKind::For) => self.parse_for_statement(),
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

    pub fn parse_return_statement(&mut self) -> ParseStatementResult {
        let start = self.expect_token(TokenKind::Return)?;
        let expr = self.parse_expression()?;
        let end = self.expect_token(TokenKind::Semicolon)?;
        Ok(Statement::encapsulating(
            start,
            end,
            StatementKind::Return(expr),
        ))
    }

    pub fn parse_block_statement(&mut self) -> ParseStatementResult {
        let start = self.expect_token(TokenKind::LeftBrace)?.start;
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
        Ok(Node::new(Span { start, end }, StatementKind::Block(decls)))
    }

    pub fn parse_if_statement(&mut self) -> ParseStatementResult {
        let start = self.expect_token(TokenKind::If)?.start;

        self.expect_token(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenKind::RightParen)?;

        let then_branch = Box::new(self.parse_statement()?);
        let mut end = then_branch.span.end;

        let mut else_branch = None;
        if self.peek_type() == Some(&TokenKind::Else) {
            self.advance()?;
            let else_stmt = self.parse_statement()?;
            end = else_stmt.span.end;
            else_branch = Some(Box::new(else_stmt));
        }

        Ok(Statement::new(
            Span { start, end },
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            },
        ))
    }

    pub fn parse_while_statement(&mut self) -> ParseStatementResult {
        let while_span = self.expect_token(TokenKind::While)?;
        self.expect_token(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(TokenKind::RightParen)?;
        let body = self.parse_statement()?;

        Ok(Statement::encapsulating(
            while_span,
            body.span,
            StatementKind::While {
                condition,
                body: Box::new(body),
            },
        ))
    }

    pub fn parse_for_statement(&mut self) -> ParseStatementResult {
        let for_span = self.expect_token(TokenKind::For)?;

        self.expect_token(TokenKind::LeftParen)?;

        let init = match self.peek_type() {
            Some(&TokenKind::Semicolon) => {
                self.advance()?;
                None
            }
            Some(&TokenKind::Var) => Some(self.parse_var_declaration()?),
            _ => Some(self.parse_statement_declaration()?),
        };

        let condition = match self.peek_type() {
            Some(&TokenKind::Semicolon) => {
                self.advance()?;
                Expression::new(
                    self.current_span(),
                    ExpressionKind::Literal(ast::Literal::True),
                )
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect_token(TokenKind::Semicolon)?;
                expr
            }
        };

        let increment = match self.peek_type() {
            Some(&TokenKind::RightParen) => None,
            _ => Some(self.parse_expression()?),
        };
        self.expect_token(TokenKind::RightParen)?;

        let mut body = self.parse_statement()?;

        if let Some(increment) = increment {
            body = vec![body.into(), increment.into()].into();
        }

        let mut block: Vec<Declaration> = vec![];
        if let Some(decl) = init {
            block.push(decl);
        };

        let while_stmt: Statement = Statement::encapsulating(
            for_span,
            body.span,
            StatementKind::While {
                condition,
                body: Box::new(body),
            },
        );
        block.push(while_stmt.into());

        Ok(block.into())
    }

    pub fn parse_expression(&mut self) -> ParseExpressionResult {
        self.parse_assignment()
    }

    pub fn parse_assignment(&mut self) -> ParseExpressionResult {
        let expr = self.parse_logic_or()?;

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

    pub fn parse_logic_or(&mut self) -> ParseExpressionResult {
        let left = self.parse_logic_and()?;
        if self.peek_type() == Some(&TokenKind::Or) {
            self.advance()?;
            let right = self.parse_logic_and()?;
            Ok(Expression::encapsulating(
                left.span,
                right.span,
                ExpressionKind::Logical(Logical {
                    left: Box::new(left),
                    operator: ast::LogicalOp::Or,
                    right: Box::new(right),
                }),
            ))
        } else {
            Ok(left)
        }
    }

    pub fn parse_logic_and(&mut self) -> ParseExpressionResult {
        let left = self.parse_equality()?;
        if self.peek_type() == Some(&TokenKind::And) {
            self.advance()?;
            let right = self.parse_equality()?;
            Ok(Expression::encapsulating(
                left.span,
                right.span,
                ExpressionKind::Logical(Logical {
                    left: Box::new(left),
                    operator: ast::LogicalOp::And,
                    right: Box::new(right),
                }),
            ))
        } else {
            Ok(left)
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
            _ => return self.parse_call(),
        };
        let start = self.advance()?.span.start;
        let inner = self.parse_unary()?;
        let end = inner.span.end;
        Ok(Expression::new(
            Span { start, end },
            ast::ExpressionKind::Unary(build(Box::new(inner))),
        ))
    }

    pub fn parse_call(&mut self) -> ParseExpressionResult {
        let mut expr = self.parse_primary()?;

        loop {
            if self.peek_type() == Some(&TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expression) -> ParseExpressionResult {
        self.expect_token(TokenKind::LeftParen)?;

        let mut arguments = vec![];
        while self.peek_type() != Some(&TokenKind::RightParen) {
            arguments.push(self.parse_expression()?);
            if self.peek_type() != Some(&TokenKind::Comma) {
                break;
            }
            self.expect_token(TokenKind::Comma)?;
        }

        // lox disallows more than 255 arguments due to some aspect of implementing the
        // bytecode vm
        if arguments.len() >= 255 {
            return Err(Diagnostic::error(
                arguments[255].span,
                "Can't have more than 255 arguments",
            ));
        }

        let end = self.expect_token(TokenKind::RightParen)?;

        Ok(Expression::encapsulating(
            callee.span,
            end,
            ExpressionKind::Call(Box::new(callee), arguments),
        ))
    }

    pub fn parse_primary(&mut self) -> ParseExpressionResult {
        let next = self.advance()?;
        let wrap = |node: ast::ExpressionKind| Ok(Expression::new(next.span, node));
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
