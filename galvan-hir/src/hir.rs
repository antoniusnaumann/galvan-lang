//! Typed high-level intermediate representation (HIR) of a Galvan program.
//!
//! The HIR is produced from the AST by the typechecker in [`crate::typecheck`].
//! Compared to the AST it:
//!
//! - reifies control flow (`if`, `for`, `try`, `else`) into dedicated nodes
//!   instead of magic function calls with trailing closures
//! - resolves names: function calls are split into [`HirFunctionCall`],
//!   [`HirMethodCall`], builtins ([`HirPrint`], [`HirAssert`]) and constructor
//!   calls with materialized default values
//! - stores the inferred type and [`Ownership`] of every expression
//! - makes all ownership and type coercions explicit through [`Adjustment`]s
//!
//! Code generation consumes the HIR mechanically and never makes type or
//! ownership decisions on its own. The [`Ownership`] stored in each node
//! therefore *is* the ownership of the generated Rust expression.

use galvan_ast::{
    ArithmeticOperator, BitwiseOperator, CmdSignature, ComparisonOperator, DeclModifier,
    FnSignature, Ident, LogicalOperator, Ownership, RangeOperator, Span, StringLiteral,
    ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl, UsePath,
};
use galvan_files::Source;

/// A fully typechecked Galvan program.
#[derive(Debug)]
pub struct HirModule {
    pub uses: Vec<ToplevelItem<UseDecl>>,
    pub types: Vec<ToplevelItem<TypeDecl>>,
    pub functions: Vec<HirFunction>,
    pub tests: Vec<HirTest>,
    pub main: Option<HirMain>,
    pub cmds: Vec<HirCmd>,
}

#[derive(Debug)]
pub struct HirFunction {
    pub signature: FnSignature,
    pub body: HirBlock,
    pub source: Source,
    pub span: Span,
}

impl HirFunction {
    pub fn is_member_function(&self) -> bool {
        self.signature.receiver().is_some()
    }
}

#[derive(Debug)]
pub struct HirTest {
    pub name: Option<StringLiteral>,
    pub body: HirBlock,
    pub source: Source,
}

#[derive(Debug)]
pub struct HirMain {
    pub kind: HirMainKind,
    pub body: HirBlock,
    pub source: Source,
}

#[derive(Debug)]
pub enum HirMainKind {
    Function { argument: Option<Ident> },
    Command { signature: CmdSignature },
}

#[derive(Debug)]
pub struct HirCmd {
    pub signature: CmdSignature,
    pub body: HirBlock,
    pub source: Source,
    pub span: Span,
}

/// A sequence of statements. When the block produces a value, the last
/// statement is an [`HirStatement::Expression`] that has already been coerced
/// to the type expected by the surrounding context.
#[derive(Clone, Debug)]
pub struct HirBlock {
    pub statements: Vec<HirStatement>,
    /// Type of the value this block evaluates to,
    /// [`TypeElement::void()`] if it does not produce one.
    pub ty: TypeElement,
    pub span: Span,
}

impl HirBlock {
    pub fn is_void(&self) -> bool {
        matches!(self.ty, TypeElement::Void(_))
    }

    /// The trailing expression that determines the block's value, if any
    pub fn trailing_expression(&self) -> Option<&HirExpression> {
        match self.statements.last() {
            Some(HirStatement::Expression(expression)) => Some(expression),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum HirStatement {
    Declaration(HirDeclaration),
    Assignment(HirAssignment),
    Expression(HirExpression),
    Return(HirReturn),
    Throw(HirThrow),
    Break(Span),
    Continue(Span),
}

#[derive(Clone, Debug)]
pub struct HirDeclaration {
    pub modifier: DeclModifier,
    pub identifier: Ident,
    /// Annotated or inferred type of the declared variable
    pub ty: TypeElement,
    pub value: Option<HirExpression>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HirAssignment {
    pub target: HirExpression,
    /// `true` when assigning through a mutable reference, requiring `*target`
    pub deref_target: bool,
    pub operator: HirAssignmentOperator,
    pub value: HirExpression,
    pub span: Span,
}

/// Assignment operators with the `++=` shape already resolved by the
/// typechecker
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirAssignmentOperator {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    PowAssign,
    ConcatAssign(ConcatKind),
}

#[derive(Clone, Debug)]
pub struct HirReturn {
    pub expression: HirExpression,
    pub is_explicit: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HirThrow {
    pub expression: HirExpression,
    pub span: Span,
}

/// A typed expression.
///
/// `ty` and `ownership` describe the value produced by `kind` *before*
/// `adjustments` are applied. The typechecker appends adjustments to coerce
/// the expression to what its context expects.
#[derive(Clone, Debug)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub ty: TypeElement,
    pub ownership: Ownership,
    pub adjustments: Vec<Adjustment>,
    pub span: Span,
}

impl HirExpression {
    pub fn new(kind: HirExpressionKind, ty: TypeElement, ownership: Ownership, span: Span) -> Self {
        Self {
            kind,
            ty,
            ownership,
            adjustments: Vec::new(),
            span,
        }
    }

    /// Placeholder expression emitted when lowering failed. Carries a comment
    /// that is included in the generated code.
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self::new(
            HirExpressionKind::Error(message.into()),
            TypeElement::infer(),
            Ownership::UniqueOwned,
            span,
        )
    }

    pub fn adjusted(mut self, adjustment: Adjustment) -> Self {
        self.adjustments.push(adjustment);
        self
    }

    /// Ownership of the expression after all adjustments are applied
    pub fn adjusted_ownership(&self) -> Ownership {
        self.adjustments
            .last()
            .map(|adjustment| match adjustment {
                Adjustment::Borrow => Ownership::Borrowed,
                Adjustment::MutBorrow => Ownership::MutBorrowed,
                Adjustment::Deref => Ownership::UniqueOwned,
                Adjustment::ToOwned => Ownership::UniqueOwned,
                Adjustment::WrapSome => Ownership::UniqueOwned,
                Adjustment::WrapOk => Ownership::UniqueOwned,
                Adjustment::WrapErr => Ownership::UniqueOwned,
                Adjustment::LockRef => Ownership::MutBorrowed,
                Adjustment::ArcClone => Ownership::UniqueOwned,
            })
            .unwrap_or(self.ownership)
    }
}

/// An explicit coercion inserted by the typechecker.
///
/// Adjustments are rendered around the expression in order, e.g.
/// `[Borrow]` renders `&expr` and `[ToOwned, WrapSome]` renders
/// `Some(expr.to_owned())`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adjustment {
    /// `&expr`
    Borrow,
    /// `&mut expr`
    MutBorrow,
    /// `*expr`
    Deref,
    /// `expr.to_owned()`
    ToOwned,
    /// `Some(expr)`
    WrapSome,
    /// `Ok(expr)`
    WrapOk,
    /// `Err(expr)`
    WrapErr,
    /// `expr.lock().unwrap()` - access the value behind a `ref` variable
    LockRef,
    /// `::std::sync::Arc::clone(&expr)` - share a `ref` variable
    ArcClone,
}

#[derive(Clone, Debug)]
pub enum HirExpressionKind {
    If(Box<HirIf>),
    ElseUnwrap(Box<HirElseUnwrap>),
    Try(Box<HirTry>),
    For(Box<HirFor>),
    Match(Box<HirMatch>),
    Assert(Box<HirAssert>),
    Print(HirPrint),
    FunctionCall(HirFunctionCall),
    MethodCall(Box<HirMethodCall>),
    FieldAccess(Box<HirFieldAccess>),
    SafeAccess(Box<HirSafeAccess>),
    ConstructorCall(HirConstructorCall),
    EnumConstructor(HirEnumConstructor),
    EnumAccess(HirEnumAccess),
    RustConstant(HirRustConstant),
    Literal(HirLiteral),
    Variable(Ident),
    Collection(HirCollection),
    Closure(Box<HirClosure>),
    Logical(Box<HirBinary<LogicalOperator>>),
    Arithmetic(Box<HirBinary<ArithmeticOperator>>),
    Bitwise(Box<HirBinary<BitwiseOperator>>),
    Comparison(Box<HirBinary<ComparisonOperator>>),
    CollectionOp(Box<HirBinary<CollectionOperator>>),
    Range(Box<HirBinary<RangeOperator>>),
    Index(Box<HirIndex>),
    /// Error propagation with the `!` postfix operator, transpiled to `?`
    Yeet(Box<HirExpression>),
    Group(Box<HirExpression>),
    /// Lowering failed; renders as a comment placeholder
    Error(String),
}

/// Collection infix operators (`++`, `--`, `in`). The concrete generated shape
/// depends on the stored operand types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionOperator {
    Concat(ConcatKind),
    Remove,
    Contains,
}

/// Shape of a `++` concatenation, decided by the typechecker from the
/// operand types.
///
/// The shape also fixes the ownership contract of the right-hand side:
/// `Element` values are coerced to an owned element by the typechecker
/// (they are consumed by `push`/`insert`), while `Collection` and
/// `Stringify` values are taken by reference or cloned inside the
/// generated pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcatKind {
    /// The right-hand side is a single element appended to the collection
    /// (`push`/`insert`)
    Element,
    /// The right-hand side is a collection of the same shape
    /// (`concat`/`union`/string append)
    Collection,
    /// The right-hand side is stringified and appended (strings only)
    Stringify,
}

#[derive(Clone, Debug)]
pub struct HirIf {
    pub condition: HirExpression,
    pub then_block: HirBlock,
    pub else_block: Option<HirBlock>,
    /// `true` when this `if` has no `else` but is used as an expression of
    /// type `T?`: the `then` tail is already wrapped in `Some` and codegen
    /// emits an `else { None }` branch.
    pub wraps_optional: bool,
}

/// `receiver else { block }` - unwrap an optional or fall back to the block.
///
/// Renders as `if let Some(__value) = receiver { __value } else { block }`,
/// where the `__value` expression carries the adjustments needed by the
/// surrounding context.
#[derive(Clone, Debug)]
pub struct HirElseUnwrap {
    pub kind: HirElseUnwrapKind,
    pub receiver: HirExpression,
    /// Bind with a `ref` pattern (`Some(ref __value)`) to avoid moving out of
    /// a receiver that is used again later
    pub by_ref: bool,
    /// The unwrapped value (a `__value` variable with coercion adjustments)
    pub value: HirExpression,
    pub err_binding: Option<Ident>,
    pub else_block: HirBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirElseUnwrapKind {
    Optional,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryKind {
    Optional,
    Result,
}

/// `try cond |bindings| { body } else |err| { else_block }`
///
/// With an `else` branch this renders as a `match` on the scrutinee. Without
/// one it renders as a call to the runtime support function `r#try`.
#[derive(Clone, Debug)]
pub struct HirTry {
    pub condition: HirExpression,
    pub kind: TryKind,
    pub ok_bindings: Vec<Ident>,
    pub err_binding: Option<Ident>,
    pub body: HirBlock,
    pub else_block: Option<HirBlock>,
}

#[derive(Clone, Debug)]
pub struct HirFor {
    /// Loop variables; the implicit `it` parameter is materialized here.
    pub bindings: Vec<HirForBinding>,
    pub iterable_kind: HirForIterableKind,
    pub iterable: HirExpression,
    pub body: HirBlock,
    /// `Some(element_type)` when the loop is used as an expression and
    /// collects the value of each iteration into a vector
    pub collect: Option<TypeElement>,
}

#[derive(Clone, Debug)]
pub struct HirForBinding {
    pub ident: Ident,
    /// Destructure this binding with a `&` pattern when iterating borrowed
    /// collections of `Copy` values.
    pub deref: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirForIterableKind {
    Normal,
    Tuple { len: usize },
}

#[derive(Clone, Debug)]
pub struct HirMatch {
    pub scrutinee: HirExpression,
    pub arms: Vec<HirMatchArm>,
}

#[derive(Clone, Debug)]
pub struct HirMatchArm {
    pub pattern: HirMatchPattern,
    pub body: HirBlock,
}

#[derive(Clone, Debug)]
pub enum HirMatchPattern {
    Wildcard,
    EnumVariant(HirEnumMatchPattern),
}

#[derive(Clone, Debug)]
pub struct HirEnumMatchPattern {
    pub target: TypeIdent,
    pub case: TypeIdent,
    pub arguments: HirMatchPatternArguments,
}

#[derive(Clone, Debug)]
pub enum HirMatchPatternArguments {
    None,
    Tuple(Vec<HirMatchBindingPattern>),
    Named(Vec<HirNamedMatchBinding>),
}

#[derive(Clone, Debug)]
pub struct HirNamedMatchBinding {
    pub field: Ident,
    pub binding: HirMatchBindingPattern,
}

#[derive(Clone, Debug)]
pub enum HirMatchBindingPattern {
    Binding(Ident),
    Wildcard,
}

#[derive(Clone, Debug)]
pub enum HirAssert {
    /// `assert_eq!(lhs, rhs, args...)`
    Eq(HirExpression, HirExpression, Vec<HirExpression>),
    /// `assert_ne!(lhs, rhs, args...)`
    Ne(HirExpression, HirExpression, Vec<HirExpression>),
    /// `assert!(args...)`
    Truthy(Vec<HirExpression>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintKind {
    Print,
    Println,
    Debug,
    Panic,
}

#[derive(Clone, Debug)]
pub struct HirPrint {
    pub kind: PrintKind,
    pub args: Vec<HirExpression>,
}

#[derive(Clone, Debug)]
pub struct HirFunctionCall {
    pub namespace: Option<UsePath>,
    pub rust_path: Option<Box<str>>,
    pub rust_return_conversion: galvan_rustdoc::RustReturnConversion,
    pub rust_arg_conversions: Vec<galvan_rustdoc::RustArgConversion>,
    pub ident: Ident,
    pub labels: Vec<Ident>,
    pub args: Vec<HirExpression>,
}

#[derive(Clone, Debug)]
pub struct HirMethodCall {
    pub receiver: HirExpression,
    pub receiver_modifier: Option<DeclModifier>,
    pub namespace: Option<UsePath>,
    pub rust_path: Option<Box<str>>,
    pub rust_return_conversion: galvan_rustdoc::RustReturnConversion,
    pub rust_receiver_conversion: galvan_rustdoc::RustArgConversion,
    pub rust_arg_conversions: Vec<galvan_rustdoc::RustArgConversion>,
    pub ident: Ident,
    pub labels: Vec<Ident>,
    pub args: Vec<HirExpression>,
}

#[derive(Clone, Debug)]
pub struct HirRustConstant {
    pub rust_path: Box<str>,
}

#[derive(Clone, Debug)]
pub struct HirFieldAccess {
    pub receiver: HirExpression,
    pub field: Ident,
}

/// How a safe access (`?.`) unwraps and re-wraps the accessed value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeAccessStyle {
    /// `receiver.as_ref().map(|__elem__| { __elem__.access.clone() })`
    RefClone,
    /// `receiver.map(|__elem__| { __elem__.access.clone() })`
    Clone,
    /// `receiver.map(|__elem__| { __elem__.access })`
    Move,
}

#[derive(Clone, Debug)]
pub enum SafeAccessKind {
    Field(Ident),
    Call(Option<UsePath>, Ident, Vec<Ident>, Vec<HirExpression>),
}

#[derive(Clone, Debug)]
pub struct HirSafeAccess {
    pub receiver: HirExpression,
    pub access: SafeAccessKind,
    pub style: SafeAccessStyle,
}

/// A constructor call with all fields present in declaration order;
/// defaults for omitted fields are already materialized.
#[derive(Clone, Debug)]
pub struct HirConstructorCall {
    pub ident: TypeIdent,
    pub kind: HirConstructorKind,
    pub args: Vec<HirConstructorArg>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirConstructorKind {
    Struct,
    Tuple,
}

#[derive(Clone, Debug)]
pub struct HirConstructorArg {
    pub field: Ident,
    pub value: HirExpression,
    pub store_as_ref: bool,
}

#[derive(Clone, Debug)]
pub struct HirEnumConstructor {
    pub target: TypeIdent,
    pub case: TypeIdent,
    pub args: Vec<HirEnumConstructorArg>,
}

#[derive(Clone, Debug)]
pub struct HirEnumConstructorArg {
    pub field: Option<Ident>,
    pub value: HirExpression,
}

#[derive(Clone, Debug)]
pub struct HirEnumAccess {
    pub target: TypeIdent,
    pub case: TypeIdent,
}

#[derive(Clone, Debug)]
pub enum HirLiteral {
    Boolean(bool),
    Number(String),
    Char(char),
    None,
    String(HirStringLiteral),
}

#[derive(Clone, Debug)]
pub struct HirStringLiteral {
    /// The Rust format string literal including quotes and `{}` placeholders.
    /// Literal braces are escaped as `{{` and `}}`.
    pub value: String,
    pub interpolations: Vec<HirExpression>,
}

#[derive(Clone, Debug)]
pub enum HirCollection {
    Array(Vec<HirExpression>),
    Set(Vec<HirExpression>),
    Dict(Vec<HirDictElement>),
    OrderedDict(Vec<HirDictElement>),
}

#[derive(Clone, Debug)]
pub struct HirDictElement {
    pub key: HirExpression,
    pub value: HirExpression,
}

#[derive(Clone, Debug)]
pub struct HirClosure {
    pub parameters: Vec<HirClosureParam>,
    pub body: HirBlock,
}

#[derive(Clone, Debug)]
pub struct HirClosureParam {
    pub ident: Ident,
    pub ty: TypeElement,
    /// Bind with a `&` pattern to undo the extra reference introduced by
    /// borrowing iterator adapters such as `filter`
    pub deref: bool,
}

#[derive(Clone, Debug)]
pub struct HirBinary<Op> {
    pub lhs: HirExpression,
    pub operator: Op,
    pub rhs: HirExpression,
}

/// Index access `base[index]`. Whether the index is borrowed depends on the
/// stored type of `base` (dictionaries and sets index by reference).
#[derive(Clone, Debug)]
pub struct HirIndex {
    pub base: HirExpression,
    pub index: HirExpression,
}
