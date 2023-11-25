use crate::{Transpile, Body};

impl Transpile for Body {
    fn transpile(self) -> String {
        transpile_body(self)
    }
}

fn transpile_body(body: Body) -> String {
    String::new()
    // TODO: Transpile statements
    // let Body { statements } = body;
    // transpile!("{}", statements)
}