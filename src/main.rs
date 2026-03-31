mod cli;
mod codegen;
mod lexer;
mod module;
mod parser;

use clap::Parser;
use cli::Args;
use std::path::Path;
use std::process::{self, Command};

fn main() {
    let args = Args::parse();

    let qbe_ir = match module::compile_file(&args.input, &args) {
        Ok(ir) => ir,
        Err(e) => { eprintln!("error: {e}"); process::exit(1); }
    };

    if args.emit_qbe {
        print!("{qbe_ir}");
        return;
    }

    let asm = run_qbe(&qbe_ir);

    if args.emit_asm {
        print!("{asm}");
        return;
    }

    let binary_path = args.output.clone().unwrap_or_else(|| {
        Path::new(&args.input)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    assemble_and_link(&asm, &binary_path);
}

/// Pipe QBE IR into `qbe` and capture the assembly output.
fn run_qbe(ir: &str) -> String {
    let mut child = Command::new("qbe")
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("error: could not run 'qbe': {e}");
            eprintln!("       make sure QBE is installed and on your PATH");
            eprintln!("       https://c9x.me/compile/");
            process::exit(1);
        });

    use std::io::Write;
    child.stdin.take().unwrap().write_all(ir.as_bytes()).unwrap();

    let out = child.wait_with_output().unwrap();
    if !out.status.success() {
        eprintln!("qbe error:\n{}", String::from_utf8_lossy(&out.stderr));
        process::exit(1);
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Write assembly to a temp file, then invoke `cc` to assemble and link.
fn assemble_and_link(asm: &str, binary_path: &str) {
    use std::env;

    let tmp_dir = env::temp_dir();
    let asm_path = tmp_dir.join("spectre_out.s");

    std::fs::write(&asm_path, asm).unwrap_or_else(|e| {
        eprintln!("error writing temp assembly: {e}");
        process::exit(1);
    });

    let status = Command::new("cc")
        .arg(&asm_path)
        .arg("-o")
        .arg(binary_path)
        .arg("-lc")
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: could not run 'cc': {e}");
            process::exit(1);
        });

    if !status.success() {
        eprintln!("error: assembler/linker failed");
        process::exit(1);
    }

    let _ = std::fs::remove_file(&asm_path);
}
