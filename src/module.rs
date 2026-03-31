use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cli::Args;
use crate::codegen::Codegen;
use crate::lexer::Lexer;
use crate::parser::{Item, Module, Parser};

/// A fully-resolved module: its parsed AST plus a map of imported sub-modules.
pub struct ResolvedModule {
    /// The current module
    pub ast: Module,

    /// name → resolved child module
    pub imports: HashMap<String, ResolvedModule>,

    /// The directory this module lives in (used to resolve relative imports)
    pub dir: PathBuf,
}

/// Entry point: compile a .spr file to QBE IR.
pub fn compile_file(input: &str, args: &Args) -> Result<String, String> {
    let path = PathBuf::from(input);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let src = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {input}: {e}"))?;

    let resolved = resolve_module(&src, &dir, &mut HashMap::new(), input)?;

    if args.emit_tokens {
        let mut lex = Lexer::new(&src);
        let tokens = lex.tokenize().map_err(|e| e)?;
        return Ok(format!("{tokens:#?}"));
    }
    if args.emit_ast {
        return Ok(format!("{:#?}", resolved.ast));
    }

    let mut cg = Codegen::new();
    cg.emit_module(&resolved, args.test)?;
    Ok(cg.finish())
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
    let tokens = lex.tokenize()?;
    let mut parser = Parser::with_filename(tokens, filename.to_string());
    let ast = parser.parse_module()?;
    let mut imports = HashMap::new();

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
    })
}

/// Turn a use-path string into a filesystem path.
///
/// Rules:
///   "std"        → <workspace>/std/std.spr   (stdlib shorthand)
///   "io.spr"     → <current_dir>/io.spr
///   "foo/bar.spr"→ <current_dir>/foo/bar.spr
fn resolve_use_path(path: &str, dir: &Path) -> PathBuf {
    if !path.ends_with(".spr") {
        let workspace_root = find_workspace_root(dir);
        workspace_root.join(path).join(format!("{path}.spr"))
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
