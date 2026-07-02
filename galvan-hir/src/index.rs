//! Position-indexed semantic information about a typechecked crate.
//!
//! The [`SymbolIndex`] is produced as a by-product of typechecking (see
//! [`crate::typecheck`]): while lowering the AST, the checker records every
//! *definition* (top-level functions and types, struct fields, enum variants,
//! parameters and local bindings) and every *reference* it resolves, each
//! keyed by the source file and byte span of the identifier token.
//!
//! This is the API that position-based tools (most importantly the language
//! server) build on:
//!
//! - [`SymbolIndex::symbol_at`] maps a cursor position to the definition it
//!   denotes (whether the cursor is on the definition itself or on a use),
//!   answering hover and go-to-definition queries.
//! - [`SymbolIndex::references`] enumerates all recorded uses of a
//!   definition, answering find-references queries.
//! - [`SymbolIndex::visible_locals`] lists the local bindings in scope at a
//!   position, for identifier completion.
//! - [`SymbolIndex::definitions`] enumerates all definitions, e.g. for
//!   completion of top-level names or members of a receiver type.

use std::collections::HashMap;
use std::path::Path;

use galvan_ast::{
    AstNode, DeclModifier, FnDecl, Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent,
};
use galvan_files::Source;

/// Index of a [`Definition`] within a [`SymbolIndex`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefinitionId(usize);

/// Index of a lexical scope within a [`SymbolIndex`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

/// A named program element that identifiers can resolve to.
#[derive(Clone, Debug)]
pub struct Definition {
    pub name: String,
    pub kind: DefinitionKind,
    /// The source the definition lives in ([`Source::Builtin`] for
    /// predefined functions, which have no location).
    pub source: Source,
    /// Span of the defining identifier token. Synthetic definitions
    /// (builtins, desugared bindings) have a default span.
    pub span: Span,
    /// Span of the whole declaration the definition belongs to.
    pub decl_span: Span,
}

impl Definition {
    /// The type associated with this definition, if it has one.
    pub fn ty(&self) -> Option<&TypeElement> {
        match &self.kind {
            DefinitionKind::Local { ty, .. }
            | DefinitionKind::Parameter { ty, .. }
            | DefinitionKind::Field { ty, .. } => Some(ty),
            DefinitionKind::Function { .. }
            | DefinitionKind::Type
            | DefinitionKind::EnumVariant { .. } => None,
        }
    }

    fn is_positional(&self) -> bool {
        self.span != Span::default()
    }
}

#[derive(Clone, Debug)]
pub enum DefinitionKind {
    /// A local binding: `let`/`mut`/`ref` declarations, loop variables,
    /// `try`/`else` bindings and match bindings.
    Local {
        modifier: DeclModifier,
        ty: TypeElement,
        scope: ScopeId,
    },
    /// A function or closure parameter.
    Parameter { ty: TypeElement, scope: ScopeId },
    /// A top-level function; methods carry their receiver type.
    Function { receiver: Option<TypeIdent> },
    /// A top-level type declaration.
    Type,
    /// A named member of a struct type. `owner` is the struct's name.
    Field { owner: TypeIdent, ty: TypeElement },
    /// A case of an enum type. `owner` is the enum's name.
    EnumVariant { owner: TypeIdent },
}

/// A resolved use of a [`Definition`] at a specific source position.
#[derive(Clone, Debug)]
pub struct Reference {
    pub source: Source,
    /// Span of the referencing identifier token.
    pub span: Span,
    pub definition: DefinitionId,
}

#[derive(Clone, Debug)]
struct ScopeRecord {
    source: Source,
    /// Byte range of source text in which the scope's bindings are visible.
    span: Span,
}

/// Position-indexed definitions, references and scopes of one typechecked
/// crate. See the [module documentation](self) for the available queries.
#[derive(Clone, Debug, Default)]
pub struct SymbolIndex {
    definitions: Vec<Definition>,
    references: Vec<Reference>,
    scopes: Vec<ScopeRecord>,
}

impl SymbolIndex {
    pub fn definition(&self, id: DefinitionId) -> &Definition {
        &self.definitions[id.0]
    }

    /// All definitions in the crate (including builtins).
    pub fn definitions(&self) -> impl Iterator<Item = (DefinitionId, &Definition)> {
        self.definitions
            .iter()
            .enumerate()
            .map(|(i, def)| (DefinitionId(i), def))
    }

    /// The definition whose defining identifier token covers `offset` in
    /// `file`.
    pub fn definition_at(&self, file: &Path, offset: usize) -> Option<DefinitionId> {
        self.definitions
            .iter()
            .position(|def| {
                def.is_positional()
                    && contains(def.span, offset)
                    && def.source.origin() == Some(file)
            })
            .map(DefinitionId)
    }

    /// The definition referenced by the identifier token covering `offset`
    /// in `file`.
    pub fn reference_at(&self, file: &Path, offset: usize) -> Option<DefinitionId> {
        self.references
            .iter()
            .find(|reference| {
                contains(reference.span, offset) && reference.source.origin() == Some(file)
            })
            .map(|reference| reference.definition)
    }

    /// The definition denoted by the identifier at `offset` in `file`,
    /// whether the position is on the definition itself or on a reference.
    pub fn symbol_at(&self, file: &Path, offset: usize) -> Option<DefinitionId> {
        self.reference_at(file, offset)
            .or_else(|| self.definition_at(file, offset))
    }

    /// All recorded references to `definition`, across the whole crate.
    pub fn references(&self, definition: DefinitionId) -> impl Iterator<Item = &Reference> {
        self.references
            .iter()
            .filter(move |reference| reference.definition == definition)
    }

    /// The local bindings and parameters visible at `offset` in `file`:
    /// those whose scope covers the position and whose declaration precedes
    /// it. Shadowed bindings are not filtered out.
    pub fn visible_locals<'a>(
        &'a self,
        file: &'a Path,
        offset: usize,
    ) -> impl Iterator<Item = (DefinitionId, &'a Definition)> + 'a {
        self.definitions
            .iter()
            .enumerate()
            .filter(move |(_, def)| {
                let scope = match def.kind {
                    DefinitionKind::Local { scope, .. } => scope,
                    DefinitionKind::Parameter { scope, .. } => scope,
                    _ => return false,
                };
                let scope = &self.scopes[scope.0];
                def.span.range.1 <= offset
                    && contains(scope.span, offset)
                    && scope.source.origin() == Some(file)
            })
            .map(|(i, def)| (DefinitionId(i), def))
    }
}

fn contains(span: Span, offset: usize) -> bool {
    span.range.0 <= offset && offset < span.range.1
}

/// Records definitions, references and scopes while the typechecker lowers a
/// crate. Consumed into a [`SymbolIndex`] by [`IndexBuilder::finish`].
#[derive(Debug)]
pub(crate) struct IndexBuilder {
    index: SymbolIndex,
    scope_stack: Vec<ScopeId>,
    /// Function declarations, keyed by declaration position (file and
    /// signature span), so that references found during lowering can be
    /// mapped back to the definition they resolve to.
    functions: HashMap<DeclKey, DefinitionId>,
    types: HashMap<String, DefinitionId>,
    /// Struct fields and enum variants, keyed by `(owner name, member name)`.
    members: HashMap<(String, String), DefinitionId>,
    current_source: Source,
}

/// Identifies a top-level declaration by its location. Builtins (which all
/// share a default span) are additionally keyed by name.
type DeclKey = (Option<std::path::PathBuf>, (usize, usize), String);

fn decl_key<T>(item: &ToplevelItem<T>, span: Span, name: &str) -> DeclKey
where
    T: galvan_ast::RootItemMarker,
{
    (
        item.source.origin().map(Path::to_path_buf),
        span.range,
        name.to_owned(),
    )
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self {
            index: SymbolIndex::default(),
            scope_stack: Vec::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            members: HashMap::new(),
            current_source: Source::Missing,
        }
    }

    pub fn set_current_source(&mut self, source: Source) {
        self.current_source = source;
    }

    pub fn push_scope(&mut self, span: Span) {
        let id = ScopeId(self.index.scopes.len());
        self.index.scopes.push(ScopeRecord {
            source: self.current_source.clone(),
            span,
        });
        self.scope_stack.push(id);
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn add_definition(&mut self, definition: Definition) -> DefinitionId {
        let id = DefinitionId(self.index.definitions.len());
        self.index.definitions.push(definition);
        id
    }

    /// Record a local binding or parameter declared in the current scope.
    /// Synthetic bindings (desugared identifiers without a source span) are
    /// not indexed.
    pub fn define_binding(
        &mut self,
        ident: &galvan_ast::Ident,
        modifier: DeclModifier,
        ty: &TypeElement,
        is_parameter: bool,
    ) -> Option<DefinitionId> {
        let span = ident.span();
        if span == Span::default() {
            return None;
        }
        let Some(&scope) = self.scope_stack.last() else {
            return None;
        };
        let kind = if is_parameter {
            DefinitionKind::Parameter {
                ty: ty.clone(),
                scope,
            }
        } else {
            DefinitionKind::Local {
                modifier,
                ty: ty.clone(),
                scope,
            }
        };
        Some(self.add_definition(Definition {
            name: ident.as_str().to_owned(),
            kind,
            source: self.current_source.clone(),
            span,
            decl_span: span,
        }))
    }

    /// Record a top-level function declaration.
    pub fn define_function(&mut self, func: &ToplevelItem<FnDecl>) {
        let signature = &func.item.signature;
        let receiver = signature.receiver().and_then(|param| match &param.param_type {
            TypeElement::Plain(basic) => Some(basic.ident.clone()),
            _ => None,
        });
        // The declaration span is the signature (not the whole body): it is
        // what tools render when presenting the function.
        let id = self.add_definition(Definition {
            name: signature.identifier.as_str().to_owned(),
            kind: DefinitionKind::Function { receiver },
            source: func.source.clone(),
            span: signature.identifier.span(),
            decl_span: signature.span,
        });
        let key = decl_key(func, signature.span, signature.identifier.as_str());
        self.functions.insert(key, id);
    }

    /// Record a top-level type declaration along with its named members
    /// (struct fields and enum variants).
    pub fn define_type(&mut self, decl: &ToplevelItem<TypeDecl>) {
        let ident = decl.item.ident().clone();
        let id = self.add_definition(Definition {
            name: ident.as_str().to_owned(),
            kind: DefinitionKind::Type,
            source: decl.source.clone(),
            span: ident.span(),
            decl_span: decl.item.span(),
        });
        self.types.insert(ident.as_str().to_owned(), id);

        match &decl.item {
            TypeDecl::Struct(decl_struct) => {
                for member in &decl_struct.members {
                    let member_id = self.add_definition(Definition {
                        name: member.ident.as_str().to_owned(),
                        kind: DefinitionKind::Field {
                            owner: ident.clone(),
                            ty: member.r#type.clone(),
                        },
                        source: decl.source.clone(),
                        span: member.ident.span(),
                        decl_span: member.span,
                    });
                    self.members.insert(
                        (ident.as_str().to_owned(), member.ident.as_str().to_owned()),
                        member_id,
                    );
                }
            }
            TypeDecl::Enum(decl_enum) => {
                for member in &decl_enum.members {
                    let member_id = self.add_definition(Definition {
                        name: member.ident.as_str().to_owned(),
                        kind: DefinitionKind::EnumVariant {
                            owner: ident.clone(),
                        },
                        source: decl.source.clone(),
                        span: member.ident.span(),
                        decl_span: member.span,
                    });
                    self.members.insert(
                        (ident.as_str().to_owned(), member.ident.as_str().to_owned()),
                        member_id,
                    );
                }
            }
            TypeDecl::Tuple(_) | TypeDecl::Alias(_) | TypeDecl::Empty(_) => {}
        }
    }

    /// Record a reference at `span` to an already-registered definition.
    pub fn reference(&mut self, span: Span, definition: DefinitionId) {
        if span == Span::default() {
            return;
        }
        self.index.references.push(Reference {
            source: self.current_source.clone(),
            span,
            definition,
        });
    }

    /// Record a reference to a resolved function declaration.
    pub fn reference_function(&mut self, span: Span, func: &ToplevelItem<FnDecl>) {
        let signature = &func.item.signature;
        let key = decl_key(func, signature.span, signature.identifier.as_str());
        if let Some(&id) = self.functions.get(&key) {
            self.reference(span, id);
        }
    }

    /// Record a reference to a type by name.
    pub fn reference_type(&mut self, ident: &TypeIdent) {
        if let Some(&id) = self.types.get(ident.as_str()) {
            self.reference(ident.span(), id);
        }
    }

    /// Record a reference to a struct field or enum variant of `owner`.
    pub fn reference_member(&mut self, owner: &TypeIdent, name: &str, span: Span) {
        if let Some(&id) = self
            .members
            .get(&(owner.as_str().to_owned(), name.to_owned()))
        {
            self.reference(span, id);
        }
    }

    /// Record references to every named type inside `ty` (e.g. a type
    /// annotation `[Dog: Int?]` references both `Dog` and `Int`).
    pub fn reference_type_element(&mut self, ty: &TypeElement) {
        let mut idents = Vec::new();
        ty.visit_type_idents(&mut |ident| idents.push(ident.clone()));
        for ident in idents {
            self.reference_type(&ident);
        }
    }

    /// Record references to the types used in a type declaration's members
    /// (struct field types, tuple element types, alias targets, enum variant
    /// field types).
    pub fn reference_type_decl_members(&mut self, decl: &TypeDecl) {
        match decl {
            TypeDecl::Struct(decl) => {
                for member in &decl.members {
                    self.reference_type_element(&member.r#type);
                }
            }
            TypeDecl::Tuple(decl) => {
                for member in &decl.members {
                    self.reference_type_element(&member.r#type);
                }
            }
            TypeDecl::Alias(decl) => self.reference_type_element(&decl.r#type),
            TypeDecl::Enum(decl) => {
                for member in &decl.members {
                    for field in &member.fields {
                        self.reference_type_element(&field.r#type);
                    }
                }
            }
            TypeDecl::Empty(_) => {}
        }
    }

    pub fn finish(self) -> SymbolIndex {
        self.index
    }
}
