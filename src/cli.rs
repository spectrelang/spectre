use clap::Parser;

const USAGE: &str = "\
spectre <input> [options]

    [file].spr                  compile to binary (./file)
    [file].spr -o out           compile to binary at path 'out'
    [file].spr --emit-qbe       print QBE IR and exit
    [file].spr --emit-asm       print assembly and exit
    [file].spr --emit-tokens    print token stream and exit
    [file].spr --emit-ast       print AST and exit";

/// Spectre compiler — lowers .spr source to a native binary via QBE
#[derive(Parser, Debug)]
#[command(
    name = "spectre",
    version,
    about = "Spectre compiler — lowers .spr source to a native binary via QBE",
    override_usage = USAGE,
    help_template = "\
{name} v{version}
{about}

usage:
  {usage}

options:
{options}
"
)]
pub struct Args {
    /// Source file to compile (.spr)
    pub input: String,

    /// Output binary path (default: input filename without extension)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,

    /// Print QBE IR to stdout and exit
    #[arg(long)]
    pub emit_qbe: bool,

    /// Print assembly to stdout and exit (runs QBE, stops before assembler)
    #[arg(long)]
    pub emit_asm: bool,

    /// Print the token stream and exit
    #[arg(long)]
    pub emit_tokens: bool,

    /// Print the AST and exit
    #[arg(long)]
    pub emit_ast: bool,
}
