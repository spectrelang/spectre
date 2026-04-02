use crate::parser::{Expr, FnDef, Item, Stmt};
use crate::module::ResolvedModule;
use std::collections::{HashMap, HashSet};

/// Run all semantic checks on a resolved module tree.
/// Returns a list of error messages (empty = no errors).
pub fn check_module(resolved: &ResolvedModule) -> Vec<String> {
    let mut errors = Vec::new();
    check_module_recursive(resolved, &mut errors);
    errors
}

fn check_module_recursive(resolved: &ResolvedModule, errors: &mut Vec<String>) {
    for child in resolved.imports.values() {
        check_module_recursive(child, errors);
    }
    for item in &resolved.ast.items {
        if let Item::Fn(f) = item {
            check_fn(f, &resolved.filename, errors);
        }
    }
}

fn check_fn(f: &FnDef, filename: &str, errors: &mut Vec<String>) {
    let mut declared: HashMap<String, bool> = HashMap::new();
    let mut for_vars: HashSet<String> = HashSet::new();

    collect_declarations(&f.body, &mut declared, &mut for_vars);

    let mut used: HashSet<String> = HashSet::new();
    collect_used_in_stmts(&f.body, &mut used);

    let mut mutated: HashSet<String> = HashSet::new();
    collect_mutated_in_stmts(&f.body, &mut mutated);

    let param_names: HashSet<String> = f.params.iter().map(|(n, _)| n.clone()).collect();

    for (name, is_mutable) in &declared {
        if !for_vars.contains(name) && *is_mutable && !mutated.contains(name) {
            errors.push(format!(
                "{filename}: in function '{}': variable '{name}' is declared as mutable but is never mutated",
                f.name
            ));
        }

        if !param_names.contains(name) && !used.contains(name) {
            errors.push(format!(
                "{filename}: in function '{}': variable '{name}' is declared but never used",
                f.name
            ));
        }
    }
}

fn collect_declarations(
    stmts: &[Stmt],
    declared: &mut HashMap<String, bool>,
    for_vars: &mut HashSet<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Val { name, mutable, .. } => {
                declared.insert(name.clone(), *mutable);
            }
            Stmt::For { init, body, .. } => {
                if let Some((var, _)) = init {
                    declared.insert(var.clone(), true);
                    for_vars.insert(var.clone());
                }
                collect_declarations(body, declared, for_vars);
            }
            Stmt::If { then, elif_, else_, .. } => {
                collect_declarations(then, declared, for_vars);
                for (_, b) in elif_ {
                    collect_declarations(b, declared, for_vars);
                }
                if let Some(b) = else_ {
                    collect_declarations(b, declared, for_vars);
                }
            }
            Stmt::Match { some_body, none_body, some_binding, .. } => {
                declared.insert(some_binding.clone(), false);
                collect_declarations(some_body, declared, for_vars);
                collect_declarations(none_body, declared, for_vars);
            }
            Stmt::Defer(body)
            | Stmt::When { body, .. }
            | Stmt::WhenIs { body, .. }
            | Stmt::Otherwise { body } => {
                collect_declarations(body, declared, for_vars);
            }
            _ => {}
        }
    }
}

fn collect_used_in_stmts(stmts: &[Stmt], used: &mut HashSet<String>) {
    for stmt in stmts {
        collect_used_in_stmt(stmt, used);
    }
}

fn collect_used_in_stmt(stmt: &Stmt, used: &mut HashSet<String>) {
    match stmt {
        Stmt::Val { expr, .. } => collect_used_in_expr(expr, used),
        Stmt::Assign { target, value } => {
            collect_used_in_assign_target(target, used);
            collect_used_in_expr(value, used);
        }
        Stmt::Return(Some(e)) => collect_used_in_expr(e, used),
        Stmt::Return(None) => {}
        Stmt::Expr(e) => collect_used_in_expr(e, used),
        Stmt::Pre(cs) | Stmt::Post(cs) => {
            for c in cs {
                collect_used_in_expr(&c.expr, used);
            }
        }
        Stmt::Assert(e, _) => collect_used_in_expr(e, used),
        Stmt::If { cond, then, elif_, else_, .. } => {
            collect_used_in_expr(cond, used);
            collect_used_in_stmts(then, used);
            for (ec, eb) in elif_ {
                collect_used_in_expr(ec, used);
                collect_used_in_stmts(eb, used);
            }
            if let Some(b) = else_ {
                collect_used_in_stmts(b, used);
            }
        }
        Stmt::For { init, cond, post, body } => {
            if let Some((_, e)) = init {
                collect_used_in_expr(e, used);
            }
            if let Some(e) = cond {
                collect_used_in_expr(e, used);
            }
            if let Some(s) = post {
                collect_used_in_stmt(s, used);
            }
            collect_used_in_stmts(body, used);
        }
        Stmt::Increment(name) => {
            used.insert(name.clone());
        }
        Stmt::Decrement(name) => {
            used.insert(name.clone());
        }
        Stmt::AddAssign(name, expr) => {
            used.insert(name.clone());
            collect_used_in_expr(expr, used);
        }
        Stmt::SubAssign(name, expr) => {
            used.insert(name.clone());
            collect_used_in_expr(expr, used);
        }
        Stmt::Defer(body)
        | Stmt::When { body, .. }
        | Stmt::Otherwise { body } => {
            collect_used_in_stmts(body, used);
        }
        Stmt::WhenIs { expr, body, .. } => {
            collect_used_in_expr(expr, used);
            collect_used_in_stmts(body, used);
        }
        Stmt::Match { expr, some_binding, some_body, none_body } => {
            collect_used_in_expr(expr, used);
            used.insert(some_binding.clone());
            collect_used_in_stmts(some_body, used);
            collect_used_in_stmts(none_body, used);
        }
        Stmt::Break => {}
    }
}

fn collect_used_in_assign_target(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(_) => {
        }
        Expr::Field(base, _) => {
            collect_used_in_expr(base, used);
        }
        other => collect_used_in_expr(other, used),
    }
}

fn collect_used_in_expr(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name) => {
            used.insert(name.clone());
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_used_in_expr(lhs, used);
            collect_used_in_expr(rhs, used);
        }
        Expr::UnOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Some(expr)
        | Expr::Trust(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => {
            collect_used_in_expr(expr, used);
        }
        Expr::Field(base, _) => collect_used_in_expr(base, used),
        Expr::Call { callee, args, .. } => {
            collect_used_in_expr(callee, used);
            for a in args {
                collect_used_in_expr(a, used);
            }
        }
        Expr::Builtin { args, .. } => {
            for a in args {
                collect_used_in_expr(a, used);
            }
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields {
                collect_used_in_expr(e, used);
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                collect_used_in_expr(e, used);
            }
        }
        Expr::IntLit(_)
        | Expr::FloatLit(_)
        | Expr::StrLit(_)
        | Expr::Bool(_)
        | Expr::None => {}
    }
}

fn collect_mutated_in_stmts(stmts: &[Stmt], mutated: &mut HashSet<String>) {
    for stmt in stmts {
        collect_mutated_in_stmt(stmt, mutated);
    }
}

fn collect_mutated_in_stmt(stmt: &Stmt, mutated: &mut HashSet<String>) {
    match stmt {
        Stmt::Assign { target, .. } => {
            if let Some(root) = expr_root_name(target) {
                mutated.insert(root);
            }
        }
        Stmt::Increment(name) => {
            mutated.insert(name.clone());
        }
        Stmt::Decrement(name) => {
            mutated.insert(name.clone());
        }
        Stmt::AddAssign(name, _) => {
            mutated.insert(name.clone());
        }
        Stmt::SubAssign(name, _) => {
            mutated.insert(name.clone());
        }
        Stmt::If { then, elif_, else_, .. } => {
            collect_mutated_in_stmts(then, mutated);
            for (_, b) in elif_ {
                collect_mutated_in_stmts(b, mutated);
            }
            if let Some(b) = else_ {
                collect_mutated_in_stmts(b, mutated);
            }
        }
        Stmt::For { body, post, .. } => {
            if let Some(s) = post {
                collect_mutated_in_stmt(s, mutated);
            }
            collect_mutated_in_stmts(body, mutated);
        }
        Stmt::Match { some_body, none_body, .. } => {
            collect_mutated_in_stmts(some_body, mutated);
            collect_mutated_in_stmts(none_body, mutated);
        }
        Stmt::Defer(body)
        | Stmt::When { body, .. }
        | Stmt::WhenIs { body, .. }
        | Stmt::Otherwise { body } => {
            collect_mutated_in_stmts(body, mutated);
        }
        _ => {}
    }
}

fn expr_root_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::Field(base, _) => expr_root_name(base),
        _ => None,
    }
}
