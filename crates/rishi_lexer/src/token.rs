use rishi_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,

    // Identifiers
    Ident(String),

    // Keywords
    Let,
    Mut,
    Fn,
    Return,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Match,
    Struct,
    Trait,
    Impl,
    Enum,
    Component,
    Spawn,
    Import,
    From,
    As,
    Const,
    SelfKw,

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Power,      // **
    Eq,         // =
    EqEq,       // ==
    NotEq,      // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    And,        // && or and
    Or,         // || or or
    Not,        // ! or not
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    Pipeline,   // |>
    Question,   // ?
    SafeDot,    // ?.
    NullCoalesce,// ??
    Arrow,      // ->
    FatArrow,   // =>
    DotDot,     // ..
    DotDotEq,   // ..=

    // Delimiters
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }
    Colon,      // :
    Comma,      // ,
    Dot,        // .
    Semicolon,  // ;

    // Indentation layout
    Newline,
    Indent,
    Dedent,

    // End of file
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
