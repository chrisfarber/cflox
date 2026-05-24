use std::collections::HashMap;

use crate::interpreter::{error::LoxError, gc::Gc, value::Value};

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
    pub fn new_with_parent(parent: &Gc<Environment>) -> Self {
        Self {
            parent: Some(parent.clone()),
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

        if let Some(parent) = &self.parent {
            parent.borrow().get(key)
        } else {
            Err(LoxError::UndefinedVariable {
                name: key.to_owned(),
            })
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

        if let Some(parent) = &self.parent {
            parent.borrow_mut().assign(key, value)
        } else {
            Err(LoxError::UndefinedVariable {
                name: key.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_environment_operations() {
        let env_ref = Gc::new(Environment::new());
        let mut env = env_ref.borrow_mut();

        // Undefined keys trigger an error
        assert_eq!(
            env.get("hello"),
            Err(LoxError::UndefinedVariable {
                name: "hello".to_owned()
            })
        );
        assert_eq!(
            env.assign("hello", Value::Boolean(true)),
            Err(LoxError::UndefinedVariable {
                name: "hello".to_owned()
            })
        );

        env.define("hello", Value::Nil);
        assert_eq!(env.get("hello").unwrap(), Value::Nil);

        env.assign("hello", Value::Number(7.0)).unwrap();
        assert_eq!(env.get("hello").unwrap(), Value::Number(7.0));
    }

    #[test]
    fn inheriting_from_parent_environments() {
        let a = Gc::new(Environment::new());
        a.borrow_mut().define("a1", Value::Nil);

        let b = Gc::new(Environment::new_with_parent(&a));
        b.borrow_mut().define("b1", Value::Boolean(true));

        let c = Gc::new(Environment::new_with_parent(&b));
        c.borrow_mut().define("c1", Value::Number(7.0));

        assert_eq!(c.borrow().get("a1").unwrap(), Value::Nil);
        assert_eq!(c.borrow().get("b1").unwrap(), Value::Boolean(true));
        assert_eq!(c.borrow().get("c1").unwrap(), Value::Number(7.0));

        // Definition leaves parent environments unmodified
        c.borrow_mut().define("a1", Value::Boolean(false));
        assert_eq!(b.borrow().get("a1").unwrap(), Value::Nil);
        assert_eq!(c.borrow().get("a1").unwrap(), Value::Boolean(false));

        // Assignment can impact parent environments, though
        c.borrow_mut().assign("b1", Value::Number(14.0)).unwrap();
        assert_eq!(b.borrow().get("b1").unwrap(), Value::Number(14.0));
        assert_eq!(c.borrow().get("b1").unwrap(), Value::Number(14.0));
    }
}
