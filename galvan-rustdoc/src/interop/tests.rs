use super::*;

use galvan_ast::Span;
use galvan_files::Source;
use serde_json::{json, Value};

fn ident(name: &str) -> Ident {
    Ident::new(name)
}

fn use_decl(segments: &[&str]) -> ToplevelItem<UseDecl> {
    ToplevelItem {
        item: UseDecl {
            path: galvan_ast::UsePath {
                segments: segments
                    .iter()
                    .map(|segment| Ident::new(*segment))
                    .collect(),
                span: Span::default(),
            },
            span: Span::default(),
        },
        source: Source::Builtin,
    }
}

fn primitive(name: &str) -> Value {
    json!({ "primitive": name })
}

fn generic(name: &str) -> Value {
    json!({ "generic": name })
}

fn generic_param(name: &str) -> Value {
    json!({
        "name": name,
        "kind": {
            "type": {
                "bounds": [],
                "default": null,
                "is_synthetic": false
            }
        }
    })
}

fn lifetime_param(name: &str) -> Value {
    json!({
        "name": name,
        "kind": {
            "lifetime": {
                "outlives": []
            }
        }
    })
}

fn type_generics(params: Vec<Value>) -> Value {
    json!({
        "params": params,
        "where_predicates": []
    })
}

fn never() -> Value {
    json!({ "never": null })
}

fn resolved(name: &str, args: Vec<Value>) -> Value {
    json!({
        "resolved_path": {
            "name": name,
            "args": {
                "angle_bracketed": {
                    "args": args
                        .into_iter()
                        .map(|arg| json!({ "type": arg }))
                        .collect::<Vec<_>>()
                }
            }
        }
    })
}

fn resolved_with_path(name: &str, path: &[&str], args: Vec<Value>) -> Value {
    json!({
        "resolved_path": {
            "name": name,
            "path": path,
            "args": {
                "angle_bracketed": {
                    "args": args
                        .into_iter()
                        .map(|arg| json!({ "type": arg }))
                        .collect::<Vec<_>>()
                }
            }
        }
    })
}

fn slice(ty: Value) -> Value {
    json!({ "slice": ty })
}

fn array(ty: Value) -> Value {
    json!({
        "array": {
            "type": ty,
            "len": "3"
        }
    })
}

fn raw_pointer(ty: Value, mutable: bool) -> Value {
    json!({
        "raw_pointer": {
            "type": ty,
            "mutable": mutable
        }
    })
}

fn qualified_path(name: &str, self_type: Value) -> Value {
    json!({
        "qualified_path": {
            "name": name,
            "args": null,
            "self_type": self_type,
            "trait": {
                "path": "demo::Visitor",
                "name": "Visitor",
                "id": "trait",
                "args": null
            }
        }
    })
}

fn dyn_trait() -> Value {
    json!({ "dyn_trait": [] })
}

fn impl_trait() -> Value {
    json!({ "impl_trait": [] })
}

fn function_pointer(inputs: Vec<Value>, output: Value) -> Value {
    json!({
        "function_pointer": {
            "sig": {
                "inputs": inputs,
                "output": output
            }
        }
    })
}

fn mut_borrowed(ty: Value) -> Value {
    json!({
        "borrowed_ref": {
            "type": ty,
            "mutable": true
        }
    })
}

fn borrowed(ty: Value) -> Value {
    json!({
        "borrowed_ref": {
            "type": ty
        }
    })
}

fn string_type() -> TypeElement {
    plain_type(TypeIdent::new("String"))
}

fn u64_type() -> TypeElement {
    plain_type(TypeIdent::new("U64"))
}

fn public_item(name: &str, inner: Value) -> Value {
    public_item_at_path(name, name, &["demo", name], inner)
}

fn public_item_at_path(id: &str, name: &str, path: &[&str], inner: Value) -> Value {
    json!({
        "id": id,
        "name": name,
        "visibility": "public",
        "path": path,
        "inner": inner
    })
}

fn public_field(name: &str, ty: Value) -> Value {
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "inner": {
            "struct_field": ty
        }
    })
}

fn public_function(name: &str, inputs: Vec<Value>, output: Value) -> Value {
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "function": {
                "sig": {
                    "inputs": inputs,
                    "output": output
                }
            }
        }
    })
}

fn public_unsafe_function(name: &str, inputs: Vec<Value>, output: Value) -> Value {
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "function": {
                "sig": {
                    "inputs": inputs,
                    "output": output
                },
                "header": {
                    "is_unsafe": true
                }
            }
        }
    })
}

fn public_constant(name: &str, ty: Value) -> Value {
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "constant": {
                "type": ty
            }
        }
    })
}

fn public_use(id: &str, name: &str, target_id: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "use": {
                "source": format!("demo::{name}"),
                "name": name,
                "id": target_id,
                "is_glob": false
            }
        }
    })
}

fn public_external_use(id: &str, name: &str, source: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "use": {
                "source": source,
                "name": name,
                "id": null,
                "is_glob": false
            }
        }
    })
}

fn public_glob_use(id: &str, name: &str, target_id: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "visibility": "public",
        "path": ["demo"],
        "inner": {
            "use": {
                "source": format!("demo::{name}::*"),
                "name": name,
                "id": target_id,
                "is_glob": true
            }
        }
    })
}

fn public_module(name: &str, items: Vec<&str>) -> Value {
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "path": ["demo", name],
        "inner": {
            "module": {
                "items": items
            }
        }
    })
}

fn imported_type<'a>(interop: &'a RustInterop, name: &str) -> &'a TypeDecl {
    &interop
        .types
        .iter()
        .find(|ty| ty.name.as_str() == name)
        .unwrap_or_else(|| panic!("expected imported type {name}"))
        .decl
        .item
}

#[test]
fn loading_a_crate_does_not_import_its_functions_unqualified() {
    let interop = RustInterop::from_crates_and_uses(["serde_json".to_string()], &[]).unwrap();

    assert!(interop
        .function(Some("serde_json"), None, &ident("to_string"), &[])
        .is_some());
    assert!(interop
        .function(None, None, &ident("to_string"), &[])
        .is_none());
}

#[test]
fn use_declarations_import_functions_unqualified() {
    let uses = [use_decl(&["serde_json"])];
    let interop = RustInterop::from_crates_and_uses([], &uses).unwrap();

    assert!(interop
        .function(None, None, &ident("to_string"), &[])
        .is_some());
}

#[test]
fn path_use_declarations_import_only_the_named_item() {
    let uses = [use_decl(&["serde_json", "to_string"])];
    let interop = RustInterop::from_crates_and_uses([], &uses).unwrap();

    assert!(interop
        .function(None, None, &ident("to_string"), &[])
        .is_some());
    assert!(interop
        .function(None, None, &ident("from_str"), &[])
        .is_none());
}

#[test]
fn use_declarations_import_types_unqualified() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            }))
        }
    });
    let uses = [use_decl(&["demo"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);
    interop.import_uses(&uses);

    let imported = interop.imported_types().collect::<Vec<_>>();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, TypeIdent::new("Ticket"));
    assert_eq!(imported[0].rust_path.as_ref(), "::demo::Ticket");
}

#[test]
fn path_use_declarations_import_only_the_named_type() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": public_item("InternalNote", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            }))
        }
    });
    let uses = [use_decl(&["demo", "Ticket"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);
    interop.import_uses(&uses);

    let imported = interop.imported_types().collect::<Vec<_>>();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, TypeIdent::new("Ticket"));
}

#[test]
fn rustdoc_type_only_crates_do_not_fall_back_to_curated_metadata() {
    let json = json!({
        "index": {
            "0": public_item_at_path("Value", "Value", &["serde_json", "Value"], json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            }))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("serde_json", &json);

    assert!(interop
        .types
        .iter()
        .any(|ty| ty.name.as_str() == "Value" && ty.rust_path.as_ref() == "::serde_json::Value"));
    assert!(interop
        .function(Some("serde_json"), None, &ident("to_string"), &[])
        .is_none());
    assert!(interop.types.iter().all(|ty| ty.name.as_str() != "Error"));
}

#[test]
fn use_declarations_import_constants_unqualified() {
    let json = json!({
        "index": {
            "0": public_constant("DEFAULT_LIMIT", primitive("u64"))
        }
    });
    let uses = [use_decl(&["demo"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);
    interop.import_uses(&uses);

    let constant = interop
        .constant(None, &ident("DEFAULT_LIMIT"))
        .expect("expected imported constant");
    assert_eq!(constant.ty, u64_type());
    assert_eq!(constant.rust_path.as_ref(), "::demo::DEFAULT_LIMIT");
}

#[test]
fn rustdoc_preserves_same_named_types_from_different_modules() {
    let json = json!({
        "index": {
            "0": public_item_at_path("http_error", "Error", &["demo", "http", "Error"], json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": public_item_at_path("db_error", "Error", &["demo", "db", "Error"], json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            }))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let mut error_paths = interop
        .types
        .iter()
        .filter(|ty| ty.name.as_str() == "Error")
        .map(|ty| ty.rust_path.as_ref())
        .collect::<Vec<_>>();
    error_paths.sort();
    assert_eq!(
        error_paths,
        vec!["::demo::db::Error", "::demo::http::Error"]
    );

    assert_eq!(
        interop
            .type_by_qualified_path(&["demo", "http", "Error"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::demo::http::Error")
    );
    assert_eq!(
        interop
            .type_by_qualified_path(&["demo", "db", "Error"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::demo::db::Error")
    );
}

#[test]
fn rustdoc_preserves_generic_resolved_paths() {
    let mut interop = RustInterop::empty();
    let ty = interop
        .type_from_json(
            "axum",
            &resolved("Json", vec![resolved("Vec", vec![primitive("u64")])]),
        )
        .unwrap();

    let TypeElement::Parametric(parametric) = ty else {
        panic!("expected Json<T>, got {ty:?}");
    };
    assert_eq!(parametric.base_type.as_str(), "Json");
    assert_eq!(parametric.type_args.len(), 1);
    assert!(matches!(parametric.type_args[0], TypeElement::Array(_)));
}

#[test]
fn rustdoc_lifts_resolved_string_as_builtin_string() {
    let mut interop = RustInterop::empty();
    let ty = interop
        .type_from_json(
            "std",
            &resolved_with_path("String", &["alloc", "string", "String"], vec![]),
        )
        .unwrap();

    assert_eq!(ty, string_type());
    assert!(interop.types.iter().all(|ty| ty.name.as_str() != "String"));
}

#[test]
fn rustdoc_preserves_qualified_paths_for_referenced_types() {
    let mut interop = RustInterop::empty();
    let ty = interop
        .type_from_json(
            "axum",
            &resolved_with_path(
                "Json",
                &["axum", "response", "Json"],
                vec![primitive("str")],
            ),
        )
        .unwrap();

    let TypeElement::Parametric(parametric) = ty else {
        panic!("expected Json<T>, got {ty:?}");
    };
    assert_eq!(parametric.base_type.as_str(), "Json");
    assert_eq!(parametric.type_args, vec![string_type()]);
    assert_eq!(
        interop
            .type_by_qualified_path(&["axum", "response", "Json"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::axum::response::Json")
    );
}

#[test]
fn rustdoc_preserves_same_named_referenced_types_from_different_modules() {
    let mut interop = RustInterop::empty();
    interop.type_from_json(
        "demo",
        &resolved_with_path("Error", &["demo", "http", "Error"], vec![]),
    );
    interop.type_from_json(
        "demo",
        &resolved_with_path("Error", &["demo", "db", "Error"], vec![]),
    );

    let mut error_paths = interop
        .types
        .iter()
        .filter(|ty| ty.name.as_str() == "Error")
        .map(|ty| ty.rust_path.as_ref())
        .collect::<Vec<_>>();
    error_paths.sort();
    assert_eq!(
        error_paths,
        vec!["::demo::db::Error", "::demo::http::Error"]
    );
}

#[test]
fn rustdoc_lifts_common_collections_and_results() {
    let mut interop = RustInterop::empty();

    let optional = interop
        .type_from_json("std", &resolved("Option", vec![primitive("u64")]))
        .unwrap();
    let TypeElement::Optional(optional) = optional else {
        panic!("expected optional, got {optional:?}");
    };
    assert_eq!(optional.inner, u64_type());

    let map = interop
        .type_from_json(
            "std",
            &resolved("HashMap", vec![primitive("str"), primitive("u64")]),
        )
        .unwrap();
    let TypeElement::Dictionary(map) = map else {
        panic!("expected dictionary, got {map:?}");
    };
    assert_eq!(map.key, string_type());
    assert_eq!(map.value, u64_type());

    let ordered_map = interop
        .type_from_json(
            "std",
            &resolved("BTreeMap", vec![primitive("str"), primitive("u64")]),
        )
        .unwrap();
    let TypeElement::OrderedDictionary(ordered_map) = ordered_map else {
        panic!("expected ordered dictionary, got {ordered_map:?}");
    };
    assert_eq!(ordered_map.key, string_type());
    assert_eq!(ordered_map.value, u64_type());

    let set = interop
        .type_from_json("std", &resolved("BTreeSet", vec![primitive("str")]))
        .unwrap();
    let TypeElement::Set(set) = set else {
        panic!("expected set, got {set:?}");
    };
    assert_eq!(set.elements, string_type());

    let deque = interop
        .type_from_json("std", &resolved("VecDeque", vec![primitive("u64")]))
        .unwrap();
    let TypeElement::Array(deque) = deque else {
        panic!("expected array, got {deque:?}");
    };
    assert_eq!(deque.elements, u64_type());

    let result = interop
        .type_from_json(
            "serde_json",
            &resolved(
                "Result",
                vec![
                    resolved("Vec", vec![primitive("u8")]),
                    resolved("Error", vec![]),
                ],
            ),
        )
        .unwrap();
    let TypeElement::Result(result) = result else {
        panic!("expected result, got {result:?}");
    };
    assert!(matches!(result.success, TypeElement::Array(_)));
    assert_eq!(result.error, Some(plain_type(TypeIdent::new("Error"))));
}

#[test]
fn rustdoc_lifts_flexible_result_types() {
    let mut interop = RustInterop::empty();

    for ty in [
        resolved_with_path(
            "FlexResult",
            &["galvan", "std", "FlexResult"],
            vec![primitive("u64")],
        ),
        resolved_with_path("Result", &["anyhow", "Result"], vec![primitive("u64")]),
    ] {
        let result = interop.type_from_json("demo", &ty).unwrap();
        let TypeElement::Result(result) = result else {
            panic!("expected flexible result, got {result:?}");
        };
        assert_eq!(result.success, u64_type());
        assert_eq!(result.error, None);
    }

    let unresolved_result = interop
        .type_from_json("demo", &resolved("Result", vec![primitive("u64")]))
        .unwrap();
    let TypeElement::Result(unresolved_result) = unresolved_result else {
        panic!("expected result, got {unresolved_result:?}");
    };
    assert_eq!(
        unresolved_result.error,
        Some(plain_type(TypeIdent::new("__UnknownRustError")))
    );
}

#[test]
fn rustdoc_lifts_slice_and_array_types() {
    let mut interop = RustInterop::empty();

    let slice = interop
        .type_from_json("std", &slice(primitive("u64")))
        .unwrap();
    let TypeElement::Array(slice) = slice else {
        panic!("expected slice to lift as array, got {slice:?}");
    };
    assert_eq!(slice.elements, u64_type());

    let array = interop
        .type_from_json("std", &array(primitive("str")))
        .unwrap();
    let TypeElement::Array(array) = array else {
        panic!("expected fixed array to lift as array, got {array:?}");
    };
    assert_eq!(array.elements, string_type());
}

#[test]
fn rustdoc_does_not_lift_raw_pointer_types() {
    let mut interop = RustInterop::empty();

    assert!(interop
        .type_from_json("std", &raw_pointer(primitive("u8"), false))
        .is_none());
}

#[test]
fn rustdoc_does_not_lift_unrepresentable_type_shapes() {
    let mut interop = RustInterop::empty();

    assert!(interop
        .type_from_json("demo", &qualified_path("Output", generic("T")))
        .is_none());
    assert!(interop.type_from_json("demo", &dyn_trait()).is_none());
    assert!(interop.type_from_json("demo", &impl_trait()).is_none());
}

#[test]
fn rustdoc_lifts_function_pointer_types() {
    let mut interop = RustInterop::empty();
    let ty = interop
        .type_from_json(
            "std",
            &function_pointer(
                vec![json!(["ticket_id", primitive("u64")]), primitive("str")],
                primitive("bool"),
            ),
        )
        .unwrap();

    let TypeElement::Closure(closure) = ty else {
        panic!("expected function pointer to lift as closure, got {ty:?}");
    };
    assert_eq!(closure.parameters, vec![u64_type(), string_type()]);
    assert_eq!(closure.return_ty, TypeElement::bool());
}

#[test]
fn rustdoc_lifts_never_types() {
    let mut interop = RustInterop::empty();

    let ty = interop.type_from_json("std", &never()).unwrap();
    assert!(matches!(ty, TypeElement::Never(_)));

    let primitive_ty = interop
        .type_from_json("std", &primitive("!"))
        .expect("expected primitive never type to lift");
    assert!(matches!(primitive_ty, TypeElement::Never(_)));
}

#[test]
fn rustdoc_lifts_arc_shared_wrappers_to_ref_parameters() {
    let mut interop = RustInterop::empty();
    for wrapper in [
        resolved("Arc", vec![resolved("Mutex", vec![generic("T")])]),
        resolved("Arc", vec![resolved("RwLock", vec![generic("T")])]),
    ] {
        let param = interop
            .param_from_json("std", &json!(["tickets", wrapper]))
            .unwrap();

        assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Ref));
        assert_eq!(param.param_type, generic_type("T"));
    }
}

#[test]
fn rustdoc_keeps_single_owner_shared_wrappers_nominal() {
    let mut interop = RustInterop::empty();
    for name in ["Mutex", "RwLock"] {
        let param = interop
            .param_from_json(
                "std",
                &json!(["tickets", resolved(name, vec![generic("T")])]),
            )
            .unwrap();

        assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Move));
        let TypeElement::Parametric(parametric) = param.param_type else {
            panic!("expected parametric {name}<T>");
        };
        assert_eq!(parametric.base_type, TypeIdent::new(name));
        assert_eq!(parametric.type_args, vec![generic_type("T")]);
    }

    let atomic = interop
        .param_from_json("std", &json!(["next_id", resolved("AtomicU64", vec![])]))
        .unwrap();
    assert_eq!(atomic.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(atomic.param_type, plain_type(TypeIdent::new("AtomicU64")));
}

#[test]
fn rustdoc_lifts_arc_atomic_primitives_to_ref_parameters() {
    let mut interop = RustInterop::empty();
    let param = interop
        .param_from_json(
            "std",
            &json!([
                "next_id",
                resolved("Arc", vec![resolved("AtomicU64", vec![])])
            ]),
        )
        .unwrap();

    assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Ref));
    assert_eq!(param.param_type, u64_type());
}

#[test]
fn rustdoc_lifts_mutable_borrowed_parameters_to_mut() {
    let mut interop = RustInterop::empty();
    let param = interop
        .param_from_json(
            "demo",
            &json!(["ticket", mut_borrowed(resolved("Ticket", vec![]))]),
        )
        .unwrap();

    assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Mut));
    assert_eq!(param.param_type, plain_type(TypeIdent::new("Ticket")));
}

#[test]
fn rustdoc_keeps_owned_copy_parameters_unmodified() {
    let mut interop = RustInterop::empty();
    let integer = interop
        .param_from_json("std", &json!(["limit", primitive("u64")]))
        .unwrap();
    assert_eq!(integer.decl_modifier, None);
    assert_eq!(integer.param_type, u64_type());

    let tuple = interop
        .param_from_json(
            "std",
            &json!(["range", { "tuple": [primitive("u64"), primitive("bool")] }]),
        )
        .unwrap();
    assert_eq!(tuple.decl_modifier, None);
    assert!(matches!(tuple.param_type, TypeElement::Tuple(_)));
}

#[test]
fn rustdoc_marks_owned_non_copy_parameters_as_move() {
    let mut interop = RustInterop::empty();
    let string = interop
        .param_from_json("std", &json!(["title", primitive("str")]))
        .unwrap();
    assert_eq!(string.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(string.param_type, string_type());

    let generic = interop
        .param_from_json("demo", &json!(["value", generic("T")]))
        .unwrap();
    assert_eq!(generic.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(generic.param_type, generic_type("T"));
}

#[test]
fn rustdoc_preserves_shared_borrow_parameter_conversions() {
    let json = json!({
        "index": {
            "0": public_function(
                "takes_ref",
                vec![json!(["value", borrowed(slice(primitive("u64")))])],
                primitive("bool")
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("takes_ref"), &[])
        .expect("expected imported function");
    assert_eq!(
        function.arg_conversions,
        vec![RustArgConversion::SharedBorrow]
    );
    assert_eq!(
        function.decl.item.signature.parameters.params[0].param_type,
        TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
            elements: u64_type(),
            span: Span::default()
        }))
    );
}

#[test]
fn rustdoc_does_not_import_unsafe_functions() {
    let json = json!({
        "index": {
            "0": public_unsafe_function("from_raw_parts", vec![], primitive("u64"))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .function(Some("demo"), None, &ident("from_raw_parts"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_functions_with_raw_pointer_signatures() {
    let json = json!({
        "index": {
            "0": public_function(
                "read_address",
                vec![json!(["bytes", raw_pointer(primitive("u8"), false)])],
                primitive("u64")
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .function(Some("demo"), None, &ident("read_address"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_functions_with_unliftable_signatures() {
    let json = json!({
        "index": {
            "0": public_function(
                "visit",
                vec![json!(["visitor", dyn_trait()])],
                qualified_path("Output", generic("V"))
            ),
            "1": public_function(
                "make_display",
                vec![],
                impl_trait()
            ),
            "2": public_constant(
                "DEFAULT_OUTPUT",
                qualified_path("Output", generic("V"))
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .function(Some("demo"), None, &ident("visit"), &[])
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("make_display"), &[])
        .is_none());
    assert!(interop
        .constant(Some("demo"), &ident("DEFAULT_OUTPUT"))
        .is_none());
}

#[test]
fn rustdoc_imports_function_pointer_parameters() {
    let json = json!({
        "index": {
            "0": public_function(
                "filter_tickets",
                vec![json!([
                    "predicate",
                    function_pointer(vec![primitive("u64")], primitive("bool"))
                ])],
                primitive("u64")
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("filter_tickets"), &[])
        .expect("expected imported function");
    let parameter = &function.decl.item.signature.parameters.params[0];
    let TypeElement::Closure(closure) = &parameter.param_type else {
        panic!(
            "expected closure parameter type, got {:?}",
            parameter.param_type
        );
    };
    assert_eq!(closure.parameters, vec![u64_type()]);
    assert_eq!(closure.return_ty, TypeElement::bool());
}

#[test]
fn rustdoc_keeps_types_with_raw_pointer_fields_opaque() {
    let json = json!({
        "index": {
            "0": public_item("Buffer", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1"]
                }
            })),
            "1": public_field("ptr", raw_pointer(primitive("u8"), false))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Empty(buffer) = imported_type(&interop, "Buffer") else {
        panic!("expected Buffer to import as an opaque type");
    };
    assert_eq!(buffer.ident, TypeIdent::new("Buffer"));
}

#[test]
fn rustdoc_keeps_types_with_unliftable_fields_opaque() {
    let json = json!({
        "index": {
            "0": public_item("VisitResult", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1"]
                }
            })),
            "1": public_field("output", qualified_path("Output", generic("V")))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Empty(visit_result) = imported_type(&interop, "VisitResult") else {
        panic!("expected VisitResult to import as an opaque type");
    };
    assert_eq!(visit_result.ident, TypeIdent::new("VisitResult"));
}

#[test]
fn rustdoc_preserves_generic_params_on_opaque_types() {
    let json = json!({
        "index": {
            "0": public_item("State", json!({
                "struct": {
                    "kind": "unit",
                    "generics": type_generics(vec![
                        lifetime_param("'a"),
                        generic_param("T"),
                    ])
                }
            }))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Empty(state) = imported_type(&interop, "State") else {
        panic!("expected opaque State type");
    };
    assert_eq!(state.generic_params, vec![Ident::new("T")]);
}

#[test]
fn rustdoc_preserves_generic_params_on_structs() {
    let json = json!({
        "index": {
            "0": public_item("Page", json!({
                "struct": {
                    "kind": "plain",
                    "generics": type_generics(vec![generic_param("T")]),
                    "fields": ["1"]
                }
            })),
            "1": public_field("items", resolved("Vec", vec![generic("T")]))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Struct(page) = imported_type(&interop, "Page") else {
        panic!("expected Page struct");
    };
    assert_eq!(page.generic_params, vec![Ident::new("T")]);
    assert_eq!(
        page.members[0].r#type,
        TypeElement::Array(Box::new(galvan_ast::ArrayTypeItem {
            elements: generic_type("T"),
            span: Span::default()
        }))
    );
}

#[test]
fn rustdoc_imports_never_returning_functions() {
    let json = json!({
        "index": {
            "0": public_function("abort_request", vec![], never())
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("abort_request"), &[])
        .expect("expected imported function");
    assert!(matches!(
        function.decl.item.signature.return_type,
        TypeElement::Never(_)
    ));
}

#[test]
fn rustdoc_lifts_owned_wrapper_parameters_with_call_conversions() {
    let json = json!({
        "index": {
            "0": public_function(
                "takes_box",
                vec![json!(["value", resolved("Box", vec![primitive("u64")])])],
                primitive("bool")
            ),
            "1": public_function(
                "takes_rc",
                vec![json!(["value", resolved("Rc", vec![resolved("Ticket", vec![])])])],
                primitive("bool")
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let takes_box = interop
        .function(Some("demo"), None, &ident("takes_box"), &[])
        .expect("expected imported Box function");
    assert_eq!(takes_box.arg_conversions, vec![RustArgConversion::BoxNew]);
    assert_eq!(
        takes_box.decl.item.signature.parameters.params[0].param_type,
        u64_type()
    );

    let takes_rc = interop
        .function(Some("demo"), None, &ident("takes_rc"), &[])
        .expect("expected imported Rc function");
    assert_eq!(takes_rc.arg_conversions, vec![RustArgConversion::RcNew]);
    assert_eq!(
        takes_rc.decl.item.signature.parameters.params[0].param_type,
        plain_type(TypeIdent::new("Ticket"))
    );
}

#[test]
fn rustdoc_lifts_box_returns_with_return_conversions() {
    let json = json!({
        "index": {
            "0": public_function(
                "boxed_ticket",
                vec![],
                resolved("Box", vec![resolved("Ticket", vec![])])
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("boxed_ticket"), &[])
        .expect("expected imported Box return function");
    assert_eq!(function.return_conversion, RustReturnConversion::BoxDeref);
    assert_eq!(
        function.decl.item.signature.return_type,
        plain_type(TypeIdent::new("Ticket"))
    );
}

#[test]
fn rustdoc_lifts_rc_returns_with_clone_return_conversions() {
    let json = json!({
        "index": {
            "0": public_function(
                "shared_ticket",
                vec![],
                resolved("Rc", vec![resolved("Ticket", vec![])])
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("shared_ticket"), &[])
        .expect("expected imported Rc return function");
    assert_eq!(
        function.return_conversion,
        RustReturnConversion::RcCloneDeref
    );
    assert_eq!(
        function.decl.item.signature.return_type,
        plain_type(TypeIdent::new("Ticket"))
    );
}

#[test]
fn rustdoc_lifts_box_struct_fields_with_field_conversions() {
    let json = json!({
        "index": {
            "0": public_item("TicketEnvelope", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1"]
                }
            })),
            "1": public_field("ticket", resolved("Box", vec![resolved("Ticket", vec![])]))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Struct(envelope) = imported_type(&interop, "TicketEnvelope") else {
        panic!("expected TicketEnvelope struct");
    };
    assert_eq!(envelope.members.len(), 1);
    assert_eq!(envelope.members[0].ident.as_str(), "ticket");
    assert_eq!(
        envelope.members[0].r#type,
        plain_type(TypeIdent::new("Ticket"))
    );
    assert_eq!(
        interop.field_return_conversion(&TypeIdent::new("TicketEnvelope"), &ident("ticket")),
        RustReturnConversion::BoxDeref
    );
    assert_eq!(
        interop.field_arg_conversion(&TypeIdent::new("TicketEnvelope"), &ident("ticket")),
        RustArgConversion::BoxNew
    );
}

#[test]
fn rustdoc_lifts_rc_struct_fields_with_field_conversions() {
    let json = json!({
        "index": {
            "0": public_item("TicketCache", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1"]
                }
            })),
            "1": public_field("latest", resolved("Rc", vec![resolved("Ticket", vec![])]))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Struct(cache) = imported_type(&interop, "TicketCache") else {
        panic!("expected TicketCache struct");
    };
    assert_eq!(cache.members.len(), 1);
    assert_eq!(cache.members[0].ident.as_str(), "latest");
    assert_eq!(
        cache.members[0].r#type,
        plain_type(TypeIdent::new("Ticket"))
    );
    assert_eq!(
        interop.field_return_conversion(&TypeIdent::new("TicketCache"), &ident("latest")),
        RustReturnConversion::RcCloneDeref
    );
    assert_eq!(
        interop.field_arg_conversion(&TypeIdent::new("TicketCache"), &ident("latest")),
        RustArgConversion::RcNew
    );
}

#[test]
fn rustdoc_imports_public_struct_fields() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1", "2", "3"]
                }
            })),
            "1": public_field("id", primitive("u64")),
            "2": public_field("title", primitive("str")),
            "3": public_field(
                "state",
                resolved("Arc", vec![resolved("RwLock", vec![resolved("TicketState", vec![])])])
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Struct(ticket) = imported_type(&interop, "Ticket") else {
        panic!("expected Ticket struct");
    };
    assert_eq!(ticket.ident.as_str(), "Ticket");
    assert_eq!(ticket.members.len(), 3);
    assert_eq!(ticket.members[0].ident.as_str(), "id");
    assert_eq!(ticket.members[0].r#type, u64_type());
    assert_eq!(ticket.members[1].ident.as_str(), "title");
    assert_eq!(ticket.members[1].r#type, string_type());
    assert_eq!(
        ticket.members[2].decl_modifier,
        Some(galvan_ast::DeclModifier::Ref)
    );
    assert_eq!(
        ticket.members[2].r#type,
        plain_type(TypeIdent::new("TicketState"))
    );
}

#[test]
fn rustdoc_imports_tuple_struct_fields() {
    let json = json!({
        "index": {
            "0": public_item("UserId", json!({
                "struct": {
                    "kind": "tuple",
                    "fields": ["1"]
                }
            })),
            "1": public_field("0", primitive("u64"))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Tuple(user_id) = imported_type(&interop, "UserId") else {
        panic!("expected UserId tuple struct");
    };
    assert_eq!(user_id.ident.as_str(), "UserId");
    assert_eq!(user_id.members.len(), 1);
    assert_eq!(user_id.members[0].r#type, u64_type());
}

#[test]
fn rustdoc_lifts_tuple_struct_wrapper_fields() {
    let json = json!({
        "index": {
            "0": public_item("SharedTicket", json!({
                "struct": {
                    "kind": "tuple",
                    "fields": ["1", "2"]
                }
            })),
            "1": public_field("0", resolved("Box", vec![resolved("Ticket", vec![])])),
            "2": public_field(
                "1",
                resolved("Arc", vec![resolved("Mutex", vec![resolved("TicketState", vec![])])])
            )
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Tuple(shared_ticket) = imported_type(&interop, "SharedTicket") else {
        panic!("expected SharedTicket tuple struct");
    };
    assert_eq!(shared_ticket.members.len(), 2);
    assert_eq!(
        shared_ticket.members[0].r#type,
        plain_type(TypeIdent::new("Ticket"))
    );
    assert_eq!(
        shared_ticket.members[1].r#type,
        plain_type(TypeIdent::new("TicketState"))
    );
    assert_eq!(
        interop.constructor_arg_conversions(&TypeIdent::new("SharedTicket")),
        vec![RustArgConversion::BoxNew, RustArgConversion::None]
    );
}

#[test]
fn rustdoc_imports_enum_variants() {
    let json = json!({
        "index": {
            "0": public_item("TicketEvent", json!({
                "enum": {
                    "variants": ["1", "2", "4"]
                }
            })),
            "1": public_item("Created", json!({
                "variant": {
                    "kind": "plain"
                }
            })),
            "2": public_item("Renamed", json!({
                "variant": {
                    "kind": {
                        "tuple": {
                            "fields": ["3"]
                        }
                    }
                }
            })),
            "3": public_field("0", primitive("str")),
            "4": public_item("Closed", json!({
                "variant": {
                    "kind": {
                        "struct": {
                            "fields": ["5"]
                        }
                    }
                }
            })),
            "5": public_field("reason", resolved("Option", vec![primitive("str")]))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Enum(event) = imported_type(&interop, "TicketEvent") else {
        panic!("expected TicketEvent enum");
    };
    assert_eq!(event.ident.as_str(), "TicketEvent");
    assert_eq!(event.members.len(), 3);
    assert_eq!(event.members[0].ident.as_str(), "Created");
    assert!(event.members[0].fields.is_empty());
    assert_eq!(event.members[1].ident.as_str(), "Renamed");
    assert_eq!(event.members[1].fields[0].name, None);
    assert_eq!(event.members[1].fields[0].r#type, string_type());
    assert_eq!(event.members[2].ident.as_str(), "Closed");
    assert_eq!(event.members[2].fields[0].name, Some(Ident::new("reason")));
    assert!(matches!(
        event.members[2].fields[0].r#type,
        TypeElement::Optional(_)
    ));
}

#[test]
fn rustdoc_lifts_enum_variant_wrapper_fields() {
    let json = json!({
        "index": {
            "0": public_item("TicketEvent", json!({
                "enum": {
                    "variants": ["1", "3"]
                }
            })),
            "1": public_item("Assigned", json!({
                "variant": {
                    "kind": {
                        "tuple": {
                            "fields": ["2"]
                        }
                    }
                }
            })),
            "2": public_field("0", resolved("Rc", vec![resolved("User", vec![])])),
            "3": public_item("Moved", json!({
                "variant": {
                    "kind": {
                        "struct": {
                            "fields": ["4", "5"]
                        }
                    }
                }
            })),
            "4": public_field("queue", resolved("Option", vec![primitive("str")])),
            "5": public_field("owner", resolved("Box", vec![resolved("User", vec![])]))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Enum(event) = imported_type(&interop, "TicketEvent") else {
        panic!("expected TicketEvent enum");
    };
    assert_eq!(event.members.len(), 2);
    assert_eq!(
        event.members[0].fields[0].r#type,
        plain_type(TypeIdent::new("User"))
    );
    assert_eq!(event.members[1].fields[0].name, Some(Ident::new("queue")));
    assert!(matches!(
        event.members[1].fields[0].r#type,
        TypeElement::Optional(_)
    ));
    assert_eq!(event.members[1].fields[1].name, Some(Ident::new("owner")));
    assert_eq!(
        event.members[1].fields[1].r#type,
        plain_type(TypeIdent::new("User"))
    );
    assert_eq!(
        interop.enum_variant_arg_conversion(
            &TypeIdent::new("TicketEvent"),
            &TypeIdent::new("Assigned"),
            0,
            None,
        ),
        RustArgConversion::RcNew
    );
    assert_eq!(
        interop.enum_variant_return_conversion(
            &TypeIdent::new("TicketEvent"),
            &TypeIdent::new("Assigned"),
            0,
            None,
        ),
        RustReturnConversion::RcCloneDeref
    );
    assert_eq!(
        interop.enum_variant_arg_conversion(
            &TypeIdent::new("TicketEvent"),
            &TypeIdent::new("Moved"),
            1,
            Some(&ident("owner")),
        ),
        RustArgConversion::BoxNew
    );
    assert_eq!(
        interop.enum_variant_return_conversion(
            &TypeIdent::new("TicketEvent"),
            &TypeIdent::new("Moved"),
            1,
            Some(&ident("owner")),
        ),
        RustReturnConversion::BoxDeref
    );
}

#[test]
fn rustdoc_imports_type_aliases_with_lifted_targets() {
    let json = json!({
        "index": {
            "0": public_item("UserId", json!({
                "type_alias": primitive("u64")
            })),
            "1": public_item("Names", json!({
                "type_alias": resolved("Vec", vec![primitive("str")])
            })),
            "2": public_item("ParseResult", json!({
                "type_alias": {
                    "type": resolved(
                        "Result",
                        vec![generic("T"), resolved("Error", vec![])]
                    ),
                    "generics": type_generics(vec![generic_param("T")])
                }
            })),
            "3": public_item("FallibleTicket", json!({
                "type_alias": {
                    "type": resolved_with_path(
                        "FlexResult",
                        &["galvan", "std", "FlexResult"],
                        vec![resolved("Ticket", vec![])]
                    )
                }
            }))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let TypeDecl::Alias(user_id) = imported_type(&interop, "UserId") else {
        panic!("expected UserId alias");
    };
    assert_eq!(user_id.r#type, u64_type());

    let TypeDecl::Alias(names) = imported_type(&interop, "Names") else {
        panic!("expected Names alias");
    };
    let TypeElement::Array(names) = &names.r#type else {
        panic!("expected lifted Vec alias, got {:?}", names.r#type);
    };
    assert_eq!(names.elements, string_type());

    let TypeDecl::Alias(parse_result) = imported_type(&interop, "ParseResult") else {
        panic!("expected ParseResult alias");
    };
    assert_eq!(parse_result.generic_params, vec![Ident::new("T")]);
    let TypeElement::Result(parse_result) = &parse_result.r#type else {
        panic!(
            "expected lifted Result alias, got {:?}",
            parse_result.r#type
        );
    };
    assert_eq!(parse_result.success, generic_type("T"));
    assert_eq!(
        parse_result.error,
        Some(plain_type(TypeIdent::new("Error")))
    );

    let TypeDecl::Alias(fallible_ticket) = imported_type(&interop, "FallibleTicket") else {
        panic!("expected FallibleTicket alias");
    };
    let TypeElement::Result(fallible_ticket) = &fallible_ticket.r#type else {
        panic!(
            "expected lifted flexible result alias, got {:?}",
            fallible_ticket.r#type
        );
    };
    assert_eq!(
        fallible_ticket.success,
        plain_type(TypeIdent::new("Ticket"))
    );
    assert_eq!(fallible_ticket.error, None);
}

#[test]
fn rustdoc_imports_top_level_constants() {
    let json = json!({
        "index": {
            "0": public_constant("DEFAULT_LIMIT", primitive("u64"))
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let constant = interop
        .constant(Some("demo"), &ident("DEFAULT_LIMIT"))
        .expect("expected namespaced constant");
    assert_eq!(constant.ty, u64_type());
    assert_eq!(constant.rust_path.as_ref(), "::demo::DEFAULT_LIMIT");
    assert!(interop.constant(None, &ident("DEFAULT_LIMIT")).is_none());
}

#[test]
fn rustdoc_imports_associated_constants() {
    let json = json!({
        "index": {
            "0": public_item("StatusCode", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": {
                "id": "1",
                "name": null,
                "visibility": "public",
                "inner": {
                    "impl": {
                        "for": resolved("StatusCode", vec![]),
                        "trait": null,
                        "items": ["2"]
                    }
                }
            },
            "2": {
                "id": "2",
                "name": "CREATED",
                "visibility": "public",
                "path": ["demo", "StatusCode"],
                "inner": {
                    "assoc_const": {
                        "type": resolved("StatusCode", vec![])
                    }
                }
            }
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop.constant(Some("demo"), &ident("CREATED")).is_none());
    let constant = interop
        .associated_constant(
            Some("demo"),
            &TypeIdent::new("StatusCode"),
            &ident("CREATED"),
        )
        .expect("expected associated constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::StatusCode::CREATED");
    assert_eq!(constant.ty, plain_type(TypeIdent::new("StatusCode")));
}

#[test]
fn rustdoc_imports_reexported_type_aliases() {
    let json = json!({
        "index": {
            "0": public_item("OriginalTicket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["1"]
                }
            })),
            "1": public_field("title", primitive("str")),
            "2": public_use("2", "Ticket", "0")
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let imported = interop
        .types
        .iter()
        .find(|ty| ty.name.as_str() == "Ticket")
        .expect("expected re-exported type alias");
    assert_eq!(imported.rust_path.as_ref(), "::demo::Ticket");
    let TypeDecl::Struct(ticket) = &imported.decl.item else {
        panic!("expected re-exported struct type");
    };
    assert_eq!(ticket.ident, TypeIdent::new("Ticket"));
    assert_eq!(ticket.members[0].ident, ident("title"));
}

#[test]
fn rustdoc_imports_external_reexported_types_without_index_targets() {
    let json = json!({
        "index": {
            "0": public_external_use("0", "StatusCode", "http::StatusCode"),
            "1": public_external_use("1", "DEFAULT_LIMIT", "http::DEFAULT_LIMIT")
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let imported = interop
        .types
        .iter()
        .find(|ty| ty.name.as_str() == "StatusCode")
        .expect("expected external re-exported type");
    assert_eq!(imported.rust_path.as_ref(), "::http::StatusCode");
    let TypeDecl::Empty(status_code) = &imported.decl.item else {
        panic!("expected external re-export to import as empty type");
    };
    assert_eq!(status_code.ident, TypeIdent::new("StatusCode"));
    assert!(interop
        .types
        .iter()
        .all(|ty| ty.name.as_str() != "DEFAULT_LIMIT"));
}

#[test]
fn rustdoc_imports_reexported_functions() {
    let json = json!({
        "index": {
            "0": public_function("nickname", vec![], primitive("str")),
            "1": public_use("1", "display_name", "0")
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(Some("demo"), None, &ident("display_name"), &[])
        .expect("expected re-exported function");
    assert_eq!(function.rust_path.as_ref(), "::demo::display_name");
    assert_eq!(
        function.decl.item.signature.identifier,
        ident("display_name")
    );
    assert_eq!(function.decl.item.signature.return_type, string_type());
}

#[test]
fn rustdoc_imports_reexported_constants() {
    let json = json!({
        "index": {
            "0": public_constant("DEFAULT_LIMIT", primitive("u64")),
            "1": public_use("1", "LIMIT", "0")
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let constant = interop
        .constant(Some("demo"), &ident("LIMIT"))
        .expect("expected re-exported constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::LIMIT");
    assert_eq!(constant.ty, u64_type());
}

#[test]
fn rustdoc_imports_glob_reexported_items() {
    let json = json!({
        "index": {
            "0": public_module("internal", vec!["1", "2", "4"]),
            "1": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": ["3"]
                }
            })),
            "2": public_function("display_name", vec![], primitive("str")),
            "3": public_field("title", primitive("str")),
            "4": public_constant("LIMIT", primitive("u64")),
            "5": public_glob_use("5", "internal", "0")
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let imported = interop
        .types
        .iter()
        .find(|ty| ty.name.as_str() == "Ticket")
        .expect("expected glob re-exported type");
    assert_eq!(imported.rust_path.as_ref(), "::demo::Ticket");

    let function = interop
        .function(Some("demo"), None, &ident("display_name"), &[])
        .expect("expected glob re-exported function");
    assert_eq!(function.rust_path.as_ref(), "::demo::display_name");

    let constant = interop
        .constant(Some("demo"), &ident("LIMIT"))
        .expect("expected glob re-exported constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::LIMIT");
    assert_eq!(constant.ty, u64_type());
}

#[test]
fn rustdoc_imports_inherent_impl_methods_with_receivers() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": {
                "id": "1",
                "name": null,
                "visibility": "public",
                "inner": {
                    "impl": {
                        "for": resolved("Ticket", vec![]),
                        "trait": null,
                        "items": ["2"]
                    }
                }
            },
            "2": {
                "id": "2",
                "name": "rename",
                "visibility": "public",
                "path": ["demo", "Ticket"],
                "inner": {
                    "function": {
                        "sig": {
                            "inputs": [
                                ["self", mut_borrowed(resolved("Ticket", vec![]))],
                                ["title", primitive("str")]
                            ],
                            "output": null
                        }
                    }
                }
            }
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .function(Some("demo"), None, &ident("rename"), &[])
        .is_none());
    let function = interop
        .function(
            Some("demo"),
            Some(&TypeIdent::new("Ticket")),
            &ident("rename"),
            &[],
        )
        .expect("expected imported Ticket.rename method");
    assert_eq!(function.rust_path.as_ref(), "::demo::Ticket::rename");
    let receiver = function.decl.item.signature.receiver().unwrap();
    assert_eq!(receiver.decl_modifier, Some(galvan_ast::DeclModifier::Mut));
    assert_eq!(receiver.param_type, plain_type(TypeIdent::new("Ticket")));
}

#[test]
fn rustdoc_imports_inherent_associated_functions() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": {
                "id": "1",
                "name": null,
                "visibility": "public",
                "inner": {
                    "impl": {
                        "for": resolved("Ticket", vec![]),
                        "trait": null,
                        "items": ["2"]
                    }
                }
            },
            "2": {
                "id": "2",
                "name": "new",
                "visibility": "public",
                "path": ["demo", "Ticket"],
                "inner": {
                    "function": {
                        "sig": {
                            "inputs": [
                                ["title", primitive("str")]
                            ],
                            "output": resolved("Ticket", vec![])
                        }
                    }
                }
            }
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .function(Some("demo"), None, &ident("new"), &[])
        .is_none());
    let function = interop
        .associated_function(Some("demo"), &TypeIdent::new("Ticket"), &ident("new"), &[])
        .expect("expected imported Ticket.new associated function");
    assert_eq!(function.rust_path.as_ref(), "::demo::Ticket::new");
    assert!(function.decl.item.signature.receiver().is_none());
    assert_eq!(
        function.decl.item.signature.return_type,
        plain_type(TypeIdent::new("Ticket"))
    );
}

#[test]
fn rustdoc_does_not_import_unsafe_associated_functions() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": {
                "id": "1",
                "name": null,
                "visibility": "public",
                "inner": {
                    "impl": {
                        "for": resolved("Ticket", vec![]),
                        "trait": null,
                        "items": ["2"]
                    }
                }
            },
            "2": {
                "id": "2",
                "name": "from_raw",
                "visibility": "public",
                "path": ["demo", "Ticket"],
                "inner": {
                    "function": {
                        "sig": {
                            "inputs": [
                                ["address", primitive("usize")]
                            ],
                            "output": resolved("Ticket", vec![])
                        },
                        "header": {
                            "is_unsafe": true
                        }
                    }
                }
            }
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    assert!(interop
        .associated_function(
            Some("demo"),
            &TypeIdent::new("Ticket"),
            &ident("from_raw"),
            &[]
        )
        .is_none());
}

#[test]
fn rustdoc_imports_trait_impl_methods() {
    let json = json!({
        "index": {
            "0": public_item("Ticket", json!({
                "struct": {
                    "kind": "plain",
                    "fields": []
                }
            })),
            "1": public_item("DisplayName", json!({
                "trait": {
                    "items": []
                }
            })),
            "2": {
                "id": "2",
                "name": null,
                "visibility": "public",
                "inner": {
                    "impl": {
                        "for": resolved_with_path("Ticket", &["demo", "Ticket"], vec![]),
                        "trait": resolved_with_path("DisplayName", &["demo", "DisplayName"], vec![]),
                        "items": ["3"]
                    }
                }
            },
            "3": {
                "id": "3",
                "name": "display_name",
                "visibility": "public",
                "path": ["demo", "DisplayName"],
                "inner": {
                    "function": {
                        "sig": {
                            "inputs": [
                                ["self", {
                                    "borrowed_ref": {
                                        "type": resolved_with_path("Ticket", &["demo", "Ticket"], vec![]),
                                        "mutable": false
                                    }
                                }]
                            ],
                            "output": primitive("str")
                        }
                    }
                }
            }
        }
    });
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &json);

    let function = interop
        .function(
            Some("demo"),
            Some(&TypeIdent::new("Ticket")),
            &ident("display_name"),
            &[],
        )
        .expect("expected imported trait method");
    assert_eq!(
        function.rust_path.as_ref(),
        "<::demo::Ticket as ::demo::DisplayName>::display_name"
    );
    let receiver = function.decl.item.signature.receiver().unwrap();
    assert_eq!(receiver.decl_modifier, None);
    assert_eq!(receiver.param_type, plain_type(TypeIdent::new("Ticket")));
    assert_eq!(function.decl.item.signature.return_type, string_type());
}
