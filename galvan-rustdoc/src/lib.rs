use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use thiserror::Error;

use galvan_ast::{
    ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, EmptyTypeDecl, EnumTypeDecl, EnumTypeMember,
    EnumVariantField, FnDecl, FnSignature, Ident, OptionalTypeItem, OrderedDictionaryTypeItem,
    Param, ParamList, ParametricTypeItem, ResultTypeItem, SetTypeItem, Span, StructTypeDecl,
    StructTypeMember, ToplevelItem, TupleTypeDecl, TupleTypeMember, TypeDecl, TypeElement,
    TypeIdent, UseDecl, Visibility,
};
use galvan_files::Source;

#[derive(Debug, Error)]
pub enum RustdocError {
    #[error("failed to run cargo metadata: {0}")]
    CargoMetadata(std::io::Error),
    #[error("cargo metadata returned invalid JSON: {0}")]
    InvalidCargoMetadata(serde_json::Error),
    #[error("failed to read rustdoc JSON cache {0}: {1}")]
    ReadCache(PathBuf, std::io::Error),
    #[error("failed to parse rustdoc JSON cache {0}: {1}")]
    ParseCache(PathBuf, serde_json::Error),
}

#[derive(Debug, Default)]
pub struct RustInterop {
    pub types: Vec<RustTypeDecl>,
    pub functions: Vec<RustFunctionDecl>,
    by_namespace_function: HashMap<(String, RustFunctionId), usize>,
    by_imported_function: HashMap<(String, RustFunctionId), usize>,
}

impl RustInterop {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_uses(uses: &[ToplevelItem<UseDecl>]) -> Result<Self, RustdocError> {
        Self::from_crates_and_uses(imported_crates(uses), uses)
    }

    pub fn from_crates_and_uses(
        crate_names: impl IntoIterator<Item = String>,
        uses: &[ToplevelItem<UseDecl>],
    ) -> Result<Self, RustdocError> {
        let mut interop = RustInterop::default();
        let imported_crates = imported_crates(uses);
        let crate_names = crate_names
            .into_iter()
            .chain(imported_crates.iter().cloned())
            .collect::<HashSet<_>>();

        for crate_name in crate_names {
            let cache = RustdocCache::new(&crate_name);
            cache.update_if_needed();
            if let Some(path) = cache.json_path() {
                let json = fs::read_to_string(&path)
                    .map_err(|error| RustdocError::ReadCache(path.clone(), error))?;
                let json = serde_json::from_str(&json)
                    .map_err(|error| RustdocError::ParseCache(path.clone(), error))?;
                interop.add_crate(&crate_name, &json);
            } else {
                interop.add_curated_crate(&crate_name);
            }
        }
        interop.import_uses(uses);

        Ok(interop)
    }

    pub fn add_crate(&mut self, crate_name: &str, json: &Value) {
        let Some(index) = json.get("index").and_then(Value::as_object) else {
            self.add_curated_crate(crate_name);
            return;
        };

        let mut type_item_ids = Vec::new();
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if public_type_name(item).is_some() {
                type_item_ids.push(item);
            }
        }

        for item in type_item_ids {
            self.push_type_from_item(crate_name, item, index);
        }

        let impl_function_ids = impl_function_ids(index);
        let mut found_function = false;
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| impl_function_ids.contains(id))
            {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(function) = item_inner(item, "function") else {
                continue;
            };
            let Some(signature) = function.get("sig") else {
                continue;
            };
            let rust_path = callable_rust_path(crate_name, name, item);
            let decl = self.function_decl(crate_name, name, signature);
            let borrowed_return = return_is_borrowed(signature);
            self.push_function(crate_name, name, rust_path, decl, borrowed_return);
            found_function = true;
        }
        found_function |= self.import_impl_functions(crate_name, index);

        if !found_function {
            self.add_curated_crate(crate_name);
        }
    }

    pub fn add_function_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function(namespace, name, rust_path.into(), decl, borrowed_return);
    }

    pub fn function(
        &self,
        namespace: Option<&str>,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&RustFunctionDecl> {
        let id = RustFunctionId::new(receiver, name.as_str(), labels);
        if let Some(namespace) = namespace {
            return self
                .by_namespace_function
                .get(&(namespace.to_string(), id))
                .and_then(|idx| self.functions.get(*idx));
        }

        self.by_imported_function
            .get(&("".to_string(), id))
            .and_then(|idx| self.functions.get(*idx))
    }

    fn push_type(&mut self, crate_name: &str, name: &str) {
        if self.types.iter().any(|ty| ty.name.as_str() == name) {
            return;
        }

        let ident = TypeIdent::new(name);
        self.types.push(RustTypeDecl {
            name: ident.clone(),
            rust_path: format!("::{crate_name}::{name}").into(),
            decl: ToplevelItem {
                item: TypeDecl::Empty(EmptyTypeDecl {
                    visibility: Visibility::public(),
                    ident,
                    span: Span::default(),
                }),
                source: Source::Builtin,
            },
        });
    }

    fn push_type_from_item(
        &mut self,
        crate_name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        let Some(name) = public_type_name(item) else {
            return;
        };

        let decl = self
            .type_decl_from_item(crate_name, name, item, index)
            .unwrap_or_else(|| empty_type_decl(name));
        let rust_path = rust_path(crate_name, name, item);
        let type_decl = RustTypeDecl {
            name: TypeIdent::new(name),
            rust_path,
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        };

        if let Some(existing) = self.types.iter_mut().find(|ty| ty.name.as_str() == name) {
            if matches!(existing.decl.item, TypeDecl::Empty(_)) {
                *existing = type_decl;
            }
            return;
        }

        self.types.push(type_decl);
    }

    fn type_decl_from_item(
        &mut self,
        crate_name: &str,
        name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<TypeDecl> {
        let inner = item.get("inner")?;
        if let Some(struct_item) = inner.get("struct") {
            return self.struct_decl_from_json(crate_name, name, struct_item, index);
        }
        if let Some(enum_item) = inner.get("enum") {
            return Some(self.enum_decl_from_json(crate_name, name, enum_item, index));
        }

        None
    }

    fn struct_decl_from_json(
        &mut self,
        crate_name: &str,
        name: &str,
        struct_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<TypeDecl> {
        let field_ids = item_ids(struct_item, "fields");
        let kind = struct_item.get("kind").and_then(Value::as_str);
        if kind == Some("tuple") {
            let members = field_ids
                .into_iter()
                .filter_map(|id| index.get(id))
                .filter_map(|field| self.tuple_member_from_json(crate_name, field))
                .collect::<Vec<_>>();
            return Some(TypeDecl::Tuple(TupleTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                members,
                span: Span::default(),
            }));
        }

        let members = field_ids
            .into_iter()
            .filter_map(|id| index.get(id))
            .filter(|field| is_public(field))
            .filter_map(|field| self.struct_member_from_json(crate_name, field))
            .collect::<Vec<_>>();

        if members.is_empty() && kind != Some("plain") {
            return None;
        }

        Some(TypeDecl::Struct(StructTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            members,
            span: Span::default(),
        }))
    }

    fn struct_member_from_json(
        &mut self,
        crate_name: &str,
        field: &Value,
    ) -> Option<StructTypeMember> {
        let name = field.get("name").and_then(Value::as_str)?;
        let field_type = item_inner(field, "struct_field")?;
        let lifted = self.lift_type_from_json(crate_name, field_type)?;

        Some(StructTypeMember {
            decl_modifier: lifted.decl_modifier,
            ident: Ident::new(name),
            r#type: lifted.ty,
            default_value: None,
            span: Span::default(),
        })
    }

    fn tuple_member_from_json(
        &mut self,
        crate_name: &str,
        field: &Value,
    ) -> Option<TupleTypeMember> {
        let field_type = item_inner(field, "struct_field")?;
        Some(TupleTypeMember {
            r#type: self.type_from_json(crate_name, field_type)?,
            span: Span::default(),
        })
    }

    fn enum_decl_from_json(
        &mut self,
        crate_name: &str,
        name: &str,
        enum_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> TypeDecl {
        let members = item_ids(enum_item, "variants")
            .into_iter()
            .filter_map(|id| index.get(id))
            .filter_map(|variant| self.enum_member_from_json(crate_name, variant, index))
            .collect::<Vec<_>>();

        TypeDecl::Enum(EnumTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            members,
            span: Span::default(),
        })
    }

    fn enum_member_from_json(
        &mut self,
        crate_name: &str,
        variant: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<EnumTypeMember> {
        let name = variant.get("name").and_then(Value::as_str)?;
        let variant = item_inner(variant, "variant")?;
        let fields = match variant.get("kind") {
            Some(Value::String(kind)) if kind == "plain" => Vec::new(),
            Some(kind) => self.enum_variant_fields_from_kind(crate_name, kind, index),
            None => Vec::new(),
        };

        Some(EnumTypeMember {
            ident: TypeIdent::new(name),
            fields,
            span: Span::default(),
        })
    }

    fn enum_variant_fields_from_kind(
        &mut self,
        crate_name: &str,
        kind: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Vec<EnumVariantField> {
        if let Some(tuple) = inner(kind, "tuple") {
            return item_ids(tuple, "fields")
                .into_iter()
                .filter_map(|id| index.get(id))
                .filter_map(|field| self.enum_variant_field_from_json(crate_name, None, field))
                .collect();
        }

        if let Some(struct_variant) = inner(kind, "struct") {
            return item_ids(struct_variant, "fields")
                .into_iter()
                .filter_map(|id| index.get(id))
                .filter_map(|field| {
                    let name = field.get("name").and_then(Value::as_str).map(Ident::new);
                    self.enum_variant_field_from_json(crate_name, name, field)
                })
                .collect();
        }

        Vec::new()
    }

    fn enum_variant_field_from_json(
        &mut self,
        crate_name: &str,
        name: Option<Ident>,
        field: &Value,
    ) -> Option<EnumVariantField> {
        let field_type = item_inner(field, "struct_field")?;
        Some(EnumVariantField {
            name,
            r#type: self.type_from_json(crate_name, field_type)?,
            span: Span::default(),
        })
    }

    fn import_impl_functions(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
    ) -> bool {
        let mut found_function = false;
        for impl_item in index.values() {
            let Some(impl_inner) = item_inner(impl_item, "impl") else {
                continue;
            };
            if impl_inner
                .get("trait")
                .is_some_and(|trait_| !trait_.is_null())
            {
                continue;
            }

            for id in item_ids(impl_inner, "items") {
                let Some(item) = index.get(id) else {
                    continue;
                };
                if !is_public(item) {
                    continue;
                }
                let Some(name) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let Some(function) = item_inner(item, "function") else {
                    continue;
                };
                let Some(signature) = function.get("sig") else {
                    continue;
                };

                let decl = self.impl_function_decl(crate_name, name, signature, impl_inner);
                let rust_path = impl_function_rust_path(crate_name, name, item, impl_inner);
                let borrowed_return = return_is_borrowed(signature);
                self.push_function(crate_name, name, rust_path, decl, borrowed_return);
                found_function = true;
            }
        }

        found_function
    }

    fn push_function(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        let labels = decl.signature.overload_labels();
        let labels = labels
            .iter()
            .map(|label| label.as_str())
            .collect::<Vec<_>>();
        let receiver = decl
            .signature
            .receiver()
            .and_then(|param| match &param.param_type {
                TypeElement::Plain(plain) => Some(&plain.ident),
                TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                _ => None,
            });
        let id = RustFunctionId::new(receiver, name, &labels);
        let idx = self.functions.len();
        self.functions.push(RustFunctionDecl {
            namespace: crate_name.into(),
            rust_path,
            borrowed_return,
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        });
        self.by_namespace_function
            .insert((crate_name.to_string(), id.clone()), idx);
    }

    fn import_uses(&mut self, uses: &[ToplevelItem<UseDecl>]) {
        for use_decl in uses {
            let Some(namespace) = use_decl.path.segments.first() else {
                continue;
            };
            let namespace = namespace.as_str();
            match use_decl.path.segments.as_slice() {
                [_] => self.import_namespace(namespace),
                [_, item] => self.import_item(namespace, item.as_str()),
                _ => {}
            }
        }
    }

    fn import_namespace(&mut self, namespace: &str) {
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
    }

    fn import_item(&mut self, namespace: &str, name: &str) {
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            if signature.identifier.as_str() != name {
                continue;
            }
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
    }

    fn function_decl(&mut self, crate_name: &str, name: &str, signature: &Value) -> FnDecl {
        let params = signature
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|param| self.param_from_json(crate_name, param))
            .collect::<Vec<_>>();

        let return_type = signature
            .get("output")
            .filter(|output| !output.is_null())
            .and_then(|output| self.type_from_json(crate_name, output))
            .unwrap_or_else(TypeElement::void);

        FnSignature {
            visibility: Visibility::public(),
            identifier: Ident::new(name),
            parameters: ParamList {
                params,
                span: Span::default(),
            },
            return_type,
            where_clause: None,
            span: Span::default(),
        }
        .into()
    }

    fn param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<Param> {
        let pair = param.as_array()?;
        let name = pair.first().and_then(Value::as_str).unwrap_or("_");
        let ty = pair.get(1)?;
        let lifted = self.lift_type_from_json(crate_name, ty)?;
        let decl_modifier = lifted.decl_modifier.or_else(|| {
            if type_is_owned(ty) {
                Some(galvan_ast::DeclModifier::Move)
            } else {
                None
            }
        });
        let param_type = lifted.ty;

        Some(Param {
            decl_modifier,
            short_name: None,
            identifier: Ident::new(name),
            param_type,
            span: Span::default(),
        })
    }

    fn impl_function_decl(
        &mut self,
        crate_name: &str,
        name: &str,
        signature: &Value,
        impl_inner: &Value,
    ) -> FnDecl {
        let mut decl = self.function_decl(crate_name, name, signature);
        let Some(first_param) = decl.signature.parameters.params.first_mut() else {
            return decl;
        };
        if !first_param.identifier.is_self() {
            return decl;
        }

        if let Some(receiver_ty) = impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
        {
            first_param.param_type = receiver_ty;
        }

        decl
    }

    fn type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<TypeElement> {
        self.lift_type_from_json(crate_name, ty)
            .map(|lifted| lifted.ty)
    }

    fn lift_type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<LiftedType> {
        if let Some(primitive) = inner_string(ty, "primitive") {
            return Some(LiftedType::new(primitive_type(primitive)));
        }
        if let Some(generic) = inner_string(ty, "generic") {
            return Some(LiftedType::new(generic_type(generic)));
        }
        if let Some(borrowed) = inner(ty, "borrowed_ref") {
            let mut lifted = borrowed
                .get("type")
                .and_then(|inner| self.lift_type_from_json(crate_name, inner))?;
            if borrowed_ref_is_mutable(borrowed) {
                lifted.decl_modifier = Some(galvan_ast::DeclModifier::Mut);
            }
            return Some(lifted);
        }
        if let Some(resolved) = inner(ty, "resolved_path") {
            let name = resolved.get("name").and_then(Value::as_str)?;
            let args = resolved_type_args(resolved)
                .into_iter()
                .filter_map(|arg| self.lift_type_from_json(crate_name, arg))
                .collect::<Vec<_>>();

            if let Some(lifted) = self.lift_known_resolved_type(name, args.as_slice()) {
                return Some(lifted);
            }

            self.push_type(crate_name, name);
            return Some(LiftedType::new(parametric_or_plain_type(name, args)));
        }
        if let Some(tuple) = inner(ty, "tuple").and_then(Value::as_array) {
            return Some(LiftedType::new(TypeElement::Tuple(Box::new(
                galvan_ast::TupleTypeItem {
                    elements: tuple
                        .iter()
                        .filter_map(|ty| self.type_from_json(crate_name, ty))
                        .collect(),
                    span: Span::default(),
                },
            ))));
        }

        Some(LiftedType::new(TypeElement::infer()))
    }

    fn lift_known_resolved_type(&mut self, name: &str, args: &[LiftedType]) -> Option<LiftedType> {
        match name {
            "Option" => Some(LiftedType::new(TypeElement::Optional(Box::new(
                OptionalTypeItem {
                    inner: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                },
            )))),
            "Result" => Some(LiftedType::new(TypeElement::Result(Box::new(
                ResultTypeItem {
                    success: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    error: args
                        .get(1)
                        .map(|arg| arg.ty.clone())
                        .or_else(|| Some(plain_type(TypeIdent::new("__UnknownRustError")))),
                    span: Span::default(),
                },
            )))),
            "Vec" => Some(LiftedType::new(TypeElement::Array(Box::new(
                ArrayTypeItem {
                    elements: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                },
            )))),
            "HashSet" => Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                elements: args
                    .first()
                    .map(|arg| arg.ty.clone())
                    .unwrap_or_else(TypeElement::infer),
                span: Span::default(),
            })))),
            "HashMap" => Some(LiftedType::new(TypeElement::Dictionary(Box::new(
                DictionaryTypeItem {
                    key: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    value: args
                        .get(1)
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                },
            )))),
            "BTreeMap" | "IndexMap" => Some(LiftedType::new(TypeElement::OrderedDictionary(
                Box::new(OrderedDictionaryTypeItem {
                    key: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    value: args
                        .get(1)
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                }),
            ))),
            "Arc" => lift_arc(args.first()),
            "Mutex" => lift_ref(args.first()),
            atomic if atomic_type(atomic).is_some() => Some(LiftedType::with_modifier(
                atomic_type(atomic).unwrap(),
                galvan_ast::DeclModifier::Ref,
            )),
            _ => None,
        }
    }

    fn add_curated_crate(&mut self, crate_name: &str) {
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
        );
    }
}

#[derive(Debug)]
pub struct RustTypeDecl {
    pub name: TypeIdent,
    pub rust_path: Box<str>,
    pub decl: ToplevelItem<TypeDecl>,
}

#[derive(Debug)]
pub struct RustFunctionDecl {
    pub namespace: Box<str>,
    pub rust_path: Box<str>,
    pub borrowed_return: bool,
    pub decl: ToplevelItem<FnDecl>,
}

#[derive(Clone, Debug)]
struct LiftedType {
    ty: TypeElement,
    decl_modifier: Option<galvan_ast::DeclModifier>,
}

impl LiftedType {
    fn new(ty: TypeElement) -> Self {
        Self {
            ty,
            decl_modifier: None,
        }
    }

    fn with_modifier(ty: TypeElement, decl_modifier: galvan_ast::DeclModifier) -> Self {
        Self {
            ty,
            decl_modifier: Some(decl_modifier),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RustFunctionId(Box<str>);

impl RustFunctionId {
    fn new(receiver: Option<&TypeIdent>, name: &str, labels: &[&str]) -> Self {
        let mut id = String::new();
        if let Some(receiver) = receiver {
            id.push_str(receiver.as_str());
            id.push_str("::");
        }
        id.push_str(name);
        if !labels.is_empty() {
            id.push(':');
            id.push_str(&labels.join(":"));
        }
        Self(id.into())
    }
}

struct RustdocCache {
    crate_name: String,
    root: PathBuf,
}

impl RustdocCache {
    fn new(crate_name: &str) -> Self {
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            crate_name: crate_name.to_string(),
            root: manifest_dir
                .join("target")
                .join("galvan")
                .join("rustdoc-json"),
        }
    }

    fn json_path(&self) -> Option<PathBuf> {
        let path = self.root.join(format!("{}.json", self.crate_name));
        path.exists().then_some(path)
    }

    fn update_if_needed(&self) {
        if self.json_path().is_some() {
            self.clear_diagnostics();
            return;
        }
        if env::var_os("GALVAN_RUSTDOC_CACHE_UPDATING").is_some() {
            return;
        }

        let _ = fs::create_dir_all(&self.root);
        let manifest_path = match dependency_manifest_path(&self.crate_name) {
            Ok(Some(path)) => path,
            Ok(None) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    format!(
                        "crate '{}' was not found in cargo metadata",
                        self.crate_name
                    ),
                );
                return;
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
                return;
            }
        };

        let target_dir = self.root.join("target");
        let output = Command::new("rustup")
            .arg("run")
            .arg("nightly")
            .arg("cargo")
            .arg("rustdoc")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir)
            .arg("--")
            .arg("-Z")
            .arg("unstable-options")
            .arg("--output-format")
            .arg("json")
            .env("GALVAN_RUSTDOC_CACHE_UPDATING", "1")
            .env_remove("RUSTC")
            .env_remove("RUSTDOC")
            .env_remove("RUSTC_WRAPPER")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let generated = target_dir
                    .join("doc")
                    .join(format!("{}.json", self.crate_name));
                let cached = self.root.join(format!("{}.json", self.crate_name));
                if fs::copy(&generated, &cached).is_ok() {
                    self.clear_diagnostics();
                } else {
                    let _ = fs::write(
                        self.root.join(format!("{}.stderr", self.crate_name)),
                        format!(
                            "rustdoc succeeded but {} was not found\n{}",
                            generated.display(),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    );
                }
            }
            Ok(output) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    String::from_utf8_lossy(&output.stderr).as_ref(),
                );
                let _ = fs::write(
                    self.root.join(format!("{}.stdout", self.crate_name)),
                    String::from_utf8_lossy(&output.stdout).as_ref(),
                );
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
            }
        }
    }

    fn clear_diagnostics(&self) {
        let _ = fs::remove_file(self.root.join(format!("{}.stderr", self.crate_name)));
        let _ = fs::remove_file(self.root.join(format!("{}.stdout", self.crate_name)));
    }
}

fn dependency_manifest_path(crate_name: &str) -> Result<Option<PathBuf>, RustdocError> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(RustdocError::CargoMetadata)?;

    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(RustdocError::InvalidCargoMetadata)?;
    let manifest_path = metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| package.get("name").and_then(Value::as_str) == Some(crate_name))
        .and_then(|package| package.get("manifest_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from);

    Ok(manifest_path)
}

fn imported_crates(uses: &[ToplevelItem<UseDecl>]) -> HashSet<String> {
    uses.iter()
        .filter_map(|use_decl| use_decl.path.segments.first())
        .map(|segment| segment.as_str().to_string())
        .collect()
}

fn inner<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => object.get(key),
        _ => None,
    }
}

fn inner_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    inner(value, key).and_then(Value::as_str)
}

fn item_inner<'a>(item: &'a Value, key: &str) -> Option<&'a Value> {
    item.get("inner").and_then(|inner| inner.get(key))
}

fn is_public(item: &Value) -> bool {
    item.get("visibility")
        .is_some_and(|visibility| match visibility {
            Value::String(value) => value == "public",
            Value::Object(object) => object.contains_key("public"),
            _ => false,
        })
}

fn public_type_name(item: &Value) -> Option<&str> {
    let name = item.get("name").and_then(Value::as_str)?;
    let inner = item.get("inner")?;
    ["struct", "enum", "type_alias", "union"]
        .iter()
        .any(|kind| inner.get(*kind).is_some())
        .then_some(name)
}

fn empty_type_decl(name: &str) -> TypeDecl {
    TypeDecl::Empty(EmptyTypeDecl {
        visibility: Visibility::public(),
        ident: TypeIdent::new(name),
        span: Span::default(),
    })
}

fn item_ids<'a>(item: &'a Value, key: &str) -> Vec<&'a str> {
    item.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn impl_function_ids(index: &serde_json::Map<String, Value>) -> HashSet<&str> {
    index
        .values()
        .filter_map(|item| item_inner(item, "impl"))
        .flat_map(|impl_item| item_ids(impl_item, "items"))
        .collect()
}

fn rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    item.get("path")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("::")
        })
        .filter(|path| !path.is_empty())
        .map(|path| format!("::{path}").into())
        .unwrap_or_else(|| format!("::{crate_name}::{name}").into())
}

fn callable_rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    let path = rust_path(crate_name, name, item);
    if path.ends_with(&format!("::{name}")) {
        path
    } else {
        format!("{path}::{name}").into()
    }
}

fn impl_function_rust_path(
    crate_name: &str,
    name: &str,
    item: &Value,
    impl_inner: &Value,
) -> Box<str> {
    let path = callable_rust_path(crate_name, name, item);
    if path.matches("::").count() > 2 {
        return path;
    };

    let Some(receiver) = impl_inner.get("for").and_then(resolved_type_name) else {
        return path;
    };
    format!("::{crate_name}::{receiver}::{name}").into()
}

fn resolved_type_name(ty: &Value) -> Option<&str> {
    inner(ty, "resolved_path").and_then(|resolved| resolved.get("name").and_then(Value::as_str))
}

fn return_is_borrowed(signature: &Value) -> bool {
    signature
        .get("output")
        .is_some_and(|output| inner(output, "borrowed_ref").is_some())
}

fn type_is_owned(ty: &Value) -> bool {
    inner(ty, "borrowed_ref").is_none()
}

fn borrowed_ref_is_mutable(borrowed: &Value) -> bool {
    borrowed.get("mutable").and_then(Value::as_bool) == Some(true)
        || borrowed.get("mutability").and_then(Value::as_str) == Some("mut")
}

fn resolved_type_args(resolved: &Value) -> Vec<&Value> {
    resolved
        .get("args")
        .and_then(|args| inner(args, "angle_bracketed"))
        .and_then(|args| args.get("args"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|arg| inner(arg, "type"))
        .collect()
}

fn parametric_or_plain_type(name: &str, args: Vec<LiftedType>) -> TypeElement {
    if args.is_empty() {
        return plain_type(TypeIdent::new(name));
    }

    TypeElement::Parametric(ParametricTypeItem {
        base_type: TypeIdent::new(name),
        type_args: args.into_iter().map(|arg| arg.ty).collect(),
        span: Span::default(),
    })
}

fn lift_arc(inner: Option<&LiftedType>) -> Option<LiftedType> {
    let inner = inner?;
    match &inner.ty {
        TypeElement::Parametric(parametric) if parametric.base_type.as_str() == "Mutex" => {
            parametric
                .type_args
                .first()
                .cloned()
                .map(|ty| LiftedType::with_modifier(ty, galvan_ast::DeclModifier::Ref))
        }
        _ if inner.decl_modifier == Some(galvan_ast::DeclModifier::Ref) => Some(inner.clone()),
        TypeElement::Plain(plain) => atomic_type(plain.ident.as_str())
            .map(|ty| LiftedType::with_modifier(ty, galvan_ast::DeclModifier::Ref)),
        _ => Some(LiftedType::new(TypeElement::Parametric(
            ParametricTypeItem {
                base_type: TypeIdent::new("Arc"),
                type_args: vec![inner.ty.clone()],
                span: Span::default(),
            },
        ))),
    }
}

fn lift_ref(inner: Option<&LiftedType>) -> Option<LiftedType> {
    let inner = inner?;
    Some(LiftedType::with_modifier(
        inner.ty.clone(),
        galvan_ast::DeclModifier::Ref,
    ))
}

fn atomic_type(name: &str) -> Option<TypeElement> {
    let galvan = match name {
        "AtomicBool" => "Bool",
        "AtomicI8" => "I8",
        "AtomicI16" => "I16",
        "AtomicI32" => "I32",
        "AtomicI64" => "I64",
        "AtomicIsize" => "ISize",
        "AtomicU8" => "U8",
        "AtomicU16" => "U16",
        "AtomicU32" => "U32",
        "AtomicU64" => "U64",
        "AtomicUsize" => "USize",
        _ => return None,
    };
    Some(plain_type(TypeIdent::new(galvan)))
}

fn plain_type(ident: TypeIdent) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident,
        span: Span::default(),
    })
}

fn generic_type(name: &str) -> TypeElement {
    TypeElement::Generic(galvan_ast::GenericTypeItem {
        ident: Ident::new(name),
        span: Span::default(),
    })
}

fn primitive_type(name: &str) -> TypeElement {
    let galvan = match name {
        "bool" => "Bool",
        "i8" => "I8",
        "i16" => "I16",
        "i32" => "I32",
        "i64" => "I64",
        "i128" => "I128",
        "isize" => "ISize",
        "u8" => "U8",
        "u16" => "U16",
        "u32" => "U32",
        "u64" => "U64",
        "u128" => "U128",
        "usize" => "USize",
        "f32" => "Float",
        "f64" => "Double",
        "char" => "Char",
        "str" => "String",
        _ => "__UnknownRustPrimitive",
    };
    plain_type(TypeIdent::new(galvan))
}

#[cfg(test)]
mod tests {
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

    fn mut_borrowed(ty: Value) -> Value {
        json!({
            "borrowed_ref": {
                "type": ty,
                "mutable": true
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

        let function = interop
            .function(Some("demo"), None, &ident("new"), &[])
            .expect("expected imported Ticket.new associated function");
        assert_eq!(function.rust_path.as_ref(), "::demo::Ticket::new");
        assert!(function.decl.item.signature.receiver().is_none());
        assert_eq!(
            function.decl.item.signature.return_type,
            plain_type(TypeIdent::new("Ticket"))
        );
    }
}
