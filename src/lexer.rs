#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Val,
    Mut,
    Pub,
    Type,
    Pre,
    Post,
    Return,
    If,
    Else,
    Trust,
    Rely,
    Use,
    Some,
    None_,
    Ident(String),
    StringLit(String),
    IntLit(i64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Eq,
    Bang,
    At,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Not,
    Arrow,
    Eof,
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                self.advance();
            }
            if self.peek() == Some('/') && self.src.get(self.pos + 1) == Some(&'/') {
                while self.peek().map(|c| c != '\n').unwrap_or(false) {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some(c) => {
                    let tok = self.next_token(c)?;
                    tokens.push(tok);
                }
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self, c: char) -> Result<Token, String> {
        if c == '"' {
            self.advance();
            let mut s = String::new();
            loop {
                match self.advance() {
                    None => return Err("unterminated string literal".into()),
                    Some('"') => break,
                    Some('\\') => match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(x) => s.push(x),
                        None => return Err("unterminated escape".into()),
                    },
                    Some(ch) => s.push(ch),
                }
            }
            return Ok(Token::StringLit(s));
        }

        if c.is_ascii_digit() {
            let mut n = String::new();
            while self.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) {
                n.push(self.advance().unwrap());
            }
            return Ok(Token::IntLit(n.parse().unwrap()));
        }

        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while self
                .peek()
                .map(|x| x.is_alphanumeric() || x == '_')
                .unwrap_or(false)
            {
                ident.push(self.advance().unwrap());
            }
            return Ok(match ident.as_str() {
                "fn" => Token::Fn,
                "val" => Token::Val,
                "mut" => Token::Mut,
                "pub" => Token::Pub,
                "type" => Token::Type,
                "pre" => Token::Pre,
                "post" => Token::Post,
                "return" => Token::Return,
                "if" => Token::If,
                "else" => Token::Else,
                "trust" => Token::Trust,
                "rely" => Token::Rely,
                "use" => Token::Use,
                "some" => Token::Some,
                "none" => Token::None_,
                "not" => Token::Not,
                _ => Token::Ident(ident),
            });
        }

        self.advance();
        Ok(match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            ',' => Token::Comma,
            '.' => Token::Dot,
            '@' => Token::At,
            '+' => Token::Plus,
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::EqEq
                } else {
                    Token::Eq
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::BangEq
                } else {
                    Token::Bang
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '&' => Token::And,
            '|' => Token::Or,
            other => return Err(format!("unexpected character: {other:?}")),
        })
    }
}
