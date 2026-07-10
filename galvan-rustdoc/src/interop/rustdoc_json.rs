use std::collections::HashSet;

use rustdoc_types::{
    Abi, Crate, Function, FunctionHeader, FunctionSignature, GenericArg, GenericArgs,
    GenericParamDefKind, Generics, Id, Item, ItemEnum, Path, StructKind, Type, VariantKind,
    Visibility,
};

use galvan_ast::{Ident, TypeElement, TypeIdent};

use super::rustdoc_path::{resolved_path_is_unqualified_or_in_crates, resolved_type_name};

pub(super) fn is_public(item: &Item) -> bool {
    matches!(item.visibility, Visibility::Public)
}

pub(super) fn public_type_name(item: &Item) -> Option<&str> {
    matches!(
        item.inner,
        ItemEnum::Struct(_)
            | ItemEnum::Enum(_)
            | ItemEnum::Union(_)
            | ItemEnum::TypeAlias(_)
            | ItemEnum::Trait(_)
    )
    .then(|| item.name.as_deref())
    .flatten()
}

pub(super) fn type_generic_params(item: &Item) -> Vec<Ident> {
    item_generics(item)
        .map(generic_type_params)
        .unwrap_or_default()
}

fn item_generics(item: &Item) -> Option<&Generics> {
    match &item.inner {
        ItemEnum::Struct(struct_) => Some(&struct_.generics),
        ItemEnum::Enum(enum_) => Some(&enum_.generics),
        ItemEnum::Union(union_) => Some(&union_.generics),
        ItemEnum::TypeAlias(alias) => Some(&alias.generics),
        ItemEnum::Trait(trait_) => Some(&trait_.generics),
        _ => None,
    }
}

pub(super) fn generic_type_params(generics: &Generics) -> Vec<Ident> {
    generics
        .params
        .iter()
        .filter(|param| matches!(param.kind, GenericParamDefKind::Type { .. }))
        .map(|param| Ident::new(&param.name))
        .collect()
}

pub(super) fn function_is_unsafe(function: &Function) -> bool {
    function.header.is_unsafe
}

pub(super) fn signature_contains_unliftable_type(
    krate: &Crate,
    signature: &FunctionSignature,
) -> bool {
    signature
        .inputs
        .iter()
        .any(|(_, ty)| type_contains_unliftable_type(krate, ty))
        || signature
            .output
            .as_ref()
            .is_some_and(|ty| type_contains_unliftable_type(krate, ty))
}

pub(super) fn type_decl_contains_unliftable_type(krate: &Crate, item: &Item) -> bool {
    match &item.inner {
        ItemEnum::TypeAlias(alias) => type_contains_unliftable_type(krate, &alias.type_),
        ItemEnum::Struct(struct_) => struct_field_ids(&struct_.kind)
            .into_iter()
            .filter_map(|id| krate.index.get(&id))
            .filter_map(struct_field_type)
            .any(|ty| type_contains_unliftable_type(krate, ty)),
        ItemEnum::Enum(enum_) => enum_
            .variants
            .iter()
            .filter_map(|id| krate.index.get(id))
            .any(|variant| variant_contains_unliftable_type(krate, variant)),
        _ => false,
    }
}

pub(super) fn type_contains_unliftable_type(krate: &Crate, ty: &Type) -> bool {
    type_contains_unliftable_type_inner(krate, ty, false)
}

fn type_contains_unliftable_type_inner(
    krate: &Crate,
    ty: &Type,
    allow_standard_lock: bool,
) -> bool {
    match ty {
        Type::RawPointer { .. }
        | Type::QualifiedPath { .. }
        | Type::DynTrait(_)
        | Type::ImplTrait(_) => true,
        Type::BorrowedRef { type_, .. } => type_contains_unliftable_type_inner(krate, type_, false),
        Type::Slice(element) => type_contains_unliftable_type_inner(krate, element, false),
        Type::Array { type_, .. } => type_contains_unliftable_type_inner(krate, type_, false),
        Type::FunctionPointer(function) => {
            function_header_is_unliftable(&function.header)
                || signature_contains_unliftable_type(krate, &function.sig)
        }
        Type::ResolvedPath(path) => {
            if resolved_type_is_standard_mutex(krate, path) && !allow_standard_lock {
                return true;
            }
            let allow_nested_standard_lock = resolved_type_is_standard_arc(krate, path);
            resolved_type_args(path).into_iter().any(|ty| {
                type_contains_unliftable_type_inner(krate, ty, allow_nested_standard_lock)
            })
        }
        Type::Tuple(types) => types
            .iter()
            .any(|ty| type_contains_unliftable_type_inner(krate, ty, false)),
        Type::Primitive(_) | Type::Generic(_) | Type::Infer | Type::Pat { .. } => false,
    }
}

fn resolved_type_is_standard_arc(krate: &Crate, path: &Path) -> bool {
    resolved_type_has_standard_name(krate, path, "Arc")
}

fn resolved_type_is_standard_mutex(krate: &Crate, path: &Path) -> bool {
    resolved_type_has_standard_name(krate, path, "Mutex")
}

fn resolved_type_has_standard_name(krate: &Crate, path: &Path, expected_name: &str) -> bool {
    resolved_type_name(path).is_some_and(|name| name.as_ref() == expected_name)
        && resolved_path_is_unqualified_or_in_crates(krate, path, &["std", "core", "alloc"])
}

fn function_header_is_unliftable(header: &FunctionHeader) -> bool {
    header.is_unsafe || !matches!(header.abi, Abi::Rust)
}

fn struct_field_ids(kind: &StructKind) -> Vec<Id> {
    match kind {
        StructKind::Unit => Vec::new(),
        StructKind::Tuple(fields) => fields.iter().flatten().copied().collect(),
        StructKind::Plain { fields, .. } => fields.clone(),
    }
}

fn variant_field_ids(kind: &VariantKind) -> Vec<Id> {
    match kind {
        VariantKind::Plain => Vec::new(),
        VariantKind::Tuple(fields) => fields.iter().flatten().copied().collect(),
        VariantKind::Struct { fields, .. } => fields.clone(),
    }
}

pub(super) fn struct_field_type(item: &Item) -> Option<&Type> {
    match &item.inner {
        ItemEnum::StructField(ty) => Some(ty),
        _ => None,
    }
}

fn variant_contains_unliftable_type(krate: &Crate, variant: &Item) -> bool {
    let ItemEnum::Variant(variant) = &variant.inner else {
        return false;
    };
    variant_field_ids(&variant.kind)
        .into_iter()
        .filter_map(|id| krate.index.get(&id))
        .filter_map(struct_field_type)
        .any(|ty| type_contains_unliftable_type(krate, ty))
}

/// IDs of the associated functions (not constants) that live inside impl blocks.
pub(super) fn impl_function_ids(krate: &Crate) -> HashSet<Id> {
    associated_ids(krate, impl_items, false)
}

/// IDs of the associated constants that live inside impl blocks.
pub(super) fn impl_constant_ids(krate: &Crate) -> HashSet<Id> {
    associated_ids(krate, impl_items, true)
}

/// IDs of the associated functions (not constants) declared inside traits.
pub(super) fn trait_function_ids(krate: &Crate) -> HashSet<Id> {
    associated_ids(krate, trait_items, false)
}

/// IDs of the associated constants declared inside traits.
pub(super) fn trait_constant_ids(krate: &Crate) -> HashSet<Id> {
    associated_ids(krate, trait_items, true)
}

fn associated_ids<'a, I>(
    krate: &'a Crate,
    collect_items: impl Fn(&'a Item) -> Option<I>,
    constants: bool,
) -> HashSet<Id>
where
    I: IntoIterator<Item = &'a Id>,
{
    krate
        .index
        .values()
        .filter_map(collect_items)
        .flatten()
        .copied()
        .filter(|id| krate.index.get(id).is_some_and(is_constant) == constants)
        .collect()
}

fn impl_items(item: &Item) -> Option<&Vec<Id>> {
    match &item.inner {
        ItemEnum::Impl(impl_) => Some(&impl_.items),
        _ => None,
    }
}

fn trait_items(item: &Item) -> Option<&Vec<Id>> {
    match &item.inner {
        ItemEnum::Trait(trait_) => Some(&trait_.items),
        _ => None,
    }
}

fn is_constant(item: &Item) -> bool {
    matches!(
        item.inner,
        ItemEnum::Constant { .. } | ItemEnum::AssocConst { .. }
    )
}

/// The declared type of a constant or associated constant item.
pub(super) fn constant_type(item: &Item) -> Option<&Type> {
    match &item.inner {
        ItemEnum::Constant { type_, .. } | ItemEnum::AssocConst { type_, .. } => Some(type_),
        _ => None,
    }
}

pub(super) fn receiver_type_ident(ty: &TypeElement) -> Option<TypeIdent> {
    match ty {
        TypeElement::Plain(plain) => Some(plain.ident.clone()),
        TypeElement::Parametric(parametric) => Some(parametric.base_type.clone()),
        TypeElement::Generic(generic) => Some(TypeIdent::new(generic.ident.as_str())),
        _ => None,
    }
}

pub(super) fn return_is_borrowed(signature: &FunctionSignature) -> bool {
    matches!(signature.output, Some(Type::BorrowedRef { .. }))
}

pub(super) fn type_is_owned(ty: &Type) -> bool {
    !matches!(ty, Type::BorrowedRef { .. })
}

pub(super) fn resolved_type_args(path: &Path) -> Vec<&Type> {
    let Some(args) = &path.args else {
        return Vec::new();
    };
    match args.as_ref() {
        GenericArgs::AngleBracketed { args, .. } => args
            .iter()
            .filter_map(|arg| match arg {
                GenericArg::Type(ty) => Some(ty),
                // Lifetime/Const/Infer args carry no liftable type: skip them.
                _ => None,
            })
            .collect(),
        GenericArgs::Parenthesized { .. } | GenericArgs::ReturnTypeNotation => Vec::new(),
    }
}

pub(super) fn resolved_type_generic_params(path: &Path) -> Vec<Ident> {
    let mut params = Vec::new();
    for (index, arg) in resolved_type_args(path).into_iter().enumerate() {
        let candidate = match arg {
            Type::Generic(name) => name.clone(),
            _ => synthetic_generic_name(index),
        };
        let name = if generic_param_exists(&params, &candidate) {
            unique_synthetic_generic_name(index, &params)
        } else {
            candidate
        };
        params.push(Ident::new(name));
    }
    params
}

fn unique_synthetic_generic_name(index: usize, existing: &[Ident]) -> String {
    let mut offset = index;
    loop {
        let candidate = synthetic_generic_name(offset);
        if !generic_param_exists(existing, &candidate) {
            return candidate;
        }
        offset += 1;
    }
}

fn generic_param_exists(params: &[Ident], name: &str) -> bool {
    params.iter().any(|param| param.as_str() == name)
}

fn synthetic_generic_name(index: usize) -> String {
    match index {
        0 => "T".to_string(),
        1 => "U".to_string(),
        2 => "V".to_string(),
        3 => "W".to_string(),
        4 => "X".to_string(),
        5 => "Y".to_string(),
        6 => "Z".to_string(),
        _ => format!("T{}", index - 7),
    }
}
