#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals:
    Identifier(String),
    String(String),
    Number(f64),

    // Keywords:
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

pub struct Scanner {
    source: Vec<char>,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source_str: &str) -> Self {
        let source = source_str.chars().collect();
        Self {
            source,
            current: 0,
            line: 1,
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    pub fn advance(&mut self) -> Option<&char> {
        let c = self.source.get(self.current);
        if self.current < self.source.len() {
            self.current += 1;
        }
        c
    }

    pub fn peek(&self) -> Option<&char> {
        self.source.get(self.current)
    }

    /// Is the next char the expected char?
    /// If so, advances and returns true. Otherwise, returns false.
    pub fn match_next(&mut self, expected: char) -> bool {
        if self.source.get(self.current) == Some(&expected) {
            self.current += 1;
            true
        } else {
            false
        }
    }
}

impl Iterator for Scanner {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.is_at_end() {
            return None;
        }

        // location before taking any chars
        let start = self.current;

        let c = self
            .advance()
            .expect("we should have proved we are not at the end");

        let token_type = match c {
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            '-' => TokenType::Minus,
            '+' => TokenType::Plus,
            ';' => TokenType::Semicolon,
            '*' => TokenType::Star,
            '!' => {
                if self.match_next('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                }
            }
            '=' => {
                if self.match_next('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                }
            }
            '<' => {
                if self.match_next('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                }
            }
            '>' => {
                if self.match_next('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                }
            }
            '/' => {
                if self.match_next('/') {
                    while let Some(n) = self.peek() {
                        if *n != '\n' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return self.next();
                } else {
                    TokenType::Slash
                }
            }
            ' ' | '\t' | '\r' => {
                return self.next();
            }
            '\n' => {
                self.line += 1;
                return self.next();
            }
            '"' => {
                let start = self.current;
                let mut end = start;
                loop {
                    if let Some(n) = self.advance() {
                        if *n == '"' {
                            break;
                        }
                        end = self.current;
                    } else {
                        panic!("unexpected end of string");
                    }
                }
                TokenType::String(self.source[start..end].iter().collect())
            }
            other => {
                if other.is_ascii_digit() {
                    let start = self.current - 1;
                    let mut saw_dot = false;
                    while let Some(c) = self.peek() {
                        if *c == '.' {
                            if saw_dot {
                                break;
                            } else {
                                saw_dot = true;
                            }
                        } else if !c.is_ascii_digit() {
                            break;
                        }
                        self.advance();
                    }
                    TokenType::Number(
                        self.source[start..self.current]
                            .iter()
                            .collect::<String>()
                            .parse()
                            .expect("didn't we prove this could parse"),
                    )
                } else if other.is_alphabetic() {
                    let start = self.current - 1;
                    while let Some(c) = self.peek()
                        && (c.is_alphabetic() || *c == '_' || c.is_ascii_digit())
                    {
                        self.advance();
                    }
                    let ident: String = self.source[start..self.current].iter().collect();
                    match ident.as_str() {
                        "and" => TokenType::And,
                        "class" => TokenType::Class,
                        "else" => TokenType::Else,
                        "false" => TokenType::False,
                        "for" => TokenType::For,
                        "fun" => TokenType::Fun,
                        "if" => TokenType::If,
                        "nil" => TokenType::Nil,
                        "or" => TokenType::Or,
                        "print" => TokenType::Print,
                        "return" => TokenType::Return,
                        "super" => TokenType::Super,
                        "this" => TokenType::This,
                        "true" => TokenType::True,
                        "var" => TokenType::Var,
                        "while" => TokenType::While,
                        _ => TokenType::Identifier(ident),
                    }
                } else {
                    panic!("unexpected input char '{}'", other);
                }
            }
        };

        Some(Token {
            start,
            end: self.current,
            token_type,
            line: self.line,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::lexing::Scanner;

    #[test]
    fn token_indexes() {
        let mut scanner = Scanner::new("8 - - 2");
        let eight = scanner.next().unwrap();
        assert_eq!(eight.start, 0);
        assert_eq!(eight.end, 1);

        let minus = scanner.next().unwrap();
        assert_eq!(minus.start, 2);
        assert_eq!(minus.end, 3);

        let minus2 = scanner.next().unwrap();
        assert_eq!(minus2.start, 4);
        assert_eq!(minus2.end, 5);
    }

    #[test]
    fn multichar_token_indexes() {
        let mut scanner = Scanner::new("   \"hello \" ");
        let str = scanner.next().unwrap();
        assert_eq!(str.start, 3);
        assert_eq!(str.end, 11);
    }
}
