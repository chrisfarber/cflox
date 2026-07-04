use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{self, IsTerminal, Write},
    path::Path,
    rc::Rc,
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
            Assign, BinaryOp, Call, Class, Declaration, DeclarationKind, Expression,
            ExpressionKind, Function, Get, If, LogicalOp, Set, Statement, StatementKind, Unary,
            Var, While,
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

    pub fn execute_declaration(
        &mut self,
        source: &Rc<str>,
        decl: &Declaration,
    ) -> Result<(), LoxError> {
        match &decl.node {
            DeclarationKind::Statement(stmt) => self.execute_statement(source, stmt),
            DeclarationKind::Function(f) => self.execute_fun_declaration(source, f),
            DeclarationKind::Class(c) => self.execute_class_declaration(source, c),
            DeclarationKind::Var(Var {
                identifier,
                initial,
            }) => {
                let value = if let Some(expr) = initial {
                    self.evaluate(source, expr)?
                } else {
                    Value::Nil
                };
                self.environment.define(identifier.in_source(source), value);
                Ok(())
            }
        }
    }

    pub fn execute_statement(
        &mut self,
        source: &Rc<str>,
        stmt: &Statement,
    ) -> Result<(), LoxError> {
        match &stmt.node {
            StatementKind::Print(expr) => {
                let val = self.evaluate(source, expr)?;
                println!("{}", val.display());
            }
            StatementKind::Return(expr) => {
                let val = if let Some(expr) = expr {
                    self.evaluate(source, expr)?
                } else {
                    Value::Nil
                };
                return Err(LoxError::Return(val));
            }
            StatementKind::Expression(expr) => {
                self.evaluate(source, expr)?;
            }
            StatementKind::Block(decls) => {
                let env = self.environment.child();
                return self.execute_block(source, decls, env);
            }
            StatementKind::If(If {
                condition,
                then_branch,
                else_branch,
            }) => {
                let cond_result = self.evaluate(source, condition)?;
                if cond_result.is_truthy() {
                    self.execute_statement(source, then_branch)?;
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(source, else_stmt)?;
                }
            }
            StatementKind::While(While { condition, body }) => {
                while self.evaluate(source, condition)?.is_truthy() {
                    self.execute_statement(source, body)?;
                }
            }
        }
        Ok(())
    }

    pub fn execute_block(
        &mut self,
        source: &Rc<str>,
        decls: &[Declaration],
        env: Environment,
    ) -> Result<(), LoxError> {
        let prev = std::mem::replace(&mut self.environment, env);
        let mut result = Ok(());
        for decl in decls {
            result = self.execute_declaration(source, decl);
            if result.is_err() {
                break;
            }
        }

        self.environment = prev;
        result
    }

    pub fn construct_fun(&mut self, source: &Rc<str>, fun_decl: &Function) -> Gc<value::Function> {
        Gc::new(value::Function::new(
            fun_decl.name.in_source(source).to_owned(),
            self.environment.clone(),
            fun_decl
                .parameter_names
                .iter()
                .map(|span| span.in_source(source).to_owned())
                .collect(),
            fun_decl.body.clone(),
            source.clone(),
        ))
    }

    pub fn execute_fun_declaration(
        &mut self,
        source: &Rc<str>,
        fun_decl: &Function,
    ) -> Result<(), LoxError> {
        let fn_val = Value::Function(self.construct_fun(source, fun_decl));
        self.environment
            .define(fun_decl.name.in_source(source), fn_val);
        Ok(())
    }

    pub fn execute_class_declaration(
        &mut self,
        source: &Rc<str>,
        class_decl: &Class,
    ) -> Result<(), LoxError> {
        let superclass = if let Some(superclass_expr) = &class_decl.superclass {
            match self.evaluate(source, superclass_expr)? {
                Value::Class(super_class) => Some(super_class),
                _ => Err(LoxError::SuperclassMustBeClass)?,
            }
        } else {
            None
        };

        let class_name = class_decl.name.in_source(source);
        self.environment.define(class_name, Value::Nil);

        let enclosing_environment = self.environment.clone();
        if let Some(super_gc) = &superclass {
            self.environment = self.environment.child();
            self.environment
                .define("super", Value::Class(super_gc.clone()));
        }

        let mut methods: HashMap<String, Gc<value::Function>> = HashMap::new();
        for func_node in &class_decl.methods {
            let fun = self.construct_fun(source, &func_node.node);
            methods.insert(func_node.node.name.in_source(source).to_owned(), fun);
        }

        let class_val = Value::Class(Gc::new(value::Class::new(
            class_name.to_owned(),
            superclass,
            methods,
        )));
        self.environment = enclosing_environment;
        self.environment.assign(class_name, class_val)?;
        Ok(())
    }

    pub fn evaluate(&mut self, source: &Rc<str>, expr: &Expression) -> Result<Value, LoxError> {
        match &expr.node {
            ExpressionKind::Literal(Literal::Nil) => Ok(Value::Nil),
            ExpressionKind::Literal(Literal::Number(n)) => Ok(Value::Number(*n)),
            ExpressionKind::Literal(Literal::String(s)) => Ok(Value::String(s.clone())),
            ExpressionKind::Literal(Literal::Bool(bool)) => Ok(Value::Boolean(*bool)),
            ExpressionKind::This => self.evaluate_variable(expr, "this"),
            ExpressionKind::Super(name_span) => {
                let dist = self
                    .resolutions
                    .resolve(expr)
                    .expect("super must have been resolved");
                let superclass = self.environment.get_at(dist, "super")?;
                let this = self.environment.get_at(dist - 1, "this")?;
                let Value::Class(superclass) = superclass else {
                    return Err(LoxError::InternalError {
                        message: "somehow, implausibly, super was not a class".to_owned(),
                    });
                };
                let Value::Instance(this) = this else {
                    return Err(LoxError::InternalError {
                        message: "somehow, implausibly, super's this was not an instance"
                            .to_owned(),
                    });
                };
                let name = name_span.in_source(source);
                let Some(fun) = superclass.borrow().get_method(name) else {
                    return Err(LoxError::UndefinedProperty {
                        property: name.to_owned(),
                    });
                };
                Ok(Value::Function(Gc::new(
                    fun.borrow().bind(Value::Instance(this)),
                )))
            }
            ExpressionKind::Variable(ident_span) => {
                self.evaluate_variable(expr, ident_span.in_source(source))
            }
            ExpressionKind::Assign(Assign { target, value }) => {
                let val = self.evaluate(source, value)?;
                let name = target.in_source(source);
                if let Some(distance) = self.resolutions.resolve(expr) {
                    self.environment.assign_at(distance, name, val.clone())?;
                } else {
                    self.globals.assign(name, val.clone())?;
                }
                Ok(val)
            }
            ExpressionKind::Call(Call { callee, arguments }) => {
                self.evaluate_call(source, callee, arguments)
            }
            ExpressionKind::Unary(Unary::Negate(inner)) => {
                if let Value::Number(n) = self.evaluate(source, inner)? {
                    Ok(Value::Number(-n))
                } else {
                    Err(LoxError::InvalidNegation)
                }
            }
            ExpressionKind::Unary(Unary::Not(inner)) => {
                let inner_is_truthy = self.evaluate(source, inner)?.is_truthy();
                Ok(Value::Boolean(!inner_is_truthy))
            }
            ExpressionKind::Logical(logical) => {
                let left_val = self.evaluate(source, &logical.left)?;
                let done = match logical.operator {
                    LogicalOp::And => !left_val.is_truthy(),
                    LogicalOp::Or => left_val.is_truthy(),
                };

                Ok(if done {
                    left_val
                } else {
                    self.evaluate(source, &logical.right)?
                })
            }
            ExpressionKind::Binary(binary) => {
                let left = self.evaluate(source, &binary.left)?;
                let right = self.evaluate(source, &binary.right)?;

                match binary.operator {
                    // `+` is overloaded for numbers and strings; equality works
                    // across all value types. The rest are numeric-only.
                    BinaryOp::Add => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                        _ => Err(LoxError::InvalidAdd),
                    },
                    BinaryOp::Equal => Ok(Value::Boolean(left.eq(&right))),
                    BinaryOp::NotEqual => Ok(Value::Boolean(!left.eq(&right))),
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
            ExpressionKind::Get(Get { object, name }) => {
                let obj_val = self.evaluate(source, object)?;
                let Value::Instance(inst) = obj_val.clone() else {
                    return Err(LoxError::InvalidPropertyAcess);
                };

                let field_name = name.in_source(source);
                let res = inst.borrow().get(field_name, obj_val).ok_or_else(|| {
                    LoxError::UndefinedProperty {
                        property: field_name.to_owned(),
                    }
                })?;

                Ok(res)
            }
            ExpressionKind::Set(Set {
                object,
                name,
                value,
            }) => {
                let obj_val = self.evaluate(source, object)?;

                let Value::Instance(inst) = obj_val else {
                    return Err(LoxError::InvalidPropertyAcess);
                };

                let val = self.evaluate(source, value)?;
                inst.borrow_mut()
                    .set_field(name.in_source(source), val.clone());

                Ok(val)
            }
        }
    }

    pub fn evaluate_variable(&mut self, expr: &Expression, ident: &str) -> Result<Value, LoxError> {
        if let Some(distance) = self.resolutions.resolve(expr) {
            self.environment.get_at(distance, ident)
        } else {
            self.globals.get(ident)
        }
    }

    pub fn evaluate_call(
        &mut self,
        source: &Rc<str>,
        callee_expr: &Expression,
        arg_exprs: &[Expression],
    ) -> Result<Value, LoxError> {
        let callee = self.evaluate(source, callee_expr)?;
        let mut args: Vec<Value> = Vec::with_capacity(arg_exprs.len());
        for arg_expr in arg_exprs {
            args.push(self.evaluate(source, arg_expr)?);
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
            Value::Function(fun_ref) => self.apply(fun_ref, &args),
            Value::Class(class_ref) => {
                let inst = Gc::new(value::Instance::new(class_ref.clone()));
                let inst_value = Value::Instance(inst.clone());

                let init = match inst.borrow().get_method("init", inst_value.clone()) {
                    Some(Value::Function(init)) => Some(init.clone()),
                    None => {
                        if !args.is_empty() {
                            return Err(LoxError::WrongArity {
                                expected: 0,
                                received: 0,
                            });
                        }
                        None
                    }
                    _ => unreachable!("the method must be a function"),
                };

                if let Some(init) = init {
                    self.apply(init, &args)?;
                }

                Ok(inst_value)
            }
            _ => Err(LoxError::InvalidFunctionCall),
        }
    }

    fn apply(&mut self, fun_ref: Gc<value::Function>, args: &[Value]) -> Result<Value, LoxError> {
        let fun = fun_ref.borrow();

        if args.len() != fun.parameter_names.len() {
            return Err(LoxError::WrongArity {
                expected: fun.parameter_names.len(),
                received: args.len(),
            });
        }
        let fun_env = fun.environment.child();
        for (name, value) in fun.parameter_names.iter().zip(args) {
            fun_env.define(name, value.clone());
        }

        let StatementKind::Block(decls) = &fun.body.node else {
            unreachable!("function bodies are always blocks");
        };
        let source = fun.source.clone();
        let result = self.execute_block(&source, decls, fun_env);

        result
            .map(|_| Value::Nil)
            .or_else(|err| {
                if let LoxError::Return(return_val) = err {
                    Ok(return_val)
                } else {
                    Err(err)
                }
            })
            .and_then(|val| {
                if fun.is_initializer {
                    fun.environment.get_at(0, "this")
                } else {
                    Ok(val)
                }
            })
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
        let resolution_diags = resolve(&mut self.resolutions, &decls, source);
        if has_error(&resolution_diags) {
            for diag in &resolution_diags {
                eprintln!("{}\n", diag.render(source, color));
            }
            self.had_error = true;
            return;
        }
        let source: Rc<str> = Rc::from(source);
        for decl in decls {
            if let DeclarationKind::Statement(Node {
                node: StatementKind::Expression(expr),
                ..
            }) = &decl.node
                && repl
            {
                // In the REPL we echo the value of a bare expression statement,
                // so we bypass `execute_declaration()` and evaluate directly.
                match self.evaluate(&source, expr) {
                    Ok(val) => {
                        println!("{}", val.repr());
                    }
                    Err(e) => {
                        self.had_runtime_error = true;
                        eprintln!("Runtime error: {}", e);
                        break;
                    }
                }
            } else if let Err(err) = self.execute_declaration(&source, &decl) {
                self.had_runtime_error = true;
                eprintln!("Runtime error: {}", err);
                break;
            }
        }
    }
}
