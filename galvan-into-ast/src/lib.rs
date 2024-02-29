use galvan_ast::AstResult;
use galvan_files::Source;

pub trait IntoAst {
    fn try_into_ast(self) -> AstResult;
}

impl IntoAst for Source {
    fn try_into_ast(self) -> AstResult {
        let parsed = parse_source(&self)?;
        parsed.try_into_ast().map(|ast| ast.with_source(self))
    }
}
