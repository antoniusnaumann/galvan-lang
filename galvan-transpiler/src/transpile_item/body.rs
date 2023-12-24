use crate::{Body, LookupContext, Transpile};

impl Transpile for Body {
    fn transpile(&self, lookup: &LookupContext) -> String {
        transpile_body(self, lookup)
    }
}

fn transpile_body(body: &Body, lookup: &LookupContext) -> String {
    String::new()
    // TODO: Transpile statements
    // let Body { statements } = body;
    // transpile!("{}", statements)
}
