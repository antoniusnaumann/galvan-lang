use galvan_ast::{
    Ast, FnDecl, MainDecl, MainKind, Point, RootItem, SegmentedAsts, Span, ToplevelItem,
    TypeElement, VisibilityKind,
};
use galvan_files::Source;
use galvan_parse::*;

mod cursor_macro;
mod items;
mod modifiers;
mod result;

use result::CursorUtil;
pub use result::{AstError, AstResult};

pub trait IntoAst {
    fn try_into_ast(self, source: Source) -> AstResult;
}

pub trait SourceIntoAst {
    fn try_into_ast(self) -> AstResult;
}

pub trait ReadCursor: Sized {
    fn read_cursor(cursor: &mut TreeCursor<'_>, source: &str) -> Result<Self, AstError>;
}

impl SourceIntoAst for Source {
    fn try_into_ast(self) -> AstResult {
        let parsed = parse_source(&self)?;
        parsed.try_into_ast(self)
    }
}

impl IntoAst for ParseTree {
    fn try_into_ast(self, source: Source) -> AstResult {
        let root = self.root_node();
        let mut cursor = root.walk();

        let mut ast = Ast {
            toplevel: vec![],
            source,
        };

        if cursor.child() {
            let item = RootItem::read_cursor(&mut cursor, &ast.source)?;

            ast.toplevel.push(item);

            while cursor.next() {
                let item = RootItem::read_cursor(&mut cursor, &ast.source)?;

                ast.toplevel.push(item);
            }
        }

        Ok(ast)
    }
}

pub trait SegmentAst {
    fn segmented(self) -> Result<SegmentedAsts, AstError>;
}

impl SegmentAst for Ast {
    fn segmented(self) -> Result<SegmentedAsts, AstError> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut cmds = Vec::new();
        let mut main = None;

        for item in self.toplevel {
            match item {
                RootItem::Type(item) => types.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Fn(item) if item.signature.identifier.as_str() == "main" => {
                    if main.is_some() {
                        return Err(AstError::DuplicateMain);
                    }

                    main = Some(ToplevelItem {
                        item: main_decl(item)?,
                        source: self.source.clone(),
                    });
                }
                RootItem::Fn(item) => functions.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Test(item) => tests.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Cmd(item) if item.signature.identifier.as_str() == "main" => {
                    if main.is_some() {
                        return Err(AstError::DuplicateMain);
                    }

                    main = Some(ToplevelItem {
                        item: MainDecl {
                            kind: MainKind::Command(item.signature),
                            body: item.body,
                            span: item.span,
                        },
                        source: self.source.clone(),
                    });
                }
                RootItem::Cmd(item) => cmds.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
            }
        }

        Ok(SegmentedAsts {
            types,
            functions,
            tests,
            main,
            cmds,
        })
    }
}

fn main_decl(function: FnDecl) -> Result<MainDecl, AstError> {
    let FnDecl {
        signature,
        body,
        span,
    } = function;
    let mut parameters = signature.parameters.params.into_iter();
    let argument = parameters.next();

    let valid_argument = argument.as_ref().is_none_or(|parameter| {
        parameter.decl_modifier.is_none()
            && parameter.short_name.is_none()
            && matches!(
                &parameter.param_type,
                TypeElement::Array(array)
                    if matches!(
                        &array.elements,
                        TypeElement::Plain(string) if string.ident.as_str() == "String"
                    )
            )
    });
    let valid_signature = parameters.next().is_none()
        && valid_argument
        && signature.visibility.kind == VisibilityKind::Inherited
        && matches!(signature.return_type, TypeElement::Void(_))
        && signature.where_clause.is_none();

    if !valid_signature {
        return Err(AstError::InvalidMainSignature);
    }

    Ok(MainDecl {
        kind: MainKind::Function { argument },
        body,
        span,
    })
}

impl SegmentAst for Vec<Ast> {
    fn segmented(self) -> Result<SegmentedAsts, AstError> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut cmds = Vec::new();
        let mut main = None;
        let segmented = self.into_iter().map(SegmentAst::segmented);

        for ast in segmented {
            let ast = ast?;
            types.extend(ast.types);
            functions.extend(ast.functions);
            tests.extend(ast.tests);
            cmds.extend(ast.cmds);
            if let Some(main_decl) = ast.main {
                if main.is_some() {
                    return Err(AstError::DuplicateMain);
                }

                main = Some(main_decl);
            }
        }

        Ok(SegmentedAsts {
            types,
            functions,
            tests,
            main,
            cmds,
        })
    }
}

pub trait SpanExt {
    fn from_node(node: Node<'_>) -> Span;
}

impl SpanExt for Span {
    fn from_node(node: Node<'_>) -> Span {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let start_point = node.start_position();
        let end_point = node.end_position();

        let start = Point {
            row: start_point.row,
            col: start_point.column,
        };
        let end = Point {
            row: end_point.row,
            col: end_point.column,
        };

        Span {
            range: (start_byte, end_byte),
            start,
            end,
        }
    }
}
