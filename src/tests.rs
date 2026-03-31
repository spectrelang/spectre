#[cfg(test)]
mod lexer_tests {
    use crate::lexer::{Lexer, Token};

    fn lex(src: &str) -> Vec<Token> {
        Lexer::new(src).tokenize().unwrap()
    }

    fn lex_err(src: &str) -> String {
        Lexer::new(src).tokenize().unwrap_err()
    }

    #[test]
    fn keywords() {
        let toks = lex("fn val mut pub type pre post return if else trust rely use ref some none not");
        assert_eq!(toks[0], Token::Fn);
        assert_eq!(toks[1], Token::Val);
        assert_eq!(toks[2], Token::Mut);
        assert_eq!(toks[3], Token::Pub);
        assert_eq!(toks[4], Token::Type);
        assert_eq!(toks[5], Token::Pre);
        assert_eq!(toks[6], Token::Post);
        assert_eq!(toks[7], Token::Return);
        assert_eq!(toks[8], Token::If);
        assert_eq!(toks[9], Token::Else);
        assert_eq!(toks[10], Token::Trust);
        assert_eq!(toks[11], Token::Rely);
        assert_eq!(toks[12], Token::Use);
        assert_eq!(toks[13], Token::Ref);
        assert_eq!(toks[14], Token::Some);
        assert_eq!(toks[15], Token::None_);
        assert_eq!(toks[16], Token::Not);
    }

    #[test]
    fn integer_literal() {
        let toks = lex("42 0 1000");
        assert_eq!(toks[0], Token::IntLit(42));
        assert_eq!(toks[1], Token::IntLit(0));
        assert_eq!(toks[2], Token::IntLit(1000));
    }

    #[test]
    fn string_literal() {
        let toks = lex(r#""hello" "world""#);
        assert_eq!(toks[0], Token::StringLit("hello".into()));
        assert_eq!(toks[1], Token::StringLit("world".into()));
    }

    #[test]
    fn string_escape_sequences() {
        let toks = lex(r#""\n\t\"\\""#);
        assert_eq!(toks[0], Token::StringLit("\n\t\"\\".into()));
    }

    #[test]
    fn operators_single_and_double() {
        let toks = lex("= == ! != < <= > >= & && | ||");
        assert_eq!(toks[0], Token::Eq);
        assert_eq!(toks[1], Token::EqEq);
        assert_eq!(toks[2], Token::Bang);
        assert_eq!(toks[3], Token::BangEq);
        assert_eq!(toks[4], Token::Lt);
        assert_eq!(toks[5], Token::LtEq);
        assert_eq!(toks[6], Token::Gt);
        assert_eq!(toks[7], Token::GtEq);
        assert_eq!(toks[8], Token::And);
        assert_eq!(toks[9], Token::And);
        assert_eq!(toks[10], Token::Or);
        assert_eq!(toks[11], Token::Or);
    }

    #[test]
    fn double_ampersand_is_single_and_token() {
        let toks = lex("a && b");
        assert_eq!(toks[0], Token::Ident("a".into()));
        assert_eq!(toks[1], Token::And);
        assert_eq!(toks[2], Token::Ident("b".into()));
    }

    #[test]
    fn double_pipe_is_single_or_token() {
        let toks = lex("a || b");
        assert_eq!(toks[0], Token::Ident("a".into()));
        assert_eq!(toks[1], Token::Or);
        assert_eq!(toks[2], Token::Ident("b".into()));
    }

    #[test]
    fn arrow_token() {
        let toks = lex("->");
        assert_eq!(toks[0], Token::Arrow);
    }

    #[test]
    fn punctuation() {
        let toks = lex("( ) { } [ ] : ; , . @ + - * / %");
        assert_eq!(toks[0], Token::LParen);
        assert_eq!(toks[1], Token::RParen);
        assert_eq!(toks[2], Token::LBrace);
        assert_eq!(toks[3], Token::RBrace);
        assert_eq!(toks[4], Token::LBracket);
        assert_eq!(toks[5], Token::RBracket);
        assert_eq!(toks[6], Token::Colon);
        assert_eq!(toks[7], Token::Semicolon);
        assert_eq!(toks[8], Token::Comma);
        assert_eq!(toks[9], Token::Dot);
        assert_eq!(toks[10], Token::At);
        assert_eq!(toks[11], Token::Plus);
        assert_eq!(toks[12], Token::Minus);
        assert_eq!(toks[13], Token::Star);
        assert_eq!(toks[14], Token::Slash);
        assert_eq!(toks[15], Token::Percent);
    }

    #[test]
    fn comments_are_skipped() {
        let toks = lex("42 // this is a comment\n99");
        assert_eq!(toks[0], Token::IntLit(42));
        assert_eq!(toks[1], Token::IntLit(99));
        assert_eq!(toks[2], Token::Eof);
    }

    #[test]
    fn eof_at_end() {
        let toks = lex("x");
        assert_eq!(*toks.last().unwrap(), Token::Eof);
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
        assert_eq!(toks[0], Token::Ident("some_var".into()));
        assert_eq!(toks[1], Token::Ident("_private".into()));
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
        let Item::TypeDef { name, fields } = &m.items[0] else { panic!() };
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
        let Stmt::Val { name, mutable, .. } = &f.body[0] else { panic!() };
        assert_eq!(name, "x");
        assert!(!mutable);
    }

    #[test]
    fn parse_val_binding_mutable() {
        let m = parse("fn f() void! = { val x: mut i32 = 10 }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { mutable, .. } = &f.body[0] else { panic!() };
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
        let Stmt::Pre(contracts) = &f.body[0] else { panic!() };
        assert_eq!(contracts[0].label.as_deref(), Some("is_pos"));
    }

    #[test]
    fn parse_and_in_contract() {
        let m = parse("fn f(x: i32, y: i32) void = { pre { x > 0 && y > 0 } post { x > 0 && y > 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Pre(contracts) = &f.body[0] else { panic!() };
        assert!(matches!(contracts[0].expr, Expr::BinOp { .. }));
    }

    #[test]
    fn parse_or_in_contract() {
        let m = parse("fn f(x: i32) void = { pre { x > 0 || x < 0 } post { x > 0 || x < 0 } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Pre(contracts) = &f.body[0] else { panic!() };
        assert!(matches!(contracts[0].expr, Expr::BinOp { .. }));
    }

    #[test]
    fn parse_trust_expr() {
        let m = parse("fn f() void = { trust foo() }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Expr(Expr::Trust(_)) = &f.body[0] else { panic!() };
    }

    #[test]
    fn parse_if_else() {
        let m = parse("fn f(x: i32) void! = { if (x > 0) { return } else { return } }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::If { else_, .. } = &f.body[0] else { panic!() };
        assert!(else_.is_some());
    }

    #[test]
    fn parse_struct_literal() {
        let m = parse("fn f() void! = { val p = {x: 1, y: 2} }");
        let Item::Fn(f) = &m.items[0] else { panic!() };
        let Stmt::Val { expr, .. } = &f.body[0] else { panic!() };
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
        let Stmt::Val { expr, .. } = &f.body[0] else { panic!() };
        let Expr::BinOp { op: crate::parser::BinOp::Add, rhs, .. } = expr else { panic!() };
        assert!(matches!(rhs.as_ref(), Expr::BinOp { op: crate::parser::BinOp::Mul, .. }));
    }

    #[test]
    fn parse_error_unexpected_top_level() {
        assert!(parse_err("42").contains("unexpected token at top level"));
    }
}

#[cfg(test)]
mod codegen_tests {
    use crate::codegen::Codegen;
    use crate::lexer::Lexer;
    use crate::module::resolve_module;
    use crate::parser::Parser;
    use std::collections::HashMap;
    use std::path::Path;

    fn compile(src: &str) -> Result<String, String> {
        let resolved = resolve_module(src, Path::new("."), &mut HashMap::new())?;
        let mut cg = Codegen::new();
        cg.emit_module(&resolved)?;
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
        let ir = compile_ok(r#"
            pub fn print(arg: ref char) void = {
                trust @puts(arg)
            }
        "#);
        assert!(ir.contains("$print"));
    }

    #[test]
    fn pure_fn_with_pre_post_ok() {
        let ir = compile_ok("fn f(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("$f"));
    }

    #[test]
    fn pure_fn_missing_pre_errors() {
        let err = compile_err("fn f(x: i32) i32 = { post { x > 0 } return x }");
        assert!(err.contains("pure function") && err.contains("pre"));
    }

    #[test]
    fn pure_fn_missing_post_errors() {
        let err = compile_err("fn f(x: i32) i32 = { pre { x > 0 } return x }");
        assert!(err.contains("pure function") && err.contains("post"));
    }

    #[test]
    fn pure_fn_missing_both_contracts_errors() {
        let err = compile_err("fn f(x: i32) i32 = { return x }");
        assert!(err.contains("pure function"));
    }

    #[test]
    fn pure_fn_with_assignment_requires_contracts() {
        let err = compile_err(r#"
            type T = { x: mut i32 }
            fn f(x: i32) void = {
                val t: mut T = {x: 1}
                t.x = 2
            }
        "#);
        assert!(err.contains("pure function"));
    }

    #[test]
    fn immutable_binding_field_assign_errors() {
        let err = compile_err(r#"
            type T = { x: mut i32 }
            pub fn main() void! = {
                val t: T = {x: 1}
                t.x = 2
            }
        "#);
        assert!(err.contains("immutable binding"));
    }

    #[test]
    fn immutable_field_assign_errors() {
        let err = compile_err(r#"
            type T = { x: i32 y: mut i32 }
            pub fn main() void! = {
                val t: mut T = {x: 1, y: 2}
                t.x = 10
            }
        "#);
        assert!(err.contains("immutable field") && err.contains("'x'"));
    }

    #[test]
    fn mutable_field_assign_ok() {
        let ir = compile_ok(r#"
            type T = { x: i32 y: mut i32 }
            pub fn main() void! = {
                val t: mut T = {x: 1, y: 2}
                t.y = 10
            }
        "#);
        assert!(ir.contains("storew"));
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
        let ir = compile_ok("fn f(x: i32) i32 = { pre { is_pos: x > 0 } post { is_pos: x > 0 } return x }");
        assert!(ir.contains("is_pos"));
    }

    #[test]
    fn contract_message_includes_fn_name() {
        let ir = compile_ok("fn my_func(x: i32) i32 = { pre { x > 0 } post { x > 0 } return x }");
        assert!(ir.contains("my_func"));
    }

    #[test]
    fn pre_contract_with_and() {
        let ir = compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 && y > 0 } post { x > 0 } return x }");
        assert!(ir.contains("and"));
    }

    #[test]
    fn pre_contract_with_or() {
        let ir = compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 || y > 0 } post { x > 0 } return x }");
        assert!(ir.contains("or"));
    }

    #[test]
    fn struct_literal_calls_malloc() {
        let ir = compile_ok(r#"
            type T = { x: i32 }
            pub fn main() void! = {
                val t: T = {x: 1}
            }
        "#);
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
        let ir = compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x + y }");
        assert!(ir.contains("=w add"));
    }

    #[test]
    fn binop_sub_emits_sub() {
        let ir = compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x - y }");
        assert!(ir.contains("=w sub"));
    }

    #[test]
    fn binop_mul_emits_mul() {
        let ir = compile_ok("fn f(x: i32, y: i32) i32 = { pre { x > 0 } post { x > 0 } return x * y }");
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
        let ir = compile_ok(r#"
            pub fn put_any(fmt: ref char, args: untyped) void! = {}
            pub fn main() void! = {
                put_any("{d}", 1)
            }
        "#);
        assert!(ir.contains("b \"%d\""));
    }

    #[test]
    fn if_without_else_emits_branches() {
        let ir = compile_ok(r#"
            pub fn main() void! = {
                if (1 == 1) { val x = 1 }
            }
        "#);
        assert!(ir.contains("@if_then_"));
        assert!(ir.contains("@if_end_"));
    }

    #[test]
    fn if_with_else_emits_else_branch() {
        let ir = compile_ok(r#"
            pub fn main() void! = {
                if (1 == 1) { val x = 1 } else { val y = 2 }
            }
        "#);
        assert!(ir.contains("@if_else_"));
    }

    #[test]
    fn undefined_variable_errors() {
        let err = compile_err("pub fn main() void! = { val x = undefined_var }");
        assert!(err.contains("undefined variable"));
    }

    #[test]
    fn pure_fn_calling_only_fns_no_contracts_needed() {
        let ir = compile_ok(r#"
            pub fn helper() void! = {}
            pub fn main() void = {
                helper()
            }
        "#);
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
        let resolved = resolve_module(&src, Path::new("."), &mut HashMap::new()).unwrap();
        let mut cg = Codegen::new();
        cg.emit_module(&resolved).unwrap();
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
