#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
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
    Elif,
    Trust,
    Use,
    Ref,
    For,
    Some,
    None_,
    Match,
    True,
    False,
    Defer,
    Break,
    As,
    Test,
    Assert,
    When,
    Otherwise,
    Is,
    Union,
    Enum,
    Extern,
    Link,
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
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
    PlusPlus,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    Shl,
    Shr,
    Not,
    Arrow,
    FatArrow,
    Tilde,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
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
            let line = self.line;
            let col = self.col;
            match self.peek() {
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        line,
                        col,
                    });
                    break;
                }
                Some(c) => {
                    let kind = self.next_token(c).map_err(|e| format!("{}:{}: {}", line, col, e))?;
                    tokens.push(Token { kind, line, col });
                }
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self, c: char) -> Result<TokenKind, String> {
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
            return Ok(TokenKind::StringLit(s));
        }

        if c.is_ascii_digit() {
            let mut n = String::new();
            if c == '0' && self.src.get(self.pos + 1).copied() == Some('x') {
                self.advance();
                self.advance();
                while self.peek().map(|x| x.is_ascii_hexdigit()).unwrap_or(false) {
                    n.push(self.advance().unwrap());
                }
                return u64::from_str_radix(&n, 16)
                    .map(|v| TokenKind::IntLit(v as i64))
                    .map_err(|e| format!("invalid hex literal '0x{n}': {e}"));
            }
            while self.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) {
                n.push(self.advance().unwrap());
            }
            let is_float = self.peek() == Some('.')
                || self.peek() == Some('e')
                || self.peek() == Some('E');
            if is_float {
                if self.peek() == Some('.') {
                    n.push(self.advance().unwrap());
                    while self.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) {
                        n.push(self.advance().unwrap());
                    }
                }
                if self.peek() == Some('e') || self.peek() == Some('E') {
                    n.push(self.advance().unwrap());
                    if self.peek() == Some('+') || self.peek() == Some('-') {
                        n.push(self.advance().unwrap());
                    }
                    while self.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) {
                        n.push(self.advance().unwrap());
                    }
                }
                return n.parse::<f64>()
                    .map(TokenKind::FloatLit)
                    .map_err(|e| format!("invalid float literal '{n}': {e}"));
            }
            return n.parse::<u64>()
                .map(|v| TokenKind::IntLit(v as i64))
                .map_err(|e| format!("invalid integer literal '{n}': {e}"));
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
                "fn" => TokenKind::Fn,
                "val" => TokenKind::Val,
                "mut" => TokenKind::Mut,
                "pub" => TokenKind::Pub,
                "type" => TokenKind::Type,
                "pre" => TokenKind::Pre,
                "post" => TokenKind::Post,
                "return" => TokenKind::Return,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "elif" => TokenKind::Elif,
                "trust" => TokenKind::Trust,
                "use" => TokenKind::Use,
                "ref" => TokenKind::Ref,
                "for" => TokenKind::For,
                "some" => TokenKind::Some,
                "none" => TokenKind::None_,
                "not" => TokenKind::Not,
                "match" => TokenKind::Match,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "defer" => TokenKind::Defer,
                "break" => TokenKind::Break,
                "as" => TokenKind::As,
                "test" => TokenKind::Test,
                "assert" => TokenKind::Assert,
                "when" => TokenKind::When,
                "otherwise" => TokenKind::Otherwise,
                "is" => TokenKind::Is,
                "union" => TokenKind::Union,
                "enum" => TokenKind::Enum,
                "extern" => TokenKind::Extern,
                "link" => TokenKind::Link,
                _ => TokenKind::Ident(ident),
            });
        }

        self.advance();
        Ok(match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '@' => TokenKind::At,
            '+' => {
                if self.peek() == Some('+') {
                    self.advance();
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqEq
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else if self.peek() == Some('<') {
                    self.advance();
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::And
                } else {
                    TokenKind::BitAnd
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::Or
                } else {
                    TokenKind::BitOr
                }
            }
            '~' => TokenKind::Tilde,
            other => return Err(format!("unexpected character: {other:?}")),
        })
    }
}
