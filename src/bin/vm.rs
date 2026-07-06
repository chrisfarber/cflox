use cflox::{
    parser::span::Span,
    vm::{VM, chunk::Chunk, op::Op, value::Value},
};
use clap::Parser;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(about = "Chris's lox VM")]
struct Cli {
    #[arg()]
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let mut chunk = Chunk::new();
    let span = Span { start: 0, end: 0 };
    chunk.write_const(Value::Number(1.2), span);
    chunk.write_const(Value::Number(3.4), span);
    chunk.write_op(Op::Add, span);
    chunk.write_const(Value::Number(5.6), span);
    chunk.write_op(Op::Divide, span);
    chunk.write_op(Op::Negate, span);
    chunk.write_op(Op::Return, span);

    #[cfg(feature = "trace")]
    println!("{:#?}", chunk);

    let mut vm = VM::new();
    vm.run(&chunk).unwrap();

    // if let Some(file_path) = cli.file {
    //     println!("file: {}", file_path);
    //     // lox.run_file(&file_path).expect("could not run file?");
    // } else {
    //     // lox.run_repl();
    // }
    exit(-1);
}
