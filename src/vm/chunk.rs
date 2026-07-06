use std::rc::Rc;

use crate::{
    parser::span::Span,
    vm::{op::Op, value::Value},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub source: Rc<str>,
    // The book uses line numbers, but, I think I'll be even less efficient than
    // the book and use spans.
    pub spans: Vec<Span>,
}

impl Chunk {
    pub fn read(&self, at: usize) -> Result<(Op, usize), ()> {
        Op::read_from(&self.code[at..]).ok_or(())
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: vec![],
            source: Rc::from(""),
            spans: vec![],
        }
    }

    pub fn add_const(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        (self.constants.len() - 1) as u8
    }

    pub fn write(&mut self, byte: u8, span: Span) {
        self.code.push(byte);
        self.spans.push(span);
    }

    pub fn write_op(&mut self, op: Op, span: Span) {
        let bytes = op.serialize();
        let bytes_len = bytes.len();
        self.code.reserve(bytes_len);
        self.spans.reserve(bytes_len);
        for byte in bytes {
            self.code.push(byte);
            self.spans.push(span);
        }
    }

    pub fn write_const(&mut self, value: Value, span: Span) {
        let idx = self.add_const(value);
        self.write_op(Op::Constant(idx), span);
    }
}
