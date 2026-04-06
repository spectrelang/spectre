mod cli;
mod codegen;
mod lexer;
mod module;
mod parser;
mod semantic;
mod tests;

use clap::Parser;
use cli::Args;
use std::path::Path;
use std::process::{self, Command};

fn main() {
    let args = Args::parse();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let input = args.input.as_ref().unwrap_or_else(|| {
        eprintln!("error: no input file provided");
        eprintln!("usage: spectre <input> [options]");
        process::exit(1);
    });

    let (qbe_ir, .., libs) = match module::compile_file(input, &args) {
        Ok((ir, warnings, libs)) => {
            for w in &warnings {
                eprintln!("warning: {w}");
            }
            (ir, warnings, libs)
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
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
        Path::new(input)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    assemble_and_link(&asm, &binary_path, &libs, args.show_cmd);

    if args.test {
        run_tests(&binary_path);
    }
}

/// Run QBE on the IR and capture the assembly output.
fn run_qbe(ir: &str) -> String {
    let tmp_dir = std::env::temp_dir();
    let ir_path = tmp_dir.join("spectre_ir.ssa");
    std::fs::write(&ir_path, ir).unwrap_or_else(|e| {
        eprintln!("error writing temp QBE IR: {e}");
        process::exit(1);
    });

    let out = Command::new("qbe")
        .arg(&ir_path)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .output()
        .unwrap_or_else(|e| {
            eprintln!("error: could not run 'qbe': {e}");
            eprintln!("       make sure QBE is installed and on your PATH");
            eprintln!("       https://c9x.me/compile/");
            process::exit(1);
        });

    let _ = std::fs::remove_file(&ir_path);

    if !out.status.success() {
        eprintln!("qbe error:\n{}", String::from_utf8_lossy(&out.stderr));
        process::exit(1);
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Write assembly to a temp file, then invoke `cc` to assemble and link.
fn assemble_and_link(asm: &str, binary_path: &str, libs: &[String], show_cmd: bool) {
    use std::env;

    let tmp_dir = env::temp_dir();
    let asm_path = tmp_dir.join("spectre_out.s");

    std::fs::write(&asm_path, asm).unwrap_or_else(|e| {
        eprintln!("error writing temp assembly: {e}");
        process::exit(1);
    });

    std::fs::create_dir_all("./s-out/").unwrap_or_else(|e| {
        eprintln!("error creating s-out directory: {e}");
        process::exit(1);
    });

    let mut cmd = Command::new("cc");
    cmd.arg(&asm_path)
        .arg("-o")
        .arg(Path::join(Path::new("./s-out/"), binary_path))
        .arg("-lc");

    for lib in libs {
        if lib.starts_with('-') {
            for part in lib.split_whitespace() {
                cmd.arg(part);
            }
        } else {
            cmd.arg(lib);
        }
    }

    if show_cmd {
        println!("cc {:?}", cmd.get_args());
    }

    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("error: could not run 'cc': {e}");
        process::exit(1);
    });

    if !status.success() {
        eprintln!("error: assembler/linker failed");
        process::exit(1);
    }

    let _ = std::fs::remove_file(&asm_path);
}

/// Run the compiled binary in test mode.
fn run_tests(binary_path: &str) {
    let full_path = Path::join(Path::new("./s-out/"), binary_path);
    eprintln!("[spectre] running test binary: {}", full_path.display());
    let status = Command::new(&full_path).status().unwrap_or_else(|e| {
        eprintln!("error: could not run test binary: {e}");
        process::exit(1);
    });
    eprintln!("[spectre] test binary exited with: {status}");

    if !status.success() {
        eprintln!("tests failed");
        process::exit(1);
    }
}
