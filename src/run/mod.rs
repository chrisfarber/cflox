use std::{
    fs::read_to_string,
    io::{self, Write},
};

use crate::parsing::lexing::Scanner;

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
    pub fn run(&mut self, source: &String) {
        let scanner = Scanner::new(source);
        for tok in scanner {
            println!("read {:?}", tok);
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
