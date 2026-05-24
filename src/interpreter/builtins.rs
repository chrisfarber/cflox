use std::process::exit;
use std::rc::Rc;
use std::time::Instant;

use crate::interpreter::{
    Interpreter,
    environment::Environment,
    error::LoxError,
    value::{BuiltinFn, Value},
};

pub fn register_builtins(env: &mut Environment) {
    let start = Instant::now();

    register(env, "clock", 0, move |_, _| {
        Ok(Value::Number(start.elapsed().as_secs_f64()))
    });

    register(env, "clock_ms", 0, move |_, _| {
        Ok(Value::Number(start.elapsed().as_secs_f64() * 1000.0))
    });

    register(env, "exit", 1, |_, args| {
        if let Some(Value::Number(num)) = args.get(0) {
            let code = *num as i32;
            exit(code);
        } else {
            Err(super::error::LoxError::ExpectedNumber)
        }
    });
}

fn register(
    env: &mut Environment,
    name: &str,
    arity: usize,
    f: impl Fn(&mut Interpreter, Vec<Value>) -> Result<Value, LoxError> + 'static,
) {
    env.define(
        &name,
        Value::BuiltinFn(Rc::new(BuiltinFn::new(name.to_owned(), arity, f))),
    );
}
