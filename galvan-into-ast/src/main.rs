use std::path::PathBuf;

use galvan_into_ast::IntoAst;
use galvan_parse::exec::parse_current_dir;

fn main() {
    let src = parse_current_dir();

    for (parsed, source) in src {
        println!();
        println!(
            "----- Source: {:?} -----",
            source.origin().unwrap_or(&PathBuf::from("?"))
        );

        let parsed = match parsed {
            Ok(parsed) => {
                println!("Parsed: {:#?}", parsed);
                parsed
            }
            Err(e) => {
                println!("Error: {}\n{:#?}", e, e);
                continue;
            }
        };

        let ast = parsed.try_into_ast(source);

        match ast {
            Ok(ast) => {
                println!("AST: {:#?}", ast);
            }
            Err(e) => println!("Error when converting to AST: {}", dbg!(e)),
        }
    }
}
