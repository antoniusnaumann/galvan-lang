use serde_json::Value;

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
    array_type, function_pointer_input_type, generic_type, member_arg_conversion, never_type,
    parametric_or_plain_type, primitive_type, resolved_path_is_unqualified_or_in_crates,
    type_is_copy,
};
use super::lift_wrappers::known_lifted_resolved_type;
use super::rustdoc_json::{
    borrowed_ref_is_mutable, inner, inner_string, is_public, item_ids, item_inner,
    resolved_type_args, resolved_type_args_strict, type_alias_type, type_contains_unliftable_type,
    type_decl_contains_unliftable_type, type_generic_params, type_inner_generic_params,
    type_is_owned,
};
use super::rustdoc_path::resolved_type_name;
use super::RustInterop;

impl RustInterop {
    pub(super) fn type_decl_from_item(
        &mut self,
        crate_name: &str,
        name: &str,
        item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<ImportedTypeDecl> {
        if type_decl_contains_unliftable_type(item, index) {
            return None;
        }

        let inner = item.get("inner")?;
        if let Some(struct_item) = inner.get("struct") {
            return self.struct_decl_from_json(crate_name, name, struct_item, index);
        }
        if let Some(enum_item) = inner.get("enum") {
            return self.enum_decl_from_json(crate_name, name, enum_item, index);
        }
        if let Some(alias_item) = inner.get("type_alias") {
            return self
                .alias_decl_from_json(crate_name, name, alias_item, type_generic_params(item))
                .map(ImportedTypeDecl::new);
        }

        None
    }

    fn alias_decl_from_json(
        &mut self,
        crate_name: &str,
        name: &str,
        alias_item: &Value,
        generic_params: Vec<Ident>,
    ) -> Option<TypeDecl> {
        Some(TypeDecl::Alias(AliasTypeDecl {
            visibility: Visibility::public(),
            ident: TypeIdent::new(name),
            generic_params,
            r#type: self.type_from_json(crate_name, type_alias_type(alias_item)?)?,
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
            let mut lifted_members = Vec::new();
            for id in field_ids {
                let field = index.get(id)?;
                if !is_public(field) {
                    return None;
                }
                lifted_members.push(self.tuple_member_from_json(crate_name, field)?);
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
                    generic_params: type_inner_generic_params(struct_item),
                    members,
                    span: Span::default(),
                }),
                field_conversions: Vec::new(),
                constructor_arg_conversions,
                enum_variant_conversions: Vec::new(),
            });
        }

        let mut lifted_members = Vec::new();
        for id in field_ids {
            let field = index.get(id)?;
            if !is_public(field) {
                return None;
            }
            lifted_members.push(self.struct_member_from_json(crate_name, field)?);
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

        if members.is_empty() && kind != Some("plain") {
            return None;
        }

        Some(ImportedTypeDecl {
            decl: TypeDecl::Struct(StructTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                generic_params: type_inner_generic_params(struct_item),
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
            arg_conversion: member_arg_conversion(lifted.return_conversion),
            return_conversion: lifted.return_conversion,
        })
    }

    fn tuple_member_from_json(
        &mut self,
        crate_name: &str,
        field: &Value,
    ) -> Option<LiftedTupleMember> {
        let field_type = item_inner(field, "struct_field")?;
        let lifted = self.lift_return_type_from_json(crate_name, field_type)?;
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
        crate_name: &str,
        name: &str,
        enum_item: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<ImportedTypeDecl> {
        let mut lifted_members = Vec::new();
        for id in item_ids(enum_item, "variants") {
            let variant = index.get(id)?;
            lifted_members.push(self.enum_member_from_json(crate_name, variant, index)?);
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
                generic_params: type_inner_generic_params(enum_item),
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
        crate_name: &str,
        variant: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<LiftedEnumMember> {
        let name = variant.get("name").and_then(Value::as_str)?;
        let variant = item_inner(variant, "variant")?;
        let lifted_fields = match variant.get("kind") {
            Some(Value::String(kind)) if kind == "plain" => Vec::new(),
            Some(kind) => self.enum_variant_fields_from_kind(crate_name, kind, index)?,
            None => Vec::new(),
        };
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
        crate_name: &str,
        kind: &Value,
        index: &serde_json::Map<String, Value>,
    ) -> Option<Vec<LiftedEnumVariantField>> {
        if let Some(tuple) = inner(kind, "tuple") {
            let mut fields = Vec::new();
            for id in item_ids(tuple, "fields") {
                let field = index.get(id)?;
                if !is_public(field) {
                    return None;
                }
                fields.push(self.enum_variant_field_from_json(crate_name, None, field)?);
            }
            return Some(fields);
        }

        if let Some(struct_variant) = inner(kind, "struct") {
            let mut fields = Vec::new();
            for id in item_ids(struct_variant, "fields") {
                let field = index.get(id)?;
                if !is_public(field) {
                    return None;
                }
                let name = field.get("name").and_then(Value::as_str).map(Ident::new);
                fields.push(self.enum_variant_field_from_json(crate_name, name, field)?);
            }
            return Some(fields);
        }

        Some(Vec::new())
    }

    fn enum_variant_field_from_json(
        &mut self,
        crate_name: &str,
        name: Option<Ident>,
        field: &Value,
    ) -> Option<LiftedEnumVariantField> {
        let field_type = item_inner(field, "struct_field")?;
        let lifted = self.lift_return_type_from_json(crate_name, field_type)?;
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
        crate_name: &str,
        name: &str,
        signature: &Value,
    ) -> Option<ImportedFunctionDecl> {
        let mut lifted_params = Vec::new();
        for param in signature
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lifted_params.push(self.lift_param_from_json(crate_name, param)?);
        }
        let params = lifted_params
            .iter()
            .map(|param| param.param.clone())
            .collect::<Vec<_>>();
        let arg_conversions = lifted_params
            .iter()
            .map(|param| param.arg_conversion)
            .collect::<Vec<_>>();

        let (return_type, return_conversion) =
            if let Some(output) = signature.get("output").filter(|output| !output.is_null()) {
                let lifted = self.lift_return_type_from_json(crate_name, output)?;
                (lifted.ty, lifted.return_conversion)
            } else {
                (TypeElement::void(), RustReturnConversion::None)
            };

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

        Some(ImportedFunctionDecl {
            decl,
            return_conversion,
            arg_conversions,
        })
    }

    #[cfg(test)]
    pub(super) fn param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<Param> {
        self.lift_param_from_json(crate_name, param)
            .map(|param| param.param)
    }

    fn lift_param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<LiftedParam> {
        let pair = param.as_array()?;
        let name = pair.first().and_then(Value::as_str).unwrap_or("_");
        let ty = pair.get(1)?;
        let lifted = if param_type_requires_wrapper_conversion(ty) {
            self.lift_param_wrapper_type_from_json(crate_name, ty)?
        } else {
            self.lift_type_from_json(crate_name, ty)?
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
        crate_name: &str,
        ty: &Value,
    ) -> Option<LiftedType> {
        let resolved = inner(ty, "resolved_path")?;
        let name = resolved_type_name(resolved)?;
        if !resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]) {
            return None;
        }
        let conversion = match name.as_ref() {
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
            let name = resolved_type_name(resolved)?;
            let standard_wrapper =
                resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]);
            let return_conversion = match name.as_ref() {
                "Box" if standard_wrapper => RustReturnConversion::BoxDeref,
                "Rc" if standard_wrapper => RustReturnConversion::RcCloneDeref,
                _ => RustReturnConversion::None,
            };
            if return_conversion != RustReturnConversion::None {
                let arg = resolved_type_args(resolved).into_iter().next()?;
                let lifted = self.lift_type_from_json(crate_name, arg)?;
                return Some(LiftedReturn {
                    ty: lifted.ty,
                    decl_modifier: lifted.decl_modifier,
                    return_conversion,
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

    pub(super) fn impl_function_decl(
        &mut self,
        crate_name: &str,
        name: &str,
        signature: &Value,
        impl_inner: &Value,
    ) -> Option<ImportedFunctionDecl> {
        let mut imported = self.function_decl(crate_name, name, signature)?;
        let Some(first_param) = imported.decl.signature.parameters.params.first_mut() else {
            return Some(imported);
        };
        if !first_param.identifier.is_self() {
            return Some(imported);
        }

        if let Some(receiver_ty) = impl_inner
            .get("for")
            .and_then(|ty| self.type_from_json(crate_name, ty))
        {
            first_param.param_type = receiver_ty;
        }

        Some(imported)
    }

    pub(super) fn type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<TypeElement> {
        self.lift_type_from_json(crate_name, ty)
            .map(|lifted| lifted.ty)
    }

    pub(super) fn lift_type_from_json(
        &mut self,
        crate_name: &str,
        ty: &Value,
    ) -> Option<LiftedType> {
        if type_contains_unliftable_type(ty) {
            return None;
        }
        if inner(ty, "never").is_some() {
            return Some(LiftedType::new(never_type()));
        }
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
        if let Some(slice) = inner(ty, "slice") {
            return self.lift_type_from_json(crate_name, slice).map(array_type);
        }
        if let Some(array) = inner(ty, "array") {
            let element = array.get("type").or_else(|| array.get("element"))?;
            return self
                .lift_type_from_json(crate_name, element)
                .map(array_type);
        }
        if let Some(function) = inner(ty, "function_pointer").or_else(|| inner(ty, "bare_function"))
        {
            return self
                .function_pointer_type_from_json(crate_name, function)
                .map(LiftedType::new);
        }
        if let Some(resolved) = inner(ty, "resolved_path") {
            let name = resolved_type_name(resolved)?;
            let standard_wrapper =
                resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"]);
            if name.as_ref() == "Arc" && standard_wrapper {
                return self.lift_arc_type_from_json(crate_name, resolved);
            }
            if matches!(name.as_ref(), "Mutex" | "RwLock") && standard_wrapper {
                return self.lift_lock_type_from_json(crate_name, resolved);
            }

            let args = self.lift_resolved_type_args_from_json(crate_name, resolved)?;

            if let Some(lifted) =
                self.lift_known_resolved_type(name.as_ref(), resolved, args.as_slice())
            {
                return Some(lifted);
            }
            if known_lifted_resolved_type(name.as_ref(), resolved) {
                return None;
            }

            self.push_resolved_type(crate_name, name.as_ref(), resolved);
            return Some(LiftedType::new(parametric_or_plain_type(
                name.as_ref(),
                args,
            )));
        }
        if let Some(tuple) = inner(ty, "tuple").and_then(Value::as_array) {
            return Some(LiftedType::new(TypeElement::Tuple(Box::new(
                galvan_ast::TupleTypeItem {
                    elements: self.lift_tuple_elements_from_json(crate_name, tuple)?,
                    span: Span::default(),
                },
            ))));
        }

        None
    }

    fn function_pointer_type_from_json(
        &mut self,
        crate_name: &str,
        function: &Value,
    ) -> Option<TypeElement> {
        let signature = function.get("sig").unwrap_or(function);
        let mut parameters = Vec::new();
        for input in signature
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(function_pointer_input_type)
        {
            parameters.push(self.type_from_json(crate_name, input)?);
        }
        let return_ty =
            if let Some(output) = signature.get("output").filter(|output| !output.is_null()) {
                self.type_from_json(crate_name, output)?
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
        crate_name: &str,
        resolved: &Value,
    ) -> Option<Vec<LiftedType>> {
        resolved_type_args_strict(resolved)?
            .into_iter()
            .map(|arg| self.lift_type_from_json(crate_name, arg))
            .collect()
    }

    fn lift_tuple_elements_from_json(
        &mut self,
        crate_name: &str,
        tuple: &[Value],
    ) -> Option<Vec<TypeElement>> {
        tuple
            .iter()
            .map(|ty| self.type_from_json(crate_name, ty))
            .collect()
    }
}

fn param_type_requires_wrapper_conversion(ty: &Value) -> bool {
    let Some(resolved) = inner(ty, "resolved_path") else {
        return false;
    };
    let Some(name) = resolved_type_name(resolved) else {
        return false;
    };
    matches!(name.as_ref(), "Box" | "Rc")
        && resolved_path_is_unqualified_or_in_crates(resolved, &["std", "core", "alloc"])
}
