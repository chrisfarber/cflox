use std::{
    fs::read_to_string,
    io::{self, Write},
};

use crate::parser::{
    Parser,
    ast::{
        BinaryOp, Declaration, Expression, Spanned, SpannedDeclaration, SpannedExpression,
        SpannedStatement, Statement, Unary,
    },
};

use crate::parser::{ast::Literal, lexing::Scanner};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match *self {
            Self::Nil => false,
            Self::Boolean(b) => b,
            _ => true,
        }
    }

    /// Would you like to unwrap your value into a number,
    /// lest a runtime exception? then this is for you
    pub fn get_number(&self) -> Result<f64, LoxError> {
        if let Self::Number(n) = self {
            Ok(*n)
        } else {
            Err(LoxError::ExpectedNumber)
        }
    }

    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Number(l), Self::Number(r)) => l == r,
            (Self::String(l), Self::String(r)) => l == r,
            _ => false,
        }
    }

    /// Build a representation of the value in lox's own syntax.
    pub fn stringify(&self) -> String {
        match self {
            Self::Nil => "nil".into(),
            Self::Boolean(true) => "true".into(),
            Self::Boolean(false) => "false".into(),
            Self::Number(n) => n.to_string(),
            // This is laughably bad, I know:
            Self::String(s) => format!("\"{}\"", s),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoxError {
    #[error("can only negate numbers")]
    InvalidNegation,
    #[error("not yet implemented")]
    NotYetImplemented,
    #[error("expected value to be a number")]
    ExpectedNumber,
    #[error("can only add two numbers or two strings")]
    InvalidAdd,
}

#[derive(Debug)]
pub struct Interpreter {
    had_error: bool,
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            had_error: false,
            had_runtime_error: false,
        }
    }

    pub fn exit_code(&self) -> i32 {
        if self.had_error {
            return 65;
        }
        if self.had_runtime_error {
            return 70;
        }
        0
    }

    pub fn execute_declaration(&mut self, decl: &SpannedDeclaration) -> Result<(), LoxError> {
        match &decl.node {
            Declaration::Statement(stmt) => self.execute_statement(stmt),
            Declaration::Var {
                identifier,
                initial,
            } => Err(LoxError::NotYetImplemented),
        }
    }

    pub fn execute_statement(&mut self, stmt: &SpannedStatement) -> Result<(), LoxError> {
        match &stmt.node {
            Statement::Print(expr) => {
                let val = self.evaluate(expr)?;
                match val {
                    Value::String(str) => {
                        println!("{}", str);
                    }
                    other => {
                        println!("{}", other.stringify());
                    }
                }
            }
            Statement::Expression(expr) => {
                self.evaluate(expr)?;
            }
        }
        Ok(())
    }

    pub fn evaluate(&mut self, expr: &SpannedExpression) -> Result<Value, LoxError> {
        match &expr.node {
            Expression::Literal(Literal::Nil) => Ok(Value::Nil),
            Expression::Literal(Literal::Number(n)) => Ok(Value::Number(*n)),
            Expression::Literal(Literal::String(s)) => Ok(Value::String(s.clone())),
            Expression::Literal(Literal::True) => Ok(Value::Boolean(true)),
            Expression::Literal(Literal::False) => Ok(Value::Boolean(false)),
            Expression::Unary(Unary::Negate(inner)) => {
                if let Value::Number(n) = self.evaluate(inner)? {
                    Ok(Value::Number(-n))
                } else {
                    Err(LoxError::InvalidNegation)
                }
            }
            Expression::Unary(Unary::Not(inner)) => {
                let inner_is_truthy = self.evaluate(inner)?.is_truthy();
                Ok(Value::Boolean(!inner_is_truthy))
            }
            Expression::Binary(binary) => {
                let left = self.evaluate(&binary.left)?;
                let right = self.evaluate(&binary.right)?;

                match binary.operator {
                    BinaryOp::Subtract => {
                        Ok(Value::Number(left.get_number()? - right.get_number()?))
                    }
                    BinaryOp::Divide => Ok(Value::Number(left.get_number()? / right.get_number()?)),
                    BinaryOp::Multiply => {
                        Ok(Value::Number(left.get_number()? * right.get_number()?))
                    }
                    BinaryOp::Greater => {
                        Ok(Value::Boolean(left.get_number()? > right.get_number()?))
                    }
                    BinaryOp::GreaterEqual => {
                        Ok(Value::Boolean(left.get_number()? >= right.get_number()?))
                    }
                    BinaryOp::Less => Ok(Value::Boolean(left.get_number()? < right.get_number()?)),
                    BinaryOp::LessEqual => {
                        Ok(Value::Boolean(left.get_number()? <= right.get_number()?))
                    }
                    BinaryOp::Add => match (left, right) {
                        (Value::Number(left_num), Value::Number(right_num)) => {
                            Ok(Value::Number(left_num + right_num))
                        }
                        (Value::String(sl), Value::String(sr)) => Ok(Value::String(sl + &sr)),
                        _ => Err(LoxError::InvalidAdd),
                    },
                    BinaryOp::Equal => Ok(Value::Boolean(left.equals(&right))),
                    BinaryOp::NotEqual => Ok(Value::Boolean(!left.equals(&right))),
                }
            }
        }
    }

    /// Start a repl
    pub fn run_repl(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut input = String::new();

        loop {
            print!("> ");
            stdout.flush().expect("Could not write to stdout?");
            match stdin.read_line(&mut input) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    self.run(&input);
                    input.clear();
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    /// Run a file
    pub fn run_file(&mut self, file_path: &String) -> Result<(), ()> {
        let contents = read_to_string(file_path).map_err(|_| ())?;
        self.run(&contents);
        Ok(())
    }

    pub fn run(&mut self, source: &str) {
        let scanner = Scanner::new(source);
        let mut parser = Parser::new(scanner.collect());
        loop {
            match parser.parse_declaration() {
                Ok(Some(decl)) => {
                    if let Declaration::Statement(Spanned {
                        node: Statement::Expression(expr),
                        start: _,
                        end: _,
                    }) = decl.node
                    {
                        let res = self.evaluate(&expr);
                        match res {
                            Ok(val) => {
                                println!("{}", val.stringify());
                            }
                            Err(e) => {
                                self.had_runtime_error = true;
                                eprintln!("Runtime error: {}", e);
                                break;
                            }
                        }
                    } else {
                        self.execute_declaration(&decl);
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(er) => {
                    eprintln!("parse error! {:#?}", er);
                    self.had_error = true;
                    break;
                }
            }
        }
    }

    pub fn error(&mut self, line: usize, message: &String) {
        self.report(line, &"".to_owned(), message);
    }

    pub fn report(&mut self, line: usize, where_at: &String, message: &String) {
        eprintln!("[line {}] Error {}: {}", line, where_at, message);
        self.had_error = true;
    }
}
