use galvan_ast::{AccessExpression, TypeElement, YeetExpression};
use galvan_resolver::Scope;

use crate::error::ErrorCollector;
use crate::type_inference::InferType;
use crate::TranspilerError;
use crate::{context::Context, macros::transpile, Transpile};

impl Transpile for AccessExpression {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // TODO: typecheck that base is a collection and the key types matches
        let collection_type = self.base.infer_type(scope, errors);

        // TODO: properly use cast function to cast index to expected index type
        match collection_type {
            TypeElement::Array(a) => {
                transpile!(ctx, scope, errors, "{}[{}]", self.base, self.index)
            }
            TypeElement::Dictionary(d) => {
                transpile!(ctx, scope, errors, "{}[&{}]", self.base, self.index)
            }

            TypeElement::OrderedDictionary(d) => {
                transpile!(ctx, scope, errors, "{}[&{}]", self.base, self.index)
            }

            TypeElement::Set(s) => transpile!(ctx, scope, errors, "{}[&{}]", self.base, self.index),
            _ => {
                errors.error(TranspilerError::InvalidOperationOnType {
                    operation: "index access".into(),
                    allowed_types: "collection types".into(),
                });
                format!("/* Invalid index access */")
            }
        }
    }
}

impl Transpile for YeetExpression {
    fn transpile(&self, ctx: &Context, scope: &mut Scope, errors: &mut ErrorCollector) -> String {
        // TODO: check that type is error or optional
        // TODO: check that we are inside a function that returns a compatible error
        transpile!(ctx, scope, errors, "{}?", self.inner)
    }
}
