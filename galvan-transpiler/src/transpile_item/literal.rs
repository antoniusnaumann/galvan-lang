use galvan_ast::{BooleanLiteral, CharLiteral, Literal, NoneLiteral, NumberLiteral, StringLiteral};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::{context::Context, macros::impl_transpile_variants, Transpile};

impl_transpile_variants! { Literal;
    BooleanLiteral,
    StringLiteral,
    CharLiteral,
    NumberLiteral,
    NoneLiteral,
}

impl Transpile for StringLiteral {
    fn transpile(&self, context: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        if self.interpolations.is_empty() {
            // No interpolation - just return the string as format!
            format!("format!({})", self.as_str())
        } else {
            // Has interpolation - transpile expressions and create positional arguments
            let mut args = Vec::new();
            
            for interpolation in &self.interpolations {
                let transpiled_expr = interpolation.transpile(context, scope, errors);
                args.push(transpiled_expr);
            }
            
            // Combine template with arguments
            if args.is_empty() {
                format!("format!({})", self.as_str())
            } else {
                format!("format!({}, {})", self.as_str(), args.join(", "))
            }
        }
    }
}

impl Transpile for NumberLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        // TODO: Parse number and validate type
        format!("{}", self.as_str())
    }
}

impl Transpile for BooleanLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        format!("{}", self.value)
    }
}

impl Transpile for NoneLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        format!("None")
    }
}

impl Transpile for CharLiteral {
    fn transpile(&self, _: &Context, _scope: &mut Scope, _errors: &mut ErrorCollector) -> String {
        format!("'{}'", self.value.escape_default())
    }
}
