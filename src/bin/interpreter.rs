use cflox::interpreter::Interpreter;
use clap::Parser;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(about = "Chris's lox interpreter")]
struct Cli {
    #[arg()]
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let mut lox = Interpreter::new();

    if let Some(file_path) = cli.file {
        lox.run_file(&file_path).expect("could not run file?");
    } else {
        lox.run_repl();
    }
    exit(lox.exit_code());
}
