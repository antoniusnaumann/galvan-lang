use serde_json::Value;

use galvan_ast::{
    AliasTypeDecl, ArrayTypeItem, BasicTypeItem, DictionaryTypeItem, EmptyTypeDecl, EnumTypeDecl,
    EnumTypeMember, EnumVariantField, FnDecl, FnSignature, Ident, OptionalTypeItem,
    OrderedDictionaryTypeItem, Param, ParamList, ParametricTypeItem, ResultTypeItem, SetTypeItem,
    Span, StructTypeDecl, StructTypeMember, TupleTypeDecl, TupleTypeMember, TypeDecl, TypeElement,
    TypeIdent, Visibility,
};

use crate::model::{
    RustArgConversion, RustEnumVariantArgConversion, RustEnumVariantConversion,
    RustFieldConversion, RustReturnConversion,
};

use super::rustdoc_json::{
    borrowed_ref_is_mutable, inner, inner_string, is_public, item_ids, item_inner,
    resolved_type_args, type_is_owned,
};
use super::RustInterop;

impl RustInterop {
    pub(super) fn type_decl_from_item(
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
            return Some(self.enum_decl_from_json(crate_name, name, enum_item, index));
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
            let lifted_members = field_ids
                .into_iter()
                .filter_map(|id| index.get(id))
                .filter_map(|field| self.tuple_member_from_json(crate_name, field))
                .collect::<Vec<_>>();
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
                    members,
                    span: Span::default(),
                }),
                field_conversions: Vec::new(),
                constructor_arg_conversions,
                enum_variant_conversions: Vec::new(),
            });
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
    ) -> ImportedTypeDecl {
        let lifted_members = item_ids(enum_item, "variants")
            .into_iter()
            .filter_map(|id| index.get(id))
            .filter_map(|variant| self.enum_member_from_json(crate_name, variant, index))
            .collect::<Vec<_>>();
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

        ImportedTypeDecl {
            decl: TypeDecl::Enum(EnumTypeDecl {
                visibility: Visibility::public(),
                ident: TypeIdent::new(name),
                members,
                span: Span::default(),
            }),
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions,
        }
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
            Some(kind) => self.enum_variant_fields_from_kind(crate_name, kind, index),
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
    ) -> Vec<LiftedEnumVariantField> {
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
    pub(super) fn param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<Param> {
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
            let return_conversion = match name {
                "Box" => RustReturnConversion::BoxDeref,
                "Rc" => RustReturnConversion::RcCloneDeref,
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

    pub(super) fn type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<TypeElement> {
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
        if let Some(slice) = inner(ty, "slice") {
            return self.lift_type_from_json(crate_name, slice).map(array_type);
        }
        if let Some(array) = inner(ty, "array") {
            let element = array.get("type").or_else(|| array.get("element"))?;
            return self
                .lift_type_from_json(crate_name, element)
                .map(array_type);
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
}

#[derive(Clone, Debug)]
pub(super) struct ImportedFunctionDecl {
    pub(super) decl: FnDecl,

    pub(super) return_conversion: RustReturnConversion,

    pub(super) arg_conversions: Vec<RustArgConversion>,
}

#[derive(Debug)]
pub(super) struct ImportedTypeDecl {
    pub(super) decl: TypeDecl,

    pub(super) field_conversions: Vec<RustFieldConversion>,

    pub(super) constructor_arg_conversions: Vec<RustArgConversion>,

    pub(super) enum_variant_conversions: Vec<RustEnumVariantConversion>,
}

impl ImportedTypeDecl {
    pub(super) fn new(decl: TypeDecl) -> Self {
        Self {
            decl,
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
        }
    }

    pub(super) fn empty(name: &str) -> Self {
        Self::new(empty_type_decl(name))
    }
}

#[derive(Debug)]
struct LiftedStructMember {
    member: StructTypeMember,
    arg_conversion: RustArgConversion,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Debug)]
struct LiftedTupleMember {
    member: TupleTypeMember,
    arg_conversion: RustArgConversion,
}

#[derive(Debug)]
struct LiftedEnumMember {
    member: EnumTypeMember,
    arg_conversions: Vec<RustEnumVariantArgConversion>,
}

#[derive(Debug)]
struct LiftedEnumVariantField {
    field: EnumVariantField,
    arg_conversion: RustArgConversion,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
struct LiftedReturn {
    ty: TypeElement,
    decl_modifier: Option<galvan_ast::DeclModifier>,
    pub(super) return_conversion: RustReturnConversion,
}

#[derive(Clone, Debug)]
struct LiftedParam {
    param: Param,
    arg_conversion: RustArgConversion,
}

#[derive(Clone, Debug)]
pub(super) struct LiftedType {
    ty: TypeElement,
    decl_modifier: Option<galvan_ast::DeclModifier>,
    arg_conversion: RustArgConversion,
}

impl LiftedType {
    pub(super) fn new(ty: TypeElement) -> Self {
        Self {
            ty,
            decl_modifier: None,
            arg_conversion: RustArgConversion::None,
        }
    }

    pub(super) fn with_modifier(ty: TypeElement, decl_modifier: galvan_ast::DeclModifier) -> Self {
        Self {
            ty,
            decl_modifier: Some(decl_modifier),
            arg_conversion: RustArgConversion::None,
        }
    }
}

fn member_arg_conversion(return_conversion: RustReturnConversion) -> RustArgConversion {
    match return_conversion {
        RustReturnConversion::None => RustArgConversion::None,
        RustReturnConversion::BoxDeref => RustArgConversion::BoxNew,
        RustReturnConversion::RcCloneDeref => RustArgConversion::RcNew,
    }
}

fn empty_type_decl(name: &str) -> TypeDecl {
    TypeDecl::Empty(EmptyTypeDecl {
        visibility: Visibility::public(),
        ident: TypeIdent::new(name),
        span: Span::default(),
    })
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

fn array_type(inner: LiftedType) -> LiftedType {
    LiftedType::new(TypeElement::Array(Box::new(ArrayTypeItem {
        elements: inner.ty,
        span: Span::default(),
    })))
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

pub(super) fn plain_type(ident: TypeIdent) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident,
        span: Span::default(),
    })
}

pub(super) fn generic_type(name: &str) -> TypeElement {
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
