use rustdoc_types::{
    Crate, Enum, FunctionPointer, FunctionSignature, Impl, Item, ItemEnum, Struct, StructKind,
    Type, TypeAlias, VariantKind,
};

use galvan_ast::{
    AliasTypeDecl, ClosureTypeItem, EnumTypeDecl, EnumTypeMember, EnumVariantField, FnSignature,
    Ident, Param, ParamList, Span, StructTypeDecl, StructTypeMember, TupleTypeDecl,
    TupleTypeMember, TypeDecl, TypeElement, TypeIdent, Visibility,
};

use crate::model::{
    RustArgConversion, RustEnumVariantArgConversion, RustEnumVariantConversion,
    RustFieldConversion, RustReturnConversion,
};

use super::lift_model::{
    ImportedFunctionDecl, ImportedTypeDecl, LiftedEnumMember, LiftedEnumVariantField, LiftedParam,
    LiftedReturn, LiftedStructMember, LiftedTupleMember, LiftedType,
};
use super::lift_type::{
    array_type, generic_type, member_arg_conversion, parametric_or_plain_type, plain_type,
    primitive_type, type_is_copy,
};
use super::lift_wrappers::known_lifted_resolved_type;
use super::rustdoc_json::{
    generic_type_params, is_public, resolved_type_args, struct_field_type,
    type_contains_unliftable_type, type_decl_contains_unliftable_type, type_is_owned,
};
use super::rustdoc_path::{resolved_path_is_unqualified_or_in_crates, resolved_type_name};
use super::RustInterop;

impl RustInterop {
    pub(super) fn type_decl_from_item(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        item: &Item,
    ) -> Option<ImportedTypeDecl> {
        if type_decl_contains_unliftable_type(krate, item) {
            return None;
        }

        match &item.inner {
            ItemEnum::Struct(struct_) => {
                self.struct_decl_from_json(krate, crate_name, name, struct_)
            }
            ItemEnum::Enum(enum_) => self.enum_decl_from_json(krate, crate_name, name, enum_),
            ItemEnum::TypeAlias(alias) => self
                .alias_decl_from_json(krate, crate_name, name, alias)
                .map(ImportedTypeDecl::new),
            // Unions carry no liftable shape.
            _ => None,
        }
    }

    fn alias_decl_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        alias: &TypeAlias,
    ) -> Option<TypeDecl> {
        Some(TypeDecl::Alias(AliasTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            generic_params: generic_type_params(&alias.generics),
            r#type: self.type_from_json(krate, crate_name, &alias.type_)?,
            span: Span::default(),
        }))
    }

    fn struct_decl_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        struct_: &Struct,
    ) -> Option<ImportedTypeDecl> {
        let generic_params = generic_type_params(&struct_.generics);
        if let StructKind::Tuple(field_ids) = &struct_.kind {
            if field_ids.iter().any(Option::is_none) {
                return None;
            }
            let mut lifted_members = Vec::new();
            for id in field_ids.iter().flatten() {
                let field = krate.index.get(id)?;
                if !is_public(field) {
                    return None;
                }
                lifted_members.push(self.tuple_member_from_json(krate, crate_name, field)?);
            }
            let constructor_arg_conversions = lifted_members
                .iter()
                .map(|member| member.arg_conversion)
                .collect::<Vec<_>>();
            let members = lifted_members
                .into_iter()
                .map(|member| member.member)
                .collect::<Vec<_>>();
            return Some(ImportedTypeDecl {
                decl: TypeDecl::Tuple(TupleTypeDecl {
                    visibility: Visibility::public(),
                    ident: TypeIdent::new(name),
                    generic_params,
                    members,
                    span: Span::default(),
                }),
                field_conversions: Vec::new(),
                constructor_arg_conversions,
                enum_variant_conversions: Vec::new(),
            });
        }

        let field_ids = match &struct_.kind {
            StructKind::Plain {
                fields,
                has_stripped_fields: false,
            } => fields.as_slice(),
            StructKind::Plain {
                has_stripped_fields: true,
                ..
            } => return None,
            _ => &[],
        };
        let mut lifted_members = Vec::new();
        for id in field_ids {
            let field = krate.index.get(id)?;
            if !is_public(field) {
                return None;
            }
            lifted_members.push(self.struct_member_from_json(krate, crate_name, field)?);
        }
        let mut members = Vec::new();
        let mut field_conversions = Vec::new();
        for member in lifted_members {
            if member.return_conversion != RustReturnConversion::None {
                field_conversions.push(RustFieldConversion {
                    field: member.member.ident.clone(),
                    arg_conversion: member.arg_conversion,
                    return_conversion: member.return_conversion,
                });
            }
            members.push(member.member);
        }

        if members.is_empty() && !matches!(struct_.kind, StructKind::Plain { .. }) {
            return None;
        }

        Some(ImportedTypeDecl {
            decl: TypeDecl::Struct(StructTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                generic_params,
                members,
                span: Span::default(),
            }),
            field_conversions,
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
        })
    }

    fn struct_member_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        field: &Item,
    ) -> Option<LiftedStructMember> {
        let name = field.name.as_deref()?;
        let field_type = struct_field_type(field)?;
        let lifted = self.lift_return_type_from_json(krate, crate_name, field_type)?;

        Some(LiftedStructMember {
            member: StructTypeMember {
                decl_modifier: lifted.decl_modifier,
                ident: Ident::new(name),
                r#type: lifted.ty,
                default_value: None,
                span: Span::default(),
            },
            arg_conversion: member_arg_conversion(lifted.return_conversion),
            return_conversion: lifted.return_conversion,
        })
    }

    fn tuple_member_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        field: &Item,
    ) -> Option<LiftedTupleMember> {
        let field_type = struct_field_type(field)?;
        let lifted = self.lift_return_type_from_json(krate, crate_name, field_type)?;
        Some(LiftedTupleMember {
            member: TupleTypeMember {
                r#type: lifted.ty,
                span: Span::default(),
            },
            arg_conversion: member_arg_conversion(lifted.return_conversion),
        })
    }

    fn enum_decl_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        enum_: &Enum,
    ) -> Option<ImportedTypeDecl> {
        if enum_.has_stripped_variants {
            return None;
        }
        let mut lifted_members = Vec::new();
        for id in &enum_.variants {
            let variant = krate.index.get(id)?;
            lifted_members.push(self.enum_member_from_json(krate, crate_name, variant)?);
        }
        let mut members = Vec::new();
        let mut enum_variant_conversions = Vec::new();
        for member in lifted_members {
            if member
                .arg_conversions
                .iter()
                .any(|arg| arg.arg_conversion != RustArgConversion::None)
            {
                enum_variant_conversions.push(RustEnumVariantConversion {
                    variant: member.member.ident.clone(),
                    args: member.arg_conversions,
                });
            }
            members.push(member.member);
        }

        Some(ImportedTypeDecl {
            decl: TypeDecl::Enum(EnumTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                generic_params: generic_type_params(&enum_.generics),
                common_fields: Vec::new(),
                members,
                span: Span::default(),
            }),
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions,
        })
    }

    fn enum_member_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        variant_item: &Item,
    ) -> Option<LiftedEnumMember> {
        let name = variant_item.name.as_deref()?;
        let ItemEnum::Variant(variant) = &variant_item.inner else {
            return None;
        };
        let lifted_fields =
            self.enum_variant_fields_from_kind(krate, crate_name, variant_item, &variant.kind)?;
        let mut fields = Vec::new();
        let mut arg_conversions = Vec::new();
        for field in lifted_fields {
            arg_conversions.push(RustEnumVariantArgConversion {
                field: field.field.name.clone(),
                arg_conversion: field.arg_conversion,
                return_conversion: field.return_conversion,
            });
            fields.push(field.field);
        }

        Some(LiftedEnumMember {
            member: EnumTypeMember {
                ident: TypeIdent::new(name),
                fields,
                span: Span::default(),
            },
            arg_conversions,
        })
    }

    fn enum_variant_fields_from_kind(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        variant: &Item,
        kind: &VariantKind,
    ) -> Option<Vec<LiftedEnumVariantField>> {
        match kind {
            VariantKind::Plain => Some(Vec::new()),
            VariantKind::Tuple(field_ids) => {
                if field_ids.iter().any(Option::is_none) {
                    return None;
                }
                let mut fields = Vec::new();
                for id in field_ids.iter().flatten() {
                    let field = krate.index.get(id)?;
                    if !enum_variant_field_is_public(variant, field) {
                        return None;
                    }
                    fields.push(self.enum_variant_field_from_json(krate, crate_name, None, field)?);
                }
                Some(fields)
            }
            VariantKind::Struct {
                fields: field_ids,
                has_stripped_fields: false,
            } => {
                let mut fields = Vec::new();
                for id in field_ids {
                    let field = krate.index.get(id)?;
                    if !enum_variant_field_is_public(variant, field) {
                        return None;
                    }
                    let name = field.name.as_deref().map(Ident::new);
                    fields.push(self.enum_variant_field_from_json(krate, crate_name, name, field)?);
                }
                Some(fields)
            }
            VariantKind::Struct {
                has_stripped_fields: true,
                ..
            } => None,
        }
    }

    fn enum_variant_field_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: Option<Ident>,
        field: &Item,
    ) -> Option<LiftedEnumVariantField> {
        let field_type = struct_field_type(field)?;
        let lifted = self.lift_return_type_from_json(krate, crate_name, field_type)?;
        Some(LiftedEnumVariantField {
            field: EnumVariantField {
                name,
                r#type: lifted.ty,
                span: Span::default(),
            },
            arg_conversion: member_arg_conversion(lifted.return_conversion),
            return_conversion: lifted.return_conversion,
        })
    }

    pub(super) fn function_decl(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        signature: &FunctionSignature,
    ) -> Option<ImportedFunctionDecl> {
        let mut lifted_params = Vec::new();
        for (param_name, param_type) in &signature.inputs {
            lifted_params
                .push(self.lift_param_from_json(krate, crate_name, param_name, param_type)?);
        }
        let params = lifted_params
            .iter()
            .map(|param| param.param.clone())
            .collect::<Vec<_>>();
        let (name, labels) = demangle_function_name(name, params.len())?;
        let label_start = params.len() - labels.len();
        let params = params
            .into_iter()
            .enumerate()
            .map(|(index, mut param)| {
                if index >= label_start {
                    param.short_name = Some(Ident::new(labels[index - label_start]));
                }
                param
            })
            .collect();
        let arg_conversions = lifted_params
            .iter()
            .map(|param| param.arg_conversion)
            .collect::<Vec<_>>();

        let (return_type, return_conversion) = if let Some(output) = &signature.output {
            let lifted = self.lift_return_type_from_json(krate, crate_name, output)?;
            (lifted.ty, lifted.return_conversion)
        } else {
            (TypeElement::void(), RustReturnConversion::None)
        };

        let decl = FnSignature {
            visibility: Visibility::public(),
            // TODO(async): lift `header.is_async` from rustdoc so awaited Rust
            // futures can be typechecked once async is supported.
            is_async: false,
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

        Some(ImportedFunctionDecl {
            decl,
            return_conversion,
            arg_conversions,
        })
    }

    #[cfg(test)]
    pub(super) fn param_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        ty: &Type,
    ) -> Option<Param> {
        self.lift_param_from_json(krate, crate_name, name, ty)
            .map(|param| param.param)
    }

    fn lift_param_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        ty: &Type,
    ) -> Option<LiftedParam> {
        let lifted = if param_type_requires_wrapper_conversion(krate, ty) {
            self.lift_param_wrapper_type_from_json(krate, crate_name, ty)?
        } else {
            self.lift_type_from_json(krate, crate_name, ty)?
        };
        let decl_modifier = lifted.decl_modifier.or_else(|| {
            if type_is_owned(ty) && !type_is_copy(&lifted.ty) {
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
        krate: &Crate,
        crate_name: &str,
        ty: &Type,
    ) -> Option<LiftedType> {
        let Type::ResolvedPath(resolved) = ty else {
            return None;
        };
        let name = resolved_type_name(resolved)?;
        if !resolved_path_is_unqualified_or_in_crates(krate, resolved, &["std", "core", "alloc"]) {
            return None;
        }
        let conversion = match name.as_ref() {
            "Box" => RustArgConversion::BoxNew,
            "Rc" => RustArgConversion::RcNew,
            _ => return None,
        };
        let arg = resolved_type_args(resolved).into_iter().next()?;
        let mut lifted = self.lift_type_from_json(krate, crate_name, arg)?;
        lifted.arg_conversion = conversion;
        Some(lifted)
    }

    fn lift_return_type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        ty: &Type,
    ) -> Option<LiftedReturn> {
        if let Type::ResolvedPath(resolved) = ty {
            let name = resolved_type_name(resolved)?;
            let standard_wrapper = resolved_path_is_unqualified_or_in_crates(
                krate,
                resolved,
                &["std", "core", "alloc"],
            );
            let return_conversion = match name.as_ref() {
                "Box" if standard_wrapper => RustReturnConversion::BoxDeref,
                _ => RustReturnConversion::None,
            };
            if return_conversion != RustReturnConversion::None {
                let arg = resolved_type_args(resolved).into_iter().next()?;
                let lifted = self.lift_type_from_json(krate, crate_name, arg)?;
                return Some(LiftedReturn {
                    ty: lifted.ty,
                    decl_modifier: lifted.decl_modifier,
                    return_conversion,
                });
            }
        }

        self.lift_type_from_json(krate, crate_name, ty)
            .map(|lifted| LiftedReturn {
                ty: lifted.ty,
                decl_modifier: lifted.decl_modifier,
                return_conversion: RustReturnConversion::None,
            })
    }

    pub(super) fn impl_function_decl(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        signature: &FunctionSignature,
        impl_: &Impl,
    ) -> Option<ImportedFunctionDecl> {
        let mut imported = self.function_decl(krate, crate_name, name, signature)?;
        if let Some(receiver_ty) = self.type_from_json(krate, crate_name, &impl_.for_) {
            substitute_self_in_function_decl(&mut imported.decl, &receiver_ty);
        }

        Some(imported)
    }

    pub(super) fn trait_function_decl(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        name: &str,
        signature: &FunctionSignature,
        receiver: &TypeIdent,
    ) -> Option<ImportedFunctionDecl> {
        let mut imported = self.function_decl(krate, crate_name, name, signature)?;
        substitute_self_in_function_decl(&mut imported.decl, &plain_type(receiver.clone()));

        Some(imported)
    }

    pub(super) fn type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        ty: &Type,
    ) -> Option<TypeElement> {
        self.lift_type_from_json(krate, crate_name, ty)
            .map(|lifted| lifted.ty)
    }

    pub(super) fn lift_type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        ty: &Type,
    ) -> Option<LiftedType> {
        if let Type::BorrowedRef {
            is_mutable,
            type_: borrowed,
            ..
        } = ty
        {
            if let Type::Array { type_: element, .. } = borrowed.as_ref() {
                let mut lifted = self
                    .lift_type_from_json(krate, crate_name, element)
                    .map(array_type)?;
                if *is_mutable {
                    lifted.decl_modifier = Some(galvan_ast::DeclModifier::Mut);
                    lifted.arg_conversion = RustArgConversion::FixedArrayMutBorrow;
                } else {
                    lifted.arg_conversion = RustArgConversion::FixedArrayBorrow;
                }
                return Some(lifted);
            }
        }

        if type_contains_unliftable_type(krate, ty) {
            return None;
        }
        match ty {
            Type::Primitive(primitive) => Some(LiftedType::new(primitive_type(primitive))),
            Type::Generic(generic) => Some(LiftedType::new(generic_type(generic))),
            Type::BorrowedRef {
                is_mutable, type_, ..
            } => {
                let mut lifted = match type_.as_ref() {
                    Type::Slice(element) => self
                        .lift_type_from_json(krate, crate_name, element)
                        .map(array_type)?,
                    _ => self.lift_type_from_json(krate, crate_name, type_)?,
                };
                if *is_mutable {
                    lifted.decl_modifier = Some(galvan_ast::DeclModifier::Mut);
                } else {
                    lifted.arg_conversion = RustArgConversion::SharedBorrow;
                }
                Some(lifted)
            }
            Type::FunctionPointer(function) => self
                .function_pointer_type_from_json(krate, crate_name, function)
                .map(LiftedType::new),
            Type::ResolvedPath(resolved) => {
                let name = resolved_type_name(resolved)?;
                let standard_wrapper = resolved_path_is_unqualified_or_in_crates(
                    krate,
                    resolved,
                    &["std", "core", "alloc"],
                );
                if name.as_ref() == "Arc" && standard_wrapper {
                    return self.lift_arc_type_from_json(krate, crate_name, resolved);
                }

                let args = self.lift_resolved_type_args_from_json(krate, crate_name, resolved)?;

                if let Some(lifted) =
                    self.lift_known_resolved_type(krate, name.as_ref(), resolved, args.as_slice())
                {
                    return Some(lifted);
                }
                if known_lifted_resolved_type(krate, name.as_ref(), resolved) {
                    return None;
                }

                // A reference to a local type alias whose target is a known galvan
                // wrapper (e.g. `serde_json::Result<T> = Result<T, Error>`) is
                // inlined: aliases are transparent, and downstream fallible /
                // optional / collection handling needs the wrapper form. rustdoc
                // may or may not expand such aliases in signatures depending on the
                // toolchain, so we normalize here. Nominal aliases keep their name.
                if let Some(lifted) = self.lift_wrapper_alias(krate, crate_name, resolved, &args) {
                    return Some(lifted);
                }

                self.push_resolved_type(krate, crate_name, name.as_ref(), resolved);
                Some(LiftedType::new(parametric_or_plain_type(
                    name.as_ref(),
                    args,
                )))
            }
            Type::Tuple(elements) => Some(LiftedType::new(TypeElement::Tuple(Box::new(
                galvan_ast::TupleTypeItem {
                    elements: self.lift_tuple_elements_from_json(krate, crate_name, elements)?,
                    span: Span::default(),
                },
            )))),
            // RawPointer/QualifiedPath/DynTrait/ImplTrait/Infer/Pat are rejected by the
            // unliftable gate above; nothing liftable remains.
            _ => None,
        }
    }

    fn function_pointer_type_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        function: &FunctionPointer,
    ) -> Option<TypeElement> {
        let mut parameters = Vec::new();
        for (_, input) in &function.sig.inputs {
            parameters.push(self.type_from_json(krate, crate_name, input)?);
        }
        let return_ty = if let Some(output) = &function.sig.output {
            self.type_from_json(krate, crate_name, output)?
        } else {
            TypeElement::void()
        };

        Some(TypeElement::Closure(Box::new(ClosureTypeItem {
            parameters,
            return_ty,
            span: Span::default(),
        })))
    }

    fn lift_resolved_type_args_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        resolved: &rustdoc_types::Path,
    ) -> Option<Vec<LiftedType>> {
        resolved_type_args(resolved)
            .into_iter()
            .map(|arg| self.lift_type_from_json(krate, crate_name, arg))
            .collect()
    }

    /// If `resolved` refers to a local type alias whose target lifts to a known
    /// galvan wrapper, inline that wrapper with the alias's type parameters
    /// substituted by the reference's arguments. Returns `None` for non-aliases
    /// and for aliases whose target is nominal (those keep their own name).
    fn lift_wrapper_alias(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        resolved: &rustdoc_types::Path,
        args: &[LiftedType],
    ) -> Option<LiftedType> {
        let ItemEnum::TypeAlias(alias) = &krate.index.get(&resolved.id)?.inner else {
            return None;
        };
        let mut lifted = self.lift_type_from_json(krate, crate_name, &alias.type_)?;
        if !type_element_is_wrapper(&lifted.ty) {
            return None;
        }

        let substitutions: std::collections::HashMap<String, TypeElement> =
            generic_type_params(&alias.generics)
                .iter()
                .zip(args)
                .map(|(param, arg)| (param.as_str().to_string(), arg.ty.clone()))
                .collect();
        lifted.ty.substitute_generics(&substitutions);
        Some(lifted)
    }

    fn lift_tuple_elements_from_json(
        &mut self,
        krate: &Crate,
        crate_name: &str,
        tuple: &[Type],
    ) -> Option<Vec<TypeElement>> {
        tuple
            .iter()
            .map(|ty| self.type_from_json(krate, crate_name, ty))
            .collect()
    }
}

fn demangle_function_name(name: &str, param_count: usize) -> Option<(&str, Vec<&str>)> {
    let mut segments = name.split("__");
    let name = segments.next()?;
    let labels = segments.collect::<Vec<_>>();
    if name.is_empty() || labels.len() > param_count || labels.iter().any(|label| label.is_empty())
    {
        return None;
    }
    Some((name, labels))
}

/// Whether a lifted type is one of galvan's built-in wrapper forms (as opposed
/// to a nominal type), i.e. one worth inlining a transparent alias down to.
fn type_element_is_wrapper(ty: &TypeElement) -> bool {
    matches!(
        ty,
        TypeElement::Result(_)
            | TypeElement::Optional(_)
            | TypeElement::Array(_)
            | TypeElement::Set(_)
            | TypeElement::Dictionary(_)
            | TypeElement::OrderedDictionary(_)
    )
}

fn param_type_requires_wrapper_conversion(krate: &Crate, ty: &Type) -> bool {
    let Type::ResolvedPath(resolved) = ty else {
        return false;
    };
    let Some(name) = resolved_type_name(resolved) else {
        return false;
    };
    matches!(name.as_ref(), "Box" | "Rc")
        && resolved_path_is_unqualified_or_in_crates(krate, resolved, &["std", "core", "alloc"])
}

fn enum_variant_field_is_public(variant: &Item, field: &Item) -> bool {
    // Real rustdoc records public enum variants and their fields as Default.
    // A Public variant with a Default field is only used by typed edge tests.
    is_public(field)
        || matches!(
            (&variant.visibility, &field.visibility),
            (
                rustdoc_types::Visibility::Default,
                rustdoc_types::Visibility::Default
            )
        )
}

fn substitute_self_in_function_decl(decl: &mut galvan_ast::FnDecl, receiver_ty: &TypeElement) {
    let substitutions =
        std::collections::HashMap::from([("Self".to_string(), receiver_ty.clone())]);
    for param in &mut decl.signature.parameters.params {
        param.param_type.substitute_generics(&substitutions);
    }
    decl.signature
        .return_type
        .substitute_generics(&substitutions);
}
