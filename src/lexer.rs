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
    In,
    Some,
    None_,
    Ok,
    Err,
    Match,
    True,
    False,
    Defer,
    Break,
    Continue,
    As,
    Test,
    Assert,
    When,
    Otherwise,
    Union,
    Enum,
    Extern,
    Link,
    Guarded,
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    CharLit(u8),
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
    DotDotDot,
    Eq,
    Bang,
    At,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
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
    BitXor,
    Shl,
    Shr,
    Not,
    Arrow,
    FatArrow,
    Tilde,
    Question,
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
        if c == '\\' && self.src.get(self.pos + 1) == Some(&'\\') {
            self.advance();
            self.advance();
            
            let mut lines = Vec::new();
            let mut current_line = String::new();
            
            loop {
                match self.peek() {
                    None | Some('\n') => {
                        lines.push(current_line.clone());
                        if self.peek() == Some('\n') {
                            self.advance();
                        }
                        break;
                    }
                    Some(ch) => {
                        current_line.push(ch);
                        self.advance();
                    }
                }
            }
            
            loop {
                let start_pos = self.pos;
                while self.peek().map(|c| c == ' ' || c == '\t').unwrap_or(false) {
                    self.advance();
                }
                
                if self.peek() == Some('\\') && self.src.get(self.pos + 1) == Some(&'\\') {
                    self.advance();
                    self.advance();
                    
                    current_line = String::new();
                    loop {
                        match self.peek() {
                            None | Some('\n') => {
                                lines.push(current_line.clone());
                                if self.peek() == Some('\n') {
                                    self.advance();
                                }
                                break;
                            }
                            Some(ch) => {
                                current_line.push(ch);
                                self.advance();
                            }
                        }
                    }
                } else {
                    self.pos = start_pos;
                    break;
                }
            }
            
            return Ok(TokenKind::StringLit(lines.join("\n")));
        }
        
        if c == '"' {
            self.advance();
            let mut s = String::new();
            loop {
                match self.advance() {
                    None => return Err("unterminated string literal".into()),
                    Some('"') => break,
                    Some('\\') => match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('x') => {
                            let h1 = self.advance().ok_or("unterminated hex escape")?;
                            let h2 = self.advance().ok_or("unterminated hex escape")?;
                            let hex = format!("{}{}", h1, h2);
                            let byte = u8::from_str_radix(&hex, 16)
                                .map_err(|_| format!("invalid hex escape '\\x{}'", hex))?;
                            s.push(byte as char);
                        }
                        Some(x) => {
                            s.push('\\');
                            s.push(x);
                        }
                        None => return Err("unterminated escape".into()),
                    },
                    Some(ch) => s.push(ch),
                }
            }
            return Ok(TokenKind::StringLit(s));
        }

        if c == '`' {
            self.advance();
            let mut s = String::new();
            loop {
                match self.advance() {
                    None => return Err("unterminated multiline string literal".into()),
                    Some('`') => break,
                    Some(ch) => s.push(ch),
                }
            }
            return Ok(TokenKind::StringLit(s));
        }

        if c == '\'' {
            self.advance();
            let byte = match self.advance() {
                Some('\\') => match self.advance() {
                    Some('n')  => b'\n',
                    Some('r')  => b'\r',
                    Some('t')  => b'\t',
                    Some('\'') => b'\'',
                    Some('\\') => b'\\',
                    Some('0')  => b'\0',
                    Some('x') => {
                        let h1 = self.advance().ok_or("unterminated hex escape in char literal")?;
                        let h2 = self.advance().ok_or("unterminated hex escape in char literal")?;
                        let hex = format!("{h1}{h2}");
                        u8::from_str_radix(&hex, 16)
                            .map_err(|_| format!("invalid hex escape '\\x{hex}' in char literal"))?
                    }
                    Some(x) => return Err(format!("unknown escape '\\{x}' in char literal")),
                    None => return Err("unterminated char literal".into()),
                },
                Some(ch) if ch.is_ascii() => ch as u8,
                Some(ch) => return Err(format!("non-ASCII character {ch:?} in char literal")),
                None => return Err("unterminated char literal".into()),
            };
            match self.advance() {
                Some('\'') => {}
                _ => return Err("char literal must contain exactly one character".into()),
            }
            return Ok(TokenKind::CharLit(byte));
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
                "in" => TokenKind::In,
                "some" => TokenKind::Some,
                "none" => TokenKind::None_,
                "ok" => TokenKind::Ok,
                "err" => TokenKind::Err,
                "not" => TokenKind::Not,
                "match" => TokenKind::Match,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "defer" => TokenKind::Defer,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "as" => TokenKind::As,
                "test" => TokenKind::Test,
                "assert" => TokenKind::Assert,
                "when" => TokenKind::When,
                "otherwise" => TokenKind::Otherwise,
                "union" => TokenKind::Union,
                "enum" => TokenKind::Enum,
                "extern" => TokenKind::Extern,
                "link" => TokenKind::Link,
                "guarded" => TokenKind::Guarded,
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
            '.' => {
                if self.peek() == Some('.') && self.src.get(self.pos + 1) == Some(&'.') {
                    self.advance();
                    self.advance();
                    TokenKind::DotDotDot
                } else {
                    TokenKind::Dot
                }
            }
            '@' => TokenKind::At,
            '+' => {
                if self.peek() == Some('+') {
                    self.advance();
                    TokenKind::PlusPlus
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some('-') {
                    self.advance();
                    TokenKind::MinusMinus
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusEq
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
            '^' => TokenKind::BitXor,
            '~' => TokenKind::Tilde,
            '?' => TokenKind::Question,
            other => return Err(format!("unexpected character: {other:?}")),
        })
    }
}
