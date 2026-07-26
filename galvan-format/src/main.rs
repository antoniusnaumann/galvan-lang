//! The `galvan-format` command-line formatter.
//!
//! With no file arguments it reads a document from stdin and writes the
//! formatted result to stdout — the contract editors like Helix expect from
//! an external formatter:
//!
//! ```toml
//! # languages.toml
//! [[language]]
//! name = "galvan"
//! formatter = { command = "galvan-format" }
//! ```
//!
//! With file arguments it rewrites the files in place; `--check` instead
//! reports files that would change and exits non-zero.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use galvan_format::{format_source, FormatOptions, LogicalStyle, UnicodeStyle};

const USAGE: &str = "\
Usage: galvan-format [OPTIONS] [FILES...]

Formats Galvan source code. Reads stdin and writes stdout when no files
are given; formats the files in place otherwise.

Options:
      --check              Don't write anything; exit 1 if a file would change
      --line-width <N>     Target maximum line width (default 100)
      --indent-width <N>   Spaces per indentation level (default 4)
      --use-tabs           Indent with tabs instead of spaces
      --operators <STYLE>  unicode | ascii | untouched (default untouched):
                           normalize operator pairs like ≠/!=, →/->, ±/+-
      --logical <STYLE>    word | symbol | untouched (default untouched):
                           normalize and/&&, or/||, not/!, in/∈
  -h, --help               Show this help
";

struct Args {
    check: bool,
    files: Vec<PathBuf>,
    options: FormatOptions,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        check: false,
        files: Vec::new(),
        options: FormatOptions::default(),
    };

    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--check" => args.check = true,
            "--use-tabs" => args.options.use_tabs = true,
            "--operators" => {
                let value = raw.next().ok_or_else(|| format!("{arg} requires a value"))?;
                args.options.unicode_operators = match value.as_str() {
                    "unicode" => UnicodeStyle::Unicode,
                    "ascii" => UnicodeStyle::Ascii,
                    "untouched" => UnicodeStyle::Untouched,
                    other => return Err(format!("--operators: unknown style: {other}")),
                };
            }
            "--logical" => {
                let value = raw.next().ok_or_else(|| format!("{arg} requires a value"))?;
                args.options.logical_operators = match value.as_str() {
                    "word" => LogicalStyle::Word,
                    "symbol" => LogicalStyle::Symbol,
                    "untouched" => LogicalStyle::Untouched,
                    other => return Err(format!("--logical: unknown style: {other}")),
                };
            }
            "--line-width" | "--indent-width" => {
                let value = raw
                    .next()
                    .ok_or_else(|| format!("{arg} requires a value"))?
                    .parse::<usize>()
                    .map_err(|_| format!("{arg} requires a number"))?;
                if value == 0 {
                    return Err(format!("{arg} must be at least 1"));
                }
                match arg.as_str() {
                    "--line-width" => args.options.max_width = value,
                    _ => args.options.indent_width = value,
                }
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ => args.files.push(PathBuf::from(arg)),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("galvan-format: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    if args.files.is_empty() {
        return format_stdin(&args);
    }
    format_files(&args)
}

/// Stdin -> stdout. On any error the *input is echoed back unchanged* so an
/// editor piping a buffer through us never loses or garbles it, and the
/// error goes to stderr with a non-zero exit.
fn format_stdin(args: &Args) -> ExitCode {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("galvan-format: could not read stdin: {error}");
        return ExitCode::FAILURE;
    }

    match format_source(&input, &args.options) {
        Ok(formatted) => {
            print!("{formatted}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            print!("{input}");
            eprintln!("galvan-format: {error}");
            ExitCode::FAILURE
        }
    }
}

fn format_files(args: &Args) -> ExitCode {
    let mut failed = false;
    let mut would_change = false;

    for path in &args.files {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("galvan-format: {}: {error}", path.display());
                failed = true;
                continue;
            }
        };
        let formatted = match format_source(&text, &args.options) {
            Ok(formatted) => formatted,
            Err(error) => {
                eprintln!("galvan-format: {}: {error}", path.display());
                failed = true;
                continue;
            }
        };
        if formatted == text {
            continue;
        }
        if args.check {
            println!("{}", path.display());
            would_change = true;
        } else if let Err(error) = std::fs::write(path, formatted) {
            eprintln!("galvan-format: {}: {error}", path.display());
            failed = true;
        }
    }

    if failed {
        ExitCode::from(2)
    } else if would_change {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
