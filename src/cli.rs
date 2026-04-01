use clap::Parser;

const USAGE: &str = "\
spectre <input> [options]

    [file].st                  compile to binary (./file)
    [file].st -o out           compile to binary at path 'out'
    [file].st --emit-qbe       print QBE IR and exit
    [file].st --emit-asm       print assembly and exit
    [file].st --emit-tokens    print token stream and exit
    [file].st --emit-ast       print AST and exit";

/// Spectre compiler — lowers .st source to a native binary via QBE
#[derive(Parser, Debug)]
#[command(
    name = "spectre",
    version,
    about = "Spectre compiler — lowers .st source to a native binary via QBE",
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
    /// Source file to compile (.st)
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

    /// Run tests in the source file
    #[arg(long)]
    pub test: bool,
}
