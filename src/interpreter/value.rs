use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::interpreter::environment::Environment;
use crate::interpreter::gc::Gc;
use crate::interpreter::{Interpreter, error::LoxError};
use crate::parser::ast::Statement;

static NEXT_FUN_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_BUILTIN_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    BuiltinFn(Rc<BuiltinFn>),
    Function(Gc<Function>),
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
            (Self::BuiltinFn(l), Self::BuiltinFn(r)) => l == r,
            (Self::Function(l), Self::Function(r)) => l == r,
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
            Self::BuiltinFn(builtin) => format!("<builtin: {}>", builtin.name),
            Self::Function(f) => format!("<function: {}>", f.borrow().name),
        }
    }
}

type BuiltinFnPtr = Rc<dyn Fn(&mut Interpreter, Vec<Value>) -> Result<Value, LoxError>>;

#[derive(Clone)]
pub struct BuiltinFn {
    id: u64,
    pub arity: usize,
    pub name: String,
    pub f: BuiltinFnPtr,
}

impl BuiltinFn {
    pub fn new(
        name: impl Into<String>,
        arity: usize,
        f: impl Fn(&mut Interpreter, Vec<Value>) -> Result<Value, LoxError> + 'static,
    ) -> Self {
        Self {
            id: NEXT_BUILTIN_ID.fetch_add(1, Ordering::Relaxed),
            arity,
            name: name.into(),
            f: Rc::new(f),
        }
    }
}

impl fmt::Debug for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<builtin fn {}>", self.name)
    }
}

impl PartialEq for BuiltinFn {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// A callable function value for a user defined function.
pub struct Function {
    id: u64,
    pub name: String,
    pub environment: Gc<Environment>,
    pub parameter_names: Vec<String>,
    pub body: Box<Statement>,
}

impl Function {
    pub fn new(
        name: String,
        environment: Gc<Environment>,
        parameter_names: Vec<String>,
        body: Box<Statement>,
    ) -> Self {
        Self {
            id: NEXT_FUN_ID.fetch_add(1, Ordering::Relaxed),
            name,
            environment,
            parameter_names,
            body,
        }
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function: {}>", self.name)
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialEq for Gc<Function> {
    fn eq(&self, other: &Self) -> bool {
        self.borrow().eq(&other.borrow())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::Interpreter;

    fn dummy_builtin(_interp: &mut Interpreter, _args: Vec<Value>) -> Result<Value, LoxError> {
        Ok(Value::Nil)
    }

    fn add_builtin(_interp: &mut Interpreter, args: Vec<Value>) -> Result<Value, LoxError> {
        let a = args[0].get_number()?;
        let b = args[1].get_number()?;
        Ok(Value::Number(a + b))
    }

    #[test]
    fn builtin_call() {
        let b = BuiltinFn::new("add", 2, add_builtin);
        let mut interp = Interpreter::new();
        let result = (b.f)(&mut interp, vec![Value::Number(1.0), Value::Number(2.0)]);
        assert_eq!(result.unwrap(), Value::Number(3.0));
    }

    #[test]
    fn clone_preserves_identity() {
        let a = BuiltinFn::new("clock", 0, dummy_builtin);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_builtins_are_not_equal() {
        let a = BuiltinFn::new("clock", 0, dummy_builtin);
        let b = BuiltinFn::new("clock", 0, dummy_builtin);
        assert_ne!(a, b);
    }

    #[test]
    fn same_fn_different_names_are_not_equal() {
        let a = BuiltinFn::new("foo", 0, dummy_builtin);
        let b = BuiltinFn::new("bar", 0, dummy_builtin);
        assert_ne!(a, b);
    }

    #[test]
    fn builtin_value_equality() {
        let a = Value::BuiltinFn(Rc::new(BuiltinFn::new("f", 0, dummy_builtin)));
        let b = a.clone();
        assert_eq!(a, b);

        let c = Value::BuiltinFn(Rc::new(BuiltinFn::new("f", 0, dummy_builtin)));
        assert_ne!(a, c);
    }
}
