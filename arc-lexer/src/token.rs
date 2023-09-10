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
    #[regex(r"\r?\n]")]
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
    #[token("!=")]
    NotEquals,
    #[token(">=")]
    GreaterEquals,
    #[token("<=")]
    SmallerEquals,

    // Access
    #[token("::")]
    DoubleColon,
    #[token(".")]
    Dot,
    #[token("_")]
    Underscore,

    // Error Handling
    #[token("?.")]
    SafeCall,
    #[token("?")]
    QuestionMark,
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

    #[token("~")]
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
    #[regex(r"[_a-zA-Z]?[_a-zA-Z0-9]*", |lex| lex.slice().to_owned())]
    Ident(String),
}
