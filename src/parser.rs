use crate::lexer::Token;

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
        name: String,
        fields: Vec<Field>,
    },
    Const {
        public: bool,
        name: String,
        expr: Expr,
    },
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub public: bool,
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
        else_: Option<Vec<Stmt>>,
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
    StrLit(String),
    Ident(String),
    Bool(bool),
    Some(Box<Expr>),
    None,
    Field(Box<Expr>, String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
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
}

#[derive(Debug, Clone)]
pub enum UnOp {
    Not,
    Neg,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {:?}", self.peek()))
        }
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_module(&mut self) -> Result<Module, String> {
        let mut items = Vec::new();
        while self.peek() != &Token::Eof {
            items.push(self.parse_item()?);
        }
        Ok(Module { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        let public = self.eat(&Token::Pub);

        match self.peek().clone() {
            Token::Fn => self.parse_fn(public),
            Token::Val => self.parse_val_item(public),
            Token::Type => self.parse_type_def(),
            other => Err(format!("unexpected token at top level: {other:?}")),
        }
    }

    fn parse_fn(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let (ret, trusted) = self.parse_ret_type()?;
        self.expect(&Token::Eq)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_stmts()?;
        self.expect(&Token::RBrace)?;
        Ok(Item::Fn(FnDef {
            public,
            name,
            params,
            ret,
            trusted,
            body,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<(String, TypeExpr)>, String> {
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let ty = self.parse_type()?;
            params.push((name, ty));
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_ret_type(&mut self) -> Result<(TypeExpr, bool), String> {
        let ty = self.parse_type()?;
        let trusted = self.eat(&Token::Bang);
        Ok((ty, trusted))
    }

    fn parse_type(&mut self) -> Result<TypeExpr, String> {
        match self.peek().clone() {
            Token::LBracket => {
                self.advance();
                self.expect(&Token::RBracket)?;
                let inner = self.parse_type()?;
                Ok(TypeExpr::Slice(Box::new(inner)))
            }
            Token::Ident(name) => {
                self.advance();
                if name == "option" {
                    self.expect(&Token::LBracket)?;
                    let inner = self.parse_type()?;
                    self.expect(&Token::RBracket)?;
                    Ok(TypeExpr::Option(Box::new(inner)))
                } else if name == "void" {
                    Ok(TypeExpr::Void)
                } else if name == "untyped" {
                    Ok(TypeExpr::Untyped)
                } else {
                    Ok(TypeExpr::Named(name))
                }
            }
            Token::Mut => {
                self.advance();
                self.parse_type()
            }
            Token::Ref => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(TypeExpr::Ref(Box::new(inner)))
            }
            other => Err(format!("expected type, got {other:?}")),
        }
    }

    fn parse_val_item(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&Token::Val)?;
        let name = self.expect_ident()?;
        self.expect(&Token::Eq)?;

        if self.peek() == &Token::Use {
            self.advance();
            self.expect(&Token::LParen)?;
            let path = self.expect_string()?;
            self.expect(&Token::RParen)?;
            return Ok(Item::Use { name, path });
        }

        let expr = self.parse_expr()?;
        Ok(Item::Const { public, name, expr })
    }

    fn parse_type_def(&mut self) -> Result<Item, String> {
        self.expect(&Token::Type)?;
        let name = self.expect_ident()?;
        self.expect(&Token::Eq)?;
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace {
            let fname = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let mutable = self.eat(&Token::Mut);
            let ty = self.parse_type()?;
            fields.push(Field {
                name: fname,
                mutable,
                ty,
            });
        }
        self.expect(&Token::RBrace)?;
        Ok(Item::TypeDef { name, fields })
    }

    fn parse_stmts(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Val => {
                self.advance();
                let name = self.expect_ident()?;
                let (mutable, ty) = if self.eat(&Token::Colon) {
                    let m = self.eat(&Token::Mut);
                    let t = self.parse_type()?;
                    (m, Some(t))
                } else {
                    (false, None)
                };
                self.expect(&Token::Eq)?;
                let expr = self.parse_expr()?;
                Ok(Stmt::Val {
                    name,
                    mutable,
                    ty,
                    expr,
                })
            }
            Token::Return => {
                self.advance();
                if self.peek() == &Token::RBrace || self.peek() == &Token::Eof {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            Token::Pre => {
                self.advance();
                self.expect(&Token::LBrace)?;
                let contracts = self.parse_contracts()?;
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Pre(contracts))
            }
            Token::Post => {
                self.advance();
                self.expect(&Token::LBrace)?;
                let contracts = self.parse_contracts()?;
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Post(contracts))
            }
            Token::If => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::LBrace)?;
                let then = self.parse_stmts()?;
                self.expect(&Token::RBrace)?;
                let else_ = if self.eat(&Token::Else) {
                    self.expect(&Token::LBrace)?;
                    let s = self.parse_stmts()?;
                    self.expect(&Token::RBrace)?;
                    Some(s)
                } else {
                    None
                };
                Ok(Stmt::If { cond, then, else_ })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.eat(&Token::Eq) {
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
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let contract = if let Token::Ident(name) = self.peek().clone() {
                if self.tokens.get(self.pos + 1) == Some(&Token::Colon) {
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
        while self.peek() == &Token::Or {
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
        let mut lhs = self.parse_cmp()?;
        while self.peek() == &Token::And {
            self.advance();
            let rhs = self.parse_cmp()?;
            lhs = Expr::BinOp {
                op: BinOp::And,
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
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::Le,
                Token::GtEq => BinOp::Ge,
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
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
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
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Not => {
                self.advance();
                Ok(Expr::UnOp {
                    op: UnOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Token::Minus => {
                self.advance();
                Ok(Expr::UnOp {
                    op: UnOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Token::Trust => {
                self.advance();
                Ok(Expr::Trust(Box::new(self.parse_call_chain()?)))
            }
            Token::Some => {
                self.advance();
                Ok(Expr::Some(Box::new(self.parse_call_chain()?)))
            }
            Token::None_ => {
                self.advance();
                Ok(Expr::None)
            }
            _ => self.parse_call_chain(),
        }
    }

    fn parse_call_chain(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(&Token::Dot) {
                let field = self.expect_ident()?;
                expr = Expr::Field(Box::new(expr), field);
            } else if self.peek() == &Token::LParen {
                self.advance();
                let args = self.parse_call_args()?;
                self.expect(&Token::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            args.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::StrLit(s))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::At => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::LParen)?;
                let args = self.parse_call_args()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::Builtin { name, args })
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }
            Token::LBrace => {
                self.advance();
                let is_struct_lit = matches!(self.peek(), Token::Ident(_))
                    && self.tokens.get(self.pos + 1) == Some(&Token::Colon);

                if is_struct_lit {
                    let mut fields = Vec::new();
                    while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                        let fname = self.expect_ident()?;
                        self.expect(&Token::Colon)?;
                        let val = self.parse_expr()?;
                        fields.push((fname, val));
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::StructLit { fields })
                } else {
                    let mut exprs = Vec::new();
                    while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                        exprs.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::ArgsPack(exprs))
                }
            }
            other => Err(format!("unexpected token in expression: {other:?}")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(format!("expected identifier, got {other:?}")),
        }
    }

    fn expect_string(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            Token::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(format!("expected string literal, got {other:?}")),
        }
    }
}
