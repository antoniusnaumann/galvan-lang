use std::fs;
use std::path::Path;

use serde_json::Value;

use galvan_ast::{
    BasicTypeItem, GenericTypeItem, Ident, ParametricTypeItem, ResultTypeItem, Span, TypeDecl,
    TypeElement, TypeIdent,
};
use galvan_rustdoc::{check_format_version, RustArgConversion, RustInterop, RustReturnConversion};

#[test]
fn committed_rustdoc_json_lifts_expected_fixture_items() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("interop_fixture.golden.json");
    let json: Value = serde_json::from_str(&fs::read_to_string(&path).expect("golden fixture"))
        .expect("valid JSON fixture");
    check_format_version(&path, &json).expect("fixture rustdoc format version");
    let krate: rustdoc_types::Crate =
        serde_json::from_value(json).expect("fixture matches rustdoc-types schema");

    let mut interop = RustInterop::empty();
    let summary = interop.add_crate("interop_fixture", &krate);

    assert_eq!(summary.crate_name.as_ref(), "interop_fixture");
    assert_eq!(summary.types, 4);
    assert_eq!(summary.functions, 3);
    assert_eq!(summary.constants, 2);
    assert_eq!(
        interop.lift_summary("interop_fixture"),
        Some(&summary),
        "add_crate should record the lift summary"
    );

    assert_payload_type(&interop);
    assert_generic_envelope_type(&interop);
    assert_fixture_error_type(&interop);
    assert_generic_event_type(&interop);
    assert_parse_payload_function(&interop);
    assert_boxed_method(&interop);
    assert_envelope_new(&interop);
    assert_associated_constants(&interop);
}

fn assert_payload_type(interop: &RustInterop) {
    let payload = imported_type(interop, "Payload");
    assert_eq!(payload.rust_path.as_ref(), "::interop_fixture::Payload");
    let TypeDecl::Struct(decl) = &payload.decl.item else {
        panic!("expected Payload to lift as a struct");
    };
    assert_eq!(decl.ident, type_ident("Payload"));
    assert!(decl.generic_params.is_empty());
    assert_eq!(decl.members.len(), 2);
    assert_eq!(decl.members[0].ident, ident("id"));
    assert_eq!(decl.members[0].r#type, plain_type("U64"));
    assert_eq!(decl.members[1].ident, ident("name"));
    assert_eq!(decl.members[1].r#type, plain_type("String"));
}

fn assert_generic_envelope_type(interop: &RustInterop) {
    let envelope = imported_type(interop, "Envelope");
    assert_eq!(envelope.rust_path.as_ref(), "::interop_fixture::Envelope");
    let TypeDecl::Struct(decl) = &envelope.decl.item else {
        panic!("expected Envelope to lift as a struct");
    };
    assert_eq!(decl.ident, type_ident("Envelope"));
    assert_eq!(decl.generic_params, vec![ident("T")]);
    assert_eq!(decl.members.len(), 1);
    assert_eq!(decl.members[0].ident, ident("value"));
    assert_eq!(decl.members[0].r#type, generic_type("T"));
}

fn assert_fixture_error_type(interop: &RustInterop) {
    let fixture_error = imported_type(interop, "FixtureError");
    assert_eq!(
        fixture_error.rust_path.as_ref(),
        "::interop_fixture::FixtureError"
    );
    let TypeDecl::Enum(decl) = &fixture_error.decl.item else {
        panic!("expected FixtureError to lift as an enum");
    };
    assert_eq!(decl.ident, type_ident("FixtureError"));
    assert_eq!(decl.members.len(), 1);
    assert_eq!(decl.members[0].ident, type_ident("Invalid"));
    assert!(decl.members[0].fields.is_empty());
}

fn assert_generic_event_type(interop: &RustInterop) {
    let event = imported_type(interop, "FixtureEvent");
    assert_eq!(event.rust_path.as_ref(), "::interop_fixture::FixtureEvent");
    let TypeDecl::Enum(decl) = &event.decl.item else {
        panic!("expected FixtureEvent to lift as an enum");
    };
    assert_eq!(decl.ident, type_ident("FixtureEvent"));
    assert_eq!(decl.generic_params, vec![ident("T")]);
    assert_eq!(decl.members.len(), 2);
    assert_eq!(decl.members[0].ident, type_ident("Ready"));
    assert_eq!(decl.members[0].fields[0].name, None);
    assert_eq!(decl.members[0].fields[0].r#type, generic_type("T"));
    assert_eq!(decl.members[1].ident, type_ident("Failed"));
    assert_eq!(decl.members[1].fields[0].name, Some(ident("message")));
    assert_eq!(decl.members[1].fields[0].r#type, plain_type("String"));
}

fn assert_parse_payload_function(interop: &RustInterop) {
    let function = interop
        .function(Some("interop_fixture"), None, &ident("parse_payload"), &[])
        .expect("expected parse_payload function");
    assert_eq!(
        function.rust_path.as_ref(),
        "::interop_fixture::parse_payload"
    );
    assert_eq!(function.return_conversion, RustReturnConversion::None);
    assert_eq!(
        function.arg_conversions,
        vec![RustArgConversion::SharedBorrow]
    );
    assert_eq!(
        function.decl.item.signature.identifier,
        ident("parse_payload")
    );
    assert_eq!(function.decl.item.signature.parameters.params.len(), 1);
    assert_eq!(
        function.decl.item.signature.parameters.params[0].identifier,
        ident("input")
    );
    assert_eq!(
        function.decl.item.signature.parameters.params[0].param_type,
        plain_type("String")
    );
    assert_eq!(
        function.decl.item.signature.return_type,
        result_type(plain_type("Payload"), plain_type("FixtureError"))
    );
}

fn assert_boxed_method(interop: &RustInterop) {
    let function = interop
        .function(
            Some("interop_fixture"),
            Some(&type_ident("Payload")),
            &ident("boxed"),
            &[],
        )
        .expect("expected Payload.boxed method");
    assert_eq!(
        function.rust_path.as_ref(),
        "::interop_fixture::Payload::boxed"
    );
    assert_eq!(function.return_conversion, RustReturnConversion::BoxDeref);
    assert_eq!(
        function.decl.item.signature.return_type,
        plain_type("Payload")
    );
    let receiver = function
        .decl
        .item
        .signature
        .receiver()
        .expect("self receiver");
    assert_eq!(receiver.identifier, ident("self"));
    assert_eq!(receiver.param_type, plain_type("Payload"));
}

fn assert_envelope_new(interop: &RustInterop) {
    let function = interop
        .associated_function(
            Some("interop_fixture"),
            &type_ident("Envelope"),
            &ident("new"),
            &[],
        )
        .expect("expected Envelope.new associated function");
    assert_eq!(
        function.rust_path.as_ref(),
        "::interop_fixture::Envelope::new"
    );
    assert_eq!(function.return_conversion, RustReturnConversion::None);
    assert_eq!(function.decl.item.signature.parameters.params.len(), 1);
    assert_eq!(
        function.decl.item.signature.parameters.params[0].param_type,
        generic_type("T")
    );
    assert_eq!(
        function.decl.item.signature.return_type,
        parametric_type("Envelope", vec![generic_type("T")])
    );
}

fn assert_associated_constants(interop: &RustInterop) {
    let default_id = interop
        .associated_constant(
            Some("interop_fixture"),
            &type_ident("Payload"),
            &ident("DEFAULT_ID"),
        )
        .expect("expected Payload.DEFAULT_ID associated constant");
    assert_eq!(
        default_id.rust_path.as_ref(),
        "::interop_fixture::Payload::DEFAULT_ID"
    );
    assert_eq!(default_id.ty, plain_type("U64"));

    let default_capacity = interop
        .associated_constant(
            Some("interop_fixture"),
            &type_ident("Envelope"),
            &ident("DEFAULT_CAPACITY"),
        )
        .expect("expected Envelope.DEFAULT_CAPACITY associated constant");
    assert_eq!(
        default_capacity.rust_path.as_ref(),
        "::interop_fixture::Envelope::DEFAULT_CAPACITY"
    );
    assert_eq!(default_capacity.ty, plain_type("USize"));
}

fn imported_type<'a>(interop: &'a RustInterop, name: &str) -> &'a galvan_rustdoc::RustTypeDecl {
    interop
        .types
        .iter()
        .find(|ty| ty.name.as_str() == name)
        .unwrap_or_else(|| panic!("expected imported type {name}"))
}

fn ident(name: &str) -> Ident {
    Ident::new(name)
}

fn type_ident(name: &str) -> TypeIdent {
    TypeIdent::new(name)
}

fn plain_type(name: &str) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident: type_ident(name),
        span: Span::default(),
    })
}

fn generic_type(name: &str) -> TypeElement {
    TypeElement::Generic(GenericTypeItem {
        ident: ident(name),
        span: Span::default(),
    })
}

fn parametric_type(name: &str, type_args: Vec<TypeElement>) -> TypeElement {
    TypeElement::Parametric(ParametricTypeItem {
        base_type: type_ident(name),
        type_args,
        span: Span::default(),
    })
}

fn result_type(success: TypeElement, error: TypeElement) -> TypeElement {
    TypeElement::Result(Box::new(ResultTypeItem {
        success,
        error: Some(error),
        span: Span::default(),
    }))
}
