use clap::Parser;

const USAGE: &str = "\
spectre <input> [options]

    [file].sx                  compile to binary (./file)
    [file].sx -o out           compile to binary at path 'out'
    [file].sx --emit-qbe       print QBE IR and exit
    [file].sx --emit-asm       print assembly and exit
    [file].sx --emit-tokens    print token stream and exit
    [file].sx --emit-ast       print AST and exit
    [file].sx --release        build in release mode, runtime safety checks off
    --help                     show this help message
    --version                  show the current version of the spectre compiler";

#[derive(Parser, Debug)]
#[command(
    name = "Spectre",
    version,
    about = "Spectre Compiler",
    override_usage = USAGE,
    help_template = "\
{about} v{version}

usage:
  {usage}

options:
{options}
"
)]
pub struct Args {
    /// Source file to compile (.sx)
    pub input: Option<String>,

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

    /// Omit all runtime safety checks from generated code
    #[arg(long)]
    pub release: bool,

    /// Print the version of the compiler
    #[arg(long, short)]
    pub version: bool,
}

/// All platforms the compiler knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Darwin,
    Windows,
    FreeBsd,
    OpenBsd,
    NetBsd,
    DragonFlyBsd,
    Solaris,
    Illumos,
    Plan9,
    Haiku,
    Android,
    Fuchsia,
    Redox,
    Unknown,
}

impl Platform {
    /// Detect the platform at compile time from Rust's cfg flags.
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::Darwin;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "freebsd")]
        return Platform::FreeBsd;
        #[cfg(target_os = "openbsd")]
        return Platform::OpenBsd;
        #[cfg(target_os = "netbsd")]
        return Platform::NetBsd;
        #[cfg(target_os = "dragonfly")]
        return Platform::DragonFlyBsd;
        #[cfg(target_os = "solaris")]
        return Platform::Solaris;
        #[cfg(target_os = "illumos")]
        return Platform::Illumos;
        #[cfg(target_os = "haiku")]
        return Platform::Haiku;
        #[cfg(target_os = "android")]
        return Platform::Android;
        #[cfg(target_os = "fuchsia")]
        return Platform::Fuchsia;
        #[cfg(target_os = "redox")]
        return Platform::Redox;
        #[allow(unreachable_code)]
        Platform::Unknown
    }

    /// Parse a platform name from a `when` clause identifier.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "linux" => Some(Platform::Linux),
            "darwin" | "macos" => Some(Platform::Darwin),
            "windows" => Some(Platform::Windows),
            "freebsd" => Some(Platform::FreeBsd),
            "openbsd" => Some(Platform::OpenBsd),
            "netbsd" => Some(Platform::NetBsd),
            "dragonflybsd" | "dragonfly" => Some(Platform::DragonFlyBsd),
            "solaris" => Some(Platform::Solaris),
            "illumos" => Some(Platform::Illumos),
            "plan9" => Some(Platform::Plan9),
            "haiku" => Some(Platform::Haiku),
            "android" => Some(Platform::Android),
            "fuchsia" => Some(Platform::Fuchsia),
            "redox" => Some(Platform::Redox),
            _ => None,
        }
    }

    /// Returns true if this platform matches the given `when` clause name.
    /// Handles both specific platform names and group aliases:
    ///   "posix"   — Linux, macOS, *BSD, Solaris, Illumos, Haiku, Android, Redox
    ///   "windows" — Windows only (also works as a specific name)
    pub fn matches_name(self, name: &str) -> bool {
        match name {
            "posix" => matches!(
                self,
                Platform::Linux
                    | Platform::Darwin
                    | Platform::FreeBsd
                    | Platform::OpenBsd
                    | Platform::NetBsd
                    | Platform::DragonFlyBsd
                    | Platform::Solaris
                    | Platform::Illumos
                    | Platform::Haiku
                    | Platform::Android
                    | Platform::Redox
            ),
            _ => Platform::from_str(name).map_or(false, |p| p == self),
        }
    }
}
