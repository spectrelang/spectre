mod cli;
mod codegen;
mod lexer;
mod module;
mod parser;

use clap::Parser;
use cli::Args;
use std::process;

fn main() {
    let args = Args::parse();

    let output = match module::compile_file(&args.input, &args) {
        Ok(qbe) => qbe,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if let Some(out_path) = &args.output {
        std::fs::write(out_path, &output).unwrap_or_else(|e| {
            eprintln!("error writing output: {e}");
            process::exit(1);
        });
    } else {
        print!("{output}");
    }
}
