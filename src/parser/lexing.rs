use crate::parser::{
    diagnostic::Diagnostic,
    node::Span,
    token::{Token, TokenKind},
};

struct Scanner {
    // unsure whether this will be needed:
    // name: String,
    pub source: Vec<char>,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
    current: usize,
    /// When building up a token, it may encapsulate multiple chars from the
    /// source. This field is used to keep track of the start of the token
    /// being considered as it is built up.
    cur_token_start: usize,
}

pub type ScanResult = (Vec<Token>, Vec<Diagnostic>);

impl Scanner {
    pub fn new(source_str: &str) -> Self {
        let source = source_str.chars().collect();
        Self {
            source,
            current: 0,
            cur_token_start: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn current_span(&self) -> Span {
        let start = self.cur_token_start;
        let end = self.current;
        if start >= end {
            panic!("impossible current span?? start {}, end {}", start, end);
        }
        Span { start, end }
    }

    /// Given a token type, build a Token
    fn push_token(&mut self, kind: TokenKind) {
        let span = self.current_span();
        let token = Token { span, node: kind };
        self.tokens.push(token);
    }

    pub fn scan(&mut self) {
        loop {
            // Each iteration begins a fresh token, so anchor its span here.
            // Without this, tokens with no whitespace between them (`a=a`)
            // would all share the span start of the last whitespace boundary.
            let start = self.current;
            self.cur_token_start = start;
            let Some(current) = self.advance() else {
                break;
            };

            match current {
                '(' => self.push_token(TokenKind::LeftParen),
                ')' => self.push_token(TokenKind::RightParen),
                '{' => self.push_token(TokenKind::LeftBrace),
                '}' => self.push_token(TokenKind::RightBrace),
                ',' => self.push_token(TokenKind::Comma),
                '.' => self.push_token(TokenKind::Dot),
                '-' => self.push_token(TokenKind::Minus),
                '+' => self.push_token(TokenKind::Plus),
                ';' => self.push_token(TokenKind::Semicolon),
                '*' => self.push_token(TokenKind::Star),
                '!' => {
                    if self.match_next('=') {
                        self.push_token(TokenKind::BangEqual);
                    } else {
                        self.push_token(TokenKind::Bang);
                    }
                }
                '=' => {
                    if self.match_next('=') {
                        self.push_token(TokenKind::EqualEqual);
                    } else {
                        self.push_token(TokenKind::Equal);
                    }
                }
                '<' => {
                    if self.match_next('=') {
                        self.push_token(TokenKind::LessEqual);
                    } else {
                        self.push_token(TokenKind::Less);
                    }
                }
                '>' => {
                    if self.match_next('=') {
                        self.push_token(TokenKind::GreaterEqual);
                    } else {
                        self.push_token(TokenKind::Greater);
                    }
                }
                '/' => {
                    if self.match_next('/') {
                        while self.peek() != Some(&'\n') {
                            self.advance();
                        }
                    } else {
                        self.push_token(TokenKind::Slash);
                    }
                }
                // Whitespace produces no token; the next iteration re-anchors
                // the span, so there is nothing to do here.
                ' ' | '\t' | '\r' | '\n' => {}
                '"' => {
                    let content_start = self.current;
                    let mut content_end = content_start;
                    loop {
                        if let Some(n) = self.advance() {
                            if *n == '"' {
                                break;
                            }
                            content_end = self.current;
                        } else {
                            self.diagnostics.push(Diagnostic::error(
                                self.current_span(),
                                "unexpected end of file",
                            ));
                            // no need to drop chars because this should be the end; actually, bail
                            return;
                        }
                    }
                    self.push_token(TokenKind::String(
                        self.source[content_start..content_end].iter().collect(),
                    ));
                }
                other => {
                    if other.is_ascii_digit() {
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
                        self.push_token(TokenKind::Number(
                            self.source[start..self.current]
                                .iter()
                                .collect::<String>()
                                .parse()
                                .expect("didn't we prove this could parse"),
                        ));
                    } else if other.is_alphabetic() {
                        while let Some(c) = self.peek()
                            && (c.is_alphabetic() || *c == '_' || c.is_ascii_digit())
                        {
                            self.advance();
                        }
                        let ident: String = self.source[start..self.current].iter().collect();
                        self.push_token(match ident.as_str() {
                            "and" => TokenKind::And,
                            "class" => TokenKind::Class,
                            "else" => TokenKind::Else,
                            "false" => TokenKind::False,
                            "for" => TokenKind::For,
                            "fun" => TokenKind::Fun,
                            "if" => TokenKind::If,
                            "nil" => TokenKind::Nil,
                            "or" => TokenKind::Or,
                            "print" => TokenKind::Print,
                            "return" => TokenKind::Return,
                            "super" => TokenKind::Super,
                            "this" => TokenKind::This,
                            "true" => TokenKind::True,
                            "var" => TokenKind::Var,
                            "while" => TokenKind::While,
                            _ => TokenKind::Identifier(ident),
                        });
                    } else {
                        let msg = format!("unexpected input char: '{}'", other);
                        self.diagnostics
                            .push(Diagnostic::error(self.current_span(), msg));
                    }
                }
            };
        }
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

pub fn scan(source: &str) -> ScanResult {
    let mut scanner = Scanner::new(source);
    scanner.scan();
    (scanner.tokens, scanner.diagnostics)
}

#[cfg(test)]
mod tests {
    use crate::parser::{diagnostic::Severity, lexing::scan, node::Span, token::TokenKind};

    #[test]
    fn token_indexes() {
        let (tokens, _) = scan("8 - - 2");
        let eight = tokens.get(0).unwrap().span;
        assert_eq!(eight.start, 0);
        assert_eq!(eight.end, 1);

        let minus = tokens.get(1).unwrap().span;
        assert_eq!(minus.start, 2);
        assert_eq!(minus.end, 3);

        let minus2 = tokens.get(2).unwrap().span;
        assert_eq!(minus2.start, 4);
        assert_eq!(minus2.end, 5);
    }

    #[test]
    fn adjacent_tokens_have_independent_spans() {
        // With no whitespace between tokens, each span must start at its own
        // token rather than accumulating from the previous token's start.
        let (tokens, _) = scan("a=a");
        assert_eq!(tokens[0].span, Span { start: 0, end: 1 }); // a
        assert_eq!(tokens[1].span, Span { start: 1, end: 2 }); // =
        assert_eq!(tokens[2].span, Span { start: 2, end: 3 }); // a

        // Multi-char operators glued to their operands stay correct too.
        let (tokens, _) = scan("1==2");
        assert_eq!(tokens[0].span, Span { start: 0, end: 1 }); // 1
        assert_eq!(tokens[1].span, Span { start: 1, end: 3 }); // ==
        assert_eq!(tokens[2].span, Span { start: 3, end: 4 }); // 2
    }

    #[test]
    fn multichar_token_indexes() {
        let (tokens, _) = scan("   \"hello \" ");
        let token = &tokens[0];
        assert_eq!(token.span, Span { start: 3, end: 11 });
        assert_eq!(token.node, TokenKind::String("hello ".into()));
    }

    #[test]
    fn comments_and_spans() {
        let source = r#"
hello
// this is a comment
// and so is this!
true
            "#;
        let (tokens, diagnostics) = scan(source);
        assert!(diagnostics.is_empty());
        let hello = &tokens[0];
        let lit_true = &tokens[1];
        assert_eq!(hello.span.in_source(source), "hello");
        assert_eq!(lit_true.span.in_source(source), "true");
    }

    #[test]
    fn unclosed_string() {
        let source = "   nil 4 + \"a string that never ends ";
        let (tokens, diagnostics) = scan(source);
        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "unexpected end of file");
        assert_eq!(diag.span.in_source(source), "\"a string that never ends ");

        let last_token = tokens.last().unwrap();
        assert_eq!(last_token.node, TokenKind::Plus);
    }
}
