use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Use {
        name: String,
        path: String,
    },
    Fn(FnDef),
    TypeDef {
        public: bool,
        name: String,
        fields: Vec<Field>,
    },
    UnionDef {
        public: bool,
        name: String,
        variants: Vec<TypeExpr>,
    },
    EnumDef {
        public: bool,
        name: String,
        variants: Vec<String>,
    },
    Const {
        public: bool,
        name: String,
        expr: Expr,
    },
    Test {
        body: Vec<Stmt>,
    },
    ExternFn {
        public: bool,
        /// Calling convention, e.g. "C", "stdcall", "fastcall"
        conv: String,
        name: String,
        params: Vec<(String, TypeExpr)>,
        ret: TypeExpr,
        /// The external symbol name, e.g. "malloc"
        symbol: String,
    },
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub public: bool,
    pub namespace: Option<String>,
    pub name: String,
    pub params: Vec<(String, TypeExpr)>,
    pub ret: TypeExpr,
    pub trusted: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub mutable: bool,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),
    Slice(Box<TypeExpr>),
    Ref(Box<TypeExpr>),
    Option(Box<TypeExpr>),
    Void,
    Untyped,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Val {
        name: String,
        mutable: bool,
        ty: Option<TypeExpr>,
        expr: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Return(Option<Expr>),
    Expr(Expr),
    Pre(Vec<Contract>),
    Post(Vec<Contract>),
    If {
        cond: Expr,
        then: Vec<Stmt>,
        elif_: Vec<(Expr, Vec<Stmt>)>,
        else_: Option<Vec<Stmt>>,
    },
    For {
        init: Option<(String, Expr)>, // var = expr
        cond: Option<Expr>,           // None = infinite
        post: Option<Box<Stmt>>,      // e.g. y++
        body: Vec<Stmt>,
    },
    Increment(String), // x++
    Defer(Vec<Stmt>),
    Break,
    Assert(Expr, usize),
    Match {
        expr: Expr,
        some_binding: String,
        some_body: Vec<Stmt>,
        none_body: Vec<Stmt>,
    },
    When {
        platform: String,
        body: Vec<Stmt>,
    },
    /// `when <expr> is <type> { ... }` — union type dispatch
    WhenIs {
        expr: Expr,
        ty: TypeExpr,
        body: Vec<Stmt>,
    },
    /// `otherwise { ... }` — fallback for union when/is chains
    Otherwise {
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub label: Option<String>,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    Ident(String),
    Bool(bool),
    Some(Box<Expr>),
    None,
    Field(Box<Expr>, String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        line: usize,
    },
    Builtin {
        name: String,
        args: Vec<Expr>,
    },
    Trust(Box<Expr>),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
    },
    StructLit {
        fields: Vec<(String, Expr)>,
    },
    /// Positional args pack: `{expr, expr, ...}` — used for varargs-style call arguments
    ArgsPack(Vec<Expr>),
    /// Type cast: `expr as TypeName`
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub enum UnOp {
    Not,
    Neg,
    BitwiseNot,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    filename: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            filename: String::new(),
        }
    }

    pub fn with_filename(tokens: Vec<Token>, filename: String) -> Self {
        Self {
            tokens,
            pos: 0,
            filename,
        }
    }

    fn error(&self, msg: &str) -> String {
        let tok = &self.tokens[self.pos];
        if self.filename.is_empty() {
            format!("{}:{}: {}", tok.line, tok.col, msg)
        } else {
            format!("{}:{}:{}: {}", self.filename, tok.line, tok.col, msg)
        }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_token(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("expected {:?}, got {:?}", expected, self.peek())))
        }
    }

    fn eat(&mut self, tok: &TokenKind) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_module(&mut self) -> Result<Module, String> {
        let mut items = Vec::new();
        while self.peek() != &TokenKind::Eof {
            items.push(self.parse_item()?);
        }
        Ok(Module { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        if let TokenKind::Test = self.peek() {
            return self.parse_test();
        }

        let public = self.eat(&TokenKind::Pub);

        if let TokenKind::Extern = self.peek() {
            return self.parse_extern_fn(public);
        }

        match self.peek().clone() {
            TokenKind::Fn => self.parse_fn(public),
            TokenKind::Val => self.parse_val_item(public),
            TokenKind::Type => self.parse_type_def(public),
            TokenKind::Union => self.parse_union_def(public),
            TokenKind::Enum => self.parse_enum_def(public),
            TokenKind::Use => self.parse_use(),
            other => Err(self.error(&format!("unexpected token at top level: {other:?}"))),
        }
    }

    fn parse_test(&mut self) -> Result<Item, String> {
        self.expect(&TokenKind::Test)?;
        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_stmts()?;
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::Test { body })
    }

    fn parse_use(&mut self) -> Result<Item, String> {
        self.expect(&TokenKind::Use)?;
        let name = self.expect_ident()?;
        let path = self.expect_string()?;
        Ok(Item::Use { name, path })
    }

    fn parse_extern_fn(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Extern)?;
        self.expect(&TokenKind::LParen)?;
        let conv = self.expect_ident()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        let (ret, trusted) = self.parse_ret_type()?;
        if !trusted {
            return Err(self.error(&format!(
                "extern fn '{name}': return type must be marked '!' — \
                 extern functions cannot be formally verified"
            )));
        }
        self.expect(&TokenKind::Eq)?;
        let symbol = self.expect_string()?;
        Ok(Item::ExternFn { public, conv, name, params, ret, symbol })
    }

    fn parse_fn(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Fn)?;

        let namespace = if self.peek() == &TokenKind::LParen {
            let is_ns = matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            ) && matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::RParen)
            );
            if is_ns {
                self.advance();
                let type_name = self.expect_ident()?;
                self.expect(&TokenKind::RParen)?;
                Some(type_name)
            } else {
                None
            }
        } else {
            None
        };

        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params_with_self(namespace.as_deref())?;
        self.expect(&TokenKind::RParen)?;
        let (ret, trusted) = self.parse_ret_type()?;
        self.expect(&TokenKind::Eq)?;
        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_stmts()?;
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::Fn(FnDef {
            public,
            namespace,
            name,
            params,
            ret,
            trusted,
            body,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<(String, TypeExpr)>, String> {
        self.parse_params_with_self(None)
    }

    fn parse_params_with_self(
        &mut self,
        self_type: Option<&str>,
    ) -> Result<Vec<(String, TypeExpr)>, String> {
        let mut params = Vec::new();
        while self.peek() != &TokenKind::RParen {
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let ty = if let (Some(type_name), TypeExpr::Named(n)) = (self_type, &ty) {
                if n == "Self" {
                    TypeExpr::Named(type_name.to_string())
                } else {
                    ty
                }
            } else {
                ty
            };
            params.push((name, ty));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_ret_type(&mut self) -> Result<(TypeExpr, bool), String> {
        let ty = self.parse_type()?;
        let trusted = self.eat(&TokenKind::Bang);
        Ok((ty, trusted))
    }

    fn parse_type(&mut self) -> Result<TypeExpr, String> {
        match self.peek().clone() {
            TokenKind::LBracket => {
                self.advance();
                self.expect(&TokenKind::RBracket)?;
                let inner = self.parse_type()?;
                Ok(TypeExpr::Slice(Box::new(inner)))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if name == "option" {
                    self.expect(&TokenKind::LBracket)?;
                    let inner = self.parse_type()?;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::Option(Box::new(inner)))
                } else if name == "void" {
                    Ok(TypeExpr::Void)
                } else if name == "untyped" {
                    Ok(TypeExpr::Untyped)
                } else {
                    Ok(TypeExpr::Named(name))
                }
            }
            TokenKind::Mut => {
                self.advance();
                self.parse_type()
            }
            TokenKind::Ref => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(TypeExpr::Ref(Box::new(inner)))
            }
            other => Err(self.error(&format!("expected type, got {other:?}"))),
        }
    }

    fn parse_val_item(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Val)?;
        let name = self.expect_ident()?;
        if self.eat(&TokenKind::Colon) {
            self.parse_type()?;
        }
        self.expect(&TokenKind::Eq)?;

        if self.peek() == &TokenKind::Use {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            let path = self.expect_string()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(Item::Use { name, path });
        }

        let expr = self.parse_expr()?;
        Ok(Item::Const { public, name, expr })
    }

    fn parse_type_def(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Type)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let mutable = self.eat(&TokenKind::Mut);
            let ty = self.parse_type()?;
            fields.push(Field {
                name: fname,
                mutable,
                ty,
            });
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::TypeDef { public, name, fields })
    }

    fn parse_union_def(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Union)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            variants.push(self.parse_type()?);
            if !self.eat(&TokenKind::BitOr) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::UnionDef { public, name, variants })
    }

    fn parse_enum_def(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            variants.push(self.expect_ident()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::EnumDef { public, name, variants })
    }

    fn parse_stmts(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            TokenKind::Val => {
                self.advance();
                let name = self.expect_ident()?;
                let (mutable, ty) = if self.eat(&TokenKind::Colon) {
                    let m = self.eat(&TokenKind::Mut);
                    let t = self.parse_type()?;
                    (m, Some(t))
                } else {
                    (false, None)
                };
                self.expect(&TokenKind::Eq)?;
                let expr = self.parse_expr()?;
                Ok(Stmt::Val {
                    name,
                    mutable,
                    ty,
                    expr,
                })
            }
            TokenKind::Return => {
                self.advance();
                if self.peek() == &TokenKind::RBrace || self.peek() == &TokenKind::Eof {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            TokenKind::Pre => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let contracts = self.parse_contracts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Pre(contracts))
            }
            TokenKind::Post => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let contracts = self.parse_contracts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Post(contracts))
            }
            TokenKind::If => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::LBrace)?;
                let then = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                let mut elif_ = Vec::new();
                loop {
                    if self.peek() == &TokenKind::Elif {
                        self.advance();
                        self.expect(&TokenKind::LParen)?;
                        let elif_cond = self.parse_expr()?;
                        self.expect(&TokenKind::RParen)?;
                        self.expect(&TokenKind::LBrace)?;
                        let elif_body = self.parse_stmts()?;
                        self.expect(&TokenKind::RBrace)?;
                        elif_.push((elif_cond, elif_body));
                    } else {
                        break;
                    }
                }
                let else_ = if self.eat(&TokenKind::Else) {
                    self.expect(&TokenKind::LBrace)?;
                    let s = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    Some(s)
                } else {
                    None
                };
                Ok(Stmt::If {
                    cond,
                    then,
                    elif_,
                    else_,
                })
            }
            TokenKind::For => {
                self.advance();
                if self.peek() == &TokenKind::LBrace {
                    self.advance();
                    let body = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    return Ok(Stmt::For {
                        init: None,
                        cond: None,
                        post: None,
                        body,
                    });
                }
                self.expect(&TokenKind::LParen)?;
                let init_name = self.expect_ident()?;
                self.expect(&TokenKind::Eq)?;
                let init_expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                let post_name = self.expect_ident()?;
                self.expect(&TokenKind::PlusPlus)?;
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::LBrace)?;
                let body = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::For {
                    init: Some((init_name.clone(), init_expr)),
                    cond: Some(cond),
                    post: Some(Box::new(Stmt::Increment(post_name))),
                    body,
                })
            }
            TokenKind::Defer => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let body = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Defer(body))
            }
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenKind::When => {
                self.advance();
                let is_when_is = matches!(self.peek(), TokenKind::Ident(_))
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Is);
                if is_when_is {
                    let expr = self.parse_expr()?;
                    self.expect(&TokenKind::Is)?;
                    let ty = self.parse_type()?;
                    self.expect(&TokenKind::LBrace)?;
                    let body = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::WhenIs { expr, ty, body })
                } else {
                    let platform = self.expect_ident()?;
                    self.expect(&TokenKind::LBrace)?;
                    let body = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::When { platform, body })
                }
            }
            TokenKind::Otherwise => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let body = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Otherwise { body })
            }
            TokenKind::Assert => {
                self.advance();
                let line = self.peek_token().line;
                let expr = self.parse_expr()?;
                Ok(Stmt::Assert(expr, line))
            }
            TokenKind::Match => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::LBrace)?;
                let mut some_binding = None;
                let mut some_body = None;
                let mut none_body = None;
                for _ in 0..2 {
                    match self.peek().clone() {
                        TokenKind::Some => {
                            self.advance();
                            let binding = self.expect_ident()?;
                            self.expect(&TokenKind::FatArrow)?;
                            self.expect(&TokenKind::LBrace)?;
                            let body = self.parse_stmts()?;
                            self.expect(&TokenKind::RBrace)?;
                            some_binding = Some(binding);
                            some_body = Some(body);
                        }
                        TokenKind::None_ => {
                            self.advance();
                            self.expect(&TokenKind::FatArrow)?;
                            self.expect(&TokenKind::LBrace)?;
                            let body = self.parse_stmts()?;
                            self.expect(&TokenKind::RBrace)?;
                            none_body = Some(body);
                        }
                        _ => break,
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Match {
                    expr,
                    some_binding: some_binding
                        .ok_or_else(|| "match: missing 'some' arm".to_string())?,
                    some_body: some_body.unwrap_or_default(),
                    none_body: none_body.ok_or_else(|| "match: missing 'none' arm".to_string())?,
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.eat(&TokenKind::Eq) {
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign {
                        target: expr,
                        value,
                    })
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_contracts(&mut self) -> Result<Vec<Contract>, String> {
        let mut contracts = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            let contract = if let TokenKind::Ident(name) = self.peek().clone() {
                if self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Colon) {
                    self.advance();
                    self.advance();
                    let expr = self.parse_expr()?;
                    Contract {
                        label: Some(name),
                        expr,
                    }
                } else {
                    Contract {
                        label: None,
                        expr: self.parse_expr()?,
                    }
                }
            } else {
                Contract {
                    label: None,
                    expr: self.parse_expr()?,
                }
            };
            contracts.push(contract);
        }
        Ok(contracts)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while self.peek() == &TokenKind::Or {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::BinOp {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitor()?;
        while self.peek() == &TokenKind::And {
            self.advance();
            let rhs = self.parse_bitor()?;
            lhs = Expr::BinOp {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == &TokenKind::BitOr {
            self.advance();
            let rhs = self.parse_bitand()?;
            lhs = Expr::BinOp {
                op: BinOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitand(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_shift()?;
        while self.peek() == &TokenKind::BitAnd {
            self.advance();
            let rhs = self.parse_shift()?;
            lhs = Expr::BinOp {
                op: BinOp::BitAnd,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cmp()?;
        loop {
            let op = match self.peek() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_cmp()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_add()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cast()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_cast()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cast(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        while self.eat(&TokenKind::As) {
            let ty = self.parse_type()?;
            expr = Expr::Cast {
                expr: Box::new(expr),
                ty,
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            TokenKind::Not | TokenKind::Bang => {
                self.advance();
                Ok(Expr::UnOp {
                    op: UnOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Minus => {
                self.advance();
                Ok(Expr::UnOp {
                    op: UnOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Tilde => {
                self.advance();
                Ok(Expr::UnOp {
                    op: UnOp::BitwiseNot,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Trust => {
                self.advance();
                Ok(Expr::Trust(Box::new(self.parse_call_chain()?)))
            }
            TokenKind::Some => {
                self.advance();
                Ok(Expr::Some(Box::new(self.parse_unary()?)))
            }
            TokenKind::None_ => {
                self.advance();
                Ok(Expr::None)
            }
            _ => self.parse_call_chain(),
        }
    }

    fn parse_call_chain(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::Dot) {
                let field = self.expect_ident()?;
                expr = Expr::Field(Box::new(expr), field);
            } else if self.peek() == &TokenKind::LParen {
                let call_line = self.peek_token().line;
                self.advance();
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    line: call_line,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
            args.push(self.parse_expr()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expr::FloatLit(f))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StrLit(s))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            TokenKind::At => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Builtin { name, args })
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBrace => {
                self.advance();
                let is_struct_lit = matches!(self.peek(), TokenKind::Ident(_))
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Colon);

                if is_struct_lit {
                    let mut fields = Vec::new();
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        let fname = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let val = self.parse_expr()?;
                        fields.push((fname, val));
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::StructLit { fields })
                } else {
                    let mut exprs = Vec::new();
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        exprs.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::ArgsPack(exprs))
                }
            }
            other => Err(self.error(&format!("unexpected token in expression: {other:?}"))),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.error(&format!("expected identifier, got {other:?}"))),
        }
    }

    fn expect_string(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.error(&format!("expected string literal, got {other:?}"))),
        }
    }
}
