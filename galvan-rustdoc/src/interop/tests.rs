use super::*;
use serde_json::json;

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
    json!({
        "id": name,
        "name": name,
        "visibility": "public",
        "path": ["demo", name],
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
fn rustdoc_lifts_shared_wrappers_to_ref_parameters() {
    let mut interop = RustInterop::empty();
    let param = interop
        .param_from_json(
            "std",
            &json!([
                "tickets",
                resolved("Arc", vec![resolved("Mutex", vec![generic("T")])])
            ]),
        )
        .unwrap();

    assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Ref));
    assert_eq!(param.param_type, generic_type("T"));
}

#[test]
fn rustdoc_lifts_shared_atomic_primitives_to_ref_parameters() {
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
fn rustdoc_preserves_shared_borrow_parameter_conversions() {
    let json = json!({
        "index": {
            "0": public_function(
                "takes_ref",
                vec![json!(["value", borrowed(primitive("u64"))])],
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
        u64_type()
    );
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
                resolved("Arc", vec![resolved("Mutex", vec![resolved("TicketState", vec![])])])
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
                            "fields": ["4"]
                        }
                    }
                }
            })),
            "4": public_field("queue", resolved("Option", vec![primitive("str")]))
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
