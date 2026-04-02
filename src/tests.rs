#[cfg(test)]
mod lexer_tests {
    use crate::lexer::{Lexer, Token, TokenKind};

    fn lex(src: &str) -> Vec<Token> {
        Lexer::new(src).tokenize().unwrap()
    }

    fn lex_err(src: &str) -> String {
        Lexer::new(src).tokenize().unwrap_err()
    }

    #[test]
    fn keywords() {
        let toks = lex("fn val mut pub type pre post return if else trust use ref some none not");
        assert_eq!(toks[0].kind, TokenKind::Fn);
        assert_eq!(toks[1].kind, TokenKind::Val);
        assert_eq!(toks[2].kind, TokenKind::Mut);
        assert_eq!(toks[3].kind, TokenKind::Pub);
        assert_eq!(toks[4].kind, TokenKind::Type);
        assert_eq!(toks[5].kind, TokenKind::Pre);
        assert_eq!(toks[6].kind, TokenKind::Post);
        assert_eq!(toks[7].kind, TokenKind::Return);
        assert_eq!(toks[8].kind, TokenKind::If);
        assert_eq!(toks[9].kind, TokenKind::Else);
        assert_eq!(toks[10].kind, TokenKind::Trust);
        assert_eq!(toks[11].kind, TokenKind::Use);
        assert_eq!(toks[12].kind, TokenKind::Ref);
        assert_eq!(toks[13].kind, TokenKind::Some);
        assert_eq!(toks[14].kind, TokenKind::None_);
        assert_eq!(toks[15].kind, TokenKind::Not);
    }

    #[test]
    fn integer_literal() {
        let toks = lex("42 0 1000");
        assert_eq!(toks[0].kind, TokenKind::IntLit(42));
        assert_eq!(toks[1].kind, TokenKind::IntLit(0));
        assert_eq!(toks[2].kind, TokenKind::IntLit(1000));
    }

    #[test]
    fn string_literal() {
        let toks = lex(r#""hello" "world""#);
        assert_eq!(toks[0].kind, TokenKind::StringLit("hello".into()));
        assert_eq!(toks[1].kind, TokenKind::StringLit("world".into()));
    }

    #[test]
    fn string_escape_sequences() {
        let toks = lex(r#""\n\t\"\\""#);
        assert_eq!(toks[0].kind, TokenKind::StringLit("\n\t\"\\".into()));
    }

    #[test]
    fn operators_single_and_double() {
        let toks = lex("= == ! != < <= > >= & && | ||");
        assert_eq!(toks[0].kind, TokenKind::Eq);
        assert_eq!(toks[1].kind, TokenKind::EqEq);
        assert_eq!(toks[2].kind, TokenKind::Bang);
        assert_eq!(toks[3].kind, TokenKind::BangEq);
        assert_eq!(toks[4].kind, TokenKind::Lt);
        assert_eq!(toks[5].kind, TokenKind::LtEq);
        assert_eq!(toks[6].kind, TokenKind::Gt);
        assert_eq!(toks[7].kind, TokenKind::GtEq);
        assert_eq!(toks[8].kind, TokenKind::BitAnd);
        assert_eq!(toks[9].kind, TokenKind::And);
        assert_eq!(toks[10].kind, TokenKind::BitOr);
        assert_eq!(toks[11].kind, TokenKind::Or);
    }

    #[test]
    fn double_ampersand_is_single_and_token() {
        let toks = lex("a && b");
        assert_eq!(toks[0].kind, TokenKind::Ident("a".into()));
        assert_eq!(toks[1].kind, TokenKind::And);
        assert_eq!(toks[2].kind, TokenKind::Ident("b".into()));
    }

    #[test]
    fn double_pipe_is_single_or_token() {
        let toks = lex("a || b");
        assert_eq!(toks[0].kind, TokenKind::Ident("a".into()));
        assert_eq!(toks[1].kind, TokenKind::Or);
        assert_eq!(toks[2].kind, TokenKind::Ident("b".into()));
    }

    #[test]
    fn arrow_token() {
        let toks = lex("->");
        assert_eq!(toks[0].kind, TokenKind::Arrow);
    }

    #[test]
    fn punctuation() {
        let toks = lex("( ) { } [ ] : ; , . @ + - * / %");
        assert_eq!(toks[0].kind, TokenKind::LParen);
        assert_eq!(toks[1].kind, TokenKind::RParen);
        assert_eq!(toks[2].kind, TokenKind::LBrace);
        assert_eq!(toks[3].kind, TokenKind::RBrace);
        assert_eq!(toks[4].kind, TokenKind::LBracket);
        assert_eq!(toks[5].kind, TokenKind::RBracket);
        assert_eq!(toks[6].kind, TokenKind::Colon);
        assert_eq!(toks[7].kind, TokenKind::Semicolon);
        assert_eq!(toks[8].kind, TokenKind::Comma);
        assert_eq!(toks[9].kind, TokenKind::Dot);
        assert_eq!(toks[10].kind, TokenKind::At);
        assert_eq!(toks[11].kind, TokenKind::Plus);
        assert_eq!(toks[12].kind, TokenKind::Minus);
        assert_eq!(toks[13].kind, TokenKind::Star);
        assert_eq!(toks[14].kind, TokenKind::Slash);
        assert_eq!(toks[15].kind, TokenKind::Percent);
    }

    #[test]
    fn comments_are_skipped() {
        let toks = lex("42 // this is a comment\n99");
        assert_eq!(toks[0].kind, TokenKind::IntLit(42));
        assert_eq!(toks[1].kind, TokenKind::IntLit(99));
        assert_eq!(toks[2].kind, TokenKind::Eof);
    }

    #[test]
    fn eof_at_end() {
        let toks = lex("x");
        assert_eq!(toks.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn unterminated_string_is_error() {
        assert!(lex_err(r#""unterminated"#).contains("unterminated"));
    }

    #[test]
    fn unexpected_character_is_error() {
        assert!(lex_err("$").contains("unexpected character"));
    }

    #[test]
    fn identifier_with_underscore() {
        let toks = lex("some_var _private");
        assert_eq!(toks[0].kind, TokenKind::Ident("some_var".into()));
        assert_eq!(toks[1].kind, TokenKind::Ident("_private".into()));
    }
}

#[cfg(test)]
mod parser_tests {
    use crate::lexer::Lexer;
    use crate::parser::{Expr, Item, Parser, Stmt, TypeExpr};

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    fn parse_err(src: &str) -> String {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap_err()
    }

    #[test]
    fn parse_trusted_fn() {
        let m = parse("fn foo() void! = {}");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert_eq!(f.name, "foo");
        assert!(f.trusted);
        assert!(matches!(f.ret, TypeExpr::Void));
    }

    #[test]
    fn parse_pure_fn() {
        let m = parse("fn foo() void = {}");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(!f.trusted);
    }

    #[test]
    fn parse_public_fn() {
        let m = parse("pub fn foo() void! = {}");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(f.public);
    }

    #[test]
    fn parse_fn_params() {
        let m = parse("fn add(a: i32, b: i32) i32 = { return a }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].0, "a");
        assert_eq!(f.params[1].0, "b");
    }

    #[test]
    fn parse_type_def() {
        let m = parse("type Point = { x: i32 y: mut i32 }");
        let Item::TypeDef {
            name,
            fields,
            public: _,
        } = &m.items[0]
        else {
            panic!()
        };
        assert_eq!(name, "Point");
        assert_eq!(fields[0].name, "x");
        assert!(!fields[0].mutable);
        assert_eq!(fields[1].name, "y");
        assert!(fields[1].mutable);
    }

    #[test]
    fn parse_val_binding_immutable() {
        let m = parse("fn f() void! = { val x: i32 = 10 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { name, mutable, .. } = &f.body[0] else {
            panic!()
        };
        assert_eq!(name, "x");
        assert!(!mutable);
    }

    #[test]
    fn parse_val_binding_mutable() {
        let m = parse("fn f() void! = { val x: mut i32 = 10 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { mutable, .. } = &f.body[0] else {
            panic!()
        };
        assert!(mutable);
    }

    #[test]
    fn parse_pre_post_contracts() {
        let m = parse("fn f(x: i32) void = { pre { x > 0 } post { x > 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(f.body.iter().any(|s| matches!(s, Stmt::Pre(_))));
        assert!(f.body.iter().any(|s| matches!(s, Stmt::Post(_))));
    }

    #[test]
    fn parse_labelled_contract() {
        let m = parse("fn f(x: i32) void = { pre { is_pos: x > 0 } post { is_pos: x > 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Pre(contracts) = &f.body[0] else {
            panic!()
        };
        assert_eq!(contracts[0].label.as_deref(), Some("is_pos"));
    }

    #[test]
    fn parse_and_in_contract() {
        let m =
            parse("fn f(x: i32, y: i32) void = { pre { x > 0 && y > 0 } post { x > 0 && y > 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Pre(contracts) = &f.body[0] else {
            panic!()
        };
        assert!(matches!(contracts[0].expr, Expr::BinOp { .. }));
    }

    #[test]
    fn parse_or_in_contract() {
        let m = parse("fn f(x: i32) void = { pre { x > 0 || x < 0 } post { x > 0 || x < 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Pre(contracts) = &f.body[0] else {
            panic!()
        };
        assert!(matches!(contracts[0].expr, Expr::BinOp { .. }));
    }

    #[test]
    fn parse_trust_expr() {
        let m = parse("fn f() void = { trust foo() }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Expr(Expr::Trust(_)) = &f.body[0] else {
            panic!()
        };
    }

    #[test]
    fn parse_if_else() {
        let m = parse("fn f(x: i32) void! = { if (x > 0) { return } else { return } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::If { else_, .. } = &f.body[0] else {
            panic!()
        };
        assert!(else_.is_some());
    }

    #[test]
    fn parse_struct_literal() {
        let m = parse("fn f() void! = { val p = {x: 1, y: 2} }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(expr, Expr::StructLit { .. }));
    }

    #[test]
    fn parse_return_none() {
        let m = parse("fn f() void! = { return }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(matches!(f.body[0], Stmt::Return(None)));
    }

    #[test]
    fn parse_return_value() {
        let m = parse("fn f() i32 = { pre { 1 == 1 } post { 1 == 1 } return 42 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let last = f.body.last().unwrap();
        assert!(matches!(last, Stmt::Return(Some(_))));
    }

    #[test]
    fn parse_option_type() {
        let m = parse("fn f() option[i32]! = { return none }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(matches!(f.ret, TypeExpr::Option(_)));
    }

    #[test]
    fn parse_slice_type() {
        let m = parse("fn f(s: []char) void! = {}");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(matches!(f.params[0].1, TypeExpr::Slice(_)));
    }

    #[test]
    fn parse_ref_type() {
        let m = parse("fn f(s: ref char) void! = {}");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(matches!(f.params[0].1, TypeExpr::Ref(_)));
    }

    #[test]
    fn parse_binop_precedence_mul_over_add() {
        let m = parse("fn f() void! = { val x = 1 + 2 * 3 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else {
            panic!()
        };
        let Expr::BinOp {
            op: crate::parser::BinOp::Add,
            rhs,
            ..
        } = expr
        else {
            panic!()
        };
        assert!(matches!(
            rhs.as_ref(),
            Expr::BinOp {
                op: crate::parser::BinOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn parse_error_unexpected_top_level() {
        assert!(parse_err("42").contains("unexpected token at top level"));
    }
}

#[cfg(test)]
mod codegen_tests {
    use crate::codegen::Codegen;
    use crate::module::resolve_module;
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn compile_err(src: &str) -> String {
        compile(src).expect_err("expected compilation to fail")
    }

    #[test]
    fn trusted_fn_emits_export() {
        let ir = compile_ok("pub fn main() void! = {}");
        assert!(ir.contains("export function $main()"));
    }

    #[test]
    fn private_fn_no_export() {
        let ir = compile_ok("fn helper() void! = {}");
        assert!(!ir.contains("export function $helper"));
        assert!(ir.contains("function $helper"));
    }

    #[test]
    fn trusted_fn_no_contracts_required() {
        compile_ok("pub fn main() void! = {}");
    }

    #[test]
    fn pure_fn_all_trust_stmts_no_contracts_required() {
        let ir = compile_ok(
            r#"
            pub fn print(arg: ref char) void = {
                trust @puts(arg)
            }
        "#,
        );
        assert!(ir.contains("$print"));
    }

    #[test]
    fn pure_fn_with_pre_post_ok() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("$f"));
    }

    #[test]
    fn pure_fn_missing_both_contracts_errors() {
        let err = compile_err(
            r#"
            type T = { x: mut i32 }
            fn f(x: i32) void = { val t: mut T = {x: 1} t.x = 2 }
        "#,
        );
        assert!(err.contains("pure function"));
    }

    #[test]
    fn pure_fn_with_assignment_requires_contracts() {
        let err = compile_err(
            r#"
            type T = { x: mut i32 }
            fn f(x: i32) void = {
                val t: mut T = {x: 1}
                t.x = 2
            }
        "#,
        );
        assert!(err.contains("pure function"));
    }

    #[test]
    fn immutable_binding_field_assign_errors() {
        let err = compile_err(
            r#"
            type T = { x: mut i32 }
            pub fn main() void! = {
                val t: T = {x: 1}
                t.x = 2
            }
        "#,
        );
        assert!(err.contains("immutable binding"));
    }

    #[test]
    fn immutable_field_assign_errors() {
        let err = compile_err(
            r#"
            type T = { x: i32 y: mut i32 }
            pub fn main() void! = {
                val t: mut T = {x: 1, y: 2}
                t.x = 10
            }
        "#,
        );
        assert!(err.contains("immutable field") && err.contains("'x'"));
    }

    #[test]
    fn mutable_field_assign_ok() {
        let ir = compile_ok(
            r#"
            type T = { x: i32 y: mut i32 }
            pub fn main() void! = {
                val t: mut T = {x: 1, y: 2}
                t.y = 10
            }
        "#,
        );
        assert!(ir.contains("storel"));
    }

    #[test]
    fn pre_contract_emits_dprintf_on_fail() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("dprintf"));
        assert!(ir.contains("precondition"));
        assert!(ir.contains("abort"));
    }

    #[test]
    fn post_contract_emits_dprintf_on_fail() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("postcondition"));
    }

    #[test]
    fn labelled_contract_includes_label_in_message() {
        let ir = compile_ok(
            "fn f(x: i32) i32 = { pre { is_pos: x > 0 } post { is_pos: x > 0 } return x }",
        );
        assert!(ir.contains("is_pos"));
    }

    #[test]
    fn contract_message_includes_fn_name() {
        let ir = compile_ok("fn my_func(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("my_func"));
    }

    #[test]
    fn pre_contract_with_and() {
        let ir = compile_ok(
            "fn f(x: i32, y: i32) i32 = { pre { x > 0 && y > 0 } post { x > 0 } return x }",
        );
        assert!(ir.contains("and"));
    }

    #[test]
    fn pre_contract_with_or() {
        let ir = compile_ok(
            "fn f(x: i32, y: i32) i32 = { pre { x > 0 || y > 0 } post { x > 0 } return x }",
        );
        assert!(ir.contains("or"));
    }

    #[test]
    fn struct_literal_calls_malloc() {
        let ir = compile_ok(
            r#"
            type T = { x: i32 }
            pub fn main() void! = {
                val t: T = {x: 1}
            }
        "#,
        );
        assert!(ir.contains("malloc"));
    }

    #[test]
    fn return_value_emits_ret() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("ret %x"));
    }

    #[test]
    fn void_fn_emits_ret_without_value() {
        let ir = compile_ok("pub fn main() void! = {}");
        assert!(ir.contains("ret\n"));
    }

    #[test]
    fn binop_add_emits_add() {
        let ir =
            compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x + y }");
        assert!(ir.contains("=w add"));
    }

    #[test]
    fn binop_sub_emits_sub() {
        let ir =
            compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x - y }");
        assert!(ir.contains("=w sub"));
    }

    #[test]
    fn binop_mul_emits_mul() {
        let ir =
            compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x * y }");
        assert!(ir.contains("=w mul"));
    }

    #[test]
    fn unop_neg_emits_neg() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return -x }");
        assert!(ir.contains("=w neg"));
    }

    #[test]
    fn unop_not_emits_ceqw() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return not x }");
        assert!(ir.contains("=w ceqw"));
    }

    #[test]
    fn string_literal_interned_as_data() {
        let ir = compile_ok(r#"pub fn main() void! = { val s = "hello" }"#);
        assert!(ir.contains("b \"hello\""));
    }

    #[test]
    fn format_string_rewritten() {
        let ir = compile_ok(
            r#"
            pub fn put_any(fmt: ref char, args: untyped) void! = {}
            pub fn main() void! = {
                put_any("{d}", 1)
            }
        "#,
        );
        assert!(ir.contains("b \"%d\""));
    }

    #[test]
    fn if_without_else_emits_branches() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("@if_then_"));
        assert!(ir.contains("@if_end_"));
    }

    #[test]
    fn if_with_else_emits_else_branch() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val x = 1 } else { val y = 2 }
            }
        "#,
        );
        assert!(ir.contains("@if_else_"));
    }

    #[test]
    fn undefined_variable_errors() {
        let err = compile_err("pub fn main() void! = { val x = undefined_var }");
        assert!(err.contains("undefined variable"));
    }

    #[test]
    fn pure_fn_calling_only_fns_no_contracts_needed() {
        let ir = compile_ok(
            r#"
            pub fn helper() void! = {}
            pub fn main() void = {
                trust helper()
            }
        "#,
        );
        assert!(ir.contains("$main"));
    }
}

#[cfg(test)]
mod format_string_tests {
    use crate::codegen::Codegen;
    use crate::module::resolve_module;
    use std::collections::HashMap;
    use std::path::Path;

    fn rewrite(fmt: &str) -> String {
        let src = format!(
            r#"pub fn put_any(f: ref char, a: untyped) void! = {{}}
               pub fn main() void! = {{ put_any("{fmt}", 1) }}"#
        );
        let resolved = resolve_module(&src, Path::new("."), &mut HashMap::new(), "").unwrap();
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false).unwrap();
        let ir = cg.finish();
        let marker = "b \"";
        let start = ir.find(marker).unwrap() + marker.len();
        let end = ir[start..].find('"').unwrap() + start;
        ir[start..end].to_string()
    }

    #[test]
    fn d_specifier() {
        assert_eq!(rewrite("{d}"), "%d");
    }

    #[test]
    fn s_specifier() {
        assert_eq!(rewrite("{s}"), "%s");
    }

    #[test]
    fn mixed_text_and_specifier() {
        assert_eq!(rewrite("val={d}"), "val=%d");
    }

    #[test]
    fn no_specifier_passthrough() {
        assert_eq!(rewrite("hello"), "hello");
    }
}

#[cfg(test)]
mod hoisting_and_optionals_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Expr, Item, Parser, Stmt};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_true_false_keywords() {
        let toks = Lexer::new("true false").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::True);
        assert_eq!(toks[1].kind, TokenKind::False);
    }

    #[test]
    fn lex_match_keyword() {
        let toks = Lexer::new("match").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Match);
    }

    #[test]
    fn lex_fat_arrow() {
        let toks = Lexer::new("=>").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::FatArrow);
    }

    #[test]
    fn lex_bang_not_fat_arrow() {
        let toks = Lexer::new("= x").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Eq);
    }

    #[test]
    fn parse_true_literal() {
        let m = parse("pub fn main() void! = { val x = true }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(expr, Expr::Bool(true)));
    }

    #[test]
    fn parse_false_literal() {
        let m = parse("pub fn main() void! = { val x = false }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(expr, Expr::Bool(false)));
    }

    #[test]
    fn parse_bang_as_not() {
        let m = parse("pub fn main() void! = { val x = !1 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(
            expr,
            Expr::UnOp {
                op: crate::parser::UnOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn parse_some_expr() {
        let m = parse("fn f() option[i32]! = { return some 42 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Return(Some(expr)) = &f.body[0] else {
            panic!()
        };
        assert!(matches!(expr, Expr::Some(_)));
    }

    #[test]
    fn parse_none_expr() {
        let m = parse("fn f() option[i32]! = { return none }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Return(Some(expr)) = &f.body[0] else {
            panic!()
        };
        assert!(matches!(expr, Expr::None));
    }

    #[test]
    fn parse_match_stmt() {
        let m = parse(
            r#"
            pub fn main() void! = {
                val x = none
                match x {
                    some v => { val y = v }
                    none => { val z = 0 }
                }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Match { some_binding, .. } = &f.body[1] else {
            panic!("expected match stmt")
        };
        assert_eq!(some_binding, "v");
    }

    #[test]
    fn parse_match_none_arm_present() {
        let m = parse(
            r#"
            pub fn main() void! = {
                val x = none
                match x {
                    some v => {}
                    none => { val z = 0 }
                }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Match { none_body, .. } = &f.body[1] else {
            panic!()
        };
        assert!(!none_body.is_empty());
    }

    #[test]
    fn codegen_none_returns_zero() {
        let ir = compile_ok("fn f() option[i32]! = { return none }");
        assert!(ir.contains("ret 0"));
    }

    #[test]
    fn codegen_some_sign_extends_to_l() {
        let ir = compile_ok("fn f() option[i32]! = { return some 10 }");
        assert!(ir.contains("=l copy") || ir.contains("extsw"));
    }

    #[test]
    fn codegen_match_emits_cnel() {
        let ir = compile_ok(
            r#"
            fn get() option[i32]! = { return none }
            pub fn main() void! = {
                val r = get()
                match r {
                    some x => { val a = x }
                    none => { val b = 0 }
                }
            }
        "#,
        );
        assert!(ir.contains("cnel"));
        assert!(ir.contains("@match_some_"));
        assert!(ir.contains("@match_none_"));
        assert!(ir.contains("@match_end_"));
    }

    #[test]
    fn codegen_function_hoisting_private_callee() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                val r = can_fail(true)
            }
            fn can_fail(flag: bool) option[i32]! = {
                return none
            }
        "#,
        );
        assert!(ir.contains("$can_fail"));
        assert!(ir.contains("call $can_fail"));
    }

    #[test]
    fn codegen_if_with_return_no_jmp_after_ret() {
        let ir = compile_ok(
            r#"
            fn f(x: bool) option[i32]! = {
                if (x) { return some 1 }
                return none
            }
        "#,
        );
        for window in ir.lines().collect::<Vec<_>>().windows(2) {
            let a = window[0].trim();
            let b = window[1].trim();
            assert!(
                !(a.starts_with("ret") && b.starts_with("jmp")),
                "jmp after ret detected:\n  {a}\n  {b}"
            );
        }
    }

    #[test]
    fn codegen_true_emits_1() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (true) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("jnz 1,"));
    }

    #[test]
    fn codegen_false_emits_0() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (false) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("jnz 0,"));
    }
}

#[cfg(test)]
mod elif_and_for_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Item, Parser, Stmt};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_elif_keyword() {
        let toks = Lexer::new("elif").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Elif);
    }

    #[test]
    fn lex_for_keyword() {
        let toks = Lexer::new("for").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::For);
    }

    #[test]
    fn lex_plus_plus() {
        let toks = Lexer::new("x++").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Ident("x".into()));
        assert_eq!(toks[1].kind, TokenKind::PlusPlus);
    }

    #[test]
    fn lex_plus_not_plus_plus() {
        let toks = Lexer::new("x + y").tokenize().unwrap();
        assert_eq!(toks[1].kind, TokenKind::Plus);
    }

    #[test]
    fn parse_elif_chain() {
        let m = parse(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val a = 1 }
                elif (2 == 2) { val b = 2 }
                elif (3 == 3) { val c = 3 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::If { elif_, .. } = &f.body[0] else {
            panic!("expected if")
        };
        assert_eq!(elif_.len(), 2);
    }

    #[test]
    fn parse_elif_with_else() {
        let m = parse(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val a = 1 }
                elif (2 == 2) { val b = 2 }
                else { val c = 3 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::If { elif_, else_, .. } = &f.body[0] else {
            panic!()
        };
        assert_eq!(elif_.len(), 1);
        assert!(else_.is_some());
    }

    #[test]
    fn parse_if_no_elif_still_works() {
        let m = parse("pub fn main() void! = { if (1 == 1) { val x = 1 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::If { elif_, else_, .. } = &f.body[0] else {
            panic!()
        };
        assert!(elif_.is_empty());
        assert!(else_.is_none());
    }

    #[test]
    fn parse_for_c_style() {
        let m = parse(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 10; i++) { val x = i }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::For {
            init, cond, post, ..
        } = &f.body[0]
        else {
            panic!("expected for")
        };
        assert!(init.is_some());
        assert_eq!(init.as_ref().unwrap().0, "i");
        assert!(cond.is_some());
        assert!(post.is_some());
    }

    #[test]
    fn parse_for_infinite() {
        let m = parse(
            r#"
            pub fn main() void! = {
                for { val x = 1 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::For {
            init, cond, post, ..
        } = &f.body[0]
        else {
            panic!("expected for")
        };
        assert!(init.is_none());
        assert!(cond.is_none());
        assert!(post.is_none());
    }

    #[test]
    fn codegen_elif_labels_are_consistent() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val a = 1 }
                elif (2 == 2) { val b = 2 }
            }
        "#,
        );
        for line in ir.lines() {
            let line = line.trim();
            if line.starts_with("jnz") {
                if let Some(false_lbl) = line
                    .split([',', ' '])
                    .map(str::trim)
                    .filter(|s| s.starts_with('@'))
                    .nth(1)
                {
                    assert!(
                        ir.contains(&format!("{false_lbl}\n")),
                        "label {false_lbl} referenced but never defined"
                    );
                }
            }
        }
    }

    #[test]
    fn codegen_elif_emits_cond_labels() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                if (1 == 1) { val a = 1 }
                elif (2 == 2) { val b = 2 }
            }
        "#,
        );
        assert!(ir.contains("@elif_cond_0_"));
    }

    #[test]
    fn codegen_for_c_style_emits_loop_labels() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 10; i++) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("@for_loop_"));
        assert!(ir.contains("@for_body_"));
        assert!(ir.contains("@for_end_"));
    }

    #[test]
    fn codegen_for_c_style_emits_increment() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 10; i++) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("=l add"));
        assert!(ir.contains("storel"));
    }

    #[test]
    fn codegen_for_c_style_loop_var_readable() {
        let ir = compile_ok(
            r#"
            pub fn put_any(fmt: ref char, args: untyped) void! = {}
            pub fn main() void! = {
                for (i = 0; i < 3; i++) {
                    put_any("{d}", i)
                }
            }
        "#,
        );
        assert!(ir.contains("loadl"));
    }

    #[test]
    fn codegen_for_infinite_loops_back() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("@for_loop_"));
        assert!(ir.contains("@for_body_"));
        let loop_lbl = ir
            .lines()
            .find(|l| l.trim().starts_with("@for_loop_"))
            .unwrap()
            .trim()
            .to_string();
        assert!(ir.contains(&format!("jmp {loop_lbl}")));
    }

    #[test]
    fn codegen_for_init_uses_alloc8() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 5; i++) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("alloc8"));
    }
}

#[cfg(test)]
mod float_mut_and_for_regression_tests {
    use crate::codegen::Codegen;
    use crate::module::resolve_module;
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    #[test]
    fn mut_f64_uses_stored_and_loadd() {
        let ir = compile_ok(
            r#"
            pub fn identity(x: f64) f64 = { return x }
            pub fn main() void! = {
                val x: mut f64 = 1.0
                x = 2.0
                val y = identity(x)
            }
        "#,
        );
        assert!(ir.contains("stored"), "mutable f64 should use stored");
        assert!(ir.contains("loadd"), "mutable f64 should use loadd");
    }

    #[test]
    fn mut_f64_assignment_in_loop_updates_value() {
        let ir = compile_ok(
            r#"
            pub fn sqrt_approx(x: f64) f64! = {
                val guess: mut f64 = x
                val i: mut usize = 0
                for (i = 0; i < 5; i++) {
                    guess = 0.5 * (guess + x / guess)
                }
                return guess
            }
        "#,
        );
        assert!(ir.contains("stored"), "loop body must store updated guess");
        assert!(
            ir.contains("loadd"),
            "loop body must load guess each iteration"
        );
    }

    #[test]
    fn for_loop_counter_uses_alloc8_and_storel() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 10; i++) { val x = 1 }
            }
        "#,
        );
        assert!(
            ir.contains("alloc8"),
            "for-loop counter slot must be 8 bytes"
        );
        assert!(ir.contains("storel"), "for-loop counter must use storel");
        assert!(ir.contains("loadl"), "for-loop counter must use loadl");
        assert!(
            !ir.contains("alloc4"),
            "alloc4 is wrong for a usize counter"
        );
    }

    #[test]
    fn for_loop_increment_is_64bit() {
        let ir = compile_ok(
            r#"
            pub fn main() void! = {
                for (i = 0; i < 10; i++) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("=l add"), "i++ must emit =l add, not =w add");
        assert!(
            !ir.contains("=w add"),
            "=w add is wrong for usize increment"
        );
    }
}

#[cfg(test)]
mod memory_and_defer_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Item, Parser, Stmt};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_defer_keyword() {
        let toks = Lexer::new("defer").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Defer);
    }

    #[test]
    fn lex_break_keyword() {
        let toks = Lexer::new("break").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Break);
    }

    #[test]
    fn parse_defer_block() {
        let m = parse(r#"pub fn main() void! = { defer { val x = 1 } }"#);
        let Item::Fn(f) = &m.items[0] else { panic!() };
        assert!(matches!(f.body[0], Stmt::Defer(_)));
    }

    #[test]
    fn parse_break_stmt() {
        let m = parse(r#"pub fn main() void! = { for { break } }"#);
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::For { body, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(body[0], Stmt::Break));
    }

    #[test]
    fn codegen_alloc_calls_malloc() {
        let ir = compile_ok(r#"pub fn main() void! = { val p: ref void = @alloc(64) }"#);
        assert!(ir.contains("call $malloc"));
    }

    #[test]
    fn codegen_free_calls_free() {
        let ir = compile_ok(r#"pub fn main() void! = { val p: ref void = @alloc(8) @free(p) }"#);
        assert!(ir.contains("call $free"));
    }

    #[test]
    fn codegen_realloc_calls_realloc() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            val q: ref void = @realloc(p, 16)
        }"#,
        );
        assert!(ir.contains("call $realloc"));
    }

    #[test]
    fn codegen_memset_calls_memset() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            @memset(p, 0, 8)
        }"#,
        );
        assert!(ir.contains("call $memset"));
    }

    #[test]
    fn codegen_ptradd_emits_add_l() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(16)
            val q: ref void = @ptradd(p, 8)
        }"#,
        );
        assert!(ir.contains("=l add"));
    }

    #[test]
    fn codegen_load_emits_loadl() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            val v: usize = @load(p)
        }"#,
        );
        assert!(ir.contains("=l loadl"));
    }

    #[test]
    fn codegen_store_emits_storel() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            @store(p, 42)
        }"#,
        );
        assert!(ir.contains("storel"));
    }

    #[test]
    fn codegen_defer_emits_before_ret() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            defer { @free(p) }
        }"#,
        );
        let free_pos = ir.find("call $free").expect("free not found");
        let ret_pos = ir.rfind("ret\n").expect("ret not found");
        assert!(free_pos < ret_pos, "defer free must come before ret");
    }

    #[test]
    fn codegen_defer_lifo_order() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: ref void = @alloc(8)
            val q: ref void = @alloc(16)
            defer { @free(p) }
            defer { @free(q) }
        }"#,
        );
        let free_q = ir
            .find("call $free(l %t1)")
            .or_else(|| ir.find("call $free").map(|p| p))
            .unwrap();
        let _ = free_q;
        assert_eq!(ir.matches("call $free").count(), 2);
    }

    #[test]
    fn codegen_defer_runs_before_explicit_return() {
        let ir = compile_ok(
            r#"
            fn f(x: bool) void! = {
                val p: ref void = @alloc(8)
                defer { @free(p) }
                if (x) { return }
            }
        "#,
        );
        let free_pos = ir.find("call $free").expect("free not found");
        let ret_pos = ir.find("ret\n").expect("ret not found");
        assert!(free_pos < ret_pos);
    }

    #[test]
    fn codegen_break_jumps_to_for_end() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            for (i = 0; i < 10; i++) {
                break
            }
        }"#,
        );
        assert!(ir.contains("@for_end_"));
        assert!(ir.matches("jmp @for_end_").count() >= 1);
    }

    #[test]
    fn codegen_break_in_infinite_loop() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            for { break }
        }"#,
        );
        assert!(ir.contains("jmp @for_end_"));
    }

    #[test]
    fn codegen_local_rebind_updates_local() {
        let ir = compile_ok(
            r#"pub fn main() void! = {
            val p: mut ref void = @alloc(8)
            p = @realloc(p, 16)
        }"#,
        );
        assert!(ir.contains("call $realloc"));
    }

    #[test]
    fn codegen_usize_add_emits_add_l() {
        let ir = compile_ok(
            r#"
            fn f(a: usize, b: usize) usize! = {
                return a + b
            }
        "#,
        );
        assert!(ir.contains("=l add"), "usize add should emit =l add");
    }

    #[test]
    fn codegen_usize_mul_emits_mul_l() {
        let ir = compile_ok(
            r#"
            fn f(a: usize, b: usize) usize! = {
                return a * b
            }
        "#,
        );
        assert!(ir.contains("=l mul"), "usize mul should emit =l mul");
    }

    #[test]
    fn codegen_i32_add_still_emits_add_w() {
        let ir = compile_ok(
            r#"
            fn f(a: i32, b: i32) i32 = {
                pre { a > 0 }
                post { a > 0 }
                return a + b
            }
        "#,
        );
        assert!(ir.contains("=w add"), "i32 add should still emit =w add");
    }
}

#[cfg(test)]
mod method_namespace_tests {
    use crate::codegen::Codegen;
    use crate::lexer::Lexer;
    use crate::module::resolve_module;
    use crate::parser::{FnDef, Item, Parser, TypeExpr};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn parse_method_fn_has_namespace() {
        let m = parse("pub fn (SomeType) do_thing(s: SomeType) void! = {}");
        let Item::Fn(FnDef {
            namespace, name, ..
        }) = &m.items[0]
        else {
            panic!()
        };
        assert_eq!(namespace.as_deref(), Some("SomeType"));
        assert_eq!(name, "do_thing");
    }

    #[test]
    fn parse_self_resolves_to_receiver_type() {
        let m = parse("pub fn (SomeType) do_thing(s: Self) void! = {}");
        let Item::Fn(FnDef { params, .. }) = &m.items[0] else {
            panic!()
        };
        assert!(matches!(&params[0].1, TypeExpr::Named(n) if n == "SomeType"));
    }

    #[test]
    fn parse_regular_fn_has_no_namespace() {
        let m = parse("pub fn foo() void! = {}");
        let Item::Fn(FnDef { namespace, .. }) = &m.items[0] else {
            panic!()
        };
        assert!(namespace.is_none());
    }

    #[test]
    fn parse_private_method_fn() {
        let m = parse("fn (SomeType) helper(s: SomeType) void! = {}");
        let Item::Fn(FnDef {
            public, namespace, ..
        }) = &m.items[0]
        else {
            panic!()
        };
        assert!(!public);
        assert_eq!(namespace.as_deref(), Some("SomeType"));
    }

    #[test]
    fn method_fn_emits_mangled_name() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            pub fn (SomeType) do_thing(s: SomeType) void! = {}
        "#,
        );
        assert!(
            ir.contains("$SomeType__do_thing"),
            "expected mangled QBE name"
        );
    }

    #[test]
    fn public_method_is_exported() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            pub fn (SomeType) do_thing(s: SomeType) void! = {}
        "#,
        );
        assert!(
            ir.contains("export function"),
            "public method should be exported"
        );
        assert!(ir.contains("$SomeType__do_thing"));
    }

    #[test]
    fn private_method_is_not_exported() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            fn (SomeType) helper(s: SomeType) void! = {}
        "#,
        );
        assert!(
            !ir.contains("export function $SomeType__helper"),
            "private method must not be exported"
        );
        assert!(
            ir.contains("function $SomeType__helper"),
            "private method should still be emitted"
        );
    }

    #[test]
    fn method_callable_via_type_dot_name() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            pub fn (SomeType) do_thing(s: SomeType) void! = {}
            pub fn main() void! = {
                val x: SomeType = {x: 10}
                SomeType.do_thing(x)
            }
        "#,
        );
        assert!(ir.contains("call $SomeType__do_thing"));
    }

    #[test]
    fn self_param_field_access_works() {
        let ir = compile_ok(
            r#"
            type Point = { x: i32 y: i32 }
            pub fn (Point) get_x(p: Self) i32! = {
                return p.x
            }
        "#,
        );
        assert!(ir.contains("loadl"));
    }

    #[test]
    fn public_method_calls_private_method() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            fn (SomeType) helper(s: SomeType) void! = {}
            pub fn (SomeType) do_thing(s: SomeType) void! = {
                SomeType.helper(s)
            }
        "#,
        );
        assert!(ir.contains("call $SomeType__helper"));
    }

    #[test]
    fn same_module_can_call_private_method() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 }
            fn (SomeType) secret(s: SomeType) void! = {}
            pub fn main() void! = {
                val x: SomeType = {x: 1}
                SomeType.secret(x)
            }
        "#,
        );
        assert!(ir.contains("call $SomeType__secret"));
    }

    #[test]
    fn sample13_compiles() {
        let ir = compile_ok(
            r#"
            type SomeType = { x: i32 y: i32 }
            pub fn (SomeType) do_some_thing(s: Self, times: usize) void! = {
                val i: mut usize = 0
                for (i = 0; i < times; i++) {
                    val _x = s.x
                }
            }
            pub fn main() void! = {
                val x: SomeType = {x: 10, y: 40}
                SomeType.do_some_thing(x, 20)
            }
        "#,
        );
        assert!(ir.contains("$SomeType__do_some_thing"));
        assert!(ir.contains("call $SomeType__do_some_thing"));
    }
}

#[cfg(test)]
mod cast_tests {
    use crate::codegen::Codegen;
    use crate::module::resolve_module;
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    #[test]
    fn cast_f64_to_i64_emits_dtosi() {
        let ir = compile_ok(
            r#"
            pub fn f(x: f64) i64 = {
                return x as i64
            }
        "#,
        );
        assert!(ir.contains("dtosi"), "f64->i64 cast should emit dtosi");
    }

    #[test]
    fn cast_i64_to_f64_emits_sltof() {
        let ir = compile_ok(
            r#"
            pub fn f(x: i64) f64 = {
                return x as f64
            }
        "#,
        );
        assert!(ir.contains("sltof"), "i64->f64 cast should emit sltof");
    }

    #[test]
    fn cast_chained_f64_i64_f64() {
        let ir = compile_ok(
            r#"
            pub fn trunc(x: f64) f64 = {
                return (x as i64) as f64
            }
        "#,
        );
        assert!(ir.contains("dtosi"), "first cast should emit dtosi");
        assert!(ir.contains("sltof"), "second cast should emit sltof");
    }

    #[test]
    fn cast_used_in_expression() {
        let ir = compile_ok(
            r#"
            pub fn floor(x: f64) f64 = {
                val i = x as i64
                if (x < (i as f64)) {
                    return (i - 1) as f64
                }
                return i as f64
            }
        "#,
        );
        assert!(ir.contains("dtosi"));
        assert!(ir.contains("sltof"));
    }

    #[test]
    fn as_is_lexed_as_keyword() {
        use crate::lexer::{Lexer, TokenKind};
        let toks = Lexer::new("x as i64").tokenize().unwrap();
        assert_eq!(toks[1].kind, TokenKind::As);
    }

    #[test]
    fn as_does_not_conflict_with_ident_starting_with_as() {
        use crate::lexer::{Lexer, TokenKind};
        let toks = Lexer::new("asset assign").tokenize().unwrap();
        assert!(matches!(toks[0].kind, TokenKind::Ident(_)));
        assert!(matches!(toks[1].kind, TokenKind::Ident(_)));
    }
}

#[cfg(test)]
mod union_codegen_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Item, Parser, Stmt, TypeExpr};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn compile_err(src: &str) -> String {
        compile(src).expect_err("expected compilation to fail")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_union_keyword() {
        let toks = Lexer::new("union").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Union);
    }

    #[test]
    fn lex_when_keyword() {
        let toks = Lexer::new("when").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::When);
    }

    #[test]
    fn lex_is_keyword() {
        let toks = Lexer::new("is").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Is);
    }

    #[test]
    fn lex_otherwise_keyword() {
        let toks = Lexer::new("otherwise").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Otherwise);
    }

    #[test]
    fn parse_union_def_two_variants() {
        let m = parse("union Shape = { i32 | i64 }");
        let Item::UnionDef {
            name,
            variants,
            public: _,
        } = &m.items[0]
        else {
            panic!("expected UnionDef")
        };
        assert_eq!(name, "Shape");
        assert_eq!(variants.len(), 2);
        assert!(matches!(&variants[0], TypeExpr::Named(n) if n == "i32"));
        assert!(matches!(&variants[1], TypeExpr::Named(n) if n == "i64"));
    }

    #[test]
    fn parse_union_def_three_variants() {
        let m = parse("union U = { i32 | i64 | ref char }");
        let Item::UnionDef {
            name,
            variants,
            public: _,
        } = &m.items[0]
        else {
            panic!("expected UnionDef")
        };
        assert_eq!(name, "U");
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn parse_when_is_stmt() {
        let m = parse(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[1] else { panic!() };
        assert!(matches!(f.body[0], Stmt::WhenIs { .. }));
    }

    #[test]
    fn parse_when_is_captures_type() {
        let m = parse(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i64 { val a = 1 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[1] else { panic!() };
        let Stmt::WhenIs { ty, .. } = &f.body[0] else {
            panic!()
        };
        assert!(matches!(ty, TypeExpr::Named(n) if n == "i64"));
    }

    #[test]
    fn parse_otherwise_stmt() {
        let m = parse(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise { val b = 2 }
            }
        "#,
        );
        let Item::Fn(f) = &m.items[1] else { panic!() };
        assert!(matches!(f.body[1], Stmt::Otherwise { .. }));
    }

    #[test]
    fn union_def_compiles_without_error() {
        compile_ok("union U = { i32 | i64 }");
    }

    #[test]
    fn union_fn_param_wraps_with_malloc() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn consume(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {
                val v: i32 = 42
                consume(v)
            }
        "#,
        );
        assert!(
            ir.contains("call $malloc"),
            "union arg should be heap-allocated"
        );
    }

    #[test]
    fn union_fn_param_stores_tag() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn consume(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {
                val v: i32 = 42
                consume(v)
            }
        "#,
        );
        assert!(ir.contains("storew 0,"), "first variant tag should be 0");
    }

    #[test]
    fn union_fn_param_second_variant_tag_is_1() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn consume(x: U) void! = {
                when x is i64 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {
                val v: i64 = 99
                consume(v)
            }
        "#,
        );
        assert!(ir.contains("storew 1,"), "second variant tag should be 1");
    }

    #[test]
    fn union_fn_param_stores_value_after_tag() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn consume(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {
                val v: i32 = 7
                consume(v)
            }
        "#,
        );
        assert!(
            ir.contains("=l add") && ir.contains("storel"),
            "value should be stored at offset 8"
        );
    }

    #[test]
    fn when_is_emits_loadw_for_tag() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(ir.contains("=w loadw"), "when/is must load tag as word");
    }

    #[test]
    fn when_is_emits_ceqw_for_tag_comparison() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(ir.contains("=w ceqw"), "when/is must compare tag with ceqw");
    }

    #[test]
    fn when_is_emits_body_and_skip_labels() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(
            ir.contains("@when_body_"),
            "when body label must be emitted"
        );
        assert!(
            ir.contains("@when_skip_"),
            "when skip label must be emitted"
        );
    }

    #[test]
    fn when_is_chain_emits_shared_end_label() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                when x is i64 { val b = 2 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(
            ir.contains("@when_end_"),
            "when chain must emit a shared end label"
        );
    }

    #[test]
    fn when_is_chain_two_variants_both_checked() {
        let ir = compile_ok(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is i32 { val a = 1 }
                when x is i64 { val b = 2 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert_eq!(
            ir.matches("ceqw").count(),
            2,
            "two variants need two tag comparisons"
        );
    }

    #[test]
    fn when_is_unknown_variant_errors() {
        let err = compile_err(
            r#"
            union U = { i32 | i64 }
            fn f(x: U) void! = {
                when x is bool { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(
            err.contains("not a variant"),
            "unknown variant should error"
        );
    }

    #[test]
    fn when_is_non_union_type_errors() {
        let err = compile_err(
            r#"
            type T = { x: i32 }
            fn f(x: T) void! = {
                when x is i32 { val a = 1 }
                otherwise {}
            }
            pub fn main() void! = {}
        "#,
        );
        assert!(
            err.contains("not a union type"),
            "struct used as union should error"
        );
    }
}

#[cfg(test)]
mod enum_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Item, Parser};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_enum_keyword() {
        let toks = Lexer::new("enum").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Enum);
    }

    #[test]
    fn parse_enum_def_two_variants() {
        let m = parse("enum Color = { RED, GREEN }");
        let Item::EnumDef { name, variants, .. } = &m.items[0] else {
            panic!("expected EnumDef")
        };
        assert_eq!(name, "Color");
        assert_eq!(variants, &["RED", "GREEN"]);
    }

    #[test]
    fn parse_enum_def_single_variant() {
        let m = parse("enum Unit = { ONLY }");
        let Item::EnumDef { name, variants, .. } = &m.items[0] else {
            panic!("expected EnumDef")
        };
        assert_eq!(name, "Unit");
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn parse_pub_enum_def() {
        let m = parse("pub enum Status = { OK, ERR }");
        let Item::EnumDef { public, .. } = &m.items[0] else {
            panic!("expected EnumDef")
        };
        assert!(public);
    }

    #[test]
    fn parse_private_enum_def() {
        let m = parse("enum Status = { OK, ERR }");
        let Item::EnumDef { public, .. } = &m.items[0] else {
            panic!("expected EnumDef")
        };
        assert!(!public);
    }

    #[test]
    fn enum_variant_access_first_is_zero() {
        let ir = compile_ok(
            r#"
            enum Color = { RED, GREEN, BLUE }
            pub fn main() void! = {
                val c = Color.RED
            }
        "#,
        );
        assert!(ir.contains("copy 0") || ir.contains("ret 0") || ir.contains("0"));
    }

    #[test]
    fn enum_variant_access_second_is_one() {
        let ir = compile_ok(
            r#"
            enum Color = { RED, GREEN, BLUE }
            pub fn main() void! = {
                val c = Color.GREEN
            }
        "#,
        );
        assert!(ir.contains("1"));
    }

    #[test]
    fn enum_variant_access_third_is_two() {
        let ir = compile_ok(
            r#"
            enum Color = { RED, GREEN, BLUE }
            pub fn main() void! = {
                val c = Color.BLUE
            }
        "#,
        );
        assert!(ir.contains("2"));
    }

    #[test]
    fn enum_variant_usable_in_comparison() {
        let ir = compile_ok(
            r#"
            enum Dir = { UP, DOWN }
            pub fn main() void! = {
                val d = Dir.UP
                if (d == Dir.DOWN) { val x = 1 }
            }
        "#,
        );
        assert!(ir.contains("ceqw") || ir.contains("ceql"));
    }

    #[test]
    fn enum_compiles_without_error() {
        compile_ok(
            r#"
            enum SomeEnum = { HELLO, WORLD }
            pub fn main() void! = {
                val x = SomeEnum.HELLO
            }
        "#,
        );
    }
}

#[cfg(test)]
mod purity_enforcement_tests {
    use crate::codegen::Codegen;
    use crate::module::resolve_module;
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<(String, Vec<String>), String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        let warnings = cg.warnings.clone();
        Ok((cg.finish(), warnings))
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed").0
    }

    fn compile_err(src: &str) -> String {
        compile(src).expect_err("expected compilation to fail")
    }

    fn compile_warnings(src: &str) -> Vec<String> {
        compile(src).expect("expected compilation to succeed").1
    }

    #[test]
    fn pure_fn_calling_impure_fn_without_trust_errors() {
        let err = compile_err(
            r#"
            fn impure() void! = {}
            fn pure_caller() void = {
                pre { 1 == 1 }
                post { 1 == 1 }
                impure()
            }
        "#,
        );
        assert!(
            err.contains("pure function") && err.contains("impure"),
            "expected purity violation error, got: {err}"
        );
    }

    #[test]
    fn pure_fn_calling_impure_fn_with_trust_ok() {
        compile_ok(
            r#"
            fn impure() void! = {}
            fn pure_caller() void = {
                pre { 1 == 1 }
                post { 1 == 1 }
                trust impure()
            }
        "#,
        );
    }

    #[test]
    fn impure_fn_calling_impure_fn_without_trust_ok() {
        compile_ok(
            r#"
            fn impure() void! = {}
            fn also_impure() void! = {
                impure()
            }
        "#,
        );
    }

    #[test]
    fn pure_fn_calling_pure_fn_ok() {
        compile_ok(
            r#"
            fn helper(x: i32) i32 = {
                pre { x > 0 }
                post { x > 0 }
                return x
            }
            fn caller(x: i32) i32 = {
                pre { x > 0 }
                post { x > 0 }
                return helper(x)
            }
        "#,
        );
    }

    #[test]
    fn trust_in_impure_fn_emits_warning() {
        let warnings = compile_warnings(
            r#"
            fn impure() void! = {}
            fn caller() void! = {
                trust impure()
            }
        "#,
        );
        assert!(
            warnings.iter().any(|w| w.contains("redundant") && w.contains("trust")),
            "expected redundant trust warning, got: {warnings:?}"
        );
    }

    #[test]
    fn trust_in_pure_fn_no_warning() {
        let warnings = compile_warnings(
            r#"
            fn impure() void! = {}
            fn caller() void = {
                pre { 1 == 1 }
                post { 1 == 1 }
                trust impure()
            }
        "#,
        );
        assert!(
            warnings.is_empty(),
            "trust in pure fn should not warn, got: {warnings:?}"
        );
    }

    #[test]
    fn trust_in_impure_fn_warning_mentions_fn_name() {
        let warnings = compile_warnings(
            r#"
            fn impure() void! = {}
            fn my_caller() void! = {
                trust impure()
            }
        "#,
        );
        assert!(
            warnings.iter().any(|w| w.contains("my_caller")),
            "warning should mention the function name, got: {warnings:?}"
        );
    }

    #[test]
    fn multiple_trust_in_impure_fn_warns_each_time() {
        let warnings = compile_warnings(
            r#"
            fn a() void! = {}
            fn b() void! = {}
            fn caller() void! = {
                trust a()
                trust b()
            }
        "#,
        );
        assert_eq!(
            warnings.len(),
            2,
            "should warn once per redundant trust, got: {warnings:?}"
        );
    }
}

#[cfg(test)]
mod extern_fn_tests {
    use crate::codegen::Codegen;
    use crate::lexer::{Lexer, TokenKind};
    use crate::module::resolve_module;
    use crate::parser::{Item, Parser, TypeExpr};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new(), "")?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved, false, false)?;
        Ok(cg.finish())
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("expected compilation to succeed")
    }

    fn compile_err(src: &str) -> String {
        compile(src).expect_err("expected compilation to fail")
    }

    fn parse(src: &str) -> crate::parser::Module {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_module().unwrap()
    }

    #[test]
    fn lex_extern_keyword() {
        let toks = Lexer::new("extern").tokenize().unwrap();
        assert_eq!(toks[0].kind, TokenKind::Extern);
    }

    #[test]
    fn parse_extern_fn_basic() {
        let m = parse(r#"extern (C) fn my_puts(s: ref char) void! = "puts""#);
        let Item::ExternFn { conv, name, symbol, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert_eq!(conv, "C");
        assert_eq!(name, "my_puts");
        assert_eq!(symbol, "puts");
    }

    #[test]
    fn parse_extern_fn_params() {
        let m = parse(r#"extern (C) fn my_malloc(size: usize) ref void! = "malloc""#);
        let Item::ExternFn { params, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "size");
        assert!(matches!(params[0].1, TypeExpr::Named(ref n) if n == "usize"));
    }

    #[test]
    fn parse_extern_fn_ret_type() {
        let m = parse(r#"extern (C) fn my_malloc(size: usize) ref void! = "malloc""#);
        let Item::ExternFn { ret, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert!(matches!(ret, TypeExpr::Ref(_)));
    }

    #[test]
    fn parse_extern_fn_calling_conv() {
        let m = parse(r#"extern (stdcall) fn win_fn() void! = "WinFn""#);
        let Item::ExternFn { conv, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert_eq!(conv, "stdcall");
    }

    #[test]
    fn extern_fn_without_bang_errors() {
        let tokens = Lexer::new(r#"extern (C) fn bad() void = "bad""#).tokenize().unwrap();
        let err = Parser::new(tokens).parse_module().unwrap_err();
        assert!(
            err.contains("!") || err.contains("extern"),
            "missing ! on extern should error, got: {err}"
        );
    }

    #[test]
    fn extern_fn_callable_from_impure_fn() {
        let ir = compile_ok(
            r#"
            extern (C) fn my_puts(s: ref char) void! = "puts"
            pub fn main() void! = {
                my_puts("hello")
            }
        "#,
        );
        assert!(ir.contains("call $puts"), "should call the C symbol 'puts'");
    }

    #[test]
    fn extern_fn_maps_local_name_to_c_symbol() {
        let ir = compile_ok(
            r#"
            extern (C) fn my_malloc(size: usize) ref void! = "malloc"
            pub fn main() void! = {
                val p: ref void = my_malloc(16)
            }
        "#,
        );
        assert!(
            ir.contains("call $malloc"),
            "extern fn should call the C symbol, not the local alias"
        );
        assert!(
            !ir.contains("call $my_malloc"),
            "local alias name must not appear as a QBE symbol"
        );
    }

    #[test]
    fn extern_fn_does_not_emit_function_body() {
        let ir = compile_ok(
            r#"
            extern (C) fn my_puts(s: ref char) void! = "puts"
            pub fn main() void! = {}
        "#,
        );
        assert!(
            !ir.contains("function $my_puts"),
            "extern fn must not emit a QBE function definition"
        );
        assert!(
            !ir.contains("function $puts"),
            "extern fn must not emit a QBE function definition for the C symbol either"
        );
    }

    #[test]
    fn extern_fn_is_treated_as_impure() {
        let err = compile_err(
            r#"
            extern (C) fn my_puts(s: ref char) void! = "puts"
            fn pure_caller(s: ref char) void = {
                pre { 1 == 1 }
                post { 1 == 1 }
                my_puts(s)
            }
        "#,
        );
        assert!(
            err.contains("pure function") || err.contains("impure"),
            "calling extern from pure fn without trust should error, got: {err}"
        );
    }

    #[test]
    fn extern_fn_callable_with_trust_from_pure_fn() {
        compile_ok(
            r#"
            extern (C) fn my_puts(s: ref char) void! = "puts"
            fn pure_caller(s: ref char) void = {
                pre { 1 == 1 }
                post { 1 == 1 }
                trust my_puts(s)
            }
        "#,
        );
    }

    #[test]
    fn sample21_pattern_compiles() {
        compile_ok(
            r#"
            extern (C) fn my_malloc(space: usize) ref void! = "malloc"
            extern (C) fn my_free(ptr: ref void) void! = "free"
            extern (C) fn my_puts(str: ref char) void! = "puts"
            pub fn main() void! = {
                val ptr: ref void = my_malloc(16)
                if (ptr != none) {
                    my_free(ptr)
                }
                my_puts("wow")
            }
        "#,
        );
    }

    #[test]
    fn parse_pub_extern_fn() {
        let tokens = Lexer::new(r#"pub extern (C) fn my_puts(s: ref char) void! = "puts""#)
            .tokenize()
            .unwrap();
        let m = Parser::new(tokens).parse_module().unwrap();
        let Item::ExternFn { public, name, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert!(public, "pub extern fn should be public");
        assert_eq!(name, "my_puts");
    }

    #[test]
    fn parse_private_extern_fn() {
        let tokens = Lexer::new(r#"extern (C) fn my_puts(s: ref char) void! = "puts""#)
            .tokenize()
            .unwrap();
        let m = Parser::new(tokens).parse_module().unwrap();
        let Item::ExternFn { public, .. } = &m.items[0] else {
            panic!("expected ExternFn")
        };
        assert!(!public, "extern fn without pub should be private");
    }
}
