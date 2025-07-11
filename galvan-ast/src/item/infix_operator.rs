use derive_more::From;
use galvan_ast_macro::AstNode;

use crate::{AstNode, Expression, Ident, PrintAst, Span};

pub trait InfixOperator {
    fn symbol(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq, From, AstNode)]
pub enum InfixExpression {
    Logical(InfixOperation<LogicalOperator>),
    Arithmetic(InfixOperation<ArithmeticOperator>),
    Collection(InfixOperation<CollectionOperator>),
    Comparison(InfixOperation<ComparisonOperator>),
    Member(InfixOperation<MemberOperator>),
    Custom(InfixOperation<CustomInfix>),
}

impl InfixExpression {
    pub fn is_comparison(&self) -> bool {
        matches!(self, Self::Comparison(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, From)]
pub struct InfixOperation<Op: InfixOperator> {
    pub lhs: Expression,
    pub operator: Op,
    pub rhs: Expression,
    pub span: Span,
}

impl<T: InfixOperator> AstNode for InfixOperation<T> {
    fn span(&self) -> Span {
        self.span
    }

    fn print(&self, indent: usize) -> String {
        let indent_str = " ".repeat(indent);
        let mut result = format!("{}{}\n", indent_str, stringify!(#struct_name));

        let field_value = self.lhs.print_ast(indent + 2);
        result.push_str(&format!("{}  {}{}\n", indent_str, "lhs", field_value));

        result.push_str(&format!(
            "{}  {}{}\n",
            indent_str,
            "op",
            self.operator.symbol()
        ));

        let field_value = self.lhs.print_ast(indent + 2);
        result.push_str(&format!("{}  {}{}\n", indent_str, "rhs", field_value));

        result
    }
}

impl InfixOperation<MemberOperator> {
    pub fn is_field(&self) -> bool {
        match self.rhs {
            Expression::Ident(_) => true,
            // TODO: Expression::Postfix(p) => match self p.without_postfix() {
            // Expression::Ident(_) => true, _ => false },
            _ => false,
        }
    }

    pub fn field_ident(&self) -> Option<&Ident> {
        match &self.rhs {
            Expression::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalOperator {
    Or,
    And,
    Xor,
}

impl InfixOperator for LogicalOperator {
    fn symbol(&self) -> &str {
        match self {
            LogicalOperator::Or => "||",
            LogicalOperator::And => "&&",
            LogicalOperator::Xor => "^",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
}

impl InfixOperator for ArithmeticOperator {
    fn symbol(&self) -> &str {
        match self {
            ArithmeticOperator::Add => "+",
            ArithmeticOperator::Sub => "-",
            ArithmeticOperator::Mul => "*",
            ArithmeticOperator::Div => "/",
            ArithmeticOperator::Rem => "%",
            ArithmeticOperator::Exp => "^",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionOperator {
    Concat,
    Remove,
    Contains,
}

impl InfixOperator for CollectionOperator {
    fn symbol(&self) -> &str {
        match self {
            CollectionOperator::Concat => "++",
            CollectionOperator::Remove => "--",
            CollectionOperator::Contains => "in",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    LessEqual,
    Less,
    GreaterEqual,
    Greater,
    Equal,
    NotEqual,
    Identical,
    NotIdentical,
}

impl InfixOperator for ComparisonOperator {
    fn symbol(&self) -> &str {
        match self {
            ComparisonOperator::LessEqual => "≤",
            ComparisonOperator::Less => "<",
            ComparisonOperator::GreaterEqual => "≥",
            ComparisonOperator::Greater => ">",
            ComparisonOperator::Equal => "==",
            ComparisonOperator::NotEqual => "≠",
            ComparisonOperator::Identical => "===",
            ComparisonOperator::NotIdentical => "!==",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberOperator {
    Dot,
    SafeCall,
}

impl InfixOperator for MemberOperator {
    fn symbol(&self) -> &str {
        match self {
            MemberOperator::Dot => ".",
            MemberOperator::SafeCall => "?.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomInfix(String);

impl InfixOperator for CustomInfix {
    fn symbol(&self) -> &str {
        &self.0
    }
}
