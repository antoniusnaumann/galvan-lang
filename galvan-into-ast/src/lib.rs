use galvan_ast::{Ast, Point, RootItem, SegmentedAsts, Span, ToplevelItem};
use galvan_files::Source;
use galvan_parse::*;

mod result;
pub use result::{AstError, AstResult};

mod modifiers;
pub(crate) use modifiers::*;

mod items;

mod cursor_macro;
pub(crate) use cursor_macro::*;

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
        parsed.try_into_ast(self).map(|ast| ast.with_source(self))
    }
}

impl IntoAst for ParseTree {
    fn try_into_ast(self, source: Source) -> AstResult {
        let root = self.root_node();
        let mut cursor = root.walk();
        
        let mut ast = Ast { toplevel: vec![], source };

        if cursor.goto_first_child() {
            let node = cursor.node();
            let item = RootItem::read_cursor(&mut cursor, &source)?;

            ast.toplevel.push(item);

            while cursor.goto_next_sibling() {
                let node = cursor.node();
                let item = RootItem::read_cursor(&mut cursor, &source)?;

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
        let mut main = None;

        for item in self.toplevel {
            match item {
                RootItem::Type(item) => types.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Fn(item) => functions.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Test(item) => tests.push(ToplevelItem {
                    item,
                    source: self.source.clone(),
                }),
                RootItem::Main(item) => {
                    if main.is_some() {
                        return Err(AstError::DuplicateMain);
                    }

                    main = Some(ToplevelItem {
                        item,
                        source: self.source.clone(),
                    })
                }
            }
        }

        Ok(SegmentedAsts {
            types,
            functions,
            tests,
            main,
        })
    }
}

impl SegmentAst for Vec<Ast> {
    fn segmented(self) -> Result<SegmentedAsts, AstError> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut main = None;
        let segmented = self.into_iter().map(SegmentAst::segmented);

        for ast in segmented {
            let ast = ast?;
            types.extend(ast.types);
            functions.extend(ast.functions);
            tests.extend(ast.tests);
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
        })
    }
}

pub trait SpanExt {
    fn from_node(node: Node<'_>) -> Span;
}

impl SpanExt for Span {
    fn from_node(node: Node<'_>) {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let start_point = node.start_position();
        let end_point = node.end_position();

        let start = Point { row: start_point.row, col: start_point.column };
        let end = Point { row: end_point.row, col: end_point.column };

        Span { range: (start_byte, end_byte), start, end }
    }
}


