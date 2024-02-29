use galvan_ast::{BooleanLiteral, Literal, NoneLiteral, NumberLiteral, StringLiteral};
use galvan_resolver::Scope;

use crate::{context::Context, macros::impl_transpile_variants, Transpile};

impl_transpile_variants! { Literal;
    BooleanLiteral,
    StringLiteral,
    NumberLiteral,
    NoneLiteral,
}

impl Transpile for StringLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        // TODO: Implement more sophisticated formatting (extract {} and put them as separate arguments)
        format!("format!({})", self.as_str())
    }
}

impl Transpile for NumberLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}

impl Transpile for BooleanLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        format!("{self}")
    }
}

impl Transpile for NoneLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope) -> String {
        format!("None")
    }
}
