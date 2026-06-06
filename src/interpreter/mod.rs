use std::{
    fs::read_to_string,
    io::{self, IsTerminal, Write},
    path::Path,
};

use crate::{
    interpreter::{
        builtins::register_builtins,
        environment::Environment,
        error::LoxError,
        gc::Gc,
        resolver::{Resolutions, resolve},
        value::Value,
    },
    parser::{
        ast::{
            BinaryOp, Declaration, DeclarationKind, Expression, ExpressionKind, Function,
            LogicalOp, Statement, StatementKind, Unary,
        },
        diagnostic::has_error,
        node::Node,
        parse_str,
    },
};

use crate::parser::ast::Literal;

mod builtins;
mod environment;
mod error;
mod gc;
mod resolver;
mod value;

#[derive(Debug)]
pub struct Interpreter {
    globals: Environment,
    environment: Environment,
    had_error: bool,
    had_runtime_error: bool,
    resolutions: Resolutions,
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Environment::new();
        register_builtins(&globals);
        Self {
            environment: globals.clone(),
            globals,
            had_error: false,
            had_runtime_error: false,
            resolutions: Resolutions::new(),
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
            DeclarationKind::Function(f) => self.execute_fun_declaration(f),
            DeclarationKind::Var {
                identifier,
                initial,
            } => {
                let value = if let Some(expr) = initial {
                    self.evaluate(expr)?
                } else {
                    Value::Nil
                };
                self.environment.define(identifier, value);
                Ok(())
            }
        }
    }

    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<(), LoxError> {
        match &stmt.node {
            StatementKind::Print(expr) => {
                let val = self.evaluate(expr)?;
                println!("{}", val.display());
            }
            StatementKind::Return(expr) => {
                let val = if let Some(expr) = expr {
                    self.evaluate(expr)?
                } else {
                    Value::Nil
                };
                return Err(LoxError::Return(val));
            }
            StatementKind::Expression(expr) => {
                self.evaluate(expr)?;
            }
            StatementKind::Block(decls) => {
                let env = self.environment.child();
                return self.execute_block(decls, env);
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

    pub fn execute_block(
        &mut self,
        decls: &[Declaration],
        env: Environment,
    ) -> Result<(), LoxError> {
        let prev = std::mem::replace(&mut self.environment, env);
        let mut result = Ok(());
        for decl in decls {
            result = self.execute_declaration(decl);
            if result.is_err() {
                break;
            }
        }

        self.environment = prev;
        result
    }

    pub fn execute_fun_declaration(&mut self, fun_decl: &Function) -> Result<(), LoxError> {
        let fn_val = Value::Function(Gc::new(value::Function::new(
            fun_decl.name.clone(),
            self.environment.clone(),
            fun_decl
                .parameter_names
                .iter()
                .map(|(_, name)| name.to_owned())
                .collect(),
            fun_decl.body.clone(),
        )));
        self.environment.define(&fun_decl.name, fn_val);
        Ok(())
    }

    pub fn evaluate(&mut self, expr: &Expression) -> Result<Value, LoxError> {
        match &expr.node {
            ExpressionKind::Literal(Literal::Nil) => Ok(Value::Nil),
            ExpressionKind::Literal(Literal::Number(n)) => Ok(Value::Number(*n)),
            ExpressionKind::Literal(Literal::String(s)) => Ok(Value::String(s.clone())),
            ExpressionKind::Literal(Literal::True) => Ok(Value::Boolean(true)),
            ExpressionKind::Literal(Literal::False) => Ok(Value::Boolean(false)),
            ExpressionKind::Variable(ident) => {
                if let Some(distance) = self.resolutions.resolve(expr) {
                    self.environment.get_at(distance, ident)
                } else {
                    self.globals.get(ident)
                }
            }
            ExpressionKind::Assign(ident, inner) => {
                let value = self.evaluate(inner)?;
                if let Some(distance) = self.resolutions.resolve(expr) {
                    self.environment.assign_at(distance, ident, value.clone())?;
                } else {
                    self.globals.assign(ident, value.clone())?;
                }
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
                    // `+` is overloaded for numbers and strings; equality works
                    // across all value types. The rest are numeric-only.
                    BinaryOp::Add => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                        _ => Err(LoxError::InvalidAdd),
                    },
                    BinaryOp::Equal => Ok(Value::Boolean(left.equals(&right))),
                    BinaryOp::NotEqual => Ok(Value::Boolean(!left.equals(&right))),
                    op => {
                        let l = left.get_number()?;
                        let r = right.get_number()?;
                        Ok(match op {
                            BinaryOp::Subtract => Value::Number(l - r),
                            BinaryOp::Divide => Value::Number(l / r),
                            BinaryOp::Multiply => Value::Number(l * r),
                            BinaryOp::Greater => Value::Boolean(l > r),
                            BinaryOp::GreaterEqual => Value::Boolean(l >= r),
                            BinaryOp::Less => Value::Boolean(l < r),
                            BinaryOp::LessEqual => Value::Boolean(l <= r),
                            BinaryOp::Add | BinaryOp::Equal | BinaryOp::NotEqual => {
                                unreachable!("handled above")
                            }
                        })
                    }
                }
            }
        }
    }

    pub fn evaluate_call(
        &mut self,
        callee_expr: &Expression,
        arg_exprs: &[Expression],
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
            Value::Function(fun_ref) => {
                let fun = fun_ref.borrow();

                if args.len() != fun.parameter_names.len() {
                    return Err(LoxError::WrongArity {
                        expected: fun.parameter_names.len(),
                        received: args.len(),
                    });
                }
                let fun_env = fun.environment.child();
                for (name, value) in fun.parameter_names.iter().zip(args) {
                    fun_env.define(name, value);
                }

                let StatementKind::Block(decls) = &fun.body.node else {
                    unreachable!("function bodies are always blocks");
                };
                let result = self.execute_block(decls, fun_env);

                if let Err(LoxError::Return(return_val)) = result {
                    Ok(return_val)
                } else {
                    result.map(|_| Value::Nil)
                }
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
                    self.run(&input, true);
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
        self.run(&contents, false);
        Ok(())
    }

    /// Parse, resolve, and execute `source`.
    ///
    /// When `repl` is true, the result of each top-level expression statement
    /// is echoed back, the way an interactive prompt would.
    pub fn run(&mut self, source: &str, repl: bool) {
        let color = io::stderr().is_terminal();
        let (decls, diags) = parse_str(source);
        if has_error(&diags) {
            for diag in &diags {
                eprintln!("{}\n", diag.render(source, color));
            }
            self.had_error = true;
            return;
        }
        let resolution_diags = resolve(&mut self.resolutions, &decls);
        if has_error(&resolution_diags) {
            for diag in &resolution_diags {
                eprintln!("{}\n", diag.render(source, color));
            }
            self.had_error = true;
            return;
        }
        for decl in decls {
            if let DeclarationKind::Statement(Node {
                node: StatementKind::Expression(expr),
                ..
            }) = &decl.node
                && repl
            {
                // In the REPL we echo the value of a bare expression statement,
                // so we bypass `execute_declaration()` and evaluate directly.
                match self.evaluate(expr) {
                    Ok(val) => {
                        println!("{}", val.repr());
                    }
                    Err(e) => {
                        self.had_runtime_error = true;
                        eprintln!("Runtime error: {}", e);
                        break;
                    }
                }
            } else if let Err(err) = self.execute_declaration(&decl) {
                self.had_runtime_error = true;
                eprintln!("Runtime error: {}", err);
                break;
            }
        }
    }
}
