use std::fmt::Display;

use logos::Logos;

use crate::LexerString;

pub trait GalvanToken {
    fn stringified(&self) -> String;
}

// TODO: Create an error type here that contains useful information
pub type Error = ();

#[derive(Clone, Debug, PartialEq, Eq, Logos)]
#[logos(skip r"[ \t\f]+")]
pub enum Token {
    // Delimiters
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token("<")]
    PointyBracketOpen,
    #[token(">")]
    PointyBracketClose,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[regex(r"\r?\n")]
    Newline,

    // Comments
    // For now comments are simply skipped
    // TODO: Capture comments and add them to syntax tree
    // TODO: Invoke separate parser for comment, this
    //  allows checking doc comments and allowing nested comments
    #[regex(r"///[^\n]*", logos::skip)]
    DocComment, //(&'source str),
    #[regex(r"//[^/][^\n]*", logos::skip)]
    Comment, //(&'source str),
    #[regex(r"/\*\*([^*]|\*[^/])*\*/", logos::skip, priority = 6)]
    MultiLineDocComment, //(&'source str),
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip, priority = 5)]
    MultiLineComment, //(&'source str),
    // Tokens for multi line comments that can be used to detect dangling comments
    #[token("/*", priority = 1)]
    StarSlashOpen,
    #[token("*/", priority = 1)]
    StarSlashClose,
    #[token("/**", priority = 1)]
    DoubleStarSlashOpen,
    #[token("**/", priority = 1)]
    DoubleStarSlashClose,

    // Arithmetic Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Divide,
    #[token("*")]
    Multiply,
    #[token("=")]
    Assign,
    #[token("%")]
    Remainder,

    // Combined Arithmetic Operators
    #[token("/=")]
    DivideAssign,
    #[token("*=")]
    MultiplyAssign,
    #[token("-=")]
    MinusAssign,
    #[token("+=")]
    PlusAssign,
    #[token("**")]
    Pow,
    #[token("**=")]
    PowAssing,

    // Logical Operators
    #[token("||")]
    LogicalOr,
    #[token("&&")]
    LogicalAnd,
    // More generic token name, since operator is also used for short circuiting on error
    #[token("!")]
    ExclamationMark,

    // Logical Operator keywords
    #[token("or")]
    LogicalOrKeyword,
    #[token("and")]
    LogicalAndKeyword,
    #[token("not")]
    LogicalNotKeyword,
    #[token("xor")]
    LogicalXorKeyword,

    // Comparison Operators
    #[token("==")]
    Equals,
    #[token("!=")]
    NotEquals,
    #[token(">=")]
    GreaterEquals,
    #[token("<=")]
    SmallerEquals,

    // Other Keywords
    #[token("is")]
    IsKeyword,
    #[token("assert")]
    AssertKeyword,

    // Access
    #[token("::")]
    DoubleColon,
    #[token(".")]
    Dot,
    #[token("_")]
    Underscore,

    // Error and Null Handling
    #[token("?.")]
    SafeCall,
    #[token("?")]
    QuestionMark,
    #[token("??")]
    CatchOperator,
    #[token("??=")]
    NullCoalescingAssign,
    #[token("catch")]
    CatchKeyword,

    // Declaration Keywords
    #[token("stored")]
    StoredKeyword,
    #[token("val")]
    ValKeyword,
    #[token("ref")]
    RefKeyword,
    #[token("fn")]
    FnKeyword,
    #[token("type")]
    TypeKeyword,
    #[token("data")]
    DataKeyword,
    #[token("pub")]
    PublicKeyword,
    #[token("const")]
    ConstKeyword,
    #[token("async")]
    AsyncKeyword,
    #[token("trait")]
    TraitKeyword,
    #[token("test")]
    TestKeyword,
    #[token("main")]
    MainKeyword,
    // reserved for future use
    #[token("build")]
    BuildKeyword,
    #[token("infix")]
    InfixKeyword,
    // not used but reserved
    #[token("struct")]
    StructKeyword,

    // Control Keywords
    #[token("return")]
    ReturnKeyword,
    #[token("yield")]
    YieldKeyword,
    #[token("move")]
    MoveKeyword,
    #[token("copy")]
    CopyKeyword,
    #[token("break")]
    BreakKeyword,

    // Operator Shorthands for control keywords
    #[token("->")]
    ReturnOperator,
    #[token(":>")]
    YieldOperator,
    #[token("<-")]
    MoveOperator,
    #[token("<:")]
    CopyOperator,
    #[token("|>")]
    BreakOperator,

    #[token("$")]
    StoredRefPrefix,
    #[token("&")]
    LocalRefPrefix,
    // TODO: Parse complete annotation as token and
    // capture name and content separately
    // #[regex(r"@[_a-zA-Z]?[_a-zA-Z0-9]*")]
    // Annotation(),
    #[token("@")]
    AtSign,
    // TODO: Allow all unicode characters that are valid for Rusts identifiers
    // TODO: Handle raw identifiers
    // TODO: Allow ? in identifiers and handle that in parser
    #[regex(r"[_a-zA-Z]?[_a-zA-Z0-9]*", |lex| LexerString::from(lex.slice()))]
    Ident(LexerString),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Stringify token here
        write!(f, "{:?}", self)
    }
}
