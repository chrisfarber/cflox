use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::interpreter::environment::Environment;
use crate::interpreter::gc::Gc;
use crate::interpreter::{Interpreter, error::LoxError};
use crate::parser::ast::Statement;

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    BuiltinFn(Rc<BuiltinFn>),
    Function(Gc<Function>),
    Class(Gc<Class>),
    Instance(Gc<Instance>),
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

    /// The user-facing display form of a value, as `print` renders it.
    /// Strings are shown verbatim, without surrounding quotes.
    pub fn display(&self) -> String {
        match self {
            Self::Nil => "nil".into(),
            Self::Boolean(true) => "true".into(),
            Self::Boolean(false) => "false".into(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => s.clone(),
            Self::BuiltinFn(builtin) => format!("<builtin: {}>", builtin.name),
            Self::Function(f) => format!("<function: {}>", f.borrow().name),
            Self::Class(c) => format!("<class: {}>", c.borrow().name),
            Self::Instance(c) => format!("<Instance: {}>", c.borrow().class_name()),
        }
    }

    /// A debugging representation of a value, used to echo results in the
    /// REPL. Strings are quoted so they can be told apart from other values.
    pub fn repr(&self) -> String {
        match self {
            Self::String(s) => format!("\"{}\"", s),
            other => other.display(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Value::Nil => matches!(other, Value::Nil),
            Value::Boolean(l) => matches!(other, Value::Boolean(r) if l == r),
            Value::Number(l) => matches!(other, Value::Number(r) if l == r),
            Value::String(l) => matches!(other, Value::String(r) if l == r),
            Value::BuiltinFn(l) => matches!(other, Value::BuiltinFn(r) if Rc::ptr_eq(l, r)),
            Value::Function(l) => matches!(other, Value::Function(r) if Gc::ptr_eq(l, r)),
            Value::Class(l) => matches!(other, Value::Class(r) if Gc::ptr_eq(l, r)),
            Value::Instance(l) => matches!(other, Value::Instance(r) if Gc::ptr_eq(l, r)),
        }
    }
}

type BuiltinFnPtr = Rc<dyn Fn(&mut Interpreter, Vec<Value>) -> Result<Value, LoxError>>;

#[derive(Clone)]
pub struct BuiltinFn {
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

/// A callable function value for a user defined function.
pub struct Function {
    pub name: String,
    pub environment: Environment,
    pub parameter_names: Vec<String>,
    pub body: Box<Statement>,
}

impl Function {
    pub fn new(
        name: String,
        environment: Environment,
        parameter_names: Vec<String>,
        body: Box<Statement>,
    ) -> Self {
        Self {
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

pub struct Class {
    pub name: String,
}

impl Class {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    /// The arity of the constructor for the class
    pub fn arity(&self) -> usize {
        0
    }
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<class: {}>", self.name)
    }
}

pub struct Instance {
    pub class: Gc<Class>,
    pub fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: Gc<Class>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }

    pub fn class_name(&self) -> String {
        self.class.borrow().name.to_owned()
    }

    pub fn get_field(&self, field: &str) -> Option<Value> {
        self.fields.get(field).cloned()
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<instance: {}>", self.class.borrow().name)
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
    fn value_equality() {
        let b1 = BuiltinFn::new("b1", 0, |_int, _env| Ok(Value::Nil));
        let vb1 = Value::BuiltinFn(Rc::new(b1.clone()));
        let vb1_copy = vb1.clone();
        let vb1_fresh = Value::BuiltinFn(Rc::new(b1.clone()));

        assert!(Value::Nil == Value::Nil);
        assert!(Value::Nil != Value::Boolean(false));
        assert!(vb1 == vb1_copy);
        assert!(vb1 != vb1_fresh);
        assert!(Value::Number(0.0) == Value::Number(0.0));
        assert!(Value::Number(0.0) != Value::Number(1.0));
    }

    #[test]
    fn builtin_call() {
        let b = BuiltinFn::new("add", 2, add_builtin);
        let mut interp = Interpreter::new();
        let result = (b.f)(&mut interp, vec![Value::Number(1.0), Value::Number(2.0)]);
        assert_eq!(result.unwrap(), Value::Number(3.0));
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
