use super::*;

use std::collections::HashMap;

use galvan_ast::Span;
use galvan_files::Source;
use rustdoc_types::{
    Abi, Constant, Crate, Enum, Function, FunctionHeader, FunctionPointer, FunctionSignature,
    GenericArg, GenericArgs, GenericParamDef, GenericParamDefKind, Generics, Id, Impl, Item,
    ItemEnum, ItemSummary, Module, Path, Struct, StructKind, Target, Trait, Type, TypeAlias, Union,
    Use, Variant, VariantKind, Visibility,
};

// ---------------------------------------------------------------------------
// Typed rustdoc-types builders
//
// Tests construct `rustdoc_types` values directly instead of walking untyped
// JSON. `Id`s are derived deterministically from the string keys the tests use
// so cross-references (`impl.items`, struct fields, `use` targets, …) stay
// readable, and `Crate.paths` is populated from each item's module path so the
// crate-aware resolution matches real rustdoc data.
// ---------------------------------------------------------------------------

fn fnv1a(value: &str) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in value.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Deterministic `Id` for a test's string key.
fn id(key: &str) -> Id {
    Id(fnv1a(key))
}

/// Deterministic `Id` for a referenced resolved path, namespaced away from item
/// keys so referenced types fall back to their usage path rather than colliding
/// with an item defined in the same crate.
fn resolved_id(path: &str) -> Id {
    Id(fnv1a(&format!("\u{1}resolved\u{1}{path}")))
}

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

// --- types ---

fn primitive(name: &str) -> Type {
    Type::Primitive(name.to_string())
}

fn generic(name: &str) -> Type {
    Type::Generic(name.to_string())
}

fn never() -> Type {
    // rustdoc v58 encodes the never type `!` as a primitive.
    Type::Primitive("!".to_string())
}

/// Canonical fully-qualified path for a bare wrapper name, so standard-library
/// wrappers resolve to `std`/`core`/`alloc`; unknown names stay unqualified.
fn canonical_path(name: &str) -> &str {
    match name {
        "Option" => "core::option::Option",
        "Result" => "core::result::Result",
        "String" => "alloc::string::String",
        "Vec" => "alloc::vec::Vec",
        "VecDeque" => "alloc::collections::vec_deque::VecDeque",
        "LinkedList" => "alloc::collections::linked_list::LinkedList",
        "HashMap" => "std::collections::hash::map::HashMap",
        "HashSet" => "std::collections::hash::set::HashSet",
        "BTreeMap" => "alloc::collections::btree::map::BTreeMap",
        "BTreeSet" => "alloc::collections::btree::set::BTreeSet",
        "Box" => "alloc::boxed::Box",
        "Rc" => "alloc::rc::Rc",
        "Arc" => "alloc::sync::Arc",
        "Mutex" => "std::sync::mutex::Mutex",
        "RwLock" => "std::sync::rwlock::RwLock",
        "AtomicU64" => "core::sync::atomic::AtomicU64",
        other => other,
    }
}

fn path_of(path: &str, args: Vec<Type>) -> Path {
    Path {
        path: path.to_string(),
        id: resolved_id(path),
        args: Some(Box::new(GenericArgs::AngleBracketed {
            args: args.into_iter().map(GenericArg::Type).collect(),
            constraints: vec![],
        })),
    }
}

fn resolved(name: &str, args: Vec<Type>) -> Type {
    Type::ResolvedPath(path_of(canonical_path(name), args))
}

fn resolved_with_path(_name: &str, path: &[&str], args: Vec<Type>) -> Type {
    Type::ResolvedPath(path_of(&path.join("::"), args))
}

fn resolved_with_string_path(path: &str, args: Vec<Type>) -> Type {
    Type::ResolvedPath(path_of(path, args))
}

/// A resolved reference whose `id` targets a specific in-crate item key, so
/// `Crate.index`/`paths` lookups resolve to it (unlike [`resolved`], which
/// namespaces its id away).
fn resolved_to(key: &str, name: &str, args: Vec<Type>) -> Type {
    Type::ResolvedPath(Path {
        path: name.to_string(),
        id: id(key),
        args: Some(Box::new(GenericArgs::AngleBracketed {
            args: args.into_iter().map(GenericArg::Type).collect(),
            constraints: vec![],
        })),
    })
}

fn slice(ty: Type) -> Type {
    Type::Slice(Box::new(ty))
}

fn array(ty: Type) -> Type {
    Type::Array {
        type_: Box::new(ty),
        len: "3".to_string(),
    }
}

fn tuple_(elements: Vec<Type>) -> Type {
    Type::Tuple(elements)
}

fn raw_pointer(ty: Type, mutable: bool) -> Type {
    Type::RawPointer {
        is_mutable: mutable,
        type_: Box::new(ty),
    }
}

fn qualified_path(name: &str, self_type: Type) -> Type {
    Type::QualifiedPath {
        name: name.to_string(),
        args: None,
        self_type: Box::new(self_type),
        trait_: Some(path_of("demo::Visitor", vec![])),
    }
}

fn dyn_trait() -> Type {
    Type::DynTrait(rustdoc_types::DynTrait {
        traits: vec![],
        lifetime: None,
    })
}

fn impl_trait() -> Type {
    Type::ImplTrait(vec![])
}

fn function_pointer(inputs: Vec<Type>, output: Type) -> Type {
    function_pointer_with(inputs, output, false, Abi::Rust)
}

fn unsafe_function_pointer(inputs: Vec<Type>, output: Type) -> Type {
    function_pointer_with(inputs, output, true, Abi::Rust)
}

fn extern_function_pointer(abi: &str, inputs: Vec<Type>, output: Type) -> Type {
    function_pointer_with(inputs, output, false, abi_(abi))
}

fn function_pointer_with(inputs: Vec<Type>, output: Type, is_unsafe: bool, abi: Abi) -> Type {
    Type::FunctionPointer(Box::new(FunctionPointer {
        sig: signature(
            inputs.into_iter().map(|ty| ("_", ty)).collect(),
            Some(output),
        ),
        generic_params: vec![],
        header: header(is_unsafe, abi),
    }))
}

fn mut_borrowed(ty: Type) -> Type {
    Type::BorrowedRef {
        lifetime: None,
        is_mutable: true,
        type_: Box::new(ty),
    }
}

fn borrowed(ty: Type) -> Type {
    Type::BorrowedRef {
        lifetime: None,
        is_mutable: false,
        type_: Box::new(ty),
    }
}

// --- generics ---

fn generic_param(name: &str) -> GenericParamDef {
    GenericParamDef {
        name: name.to_string(),
        kind: GenericParamDefKind::Type {
            bounds: vec![],
            default: None,
            is_synthetic: false,
        },
    }
}

fn lifetime_param(name: &str) -> GenericParamDef {
    GenericParamDef {
        name: name.to_string(),
        kind: GenericParamDefKind::Lifetime { outlives: vec![] },
    }
}

fn type_generics(params: Vec<GenericParamDef>) -> Generics {
    Generics {
        params,
        where_predicates: vec![],
    }
}

fn no_generics() -> Generics {
    type_generics(vec![])
}

// --- item inners ---

fn abi_(name: &str) -> Abi {
    match name {
        "Rust" => Abi::Rust,
        "C" => Abi::C { unwind: false },
        other => Abi::Other(other.to_string()),
    }
}

fn header(is_unsafe: bool, abi: Abi) -> FunctionHeader {
    FunctionHeader {
        is_const: false,
        is_unsafe,
        is_async: false,
        abi,
    }
}

fn signature(inputs: Vec<(&str, Type)>, output: Option<Type>) -> FunctionSignature {
    FunctionSignature {
        inputs: inputs
            .into_iter()
            .map(|(name, ty)| (name.to_string(), ty))
            .collect(),
        output,
        is_c_variadic: false,
    }
}

fn struct_plain(fields: &[&str]) -> ItemEnum {
    struct_plain_generic(fields, no_generics())
}

fn struct_plain_generic(fields: &[&str], generics: Generics) -> ItemEnum {
    ItemEnum::Struct(Struct {
        kind: StructKind::Plain {
            fields: ids(fields),
            has_stripped_fields: false,
        },
        generics,
        impls: vec![],
    })
}

fn struct_tuple(fields: &[&str]) -> ItemEnum {
    ItemEnum::Struct(Struct {
        kind: StructKind::Tuple(some_ids(fields)),
        generics: no_generics(),
        impls: vec![],
    })
}

fn struct_unit_generic(generics: Generics) -> ItemEnum {
    ItemEnum::Struct(Struct {
        kind: StructKind::Unit,
        generics,
        impls: vec![],
    })
}

fn enum_(variants: &[&str]) -> ItemEnum {
    ItemEnum::Enum(Enum {
        generics: no_generics(),
        has_stripped_variants: false,
        variants: ids(variants),
        impls: vec![],
    })
}

fn union_generic(fields: &[&str], generics: Generics) -> ItemEnum {
    ItemEnum::Union(Union {
        generics,
        has_stripped_fields: false,
        fields: ids(fields),
        impls: vec![],
    })
}

fn trait_(items: &[&str]) -> ItemEnum {
    ItemEnum::Trait(Trait {
        is_auto: false,
        is_unsafe: false,
        is_dyn_compatible: true,
        items: ids(items),
        generics: no_generics(),
        bounds: vec![],
        implementations: vec![],
    })
}

fn type_alias(ty: Type) -> ItemEnum {
    type_alias_generic(ty, no_generics())
}

fn type_alias_generic(ty: Type, generics: Generics) -> ItemEnum {
    ItemEnum::TypeAlias(TypeAlias {
        type_: ty,
        generics,
    })
}

fn variant_plain() -> ItemEnum {
    ItemEnum::Variant(Variant {
        kind: VariantKind::Plain,
        discriminant: None,
    })
}

fn variant_tuple(fields: &[&str]) -> ItemEnum {
    ItemEnum::Variant(Variant {
        kind: VariantKind::Tuple(some_ids(fields)),
        discriminant: None,
    })
}

fn variant_struct(fields: &[&str]) -> ItemEnum {
    ItemEnum::Variant(Variant {
        kind: VariantKind::Struct {
            fields: ids(fields),
            has_stripped_fields: false,
        },
        discriminant: None,
    })
}

fn impl_item(for_: Type, trait_: Option<Type>, items: &[&str]) -> ItemEnum {
    ItemEnum::Impl(Impl {
        is_unsafe: false,
        generics: no_generics(),
        provided_trait_methods: vec![],
        trait_: trait_.map(expect_path),
        for_,
        items: ids(items),
        is_negative: false,
        is_synthetic: false,
        blanket_impl: None,
    })
}

fn function_item(inputs: Vec<(&str, Type)>, output: Option<Type>) -> ItemEnum {
    ItemEnum::Function(Function {
        sig: signature(inputs, output),
        generics: no_generics(),
        header: header(false, Abi::Rust),
        has_body: true,
        default_unstable: None,
    })
}

fn unsafe_function_item(inputs: Vec<(&str, Type)>, output: Option<Type>) -> ItemEnum {
    ItemEnum::Function(Function {
        sig: signature(inputs, output),
        generics: no_generics(),
        header: header(true, Abi::Rust),
        has_body: true,
        default_unstable: None,
    })
}

fn constant_inner(ty: Type) -> ItemEnum {
    ItemEnum::Constant {
        type_: ty,
        const_: Constant {
            expr: String::new(),
            value: None,
            is_literal: false,
        },
    }
}

fn assoc_const(ty: Type) -> ItemEnum {
    ItemEnum::AssocConst {
        type_: ty,
        value: None,
        default_unstable: None,
    }
}

fn use_inner(source: &str, name: &str, target: Option<&str>, is_glob: bool) -> ItemEnum {
    ItemEnum::Use(Use {
        source: source.to_string(),
        name: name.to_string(),
        id: target.map(id),
        is_glob,
    })
}

fn module_inner(items: &[&str]) -> ItemEnum {
    ItemEnum::Module(Module {
        is_crate: false,
        items: ids(items),
        is_stripped: false,
    })
}

fn ids(keys: &[&str]) -> Vec<Id> {
    keys.iter().map(|key| id(key)).collect()
}

fn some_ids(keys: &[&str]) -> Vec<Option<Id>> {
    keys.iter().map(|key| Some(id(key))).collect()
}

fn expect_path(ty: Type) -> Path {
    match ty {
        Type::ResolvedPath(path) => path,
        other => panic!("expected resolved path, got {other:?}"),
    }
}

// --- items and crates ---

struct TestItem {
    name: Option<String>,
    visibility: Visibility,
    module_path: Option<Vec<String>>,
    inner: ItemEnum,
}

fn public_item(name: &str, inner: ItemEnum) -> TestItem {
    public_item_at_path(name, &["demo", name], inner)
}

fn public_item_at_path(name: &str, path: &[&str], inner: ItemEnum) -> TestItem {
    TestItem {
        name: Some(name.to_string()),
        visibility: Visibility::Public,
        module_path: Some(path.iter().map(|segment| segment.to_string()).collect()),
        inner,
    }
}

fn public_item_at_string_path(name: &str, path: &str, inner: ItemEnum) -> TestItem {
    TestItem {
        name: Some(name.to_string()),
        visibility: Visibility::Public,
        module_path: Some(
            path.split("::")
                .map(|segment| segment.to_string())
                .collect(),
        ),
        inner,
    }
}

fn public_field(name: &str, ty: Type) -> TestItem {
    TestItem {
        name: Some(name.to_string()),
        visibility: Visibility::Public,
        module_path: None,
        inner: ItemEnum::StructField(ty),
    }
}

fn private_field(name: &str, ty: Type) -> TestItem {
    TestItem {
        name: Some(name.to_string()),
        visibility: Visibility::Default,
        module_path: None,
        inner: ItemEnum::StructField(ty),
    }
}

/// A public impl block (no name, no module path).
fn public_impl(for_: Type, trait_: Option<Type>, items: &[&str]) -> TestItem {
    TestItem {
        name: None,
        visibility: Visibility::Public,
        module_path: None,
        inner: impl_item(for_, trait_, items),
    }
}

fn public_function(name: &str, inputs: Vec<(&str, Type)>, output: Type) -> TestItem {
    public_item_at_path(name, &["demo"], function_item(inputs, Some(output)))
}

fn public_unsafe_function(name: &str, inputs: Vec<(&str, Type)>, output: Type) -> TestItem {
    public_item_at_path(name, &["demo"], unsafe_function_item(inputs, Some(output)))
}

fn public_constant(name: &str, ty: Type) -> TestItem {
    public_item_at_path(name, &["demo"], constant_inner(ty))
}

fn public_use(name: &str, target: &str) -> TestItem {
    public_item_at_path(
        name,
        &["demo"],
        use_inner(&format!("demo::{name}"), name, Some(target), false),
    )
}

fn public_external_use(name: &str, source: &str) -> TestItem {
    public_item_at_path(name, &["demo"], use_inner(source, name, None, false))
}

fn public_glob_use(name: &str, target: &str) -> TestItem {
    public_item_at_path(
        name,
        &["demo"],
        use_inner(&format!("demo::{name}::*"), name, Some(target), true),
    )
}

fn public_module(name: &str, items: &[&str]) -> TestItem {
    public_item_at_path(name, &["demo", name], module_inner(items))
}

fn crate_(entries: Vec<(&str, TestItem)>) -> Crate {
    let mut index = HashMap::new();
    let mut paths = HashMap::new();
    for (key, item) in entries {
        let item_id = id(key);
        let TestItem {
            name,
            visibility,
            module_path,
            inner,
        } = item;
        if let Some(path) = module_path {
            paths.insert(
                item_id,
                ItemSummary {
                    crate_id: 0,
                    path,
                    kind: inner.item_kind(),
                },
            );
        }
        index.insert(
            item_id,
            Item {
                id: item_id,
                crate_id: 0,
                name,
                span: None,
                visibility,
                docs: None,
                links: HashMap::new(),
                attrs: vec![],
                deprecation: None,
                stability: None,
                const_stability: None,
                inner,
            },
        );
    }
    Crate {
        root: Id(0),
        crate_version: None,
        includes_private: false,
        index,
        paths,
        external_crates: HashMap::new(),
        target: Target {
            triple: String::new(),
            target_features: vec![],
        },
        format_version: rustdoc_types::FORMAT_VERSION,
    }
}

fn empty_crate() -> Crate {
    crate_(vec![])
}

/// Lift a standalone type without any crate context (resolution falls back to
/// the type's own usage path).
fn lift_type(interop: &mut RustInterop, crate_name: &str, ty: &Type) -> Option<TypeElement> {
    interop.type_from_json(&empty_crate(), crate_name, ty)
}

fn lift_param(
    interop: &mut RustInterop,
    crate_name: &str,
    name: &str,
    ty: &Type,
) -> Option<galvan_ast::Param> {
    interop.param_from_json(&empty_crate(), crate_name, name, ty)
}

fn string_type() -> TypeElement {
    plain_type(TypeIdent::new("String"))
}

fn u64_type() -> TypeElement {
    plain_type(TypeIdent::new("U64"))
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

// ---------------------------------------------------------------------------
// Import behavior tests.
// ---------------------------------------------------------------------------

fn serde_json_import_fixture() -> Crate {
    crate_(vec![
        (
            "to_string",
            public_function("to_string", vec![], primitive("u64")),
        ),
        (
            "from_str",
            public_function("from_str", vec![], primitive("u64")),
        ),
    ])
}

#[test]
fn loading_a_crate_does_not_import_its_functions_unqualified() {
    let mut interop = RustInterop::empty();
    interop.add_crate("serde_json", &serde_json_import_fixture());

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
    let mut interop = RustInterop::empty();
    interop.add_crate("serde_json", &serde_json_import_fixture());
    interop.import_uses(&uses);

    assert!(interop
        .function(None, None, &ident("to_string"), &[])
        .is_some());
}

#[test]
fn path_use_declarations_import_only_the_named_item() {
    let uses = [use_decl(&["serde_json", "to_string"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("serde_json", &serde_json_import_fixture());
    interop.import_uses(&uses);

    assert!(interop
        .function(None, None, &ident("to_string"), &[])
        .is_some());
    assert!(interop
        .function(None, None, &ident("from_str"), &[])
        .is_none());
}

// ---------------------------------------------------------------------------
// Typed-builder tests.
// ---------------------------------------------------------------------------

#[test]
fn use_declarations_import_types_unqualified() {
    let krate = crate_(vec![("0", public_item("Ticket", struct_plain(&[])))]);
    let uses = [use_decl(&["demo"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);
    interop.import_uses(&uses);

    let imported = interop.imported_types().collect::<Vec<_>>();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, TypeIdent::new("Ticket"));
    assert_eq!(imported[0].rust_path.as_ref(), "::demo::Ticket");
}

#[test]
fn path_use_declarations_import_only_the_named_type() {
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_item("InternalNote", struct_plain(&[]))),
    ]);
    let uses = [use_decl(&["demo", "Ticket"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);
    interop.import_uses(&uses);

    let imported = interop.imported_types().collect::<Vec<_>>();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, TypeIdent::new("Ticket"));
}

#[test]
fn rustdoc_crates_expose_only_their_lifted_items() {
    // A crate's imported surface is exactly what rustdoc lifting produced:
    // nothing is fabricated on top of it (e.g. a `to_string` fn or `Error`
    // type that the JSON did not declare).
    let krate = crate_(vec![(
        "Value",
        public_item_at_path("Value", &["serde_json", "Value"], struct_plain(&[])),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("serde_json", &krate);

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
    let krate = crate_(vec![(
        "0",
        public_constant("DEFAULT_LIMIT", primitive("u64")),
    )]);
    let uses = [use_decl(&["demo"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);
    interop.import_uses(&uses);

    let constant = interop
        .constant(None, &ident("DEFAULT_LIMIT"))
        .expect("expected imported constant");
    assert_eq!(constant.ty, u64_type());
    assert_eq!(constant.rust_path.as_ref(), "::demo::DEFAULT_LIMIT");
}

#[test]
fn use_declarations_suppress_ambiguous_unqualified_items() {
    let http = crate_(vec![
        (
            "0",
            public_item_at_path("Ticket", &["http", "Ticket"], struct_plain(&[])),
        ),
        (
            "1",
            public_function("parse", vec![], resolved("Ticket", vec![])),
        ),
        ("2", public_constant("DEFAULT_LIMIT", primitive("u64"))),
    ]);
    let db = crate_(vec![
        (
            "0",
            public_item_at_path("Ticket", &["db", "Ticket"], struct_plain(&[])),
        ),
        (
            "1",
            public_function("parse", vec![], resolved("Ticket", vec![])),
        ),
        ("2", public_constant("DEFAULT_LIMIT", primitive("u64"))),
    ]);
    let uses = [use_decl(&["http"]), use_decl(&["db"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("http", &http);
    interop.add_crate("db", &db);
    interop.import_uses(&uses);

    let imported = interop
        .imported_types()
        .map(|ty| format!("{} {}", ty.name.as_str(), ty.rust_path.as_ref()))
        .collect::<Vec<_>>();
    assert!(imported.is_empty(), "{imported:?}");
    assert!(interop.function(None, None, &ident("parse"), &[]).is_none());
    assert!(interop.constant(None, &ident("DEFAULT_LIMIT")).is_none());
    assert!(interop
        .function(Some("http"), None, &ident("parse"), &[])
        .is_some());
    assert!(interop
        .function(Some("db"), None, &ident("parse"), &[])
        .is_some());
    assert!(interop
        .constant(Some("http"), &ident("DEFAULT_LIMIT"))
        .is_some());
    assert!(interop
        .constant(Some("db"), &ident("DEFAULT_LIMIT"))
        .is_some());
}

#[test]
fn path_use_declarations_suppress_ambiguous_unqualified_items() {
    let http = crate_(vec![(
        "0",
        public_item_at_path("Ticket", &["http", "Ticket"], struct_plain(&[])),
    )]);
    let db = crate_(vec![(
        "0",
        public_item_at_path("Ticket", &["db", "Ticket"], struct_plain(&[])),
    )]);
    let uses = [use_decl(&["http", "Ticket"]), use_decl(&["db", "Ticket"])];
    let mut interop = RustInterop::empty();
    interop.add_crate("http", &http);
    interop.add_crate("db", &db);
    interop.import_uses(&uses);

    let imported = interop
        .imported_types()
        .map(|ty| format!("{} {}", ty.name.as_str(), ty.rust_path.as_ref()))
        .collect::<Vec<_>>();
    assert!(imported.is_empty(), "{imported:?}");
    assert!(interop
        .type_by_qualified_path(&["http", "Ticket"])
        .is_some());
    assert!(interop.type_by_qualified_path(&["db", "Ticket"]).is_some());
}

#[test]
fn rustdoc_preserves_same_named_types_from_different_modules() {
    let krate = crate_(vec![
        (
            "http_error",
            public_item_at_path("Error", &["demo", "http", "Error"], struct_plain(&[])),
        ),
        (
            "db_error",
            public_item_at_path("Error", &["demo", "db", "Error"], struct_plain(&[])),
        ),
        (
            "auth_error",
            public_item_at_string_path("Error", "crate::auth::Error", struct_plain(&[])),
        ),
        (
            "internal_error",
            public_item_at_string_path("Error", "$crate::internal::Error", struct_plain(&[])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let mut error_paths = interop
        .types
        .iter()
        .filter(|ty| ty.name.as_str() == "Error")
        .map(|ty| ty.rust_path.as_ref())
        .collect::<Vec<_>>();
    error_paths.sort();
    assert_eq!(
        error_paths,
        vec![
            "::demo::auth::Error",
            "::demo::db::Error",
            "::demo::http::Error",
            "::demo::internal::Error"
        ]
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
    assert_eq!(
        interop
            .type_by_qualified_path(&["demo", "auth", "Error"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::demo::auth::Error")
    );
    assert_eq!(
        interop
            .type_by_qualified_path(&["demo", "internal", "Error"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::demo::internal::Error")
    );
}

#[test]
fn rustdoc_preserves_generic_resolved_paths() {
    let mut interop = RustInterop::empty();
    let ty = lift_type(
        &mut interop,
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

    let TypeDecl::Empty(json) = imported_type(&interop, "Json") else {
        panic!("expected referenced Json type to be recorded");
    };
    assert_eq!(json.generic_params, vec![Ident::new("T")]);
}

#[test]
fn rustdoc_preserves_generic_arity_for_referenced_type_placeholders() {
    let mut interop = RustInterop::empty();
    lift_type(
        &mut interop,
        "demo",
        &resolved_with_path(
            "Pair",
            &["demo", "Pair"],
            vec![generic("Item"), primitive("u64")],
        ),
    );

    let TypeDecl::Empty(pair) = imported_type(&interop, "Pair") else {
        panic!("expected referenced Pair type to be recorded");
    };
    assert_eq!(
        pair.generic_params,
        vec![Ident::new("Item"), Ident::new("U")]
    );

    lift_type(
        &mut interop,
        "demo",
        &resolved_with_path(
            "Pair",
            &["demo", "Pair"],
            vec![generic("Item"), primitive("u64"), primitive("str")],
        ),
    );

    let TypeDecl::Empty(pair) = imported_type(&interop, "Pair") else {
        panic!("expected referenced Pair type to remain opaque");
    };
    assert_eq!(
        pair.generic_params,
        vec![Ident::new("Item"), Ident::new("U"), Ident::new("V")]
    );
}

#[test]
fn rustdoc_lifts_resolved_string_as_builtin_string() {
    let mut interop = RustInterop::empty();
    let ty = lift_type(
        &mut interop,
        "std",
        &resolved_with_path("String", &["alloc", "string", "String"], vec![]),
    )
    .unwrap();

    assert_eq!(ty, string_type());
    assert!(interop.types.iter().all(|ty| ty.name.as_str() != "String"));

    let path_only = lift_type(
        &mut interop,
        "std",
        &resolved_with_string_path("alloc::string::String", vec![]),
    )
    .unwrap();
    assert_eq!(path_only, string_type());
}

#[test]
fn rustdoc_preserves_qualified_paths_for_referenced_types() {
    let mut interop = RustInterop::empty();
    let ty = lift_type(
        &mut interop,
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

    let string_path_ty = lift_type(
        &mut interop,
        "axum",
        &resolved_with_string_path("axum::extract::State", vec![primitive("str")]),
    )
    .unwrap();
    let TypeElement::Parametric(parametric) = string_path_ty else {
        panic!("expected State<T>, got {string_path_ty:?}");
    };
    assert_eq!(parametric.base_type.as_str(), "State");
    assert_eq!(
        interop
            .type_by_qualified_path(&["axum", "extract", "State"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::axum::extract::State")
    );

    let crate_path_ty = lift_type(
        &mut interop,
        "axum",
        &resolved_with_string_path("crate::routing::Router", vec![]),
    )
    .unwrap();
    assert_eq!(crate_path_ty, plain_type(TypeIdent::new("Router")));
    assert_eq!(
        interop
            .type_by_qualified_path(&["axum", "routing", "Router"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::axum::routing::Router")
    );
}

#[test]
fn rustdoc_preserves_same_named_referenced_types_from_different_modules() {
    let mut interop = RustInterop::empty();
    lift_type(
        &mut interop,
        "demo",
        &resolved_with_path("Error", &["demo", "http", "Error"], vec![]),
    );
    lift_type(
        &mut interop,
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

    let optional = lift_type(
        &mut interop,
        "std",
        &resolved_with_path(
            "Option",
            &["core", "option", "Option"],
            vec![primitive("u64")],
        ),
    )
    .unwrap();
    let TypeElement::Optional(optional) = optional else {
        panic!("expected optional, got {optional:?}");
    };
    assert_eq!(optional.inner, u64_type());

    let map = lift_type(
        &mut interop,
        "std",
        &resolved("HashMap", vec![primitive("str"), primitive("u64")]),
    )
    .unwrap();
    let TypeElement::Dictionary(map) = map else {
        panic!("expected dictionary, got {map:?}");
    };
    assert_eq!(map.key, string_type());
    assert_eq!(map.value, u64_type());

    let ordered_map = lift_type(
        &mut interop,
        "std",
        &resolved("BTreeMap", vec![primitive("str"), primitive("u64")]),
    )
    .unwrap();
    let TypeElement::OrderedDictionary(ordered_map) = ordered_map else {
        panic!("expected ordered dictionary, got {ordered_map:?}");
    };
    assert_eq!(ordered_map.key, string_type());
    assert_eq!(ordered_map.value, u64_type());

    let set = lift_type(
        &mut interop,
        "std",
        &resolved("BTreeSet", vec![primitive("str")]),
    )
    .unwrap();
    let TypeElement::Set(set) = set else {
        panic!("expected set, got {set:?}");
    };
    assert_eq!(set.elements, string_type());

    let deque = lift_type(
        &mut interop,
        "std",
        &resolved("VecDeque", vec![primitive("u64")]),
    )
    .unwrap();
    let TypeElement::Array(deque) = deque else {
        panic!("expected array, got {deque:?}");
    };
    assert_eq!(deque.elements, u64_type());

    let result = lift_type(
        &mut interop,
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
fn rustdoc_preserves_dependency_types_named_like_known_wrappers() {
    let mut interop = RustInterop::empty();

    let optional = lift_type(
        &mut interop,
        "demo",
        &resolved_with_path(
            "Option",
            &["demo", "schema", "Option"],
            vec![primitive("u64")],
        ),
    )
    .unwrap();
    let TypeElement::Parametric(optional) = optional else {
        panic!("expected nominal Option<T>, got {optional:?}");
    };
    assert_eq!(optional.base_type, TypeIdent::new("Option"));
    assert_eq!(optional.type_args, vec![u64_type()]);
    assert_eq!(
        interop
            .type_by_qualified_path(&["demo", "schema", "Option"])
            .map(|ty| ty.rust_path.as_ref()),
        Some("::demo::schema::Option")
    );

    let list = lift_type(
        &mut interop,
        "demo",
        &resolved_with_path("Vec", &["demo", "schema", "Vec"], vec![primitive("str")]),
    )
    .unwrap();
    let TypeElement::Parametric(list) = list else {
        panic!("expected nominal Vec<T>, got {list:?}");
    };
    assert_eq!(list.base_type, TypeIdent::new("Vec"));
    assert_eq!(list.type_args, vec![string_type()]);

    let result = lift_type(
        &mut interop,
        "demo",
        &resolved_with_path(
            "Result",
            &["demo", "schema", "Result"],
            vec![
                primitive("u64"),
                resolved_with_path("Error", &["demo", "schema", "Error"], vec![]),
            ],
        ),
    )
    .unwrap();
    let TypeElement::Parametric(result) = result else {
        panic!("expected nominal Result<T, E>, got {result:?}");
    };
    assert_eq!(result.base_type, TypeIdent::new("Result"));
    assert_eq!(
        result.type_args,
        vec![u64_type(), plain_type(TypeIdent::new("Error"))]
    );

    let shared = lift_type(
        &mut interop,
        "demo",
        &resolved_with_path(
            "Arc",
            &["demo", "sync", "Arc"],
            vec![resolved_with_path(
                "Mutex",
                &["demo", "sync", "Mutex"],
                vec![primitive("u64")],
            )],
        ),
    )
    .unwrap();
    let TypeElement::Parametric(shared) = shared else {
        panic!("expected nominal Arc<T>, got {shared:?}");
    };
    assert_eq!(shared.base_type, TypeIdent::new("Arc"));
    let TypeElement::Parametric(lock) = &shared.type_args[0] else {
        panic!("expected nominal Mutex<T>, got {:?}", shared.type_args[0]);
    };
    assert_eq!(lock.base_type, TypeIdent::new("Mutex"));
    assert_eq!(lock.type_args, vec![u64_type()]);
}

#[test]
fn rustdoc_inlines_wrapper_type_aliases_at_references() {
    // A reference to a local `type Result<T> = core::result::Result<T, Error>`
    // is inlined to a galvan Result (aliases are transparent), rather than kept
    // as a nominal `Result<String>` — so downstream fallible handling works
    // regardless of whether rustdoc expands the alias in the signature.
    let krate = crate_(vec![
        (
            "alias",
            public_item_at_path(
                "Result",
                &["demo", "error", "Result"],
                type_alias_generic(
                    resolved("Result", vec![generic("T"), resolved("Error", vec![])]),
                    type_generics(vec![generic_param("T")]),
                ),
            ),
        ),
        (
            "f",
            public_function(
                "make",
                vec![],
                resolved_to("alias", "Result", vec![primitive("str")]),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let function = interop
        .function(Some("demo"), None, &ident("make"), &[])
        .expect("expected imported function");
    let TypeElement::Result(result) = &function.decl.item.signature.return_type else {
        panic!(
            "expected inlined galvan Result, got {:?}",
            function.decl.item.signature.return_type
        );
    };
    assert_eq!(result.success, string_type());
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
        resolved_with_string_path("anyhow::Result", vec![primitive("u64")]),
    ] {
        let result = lift_type(&mut interop, "demo", &ty).unwrap();
        let TypeElement::Result(result) = result else {
            panic!("expected flexible result, got {result:?}");
        };
        assert_eq!(result.success, u64_type());
        assert_eq!(result.error, None);
    }

    let unresolved_result = lift_type(
        &mut interop,
        "demo",
        &resolved("Result", vec![primitive("u64")]),
    )
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

    let slice = lift_type(&mut interop, "std", &slice(primitive("u64"))).unwrap();
    let TypeElement::Array(slice) = slice else {
        panic!("expected slice to lift as array, got {slice:?}");
    };
    assert_eq!(slice.elements, u64_type());

    let array = lift_type(&mut interop, "std", &array(primitive("str"))).unwrap();
    let TypeElement::Array(array) = array else {
        panic!("expected fixed array to lift as array, got {array:?}");
    };
    assert_eq!(array.elements, string_type());
}

#[test]
fn rustdoc_does_not_lift_raw_pointer_types() {
    let mut interop = RustInterop::empty();

    assert!(lift_type(&mut interop, "std", &raw_pointer(primitive("u8"), false)).is_none());
}

#[test]
fn rustdoc_does_not_lift_unsafe_function_pointer_types() {
    let mut interop = RustInterop::empty();

    assert!(lift_type(
        &mut interop,
        "std",
        &unsafe_function_pointer(vec![primitive("u64")], primitive("bool"))
    )
    .is_none());
}

#[test]
fn rustdoc_does_not_lift_non_rust_abi_function_pointer_types() {
    let mut interop = RustInterop::empty();

    assert!(lift_type(
        &mut interop,
        "std",
        &extern_function_pointer("C", vec![primitive("u64")], primitive("bool"))
    )
    .is_none());
}

#[test]
fn rustdoc_does_not_lift_unrepresentable_type_shapes() {
    let mut interop = RustInterop::empty();

    assert!(lift_type(
        &mut interop,
        "demo",
        &qualified_path("Output", generic("T"))
    )
    .is_none());
    assert!(lift_type(&mut interop, "demo", &dyn_trait()).is_none());
    assert!(lift_type(&mut interop, "demo", &impl_trait()).is_none());
}

#[test]
fn rustdoc_does_not_lift_partial_type_shapes() {
    let mut interop = RustInterop::empty();

    assert!(lift_type(&mut interop, "std", &resolved("Option", vec![])).is_none());
    assert!(lift_type(&mut interop, "std", &resolved("Vec", vec![])).is_none());
    assert!(lift_type(
        &mut interop,
        "std",
        &resolved("HashMap", vec![primitive("str")])
    )
    .is_none());
    assert!(lift_type(&mut interop, "std", &resolved("Result", vec![])).is_none());
    assert!(lift_type(
        &mut interop,
        "demo",
        &resolved_with_path("Result", &["anyhow", "Result"], vec![])
    )
    .is_none());
    assert!(lift_type(
        &mut interop,
        "demo",
        &resolved_with_path("FlexResult", &["galvan", "std", "FlexResult"], vec![])
    )
    .is_none());
}

#[test]
fn rustdoc_lifts_function_pointer_types() {
    let mut interop = RustInterop::empty();
    let ty = lift_type(
        &mut interop,
        "std",
        &function_pointer(vec![primitive("u64"), primitive("str")], primitive("bool")),
    )
    .unwrap();

    let TypeElement::Closure(closure) = ty else {
        panic!("expected function pointer to lift as closure, got {ty:?}");
    };
    assert_eq!(closure.parameters, vec![u64_type(), string_type()]);
    assert_eq!(closure.return_ty, TypeElement::bool());

    let rust_abi = lift_type(
        &mut interop,
        "std",
        &extern_function_pointer("Rust", vec![primitive("u64")], primitive("bool")),
    )
    .unwrap();
    assert!(matches!(rust_abi, TypeElement::Closure(_)));
}

#[test]
fn rustdoc_lifts_never_types() {
    let mut interop = RustInterop::empty();

    let ty = lift_type(&mut interop, "std", &never()).unwrap();
    assert!(matches!(ty, TypeElement::Never(_)));

    let primitive_ty = lift_type(&mut interop, "std", &primitive("!"))
        .expect("expected primitive never type to lift");
    assert!(matches!(primitive_ty, TypeElement::Never(_)));
}

#[test]
fn rustdoc_lifts_shared_wrappers_to_ref_parameters() {
    let mut interop = RustInterop::empty();
    for (wrapper, leaked_name) in [
        (
            resolved_with_path(
                "Arc",
                &["alloc", "sync", "Arc"],
                vec![resolved_with_path(
                    "Mutex",
                    &["std", "sync", "Mutex"],
                    vec![generic("T")],
                )],
            ),
            "Mutex",
        ),
        (
            resolved("Arc", vec![resolved("Mutex", vec![generic("T")])]),
            "Mutex",
        ),
        (
            resolved("Arc", vec![resolved("RwLock", vec![generic("T")])]),
            "RwLock",
        ),
    ] {
        let param = lift_param(&mut interop, "std", "tickets", &wrapper).unwrap();

        assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Ref));
        assert_eq!(param.param_type, generic_type("T"));
        assert!(interop
            .types
            .iter()
            .all(|ty| !matches!(ty.name.as_str(), "Arc") && ty.name.as_str() != leaked_name));
    }
}

#[test]
fn rustdoc_skips_bare_standard_lock_wrappers() {
    let mut interop = RustInterop::empty();

    for wrapper in [
        resolved("Mutex", vec![generic("T")]),
        resolved("RwLock", vec![generic("T")]),
    ] {
        assert!(lift_type(&mut interop, "std", &wrapper).is_none());
        assert!(lift_param(&mut interop, "std", "tickets", &wrapper).is_none());
    }
}

#[test]
fn rustdoc_preserves_non_shared_arc_types_nominally() {
    let mut interop = RustInterop::empty();
    let ty = lift_type(
        &mut interop,
        "std",
        &resolved("Arc", vec![resolved("Ticket", vec![])]),
    )
    .unwrap();

    let TypeElement::Parametric(parametric) = ty else {
        panic!("expected Arc<Ticket>, got {ty:?}");
    };
    assert_eq!(parametric.base_type, TypeIdent::new("Arc"));
    assert_eq!(
        parametric.type_args,
        vec![plain_type(TypeIdent::new("Ticket"))]
    );
    assert!(interop.types.iter().any(|ty| ty.name.as_str() == "Arc"));
    assert!(interop.types.iter().any(|ty| ty.name.as_str() == "Ticket"));
}

#[test]
fn rustdoc_keeps_single_owner_atomics_nominal() {
    let mut interop = RustInterop::empty();
    let atomic = lift_param(
        &mut interop,
        "std",
        "next_id",
        &resolved("AtomicU64", vec![]),
    )
    .unwrap();
    assert_eq!(atomic.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(atomic.param_type, plain_type(TypeIdent::new("AtomicU64")));
}

#[test]
fn rustdoc_lifts_arc_atomic_primitives_to_ref_parameters() {
    let mut interop = RustInterop::empty();
    let param = lift_param(
        &mut interop,
        "std",
        "next_id",
        &resolved("Arc", vec![resolved("AtomicU64", vec![])]),
    )
    .unwrap();

    assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Ref));
    assert_eq!(param.param_type, u64_type());
    assert!(interop
        .types
        .iter()
        .all(|ty| !matches!(ty.name.as_str(), "Arc" | "AtomicU64")));
}

#[test]
fn rustdoc_lifts_mutable_borrowed_parameters_to_mut() {
    let mut interop = RustInterop::empty();
    let param = lift_param(
        &mut interop,
        "demo",
        "ticket",
        &mut_borrowed(resolved("Ticket", vec![])),
    )
    .unwrap();

    assert_eq!(param.decl_modifier, Some(galvan_ast::DeclModifier::Mut));
    assert_eq!(param.param_type, plain_type(TypeIdent::new("Ticket")));
}

#[test]
fn rustdoc_keeps_owned_copy_parameters_unmodified() {
    let mut interop = RustInterop::empty();
    let integer = lift_param(&mut interop, "std", "limit", &primitive("u64")).unwrap();
    assert_eq!(integer.decl_modifier, None);
    assert_eq!(integer.param_type, u64_type());

    let tuple = lift_param(
        &mut interop,
        "std",
        "range",
        &tuple_(vec![primitive("u64"), primitive("bool")]),
    )
    .unwrap();
    assert_eq!(tuple.decl_modifier, None);
    assert!(matches!(tuple.param_type, TypeElement::Tuple(_)));
}

#[test]
fn rustdoc_marks_owned_non_copy_parameters_as_move() {
    let mut interop = RustInterop::empty();
    let string = lift_param(&mut interop, "std", "title", &primitive("str")).unwrap();
    assert_eq!(string.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(string.param_type, string_type());

    let generic = lift_param(&mut interop, "demo", "value", &generic("T")).unwrap();
    assert_eq!(generic.decl_modifier, Some(galvan_ast::DeclModifier::Move));
    assert_eq!(generic.param_type, generic_type("T"));
}

#[test]
fn rustdoc_preserves_shared_borrow_parameter_conversions() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "takes_ref",
            vec![("value", borrowed(slice(primitive("u64"))))],
            primitive("bool"),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![(
        "0",
        public_unsafe_function("from_raw_parts", vec![], primitive("u64")),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("from_raw_parts"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_functions_with_raw_pointer_signatures() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "read_address",
            vec![("bytes", raw_pointer(primitive("u8"), false))],
            primitive("u64"),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("read_address"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_functions_with_unsafe_function_pointer_signatures() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "visit",
            vec![(
                "callback",
                unsafe_function_pointer(vec![primitive("u64")], primitive("bool")),
            )],
            primitive("bool"),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("visit"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_functions_with_unliftable_signatures() {
    let krate = crate_(vec![
        (
            "0",
            public_function(
                "visit",
                vec![("visitor", dyn_trait())],
                qualified_path("Output", generic("V")),
            ),
        ),
        ("1", public_function("make_display", vec![], impl_trait())),
        (
            "5",
            public_constant("DEFAULT_OUTPUT", qualified_path("Output", generic("V"))),
        ),
        (
            "7",
            public_function(
                "bare_lock_input",
                vec![(
                    "tickets",
                    resolved("Mutex", vec![resolved("Ticket", vec![])]),
                )],
                primitive("bool"),
            ),
        ),
        (
            "8",
            public_function(
                "nested_bare_lock_input",
                vec![(
                    "tickets",
                    resolved(
                        "Option",
                        vec![resolved("RwLock", vec![resolved("Ticket", vec![])])],
                    ),
                )],
                primitive("bool"),
            ),
        ),
        (
            "9",
            public_constant(
                "LOCKED_TICKETS",
                resolved("Mutex", vec![resolved("Ticket", vec![])]),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("visit"), &[])
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("make_display"), &[])
        .is_none());
    assert!(interop
        .constant(Some("demo"), &ident("DEFAULT_OUTPUT"))
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("bare_lock_input"), &[])
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("nested_bare_lock_input"), &[])
        .is_none());
    assert!(interop
        .constant(Some("demo"), &ident("LOCKED_TICKETS"))
        .is_none());
}

#[test]
fn rustdoc_imports_functions_with_shared_arc_lock_signatures() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "replace_tickets",
            vec![(
                "tickets",
                resolved(
                    "Arc",
                    vec![resolved("Mutex", vec![resolved("Ticket", vec![])])],
                ),
            )],
            resolved(
                "Arc",
                vec![resolved("RwLock", vec![resolved("Ticket", vec![])])],
            ),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let function = interop
        .function(Some("demo"), None, &ident("replace_tickets"), &[])
        .expect("expected shared lock wrapper function to import");
    assert_eq!(
        function.decl.item.signature.parameters.params[0].decl_modifier,
        Some(galvan_ast::DeclModifier::Ref)
    );
    assert_eq!(
        function.decl.item.signature.parameters.params[0].param_type,
        plain_type(TypeIdent::new("Ticket"))
    );
    assert_eq!(
        function.decl.item.signature.return_type,
        plain_type(TypeIdent::new("Ticket"))
    );
}

#[test]
fn rustdoc_imports_function_pointer_parameters() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "filter_tickets",
            vec![(
                "predicate",
                function_pointer(vec![primitive("u64")], primitive("bool")),
            )],
            primitive("u64"),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("Buffer", struct_plain(&["1"]))),
        (
            "1",
            public_field("ptr", raw_pointer(primitive("u8"), false)),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(buffer) = imported_type(&interop, "Buffer") else {
        panic!("expected Buffer to import as an opaque type");
    };
    assert_eq!(buffer.ident, TypeIdent::new("Buffer"));
}

#[test]
fn rustdoc_keeps_types_with_unsafe_function_pointer_fields_opaque() {
    let krate = crate_(vec![
        ("0", public_item("CallbackRegistry", struct_plain(&["1"]))),
        (
            "1",
            public_field(
                "callback",
                unsafe_function_pointer(vec![primitive("u64")], primitive("bool")),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(registry) = imported_type(&interop, "CallbackRegistry") else {
        panic!("expected CallbackRegistry to import as an opaque type");
    };
    assert_eq!(registry.ident, TypeIdent::new("CallbackRegistry"));
}

#[test]
fn rustdoc_keeps_types_with_unliftable_fields_opaque() {
    let krate = crate_(vec![
        ("0", public_item("VisitResult", struct_plain(&["1"]))),
        (
            "1",
            public_field("output", qualified_path("Output", generic("V"))),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(visit_result) = imported_type(&interop, "VisitResult") else {
        panic!("expected VisitResult to import as an opaque type");
    };
    assert_eq!(visit_result.ident, TypeIdent::new("VisitResult"));
}

#[test]
fn rustdoc_imports_unions_as_opaque_types() {
    let krate = crate_(vec![
        (
            "0",
            public_item(
                "Bits",
                union_generic(&["1"], type_generics(vec![generic_param("T")])),
            ),
        ),
        ("1", public_field("value", generic("T"))),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(bits) = imported_type(&interop, "Bits") else {
        panic!("expected Bits union to import as an opaque type");
    };
    assert_eq!(bits.ident, TypeIdent::new("Bits"));
    assert_eq!(bits.generic_params, vec![Ident::new("T")]);
}

#[test]
fn rustdoc_keeps_types_with_incomplete_field_metadata_opaque() {
    let krate = crate_(vec![
        (
            "0",
            public_item("PartialTicket", struct_plain(&["1", "missing"])),
        ),
        ("1", public_field("title", primitive("str"))),
        (
            "2",
            public_item("PartialTuple", struct_tuple(&["3", "missing"])),
        ),
        ("3", public_field("0", primitive("u64"))),
        ("4", public_item("PartialEvent", enum_(&["5"]))),
        (
            "5",
            public_item("Renamed", variant_tuple(&["6", "missing"])),
        ),
        ("6", public_field("0", primitive("str"))),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    for name in ["PartialTicket", "PartialTuple", "PartialEvent"] {
        let TypeDecl::Empty(opaque) = imported_type(&interop, name) else {
            panic!("expected {name} to import as an opaque type");
        };
        assert_eq!(opaque.ident, TypeIdent::new(name));
    }
}

#[test]
fn rustdoc_keeps_types_with_non_public_fields_opaque() {
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&["1", "2"]))),
        ("1", public_field("title", primitive("str"))),
        ("2", private_field("secret", primitive("str"))),
        ("3", public_item("UserId", struct_tuple(&["4"]))),
        ("4", private_field("0", primitive("u64"))),
        ("5", public_item("TicketEvent", enum_(&["6"]))),
        ("6", public_item("Moved", variant_struct(&["7"]))),
        ("7", private_field("queue", primitive("str"))),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    for name in ["Ticket", "UserId", "TicketEvent"] {
        let TypeDecl::Empty(opaque) = imported_type(&interop, name) else {
            panic!("expected {name} to import as an opaque type");
        };
        assert_eq!(opaque.ident, TypeIdent::new(name));
    }
}

#[test]
fn rustdoc_preserves_generic_params_on_opaque_types() {
    let krate = crate_(vec![(
        "0",
        public_item(
            "State",
            struct_unit_generic(type_generics(vec![
                lifetime_param("'a"),
                generic_param("T"),
            ])),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(state) = imported_type(&interop, "State") else {
        panic!("expected opaque State type");
    };
    assert_eq!(state.generic_params, vec![Ident::new("T")]);
}

#[test]
fn rustdoc_preserves_generic_params_on_structs() {
    let krate = crate_(vec![
        (
            "0",
            public_item(
                "Page",
                struct_plain_generic(&["1"], type_generics(vec![generic_param("T")])),
            ),
        ),
        (
            "1",
            public_field("items", resolved("Vec", vec![generic("T")])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![(
        "0",
        public_function("abort_request", vec![], never()),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        (
            "0",
            public_function(
                "takes_box",
                vec![("value", resolved("Box", vec![primitive("u64")]))],
                primitive("bool"),
            ),
        ),
        (
            "1",
            public_function(
                "takes_rc",
                vec![("value", resolved("Rc", vec![resolved("Ticket", vec![])]))],
                primitive("bool"),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
fn rustdoc_does_not_import_incomplete_owned_wrapper_parameters() {
    let krate = crate_(vec![
        (
            "0",
            public_function(
                "takes_box",
                vec![("value", resolved("Box", vec![]))],
                primitive("bool"),
            ),
        ),
        (
            "1",
            public_function(
                "takes_rc",
                vec![("value", resolved("Rc", vec![]))],
                primitive("bool"),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("takes_box"), &[])
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("takes_rc"), &[])
        .is_none());
}

#[test]
fn rustdoc_does_not_import_incomplete_owned_wrapper_returns() {
    let krate = crate_(vec![
        (
            "0",
            public_function("returns_box", vec![], resolved("Box", vec![])),
        ),
        (
            "1",
            public_function("returns_rc", vec![], resolved("Rc", vec![])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("returns_box"), &[])
        .is_none());
    assert!(interop
        .function(Some("demo"), None, &ident("returns_rc"), &[])
        .is_none());
}

#[test]
fn rustdoc_preserves_dependency_owned_wrapper_names_without_conversions() {
    let krate = crate_(vec![
        (
            "0",
            public_function(
                "takes_box",
                vec![(
                    "value",
                    resolved_with_path("Box", &["demo", "smart", "Box"], vec![primitive("u64")]),
                )],
                primitive("bool"),
            ),
        ),
        (
            "1",
            public_function(
                "returns_rc",
                vec![],
                resolved_with_path("Rc", &["demo", "smart", "Rc"], vec![primitive("str")]),
            ),
        ),
        ("2", public_item("Envelope", struct_plain(&["3"]))),
        (
            "3",
            public_field(
                "ticket",
                resolved_with_path(
                    "Box",
                    &["demo", "smart", "Box"],
                    vec![resolved("Ticket", vec![])],
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let takes_box = interop
        .function(Some("demo"), None, &ident("takes_box"), &[])
        .expect("expected imported function using dependency Box");
    assert_eq!(takes_box.arg_conversions, vec![RustArgConversion::None]);
    let TypeElement::Parametric(param_type) =
        &takes_box.decl.item.signature.parameters.params[0].param_type
    else {
        panic!(
            "expected dependency Box<T> parameter, got {:?}",
            takes_box.decl.item.signature.parameters.params[0].param_type
        );
    };
    assert_eq!(param_type.base_type, TypeIdent::new("Box"));
    assert_eq!(param_type.type_args, vec![u64_type()]);

    let returns_rc = interop
        .function(Some("demo"), None, &ident("returns_rc"), &[])
        .expect("expected imported function returning dependency Rc");
    assert_eq!(returns_rc.return_conversion, RustReturnConversion::None);
    let TypeElement::Parametric(return_type) = &returns_rc.decl.item.signature.return_type else {
        panic!(
            "expected dependency Rc<T> return, got {:?}",
            returns_rc.decl.item.signature.return_type
        );
    };
    assert_eq!(return_type.base_type, TypeIdent::new("Rc"));
    assert_eq!(return_type.type_args, vec![string_type()]);

    let TypeDecl::Struct(envelope) = imported_type(&interop, "Envelope") else {
        panic!("expected Envelope struct");
    };
    let TypeElement::Parametric(field_type) = &envelope.members[0].r#type else {
        panic!(
            "expected dependency Box<T> field, got {:?}",
            envelope.members[0].r#type
        );
    };
    assert_eq!(field_type.base_type, TypeIdent::new("Box"));
    assert_eq!(
        field_type.type_args,
        vec![plain_type(TypeIdent::new("Ticket"))]
    );
    assert_eq!(
        interop.field_return_conversion(&TypeIdent::new("Envelope"), &ident("ticket")),
        RustReturnConversion::None
    );
    assert_eq!(
        interop.field_arg_conversion(&TypeIdent::new("Envelope"), &ident("ticket")),
        RustArgConversion::None
    );
}

#[test]
fn rustdoc_lifts_box_returns_with_return_conversions() {
    let krate = crate_(vec![(
        "0",
        public_function(
            "boxed_ticket",
            vec![],
            resolved("Box", vec![resolved("Ticket", vec![])]),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![(
        "0",
        public_function(
            "shared_ticket",
            vec![],
            resolved("Rc", vec![resolved("Ticket", vec![])]),
        ),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("TicketEnvelope", struct_plain(&["1"]))),
        (
            "1",
            public_field("ticket", resolved("Box", vec![resolved("Ticket", vec![])])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("TicketCache", struct_plain(&["1"]))),
        (
            "1",
            public_field("latest", resolved("Rc", vec![resolved("Ticket", vec![])])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
fn rustdoc_suppresses_conversions_for_ambiguous_type_names() {
    let krate = crate_(vec![
        (
            "0",
            public_item_at_path(
                "Envelope",
                &["demo", "http", "Envelope"],
                struct_plain(&["1"]),
            ),
        ),
        (
            "1",
            public_field("ticket", resolved("Box", vec![resolved("Ticket", vec![])])),
        ),
        (
            "2",
            public_item_at_path(
                "Envelope",
                &["demo", "db", "Envelope"],
                struct_plain(&["3"]),
            ),
        ),
        (
            "3",
            public_field("ticket", resolved("Rc", vec![resolved("Ticket", vec![])])),
        ),
        (
            "4",
            public_item_at_path("Pair", &["demo", "http", "Pair"], struct_tuple(&["5"])),
        ),
        (
            "5",
            public_field("0", resolved("Box", vec![resolved("Ticket", vec![])])),
        ),
        (
            "6",
            public_item_at_path("Pair", &["demo", "db", "Pair"], struct_tuple(&["7"])),
        ),
        (
            "7",
            public_field("0", resolved("Rc", vec![resolved("Ticket", vec![])])),
        ),
        (
            "8",
            public_item_at_path("Event", &["demo", "http", "Event"], enum_(&["9"])),
        ),
        ("9", public_item("Assigned", variant_tuple(&["10"]))),
        (
            "10",
            public_field("0", resolved("Box", vec![resolved("Ticket", vec![])])),
        ),
        (
            "11",
            public_item_at_path("Event", &["demo", "db", "Event"], enum_(&["12"])),
        ),
        ("12", public_item("Assigned", variant_tuple(&["13"]))),
        (
            "13",
            public_field("0", resolved("Rc", vec![resolved("Ticket", vec![])])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert_eq!(
        interop.field_return_conversion(&TypeIdent::new("Envelope"), &ident("ticket")),
        RustReturnConversion::None
    );
    assert_eq!(
        interop.field_arg_conversion(&TypeIdent::new("Envelope"), &ident("ticket")),
        RustArgConversion::None
    );
    assert!(interop
        .constructor_arg_conversions(&TypeIdent::new("Pair"))
        .is_empty());
    assert_eq!(
        interop.enum_variant_arg_conversion(
            &TypeIdent::new("Event"),
            &TypeIdent::new("Assigned"),
            0,
            None,
        ),
        RustArgConversion::None
    );
    assert_eq!(
        interop.enum_variant_return_conversion(
            &TypeIdent::new("Event"),
            &TypeIdent::new("Assigned"),
            0,
            None,
        ),
        RustReturnConversion::None
    );
}

#[test]
fn rustdoc_keeps_types_with_incomplete_owned_wrapper_fields_opaque() {
    let krate = crate_(vec![
        (
            "0",
            public_item("TicketEnvelope", struct_plain(&["1", "2"])),
        ),
        ("1", public_field("ticket", resolved("Box", vec![]))),
        ("2", public_field("latest", resolved("Rc", vec![]))),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(envelope) = imported_type(&interop, "TicketEnvelope") else {
        panic!("expected TicketEnvelope to import as an opaque type");
    };
    assert_eq!(envelope.ident, TypeIdent::new("TicketEnvelope"));
}

#[test]
fn rustdoc_imports_public_struct_fields() {
    let krate = crate_(vec![
        (
            "0",
            public_item("Ticket", struct_plain(&["1", "2", "3", "4"])),
        ),
        ("1", public_field("id", primitive("u64"))),
        ("2", public_field("title", primitive("str"))),
        (
            "3",
            public_field(
                "state",
                resolved(
                    "Arc",
                    vec![resolved("RwLock", vec![resolved("TicketState", vec![])])],
                ),
            ),
        ),
        (
            "4",
            public_field(
                "draft",
                resolved(
                    "Arc",
                    vec![resolved("Mutex", vec![resolved("TicketDraft", vec![])])],
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Struct(ticket) = imported_type(&interop, "Ticket") else {
        panic!("expected Ticket struct");
    };
    assert_eq!(ticket.ident.as_str(), "Ticket");
    assert_eq!(ticket.members.len(), 4);
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
    assert_eq!(
        ticket.members[3].decl_modifier,
        Some(galvan_ast::DeclModifier::Ref)
    );
    assert_eq!(
        ticket.members[3].r#type,
        plain_type(TypeIdent::new("TicketDraft"))
    );
}

#[test]
fn rustdoc_keeps_types_with_bare_lock_fields_opaque() {
    let krate = crate_(vec![
        ("0", public_item("TicketStore", struct_plain(&["1"]))),
        (
            "1",
            public_field(
                "tickets",
                resolved("Mutex", vec![resolved("Ticket", vec![])]),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(store) = imported_type(&interop, "TicketStore") else {
        panic!("expected TicketStore to import as an opaque type");
    };
    assert_eq!(store.ident, TypeIdent::new("TicketStore"));
}

#[test]
fn rustdoc_imports_tuple_struct_fields() {
    let krate = crate_(vec![
        ("0", public_item("UserId", struct_tuple(&["1"]))),
        ("1", public_field("0", primitive("u64"))),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Tuple(user_id) = imported_type(&interop, "UserId") else {
        panic!("expected UserId tuple struct");
    };
    assert_eq!(user_id.ident.as_str(), "UserId");
    assert_eq!(user_id.members.len(), 1);
    assert_eq!(user_id.members[0].r#type, u64_type());
}

#[test]
fn rustdoc_lifts_tuple_struct_wrapper_fields() {
    let krate = crate_(vec![
        ("0", public_item("SharedTicket", struct_tuple(&["1", "2"]))),
        (
            "1",
            public_field("0", resolved("Box", vec![resolved("Ticket", vec![])])),
        ),
        (
            "2",
            public_field(
                "1",
                resolved(
                    "Arc",
                    vec![resolved("Mutex", vec![resolved("TicketState", vec![])])],
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("TicketEvent", enum_(&["1", "2", "4"]))),
        ("1", public_item("Created", variant_plain())),
        ("2", public_item("Renamed", variant_tuple(&["3"]))),
        ("3", public_field("0", primitive("str"))),
        ("4", public_item("Closed", variant_struct(&["5"]))),
        (
            "5",
            public_field("reason", resolved("Option", vec![primitive("str")])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("TicketEvent", enum_(&["1", "3"]))),
        ("1", public_item("Assigned", variant_tuple(&["2"]))),
        (
            "2",
            public_field("0", resolved("Rc", vec![resolved("User", vec![])])),
        ),
        ("3", public_item("Moved", variant_struct(&["4", "5"]))),
        (
            "4",
            public_field("queue", resolved("Option", vec![primitive("str")])),
        ),
        (
            "5",
            public_field("owner", resolved("Box", vec![resolved("User", vec![])])),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("UserId", type_alias(primitive("u64")))),
        (
            "1",
            public_item("Names", type_alias(resolved("Vec", vec![primitive("str")]))),
        ),
        (
            "2",
            public_item(
                "ParseResult",
                type_alias_generic(
                    resolved("Result", vec![generic("T"), resolved("Error", vec![])]),
                    type_generics(vec![generic_param("T")]),
                ),
            ),
        ),
        (
            "3",
            public_item(
                "FallibleTicket",
                type_alias(resolved_with_path(
                    "FlexResult",
                    &["galvan", "std", "FlexResult"],
                    vec![resolved("Ticket", vec![])],
                )),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![(
        "0",
        public_constant("DEFAULT_LIMIT", primitive("u64")),
    )]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let constant = interop
        .constant(Some("demo"), &ident("DEFAULT_LIMIT"))
        .expect("expected namespaced constant");
    assert_eq!(constant.ty, u64_type());
    assert_eq!(constant.rust_path.as_ref(), "::demo::DEFAULT_LIMIT");
    assert!(interop.constant(None, &ident("DEFAULT_LIMIT")).is_none());
}

#[test]
fn rustdoc_imports_associated_constants() {
    let krate = crate_(vec![
        ("0", public_item("StatusCode", struct_plain(&[]))),
        (
            "1",
            public_impl(resolved("StatusCode", vec![]), None, &["2"]),
        ),
        (
            "2",
            public_item_at_path(
                "CREATED",
                &["demo", "StatusCode"],
                assoc_const(resolved("StatusCode", vec![])),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
fn rustdoc_imports_associated_constants_for_reexported_types() {
    let krate = crate_(vec![
        ("0", public_item("InternalStatusCode", struct_plain(&[]))),
        (
            "1",
            public_impl(resolved("InternalStatusCode", vec![]), None, &["2"]),
        ),
        (
            "2",
            public_item_at_path(
                "CREATED",
                &["demo", "InternalStatusCode"],
                assoc_const(resolved("InternalStatusCode", vec![])),
            ),
        ),
        ("3", public_use("StatusCode", "0")),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let constant = interop
        .associated_constant(
            Some("demo"),
            &TypeIdent::new("StatusCode"),
            &ident("CREATED"),
        )
        .expect("expected re-exported associated constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::StatusCode::CREATED");
    assert_eq!(constant.ty, plain_type(TypeIdent::new("StatusCode")));
}

#[test]
fn rustdoc_imports_trait_impl_associated_constants() {
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_item("TicketKind", trait_(&[]))),
        (
            "2",
            public_impl(
                resolved_with_string_path("crate::Ticket", vec![]),
                Some(resolved_with_string_path("$crate::TicketKind", vec![])),
                &["3"],
            ),
        ),
        (
            "3",
            public_item_at_path(
                "KIND",
                &["demo", "TicketKind"],
                assoc_const(primitive("str")),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop.constant(Some("demo"), &ident("KIND")).is_none());
    let constant = interop
        .associated_constant(Some("demo"), &TypeIdent::new("Ticket"), &ident("KIND"))
        .expect("expected trait impl associated constant");
    assert_eq!(
        constant.rust_path.as_ref(),
        "<::demo::Ticket as ::demo::TicketKind>::KIND"
    );
    assert_eq!(constant.ty, string_type());
}

#[test]
fn rustdoc_imports_trait_methods_and_constants() {
    let krate = crate_(vec![
        ("0", public_item("DisplayName", trait_(&["1", "2", "3"]))),
        (
            "1",
            public_item_at_path(
                "display_name",
                &["demo", "DisplayName"],
                function_item(
                    vec![("self", borrowed(resolved("Self", vec![])))],
                    Some(primitive("str")),
                ),
            ),
        ),
        (
            "2",
            public_item_at_path(
                "clone_display",
                &["demo", "DisplayName"],
                function_item(
                    vec![("self", borrowed(resolved("Self", vec![])))],
                    Some(resolved("Self", vec![])),
                ),
            ),
        ),
        (
            "3",
            public_item_at_path(
                "KIND",
                &["demo", "DisplayName"],
                assoc_const(primitive("str")),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let TypeDecl::Empty(display_name) = imported_type(&interop, "DisplayName") else {
        panic!("expected DisplayName trait to import as an opaque type");
    };
    assert_eq!(display_name.ident, TypeIdent::new("DisplayName"));
    assert!(interop
        .function(Some("demo"), None, &ident("display_name"), &[])
        .is_none());
    assert!(interop.constant(Some("demo"), &ident("KIND")).is_none());

    let function = interop
        .function(
            Some("demo"),
            Some(&TypeIdent::new("DisplayName")),
            &ident("display_name"),
            &[],
        )
        .expect("expected imported DisplayName.display_name trait method");
    assert_eq!(
        function.rust_path.as_ref(),
        "::demo::DisplayName::display_name"
    );
    let receiver = function.decl.item.signature.receiver().unwrap();
    assert_eq!(
        receiver.param_type,
        plain_type(TypeIdent::new("DisplayName"))
    );
    assert_eq!(
        function.arg_conversions,
        vec![RustArgConversion::SharedBorrow]
    );
    assert_eq!(function.decl.item.signature.return_type, string_type());

    let clone_function = interop
        .function(
            Some("demo"),
            Some(&TypeIdent::new("DisplayName")),
            &ident("clone_display"),
            &[],
        )
        .expect("expected imported DisplayName.clone_display trait method");
    assert_eq!(
        clone_function.decl.item.signature.return_type,
        plain_type(TypeIdent::new("DisplayName"))
    );

    let constant = interop
        .associated_constant(Some("demo"), &TypeIdent::new("DisplayName"), &ident("KIND"))
        .expect("expected imported DisplayName.KIND trait constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::DisplayName::KIND");
    assert_eq!(constant.ty, string_type());
}

#[test]
fn rustdoc_suppresses_ambiguous_unqualified_associated_items() {
    let http = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        (
            "1",
            public_impl(resolved("Ticket", vec![]), None, &["2", "3"]),
        ),
        (
            "2",
            public_item_at_path(
                "new",
                &["http", "Ticket"],
                function_item(vec![], Some(resolved("Ticket", vec![]))),
            ),
        ),
        (
            "3",
            public_item_at_path(
                "DEFAULT",
                &["http", "Ticket"],
                assoc_const(resolved("Ticket", vec![])),
            ),
        ),
    ]);
    let db = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        (
            "1",
            public_impl(resolved("Ticket", vec![]), None, &["2", "3"]),
        ),
        (
            "2",
            public_item_at_path(
                "new",
                &["db", "Ticket"],
                function_item(vec![], Some(resolved("Ticket", vec![]))),
            ),
        ),
        (
            "3",
            public_item_at_path(
                "DEFAULT",
                &["db", "Ticket"],
                assoc_const(resolved("Ticket", vec![])),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("http", &http);
    interop.add_crate("db", &db);

    assert!(interop
        .associated_function(None, &TypeIdent::new("Ticket"), &ident("new"), &[])
        .is_none());
    assert!(interop
        .associated_constant(None, &TypeIdent::new("Ticket"), &ident("DEFAULT"))
        .is_none());
    assert!(interop
        .associated_function(Some("http"), &TypeIdent::new("Ticket"), &ident("new"), &[])
        .is_some());
    assert!(interop
        .associated_function(Some("db"), &TypeIdent::new("Ticket"), &ident("new"), &[])
        .is_some());
    assert!(interop
        .associated_constant(Some("http"), &TypeIdent::new("Ticket"), &ident("DEFAULT"))
        .is_some());
    assert!(interop
        .associated_constant(Some("db"), &TypeIdent::new("Ticket"), &ident("DEFAULT"))
        .is_some());
}

#[test]
fn rustdoc_imports_reexported_type_aliases() {
    let krate = crate_(vec![
        ("0", public_item("OriginalTicket", struct_plain(&["1"]))),
        ("1", public_field("title", primitive("str"))),
        ("2", public_use("Ticket", "0")),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_external_use("StatusCode", "http::StatusCode")),
        (
            "1",
            public_external_use("DEFAULT_LIMIT", "http::DEFAULT_LIMIT"),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_function("nickname", vec![], primitive("str"))),
        ("1", public_use("display_name", "0")),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_constant("DEFAULT_LIMIT", primitive("u64"))),
        ("1", public_use("LIMIT", "0")),
        (
            "2",
            public_constant("RAW_BUFFER", raw_pointer(primitive("u8"), false)),
        ),
        ("3", public_use("BUFFER", "2")),
        (
            "4",
            public_constant(
                "LOCKED_TICKETS",
                resolved("Mutex", vec![resolved("Ticket", vec![])]),
            ),
        ),
        ("5", public_use("TICKETS", "4")),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    let constant = interop
        .constant(Some("demo"), &ident("LIMIT"))
        .expect("expected re-exported constant");
    assert_eq!(constant.rust_path.as_ref(), "::demo::LIMIT");
    assert_eq!(constant.ty, u64_type());
    assert!(interop.constant(Some("demo"), &ident("BUFFER")).is_none());
    assert!(interop.constant(Some("demo"), &ident("TICKETS")).is_none());
}

#[test]
fn rustdoc_imports_glob_reexported_items() {
    let krate = crate_(vec![
        ("0", public_module("internal", &["1", "2", "4", "6", "7"])),
        ("1", public_item("Ticket", struct_plain(&["3"]))),
        (
            "2",
            public_function("display_name", vec![], primitive("str")),
        ),
        ("3", public_field("title", primitive("str"))),
        ("4", public_constant("LIMIT", primitive("u64"))),
        ("5", public_glob_use("internal", "0")),
        (
            "6",
            public_constant("RAW_BUFFER", raw_pointer(primitive("u8"), false)),
        ),
        (
            "7",
            public_constant(
                "LOCKED_TICKETS",
                resolved("RwLock", vec![resolved("Ticket", vec![])]),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    assert!(interop
        .constant(Some("demo"), &ident("RAW_BUFFER"))
        .is_none());
    assert!(interop
        .constant(Some("demo"), &ident("LOCKED_TICKETS"))
        .is_none());
}

#[test]
fn rustdoc_imports_inherent_impl_methods_with_receivers() {
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_impl(resolved("Ticket", vec![]), None, &["2"])),
        (
            "2",
            public_item_at_path(
                "rename",
                &["demo", "Ticket"],
                function_item(
                    vec![
                        ("self", mut_borrowed(resolved("Ticket", vec![]))),
                        ("title", primitive("str")),
                    ],
                    None,
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_impl(resolved("Ticket", vec![]), None, &["2"])),
        (
            "2",
            public_item_at_path(
                "new",
                &["demo", "Ticket"],
                function_item(
                    vec![
                        ("title", primitive("str")),
                        ("parent", resolved("Option", vec![resolved("Self", vec![])])),
                    ],
                    Some(resolved("Self", vec![])),
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let TypeElement::Optional(parent) =
        &function.decl.item.signature.parameters.params[1].param_type
    else {
        panic!(
            "expected Option<Ticket> parent parameter, got {:?}",
            function.decl.item.signature.parameters.params[1].param_type
        );
    };
    assert_eq!(parent.inner, plain_type(TypeIdent::new("Ticket")));
}

#[test]
fn rustdoc_does_not_import_impl_items_with_unliftable_receivers() {
    let krate = crate_(vec![
        (
            "0",
            public_impl(raw_pointer(primitive("u8"), false), None, &["1", "2"]),
        ),
        (
            "1",
            public_item_at_path(
                "from_address",
                &["demo", "PointerExt"],
                function_item(vec![], Some(primitive("bool"))),
            ),
        ),
        (
            "2",
            public_item_at_path(
                "KIND",
                &["demo", "PointerExt"],
                assoc_const(primitive("str")),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

    assert!(interop
        .function(Some("demo"), None, &ident("from_address"), &[])
        .is_none());
    assert!(interop.constant(Some("demo"), &ident("KIND")).is_none());
}

#[test]
fn rustdoc_does_not_import_unsafe_associated_functions() {
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_impl(resolved("Ticket", vec![]), None, &["2"])),
        (
            "2",
            public_item_at_path(
                "from_raw",
                &["demo", "Ticket"],
                unsafe_function_item(
                    vec![("address", primitive("usize"))],
                    Some(resolved("Ticket", vec![])),
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
    let krate = crate_(vec![
        ("0", public_item("Ticket", struct_plain(&[]))),
        ("1", public_item("DisplayName", trait_(&[]))),
        (
            "2",
            public_impl(
                resolved_with_string_path("crate::Ticket", vec![]),
                Some(resolved_with_string_path("$crate::DisplayName", vec![])),
                &["3"],
            ),
        ),
        (
            "3",
            public_item_at_path(
                "display_name",
                &["demo", "DisplayName"],
                function_item(
                    vec![(
                        "self",
                        borrowed(resolved_with_string_path("crate::Ticket", vec![])),
                    )],
                    Some(primitive("str")),
                ),
            ),
        ),
    ]);
    let mut interop = RustInterop::empty();
    interop.add_crate("demo", &krate);

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
