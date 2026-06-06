use std::collections::HashMap;

use crate::interpreter::{error::LoxError, gc::Gc, value::Value};

#[derive(Debug)]
struct InnerEnvironment {
    parent: Option<Environment>,
    values: HashMap<String, Value>,
}

/// The environment is where we name and retain references to values
impl InnerEnvironment {
    /// Look up a value in the environment or its parent environment.
    ///
    /// If no such name is defined, the operation will fail with a `LoxError`
    pub fn get(&self, key: &str) -> Result<Value, LoxError> {
        if let Some(get) = self.values.get(key) {
            return Ok(get.clone());
        }

        if let Some(parent) = &self.parent {
            parent.get(key)
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
            parent.assign(key, value)
        } else {
            Err(LoxError::UndefinedVariable {
                name: key.to_owned(),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment(Gc<InnerEnvironment>);

impl Environment {
    /// Build a new root environment
    pub fn new() -> Self {
        Self(Gc::new(InnerEnvironment {
            parent: None,
            values: HashMap::new(),
        }))
    }

    /// Build an environment that inherits from a parent environment
    pub fn new_with_parent(parent: &Environment) -> Self {
        Self(Gc::new(InnerEnvironment {
            parent: Some(parent.clone()),
            values: HashMap::new(),
        }))
    }

    /// Create a new environment with this environment as its parent
    pub fn child(&self) -> Self {
        Self::new_with_parent(self)
    }

    pub fn get(&self, key: &str) -> Result<Value, LoxError> {
        self.0.borrow().get(key)
    }
    pub fn define(&self, key: &str, value: Value) {
        self.0.borrow_mut().define(key, value);
    }

    pub fn assign(&self, key: &str, value: Value) -> Result<(), LoxError> {
        self.0.borrow_mut().assign(key, value)
    }

    fn ancestor(&self, distance: u32) -> Option<Environment> {
        let mut distance = distance;
        let mut target = self.clone();
        while distance > 0 {
            let inner = target.0.borrow().parent.clone();
            match inner {
                None => return None,
                Some(parent) => {
                    target = parent;
                    distance -= 1;
                }
            }
        }
        Some(target)
    }

    pub fn get_at(&self, distance: u32, key: &str) -> Result<Value, LoxError> {
        self.ancestor(distance)
            .expect("environment distance not found!")
            .get(key)
    }

    pub fn assign_at(&self, distance: u32, key: &str, value: Value) -> Result<(), LoxError> {
        self.ancestor(distance)
            .expect("environment distance not found!")
            .assign(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_environment_operations() {
        let env = Environment::new();

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
        let a = Environment::new();
        a.define("a1", Value::Nil);

        let b = a.child();
        b.define("b1", Value::Boolean(true));

        let c = b.child();
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

    #[test]
    fn specifying_distance() {
        let a = Environment::new();
        let b = a.child();
        let c = b.child();
        let d = c.child();

        c.define("hello", Value::Number(1.0));
        assert_eq!(d.get_at(1, "hello"), Ok(Value::Number(1.0)));
        assert_eq!(
            d.get_at(2, "hello"),
            Err(LoxError::UndefinedVariable {
                name: "hello".into()
            })
        );
    }

    #[test]
    #[should_panic]
    fn too_far() {
        let a = Environment::new();
        let b = a.child();
        let c = b.child();

        #[allow(unused_must_use)]
        c.get_at(3, "anything");
    }
}
