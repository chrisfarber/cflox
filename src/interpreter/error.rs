use crate::interpreter::value::Value;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum LoxError {
    #[error("can only negate numbers")]
    InvalidNegation,
    #[allow(dead_code)]
    #[error("not yet implemented")]
    NotYetImplemented,
    #[error("expected value to be a number")]
    ExpectedNumber,
    #[error("can only add two numbers or two strings")]
    InvalidAdd,
    #[error("undefined variable: {name:?}")]
    UndefinedVariable { name: String },
    #[error("can only call functions and classes")]
    InvalidFunctionCall,
    #[error("function expected {expected:?} arguments, received {received:?}")]
    WrongArity { expected: usize, received: usize },
    #[error("return")]
    Return(Value),
    #[error("only instances have properties")]
    InvalidPropertyAcess,
    #[error("Undefined property {property:?}")]
    UndefinedProperty { property: String },
    #[error("Superclass must be a class.")]
    SuperclassMustBeClass,
    #[error("an internal interpreter error occurred: {message:?}")]
    InternalError { message: String },
}
