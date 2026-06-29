use std::collections::{HashMap, HashSet};
use std::fs;

use serde_json::Value;

use galvan_ast::{
    AliasTypeDecl, ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, EmptyTypeDecl, EnumTypeDecl,
    EnumTypeMember, EnumVariantField, FnDecl, FnSignature, Ident, OptionalTypeItem,
    OrderedDictionaryTypeItem, Param, ParamList, ParametricTypeItem, ResultTypeItem, SetTypeItem,
    Span, StructTypeDecl, StructTypeMember, ToplevelItem, TupleTypeDecl, TupleTypeMember, TypeDecl,
    TypeElement, TypeIdent, UseDecl, Visibility,
};
use galvan_files::Source;

use crate::cache::RustdocCache;
use crate::model::{
    RustArgConversion, RustConstantDecl, RustFieldConversion, RustFunctionDecl,
    RustReturnConversion, RustTypeDecl,
};
use crate::RustdocError;

#[derive(Debug, Default)]
pub struct RustInterop {
    pub types: Vec<RustTypeDecl>,
    pub functions: Vec<RustFunctionDecl>,
    pub constants: Vec<RustConstantDecl>,
    by_imported_type: HashMap<TypeIdent, usize>,
    by_namespace_function: HashMap<(String, RustFunctionId), usize>,
    by_imported_function: HashMap<(String, RustFunctionId), usize>,
    by_namespace_associated_function: HashMap<(String, TypeIdent, RustFunctionId), usize>,
    by_namespace_constant: HashMap<(String, Ident), usize>,
    by_imported_constant: HashMap<Ident, usize>,
    by_namespace_associated_constant: HashMap<(String, TypeIdent, Ident), usize>,
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
        let impl_constant_ids = impl_constant_ids(index);
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
            let imported = self.function_decl(crate_name, name, signature);
            let borrowed_return = return_is_borrowed(signature);
            self.push_function(
                crate_name,
                name,
                rust_path,
                imported.decl,
                borrowed_return,
                imported.return_conversion,
                imported.arg_conversions,
            );
            found_function = true;
        }
        self.import_top_level_constants(crate_name, index, &impl_constant_ids);
        found_function |= self.import_impl_functions(crate_name, index);
        found_function |= self.import_public_reexports(crate_name, index);

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
        self.push_function(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            RustReturnConversion::None,
            Vec::new(),
        );
    }

    pub fn add_type_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: TypeDecl,
    ) {
        let type_decl = RustTypeDecl {
            namespace: namespace.into(),
            name: TypeIdent::new(name),
            rust_path: rust_path.into(),
            field_conversions: Vec::new(),
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        };
        if let Some(existing) = self.types.iter_mut().find(|ty| ty.name.as_str() == name) {
            *existing = type_decl;
        } else {
            self.types.push(type_decl);
        }
    }

    pub fn add_constant_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, None, name, rust_path.into(), ty);
    }

    pub fn add_associated_constant_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, Some(receiver), name, rust_path.into(), ty);
    }

    pub fn add_associated_function_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function_with_associated_receiver(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            Some(receiver),
            RustReturnConversion::None,
            Vec::new(),
        );
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

    pub fn associated_function(
        &self,
        namespace: Option<&str>,
        receiver: &TypeIdent,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&RustFunctionDecl> {
        let id = RustFunctionId::new(None, name.as_str(), labels);
        if let Some(namespace) = namespace {
            return self
                .by_namespace_associated_function
                .get(&(namespace.to_string(), receiver.clone(), id))
                .and_then(|idx| self.functions.get(*idx));
        }

        self.by_namespace_associated_function
            .iter()
            .find(|((_, stored_receiver, stored_id), _)| {
                stored_receiver == receiver && stored_id == &id
            })
            .and_then(|(_, idx)| self.functions.get(*idx))
    }

    pub fn imported_types(&self) -> impl Iterator<Item = &RustTypeDecl> {
        self.by_imported_type
            .values()
            .filter_map(|idx| self.types.get(*idx))
    }

    pub fn field_return_conversion(
        &self,
        receiver: &TypeIdent,
        field: &Ident,
    ) -> RustReturnConversion {
        self.types
            .iter()
            .find(|ty| ty.name == *receiver)
            .and_then(|ty| {
                ty.field_conversions
                    .iter()
                    .find(|conversion| conversion.field == *field)
            })
            .map(|conversion| conversion.return_conversion)
            .unwrap_or_default()
    }

    pub fn constant(&self, namespace: Option<&str>, name: &Ident) -> Option<&RustConstantDecl> {
        if let Some(namespace) = namespace {
            return self
                .by_namespace_constant
                .get(&(namespace.to_string(), name.clone()))
                .and_then(|idx| self.constants.get(*idx));
        }

        self.by_imported_constant
            .get(name)
            .and_then(|idx| self.constants.get(*idx))
    }

    pub fn associated_constant(
        &self,
        namespace: Option<&str>,
        receiver: &TypeIdent,
        name: &Ident,
    ) -> Option<&RustConstantDecl> {
        if let Some(namespace) = namespace {
            return self
                .by_namespace_associated_constant
                .get(&(namespace.to_string(), receiver.clone(), name.clone()))
                .and_then(|idx| self.constants.get(*idx));
        }

        self.by_namespace_associated_constant
            .iter()
            .find(|((_, stored_receiver, stored_name), _)| {
                stored_receiver == receiver && stored_name == name
            })
            .and_then(|(_, idx)| self.constants.get(*idx))
    }

    fn push_type(&mut self, crate_name: &str, name: &str) {
        if self.types.iter().any(|ty| ty.name.as_str() == name) {
            return;
        }

        let ident = TypeIdent::new(name);
        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: ident.clone(),
            rust_path: format!("::{crate_name}::{name}").into(),
            field_conversions: Vec::new(),
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

        let imported = self
            .type_decl_from_item(crate_name, name, item, index)
            .unwrap_or_else(|| ImportedTypeDecl::empty(name));
        let rust_path = rust_path(crate_name, name, item);
        let type_decl = RustTypeDecl {
            namespace: crate_name.into(),
            name: TypeIdent::new(name),
            rust_path,
            field_conversions: imported.field_conversions,
            decl: ToplevelItem {
                item: imported.decl,
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

    fn push_reexported_type_from_item(
        &mut self,
        crate_name: &str,
        exported_name: &str,
        rust_path: Box<str>,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        if self
            .types
            .iter()
            .any(|ty| ty.name.as_str() == exported_name)
        {
            return;
        }

        let imported = self
            .type_decl_from_item(crate_name, exported_name, item, index)
            .unwrap_or_else(|| ImportedTypeDecl::empty(exported_name));
        self.types.push(RustTypeDecl {
            namespace: crate_name.into(),
            name: TypeIdent::new(exported_name),
            rust_path,
            field_conversions: imported.field_conversions,
            decl: ToplevelItem {
                item: imported.decl,
                source: Source::Builtin,
            },
        });
    }

    fn type_decl_from_item(
        &mut self,
        crate_name: &str,
        name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<ImportedTypeDecl> {
        let inner = item.get("inner")?;
        if let Some(struct_item) = inner.get("struct") {
            return self.struct_decl_from_json(crate_name, name, struct_item, index);
        }
        if let Some(enum_item) = inner.get("enum") {
            return Some(ImportedTypeDecl::new(
                self.enum_decl_from_json(crate_name, name, enum_item, index),
            ));
        }
        if let Some(alias_item) = inner.get("type_alias") {
            return self
                .alias_decl_from_json(crate_name, name, alias_item)
                .map(ImportedTypeDecl::new);
        }

        None
    }

    fn alias_decl_from_json(
        &mut self,
        crate_name: &str,
        name: &str,
        alias_item: &Value,
    ) -> Option<TypeDecl> {
        Some(TypeDecl::Alias(AliasTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            r#type: self.type_from_json(crate_name, alias_item)?,
            span: Span::default(),
        }))
    }

    fn struct_decl_from_json(
        &mut self,
        crate_name: &str,
        name: &str,
        struct_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<ImportedTypeDecl> {
        let field_ids = item_ids(struct_item, "fields");
        let kind = struct_item.get("kind").and_then(Value::as_str);
        if kind == Some("tuple") {
            let members = field_ids
                .into_iter()
                .filter_map(|id| index.get(id))
                .filter_map(|field| self.tuple_member_from_json(crate_name, field))
                .collect::<Vec<_>>();
            return Some(ImportedTypeDecl::new(TypeDecl::Tuple(TupleTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                members,
                span: Span::default(),
            })));
        }

        let lifted_members = field_ids
            .into_iter()
            .filter_map(|id| index.get(id))
            .filter(|field| is_public(field))
            .filter_map(|field| self.struct_member_from_json(crate_name, field))
            .collect::<Vec<_>>();
        let mut members = Vec::new();
        let mut field_conversions = Vec::new();
        for member in lifted_members {
            if member.return_conversion != RustReturnConversion::None {
                field_conversions.push(RustFieldConversion {
                    field: member.member.ident.clone(),
                    return_conversion: member.return_conversion,
                });
            }
            members.push(member.member);
        }

        if members.is_empty() && kind != Some("plain") {
            return None;
        }

        Some(ImportedTypeDecl {
            decl: TypeDecl::Struct(StructTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                members,
                span: Span::default(),
            }),
            field_conversions,
        })
    }

    fn struct_member_from_json(
        &mut self,
        crate_name: &str,
        field: &Value,
    ) -> Option<LiftedStructMember> {
        let name = field.get("name").and_then(Value::as_str)?;
        let field_type = item_inner(field, "struct_field")?;
        let lifted = self.lift_return_type_from_json(crate_name, field_type)?;

        Some(LiftedStructMember {
            member: StructTypeMember {
                decl_modifier: lifted.decl_modifier,
                ident: Ident::new(name),
                r#type: lifted.ty,
                default_value: None,
                span: Span::default(),
            },
            return_conversion: lifted.return_conversion,
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
            self.import_impl_constants(crate_name, impl_inner, index);

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

                let associated_receiver = impl_inner
                    .get("for")
                    .and_then(|ty| self.type_from_json(crate_name, ty))
                    .and_then(|ty| receiver_type_ident(&ty));
                let imported = self.impl_function_decl(crate_name, name, signature, impl_inner);
                let rust_path = impl_function_rust_path(crate_name, name, item, impl_inner);
                let borrowed_return = return_is_borrowed(signature);
                self.push_function_with_associated_receiver(
                    crate_name,
                    name,
                    rust_path,
                    imported.decl,
                    borrowed_return,
                    associated_receiver,
                    imported.return_conversion,
                    imported.arg_conversions,
                );
                found_function = true;
            }
        }

        found_function
    }

    fn import_public_reexports(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
    ) -> bool {
        let mut found_function = false;
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            let Some(use_item) = item_inner(item, "use") else {
                continue;
            };
            if use_item.get("is_glob").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let Some(exported_name) = item
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| use_item.get("name").and_then(Value::as_str))
            else {
                continue;
            };
            let Some(target_id) = use_item.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(target) = index.get(target_id) else {
                continue;
            };
            let rust_path = callable_rust_path(crate_name, exported_name, item);

            if public_type_name(target).is_some() {
                self.push_reexported_type_from_item(
                    crate_name,
                    exported_name,
                    rust_path,
                    target,
                    index,
                );
                continue;
            }

            if let Some(function) = item_inner(target, "function") {
                if let Some(signature) = function.get("sig") {
                    let imported = self.function_decl(crate_name, exported_name, signature);
                    let borrowed_return = return_is_borrowed(signature);
                    self.push_function(
                        crate_name,
                        exported_name,
                        rust_path,
                        imported.decl,
                        borrowed_return,
                        imported.return_conversion,
                        imported.arg_conversions,
                    );
                    found_function = true;
                }
                continue;
            }

            if let Some(constant) = constant_inner(target) {
                let Some(ty) =
                    constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
                else {
                    continue;
                };
                self.push_constant(crate_name, None, exported_name, rust_path, ty);
            }
        }
        found_function
    }

    fn import_top_level_constants(
        &mut self,
        crate_name: &str,
        index: &serde_json::Map<String, Value>,
        impl_constant_ids: &HashSet<&str>,
    ) {
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| impl_constant_ids.contains(id))
            {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(constant) = constant_inner(item) else {
                continue;
            };
            let Some(ty) =
                constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
            else {
                continue;
            };
            self.push_constant(
                crate_name,
                None,
                name,
                callable_rust_path(crate_name, name, item),
                ty,
            );
        }
    }

    fn import_impl_constants(
        &mut self,
        crate_name: &str,
        impl_inner: &Value,
        index: &serde_json::Map<String, Value>,
    ) {
        let receiver = impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
            .and_then(|ty| receiver_type_ident(&ty));

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
            let Some(constant) = constant_inner(item) else {
                continue;
            };
            let Some(ty) =
                constant_type(constant).and_then(|ty| self.type_from_json(crate_name, ty))
            else {
                continue;
            };
            let rust_path = impl_constant_rust_path(crate_name, name, item, impl_inner);
            self.push_constant(crate_name, receiver.clone(), name, rust_path, ty);
        }
    }

    fn push_function(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
        return_conversion: RustReturnConversion,
        arg_conversions: Vec<RustArgConversion>,
    ) {
        self.push_function_with_associated_receiver(
            crate_name,
            name,
            rust_path,
            decl,
            borrowed_return,
            None,
            return_conversion,
            arg_conversions,
        );
    }

    fn push_constant(
        &mut self,
        crate_name: &str,
        associated_receiver: Option<TypeIdent>,
        name: &str,
        rust_path: Box<str>,
        ty: TypeElement,
    ) {
        let idx = self.constants.len();
        let ident = Ident::new(name);
        self.constants.push(RustConstantDecl {
            namespace: crate_name.into(),
            associated_receiver: associated_receiver.clone(),
            name: ident.clone(),
            rust_path,
            ty,
        });

        if let Some(receiver) = associated_receiver {
            self.by_namespace_associated_constant
                .insert((crate_name.to_string(), receiver, ident), idx);
        } else {
            self.by_namespace_constant
                .insert((crate_name.to_string(), ident), idx);
        }
    }

    fn push_function_with_associated_receiver(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
        associated_receiver: Option<TypeIdent>,
        return_conversion: RustReturnConversion,
        arg_conversions: Vec<RustArgConversion>,
    ) {
        let labels = decl.signature.overload_labels();
        let labels = labels
            .iter()
            .map(|label| label.as_str())
            .collect::<Vec<_>>();
        let has_receiver = decl
            .signature
            .receiver()
            .and_then(|param| receiver_type_ident(&param.param_type))
            .is_some();
        let id = RustFunctionId::new(
            decl.signature
                .receiver()
                .and_then(|param| receiver_type_ident(&param.param_type))
                .as_ref(),
            name,
            &labels,
        );
        let idx = self.functions.len();
        self.functions.push(RustFunctionDecl {
            namespace: crate_name.into(),
            rust_path,
            borrowed_return,
            return_conversion,
            arg_conversions,
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        });
        if !has_receiver {
            if let Some(associated_receiver) = associated_receiver {
                self.by_namespace_associated_function.insert(
                    (crate_name.to_string(), associated_receiver, id.clone()),
                    idx,
                );
            } else {
                self.by_namespace_function
                    .insert((crate_name.to_string(), id.clone()), idx);
            }
        } else {
            self.by_namespace_function
                .insert((crate_name.to_string(), id.clone()), idx);
        }
    }

    pub fn import_uses(&mut self, uses: &[ToplevelItem<UseDecl>]) {
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
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            self.by_imported_type.insert(ty.name.clone(), idx);
        }
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
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            self.by_imported_constant.insert(constant.name.clone(), idx);
        }
    }

    fn import_item(&mut self, namespace: &str, name: &str) {
        for (idx, ty) in self.types.iter().enumerate() {
            if ty.namespace.as_ref() != namespace {
                continue;
            }
            if ty.name.as_str() != name {
                continue;
            }
            self.by_imported_type.insert(ty.name.clone(), idx);
        }
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
        for (idx, constant) in self.constants.iter().enumerate() {
            if constant.namespace.as_ref() != namespace || constant.associated_receiver.is_some() {
                continue;
            }
            if constant.name.as_str() != name {
                continue;
            }
            self.by_imported_constant.insert(constant.name.clone(), idx);
        }
    }

    fn function_decl(
        &mut self,
        crate_name: &str,
        name: &str,
        signature: &Value,
    ) -> ImportedFunctionDecl {
        let lifted_params = signature
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|param| self.lift_param_from_json(crate_name, param))
            .collect::<Vec<_>>();
        let params = lifted_params
            .iter()
            .map(|param| param.param.clone())
            .collect::<Vec<_>>();
        let arg_conversions = lifted_params
            .iter()
            .map(|param| param.arg_conversion)
            .collect::<Vec<_>>();

        let return_type = signature
            .get("output")
            .filter(|output| !output.is_null())
            .and_then(|output| self.lift_return_type_from_json(crate_name, output));
        let (return_type, return_conversion) = return_type
            .map(|lifted| (lifted.ty, lifted.return_conversion))
            .unwrap_or_else(|| (TypeElement::void(), RustReturnConversion::None));

        let decl = FnSignature {
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
        .into();

        ImportedFunctionDecl {
            decl,
            return_conversion,
            arg_conversions,
        }
    }

    #[cfg(test)]
    fn param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<Param> {
        self.lift_param_from_json(crate_name, param)
            .map(|param| param.param)
    }

    fn lift_param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<LiftedParam> {
        let pair = param.as_array()?;
        let name = pair.first().and_then(Value::as_str).unwrap_or("_");
        let ty = pair.get(1)?;
        let lifted = self
            .lift_param_wrapper_type_from_json(crate_name, ty)
            .or_else(|| self.lift_type_from_json(crate_name, ty))?;
        let decl_modifier = lifted.decl_modifier.or_else(|| {
            if type_is_owned(ty) {
                Some(galvan_ast::DeclModifier::Move)
            } else {
                None
            }
        });
        let param_type = lifted.ty;

        Some(LiftedParam {
            param: Param {
                decl_modifier,
                short_name: None,
                identifier: Ident::new(name),
                param_type,
                span: Span::default(),
            },
            arg_conversion: lifted.arg_conversion,
        })
    }

    fn lift_param_wrapper_type_from_json(
        &mut self,
        crate_name: &str,
        ty: &Value,
    ) -> Option<LiftedType> {
        let resolved = inner(ty, "resolved_path")?;
        let name = resolved.get("name").and_then(Value::as_str)?;
        let conversion = match name {
            "Box" => RustArgConversion::BoxNew,
            "Rc" => RustArgConversion::RcNew,
            _ => return None,
        };
        let arg = resolved_type_args(resolved).into_iter().next()?;
        let mut lifted = self.lift_type_from_json(crate_name, arg)?;
        lifted.arg_conversion = conversion;
        Some(lifted)
    }

    fn lift_return_type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<LiftedReturn> {
        if let Some(resolved) = inner(ty, "resolved_path") {
            let name = resolved.get("name").and_then(Value::as_str)?;
            if name == "Box" {
                let arg = resolved_type_args(resolved).into_iter().next()?;
                let lifted = self.lift_type_from_json(crate_name, arg)?;
                return Some(LiftedReturn {
                    ty: lifted.ty,
                    decl_modifier: lifted.decl_modifier,
                    return_conversion: RustReturnConversion::BoxDeref,
                });
            }
        }

        self.lift_type_from_json(crate_name, ty)
            .map(|lifted| LiftedReturn {
                ty: lifted.ty,
                decl_modifier: lifted.decl_modifier,
                return_conversion: RustReturnConversion::None,
            })
    }

    fn impl_function_decl(
        &mut self,
        crate_name: &str,
        name: &str,
        signature: &Value,
        impl_inner: &Value,
    ) -> ImportedFunctionDecl {
        let mut imported = self.function_decl(crate_name, name, signature);
        let Some(first_param) = imported.decl.signature.parameters.params.first_mut() else {
            return imported;
        };
        if !first_param.identifier.is_self() {
            return imported;
        }

        if let Some(receiver_ty) = impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
        {
            first_param.param_type = receiver_ty;
        }

        imported
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
            } else {
                lifted.arg_conversion = RustArgConversion::SharedBorrow;
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
            "Vec" | "VecDeque" | "LinkedList" => Some(LiftedType::new(TypeElement::Array(
                Box::new(ArrayTypeItem {
                    elements: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                }),
            ))),
            "HashSet" | "BTreeSet" | "IndexSet" => {
                Some(LiftedType::new(TypeElement::Set(Box::new(SetTypeItem {
                    elements: args
                        .first()
                        .map(|arg| arg.ty.clone())
                        .unwrap_or_else(TypeElement::infer),
                    span: Span::default(),
                }))))
            }
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
            RustReturnConversion::None,
            Vec::new(),
        );
    }
}

#[derive(Clone, Debug)]
struct ImportedFunctionDecl {
    decl: FnDecl,
    return_conversion: RustReturnConversion,
    arg_conversions: Vec<RustArgConversion>,
}

#[derive(Debug)]
struct ImportedTypeDecl {
    decl: TypeDecl,
    field_conversions: Vec<RustFieldConversion>,
}

impl ImportedTypeDecl {
    fn new(decl: TypeDecl) -> Self {
        Self {
            decl,
            field_conversions: Vec::new(),
        }
    }

    fn empty(name: &str) -> Self {
        Self::new(empty_type_decl(name))
    }
}

#[derive(Debug)]
struct LiftedStructMember {
    member: StructTypeMember,
    return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
struct LiftedReturn {
    ty: TypeElement,
    decl_modifier: Option<galvan_ast::DeclModifier>,
    return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
struct LiftedParam {
    param: Param,
    arg_conversion: RustArgConversion,
}

#[derive(Clone, Debug)]
struct LiftedType {
    ty: TypeElement,
    decl_modifier: Option<galvan_ast::DeclModifier>,
    arg_conversion: RustArgConversion,
}

impl LiftedType {
    fn new(ty: TypeElement) -> Self {
        Self {
            ty,
            decl_modifier: None,
            arg_conversion: RustArgConversion::None,
        }
    }

    fn with_modifier(ty: TypeElement, decl_modifier: galvan_ast::DeclModifier) -> Self {
        Self {
            ty,
            decl_modifier: Some(decl_modifier),
            arg_conversion: RustArgConversion::None,
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
        .filter(|id| index.get(*id).and_then(item_inner_constant).is_none())
        .collect()
}

fn impl_constant_ids(index: &serde_json::Map<String, Value>) -> HashSet<&str> {
    index
        .values()
        .filter_map(|item| item_inner(item, "impl"))
        .flat_map(|impl_item| item_ids(impl_item, "items"))
        .filter(|id| index.get(*id).and_then(item_inner_constant).is_some())
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
    let Some(receiver) = impl_inner.get("for").and_then(resolved_rust_type_path) else {
        return path;
    };
    if let Some(trait_path) = impl_inner
        .get("trait")
        .filter(|trait_| !trait_.is_null())
        .and_then(resolved_rust_type_path)
    {
        return format!("<::{receiver} as ::{trait_path}>::{name}").into();
    }
    if path.matches("::").count() > 2 {
        return path;
    };
    if receiver.contains("::") {
        format!("::{receiver}::{name}").into()
    } else {
        format!("::{crate_name}::{receiver}::{name}").into()
    }
}

fn impl_constant_rust_path(
    crate_name: &str,
    name: &str,
    item: &Value,
    impl_inner: &Value,
) -> Box<str> {
    impl_function_rust_path(crate_name, name, item, impl_inner)
}

fn resolved_rust_type_path(ty: &Value) -> Option<Box<str>> {
    let resolved = inner(ty, "resolved_path")?;
    let name = resolved.get("name").and_then(Value::as_str)?;
    let mut segments = resolved
        .get("path")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if segments.last().is_some_and(|segment| *segment == name) {
        segments.pop();
    }

    if segments.is_empty() {
        Some(name.into())
    } else {
        Some(format!("{}::{name}", segments.join("::")).into())
    }
}

fn item_inner_constant(item: &Value) -> Option<&Value> {
    item_inner(item, "constant").or_else(|| item_inner(item, "assoc_const"))
}

fn constant_inner(item: &Value) -> Option<&Value> {
    item_inner_constant(item)
}

fn constant_type(constant: &Value) -> Option<&Value> {
    constant.get("type").or_else(|| constant.get("ty"))
}

fn receiver_type_ident(ty: &TypeElement) -> Option<TypeIdent> {
    match ty {
        TypeElement::Plain(plain) => Some(plain.ident.clone()),
        TypeElement::Parametric(parametric) => Some(parametric.base_type.clone()),
        TypeElement::Generic(generic) => Some(TypeIdent::new(generic.ident.as_str())),
        _ => None,
    }
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
mod tests;
