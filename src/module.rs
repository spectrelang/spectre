use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cli::Args;
use crate::codegen::Codegen;
use crate::lexer::Lexer;
use crate::parser::{Expr, Item, Module, Parser};
use crate::semantic;

/// A fully-resolved module: its parsed AST plus a map of imported sub-modules.
#[derive(Clone)]
pub struct ResolvedModule {
    pub ast: Module,
    pub imports: HashMap<String, ResolvedModule>,
    pub dir: PathBuf,
    pub filename: String,
    pub links: Vec<String>,
    pub warnings: Vec<String>,
}

/// Entry point: compile a .sx file to QBE IR.
pub fn compile_file(input: &str, args: &Args) -> Result<(String, Vec<String>, Vec<String>), String> {
    let path = PathBuf::from(input);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let src = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {input}: {e}"))?;
    let resolved = resolve_module(&src, &dir, &mut HashMap::new(), &mut Vec::new(), input, None)?;

    if args.emit_tokens {
        let mut lex = Lexer::new(&src);
        let tokens = lex.tokenize().map_err(|e| format!("{input}: {e}"))?;
        return Ok((format!("{tokens:#?}"), vec![], vec![]));
    }
    if args.emit_ast {
        return Ok((format!("{:#?}", resolved.ast), vec![], vec![]));
    }

    let sem_errors = semantic::check_module(&resolved);
    if !sem_errors.is_empty() {
        return Err(sem_errors.join("\n"));
    }

    let libs = collect_used_libs(&resolved);
    let mut cg = Codegen::new();

    cg.emit_module(&resolved, args.test, args.release)?;
    let mut warnings = cg.warnings.clone();
    collect_parse_warnings(&resolved, &mut warnings);
    Ok((cg.finish(), warnings, libs))
}

/// Recursively collect parser warnings from a resolved module tree.
fn collect_parse_warnings(resolved: &ResolvedModule, out: &mut Vec<String>) {
    out.extend(resolved.warnings.iter().cloned());
    for child in resolved.imports.values() {
        collect_parse_warnings(child, out);
    }
}

/// Collect link libs:
///
/// - Always include libs declared in the root module itself.
/// - For each imported child module, only include its libs (recursively) if
///   that import is actually referenced somewhere in the root's AST.
pub fn collect_used_libs(root: &ResolvedModule) -> Vec<String> {
    let mut libs: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for lib in &root.links {
        if seen.insert(lib.clone()) {
            libs.push(lib.clone());
        }
    }

    let used_imports = used_import_names(&root.ast, &root.imports);
    for name in &used_imports {
        if let Some(child) = root.imports.get(name) {
            collect_libs_recursive(child, &mut libs, &mut seen);
        }
    }

    libs
}

/// Collect all `link` libs from a module and its transitively-used imports.
fn collect_libs_recursive(
    module: &ResolvedModule,
    libs: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    for lib in &module.links {
        if seen.insert(lib.clone()) {
            libs.push(lib.clone());
        }
    }

    let used = used_import_names(&module.ast, &module.imports);
    for name in &used {
        if let Some(child) = module.imports.get(name) {
            collect_libs_recursive(child, libs, seen);
        }
    }
}

/// Return the set of import binding names that are referenced anywhere in `ast`.
fn used_import_names(ast: &Module, imports: &HashMap<String, ResolvedModule>) -> HashSet<String> {
    let mut used = HashSet::new();
    for item in &ast.items {
        collect_used_imports_in_item(item, imports, &mut used);
    }
    used
}

fn collect_used_imports_in_item(
    item: &Item,
    imports: &HashMap<String, ResolvedModule>,
    used: &mut HashSet<String>,
) {
    match item {
        Item::Fn(f) => {
            for stmt in &f.body {
                collect_used_imports_in_stmt(stmt, imports, used);
            }
        }
        Item::Test { body } => {
            for stmt in body {
                collect_used_imports_in_stmt(stmt, imports, used);
            }
        }
        Item::Const { expr, .. } => collect_used_imports_in_expr(expr, imports, used),
        _ => {}
    }
}

fn collect_used_imports_in_stmt(
    stmt: &crate::parser::Stmt,
    imports: &HashMap<String, ResolvedModule>,
    used: &mut HashSet<String>,
) {
    use crate::parser::Stmt;
    match stmt {
        Stmt::Val { expr, .. } => collect_used_imports_in_expr(expr, imports, used),
        Stmt::Assign { target, value } => {
            collect_used_imports_in_expr(target, imports, used);
            collect_used_imports_in_expr(value, imports, used);
        }
        Stmt::Return(Some(e)) => collect_used_imports_in_expr(e, imports, used),
        Stmt::Expr(e) => collect_used_imports_in_expr(e, imports, used),
        Stmt::Pre(cs) | Stmt::Post(cs) => {
            for c in cs { collect_used_imports_in_expr(&c.expr, imports, used); }
        }
        Stmt::Assert(e, _) => collect_used_imports_in_expr(e, imports, used),
        Stmt::If { cond, then, elif_, else_, .. } => {
            collect_used_imports_in_expr(cond, imports, used);
            for s in then { collect_used_imports_in_stmt(s, imports, used); }
            for (ec, eb) in elif_ {
                collect_used_imports_in_expr(ec, imports, used);
                for s in eb { collect_used_imports_in_stmt(s, imports, used); }
            }
            if let Some(b) = else_ {
                for s in b { collect_used_imports_in_stmt(s, imports, used); }
            }
        }
        Stmt::For { init, cond, post, body } => {
            if let Some((_, e)) = init { collect_used_imports_in_expr(e, imports, used); }
            if let Some(e) = cond { collect_used_imports_in_expr(e, imports, used); }
            if let Some(s) = post { collect_used_imports_in_stmt(s, imports, used); }
            for s in body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::ForIn { iterable, body, .. } => {
            collect_used_imports_in_expr(iterable, imports, used);
            for s in body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            for s in body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::MatchUnion { expr, arms, else_body } => {
            collect_used_imports_in_expr(expr, imports, used);
            for (_, body) in arms {
                for s in body { collect_used_imports_in_stmt(s, imports, used); }
            }
            if let Some(body) = else_body {
                for s in body { collect_used_imports_in_stmt(s, imports, used); }
            }
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            collect_used_imports_in_expr(expr, imports, used);
            for s in some_body { collect_used_imports_in_stmt(s, imports, used); }
            for s in none_body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            collect_used_imports_in_expr(expr, imports, used);
            for s in ok_body { collect_used_imports_in_stmt(s, imports, used); }
            for s in err_body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::MatchEnum { expr, arms } => {
            collect_used_imports_in_expr(expr, imports, used);
            for (_, body) in arms {
                for s in body { collect_used_imports_in_stmt(s, imports, used); }
            }
        }
        Stmt::MatchString { expr, arms, else_body } => {
            collect_used_imports_in_expr(expr, imports, used);
            for (_, body) in arms {
                for s in body { collect_used_imports_in_stmt(s, imports, used); }
            }
            if let Some(body) = else_body {
                for s in body { collect_used_imports_in_stmt(s, imports, used); }
            }
        }
        _ => {}
    }
}

fn collect_used_imports_in_expr(
    expr: &Expr,
    imports: &HashMap<String, ResolvedModule>,
    used: &mut HashSet<String>,
) {
    match expr {
        Expr::Ident(name) => {
            if imports.contains_key(name) {
                used.insert(name.clone());
            }
        }
        Expr::Field(base, _) => {
            if let Expr::Ident(name) = base.as_ref() {
                if imports.contains_key(name) {
                    used.insert(name.clone());
                }
            }
            collect_used_imports_in_expr(base, imports, used);
        }
        Expr::Call { callee, args, .. } => {
            collect_used_imports_in_expr(callee, imports, used);
            for a in args { collect_used_imports_in_expr(a, imports, used); }
        }
        Expr::Builtin { args, .. } => {
            for a in args { collect_used_imports_in_expr(a, imports, used); }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_used_imports_in_expr(lhs, imports, used);
            collect_used_imports_in_expr(rhs, imports, used);
        }
        Expr::UnOp { expr, .. } | Expr::Cast { expr, .. } | Expr::Some(expr) | Expr::OkVal(expr) | Expr::ErrVal(expr) | Expr::Try(expr) | Expr::Trust(expr) | Expr::Addr(expr) | Expr::Deref(expr) => {
            collect_used_imports_in_expr(expr, imports, used);
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields { collect_used_imports_in_expr(e, imports, used); }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs { collect_used_imports_in_expr(e, imports, used); }
        }
        _ => {}
    }
}

/// Recursively parse and resolve a module from source text.
/// Only imports that are actually referenced in the AST are loaded.
pub fn resolve_module(
    src: &str,
    dir: &Path,
    cache: &mut HashMap<PathBuf, ResolvedModule>,
    in_progress: &mut Vec<PathBuf>,
    filename: &str,
    needed_children: Option<&HashSet<String>>,
) -> Result<ResolvedModule, String> {
    let mut lex = Lexer::new(src);
    let tokens = lex.tokenize().map_err(|e| format!("{filename}:{e}"))?;
    let mut parser = Parser::with_filename(tokens, filename.to_string());
    let ast = parser.parse_module()?;
    let parse_warnings = parser.warnings;
    let mut imports = HashMap::new();
    let self_path = PathBuf::from(filename);
    if let Some(cycle_start) = in_progress.iter().position(|p| p == &self_path) {
        let chain: Vec<String> = in_progress[cycle_start..]
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        let cycle_str = chain.join(" -> ");
        return Err(format!(
            "cyclic import detected: {cycle_str} -> {filename}"
        ));
    }
    in_progress.push(self_path.clone());

    let declared_uses: HashMap<String, PathBuf> = ast
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Use { name, path, .. } = item {
                Some((name.clone(), resolve_use_path(path, dir)))
            } else {
                None
            }
        })
        .collect();

    let mut locally_referenced: HashSet<String> = HashSet::new();
    for name in declared_uses.keys() {
        if ast_references_name(&ast, name) {
            locally_referenced.insert(name.clone());
        }
    }

    for (name, resolved_path) in &declared_uses {
        let is_needed = if let Some(needed) = needed_children {
            needed.contains(name)
        } else {
            false
        };

        if !is_needed && !locally_referenced.contains(name) {
            continue;
        }

        let needed_grandchildren = collect_needed_subnames_transitive(
            &ast,
            name,
            &declared_uses,
            dir,
            in_progress,
        );

        if let Some(cached) = cache.get(resolved_path) {
            let missing = needed_grandchildren
                .iter()
                .any(|sub| !cached.imports.contains_key(sub));
            if !missing {
                imports.insert(name.clone(), cached.clone());
                continue;
            }
        }

        let child_src = std::fs::read_to_string(resolved_path)
            .map_err(|e| format!("cannot load module '{}': {e}", resolved_path.display()))?;
        let child_dir = resolved_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let child_filename = resolved_path.to_string_lossy().to_string();

        let child = resolve_module(
            &child_src,
            &child_dir,
            cache,
            in_progress,
            &child_filename,
            Some(&needed_grandchildren),
        )?;
        imports.insert(name.clone(), child);
    }

    in_progress.retain(|p| p != &self_path);

    let current_platform = crate::cli::Platform::current();
    let mut links: Vec<String> = Vec::new();
    for item in &ast.items {
        if let Item::LinkWhen { platform, libs } = item {
            if current_platform.matches_name(platform) {
                links.extend(libs.iter().cloned());
            }
        }
    }
    for item in &ast.items {
        if let Item::Link { lib } = item {
            links.push(lib.clone());
        }
    }

    let resolved = ResolvedModule {
        ast,
        imports,
        dir: dir.to_path_buf(),
        filename: filename.to_string(),
        links,
        warnings: parse_warnings,
    };
    cache.insert(self_path, resolved.clone());
    Ok(resolved)
}
fn ast_references_name(ast: &Module, name: &str) -> bool {
    ast.items.iter().any(|item| item_references_name(item, name))
}

fn item_references_name(item: &Item, name: &str) -> bool {
    match item {
        Item::Fn(f) => f.body.iter().any(|s| stmt_references_name(s, name)),
        Item::Test { body } => body.iter().any(|s| stmt_references_name(s, name)),
        Item::Const { expr, .. } => expr_references_name(expr, name),
        _ => false,
    }
}

fn stmt_references_name(stmt: &crate::parser::Stmt, name: &str) -> bool {
    use crate::parser::Stmt;
    match stmt {
        Stmt::Val { expr, .. } => expr_references_name(expr, name),
        Stmt::Assign { target, value } => expr_references_name(target, name) || expr_references_name(value, name),
        Stmt::Return(Some(e)) => expr_references_name(e, name),
        Stmt::Expr(e) => expr_references_name(e, name),
        Stmt::Pre(cs) | Stmt::Post(cs) => cs.iter().any(|c| expr_references_name(&c.expr, name)),
        Stmt::Assert(e, _) => expr_references_name(e, name),
        Stmt::If { cond, then, elif_, else_, .. } => {
            expr_references_name(cond, name)
                || then.iter().any(|s| stmt_references_name(s, name))
                || elif_.iter().any(|(ec, eb)| expr_references_name(ec, name) || eb.iter().any(|s| stmt_references_name(s, name)))
                || else_.as_ref().map_or(false, |b| b.iter().any(|s| stmt_references_name(s, name)))
        }
        Stmt::For { init, cond, post, body } => {
            init.as_ref().map_or(false, |(_, e)| expr_references_name(e, name))
                || cond.as_ref().map_or(false, |e| expr_references_name(e, name))
                || post.as_ref().map_or(false, |s| stmt_references_name(s, name))
                || body.iter().any(|s| stmt_references_name(s, name))
        }
        Stmt::ForIn { iterable, body, .. } => {
            expr_references_name(iterable, name)
                || body.iter().any(|s| stmt_references_name(s, name))
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            body.iter().any(|s| stmt_references_name(s, name))
        }
        Stmt::MatchUnion { expr, arms, else_body } => {
            expr_references_name(expr, name)
                || arms.iter().any(|(_, b)| b.iter().any(|s| stmt_references_name(s, name)))
                || else_body.as_ref().map_or(false, |b| b.iter().any(|s| stmt_references_name(s, name)))
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            expr_references_name(expr, name)
                || some_body.iter().any(|s| stmt_references_name(s, name))
                || none_body.iter().any(|s| stmt_references_name(s, name))
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            expr_references_name(expr, name)
                || ok_body.iter().any(|s| stmt_references_name(s, name))
                || err_body.iter().any(|s| stmt_references_name(s, name))
        }
        Stmt::MatchEnum { expr, arms } => {
            expr_references_name(expr, name)
                || arms.iter().any(|(_, b)| b.iter().any(|s| stmt_references_name(s, name)))
        }
        Stmt::MatchString { expr, arms, else_body } => {
            expr_references_name(expr, name)
                || arms.iter().any(|(_, b)| b.iter().any(|s| stmt_references_name(s, name)))
                || else_body.as_ref().map_or(false, |b| b.iter().any(|s| stmt_references_name(s, name)))
        }
        _ => false,
    }
}

fn expr_references_name(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Ident(n) => n == name,
        Expr::Field(base, _) => {
            if let Expr::Ident(n) = base.as_ref() {
                if n == name { return true; }
            }
            expr_references_name(base, name)
        }
        Expr::Call { callee, args, .. } => {
            expr_references_name(callee, name) || args.iter().any(|a| expr_references_name(a, name))
        }
        Expr::Builtin { args, .. } => args.iter().any(|a| expr_references_name(a, name)),
        Expr::BinOp { lhs, rhs, .. } => expr_references_name(lhs, name) || expr_references_name(rhs, name),
        Expr::UnOp { expr, .. } | Expr::Cast { expr, .. } | Expr::Some(expr) | Expr::OkVal(expr) | Expr::ErrVal(expr) | Expr::Try(expr) | Expr::Trust(expr) | Expr::Addr(expr) | Expr::Deref(expr) => {
            expr_references_name(expr, name)
        }
        Expr::ArgsPack(exprs) => exprs.iter().any(|e| expr_references_name(e, name)),
        _ => false,
    }
}

/// Collect the set of sub-module names accessed via `<import_name>.<sub>` in the AST.
/// e.g. for `import_name = "std"`, finds `std.io`, `std.math`, etc. and returns {"io", "math"}.
fn collect_needed_subnames(ast: &Module, import_name: &str) -> HashSet<String> {
    let mut needed = HashSet::new();
    for item in &ast.items {
        collect_needed_subnames_in_item(item, import_name, &mut needed);
    }
    needed
}

/// Like `collect_needed_subnames`, but also transitively expands through the ASTs of the
/// needed children. This handles cases where a child module (e.g. string.sx) uses a sibling
/// (e.g. std.collections) that the parent didn't directly reference.
fn collect_needed_subnames_transitive(
    ast: &Module,
    import_name: &str,
    declared_uses: &HashMap<String, PathBuf>,
    dir: &Path,
    in_progress: &[PathBuf],
) -> HashSet<String> {
    let mut needed = collect_needed_subnames(ast, import_name);
    let mut visited_for_expansion: HashSet<String> = HashSet::new();
    let mut changed = true;

    while changed {
        changed = false;
        let current: Vec<String> = needed.iter().cloned().collect();
        for sub_name in current {
            if visited_for_expansion.contains(&sub_name) {
                continue;
            }
            visited_for_expansion.insert(sub_name.clone());

            let import_path = match declared_uses.get(import_name) {
                Some(p) => p,
                None => continue,
            };
            let import_dir = import_path.parent().unwrap_or(Path::new("."));

            let import_src = match std::fs::read_to_string(import_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let sub_path = {
                let mut lex = Lexer::new(&import_src);
                let tokens = match lex.tokenize() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let mut parser = Parser::with_filename(tokens, import_path.to_string_lossy().to_string());
                let import_ast = match parser.parse_module() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                import_ast.items.iter().find_map(|item| {
                    if let Item::Use { name, path, .. } = item {
                        if name == &sub_name {
                            Some(resolve_use_path(path, import_dir))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            };

            let sub_path = match sub_path {
                Some(p) => p,
                None => continue,
            };

            if in_progress.contains(&sub_path) {
                continue;
            }

            let sub_src = match std::fs::read_to_string(&sub_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut lex = Lexer::new(&sub_src);
            let tokens = match lex.tokenize() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut parser = Parser::with_filename(tokens, sub_path.to_string_lossy().to_string());
            let sub_ast = match parser.parse_module() {
                Ok(a) => a,
                Err(_) => continue,
            };

            for extra in collect_needed_subnames(&sub_ast, import_name) {
                if needed.insert(extra) {
                    changed = true;
                }
            }
        }
    }

    needed
}

fn collect_needed_subnames_in_item(item: &Item, import_name: &str, needed: &mut HashSet<String>) {
    match item {
        Item::Fn(f) => {
            for stmt in &f.body {
                collect_needed_subnames_in_stmt(stmt, import_name, needed);
            }
        }
        Item::Test { body } => {
            for stmt in body {
                collect_needed_subnames_in_stmt(stmt, import_name, needed);
            }
        }
        Item::Const { expr, .. } => collect_needed_subnames_in_expr(expr, import_name, needed),
        _ => {}
    }
}

fn collect_needed_subnames_in_stmt(
    stmt: &crate::parser::Stmt,
    import_name: &str,
    needed: &mut HashSet<String>,
) {
    use crate::parser::Stmt;
    match stmt {
        Stmt::Val { expr, .. } => collect_needed_subnames_in_expr(expr, import_name, needed),
        Stmt::Assign { target, value } => {
            collect_needed_subnames_in_expr(target, import_name, needed);
            collect_needed_subnames_in_expr(value, import_name, needed);
        }
        Stmt::Return(Some(e)) => collect_needed_subnames_in_expr(e, import_name, needed),
        Stmt::Expr(e) => collect_needed_subnames_in_expr(e, import_name, needed),
        Stmt::Pre(cs) | Stmt::Post(cs) => {
            for c in cs { collect_needed_subnames_in_expr(&c.expr, import_name, needed); }
        }
        Stmt::Assert(e, _) => collect_needed_subnames_in_expr(e, import_name, needed),
        Stmt::If { cond, then, elif_, else_, .. } => {
            collect_needed_subnames_in_expr(cond, import_name, needed);
            for s in then { collect_needed_subnames_in_stmt(s, import_name, needed); }
            for (ec, eb) in elif_ {
                collect_needed_subnames_in_expr(ec, import_name, needed);
                for s in eb { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
            if let Some(b) = else_ {
                for s in b { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
        }
        Stmt::For { init, cond, post, body } => {
            if let Some((_, e)) = init { collect_needed_subnames_in_expr(e, import_name, needed); }
            if let Some(e) = cond { collect_needed_subnames_in_expr(e, import_name, needed); }
            if let Some(s) = post { collect_needed_subnames_in_stmt(s, import_name, needed); }
            for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
        }
        Stmt::ForIn { iterable, body, .. } => {
            collect_needed_subnames_in_expr(iterable, import_name, needed);
            for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => {
            for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
        }
        Stmt::MatchUnion { expr, arms, else_body } => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
            for (_, body) in arms {
                for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
            if let Some(body) = else_body {
                for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
            for s in some_body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            for s in none_body { collect_needed_subnames_in_stmt(s, import_name, needed); }
        }
        Stmt::MatchResult { expr, ok_body, err_body, .. } => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
            for s in ok_body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            for s in err_body { collect_needed_subnames_in_stmt(s, import_name, needed); }
        }
        Stmt::MatchEnum { expr, arms } => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
            for (_, body) in arms {
                for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
        }
        Stmt::MatchString { expr, arms, else_body } => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
            for (_, body) in arms {
                for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
            if let Some(body) = else_body {
                for s in body { collect_needed_subnames_in_stmt(s, import_name, needed); }
            }
        }
        _ => {}
    }
}

fn collect_needed_subnames_in_expr(expr: &Expr, import_name: &str, needed: &mut HashSet<String>) {
    match expr {
        Expr::Field(base, field) => {
            if let Expr::Ident(name) = base.as_ref() {
                if name == import_name {
                    needed.insert(field.clone());
                }
            }
            collect_needed_subnames_in_expr(base, import_name, needed);
        }
        Expr::Call { callee, args, .. } => {
            collect_needed_subnames_in_expr(callee, import_name, needed);
            for a in args { collect_needed_subnames_in_expr(a, import_name, needed); }
        }
        Expr::Builtin { args, .. } => {
            for a in args { collect_needed_subnames_in_expr(a, import_name, needed); }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_needed_subnames_in_expr(lhs, import_name, needed);
            collect_needed_subnames_in_expr(rhs, import_name, needed);
        }
        Expr::UnOp { expr, .. } | Expr::Cast { expr, .. } | Expr::Some(expr) | Expr::OkVal(expr) | Expr::ErrVal(expr) | Expr::Try(expr) | Expr::Trust(expr) | Expr::Addr(expr) | Expr::Deref(expr) => {
            collect_needed_subnames_in_expr(expr, import_name, needed);
        }
        Expr::StructLit { fields } => {
            for (_, e) in fields { collect_needed_subnames_in_expr(e, import_name, needed); }
        }
        Expr::ArgsPack(exprs) => {
            for e in exprs { collect_needed_subnames_in_expr(e, import_name, needed); }
        }
        _ => {}
    }
}

/// Turn a use-path string into a filesystem path.
///
/// Rules:
///   "std"        -> <workspace>/std/std.sx (special case)
///   "io.sx"      -> <current_dir>/io.sx
///   "foo/bar.sx" -> <current_dir>/foo/bar.sx
fn resolve_use_path(path: &str, dir: &Path) -> PathBuf {
    if !path.ends_with(".sx") {
        let workspace_root = find_workspace_root(dir);
        let candidate = workspace_root.join(path).join(format!("{path}.sx"));
        if candidate.exists() {
            return candidate;
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                return exe_dir.join(path).join(format!("{path}.sx"));
            }
        }
        candidate
    } else {
        dir.join(path)
    }
}

fn find_workspace_root(start: &Path) -> PathBuf {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join("Cargo.toml").exists() || cur.join("spectre.mod").exists() {
            return cur;
        }
        if !cur.pop() {
            break;
        }
    }
    start.to_path_buf()
}
