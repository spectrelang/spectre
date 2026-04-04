use crate::module::ResolvedModule;
use crate::parser::{Expr, Field, FnDef, Item, Stmt, TypeExpr};
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
    let fn_ret_lookup = build_fn_ret_lookup(resolved);
    let type_lookup = build_type_lookup(resolved);
    let union_lookup = build_union_lookup(resolved);

    for item in &resolved.ast.items {
        if let Item::Fn(f) = item {
            check_fn(
                f,
                &resolved.filename,
                &fn_lookup,
                &fn_ret_lookup,
                &type_lookup,
                &union_lookup,
                errors,
            );
        }
    }
}

/// Build a map from qualified function path (e.g. "foo", "mod.foo") to its parameter list.
/// This is used to check whether a call passes an immutable value to a `ref` parameter.
fn build_fn_lookup<'a>(
    resolved: &'a ResolvedModule,
) -> HashMap<String, &'a Vec<(String, TypeExpr)>> {
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

/// Build a map from qualified function path to its return type.
fn build_fn_ret_lookup<'a>(resolved: &'a ResolvedModule) -> HashMap<String, &'a TypeExpr> {
    let mut map = HashMap::new();
    for item in &resolved.ast.items {
        if let Item::Fn(f) = item {
            let key = match &f.namespace {
                Some(ns) => format!("{ns}.{}", f.name),
                None => f.name.clone(),
            };
            map.insert(key, &f.ret);
        }
    }
    for (mod_name, child) in &resolved.imports {
        for item in &child.ast.items {
            if let Item::Fn(f) = item {
                let key = match &f.namespace {
                    Some(ns) => format!("{mod_name}.{ns}.{}", f.name),
                    None => format!("{mod_name}.{}", f.name),
                };
                map.insert(key, &f.ret);
            }
        }
    }
    map
}

/// Build a map from type name (e.g. "StringBuilder", "String") to its field definitions.
fn build_type_lookup<'a>(resolved: &'a ResolvedModule) -> HashMap<String, &'a Vec<Field>> {
    let mut map = HashMap::new();
    for item in &resolved.ast.items {
        if let Item::TypeDef { name, fields, .. } = item {
            map.insert(name.clone(), fields);
        }
        if let Item::ExternTypeDef { name, fields, .. } = item {
            map.insert(name.clone(), fields);
        }
    }
    for child in resolved.imports.values() {
        for item in &child.ast.items {
            if let Item::TypeDef { name, fields, .. } = item {
                map.insert(name.clone(), fields);
            }
            if let Item::ExternTypeDef { name, fields, .. } = item {
                map.insert(name.clone(), fields);
            }
        }
    }
    map
}

/// Build a map from union type name to its variant types.
fn build_union_lookup<'a>(resolved: &'a ResolvedModule) -> HashMap<String, &'a Vec<TypeExpr>> {
    let mut map = HashMap::new();
    for item in &resolved.ast.items {
        if let Item::UnionDef { name, variants, .. } = item {
            map.insert(name.clone(), variants);
        }
    }
    for child in resolved.imports.values() {
        for item in &child.ast.items {
            if let Item::UnionDef { name, variants, .. } = item {
                map.insert(name.clone(), variants);
            }
        }
    }
    map
}

fn check_fn(
    f: &FnDef,
    filename: &str,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    errors: &mut Vec<String>,
) {
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
        if !for_vars.contains(name)
            && *is_mutable
            && !mutated.contains(name)
            && !ref_used.contains(name)
        {
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

    check_immutable_args_in_stmts(
        &f.body,
        &var_mutability,
        &var_types,
        fn_lookup,
        &f.name,
        filename,
        errors,
    );
    check_call_arg_types(
        &f.body,
        &var_types,
        fn_lookup,
        fn_ret_lookup,
        type_lookup,
        union_lookup,
        &f.name,
        filename,
        errors,
    );
    check_type_annotations(
        &f.body,
        &var_types,
        fn_ret_lookup,
        type_lookup,
        union_lookup,
        &f.name,
        filename,
        &f.ret,
        errors,
    );

    if !matches!(f.ret, TypeExpr::Void) {
        check_return_paths(&f.body, &f.name, &f.ret, filename, errors);
    }
}

/// Check that all control flow paths in a function body end with a `return` statement
/// (or `match`/`if` where all branches return).
fn check_return_paths(
    stmts: &[Stmt],
    fn_name: &str,
    ret_type: &TypeExpr,
    filename: &str,
    errors: &mut Vec<String>,
) {
    if !all_paths_return(stmts) {
        let ret_str = type_to_string(ret_type);
        errors.push(format!(
            "{filename}: in function '{fn_name}': not all code paths return a value (expected '{ret_str}')"
        ));
    }
}

/// Returns true if all control flow paths through these statements end with a return
/// (or a construct where all branches return, like a complete if/else or match).
fn all_paths_return(stmts: &[Stmt]) -> bool {
    let mut i = 0;
    let len = stmts.len();

    while i < len {
        let stmt = &stmts[i];

        if let Stmt::Return(_) = stmt {
            return true;
        }

        if let Stmt::If {
            then, elif_, else_, ..
        } = stmt
        {
            let then_returns = all_paths_return(then);
            let all_elif_return = elif_.iter().all(|(_, body)| all_paths_return(body));
            let else_returns = else_.as_ref().map_or(false, |b| all_paths_return(b));

            if then_returns && all_elif_return && else_returns {
                return true;
            }
        }

        if let Stmt::Match {
            some_body,
            none_body,
            ..
        } = stmt
        {
            if all_paths_return(some_body) && all_paths_return(none_body) {
                return true;
            }
        }

        if let Stmt::MatchResult {
            ok_body, err_body, ..
        } = stmt
        {
            if all_paths_return(ok_body) && all_paths_return(err_body) {
                return true;
            }
        }

        if let Stmt::MatchEnum { arms, .. } = stmt {
            if arms.iter().all(|(_, body)| all_paths_return(body)) {
                return true;
            }
        }

        if let Stmt::MatchUnion {
            arms, else_body, ..
        } = stmt
        {
            let arms_return = arms.iter().all(|(_, body)| all_paths_return(body));
            let else_return = else_body.as_ref().map_or(false, |b| all_paths_return(b));
            if arms_return && else_return {
                return true;
            }
        }

        if let Stmt::MatchString {
            arms, else_body, ..
        } = stmt
        {
            let arms_return = arms.iter().all(|(_, body)| all_paths_return(body));
            let else_return = else_body.as_ref().map_or(false, |b| all_paths_return(b));
            if arms_return && else_return {
                return true;
            }
        }

        i += 1;
    }

    false
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
        Stmt::ForIn { binding, body, .. } => {
            scope_stack.push(HashSet::new());
            let shadowed = scope_stack
                .iter()
                .rev()
                .skip(1)
                .any(|s| s.contains(binding));
            if shadowed {
                errors.push(format!(
                    "{filename}: in function '{fn_name}': '{binding}' shadows an existing declaration"
                ));
            }
            if let Some(top) = scope_stack.last_mut() {
                top.insert(binding.clone());
            }
            check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
            scope_stack.pop();
        }
        Stmt::If {
            then, elif_, else_, ..
        } => {
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
        Stmt::Match {
            some_binding,
            some_body,
            none_body,
            ..
        } => {
            scope_stack.push(HashSet::new());
            let shadowed = scope_stack
                .iter()
                .rev()
                .skip(1)
                .any(|s| s.contains(some_binding));
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
        Stmt::MatchResult {
            ok_binding,
            ok_body,
            err_binding,
            err_body,
            ..
        } => {
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
        Stmt::MatchUnion {
            arms, else_body, ..
        } => {
            for (_, body) in arms {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
            if let Some(body) = else_body {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
        }
        Stmt::MatchString {
            arms, else_body, ..
        } => {
            for (_, body) in arms {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
            if let Some(body) = else_body {
                scope_stack.push(HashSet::new());
                check_shadowing_in_stmts(body, scope_stack, fn_name, filename, errors);
                scope_stack.pop();
            }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
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
            Stmt::ForIn { binding, body, .. } => {
                declared.insert(binding.clone(), false);
                collect_declarations(body, declared, for_vars);
            }
            Stmt::If {
                then, elif_, else_, ..
            } => {
                collect_declarations(then, declared, for_vars);
                for (_, b) in elif_ {
                    collect_declarations(b, declared, for_vars);
                }
                if let Some(b) = else_ {
                    collect_declarations(b, declared, for_vars);
                }
            }
            Stmt::Match {
                some_body,
                none_body,
                some_binding,
                ..
            } => {
                declared.insert(some_binding.clone(), false);
                collect_declarations(some_body, declared, for_vars);
                collect_declarations(none_body, declared, for_vars);
            }
            Stmt::MatchResult {
                ok_binding,
                ok_body,
                err_binding,
                err_body,
                ..
            } => {
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
            Stmt::MatchUnion {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_declarations(body, declared, for_vars);
                }
                if let Some(body) = else_body {
                    collect_declarations(body, declared, for_vars);
                }
            }
            Stmt::MatchString {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_declarations(body, declared, for_vars);
                }
                if let Some(body) = else_body {
                    collect_declarations(body, declared, for_vars);
                }
            }
            Stmt::Defer(body) | Stmt::When { body, .. } => {
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
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
            ..
        } => {
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
        Stmt::For {
            init,
            cond,
            post,
            body,
        } => {
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
        Stmt::ForIn {
            binding,
            iterable,
            body,
        } => {
            collect_used_in_expr(iterable, used);
            used.insert(binding.clone());
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
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            collect_used_in_stmts(body, used);
        }
        Stmt::Match {
            expr,
            some_binding,
            some_body,
            none_body,
        } => {
            collect_used_in_expr(expr, used);
            used.insert(some_binding.clone());
            collect_used_in_stmts(some_body, used);
            collect_used_in_stmts(none_body, used);
        }
        Stmt::MatchResult {
            expr,
            ok_binding,
            ok_body,
            err_binding,
            err_body,
        } => {
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
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => {
            collect_used_in_expr(expr, used);
            for (_, body) in arms {
                collect_used_in_stmts(body, used);
            }
            if let Some(body) = else_body {
                collect_used_in_stmts(body, used);
            }
        }
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => {
            collect_used_in_expr(expr, used);
            for (_, body) in arms {
                collect_used_in_stmts(body, used);
            }
            if let Some(body) = else_body {
                collect_used_in_stmts(body, used);
            }
        }
        Stmt::Break => {}
        Stmt::Continue => {}
    }
}

fn collect_used_in_assign_target(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(_) => {}
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
        Expr::ListLit(elems) => {
            for e in elems {
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
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
            ..
        } => {
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
        Stmt::For {
            init,
            cond,
            body,
            post,
        } => {
            if let Some((_, e)) = init {
                collect_addr_deref_in_expr(e, mutated);
            }
            if let Some(e) = cond {
                collect_addr_deref_in_expr(e, mutated);
            }
            if let Some(s) = post {
                collect_mutated_in_stmt(s, mutated);
            }
            collect_mutated_in_stmts(body, mutated);
        }
        Stmt::ForIn { iterable, body, .. } => {
            collect_addr_deref_in_expr(iterable, mutated);
            collect_mutated_in_stmts(body, mutated);
        }
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => {
            collect_addr_deref_in_expr(expr, mutated);
            collect_mutated_in_stmts(some_body, mutated);
            collect_mutated_in_stmts(none_body, mutated);
        }
        Stmt::MatchResult {
            expr,
            ok_body,
            err_body,
            ..
        } => {
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
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => {
            collect_addr_deref_in_expr(expr, mutated);
            for (_, body) in arms {
                collect_mutated_in_stmts(body, mutated);
            }
            if let Some(body) = else_body {
                collect_mutated_in_stmts(body, mutated);
            }
        }
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => {
            collect_addr_deref_in_expr(expr, mutated);
            for (_, body) in arms {
                collect_mutated_in_stmts(body, mutated);
            }
            if let Some(body) = else_body {
                collect_mutated_in_stmts(body, mutated);
            }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
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
            for a in args {
                collect_addr_deref_in_expr(a, mutated);
            }
        }
        Expr::Builtin { args, .. } => {
            for a in args {
                collect_addr_deref_in_expr(a, mutated);
            }
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
            for (_, e) in fields {
                collect_addr_deref_in_expr(e, mutated);
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                collect_addr_deref_in_expr(e, mutated);
            }
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
            Stmt::Val {
                name, ty: Some(ty), ..
            } => {
                map.insert(name.clone(), ty.clone());
            }
            Stmt::For { body, .. } => collect_var_types(body, map),
            Stmt::ForIn { body, .. } => collect_var_types(body, map),
            Stmt::If {
                then, elif_, else_, ..
            } => {
                collect_var_types(then, map);
                for (_, b) in elif_ {
                    collect_var_types(b, map);
                }
                if let Some(b) = else_ {
                    collect_var_types(b, map);
                }
            }
            Stmt::Match {
                some_body,
                none_body,
                ..
            } => {
                collect_var_types(some_body, map);
                collect_var_types(none_body, map);
            }
            Stmt::MatchResult {
                ok_body, err_body, ..
            } => {
                collect_var_types(ok_body, map);
                collect_var_types(err_body, map);
            }
            Stmt::MatchEnum { arms, .. } => {
                for (_, body) in arms {
                    collect_var_types(body, map);
                }
            }
            Stmt::MatchUnion {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_var_types(body, map);
                }
                if let Some(body) = else_body {
                    collect_var_types(body, map);
                }
            }
            Stmt::MatchString {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_var_types(body, map);
                }
                if let Some(body) = else_body {
                    collect_var_types(body, map);
                }
            }
            Stmt::Defer(body) | Stmt::When { body, .. } => collect_var_types(body, map),
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
            for c in cs {
                collect_ref_used_in_expr(&c.expr, fn_lookup, out);
            }
        }
        Stmt::Assert(e, _) => collect_ref_used_in_expr(e, fn_lookup, out),
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
            ..
        } => {
            collect_ref_used_in_expr(cond, fn_lookup, out);
            collect_ref_used_in_stmts(then, fn_lookup, out);
            for (ec, eb) in elif_ {
                collect_ref_used_in_expr(ec, fn_lookup, out);
                collect_ref_used_in_stmts(eb, fn_lookup, out);
            }
            if let Some(b) = else_ {
                collect_ref_used_in_stmts(b, fn_lookup, out);
            }
        }
        Stmt::For {
            init,
            cond,
            post,
            body,
        } => {
            if let Some((_, e)) = init {
                collect_ref_used_in_expr(e, fn_lookup, out);
            }
            if let Some(e) = cond {
                collect_ref_used_in_expr(e, fn_lookup, out);
            }
            if let Some(s) = post {
                collect_ref_used_in_stmt(s, fn_lookup, out);
            }
            collect_ref_used_in_stmts(body, fn_lookup, out);
        }
        Stmt::ForIn { iterable, body, .. } => {
            collect_ref_used_in_expr(iterable, fn_lookup, out);
            collect_ref_used_in_stmts(body, fn_lookup, out);
        }
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            collect_ref_used_in_stmts(some_body, fn_lookup, out);
            collect_ref_used_in_stmts(none_body, fn_lookup, out);
        }
        Stmt::MatchResult {
            expr,
            ok_body,
            err_body,
            ..
        } => {
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
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            for (_, body) in arms {
                collect_ref_used_in_stmts(body, fn_lookup, out);
            }
            if let Some(body) = else_body {
                collect_ref_used_in_stmts(body, fn_lookup, out);
            }
        }
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => {
            collect_ref_used_in_expr(expr, fn_lookup, out);
            for (_, body) in arms {
                collect_ref_used_in_stmts(body, fn_lookup, out);
            }
            if let Some(body) = else_body {
                collect_ref_used_in_stmts(body, fn_lookup, out);
            }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
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
                for arg in args {
                    collect_ref_used_in_expr(arg, fn_lookup, out);
                }
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
            for (_, e) in fields {
                collect_ref_used_in_expr(e, fn_lookup, out);
            }
        }
        Expr::ListLit(elems) => {
            for e in elems {
                collect_ref_used_in_expr(e, fn_lookup, out);
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                collect_ref_used_in_expr(e, fn_lookup, out);
            }
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
            Stmt::If {
                then, elif_, else_, ..
            } => {
                collect_var_mutability(then, map);
                for (_, b) in elif_ {
                    collect_var_mutability(b, map);
                }
                if let Some(b) = else_ {
                    collect_var_mutability(b, map);
                }
            }
            Stmt::Match {
                some_binding,
                some_body,
                none_body,
                ..
            } => {
                map.insert(some_binding.clone(), false);
                collect_var_mutability(some_body, map);
                collect_var_mutability(none_body, map);
            }
            Stmt::MatchResult {
                ok_binding,
                ok_body,
                err_binding,
                err_body,
                ..
            } => {
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
            Stmt::MatchUnion {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_var_mutability(body, map);
                }
                if let Some(body) = else_body {
                    collect_var_mutability(body, map);
                }
            }
            Stmt::MatchString {
                arms, else_body, ..
            } => {
                for (_, body) in arms {
                    collect_var_mutability(body, map);
                }
                if let Some(body) = else_body {
                    collect_var_mutability(body, map);
                }
            }
            Stmt::Defer(body) | Stmt::When { body, .. } => {
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
        check_immutable_args_in_stmt(
            stmt, var_mut, var_types, fn_lookup, fn_name, filename, errors,
        );
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
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::Assign { target, value } => {
            check_immutable_args_in_expr(
                target, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_expr(
                value, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::Return(Some(e)) => {
            check_immutable_args_in_expr(
                e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::Expr(e) => {
            check_immutable_args_in_expr(
                e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::Pre(cs) | Stmt::Post(cs) | Stmt::GuardedPre(cs) | Stmt::GuardedPost(cs) => {
            for c in cs {
                check_immutable_args_in_expr(
                    &c.expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Stmt::Assert(e, _) => {
            check_immutable_args_in_expr(
                e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
            ..
        } => {
            check_immutable_args_in_expr(
                cond, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_stmts(
                then, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            for (ec, eb) in elif_ {
                check_immutable_args_in_expr(
                    ec, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
                check_immutable_args_in_stmts(
                    eb, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            if let Some(b) = else_ {
                check_immutable_args_in_stmts(
                    b, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Stmt::For {
            init,
            cond,
            post,
            body,
        } => {
            if let Some((_, e)) = init {
                check_immutable_args_in_expr(
                    e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            if let Some(e) = cond {
                check_immutable_args_in_expr(
                    e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            if let Some(s) = post {
                check_immutable_args_in_stmt(
                    s, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            check_immutable_args_in_stmts(
                body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => {
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_stmts(
                some_body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_stmts(
                none_body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::MatchResult {
            expr,
            ok_body,
            err_body,
            ..
        } => {
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_stmts(
                ok_body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_stmts(
                err_body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Stmt::MatchEnum { expr, arms } => {
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            for (_, body) in arms {
                check_immutable_args_in_stmts(
                    body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => {
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            for (_, body) in arms {
                check_immutable_args_in_stmts(
                    body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            if let Some(body) = else_body {
                check_immutable_args_in_stmts(
                    body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => {
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            for (_, body) in arms {
                check_immutable_args_in_stmts(
                    body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
            if let Some(body) = else_body {
                check_immutable_args_in_stmts(
                    body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            check_immutable_args_in_stmts(
                body, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
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
                if let Some(var_name) = immutable_non_ref_ident(arg, var_mut, var_types) {
                    errors.push(format!(
                        "{filename}: in function '{fn_name}': \
                         immutable variable '{var_name}' cannot be passed to builtin '@{name}'; \
                         declare it as 'val {var_name}: mut <type>' to allow this"
                    ));
                }
                check_immutable_args_in_expr(
                    arg, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Expr::Call { callee, args, .. } => {
            check_immutable_args_in_expr(
                callee, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );

            let callee_path = expr_to_call_path(callee);
            if let Some(params) = callee_path.as_deref().and_then(|p| fn_lookup.get(p)) {
                for (i, arg) in args.iter().enumerate() {
                    if let Some((_, param_ty)) = params.get(i) {
                        if is_ref_type(param_ty) {
                            // Only flag if the argument's own type is NOT already a ref/pointer.
                            // Passing a ref void to a ref void param is just forwarding a pointer.
                            if let Some(var_name) = immutable_non_ref_ident(arg, var_mut, var_types)
                            {
                                errors.push(format!(
                                    "{filename}: in function '{fn_name}': \
                                     immutable variable '{var_name}' cannot be passed to a 'ref' parameter; \
                                     declare it as 'val {var_name}: mut <type>' to allow this"
                                ));
                            }
                        }
                    }
                    check_immutable_args_in_expr(
                        arg, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                    );
                }
            } else {
                for arg in args {
                    check_immutable_args_in_expr(
                        arg, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                    );
                }
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            check_immutable_args_in_expr(
                lhs, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
            check_immutable_args_in_expr(
                rhs, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
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
            check_immutable_args_in_expr(
                expr, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Expr::Field(base, _) => {
            check_immutable_args_in_expr(
                base, var_mut, var_types, fn_lookup, fn_name, filename, errors,
            );
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields {
                check_immutable_args_in_expr(
                    e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                check_immutable_args_in_expr(
                    e, var_mut, var_types, fn_lookup, fn_name, filename, errors,
                );
            }
        }
        _ => {}
    }
}

/// If `expr` is an immutable `Ident` whose declared type is NOT a ref/pointer, return its name.
fn immutable_non_ref_ident<'a>(
    expr: &'a Expr,
    var_mut: &HashMap<String, bool>,
    var_types: &HashMap<String, TypeExpr>,
) -> Option<&'a str> {
    if let Expr::Ident(name) = expr {
        if var_mut.get(name.as_str()) == Some(&false) {
            let is_ptr = var_types.get(name.as_str()).map_or(true, is_pointer_type);
            if !is_ptr {
                return Some(name.as_str());
            }
        }
    }
    None
}

/// Returns true if the type is a reference, slice, or untyped pointer — i.e. already a pointer value.
fn is_pointer_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Ref(_) | TypeExpr::Slice(_) | TypeExpr::Untyped
    ) || matches!(ty, TypeExpr::Named(n) if n == "ptr" || n == "rawptr")
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

/// Check that function call argument types match parameter types.
fn check_call_arg_types(
    stmts: &[Stmt],
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        check_call_arg_types_in_stmt(
            stmt,
            var_types,
            fn_lookup,
            fn_ret_lookup,
            type_lookup,
            union_lookup,
            fn_name,
            filename,
            errors,
        );
    }
}

fn check_call_arg_types_in_stmt(
    stmt: &Stmt,
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Stmt::Val { expr, .. } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::Assign { target, value } => {
            check_call_arg_types_in_expr(
                target,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types_in_expr(
                value,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::Return(Some(e)) => {
            check_call_arg_types_in_expr(
                e,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::Expr(e) => {
            check_call_arg_types_in_expr(
                e,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
            ..
        } => {
            check_call_arg_types_in_expr(
                cond,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                then,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            for (ec, eb) in elif_ {
                check_call_arg_types_in_expr(
                    ec,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                check_call_arg_types(
                    eb,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            if let Some(b) = else_ {
                check_call_arg_types(
                    b,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Stmt::For {
            init,
            cond,
            post,
            body,
        } => {
            if let Some((_, e)) = init {
                check_call_arg_types_in_expr(
                    e,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            if let Some(e) = cond {
                check_call_arg_types_in_expr(
                    e,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            if let Some(s) = post {
                check_call_arg_types_in_stmt(
                    s,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            check_call_arg_types(
                body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::ForIn { iterable, body, .. } => {
            check_call_arg_types_in_expr(
                iterable,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                some_body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                none_body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::MatchResult {
            expr,
            ok_body,
            err_body,
            ..
        } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                ok_body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types(
                err_body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Stmt::MatchEnum { expr, arms } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            for (_, body) in arms {
                check_call_arg_types(
                    body,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            for (_, body) in arms {
                check_call_arg_types(
                    body,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            if let Some(body) = else_body {
                check_call_arg_types(
                    body,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => {
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            for (_, body) in arms {
                check_call_arg_types(
                    body,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            if let Some(body) = else_body {
                check_call_arg_types(
                    body,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            check_call_arg_types(
                body,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        _ => {}
    }
}

fn check_call_arg_types_in_expr(
    expr: &Expr,
    var_types: &HashMap<String, TypeExpr>,
    fn_lookup: &HashMap<String, &Vec<(String, TypeExpr)>>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expr::Call { callee, args, .. } => {
            for arg in args {
                check_call_arg_types_in_expr(
                    arg,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }

            let callee_path = expr_to_call_path(callee);
            if let Some(fn_path) = callee_path {
                if let Some(param_types) = fn_lookup.get(&fn_path) {
                    for (i, arg) in args.iter().enumerate() {
                        if i < param_types.len() {
                            let (_, expected_type) = &param_types[i];
                            if let Some(actual_type) =
                                infer_expr_type(arg, var_types, type_lookup, fn_ret_lookup)
                            {
                                if !types_compatible(expected_type, &actual_type, union_lookup) {
                                    let arg_desc = match arg {
                                        Expr::Ident(n) => format!("'{n}'"),
                                        Expr::IntLit(_) => "integer literal".to_string(),
                                        Expr::FloatLit(_) => "float literal".to_string(),
                                        Expr::StrLit(_) => "string literal".to_string(),
                                        Expr::Bool(_) => "boolean literal".to_string(),
                                        _ => "expression".to_string(),
                                    };
                                    errors.push(format!(
                                        "{filename}: in function '{fn_name}': \
                                         cannot pass {arg_desc} of type '{}' to parameter of type '{}'",
                                        type_to_string(&actual_type),
                                        type_to_string(expected_type)
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            check_call_arg_types_in_expr(
                callee,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::Builtin { args, .. } => {
            for arg in args {
                check_call_arg_types_in_expr(
                    arg,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            check_call_arg_types_in_expr(
                lhs,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_call_arg_types_in_expr(
                rhs,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
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
            check_call_arg_types_in_expr(
                expr,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::Field(base, _) => {
            check_call_arg_types_in_expr(
                base,
                var_types,
                fn_lookup,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields {
                check_call_arg_types_in_expr(
                    e,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::ListLit(elems) => {
            for e in elems {
                check_call_arg_types_in_expr(
                    e,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs {
                check_call_arg_types_in_expr(
                    e,
                    var_types,
                    fn_lookup,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        _ => {}
    }
}

/// Infer the type of an expression based on variable types and literal kinds.
fn infer_expr_type(
    expr: &Expr,
    var_types: &HashMap<String, TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
) -> Option<TypeExpr> {
    match expr {
        Expr::Ident(name) => var_types.get(name).cloned(),
        Expr::IntLit(_) => Some(TypeExpr::Named("i32".to_string())),
        Expr::FloatLit(_) => Some(TypeExpr::Named("f64".to_string())),
        Expr::StrLit(_) => Some(TypeExpr::Ref(Box::new(TypeExpr::Named("char".to_string())))),
        Expr::Bool(_) => Some(TypeExpr::Named("bool".to_string())),
        Expr::None => Some(TypeExpr::Option(Box::new(TypeExpr::Untyped))),
        Expr::Some(inner) => infer_expr_type(inner, var_types, type_lookup, fn_ret_lookup)
            .map(|t| TypeExpr::Option(Box::new(t))),
        Expr::OkVal(inner) | Expr::ErrVal(inner) => {
            infer_expr_type(inner, var_types, type_lookup, fn_ret_lookup)
        }
        Expr::Cast { ty, .. } => Some(ty.clone()),
        Expr::ZeroInit(ty) => Some(TypeExpr::Named(ty.clone())),
        Expr::BinOp { op, lhs, .. } => {
            use crate::parser::BinOp;
            match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    Some(TypeExpr::Named("bool".to_string()))
                }
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Rem
                | BinOp::And
                | BinOp::Or
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => infer_expr_type(lhs, var_types, type_lookup, fn_ret_lookup),
            }
        }
        Expr::UnOp { op, expr } => {
            use crate::parser::UnOp;
            match op {
                UnOp::Neg | UnOp::BitwiseNot => {
                    infer_expr_type(expr, var_types, type_lookup, fn_ret_lookup)
                }
                UnOp::Not => Some(TypeExpr::Named("bool".to_string())),
            }
        }
        Expr::Deref(inner) => {
            if let Expr::Ident(name) = inner.as_ref() {
                if let Some(ty) = var_types.get(name) {
                    if let TypeExpr::Ref(inner_ty) = ty {
                        return Some((**inner_ty).clone());
                    }
                }
            }
            None
        }
        Expr::Addr(inner) => {
            if let Some(ty) = infer_expr_type(inner, var_types, type_lookup, fn_ret_lookup) {
                Some(TypeExpr::Ref(Box::new(ty)))
            } else {
                None
            }
        }
        Expr::Field(base, field) => {
            if let Some(base_type) = infer_expr_type(base, var_types, type_lookup, fn_ret_lookup) {
                if let TypeExpr::Named(ref type_name) = base_type {
                    if let Some(fields) = type_lookup.get(type_name) {
                        for f in *fields {
                            if f.name == *field {
                                return Some(f.ty.clone());
                            }
                        }
                    }
                }
            }
            // Whatever: just return the base type
            infer_expr_type(base, var_types, type_lookup, fn_ret_lookup)
        }
        Expr::Call { callee, .. } => {
            let callee_path = expr_to_call_path(callee)?;
            fn_ret_lookup
                .get(&callee_path)
                .map(|ret_ty| (**ret_ty).clone())
        }
        Expr::Builtin { name, .. } => match name.as_str() {
            "alloc" | "realloc" | "ptradd" => {
                Some(TypeExpr::Ref(Box::new(TypeExpr::Named("void".to_string()))))
            }
            "load" | "load8" | "loadf" => Some(TypeExpr::Untyped),
            "store" | "store8" | "storef" | "memset" | "memcpy" | "free" => Some(TypeExpr::Void),
            _ => None,
        },
        Expr::ListLit(_) => Some(TypeExpr::List(Box::new(TypeExpr::Untyped))),
        Expr::StructLit { .. } => None,
        Expr::ArgsPack(_) => None,
        Expr::Try(inner) | Expr::Trust(inner) => {
            infer_expr_type(inner, var_types, type_lookup, fn_ret_lookup)
        }
    }
}

/// Check if two types are compatible for function call argument passing.
/// This is conservative: we only allow exact matches or very close types.
fn types_compatible(
    expected: &TypeExpr,
    actual: &TypeExpr,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
) -> bool {
    if type_eq(expected, actual) {
        return true;
    }

    match (expected, actual) {
        (TypeExpr::Named(a), TypeExpr::Named(b)) => {
            let int_types = [
                "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "isize", "usize",
            ];
            if int_types.contains(&a.as_str()) && int_types.contains(&b.as_str()) {
                return true;
            }
            if a == "ptr" || a == "rawptr" {
                if b == "ptr" || b == "rawptr" {
                    return true;
                }
            }
            if let Some(variants) = union_lookup.get(a) {
                for variant in *variants {
                    if types_compatible(variant, &TypeExpr::Named(b.clone()), union_lookup) {
                        return true;
                    }
                }
            }
            false
        }
        (TypeExpr::Ref(a), TypeExpr::Ref(b)) => types_compatible(a, b, union_lookup),
        (TypeExpr::Named(union_name), TypeExpr::Ref(_)) => {
            if let Some(variants) = union_lookup.get(union_name) {
                for variant in *variants {
                    if types_compatible(variant, actual, union_lookup) {
                        return true;
                    }
                }
            }
            false
        }
        (TypeExpr::Slice(_), TypeExpr::Slice(_)) => true,
        (TypeExpr::List(_), TypeExpr::List(_)) => true,
        (TypeExpr::Option(_), TypeExpr::Option(_)) => true,
        _ => false,
    }
}

/// Check if two TypeExpr values are exactly equal.
fn type_eq(a: &TypeExpr, b: &TypeExpr) -> bool {
    match (a, b) {
        (TypeExpr::Named(a), TypeExpr::Named(b)) => a == b,
        (TypeExpr::Slice(a), TypeExpr::Slice(b)) => type_eq(a, b),
        (TypeExpr::Ref(a), TypeExpr::Ref(b)) => type_eq(a, b),
        (TypeExpr::Option(a), TypeExpr::Option(b)) => type_eq(a, b),
        (TypeExpr::List(a), TypeExpr::List(b)) => type_eq(a, b),
        (TypeExpr::Result(a1, a2), TypeExpr::Result(b1, b2)) => type_eq(a1, b1) && type_eq(a2, b2),
        (
            TypeExpr::FnPtr {
                params: pa,
                ret: ra,
            },
            TypeExpr::FnPtr {
                params: pb,
                ret: rb,
            },
        ) => {
            pa.len() == pb.len()
                && pa.iter().zip(pb.iter()).all(|(a, b)| type_eq(a, b))
                && type_eq(ra, rb)
        }
        (TypeExpr::Void, TypeExpr::Void) => true,
        (TypeExpr::Untyped, TypeExpr::Untyped) => true,
        _ => false,
    }
}

/// Convert a TypeExpr to a readable string for error messages.
fn type_to_string(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Slice(t) => format!("slice[{}]", type_to_string(t)),
        TypeExpr::FixedArray(count, elem) => format!("[{}]{}", count, type_to_string(elem)),
        TypeExpr::Ref(t) => format!("ref {}", type_to_string(t)),
        TypeExpr::Option(t) => format!("option[{}]", type_to_string(t)),
        TypeExpr::List(t) => format!("list[{}]", type_to_string(t)),
        TypeExpr::Result(t, e) => format!("result[{}, {}]", type_to_string(t), type_to_string(e)),
        TypeExpr::FnPtr { params, ret } => {
            let param_strs: Vec<String> = params.iter().map(type_to_string).collect();
            format!("fn({}) {}", param_strs.join(", "), type_to_string(ret))
        }
        TypeExpr::Void => "void".to_string(),
        TypeExpr::Untyped => "untyped".to_string(),
    }
}

/// Check that explicit type annotations on `val` declarations match the inferred type.
/// This catches bugs like `val x: ref void = PtrList.new(8)` where new() returns `result[...]`.
fn check_type_annotations(
    stmts: &[Stmt],
    var_types: &HashMap<String, TypeExpr>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    fn_name: &str,
    filename: &str,
    fn_ret_type: &TypeExpr,
    errors: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Val {
                name,
                ty: Some(declared_ty),
                expr,
                ..
            } => {
                if let Some(inferred) = infer_expr_type(expr, var_types, type_lookup, fn_ret_lookup)
                {
                    if !types_compatible_for_annotation(declared_ty, &inferred, union_lookup) {
                        errors.push(format!(
                            "{filename}: in function '{fn_name}': type mismatch for '{name}': \
                             declared '{declared}', but expression has type '{inferred}'",
                            declared = type_to_string(declared_ty),
                            inferred = type_to_string(&inferred)
                        ));
                    }
                }
                check_type_annotations_in_expr(
                    expr,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            Stmt::Return(Some(ret_expr)) => {
                let expected_inner: Option<TypeExpr> = match ret_expr {
                    Expr::OkVal(_) => {
                        if let TypeExpr::Result(ok_ty, _) = fn_ret_type {
                            Some((**ok_ty).clone())
                        } else {
                            None
                        }
                    }
                    Expr::ErrVal(_) => {
                        if let TypeExpr::Result(_, err_ty) = fn_ret_type {
                            Some((**err_ty).clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                let inner_expr = match ret_expr {
                    Expr::OkVal(inner) | Expr::ErrVal(inner) => inner.as_ref(),
                    other => other,
                };

                if let Some(expected) = expected_inner {
                    if !matches!(expected, TypeExpr::Void) {
                        if let Some(inferred) =
                            infer_expr_type(inner_expr, var_types, type_lookup, fn_ret_lookup)
                        {
                            if !types_compatible_for_annotation(&expected, &inferred, union_lookup)
                            {
                                errors.push(format!(
                                    "{filename}: in function '{fn_name}': return type mismatch: \
                                     expected '{expected}', but expression has type '{inferred}'",
                                    expected = type_to_string(&expected),
                                    inferred = type_to_string(&inferred)
                                ));
                            }
                        }
                    }
                } else if !matches!(fn_ret_type, TypeExpr::Result(_, _)) {
                    if !matches!(ret_expr, Expr::None) {
                        if let Some(inferred) =
                            infer_expr_type(ret_expr, var_types, type_lookup, fn_ret_lookup)
                        {
                            if !types_compatible_for_annotation(
                                fn_ret_type,
                                &inferred,
                                union_lookup,
                            ) {
                                errors.push(format!(
                                    "{filename}: in function '{fn_name}': return type mismatch: \
                                     expected '{expected}', but expression has type '{inferred}'",
                                    expected = type_to_string(fn_ret_type),
                                    inferred = type_to_string(&inferred)
                                ));
                            }
                        }
                    }
                }
                check_type_annotations_in_expr(
                    ret_expr,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
            Stmt::If {
                cond,
                then,
                elif_,
                else_,
            } => {
                check_type_annotations_in_expr(
                    cond,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                for s in then {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
                for (e, b) in elif_ {
                    check_type_annotations_in_expr(
                        e,
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        errors,
                    );
                    for s in b {
                        check_type_annotations(
                            std::slice::from_ref(s),
                            var_types,
                            fn_ret_lookup,
                            type_lookup,
                            union_lookup,
                            fn_name,
                            filename,
                            fn_ret_type,
                            errors,
                        );
                    }
                }
                if let Some(else_body) = else_ {
                    for s in else_body {
                        check_type_annotations(
                            std::slice::from_ref(s),
                            var_types,
                            fn_ret_lookup,
                            type_lookup,
                            union_lookup,
                            fn_name,
                            filename,
                            fn_ret_type,
                            errors,
                        );
                    }
                }
            }
            Stmt::For {
                init: Some((_, init_expr)),
                cond: Some(cond),
                post: Some(post),
                body,
            } => {
                check_type_annotations_in_expr(
                    init_expr,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                check_type_annotations_in_expr(
                    cond,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                check_type_annotations(
                    std::slice::from_ref(post),
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    fn_ret_type,
                    errors,
                );
                for s in body {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
            }
            Stmt::For {
                init: None,
                cond: Some(cond),
                post: Some(post),
                body,
            } => {
                check_type_annotations_in_expr(
                    cond,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                check_type_annotations(
                    std::slice::from_ref(post),
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    fn_ret_type,
                    errors,
                );
                for s in body {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
            }
            Stmt::For {
                init: Some((_, init_expr)),
                cond: None,
                post: Some(post),
                body,
            } => {
                check_type_annotations_in_expr(
                    init_expr,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                check_type_annotations(
                    std::slice::from_ref(post),
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    fn_ret_type,
                    errors,
                );
                for s in body {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
            }
            Stmt::For {
                init: None,
                cond: None,
                post: Some(post),
                body,
            } => {
                check_type_annotations(
                    std::slice::from_ref(post),
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    fn_ret_type,
                    errors,
                );
                for s in body {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
            }
            Stmt::ForIn { iterable, body, .. } => {
                check_type_annotations_in_expr(
                    iterable,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
                for s in body {
                    check_type_annotations(
                        std::slice::from_ref(s),
                        var_types,
                        fn_ret_lookup,
                        type_lookup,
                        union_lookup,
                        fn_name,
                        filename,
                        fn_ret_type,
                        errors,
                    );
                }
            }
            _ => {}
        }
    }
}

fn check_type_annotations_in_expr(
    expr: &Expr,
    var_types: &HashMap<String, TypeExpr>,
    fn_ret_lookup: &HashMap<String, &TypeExpr>,
    type_lookup: &HashMap<String, &Vec<Field>>,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
    fn_name: &str,
    filename: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expr::Call { args, .. } => {
            for a in args {
                check_type_annotations_in_expr(
                    a,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::Some(inner)
        | Expr::OkVal(inner)
        | Expr::ErrVal(inner)
        | Expr::Trust(inner)
        | Expr::Try(inner)
        | Expr::Addr(inner)
        | Expr::Deref(inner)
        | Expr::Cast { expr: inner, .. } => {
            check_type_annotations_in_expr(
                inner,
                var_types,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::BinOp { lhs, rhs, .. } => {
            check_type_annotations_in_expr(
                lhs,
                var_types,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
            check_type_annotations_in_expr(
                rhs,
                var_types,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::UnOp { expr: inner, .. } => {
            check_type_annotations_in_expr(
                inner,
                var_types,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::StructLit { fields, .. } => {
            for (_, e) in fields {
                check_type_annotations_in_expr(
                    e,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::ListLit(items) => {
            for e in items {
                check_type_annotations_in_expr(
                    e,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        Expr::Field(base, _) => {
            check_type_annotations_in_expr(
                base,
                var_types,
                fn_ret_lookup,
                type_lookup,
                union_lookup,
                fn_name,
                filename,
                errors,
            );
        }
        Expr::ArgsPack(items) => {
            for e in items {
                check_type_annotations_in_expr(
                    e,
                    var_types,
                    fn_ret_lookup,
                    type_lookup,
                    union_lookup,
                    fn_name,
                    filename,
                    errors,
                );
            }
        }
        _ => {}
    }
}

/// Check if a declared type is compatible with the inferred expression type.
/// More permissive than strict equality (allows ptr/ref compatibility, int widening, etc.)
fn types_compatible_for_annotation(
    declared: &TypeExpr,
    inferred: &TypeExpr,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
) -> bool {
    if type_eq(declared, inferred) {
        return true;
    }
    if type_to_string(declared) == type_to_string(inferred) {
        return true;
    }
    match (declared, inferred) {
        (TypeExpr::Named(a), TypeExpr::Named(b)) => {
            let int_types = [
                "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "isize", "usize",
            ];
            if int_types.contains(&a.as_str()) && int_types.contains(&b.as_str()) {
                return true;
            }
            if a == "ptr" || a == "rawptr" {
                if b == "ptr" || b == "rawptr" {
                    return true;
                }
            }
            false
        }
        (TypeExpr::Ref(inner_d), TypeExpr::Ref(inner_i)) => {
            types_compatible_for_annotation(inner_d, inner_i, union_lookup)
        }
        (TypeExpr::Ref(inner), TypeExpr::Untyped) => {
            matches!(inner.as_ref(), TypeExpr::Named(n) if n == "void")
                || matches!(inner.as_ref(), TypeExpr::Void)
        }
        (TypeExpr::Named(a), TypeExpr::Ref(inner)) if a == "ptr" || a == "rawptr" => {
            types_compatible_for_annotation(&TypeExpr::Named(a.clone()), inner, union_lookup)
        }
        (TypeExpr::Named(a), TypeExpr::Ref(_)) if a == "ptr" || a == "rawptr" => true,
        (TypeExpr::Option(d), TypeExpr::Option(i)) => {
            types_compatible_for_annotation(d, i, union_lookup)
        }
        (TypeExpr::Named(name), _) => {
            if let Some(variants) = union_lookup.get(name) {
                matches_union_variant(variants, inferred, union_lookup)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn matches_union_variant(
    variants: &[TypeExpr],
    ty: &TypeExpr,
    union_lookup: &HashMap<String, &Vec<TypeExpr>>,
) -> bool {
    for v in variants {
        if type_eq(v, ty) {
            return true;
        }
        if let TypeExpr::Named(name) = v {
            if let Some(inner_variants) = union_lookup.get(name) {
                if matches_union_variant(inner_variants, ty, union_lookup) {
                    return true;
                }
            }
        }
    }
    false
}
