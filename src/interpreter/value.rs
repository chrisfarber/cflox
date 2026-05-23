use crate::interpreter::error::LoxError;

#[derive(Debug, Clone, PartialEq)]
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
