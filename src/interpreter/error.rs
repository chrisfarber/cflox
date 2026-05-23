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
    #[error("undefined variable")]
    UndefinedVariable,
}
