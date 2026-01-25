use std::{
    fs::read_to_string,
    io::{self, Write},
};

use crate::parser::Parser;
use crate::parser::lexing::Scanner;

#[derive(Debug)]
pub struct LoxInterpreter {
    had_error: bool,
}

impl LoxInterpreter {
    pub fn new() -> Self {
        Self { had_error: false }
    }

    /// Start a repl
    pub fn run_repl(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut input = String::new();

        loop {
            print!("> ");
            stdout.flush().expect("Could not write to stdout?");
            match stdin.read_line(&mut input) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    self.run(&input);
                    input.clear();
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    /// Run a file
    pub fn run_file(&mut self, file_path: &String) -> Result<(), ()> {
        let contents = read_to_string(file_path).map_err(|_| ())?;
        self.run(&contents);
        Ok(())
    }

    // parse and execute source code??
    pub fn run(&mut self, source: &str) {
        let scanner = Scanner::new(source);
        let mut parser = Parser::new(scanner.collect());
        loop {
            match parser.parse_expression() {
                Ok(Some(expr)) => {
                    println!("parsed expr: {:#?}", expr);
                }
                Ok(None) => {
                    println!("done");
                    break;
                }
                Err(er) => {
                    eprintln!("parse error! {:#?}", er);
                    break;
                }
            }
        }
    }

    pub fn error(&mut self, line: usize, message: &String) {
        self.report(line, &"".to_owned(), message);
    }

    pub fn report(&mut self, line: usize, where_at: &String, message: &String) {
        eprintln!("[line {}] Error {}: {}", line, where_at, message);
        self.had_error = true;
    }
}
