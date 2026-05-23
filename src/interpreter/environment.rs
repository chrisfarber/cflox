use std::collections::HashMap;

use crate::{
    gc::Gc,
    interpreter::{error::LoxError, value::Value},
};

#[derive(Debug)]
pub struct Environment {
    parent: Option<Gc<Environment>>,
    values: HashMap<String, Value>,
}

/// The environment is where we name and retain references to values
impl Environment {
    /// Build a new root environment
    pub fn new() -> Self {
        Self {
            parent: None,
            values: HashMap::new(),
        }
    }

    /// Build an environment that inherits from a parent environment
    pub fn new_with_parent(parent: Gc<Environment>) -> Self {
        Self {
            parent: Some(parent),
            values: HashMap::new(),
        }
    }

    /// Look up a value in the environment or its parent environment.
    ///
    /// If no such name is defined, the operation will fail with a `LoxError`
    pub fn get(&self, key: &str) -> Result<Value, LoxError> {
        if let Some(get) = self.values.get(key) {
            return Ok(get.clone());
        }

        if let Some(parent) = self.parent {
            parent.get(key)
        } else {
            Err(LoxError::UndefinedVariable)
        }
    }

    pub fn define(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_owned(), value);
    }

    pub fn assign(&mut self, key: &str, value: Value) -> Result<(), LoxError> {
        if self.values.contains_key(key) {
            self.values.insert(key.to_owned(), value.clone());
            return Ok(());
        }

        if let Some(mut parent) = self.parent {
            parent.assign(key, value)
        } else {
            Err(LoxError::UndefinedVariable)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gc::Heap;

    use super::*;

    #[test]
    fn basic_environment_operations() {
        let mut heap = Heap::new();
        let mut env = heap.alloc(Environment::new());

        // Undefined keys trigger an error
        assert_eq!(env.get("hello"), Err(LoxError::UndefinedVariable));
        assert_eq!(
            env.assign("hello", Value::Boolean(true)),
            Err(LoxError::UndefinedVariable)
        );

        env.define("hello", Value::Nil);
        assert_eq!(env.get("hello").unwrap(), Value::Nil);

        env.assign("hello", Value::Number(7.0)).unwrap();
        assert_eq!(env.get("hello").unwrap(), Value::Number(7.0));
    }

    #[test]
    fn inheriting_from_parent_environments() {
        let mut heap = Heap::new();
        let mut a = heap.alloc(Environment::new());
        a.define("a1", Value::Nil);

        let mut b = heap.alloc(Environment::new_with_parent(a));
        b.define("b1", Value::Boolean(true));

        let mut c = heap.alloc(Environment::new_with_parent(b));
        c.define("c1", Value::Number(7.0));

        assert_eq!(c.get("a1").unwrap(), Value::Nil);
        assert_eq!(c.get("b1").unwrap(), Value::Boolean(true));
        assert_eq!(c.get("c1").unwrap(), Value::Number(7.0));

        // Definition leaves parent environments unmodified
        c.define("a1", Value::Boolean(false));
        assert_eq!(b.get("a1").unwrap(), Value::Nil);
        assert_eq!(c.get("a1").unwrap(), Value::Boolean(false));

        // Assignment can impact parent environments, though
        c.assign("b1", Value::Number(14.0)).unwrap();
        assert_eq!(b.get("b1").unwrap(), Value::Number(14.0));
        assert_eq!(c.get("b1").unwrap(), Value::Number(14.0));
    }
}
