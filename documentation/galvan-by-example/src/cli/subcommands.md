# Subcommands

Every additional `cmd` becomes a subcommand. Its doc comment is the
subcommand description; parameter doc comments describe the flags:

```galvan
/// Greets the user
cmd greet(
    /// First name of the person to greet
    n name: String,
    /// Surname of the person that should be greeted
    s surname: String?
) {
    try surname |surname| {
        println "Hello \(name) \(surname)!"
    } else {
        println "Hello \(name)!"
    }
}
```

```text
$ my-app greet --help
Greets the user

Usage: my-app greet [OPTIONS] --name <NAME>

Options:
  -n, --name <NAME>        First name of the person to greet
  -s, --surname <SURNAME>  Surname of the person that should be greeted
  -h, --help               Print help

$ my-app greet -n Grace -s Hopper
Hello Grace Hopper!
```

<details>
<summary>Generated Rust</summary>

```rust
fn greet(name: String, surname: Option<String>) {
    match surname {
        Some((surname)) => {
            println!("{}", &format!("Hello {} {}!", name, surname));
        }
        None => {
            println!("{}", &format!("Hello {}!", name));
        }
    };
}

#[derive(Subcommand)]
enum Commands {
    /// Greets the user
    Greet(GreetArgs),
}

#[derive(clap::Args, Debug)]
struct GreetArgs {
    #[arg(short = 'n', long = "name", help = "First name of the person to greet")]
    pub name: String,
    #[arg(
        short = 's',
        long = "surname",
        help = "Surname of the person that should be greeted"
    )]
    pub surname: Option<String>,
}
```

Each `cmd` contributes a variant to the generated `Commands` enum and an args
struct with `clap` attributes assembled from the declaration.

</details>

Required flags come from required parameters (`String`), optional flags from
optional ones (`String?`).
