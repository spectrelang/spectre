use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cli::Args;
use crate::codegen::Codegen;
use crate::lexer::Lexer;
use crate::parser::{Item, Module, Parser};
use crate::semantic;

/// A fully-resolved module: its parsed AST plus a map of imported sub-modules.
pub struct ResolvedModule {
    pub ast: Module,
    pub imports: HashMap<String, ResolvedModule>,
    pub dir: PathBuf,
    pub filename: String,
}

/// Entry point: compile a .sx file to QBE IR.
pub fn compile_file(input: &str, args: &Args) -> Result<(String, Vec<String>), String> {
    let path = PathBuf::from(input);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let src = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {input}: {e}"))?;

    let resolved = resolve_module(&src, &dir, &mut HashMap::new(), input)?;

    if args.emit_tokens {
        let mut lex = Lexer::new(&src);
        let tokens = lex.tokenize().map_err(|e| format!("{input}: {e}"))?;
        return Ok((format!("{tokens:#?}"), vec![]));
    }
    if args.emit_ast {
        return Ok((format!("{:#?}", resolved.ast), vec![]));
    }

    let sem_errors = semantic::check_module(&resolved);
    if !sem_errors.is_empty() {
        return Err(sem_errors.join("\n"));
    }

    let mut cg = Codegen::new();
    cg.emit_module(&resolved, args.test, args.release)?;
    let warnings = cg.warnings.clone();
    Ok((cg.finish(), warnings))
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

    Ok(ResolvedModule {
        ast,
        imports,
        dir: dir.to_path_buf(),
        filename: filename.to_string(),
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
