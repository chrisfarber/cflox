#[derive(thiserror::Error, Debug)]
pub enum LoxError {
    #[error("can only negate numbers")]
    InvalidNegation,
    #[error("not yet implemented")]
    NotYetImplemented,
    #[error("expected value to be a number")]
    ExpectedNumber,
    #[error("can only add two numbers or two strings")]
    InvalidAdd,
}
