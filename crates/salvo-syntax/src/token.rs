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
    /// `rename fn add2 = add(a: Int, b: Int)` — a scope-local name for one
    /// overload, which stops answering to its own name [fn-rename].
    KwRename,
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
    /// [iter-fn] The `state { ... }` block at the top of an `iter fn`: the
    /// pass's own fields, declared like a struct's and initialized once per
    /// pass.
    ///
    /// `iter` itself is *not* a keyword — it has to stay callable, since that is
    /// the name of the function an `iter fn` generates — so the form is
    /// recognised at item level from `iter` followed by `fn`.
    KwState,
    KwUse,
    KwTry,
    KwTrue,
    KwFalse,
    /// [obligation-spelling] `proj` — the borrow obligation, in type
    /// positions and deduction entries (`proj(p)`).
    KwProj,
    /// [obligation-spelling] `once` — the at-most-once obligation.
    KwOnce,
    /// [obligation-spelling] `linear` — the exactly-once obligation:
    /// `linear struct X` declarations and `canbe linear` bounds.
    KwLinear,
    /// [proj-infer] `holds` — the opaque-projection annotation after a
    /// return type (`-> T holds proj(a)`). Reserved rather than contextual
    /// because a type is a chain of space-separated qualifiers, so a bare
    /// word after one would be read as another qualifier (user decision
    /// 2026-09-25).
    KwHolds,

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
    FatArrow, // =>
    Pipe,     // |
    PipePipe, // ||
    AmpAmp,   // &&
    Question, // ?
    /// [elvis] `?:` — the optional-or-else operator.
    QuestionColon,
    /// [safe-call] `?.` — a field read or dot-notation call on the non-`None`
    /// side of an optional.
    QuestionDot,
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
    MinusMinus, // --
    // [op-compound] Compound assignment: `x += 1` is `x = x + 1`.
    PlusEq,   // +=
    MinusEq,  // -=
    StarEq,   // *=
    SlashEq,  // /=
    Caret,    // ^ — the qualifier-widening check [qual-lift]
    At,       // @ — the scope selector on a call [fn-overload-at]

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
    ("rename", TokenKind::KwRename),
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
    ("state", TokenKind::KwState),
    ("use", TokenKind::KwUse),
    ("try", TokenKind::KwTry),
    ("true", TokenKind::KwTrue),
    ("false", TokenKind::KwFalse),
    // [obligation-spelling] The obligation keywords: lowercase, reserved —
    // compiler-owned behaviors, visually distinct from user qualifiers
    // (user decision 2026-09-12). `proj`/`once` appear in type positions;
    // `linear` before `struct` and in `canbe linear` bounds.
    ("proj", TokenKind::KwProj),
    ("once", TokenKind::KwOnce),
    ("linear", TokenKind::KwLinear),
    // [proj-infer] `holds` — the opaque projection, after the type it is
    // about: `-> Mut List<proj T> holds proj(it)`.
    ("holds", TokenKind::KwHolds),
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
            TokenKind::FatArrow => "=>",
            TokenKind::Pipe => "|",
            TokenKind::PipePipe => "||",
            TokenKind::AmpAmp => "&&",
            TokenKind::Question => "?",
            TokenKind::QuestionColon => "?:",
            TokenKind::QuestionDot => "?.",
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
            TokenKind::MinusMinus => "--",
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",
            TokenKind::Caret => "^",
            TokenKind::At => "@",
            _ => unreachable!("symbol() called on non-symbol token"),
        }
    }
}
