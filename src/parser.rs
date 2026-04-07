use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Use {
        public: bool,
        name: String,
        path: String,
    },
    Fn(FnDef),
    TypeDef {
        public: bool,
        name: String,
        fields: Vec<Field>,
    },
    ExternTypeDef {
        public: bool,
        name: String,
        fields: Vec<Field>,
    },
    UnionDef {
        public: bool,
        name: String,
        variants: Vec<TypeExpr>,
    },
    TaggedUnionDef {
        public: bool,
        name: String,
        variants: Vec<(String, Vec<TypeExpr>)>,
    },
    EnumDef {
        public: bool,
        name: String,
        variants: Vec<String>,
    },
    Global {
        public: bool,
        name: String,
        ty: Option<TypeExpr>,
        mutable: bool,
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
        /// Number of fixed parameters before the variadic `...` (None = not variadic)
        variadic_after: Option<usize>,
        ret: TypeExpr,
        /// The external symbol name, e.g. "malloc"
        symbol: String,
    },
    /// `link "libname"` — request the linker to link an external library
    Link {
        lib: String,
    },
    /// `when <platform> { link "..." ... }` — platform-conditional link flags
    LinkWhen {
        platform: String,
        libs: Vec<String>,
    },
    /// `when <platform> { ... items ... }` — platform-conditional item declarations
    /// Supports `or when <platform> { }` chaining and a final `otherwise { }` fallback
    WhenItems {
        platform: String,
        items: Vec<Item>,
        /// Additional branches checked in order if the first doesn't match
        or_when: Vec<(String, Vec<Item>)>,
        /// Items to use if no branch matched
        otherwise: Vec<Item>,
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
    /// Some named type
    Named(String),

    /// `[T]` — slice type (fat pointer: data + length)
    Slice(Box<TypeExpr>),

    /// `[N]T` — fixed-size array of N elements of type T (inline, no indirection)
    FixedArray(u64, Box<TypeExpr>),

    /// Some ref type
    Ref(Box<TypeExpr>),

    /// Some optional type
    Option(Box<TypeExpr>),

    /// `list[T]` — dynamic list type with element type T
    List(Box<TypeExpr>),

    /// `result[T, E]` — result type with ok payload T and err payload E
    Result(Box<TypeExpr>, Box<TypeExpr>),

    /// `fn(T, T) R` — function pointer type
    FnPtr {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },

    /// `mut T` — mutable parameter (e.g. `s: mut self`)
    Mut(Box<TypeExpr>),

    /// The void type
    Void,

    /// The untyped (nothing) type
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
    GuardedPre(Vec<Contract>),
    GuardedPost(Vec<Contract>),
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
    /// `for x in iterable { ... }` — for-in loop over a list
    ForIn {
        binding: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Increment(String),       // x++
    Decrement(String),       // x--
    AddAssign(String, Expr), // x += expr
    SubAssign(String, Expr), // x -= expr
    Defer(Vec<Stmt>),
    Break,
    Continue,
    Assert(Expr, usize),
    Match {
        expr: Expr,
        some_binding: String,
        some_body: Vec<Stmt>,
        none_body: Vec<Stmt>,
    },
    /// `match expr { ok v => { ... } err e => { ... } }` — match on result type
    MatchResult {
        expr: Expr,
        ok_binding: String,
        ok_body: Vec<Stmt>,
        err_binding: String,
        err_body: Vec<Stmt>,
    },
    /// `match expr { EnumType.Variant => { ... } ... }` — match on enum value
    MatchEnum {
        expr: Expr,
        arms: Vec<(String, Vec<Stmt>)>, // (variant_name, body)
    },
    /// `when <platform> { ... }` — platform-conditional statements
    When {
        platform: String,
        body: Vec<Stmt>,
        or_when: Vec<(String, Vec<Stmt>)>,
        otherwise: Vec<Stmt>,
    },
    /// `match expr { TypeName => { ... } else => { ... } }` — union type dispatch
    MatchUnion {
        expr: Expr,
        arms: Vec<(TypeExpr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    /// `match expr { Variant binding => { ... } else => { ... } }` — tagged union dispatch
    MatchTaggedUnion {
        expr: Expr,
        /// (variant_name, bindings, body) — bindings are variable names or "_" to discard
        arms: Vec<(String, Vec<String>, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    /// `match expr { "string" => { ... } ... }` — match on string/char pointer value
    MatchString {
        expr: Expr,
        arms: Vec<(String, Vec<Stmt>)>, // (string_literal, body)
        else_body: Option<Vec<Stmt>>,
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
    /// `ok expr` — wrap a value in the ok variant of result
    OkVal(Box<Expr>),
    /// `err expr` — wrap a value in the err variant of result
    ErrVal(Box<Expr>),
    /// `expr?` — propagate error: if err, return err; if ok, unwrap
    Try(Box<Expr>),
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
    /// List literal: `[expr, expr, ...]` or `[]`
    ListLit(Vec<Expr>),
    /// Positional args pack: `{expr, expr, ...}` — used for varargs-style call arguments
    ArgsPack(Vec<Expr>),
    /// `TypeName{}` — zero-initialize all fields of a named struct type
    ZeroInit(String),
    /// Type cast: `expr as TypeName`
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
    },
    /// `addr(expr)` — take the address of a function or variable
    Addr(Box<Expr>),
    /// `deref(expr)` — dereference a pointer
    Deref(Box<Expr>),
    /// `@sizeof(TypeExpr)` — size of a type (supports generic types like list[T])
    SizeofType(TypeExpr),
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
    BitXor,
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
    pub warnings: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            filename: String::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_filename(tokens: Vec<Token>, filename: String) -> Self {
        Self {
            tokens,
            pos: 0,
            filename,
            warnings: Vec::new(),
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

        if self.peek() == &TokenKind::Link {
            self.advance();
            let lib = self.expect_string()?;
            return Ok(Item::Link { lib });
        }

        if self.peek() == &TokenKind::When {
            self.advance();
            let platform = self.expect_ident()?;
            self.expect(&TokenKind::LBrace)?;
            let is_link_only = matches!(self.peek(), TokenKind::Link);
            if is_link_only {
                let mut libs = Vec::new();
                while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                    if self.peek() == &TokenKind::Link {
                        self.advance();
                        libs.push(self.expect_string()?);
                    } else {
                        return Err(self.error("expected 'link' inside top-level 'when' block"));
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                return Ok(Item::LinkWhen { platform, libs });
            }
            let mut items = Vec::new();
            while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                items.push(self.parse_item()?);
            }
            self.expect(&TokenKind::RBrace)?;
            let mut or_when: Vec<(String, Vec<Item>)> = Vec::new();
            while self.peek() == &TokenKind::Ident("or".to_string()) {
                if self.tokens.get(self.pos + 1).map(|t| &t.kind) != Some(&TokenKind::When) {
                    break;
                }
                self.advance();
                self.advance();
                let ow_platform = self.expect_ident()?;
                self.expect(&TokenKind::LBrace)?;
                let mut ow_items = Vec::new();
                while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                    ow_items.push(self.parse_item()?);
                }
                self.expect(&TokenKind::RBrace)?;
                or_when.push((ow_platform, ow_items));
            }
            let otherwise = if self.eat(&TokenKind::Otherwise) {
                self.expect(&TokenKind::LBrace)?;
                let mut ow = Vec::new();
                while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                    ow.push(self.parse_item()?);
                }
                self.expect(&TokenKind::RBrace)?;
                ow
            } else {
                Vec::new()
            };
            return Ok(Item::WhenItems {
                platform,
                items,
                or_when,
                otherwise,
            });
        }

        let public = self.eat(&TokenKind::Pub);

        if let TokenKind::Extern = self.peek() {
            self.advance();
            if self.peek() == &TokenKind::Type {
                return self.parse_extern_type(public);
            } else {
                return self.parse_extern_fn_after_extern(public);
            }
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
        Ok(Item::Use {
            public: false,
            name,
            path,
        })
    }

    fn parse_extern_fn_after_extern(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::LParen)?;
        let conv = self.expect_ident()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let (params, variadic_after) = self.parse_extern_params()?;
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
        Ok(Item::ExternFn {
            public,
            conv,
            name,
            params,
            variadic_after,
            ret,
            symbol,
        })
    }

    fn parse_extern_fn(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Extern)?;
        self.parse_extern_fn_after_extern(public)
    }

    fn parse_extern_type(&mut self, public: bool) -> Result<Item, String> {
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
        Ok(Item::ExternTypeDef {
            public,
            name,
            fields,
        })
    }

    /// Parse extern function parameters, recognising a trailing `...` for variadic functions.
    /// Returns the parameter list and the index after which variadic args begin (if any).
    fn parse_extern_params(&mut self) -> Result<(Vec<(String, TypeExpr)>, Option<usize>), String> {
        let mut params = Vec::new();
        let mut variadic_after = None;
        while self.peek() != &TokenKind::RParen {
            if self.peek() == &TokenKind::DotDotDot {
                self.advance();
                variadic_after = Some(params.len());
                break;
            }
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push((name, ty));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok((params, variadic_after))
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
            let is_mut = self.eat(&TokenKind::Mut);
            let ty = self.parse_type()?;
            let ty = if let (Some(type_name), TypeExpr::Named(n)) = (self_type, &ty) {
                if n == "self" {
                    TypeExpr::Named(type_name.to_string())
                } else {
                    ty
                }
            } else {
                ty
            };
            let ty = if is_mut {
                TypeExpr::Mut(Box::new(ty))
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
                // Check if this is `[N]T` (fixed array) or `[]T` (slice)
                if self.peek() == &TokenKind::RBracket {
                    // `[]T` — slice type
                    self.advance();
                    let inner = self.parse_type()?;
                    Ok(TypeExpr::Slice(Box::new(inner)))
                } else {
                    // `[N]T` — fixed-size array
                    if let TokenKind::IntLit(n) = self.peek().clone() {
                        self.advance();
                        self.expect(&TokenKind::RBracket)?;
                        let inner = self.parse_type()?;
                        Ok(TypeExpr::FixedArray(n as u64, Box::new(inner)))
                    } else {
                        Err(self.error("expected integer or ']' after '[' in type"))
                    }
                }
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let mut params = Vec::new();
                while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
                    params.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                let ret = self.parse_type()?;
                Ok(TypeExpr::FnPtr {
                    params,
                    ret: Box::new(ret),
                })
            }
            TokenKind::Ident(name) => {
                self.advance();
                if name == "option" {
                    self.expect(&TokenKind::LBracket)?;
                    let inner = self.parse_type()?;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::Option(Box::new(inner)))
                } else if name == "result" {
                    self.expect(&TokenKind::LBracket)?;
                    let ok_ty = self.parse_type()?;
                    self.expect(&TokenKind::Comma)?;
                    let err_ty = self.parse_type()?;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::Result(Box::new(ok_ty), Box::new(err_ty)))
                } else if name == "list" {
                    self.expect(&TokenKind::LBracket)?;
                    let inner = self.parse_type()?;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::List(Box::new(inner)))
                } else if name == "void" {
                    Ok(TypeExpr::Void)
                } else if name == "untyped" {
                    Ok(TypeExpr::Untyped)
                } else {
                    let mut full_name = name;
                    while self.peek() == &TokenKind::Dot {
                        self.advance();
                        let segment = self.expect_ident()?;
                        full_name.push('.');
                        full_name.push_str(&segment);
                    }
                    Ok(TypeExpr::Named(full_name))
                }
            }
            TokenKind::Mut => {
                self.advance();
                if self.peek() == &TokenKind::Ref {
                    let tok = self.peek_token();
                    let loc = if self.filename.is_empty() {
                        format!("{}:{}", tok.line, tok.col)
                    } else {
                        format!("{}:{}:{}", self.filename, tok.line, tok.col)
                    };
                    self.warnings.push(format!(
                        "{loc}: 'mut ref' is redundant because references are mutable by definition; use 'ref' instead"
                    ));
                }
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
        let (mutable, ty) = if self.eat(&TokenKind::Colon) {
            let m = self.eat(&TokenKind::Mut);
            let t = self.parse_type()?;
            (m, Some(t))
        } else {
            (false, None)
        };
        self.expect(&TokenKind::Eq)?;

        if self.peek() == &TokenKind::Use {
            self.advance();
            self.expect(&TokenKind::LParen)?;
            let path = self.expect_string()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(Item::Use { public, name, path });
        }

        let expr = self.parse_expr()?;
        Ok(Item::Global {
            public,
            name,
            ty,
            mutable,
            expr,
        })
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
        Ok(Item::TypeDef {
            public,
            name,
            fields,
        })
    }

    fn parse_union_def(&mut self, public: bool) -> Result<Item, String> {
        self.expect(&TokenKind::Union)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        self.expect(&TokenKind::LBrace)?;

        let is_tagged = matches!(self.peek(), TokenKind::Ident(_))
            && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::LParen);

        if is_tagged {
            let mut variants: Vec<(String, Vec<TypeExpr>)> = Vec::new();
            while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                let variant_name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let mut fields = Vec::new();
                while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
                    fields.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                variants.push((variant_name, fields));
                if !self.eat(&TokenKind::BitOr) {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace)?;
            Ok(Item::TaggedUnionDef {
                public,
                name,
                variants,
            })
        } else {
            let mut variants = Vec::new();
            while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                let ty = self.parse_type()?;
                if !is_primitive_type(&ty) {
                    return Err(format!(
                        "union '{name}': non-primitive type in untagged union — \
                         use tagged union syntax instead: `VariantName(Type)`"
                    ));
                }
                variants.push(ty);
                if !self.eat(&TokenKind::BitOr) {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace)?;
            Ok(Item::UnionDef {
                public,
                name,
                variants,
            })
        }
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
        Ok(Item::EnumDef {
            public,
            name,
            variants,
        })
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
            TokenKind::Guarded => {
                self.advance();
                match self.peek().clone() {
                    TokenKind::Pre => {
                        self.advance();
                        self.expect(&TokenKind::LBrace)?;
                        let contracts = self.parse_contracts()?;
                        self.expect(&TokenKind::RBrace)?;
                        Ok(Stmt::GuardedPre(contracts))
                    }
                    TokenKind::Post => {
                        self.advance();
                        self.expect(&TokenKind::LBrace)?;
                        let contracts = self.parse_contracts()?;
                        self.expect(&TokenKind::RBrace)?;
                        Ok(Stmt::GuardedPost(contracts))
                    }
                    other => Err(self.error(&format!(
                        "'guarded' must be followed by 'pre' or 'post', got {other:?}"
                    ))),
                }
            }
            TokenKind::If => {
                self.advance();
                let has_paren = self.eat(&TokenKind::LParen);
                let cond = self.parse_expr()?;
                if has_paren {
                    self.expect(&TokenKind::RParen)?;
                }
                self.expect(&TokenKind::LBrace)?;
                let then = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                let mut elif_ = Vec::new();
                loop {
                    if self.peek() == &TokenKind::Elif {
                        self.advance();
                        let elif_has_paren = self.eat(&TokenKind::LParen);
                        let elif_cond = self.parse_expr()?;
                        if elif_has_paren {
                            self.expect(&TokenKind::RParen)?;
                        }
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
                let has_paren = self.eat(&TokenKind::LParen);

                let first_name = self.expect_ident()?;
                if self.eat(&TokenKind::In) {
                    let iterable = self.parse_expr()?;
                    if has_paren {
                        self.expect(&TokenKind::RParen)?;
                    }
                    self.expect(&TokenKind::LBrace)?;
                    let body = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    return Ok(Stmt::ForIn {
                        binding: first_name,
                        iterable,
                        body,
                    });
                }

                self.expect(&TokenKind::Eq)?;
                let init_expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                let post_name = self.expect_ident()?;
                let post_stmt = if self.eat(&TokenKind::PlusPlus) {
                    Stmt::Increment(post_name)
                } else if self.eat(&TokenKind::MinusMinus) {
                    Stmt::Decrement(post_name.clone())
                } else if self.eat(&TokenKind::PlusEq) {
                    let v = self.parse_expr()?;
                    Stmt::AddAssign(post_name, v)
                } else if self.eat(&TokenKind::MinusEq) {
                    let v = self.parse_expr()?;
                    Stmt::SubAssign(post_name, v)
                } else {
                    return Err("expected '++', '--', '+=', or '-=' in for loop post".to_string());
                };
                if has_paren {
                    self.expect(&TokenKind::RParen)?;
                }
                self.expect(&TokenKind::LBrace)?;
                let body = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::For {
                    init: Some((first_name.clone(), init_expr)),
                    cond: Some(cond),
                    post: Some(Box::new(post_stmt)),
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
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            TokenKind::When => {
                self.advance();
                let platform = self.expect_ident()?;
                self.expect(&TokenKind::LBrace)?;
                let body = self.parse_stmts()?;
                self.expect(&TokenKind::RBrace)?;
                let mut or_when: Vec<(String, Vec<Stmt>)> = Vec::new();
                while self.peek() == &TokenKind::Ident("or".to_string()) {
                    if self.tokens.get(self.pos + 1).map(|t| &t.kind) != Some(&TokenKind::When) {
                        break;
                    }
                    self.advance();
                    self.advance();
                    let ow_platform = self.expect_ident()?;
                    self.expect(&TokenKind::LBrace)?;
                    let ow_body = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    or_when.push((ow_platform, ow_body));
                }
                let otherwise = if self.eat(&TokenKind::Otherwise) {
                    self.expect(&TokenKind::LBrace)?;
                    let ow = self.parse_stmts()?;
                    self.expect(&TokenKind::RBrace)?;
                    ow
                } else {
                    Vec::new()
                };
                Ok(Stmt::When {
                    platform,
                    body,
                    or_when,
                    otherwise,
                })
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

                // Need to fucking look AHEAD to see if this is a result match (ok/err) or option match (some/none)
                // ...or an enum match (EnumType.Variant => ...)
                // ...or a union match (TypeName => ...)
                // ...or a string match ("string" => ...)
                let is_result_match = matches!(self.peek(), TokenKind::Ok | TokenKind::Err);
                let is_enum_match = !is_result_match
                    && matches!(self.peek(), TokenKind::Ident(_))
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Dot);
                let is_string_match = !is_result_match
                    && !is_enum_match
                    && matches!(self.peek(), TokenKind::StringLit(_));
                let is_union_match = !is_result_match
                    && !is_enum_match
                    && !is_string_match
                    && (matches!(self.peek(), TokenKind::Ref)
                        || (matches!(self.peek(), TokenKind::Ident(_))
                            && self.tokens.get(self.pos + 1).map(|t| &t.kind)
                                == Some(&TokenKind::FatArrow))
                        || (matches!(self.peek(), TokenKind::Else)));
                let is_tagged_union_match = !is_result_match
                    && !is_enum_match
                    && !is_string_match
                    && !is_union_match
                    && (matches!(self.peek(), TokenKind::Ident(_))
                        && matches!(
                            self.tokens.get(self.pos + 1).map(|t| &t.kind),
                            Some(TokenKind::Ident(_))
                            | Some(TokenKind::LParen)
                        ));

                if is_result_match {
                    let mut ok_binding = None;
                    let mut ok_body = None;
                    let mut err_binding = None;
                    let mut err_body = None;
                    for _ in 0..2 {
                        match self.peek().clone() {
                            TokenKind::Ok => {
                                self.advance();
                                let binding = self.expect_ident()?;
                                self.expect(&TokenKind::FatArrow)?;
                                self.expect(&TokenKind::LBrace)?;
                                let body = self.parse_stmts()?;
                                self.expect(&TokenKind::RBrace)?;
                                ok_binding = Some(binding);
                                ok_body = Some(body);
                            }
                            TokenKind::Err => {
                                self.advance();
                                let binding = self.expect_ident()?;
                                self.expect(&TokenKind::FatArrow)?;
                                self.expect(&TokenKind::LBrace)?;
                                let body = self.parse_stmts()?;
                                self.expect(&TokenKind::RBrace)?;
                                err_binding = Some(binding);
                                err_body = Some(body);
                            }
                            _ => break,
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::MatchResult {
                        expr,
                        ok_binding: ok_binding
                            .ok_or_else(|| "match: missing 'ok' arm".to_string())?,
                        ok_body: ok_body.unwrap_or_default(),
                        err_binding: err_binding
                            .ok_or_else(|| "match: missing 'err' arm".to_string())?,
                        err_body: err_body.unwrap_or_default(),
                    })
                } else if is_enum_match {
                    let mut arms = Vec::new();
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        let _enum_type = self.expect_ident()?;
                        self.expect(&TokenKind::Dot)?;
                        let variant = self.expect_ident()?;
                        self.expect(&TokenKind::FatArrow)?;
                        self.expect(&TokenKind::LBrace)?;
                        let body = self.parse_stmts()?;
                        self.expect(&TokenKind::RBrace)?;
                        arms.push((variant, body));
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::MatchEnum { expr, arms })
                } else if is_string_match {
                    let mut arms: Vec<(String, Vec<Stmt>)> = Vec::new();
                    let mut else_body = None;
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        if self.peek() == &TokenKind::Else {
                            self.advance();
                            self.expect(&TokenKind::FatArrow)?;
                            self.expect(&TokenKind::LBrace)?;
                            else_body = Some(self.parse_stmts()?);
                            self.expect(&TokenKind::RBrace)?;
                            break;
                        }
                        let pattern = match self.peek().clone() {
                            TokenKind::StringLit(s) => {
                                self.advance();
                                s
                            }
                            _ => break,
                        };
                        self.expect(&TokenKind::FatArrow)?;
                        self.expect(&TokenKind::LBrace)?;
                        let body = self.parse_stmts()?;
                        self.expect(&TokenKind::RBrace)?;
                        arms.push((pattern, body));
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::MatchString {
                        expr,
                        arms,
                        else_body,
                    })
                } else if is_union_match {
                    let mut arms: Vec<(TypeExpr, Vec<Stmt>)> = Vec::new();
                    let mut else_body = None;
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        if self.peek() == &TokenKind::Else {
                            self.advance();
                            self.expect(&TokenKind::FatArrow)?;
                            self.expect(&TokenKind::LBrace)?;
                            else_body = Some(self.parse_stmts()?);
                            self.expect(&TokenKind::RBrace)?;
                            break;
                        }
                        let ty = self.parse_type()?;
                        self.expect(&TokenKind::FatArrow)?;
                        self.expect(&TokenKind::LBrace)?;
                        let body = self.parse_stmts()?;
                        self.expect(&TokenKind::RBrace)?;
                        arms.push((ty, body));
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::MatchUnion {
                        expr,
                        arms,
                        else_body,
                    })
                } else if is_tagged_union_match {
                    let mut arms: Vec<(String, Vec<String>, Vec<Stmt>)> = Vec::new();
                    let mut else_body = None;
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        if self.peek() == &TokenKind::Else {
                            self.advance();
                            self.expect(&TokenKind::FatArrow)?;
                            self.expect(&TokenKind::LBrace)?;
                            else_body = Some(self.parse_stmts()?);
                            self.expect(&TokenKind::RBrace)?;
                            break;
                        }
                        let variant_name = self.expect_ident()?;
                        let mut bindings = Vec::new();
                        if self.peek() == &TokenKind::LParen {
                            self.advance();
                            self.expect(&TokenKind::RParen)?;
                        } else {
                            while self.peek() != &TokenKind::FatArrow
                                && self.peek() != &TokenKind::RBrace
                                && self.peek() != &TokenKind::Eof
                            {
                                let b = self.expect_ident()?;
                                bindings.push(b);
                                if !self.eat(&TokenKind::Comma) {
                                    break;
                                }
                            }
                        }
                        self.expect(&TokenKind::FatArrow)?;
                        self.expect(&TokenKind::LBrace)?;
                        let body = self.parse_stmts()?;
                        self.expect(&TokenKind::RBrace)?;
                        arms.push((variant_name, bindings, body));
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Stmt::MatchTaggedUnion {
                        expr,
                        arms,
                        else_body,
                    })
                } else {
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
                        none_body: none_body
                            .ok_or_else(|| "match: missing 'none' arm".to_string())?,
                    })
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.eat(&TokenKind::Eq) {
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign {
                        target: expr,
                        value,
                    })
                } else if self.eat(&TokenKind::PlusPlus) {
                    if let Expr::Ident(name) = expr {
                        Ok(Stmt::Increment(name))
                    } else {
                        Err("'++' can only be applied to a variable".to_string())
                    }
                } else if self.eat(&TokenKind::MinusMinus) {
                    if let Expr::Ident(name) = expr {
                        Ok(Stmt::Decrement(name))
                    } else {
                        Err("'--' can only be applied to a variable".to_string())
                    }
                } else if self.eat(&TokenKind::PlusEq) {
                    if let Expr::Ident(name) = expr {
                        let value = self.parse_expr()?;
                        Ok(Stmt::AddAssign(name, value))
                    } else {
                        Err("'+=' can only be applied to a variable".to_string())
                    }
                } else if self.eat(&TokenKind::MinusEq) {
                    if let Expr::Ident(name) = expr {
                        let value = self.parse_expr()?;
                        Ok(Stmt::SubAssign(name, value))
                    } else {
                        Err("'-=' can only be applied to a variable".to_string())
                    }
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
        let mut lhs = self.parse_bitxor()?;
        while self.peek() == &TokenKind::BitOr {
            self.advance();
            let rhs = self.parse_bitxor()?;
            lhs = Expr::BinOp {
                op: BinOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == &TokenKind::BitXor {
            self.advance();
            let rhs = self.parse_bitand()?;
            lhs = Expr::BinOp {
                op: BinOp::BitXor,
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
            TokenKind::Ok => {
                self.advance();
                Ok(Expr::OkVal(Box::new(self.parse_unary()?)))
            }
            TokenKind::Err => {
                self.advance();
                Ok(Expr::ErrVal(Box::new(self.parse_unary()?)))
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
            } else if self.eat(&TokenKind::Question) {
                expr = Expr::Try(Box::new(expr));
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
                // addr(expr) — take address of a function or variable
                if name == "addr" {
                    self.expect(&TokenKind::LParen)?;
                    let inner = self.parse_expr()?;
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::Addr(Box::new(inner)));
                }
                // deref(expr) — dereference a pointer
                if name == "deref" {
                    self.expect(&TokenKind::LParen)?;
                    let inner = self.parse_expr()?;
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::Deref(Box::new(inner)));
                }
                // TypeName{} — zero-initialize all fields
                if self.peek() == &TokenKind::LBrace
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::RBrace)
                {
                    self.advance(); // consume {
                    self.advance(); // consume }
                    return Ok(Expr::ZeroInit(name));
                }
                // TypeName{field: val, ...} — named struct literal
                if self.peek() == &TokenKind::LBrace
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.kind),
                        Some(TokenKind::Ident(_))
                    )
                    && self.tokens.get(self.pos + 2).map(|t| &t.kind) == Some(&TokenKind::Colon)
                {
                    self.advance(); // consume {
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
                    return Ok(Expr::StructLit { fields });
                }
                Ok(Expr::Ident(name))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while self.peek() != &TokenKind::RBracket && self.peek() != &TokenKind::Eof {
                    elems.push(self.parse_expr()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::ListLit(elems))
            }
            TokenKind::At => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                if name == "sizeof" {
                    let ty = self.parse_type()?;
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::SizeofType(ty));
                }
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Builtin { name, args })
            }
            TokenKind::LParen => {
                self.advance();
                // `()` — unit/void literal
                if self.peek() == &TokenKind::RParen {
                    self.advance();
                    return Ok(Expr::IntLit(0));
                }
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

/// Returns true if `ty` is a primitive type that can appear in an untagged union.
pub fn is_primitive_type(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Named(n) => matches!(
            n.as_str(),
            "i8" | "i16" | "i32" | "i64"
                | "u8" | "u16" | "u32" | "u64"
                | "usize" | "isize"
                | "f32" | "f64"
                | "bool" | "char" | "void"
        ),
        TypeExpr::Ref(_) => true,
        _ => false,
    }
}
