use std::{
    fs::read_to_string,
    io::{self, Write},
    path::Path,
};

use crate::{
    interpreter::{error::LoxError, value::Value},
    parser::{
        ast::{
            BinaryOp, Declaration, DeclarationKind, Expression, ExpressionKind, Statement,
            StatementKind, Unary,
        },
        diagnostic::Severity,
        parse_str,
        span::Spanned,
    },
};

use crate::parser::ast::Literal;

mod environment;
mod error;
mod value;

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

    pub fn execute_declaration(&mut self, decl: &Declaration) -> Result<(), LoxError> {
        match &decl.node {
            DeclarationKind::Statement(stmt) => self.execute_statement(stmt),
            DeclarationKind::Var {
                identifier: _identifier,
                initial: _initial,
            } => {
                println!("we can't yet execute this var decl {:#?}", decl);
                Err(LoxError::NotYetImplemented)
            }
        }
    }

    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<(), LoxError> {
        match &stmt.node {
            StatementKind::Print(expr) => {
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
            StatementKind::Expression(expr) => {
                self.evaluate(expr)?;
            }
        }
        Ok(())
    }

    pub fn evaluate(&mut self, expr: &Expression) -> Result<Value, LoxError> {
        match &expr.node {
            ExpressionKind::Literal(Literal::Nil) => Ok(Value::Nil),
            ExpressionKind::Literal(Literal::Number(n)) => Ok(Value::Number(*n)),
            ExpressionKind::Literal(Literal::String(s)) => Ok(Value::String(s.clone())),
            ExpressionKind::Literal(Literal::True) => Ok(Value::Boolean(true)),
            ExpressionKind::Literal(Literal::False) => Ok(Value::Boolean(false)),
            ExpressionKind::Unary(Unary::Negate(inner)) => {
                if let Value::Number(n) = self.evaluate(inner)? {
                    Ok(Value::Number(-n))
                } else {
                    Err(LoxError::InvalidNegation)
                }
            }
            ExpressionKind::Unary(Unary::Not(inner)) => {
                let inner_is_truthy = self.evaluate(inner)?.is_truthy();
                Ok(Value::Boolean(!inner_is_truthy))
            }
            ExpressionKind::Binary(binary) => {
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
    pub fn run_file<T: AsRef<Path>>(&mut self, file_path: T) -> Result<(), ()> {
        let contents = read_to_string(file_path).map_err(|_| ())?;
        self.run(&contents);
        Ok(())
    }

    pub fn run(&mut self, source: &str) {
        let (decls, diags) = parse_str(source);
        if diags.iter().any(|d| d.severity == Severity::Error) {
            for diag in diags {
                eprintln!("parse error: {:#?}", diag);
            }
            eprintln!("parsed AST: {:#?}", decls);
        } else {
            for decl in decls {
                if let DeclarationKind::Statement(Spanned {
                    node: StatementKind::Expression(expr),
                    span: _,
                }) = decl.node
                {
                    // Here we are in a special case: the declaration is actually a
                    // statement containing an expression.
                    //
                    // We want to print the result of the expression, so, we bypass
                    // `execute_declaration()` and call `evaluate()` directly.
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
                    if let Err(err) = self.execute_declaration(&decl) {
                        self.had_runtime_error = true;
                        eprintln!("Runtime error: {}", err);
                        break;
                    }
                }
            }
        }
    }

    // pub fn error(&mut self, line: usize, message: &String) {
    //     self.report(line, &"".to_owned(), message);
    // }

    // pub fn report(&mut self, line: usize, where_at: &String, message: &String) {
    //     eprintln!("[line {}] Error {}: {}", line, where_at, message);
    //     self.had_error = true;
    // }
}
