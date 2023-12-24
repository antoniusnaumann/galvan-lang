use crate::{Block, LookupContext, Transpile};

impl Transpile for Block {
    fn transpile(&self, lookup: &LookupContext) -> String {
        transpile_body(self, lookup)
    }
}

fn transpile_body(body: &Block, lookup: &LookupContext) -> String {
    // TODO: Transpile statements
    "{ }".to_string()
}
