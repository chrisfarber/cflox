use crate::vm::{chunk::Chunk, op::Op, value::Value};

pub mod chunk;
pub mod op;
pub mod value;

#[derive(Debug)]
pub struct VM {
    ip: usize,
    stack: Vec<Value>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            ip: 0,
            stack: vec![],
        }
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<(), ()> {
        self.ip = 0;
        loop {
            #[cfg(feature = "trace")]
            {
                print!("Stack:  ");
                for value in &self.stack {
                    print!("[ {:?} ]", value);
                }
                println!();
            }
            let (op, read) = chunk.read(self.ip)?;
            self.ip += read;
            match op {
                Op::Return => {
                    let val = self.stack.pop();
                    match val {
                        Some(Value::Number(n)) => println!("{}", n),
                        None => {}
                    }
                    return Ok(());
                }
                Op::Constant(ptr) => {
                    let val = &chunk.constants[ptr as usize];
                    self.stack.push(val.clone());
                }
                Op::Negate => match self.stack.last_mut() {
                    Some(Value::Number(n)) => *n = -*n,
                    None => return Err(()),
                },
                Op::Add => self.binary_arithmetic_op(|a, b| a + b)?,
                Op::Subtract => self.binary_arithmetic_op(|a, b| a - b)?,
                Op::Multiply => self.binary_arithmetic_op(|a, b| a * b)?,
                Op::Divide => self.binary_arithmetic_op(|a, b| a / b)?,
            }
        }
    }

    fn binary_arithmetic_op<F>(&mut self, op: F) -> Result<(), ()>
    where
        F: Fn(f64, f64) -> f64,
    {
        let b = self.stack.pop();
        let a = self.stack.pop();
        match (a, b) {
            (Some(Value::Number(a)), Some(Value::Number(b))) => {
                #[cfg(feature = "trace")]
                println!("{} {} -> {}", a, b, op(a, b));
                self.stack.push(Value::Number(op(a, b)));
            }
            _ => return Err(()),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}
