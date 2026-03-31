use clap::Parser;

const USAGE: &str = "\
    spectre <input> [options]

    spectre simple.spr
    spectre simple.spr -o simple.ssa
    spectre simple.spr --emit-tokens
    spectre simple.spr --emit-ast";

#[derive(Parser, Debug)]
#[command(
    name = "spectre",
    version,
    about = "Spectre compiler",
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

    /// Output file (defaults to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,

    /// Print the token stream and exit
    #[arg(long)]
    pub emit_tokens: bool,

    /// Print the AST and exit
    #[arg(long)]
    pub emit_ast: bool,
}
