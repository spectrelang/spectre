use crate::parser::{Expr, FnDef, Item, Stmt, TypeExpr};
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

    let fn_lookup = build_fn_lookup(resolved);

    for item in &resolved.ast.items {
        if let Item::Fn(f) = item {
            check_fn(f, &resolved.filename, &fn_lookup, errors);
        }
    }
}

/// Build a map from qualified function path (e.g. "foo", "mod.foo") to its parameter list.
/// This is used to check whether a call passes an immutable value to a `ref` parameter.
fn build_fn_lookup<'a>(resolved: &'a ResolvedModule) -> HashMap<String, &'a Vec<(String, TypeExpr)>> {
    let mut map = HashMap::new();
    for item in &resolved.ast.items {
        if let Item::Fn(f) = item {
            let key = match &f.namespace {
                Some(ns) => format!("{ns}.{}", f.name),
                None => f.name.clone(),
            };
            map.insert(key, &f.params);
        }
    }
    for (mod_name, child) in &resolved.imports {
        for item in &child.ast.items {
            if let Item::Fn(f) = item {
                let key = match &f.namespace {
                    Some(ns) => format!("{mod_name}.{ns}.{}", f.name),
                    None => format!("{mod_name}.{}", f.name),
                };
                map.insert(key, &f.params);
            }
        }
    }
    map
}

fn check_fn(f: &FnDef, filename: &str, fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>, errors: &mut Vec<String>) {
    let mut declared: HashMap<String, bool> = HashMap::new();
    let mut for_vars: HashSet<String> = HashSet::new();

    collect_declarations(&f.body, &mut declared, &mut for_vars);

    let mut used: HashSet<String> = HashSet::new();
    collect_used_in_stmts(&f.body, &mut used);

    let mut mutated: HashSet<String> = HashSet::new();
    collect_mutated_in_stmts(&f.body, &mut mutated);

    let mut ref_used: HashSet<String> = HashSet::new();
    collect_ref_used_in_stmts(&f.body, fn_lookup, &mut ref_used);

    let param_names: HashSet<String> = f.params.iter().map(|(n, _)| n.clone()).collect();

    let mut scope_stack: Vec<HashSet<String>> = vec![param_names.clone()];
    check_shadowing_in_stmts(&f.body, &mut scope_stack, &f.name, filename, errors);

    for (name, is_mutable) in &declared {
        if !for_vars.contains(name) && *is_mutable && !mutated.contains(name) && !ref_used.contains(name) {
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

    let mut var_mutability: HashMap<String, bool> = HashMap::new();
    let mut var_types: HashMap<String, TypeExpr> = HashMap::new();
    for (pname, pty) in &f.params {
        var_mutability.insert(pname.clone(), true);
        var_types.insert(pname.clone(), pty.clone());
    }
    collect_var_mutability(&f.body, &mut var_mutability);
    collect_var_types(&f.body, &mut var_types);

    check_immutable_args_in_stmts(&f.body, &var_mutability, &var_types, fn_lookup, &f.name, filename, errors);
}

fn check_shadowing_in_stmts(
    stmts: &[Stmt],
    scope_stack: &mut Vec<HashSet<String>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        check_shadowing_in_stmt(stmt, scope_stack, fn_name, filename, errors);
    }
}

fn check_shadowing_in_stmt(
    stmt: &Stmt,
    scope_stack: &mut Vec<HashSet<String>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Stmt::Val { name, .. } => {
            let shadowed = scope_stack.iter().any(|s| s.contains(name));
            if shadowed {
                errors.push(format!(
                    "{filename}: in function '{fn_name}': '{name}' shadows an existing declaration"
                ));
            }
            if let Some(top) = scope_stack.last_mut() {
                top.insert(name.clone());
            }
        }
        Stmt::For { init, body, .. } => {
            scope_stack.push(HashSet::new());
            if let Some((var, _)) = init {
                if let Some(top) = scope_stack.last_mut() {
                    top.insert(var.clone());
                }
            }
            check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
        }
        Stmt::If { then, elif_, else_, .. } => {
            scope_stack.push(HashSet::new());
            check_shadowing_in_stmts(then, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
            for (_, b) in elif_ {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(b, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
            if let Some(b) = else_ {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(b, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
        }
        Stmt::Match { some_binding, some_body, none_body, .. } => {
            scope_stack.push(HashSet::new());
            let shadowed = scope_stack.iter().rev().skip(1).any(|s| s.contains(some_binding));
            if shadowed {
                errors.push(format!(
                    "{filename}: in function '{fn_name}': '{some_binding}' shadows an existing declaration"
                ));
            }
            if let Some(top) = scope_stack.last_mut() {
                top.insert(some_binding.clone());
            }
            check_shadowing_in_stmts(some_body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
            scope_stack.push(HashSet::new());
            check_shadowing_in_stmts(none_body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
        }
        Stmt::MatchResult { ok_binding, ok_body, err_binding, err_body, .. } => {
            scope_stack.push(HashSet::new());
            if let Some(top) = scope_stack.last_mut() {
                top.insert(ok_binding.clone());
            }
            check_shadowing_in_stmts(ok_body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
            scope_stack.push(HashSet::new());
            if let Some(top) = scope_stack.last_mut() {
                top.insert(err_binding.clone());
            }
            check_shadowing_in_stmts(err_body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
        }
        Stmt::MatchEnum { arms, .. } => {
            for (_, body) in arms {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
        }
        Stmt::Defer(body)
        | Stmt::When { body, .. }
        | Stmt::WhenIs { body, .. }
        | Stmt::Otherwise { body } => {
            scope_stack.push(HashSet::new());
            check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
        }
        _ => {}
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
            Stmt::MatchResult { ok_binding, ok_body, err_binding, err_body, .. } => {
                declared.insert(ok_binding.clone(), false);
                declared.insert(err_binding.clone(), false);
                collect_declarations(ok_body, declared, for_vars);
                collect_declarations(err_body, declared, for_vars);
            }
            Stmt::MatchEnum { arms, .. } => {
                for (_, body) in arms {
                    collect_declarations(body, declared, for_vars);
                }
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
        Stmt::Pre(cs) | Stmt::Post(cs) | Stmt::GuardedPre(cs) | Stmt::GuardedPost(cs) => {
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
        Stmt::MatchResult { expr, ok_binding, ok_body, err_binding, err_body } => {
            collect_used_in_expr(expr, used);
            used.insert(ok_binding.clone());
            used.insert(err_binding.clone());
            collect_used_in_stmts(ok_body, used);
            collect_used_in_stmts(err_body, used);
        }
        Stmt::MatchEnum { expr, arms } => {
            collect_used_in_expr(expr, used);
            for (_, body) in arms {
                collect_used_in_stmts(body, used);
            }
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
        | Expr::OkVal(expr)
        | Expr::ErrVal(expr)
        | Expr::Try(expr)
        | Expr::Trust(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => {
            collect_used_in_expr(expr, used);
        }
        Expr::Field(base, _) => collect_used_in_expr(base, used),        Expr::Call { callee, args, .. } => {
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
        | Expr::None
        | Expr::ZeroInit(_) => {}
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
        Stmt::Val { expr, .. } => collect_addr_deref_in_expr(expr, mutated),
        Stmt::Expr(expr) => collect_addr_deref_in_expr(expr, mutated),
        Stmt::Return(Some(expr)) => collect_addr_deref_in_expr(expr, mutated),
        Stmt::If { cond, then, elif_, else_, .. } => {
            collect_addr_deref_in_expr(cond, mutated);
            collect_mutated_in_stmts(then, mutated);
            for (ec, b) in elif_ {
                collect_addr_deref_in_expr(ec, mutated);
                collect_mutated_in_stmts(b, mutated);
            }
            if let Some(b) = else_ {
                collect_mutated_in_stmts(b, mutated);
            }
        }
        Stmt::For { init, cond, body, post } => {
            if let Some((_, e)) = init { collect_addr_deref_in_expr(e, mutated); }
            if let Some(e) = cond { collect_addr_deref_in_expr(e, mutated); }
            if let Some(s) = post {
                collect_mutated_in_stmt(s, mutated);
            }
            collect_mutated_in_stmts(body, mutated);
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            collect_addr_deref_in_expr(expr, mutated);
            collect_mutated_in_stmts(some_body, mutated);
            collect_mutated_in_stmts(none_body, mutated);
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            collect_addr_deref_in_expr(expr, mutated);
            collect_mutated_in_stmts(ok_body, mutated);
            collect_mutated_in_stmts(err_body, mutated);
        }
        Stmt::MatchEnum { expr, arms } => {
            collect_addr_deref_in_expr(expr, mutated);
            for (_, body) in arms {
                collect_mutated_in_stmts(body, mutated);
            }
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
fn collect_addr_deref_in_expr(expr: &Expr, mutated: &mut HashSet<String>) {
    match expr {
        Expr::Addr(inner) | Expr::Deref(inner) => {
            if let Expr::Ident(name) = inner.as_ref() {
                mutated.insert(name.clone());
            }
            collect_addr_deref_in_expr(inner, mutated);
        }
        Expr::Call { callee, args, .. } => {
            collect_addr_deref_in_expr(callee, mutated);
            for a in args { collect_addr_deref_in_expr(a, mutated); }
        }
        Expr::Builtin { args, .. } => {
            for a in args { collect_addr_deref_in_expr(a, mutated); }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_addr_deref_in_expr(lhs, mutated);
            collect_addr_deref_in_expr(rhs, mutated);
        }
        Expr::UnOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Some(expr)
        | Expr::OkVal(expr)
        | Expr::ErrVal(expr)
        | Expr::Try(expr)
        | Expr::Trust(expr)
        | Expr::Field(expr, _) => collect_addr_deref_in_expr(expr, mutated),
        Expr::StructLit { fields } => {
            for (_, e) in fields { collect_addr_deref_in_expr(e, mutated); }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs { collect_addr_deref_in_expr(e, mutated); }
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

/// Build a flat map of variable name -> declared TypeExpr from `val` declarations.
fn collect_var_types(stmts: &[Stmt], map: &mut HashMap<String, TypeExpr>) {
    for stmt in stmts {
        match stmt {
            Stmt::Val { name, ty: Some(ty), .. } => { map.insert(name.clone(), ty.clone()); }
            Stmt::For { body, .. } => collect_var_types(body, map),
            Stmt::If { then, elif_, else_, .. } => {
                collect_var_types(then, map);
                for (_, b) in elif_ { collect_var_types(b, map); }
                if let Some(b) = else_ { collect_var_types(b, map); }
            }
            Stmt::Match { some_body, none_body, .. } => {
                collect_var_types(some_body, map);
                collect_var_types(none_body, map);
            }
            Stmt::MatchResult { ok_body, err_body, .. } => {
                collect_var_types(ok_body, map);
                collect_var_types(err_body, map);
            }
            Stmt::MatchEnum { arms, .. } => {
                for (_, body) in arms {
                    collect_var_types(body, map);
                }
            }
            Stmt::Defer(body)
            | Stmt::When { body, .. }
            | Stmt::WhenIs { body, .. }
            | Stmt::Otherwise { body } => collect_var_types(body, map),
            _ => {}
        }
    }
}

/// Collect variable names that are passed to builtins or to `ref` parameters.
/// These are considered "effectively mutated" for the purposes of the mut-but-never-mutated check.
fn collect_ref_used_in_stmts(
    stmts: &[Stmt],
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    out: &mut HashSet<String>,
) {
    for stmt in stmts {
        collect_ref_used_in_stmt(stmt, fn_lookup, out);
    }
}

fn collect_ref_used_in_stmt(
    stmt: &Stmt,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    out: &mut HashSet<String>,
) {
    match stmt {
        Stmt::Val { expr, .. } => collect_ref_used_in_expr(expr, fn_lookup, out),
        Stmt::Assign { target, value } => {
            collect_ref_used_in_expr(target, fn_lookup, out);
            collect_ref_used_in_expr(value, fn_lookup, out);
        }
        Stmt::Return(Some(e)) => collect_ref_used_in_expr(e, fn_lookup, out),
        Stmt::Expr(e) => collect_ref_used_in_expr(e, fn_lookup, out),
        Stmt::Pre(cs) | Stmt::Post(cs) | Stmt::GuardedPre(cs) | Stmt::GuardedPost(cs) => {
            for c in cs { collect_ref_used_in_expr(&c.expr, fn_lookup, out); }
        }
        Stmt::Assert(e, _) => collect_ref_used_in_expr(e, fn_lookup, out),
        Stmt::If { cond, then, elif_, else_, .. } => {
            collect_ref_used_in_expr(cond, fn_lookup, out);
            collect_ref_used_in_stmts(then, fn_lookup, out);
            for (ec, eb) in elif_ {
                collect_ref_used_in_expr(ec, fn_lookup, out);
                collect_ref_used_in_stmts(eb, fn_lookup, out);
            }
            if let Some(b) = else_ { collect_ref_used_in_stmts(b, fn_lookup, out); }
        }
        Stmt::For { init, cond, post, body } => {
            if let Some((_, e)) = init { collect_ref_used_in_expr(e, fn_lookup, out); }
            if let Some(e) = cond { collect_ref_used_in_expr(e, fn_lookup, out); }
            if let Some(s) = post { collect_ref_used_in_stmt(s, fn_lookup, out); }
            collect_ref_used_in_stmts(body, fn_lookup, out);
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            collect_ref_used_in_stmts(some_body, fn_lookup, out);
            collect_ref_used_in_stmts(none_body, fn_lookup, out);
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            collect_ref_used_in_stmts(ok_body, fn_lookup, out);
            collect_ref_used_in_stmts(err_body, fn_lookup, out);
        }
        Stmt::MatchEnum { expr, arms } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            for (_, body) in arms {
                collect_ref_used_in_stmts(body, fn_lookup, out);
            }
        }
        Stmt::Defer(body)
        | Stmt::When { body, .. }
        | Stmt::WhenIs { body, .. }
        | Stmt::Otherwise { body } => {
            collect_ref_used_in_stmts(body, fn_lookup, out);
        }
        _ => {}
    }
}

fn collect_ref_used_in_expr(
    expr: &Expr,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    out: &mut HashSet<String>,
) {
    match expr {
        Expr::Builtin { args, .. } => {
            for arg in args {
                if let Expr::Ident(name) = arg {
                    out.insert(name.clone());
                }
                collect_ref_used_in_expr(arg, fn_lookup, out);
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_ref_used_in_expr(callee, fn_lookup, out);
            let callee_path = expr_to_call_path(callee);
            if let Some(params) = callee_path.as_deref().and_then(|p| fn_lookup.get(p)) {
                for (i, arg) in args.iter().enumerate() {
                    if let Some((_, param_ty)) = params.get(i) {
                        if is_ref_type(param_ty) {
                            if let Expr::Ident(name) = arg {
                                out.insert(name.clone());
                            }
                        }
                    }
                    collect_ref_used_in_expr(arg, fn_lookup, out);
                }
            } else {
                for arg in args { collect_ref_used_in_expr(arg, fn_lookup, out); }
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_ref_used_in_expr(lhs, fn_lookup, out);
            collect_ref_used_in_expr(rhs, fn_lookup, out);
        }
        Expr::UnOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Some(expr)
        | Expr::OkVal(expr)
        | Expr::ErrVal(expr)
        | Expr::Try(expr)
        | Expr::Trust(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => collect_ref_used_in_expr(expr, fn_lookup, out),
        Expr::Field(base, _) => collect_ref_used_in_expr(base, fn_lookup, out),
        Expr::StructLit { fields } => {
            for (_, e) in fields { collect_ref_used_in_expr(e, fn_lookup, out); }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs { collect_ref_used_in_expr(e, fn_lookup, out); }
        }
        _ => {}
    }
}

/// Build a flat map of variable name -> is_mutable from `val` declarations in a body.
/// For nested scopes (if/for/etc.) we collect all declarations conservatively.
fn collect_var_mutability(stmts: &[Stmt], map: &mut HashMap<String, bool>) {
    for stmt in stmts {
        match stmt {
            Stmt::Val { name, mutable, .. } => {
                map.insert(name.clone(), *mutable);
            }
            Stmt::For { init, body, .. } => {
                if let Some((var, _)) = init {
                    map.insert(var.clone(), true);
                }
                collect_var_mutability(body, map);
            }
            Stmt::If { then, elif_, else_, .. } => {
                collect_var_mutability(then, map);
                for (_, b) in elif_ { collect_var_mutability(b, map); }
                if let Some(b) = else_ { collect_var_mutability(b, map); }
            }
            Stmt::Match { some_binding, some_body, none_body, .. } => {
                map.insert(some_binding.clone(), false);
                collect_var_mutability(some_body, map);
                collect_var_mutability(none_body, map);
            }
            Stmt::MatchResult { ok_binding, ok_body, err_binding, err_body, .. } => {
                map.insert(ok_binding.clone(), false);
                map.insert(err_binding.clone(), false);
                collect_var_mutability(ok_body, map);
                collect_var_mutability(err_body, map);
            }
            Stmt::MatchEnum { arms, .. } => {
                for (_, body) in arms {
                    collect_var_mutability(body, map);
                }
            }
            Stmt::Defer(body)
            | Stmt::When { body, .. }
            | Stmt::WhenIs { body, .. }
            | Stmt::Otherwise { body } => {
                collect_var_mutability(body, map);
            }
            _ => {}
        }
    }
}

/// Walk all statements and check that immutable variables are not passed to builtins
/// or to function parameters typed as `ref`.
fn check_immutable_args_in_stmts(
    stmts: &[Stmt],
    var_mut: &HashMap<String, bool>,
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        check_immutable_args_in_stmt(stmt, var_mut, var_types, fn_lookup, fn_name, filename, errors);
    }
}

fn check_immutable_args_in_stmt(
    stmt: &Stmt,
    var_mut: &HashMap<String, bool>,
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Stmt::Val { expr, .. } => {
            check_immutable_args_in_expr(expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::Assign { target, value } => {
            check_immutable_args_in_expr(target, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_expr(value, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::Return(Some(e)) => {
            check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::Expr(e) => {
            check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::Pre(cs) | Stmt::Post(cs) | Stmt::GuardedPre(cs) | Stmt::GuardedPost(cs) => {
            for c in cs {
                check_immutable_args_in_expr(&c.expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        Stmt::Assert(e, _) => {
            check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::If { cond, then, elif_, else_, .. } => {
            check_immutable_args_in_expr(cond, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_stmts(then, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            for (ec, eb) in elif_ {
                check_immutable_args_in_expr(ec, var_mut, var_types, fn_lookup, fn_name, filename, errors);
                check_immutable_args_in_stmts(eb, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
            if let Some(b) = else_ {
                check_immutable_args_in_stmts(b, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        Stmt::For { init, cond, post, body } => {
            if let Some((_, e)) = init {
                check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
            if let Some(e) = cond {
                check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
            if let Some(s) = post {
                check_immutable_args_in_stmt(s, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
            check_immutable_args_in_stmts(body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            check_immutable_args_in_expr(expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_stmts(some_body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_stmts(none_body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            check_immutable_args_in_expr(expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_stmts(ok_body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_stmts(err_body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Stmt::MatchEnum { expr, arms } => {
            check_immutable_args_in_expr(expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            for (_, body) in arms {
                check_immutable_args_in_stmts(body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        Stmt::Defer(body)
        | Stmt::When { body, .. }
        | Stmt::WhenIs { body, .. }
        | Stmt::Otherwise { body } => {
            check_immutable_args_in_stmts(body, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        _ => {}
    }
}

fn check_immutable_args_in_expr(
    expr: &Expr,
    var_mut: &HashMap<String, bool>,
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expr::Builtin { name, args } => {
            for arg in args {
                // If the variable is already a ref/pointer type, passing it to a builtin
                // is just passing a pointer value — the binding itself isn't mutated.
                if let Some(var_name) = immutable_non_ref_ident(arg, var_mut, var_types) {
                    errors.push(format!(
                        "{filename}: in function '{fn_name}': \
                         immutable variable '{var_name}' cannot be passed to builtin '@{name}'; \
                         declare it as 'val {var_name}: mut <type>' to allow this"
                    ));
                }
                check_immutable_args_in_expr(arg, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        Expr::Call { callee, args, .. } => {
            check_immutable_args_in_expr(callee, var_mut, var_types, fn_lookup, fn_name, filename, errors);

            let callee_path = expr_to_call_path(callee);
            if let Some(params) = callee_path.as_deref().and_then(|p| fn_lookup.get(p)) {
                for (i, arg) in args.iter().enumerate() {
                    if let Some((_, param_ty)) = params.get(i) {
                        if is_ref_type(param_ty) {
                            // Only flag if the argument's own type is NOT already a ref/pointer.
                            // Passing a ref void to a ref void param is just forwarding a pointer.
                            if let Some(var_name) = immutable_non_ref_ident(arg, var_mut, var_types) {
                                errors.push(format!(
                                    "{filename}: in function '{fn_name}': \
                                     immutable variable '{var_name}' cannot be passed to a 'ref' parameter; \
                                     declare it as 'val {var_name}: mut <type>' to allow this"
                                ));
                            }
                        }
                    }
                    check_immutable_args_in_expr(arg, var_mut, var_types, fn_lookup, fn_name, filename, errors);
                }
            } else {
                for arg in args {
                    check_immutable_args_in_expr(arg, var_mut, var_types, fn_lookup, fn_name, filename, errors);
                }
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            check_immutable_args_in_expr(lhs, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            check_immutable_args_in_expr(rhs, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Expr::UnOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Some(expr)
        | Expr::OkVal(expr)
        | Expr::ErrVal(expr)
        | Expr::Try(expr)
        | Expr::Trust(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => {
            check_immutable_args_in_expr(expr, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Expr::Field(base, _) => {
            check_immutable_args_in_expr(base, var_mut, var_types, fn_lookup, fn_name, filename, errors);
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields {
                check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                check_immutable_args_in_expr(e, var_mut, var_types, fn_lookup, fn_name, filename, errors);
            }
        }
        _ => {}
    }
}

/// If `expr` is an immutable `Ident` whose declared type is NOT a ref/pointer, return its name.
/// Variables that are already ref/pointer types can be freely passed to ref parameters or builtins
/// because doing so passes the pointer value — it doesn't mutate the binding itself.
fn immutable_non_ref_ident<'a>(
    expr: &'a Expr,
    var_mut: &HashMap<String, bool>,
    var_types: &HashMap<String, TypeExpr>,
) -> Option<&'a str> {
    if let Expr::Ident(name) = expr {
        if var_mut.get(name.as_str()) == Some(&false) {
            // If we have no type info, assume it could be a pointer — don't flag it.
            // If we do have type info and it's a pointer/ref type, also don't flag it.
            let is_ptr = var_types.get(name.as_str())
                .map_or(true, is_pointer_type);
            if !is_ptr {
                return Some(name.as_str());
            }
        }
    }
    None
}

/// Returns true if the type is a reference, slice, or untyped pointer — i.e. already a pointer value.
fn is_pointer_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Ref(_) | TypeExpr::Slice(_) | TypeExpr::Untyped)
        || matches!(ty, TypeExpr::Named(n) if n == "ptr" || n == "rawptr")
}

/// Returns true if the type is a `ref`.
fn is_ref_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Ref(_))
}

/// Convert a callee expression to a dotted path string for fn_lookup resolution.
/// e.g. `Ident("foo")` -> `"foo"`, `Field(Ident("mod"), "bar")` -> `"mod.bar"`
fn expr_to_call_path(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::Field(base, field) => {
            expr_to_call_path(base).map(|base_path| format!("{base_path}.{field}"))
        }
        _ => None,
    }
}
