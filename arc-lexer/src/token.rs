use logos::Logos;

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\f]+")]
enum Token {
    // Delimiters
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
    #[regex(r"\r|[\r?\n]")]
    Newline,

    // Comments
    #[token("///")]
    TripleSlash,
    #[token("//")]
    DoubleSlash,
    #[token("/*")]
    StarSlashOpen,
    #[token("*/")]
    StarSlashClose,
    #[token("/**")]
    DoubleStarSlashOpen,
    #[token("**/")]
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

    // Logical Operators
    #[token("||")]
    LogicalOr,
    #[token("&&")]
    LogicalAnd,
    #[token("!")]
    LogicalNot,

    // Logical Operator keywords
    #[token("or")]
    LogicalOrKeyword,
    #[token("and")]
    LogicalAndKeyword,
    #[token("not")]
    LogicalNotKeyword,

    // Comparison Operators
    #[token("==")]
    Equals,
    // TODO: more

    // Access
    #[token("::")]
    DoubleColon,
    #[token(".")]
    Dot,
    #[token("_")]
    Underscore,

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

    #[token("~")]
    StoredRefPrefix,
    #[token("&")]
    LocalRefPrefix,

    // TODO: Capture name and content separately
    #[regex(r"@[_|a-z|A-Z][\(\)]?")]
    Annotation(),
    // TODO: Allow all unicode characters that are valid for Rusts identifiers
    // TODO: Handle raw identifiers
    // TODO: Allow ? in identifiers and handle that in parser
    #[regex(r"[_|a-z|A-Z]", |lex| lex.slice().to_owned())]
    Ident(String),
}

impl Default for Token {
    fn default() -> Self {
        Self::BraceOpen
    }
}
