//! Token definitions for the Salvo lexer.

use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// True when at least one newline appears between the previous token and
    /// this one. Used by the parser for implicit statement termination.
    pub newline_before: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Literals and identifiers
    Ident(String),
    /// Integer literal [lit-numeric]: `1` is `Int`; the `L` suffix
    /// (`1L`, `long: true`) makes it a `Long`.
    Int { value: i64, long: bool },
    /// Floating-point literal [lit-numeric]: `1.2` is `Double`; the `f`
    /// suffix (`1.2f`, `single: true`) makes it a `Float`.
    Float { value: f64, single: bool },
    /// String literal, decomposed into text and `${...}` interpolation parts.
    Str(Vec<StrPart>),
    Char(char),
    /// Raw template contents between `` delimiters (used in `define` blocks).

    // Keywords
    KwFn,
    KwLet,
    KwStruct,
    KwQualifier,
    KwEffect,
    KwHandler,
    KwParams,
    KwType,
    KwIntrinsic,
    KwPlatform,
    KwImport,
    KwAs,
    KwOf,
    KwWith,
    KwCanbe,
    KwProvenance,
    /// `refn` — a qualifier *refinement* of an existing function
    /// [qual-refn].
    KwRefn,
    KwIs,
    KwIf,
    KwElif,
    KwElse,
    KwWhen,
    KwWhile,
    KwFor,
    KwIn,
    KwReturn,
    KwBreak,
    KwContinue,
    KwYield,
    KwUse,
    KwDefer,
    KwTry,
    KwTrue,
    KwFalse,

    // Punctuation and operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Dot,
    Ellipsis, // ...
    Arrow,    // ->
    Pipe,     // |
    PipePipe, // ||
    AmpAmp,   // &&
    Question, // ?
    Bang,     // !
    Eq,       // =
    EqEq,     // ==
    BangEq,   // !=
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    PlusPlus, // ++
    Caret,    // ^ — the qualifier-widening check [qual-widen]

    Eof,
}

/// A piece of a string literal: either literal text or an interpolated
/// expression. Interpolations keep their raw source and the byte offset of
/// that source within the file, so the parser can parse them with correct
/// spans.
#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Text(String),
    /// Raw expression source of a `${...}` interpolation and the byte offset
    /// where it starts in the original file.
    Interp { source: String, offset: u32 },
}

/// Every Salvo keyword, paired with its token. The single source of truth
/// shared by the lexer (`TokenKind::keyword`) and tooling that needs the
/// keyword inventory (e.g. `salvo lang tm-grammar` [cli-lang]).
pub const KEYWORDS: &[(&str, TokenKind)] = &[
    ("fn", TokenKind::KwFn),
    ("let", TokenKind::KwLet),
    ("struct", TokenKind::KwStruct),
    ("qualifier", TokenKind::KwQualifier),
    ("effect", TokenKind::KwEffect),
    ("handler", TokenKind::KwHandler),
    ("params", TokenKind::KwParams),
    ("type", TokenKind::KwType),
    ("intrinsic", TokenKind::KwIntrinsic),
    ("platform", TokenKind::KwPlatform),
    ("import", TokenKind::KwImport),
    ("as", TokenKind::KwAs),
    ("of", TokenKind::KwOf),
    ("with", TokenKind::KwWith),
    ("canbe", TokenKind::KwCanbe),
    ("provenance", TokenKind::KwProvenance),
    ("refn", TokenKind::KwRefn),
    ("is", TokenKind::KwIs),
    ("if", TokenKind::KwIf),
    ("elif", TokenKind::KwElif),
    ("else", TokenKind::KwElse),
    ("when", TokenKind::KwWhen),
    ("while", TokenKind::KwWhile),
    ("for", TokenKind::KwFor),
    ("in", TokenKind::KwIn),
    ("return", TokenKind::KwReturn),
    ("break", TokenKind::KwBreak),
    ("continue", TokenKind::KwContinue),
    ("yield", TokenKind::KwYield),
    ("use", TokenKind::KwUse),
    ("defer", TokenKind::KwDefer),
    ("try", TokenKind::KwTry),
    ("true", TokenKind::KwTrue),
    ("false", TokenKind::KwFalse),
];

impl TokenKind {
    pub fn keyword(ident: &str) -> Option<TokenKind> {
        KEYWORDS
            .iter()
            .find(|(text, _)| *text == ident)
            .map(|(_, kind)| kind.clone())
    }

    /// Human-readable description for diagnostics.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(name) => format!("identifier `{name}`"),
            TokenKind::Int { value, long } => {
                format!("integer `{value}{}`", if *long { "L" } else { "" })
            }
            TokenKind::Float { value, single } => {
                format!("float `{value}{}`", if *single { "f" } else { "" })
            }
            TokenKind::Str(_) => "string literal".to_string(),
            TokenKind::Char(c) => format!("character literal `{c}`"),
            TokenKind::Eof => "end of file".to_string(),
            other => format!("`{}`", other.symbol()),
        }
    }

    /// The source spelling of a symbol or keyword token. Keywords resolve
    /// through `KEYWORDS`, the single source of truth, so adding one to the
    /// table is enough — a missing arm here used to panic a diagnostic.
    fn symbol(&self) -> &'static str {
        if let Some((text, _)) = KEYWORDS.iter().find(|(_, kind)| kind == self) {
            return text;
        }
        match self {
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Comma => ",",
            TokenKind::Colon => ":",
            TokenKind::Dot => ".",
            TokenKind::Ellipsis => "...",
            TokenKind::Arrow => "->",
            TokenKind::Pipe => "|",
            TokenKind::PipePipe => "||",
            TokenKind::AmpAmp => "&&",
            TokenKind::Question => "?",
            TokenKind::Bang => "!",
            TokenKind::Eq => "=",
            TokenKind::EqEq => "==",
            TokenKind::BangEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::Gt => ">",
            TokenKind::LtEq => "<=",
            TokenKind::GtEq => ">=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::PlusPlus => "++",
            TokenKind::Caret => "^",
            _ => unreachable!("symbol() called on non-symbol token"),
        }
    }
}
