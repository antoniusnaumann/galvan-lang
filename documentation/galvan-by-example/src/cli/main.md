# Flags on cmd main

`cmd main` declares the top-level command. Each parameter is a flag: the
short name comes first, then the parameter name (which doubles as the long
flag), and `///` doc comments become the help text. Optional parameters are
optional flags:

```galvan
cmd main(
    /// Optional name to greet when no subcommand is selected
    n name: String?
) {
    try name |name| {
        println "Hello \(name)!"
    } else {
        println "Hello World!"
    }
}
```

```text
$ my-app --help
Options:
  -n, --name <NAME>  Optional name to greet when no subcommand is selected
  -h, --help         Print help
  -V, --version      Print version

$ my-app --name Ada
Hello Ada!
```

A project uses either `fn main` *or* `cmd main` — the `cmd` form takes over
the entry point and dispatches through the generated parser.

<details>
<summary>Generated Rust</summary>

```rust
fn __main_command(name: Option<String>) {
    match name {
        Some((name)) => {
            println!("{}", &format!("Hello {}!", name));
        }
        None => {
            println!("{}", &format!("Hello World!"));
        }
    };
}

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None, subcommand_negates_reqs = true)]
struct Cli {
    #[arg(
        short = 'n',
        long = "name",
        help = "Optional name to greet when no subcommand is selected"
    )]
    pub name: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

pub(crate) fn __cli_main() {
    let Cli { name, command } = Cli::parse();
    match command {
        Some(Commands::Greet(args)) => greet(args.name, args.surname),
        None => __main_command(name),
    }
}
```

The `Commands::Greet` arm belongs to the subcommand declared on the
[next page](subcommands.md) — the parser dispatches every `cmd` in the
project from one place.

</details>
