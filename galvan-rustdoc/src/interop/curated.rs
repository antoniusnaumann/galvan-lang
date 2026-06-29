use galvan_ast::{
    FnSignature, Ident, Param, ParamList, ResultTypeItem, Span, TypeElement, TypeIdent, Visibility,
};

use crate::model::RustReturnConversion;

use super::lift::{generic_type, plain_type};
use super::RustInterop;

impl RustInterop {
    pub(super) fn add_curated_crate(&mut self, crate_name: &str) {
        if crate_name != "serde_json" {
            return;
        }

        self.push_type(crate_name, "Error");
        self.push_type(crate_name, "Value");
        self.push_function(
            crate_name,
            "to_string",
            "::serde_json::to_string".into(),
            FnSignature {
                visibility: Visibility::public(),
                identifier: Ident::new("to_string"),
                parameters: ParamList {
                    params: vec![Param {
                        decl_modifier: None,
                        short_name: None,
                        identifier: Ident::new("value"),
                        param_type: generic_type("T"),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                },
                return_type: TypeElement::Result(Box::new(ResultTypeItem {
                    success: plain_type(TypeIdent::new("String")),
                    error: Some(plain_type(TypeIdent::new("Error"))),
                    span: Span::default(),
                })),
                where_clause: None,
                span: Span::default(),
            }
            .into(),
            false,
            RustReturnConversion::None,
            Vec::new(),
        );
    }
}
