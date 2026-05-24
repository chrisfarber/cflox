use std::{
    fs::read_to_string,
    io::{self, Write},
    path::Path,
};

use crate::{
    interpreter::{
        builtins::register_builtins, environment::Environment, error::LoxError, gc::Gc,
        value::Value,
    },
    parser::{
        ast::{
            BinaryOp, Declaration, DeclarationKind, Expression, ExpressionKind, LogicalOp,
            Statement, StatementKind, Unary,
        },
        diagnostic::Severity,
        parse_str,
        span::Spanned,
    },
};

use crate::parser::ast::Literal;

mod builtins;
mod environment;
mod error;
mod gc;
mod value;

#[derive(Debug)]
pub struct Interpreter {
    environment: Gc<Environment>,
    had_error: bool,
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let environment = Gc::new(Environment::new());
        register_builtins(&mut environment.borrow_mut());
        Self {
            environment,
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
                identifier,
                initial,
            } => {
                let value = if let Some(expr) = initial {
                    self.evaluate(expr)?
                } else {
                    Value::Nil
                };
                self.environment.borrow_mut().define(identifier, value);
                Ok(())
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
            StatementKind::Block(decls) => {
                let previous = self.environment.clone();
                let mut result = Ok(());
                self.environment = Gc::new(Environment::new_with_parent(&previous));

                for stmt in decls {
                    result = self.execute_declaration(stmt);
                    if result.is_err() {
                        break;
                    }
                }

                self.environment = previous;
                return result;
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_result = self.evaluate(condition)?;
                if cond_result.is_truthy() {
                    self.execute_statement(then_branch)?;
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(else_stmt)?;
                }
            }
            StatementKind::While { condition, body } => {
                while self.evaluate(condition)?.is_truthy() {
                    self.execute_statement(body)?;
                }
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
            ExpressionKind::Variable(ident) => self.environment.borrow().get(ident),
            ExpressionKind::Assign(ident, inner) => {
                let value = self.evaluate(inner)?;
                self.environment.borrow_mut().assign(ident, value.clone())?;
                Ok(value)
            }
            ExpressionKind::Call(callee, args) => self.evaluate_call(callee, args),
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
            ExpressionKind::Logical(logical) => {
                let left_val = self.evaluate(&logical.left)?;
                let done = match logical.operator {
                    LogicalOp::And => !left_val.is_truthy(),
                    LogicalOp::Or => left_val.is_truthy(),
                };

                Ok(if done {
                    left_val
                } else {
                    self.evaluate(&logical.right)?
                })
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

    pub fn evaluate_call(
        &mut self,
        callee_expr: &Expression,
        arg_exprs: &Vec<Expression>,
    ) -> Result<Value, LoxError> {
        let callee = self.evaluate(callee_expr)?;
        let mut args: Vec<Value> = Vec::with_capacity(arg_exprs.len());
        for arg_expr in arg_exprs {
            args.push(self.evaluate(arg_expr)?);
        }

        match callee {
            Value::BuiltinFn(builtin) => {
                let arg_count = args.len();
                if arg_count != builtin.arity {
                    return Err(LoxError::WrongArity {
                        expected: builtin.arity,
                        received: arg_count,
                    });
                }
                let out = (builtin.f)(self, args)?;
                Ok(out)
            }
            _ => Err(LoxError::InvalidFunctionCall),
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
