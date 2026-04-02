use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cli::Args;
use crate::codegen::Codegen;
use crate::lexer::Lexer;
use crate::parser::{Expr, Item, Module, Parser};
use crate::semantic;

/// A fully-resolved module: its parsed AST plus a map of imported sub-modules.
pub struct ResolvedModule {
    pub ast: Module,
    pub imports: HashMap<String, ResolvedModule>,
    pub dir: PathBuf,
    pub filename: String,
    pub links: Vec<String>,
}

/// Entry point: compile a .sx file to QBE IR.
pub fn compile_file(input: &str, args: &Args) -> Result<(String, Vec<String>, Vec<String>), String> {
    let path = PathBuf::from(input);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let src = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {input}: {e}"))?;
    let resolved = resolve_module(&src, &dir, &mut HashMap::new(), input)?;

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
    let warnings = cg.warnings.clone();
    Ok((cg.finish(), warnings, libs))
}

/// Collect link libs:
/// - Always include libs declared in the root module itself.
/// - For each imported child module, only include its libs (recursively) if
///   that import is actually referenced somewhere in the root's AST.
pub fn collect_used_libs(root: &ResolvedModule) -> Vec<String> {
    let mut libs: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Always include the root's own link declarations.
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
        Stmt::Defer(body) | Stmt::When { body, .. } | Stmt::Otherwise { body } => {
            for s in body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::WhenIs { expr, body, .. } => {
            collect_used_imports_in_expr(expr, imports, used);
            for s in body { collect_used_imports_in_stmt(s, imports, used); }
        }
        Stmt::Match { expr, some_body, none_body, .. } => {
            collect_used_imports_in_expr(expr, imports, used);
            for s in some_body { collect_used_imports_in_stmt(s, imports, used); }
            for s in none_body { collect_used_imports_in_stmt(s, imports, used); }
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
            // The root of a dotted path like `std.net.Http.get` is an Ident.
            // Mark it used, then recurse to catch nested calls.
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
        Expr::UnOp { expr, .. } | Expr::Cast { expr, .. } | Expr::Some(expr) | Expr::Trust(expr) => {
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
/// `visited` prevents infinite loops on circular imports.
pub fn resolve_module(
    src: &str,
    dir: &Path,
    visited: &mut HashMap<PathBuf, ()>,
    filename: &str,
) -> Result<ResolvedModule, String> {
    let mut lex = Lexer::new(src);
    let tokens = lex.tokenize().map_err(|e| format!("{filename}:{e}"))?;
    let mut parser = Parser::with_filename(tokens, filename.to_string());
    let ast = parser.parse_module()?;
    let mut imports = HashMap::new();
    let self_path = PathBuf::from(filename);

    visited.insert(self_path, ());

    for item in &ast.items {
        if let Item::Use { name, path } = item {
            let resolved_path = resolve_use_path(path, dir);
            if visited.contains_key(&resolved_path) {
                continue;
            }
            visited.insert(resolved_path.clone(), ());
            let child_src = std::fs::read_to_string(&resolved_path)
                .map_err(|e| format!("cannot load module '{}': {e}", resolved_path.display()))?;
            let child_dir = resolved_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();
            let child_filename = resolved_path.to_string_lossy().to_string();
            let child = resolve_module(&child_src, &child_dir, visited, &child_filename)?;
            imports.insert(name.clone(), child);
        }
    }

    let current_platform = crate::cli::Platform::current();
    let links: Vec<String> = ast.items.iter().flat_map(|item| -> Vec<String> {
        match item {
            Item::Link { lib } => vec![lib.clone()],
            Item::LinkWhen { platform, libs } => {
                if crate::cli::Platform::from_str(platform)
                    .map(|p| p == current_platform)
                    .unwrap_or(false)
                {
                    libs.clone()
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }).collect();

    Ok(ResolvedModule {
        ast,
        imports,
        dir: dir.to_path_buf(),
        filename: filename.to_string(),
        links,
    })
}

/// Turn a use-path string into a filesystem path.
///
/// Rules:
///   "std"        -> <workspace>/std/std.sx   (stdlib shorthand)
///   "io.sx"      -> <current_dir>/io.sx
///   "foo/bar.sx" -> <current_dir>/foo/bar.sx
fn resolve_use_path(path: &str, dir: &Path) -> PathBuf {
    if !path.ends_with(".sx") {
        let workspace_root = find_workspace_root(dir);
        workspace_root.join(path).join(format!("{path}.sx"))
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
