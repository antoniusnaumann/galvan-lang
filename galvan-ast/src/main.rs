use from_pest::FromPest;
use galvan_ast::{AstResult, IntoAst, RootItem};
use galvan_pest::exec::parse_current_dir;

fn main() {
    let src = parse_current_dir();

    for (parsed, source) in src {
        println!();
        println!("----- Source: {:?} -----", source.origin().unwrap_or("?"));

        let Ok(parsed) = parsed else { println!("Error during parsing"); continue; };
        let ast = parsed.try_into_ast();

        match ast {
            Ok(ast) => {
                println!("AST: {:#?}", ast);
            }
            Err(e) => println!("Error when converting to AST: {}", e)
        }
    }
}