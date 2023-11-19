use galvan_ast::*;
use crate::{ParserNodes, ParserNode, Rule, Span};

pub struct AstError {
    message: String,
    span: Span,
    kind: AstErrorKind,
    expected: Option<Vec<Rule>>,
}

pub enum AstErrorKind {
    Unexpected,
    Missing,
}

impl AstError {
    fn unexpected(node: ParserNode<'_>) -> Self {
        AstError {
            message: "Unexpected node".into(),
            span: node.as_span().into(),
            kind: AstErrorKind::Unexpected,
            expected: None,
        }
    }

    fn missing() -> Self {
        // TODO: Meaningful span
        AstError {
            message: "Missing node".into(),
            span: Span(0, 0),
            kind: AstErrorKind::Missing,
            expected: None,
        }
    }

    fn with_expected(mut self, expected: Vec<Rule>) -> Self {
        self.expected = Some(expected);
        self
    }
}

pub struct Ast {
    top_level: Vec<RootItem>
}

pub type AstResult = Result<Ast, AstError>;


impl TryFrom<ParserNodes<'_>> for Ast {
    type Error = AstError;

    fn try_from(value: ParserNodes<'_>) -> AstResult {
        let top_level = value.into_iter().map(|node| {
            match node.as_rule() {
                Rule::toplevel => RootItem::from_nodes(node.into_inner()),
                _ => Err(AstError::unexpected(node))
            }
        }).collect::<Result<Vec<_>, AstError>>()?;

        Ok(Ast {
            top_level
        })
    }
}

trait FromNode {
    fn from_nodes(node: ParserNodes<'_>) -> Result<Self, AstError> where Self: Sized;
}

trait EnsureSingleNode<'a> {
    fn single(self) -> Result<ParserNode<'a>, AstError>;
}

impl<'a> EnsureSingleNode<'a> for ParserNodes<'a> {
    fn single(self) -> Result<ParserNode<'a>, AstError> {
        let mut iter = self.into_iter();
        let first = iter.next().ok_or_else(AstError::missing)?;
        if let Some(node) = iter.next() {
            Err(AstError::unexpected(node))
        } else {
            Ok(first)
        }
    }
}

// TODO: ast_node should be a ty but then it does not compile somehow
macro_rules! map_rule(
    ($node:expr, $($rule:ident => $ast_node:ident,)*) => {
        match $node.as_rule() {
            $(Rule::$rule => $ast_node::from_nodes($node.into_inner()).map(|n| n.into()),)*
            _ => Err(AstError::unexpected($node).with_expected(vec![$(Rule::$rule,)*])),
        }
    }
);

macro_rules! impl_from_node_single_map(
    ($ty:ty, $($mapping:tt)*) => {
        impl FromNode for $ty {
            fn from_nodes(node: ParserNodes<'_>) -> Result<Self, AstError> {
                let node = node.single()?;
                map_rule! { node, $($mapping)* }
            }
        }
    }
);

impl_from_node_single_map! { RootItem,
    main => MainDecl,
    function => FnDecl,
    type_decl => TypeDecl,
    test => TestDecl,
}

impl_from_node_single_map! { TypeDecl,
    struct_type_decl => StructTypeDecl,
    alias_type_decl => AliasTypeDecl,
    tuple_type_decl => TupleTypeDecl,
}

impl FromNode for

impl FromNode for MainDecl {
    fn from_nodes(node: ParserNodes<'_>) -> Result<Self, AstError> {
        todo!()
    }
}

impl FromNode for FnDecl {
    fn from_nodes(node: ParserNodes<'_>) -> Result<Self, AstError> {
        todo!()
    }
}

impl FromNode for TestDecl {
    fn from_nodes(node: ParserNodes<'_>) -> Result<Self, AstError> {
        todo!()
    }
}