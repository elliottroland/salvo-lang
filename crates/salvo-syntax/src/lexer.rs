//! Hand-written lexer for Salvo source code.

use crate::diag::Diagnostic;
use crate::span::Span;
use crate::token::{StrPart, Token, TokenKind};

pub struct LexResult {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
    /// Every `//` comment in source order [doc-comment]. Comments are not
    /// tokens (the grammar never sees them); they are collected here so
    /// the parser can attach the run above a declaration as its docs.
    pub comments: Vec<Comment>,
}

/// One `//` comment line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    /// Spans the `//` through the end of the line (excluding the newline).
    pub span: Span,
    /// The text after `//`, with a single leading space removed so that
    /// `// text` and `//text` read alike; further indentation is kept for
    /// markdown [doc-markdown].
    pub text: String,
    /// Whether only whitespace precedes the `//` on its line. A trailing
    /// comment (`let x = 1 // note`) documents nothing.
    pub own_line: bool,
}

/// Lex a full source file into tokens. Never fails: unknown characters are
/// reported as diagnostics and skipped.
pub fn lex(source: &str) -> LexResult {
    Lexer::new(source).run()
}

struct Lexer<'s> {
    source: &'s str,
    chars: Vec<(usize, char)>,
    /// Index into `chars`.
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    comments: Vec<Comment>,
    newline_pending: bool,
}

impl<'s> Lexer<'s> {
    fn new(source: &'s str) -> Self {
        Lexer {
            source,
            chars: source.char_indices().collect(),
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            comments: Vec::new(),
            newline_pending: false,
        }
    }

    fn run(mut self) -> LexResult {
        while let Some(c) = self.peek() {
            let start = self.offset();
            match c {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.newline_pending = true;
                    self.bump();
                }
                '/' if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    self.push_comment(start);
                }
                '`' if self.peek_at(1) == Some('`') => self.template(start),
                '"' => self.string(start),
                '\'' => self.char_literal(start),
                c if c.is_ascii_digit() => self.number(start),
                c if c.is_alphabetic() || c == '_' => self.ident(start),
                _ => self.symbol(start),
            }
        }
        let end = self.source.len() as u32;
        self.push(TokenKind::Eof, Span::new(end, end));
        LexResult {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
            comments: self.comments,
        }
    }

    // --- Character helpers ---

    fn peek(&self) -> Option<char> {
        self.peek_at(0)
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).map(|&(_, c)| c)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// Byte offset of the current position.
    fn offset(&self) -> u32 {
        self.chars
            .get(self.pos)
            .map(|&(i, _)| i as u32)
            .unwrap_or(self.source.len() as u32)
    }

    fn push(&mut self, kind: TokenKind, span: Span) {
        let newline_before = std::mem::take(&mut self.newline_pending);
        self.tokens.push(Token {
            kind,
            span,
            newline_before,
        });
    }

    fn push_here(&mut self, kind: TokenKind, start: u32) {
        let span = Span::new(start, self.offset());
        self.push(kind, span);
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    /// Records the `//` comment running from `start` to the current
    /// position [doc-comment].
    fn push_comment(&mut self, start: u32) {
        let span = Span::new(start, self.offset());
        let raw = &self.source[(start as usize + 2)..span.end as usize];
        let text = raw.strip_prefix(' ').unwrap_or(raw).to_string();
        let line_start = self.source[..start as usize]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let own_line = self.source[line_start..start as usize]
            .chars()
            .all(char::is_whitespace);
        self.comments.push(Comment {
            span,
            text,
            own_line,
        });
    }

    // --- Token scanners ---

    fn ident(&mut self, start: u32) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                text.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let kind = TokenKind::keyword(&text).unwrap_or(TokenKind::Ident(text));
        self.push_here(kind, start);
    }

    /// Scans a numeric literal [lit-numeric]: `1` (`Int`), `1L` (`Long`),
    /// `1.2` (`Double`), `1.2f` (`Float`). The `f` suffix requires a
    /// decimal point; `L` forbids one.
    fn number(&mut self, start: u32) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    text.push(c);
                }
                self.bump();
            } else {
                break;
            }
        }
        // A float requires `.` followed by a digit (so `1.size()` stays an
        // int followed by a method call).
        //
        // [expr-tuple-index] A digit sequence that *follows* a `.` is a
        // tuple index, so it never takes a decimal point of its own:
        // `t.0.1` is element 0 of element... 1, two indices — not `t.0`
        // and the float `0.1`. Nothing else in the grammar puts a numeric
        // literal directly after a dot (paths and dot-calls take
        // identifiers, spread is one `...` token), so the rule is
        // unambiguous.
        let after_dot = matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(TokenKind::Dot)
        );
        let mut is_float = false;
        if !after_dot
            && self.peek() == Some('.')
            && self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            text.push('.');
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
        }
        // Optional suffix: `f` (Float, fractional literals only) or `L`
        // (Long, integer literals only) [lit-numeric].
        let suffix = match self.peek() {
            Some(c @ ('f' | 'L')) => {
                self.bump();
                Some(c)
            }
            _ => None,
        };
        // The literal must end here: `1.2fx` or `10Lx` is an error, and a
        // suffixless literal followed by an identifier char (`10x`) too.
        let lit_end = self.offset();
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.bump();
        }
        let span = Span::new(start, self.offset());
        if span.end > lit_end {
            let source = &self.source[start as usize..span.end as usize];
            self.error(format!("invalid numeric literal `{source}`"), span);
            return;
        }
        match suffix {
            Some('f') if !is_float => {
                self.error(
                    format!("float suffix `f` requires a decimal point (write `{text}.0f`)"),
                    span,
                );
            }
            Some('L') if is_float => {
                self.error(
                    format!("long suffix `L` is not valid on the float literal `{text}`"),
                    span,
                );
            }
            Some('f') => match text.parse::<f64>() {
                Ok(value) => self.push(TokenKind::Float { value, single: true }, span),
                Err(_) => self.error(format!("invalid float literal `{text}`"), span),
            },
            Some('L') => match text.parse::<i64>() {
                Ok(value) => self.push(TokenKind::Int { value, long: true }, span),
                Err(_) => self.error(format!("invalid integer literal `{text}`"), span),
            },
            _ if is_float => match text.parse::<f64>() {
                Ok(value) => self.push(TokenKind::Float { value, single: false }, span),
                Err(_) => self.error(format!("invalid float literal `{text}`"), span),
            },
            _ => match text.parse::<i64>() {
                Ok(value) => self.push(TokenKind::Int { value, long: false }, span),
                Err(_) => self.error(format!("invalid integer literal `{text}`"), span),
            },
        }
    }

    /// Scans a `"..."` string literal, splitting out `${...}` interpolations.
    fn string(&mut self, start: u32) {
        self.bump(); // consume opening quote
        let mut parts: Vec<StrPart> = Vec::new();
        let mut text = String::new();
        loop {
            let Some(c) = self.peek() else {
                let span = Span::new(start, self.offset());
                self.error("unterminated string literal", span);
                break;
            };
            match c {
                '"' => {
                    self.bump();
                    break;
                }
                '\n' => {
                    let span = Span::new(start, self.offset());
                    self.error("unterminated string literal", span);
                    break;
                }
                '\\' => {
                    self.bump();
                    match self.bump() {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some('r') => text.push('\r'),
                        Some('\\') => text.push('\\'),
                        Some('"') => text.push('"'),
                        Some('$') => text.push('$'),
                        Some('\'') => text.push('\''),
                        Some('0') => text.push('\0'),
                        other => {
                            let span = Span::new(self.offset().saturating_sub(1), self.offset());
                            self.error(
                                match other {
                                    Some(c) => format!("unknown escape sequence `\\{c}`"),
                                    None => "unterminated string literal".to_string(),
                                },
                                span,
                            );
                        }
                    }
                }
                '$' if self.peek_at(1) == Some('{') => {
                    if !text.is_empty() {
                        parts.push(StrPart::Text(std::mem::take(&mut text)));
                    }
                    self.bump(); // $
                    self.bump(); // {
                    let expr_start = self.offset();
                    let mut depth = 1usize;
                    while let Some(c) = self.peek() {
                        match c {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            '"' | '\n' => break, // nested strings unsupported inside `${}`
                            _ => {}
                        }
                        self.bump();
                    }
                    let expr_end = self.offset();
                    if self.peek() == Some('}') {
                        self.bump();
                    } else {
                        let span = Span::new(expr_start, expr_end);
                        self.error("unterminated `${...}` interpolation", span);
                    }
                    let source = self.source[expr_start as usize..expr_end as usize].to_string();
                    parts.push(StrPart::Interp {
                        source,
                        offset: expr_start,
                    });
                }
                _ => {
                    text.push(c);
                    self.bump();
                }
            }
        }
        if !text.is_empty() || parts.is_empty() {
            parts.push(StrPart::Text(text));
        }
        self.push_here(TokenKind::Str(parts), start);
    }

    fn char_literal(&mut self, start: u32) {
        self.bump(); // opening quote
        let c = match self.bump() {
            Some('\\') => match self.bump() {
                Some('n') => Some('\n'),
                Some('t') => Some('\t'),
                Some('r') => Some('\r'),
                Some('\\') => Some('\\'),
                Some('\'') => Some('\''),
                Some('"') => Some('"'),
                Some('0') => Some('\0'),
                _ => None,
            },
            Some(c) if c != '\'' && c != '\n' => Some(c),
            _ => None,
        };
        let closed = self.peek() == Some('\'');
        if closed {
            self.bump();
        }
        let span = Span::new(start, self.offset());
        match (c, closed) {
            (Some(c), true) => self.push(TokenKind::Char(c), span),
            _ => self.error("invalid character literal", span),
        }
    }

    /// Scans a raw template between double-backtick delimiters, as used in
    /// `define` blocks: `` inline: ``...`` ``.
    fn template(&mut self, start: u32) {
        self.bump(); // `
        self.bump(); // `
        let content_start = self.offset();
        let content_end;
        loop {
            if self.peek().is_none() {
                let span = Span::new(start, self.offset());
                self.error("unterminated `` template (missing closing ``)", span);
                content_end = self.offset();
                break;
            }
            if self.peek() == Some('`') && self.peek_at(1) == Some('`') {
                content_end = self.offset();
                self.bump();
                self.bump();
                break;
            }
            self.bump();
        }
        let raw = self.source[content_start as usize..content_end as usize].to_string();
        self.push_here(TokenKind::Template(dedent_template(&raw)), start);
    }

    fn symbol(&mut self, start: u32) {
        let c = self.bump().expect("symbol() requires a current char");
        let two = self.peek();
        let kind = match (c, two) {
            ('-', Some('>')) => {
                self.bump();
                TokenKind::Arrow
            }
            ('.', Some('.')) if self.peek_at(1) == Some('.') => {
                self.bump();
                self.bump();
                TokenKind::Ellipsis
            }
            ('|', Some('|')) => {
                self.bump();
                TokenKind::PipePipe
            }
            ('&', Some('&')) => {
                self.bump();
                TokenKind::AmpAmp
            }
            ('=', Some('=')) => {
                self.bump();
                TokenKind::EqEq
            }
            ('!', Some('=')) => {
                self.bump();
                TokenKind::BangEq
            }
            ('<', Some('=')) => {
                self.bump();
                TokenKind::LtEq
            }
            ('>', Some('=')) => {
                self.bump();
                TokenKind::GtEq
            }
            ('+', Some('+')) => {
                self.bump();
                TokenKind::PlusPlus
            }
            ('(', _) => TokenKind::LParen,
            (')', _) => TokenKind::RParen,
            ('{', _) => TokenKind::LBrace,
            ('}', _) => TokenKind::RBrace,
            ('[', _) => TokenKind::LBracket,
            (']', _) => TokenKind::RBracket,
            (',', _) => TokenKind::Comma,
            (':', _) => TokenKind::Colon,
            ('.', _) => TokenKind::Dot,
            ('|', _) => TokenKind::Pipe,
            ('?', _) => TokenKind::Question,
            ('!', _) => TokenKind::Bang,
            ('=', _) => TokenKind::Eq,
            ('<', _) => TokenKind::Lt,
            ('>', _) => TokenKind::Gt,
            ('+', _) => TokenKind::Plus,
            ('-', _) => TokenKind::Minus,
            ('*', _) => TokenKind::Star,
            ('/', _) => TokenKind::Slash,
            ('%', _) => TokenKind::Percent,
            ('^', _) => TokenKind::Caret,
            _ => {
                let span = Span::new(start, self.offset());
                self.error(format!("unexpected character `{c}`"), span);
                return;
            }
        };
        self.push_here(kind, start);
    }
}

/// Strips a leading/trailing blank line and common indentation from template
/// content, so that
///
/// ```text
///     inline: ``
///     ${str}.length
///     ``
/// ```
///
/// yields exactly `${str}.length`.
fn dedent_template(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.lines().collect();
    if lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| if l.len() >= indent { &l[indent..] } else { l })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    fn kinds(source: &str) -> (Vec<TokenKind>, Vec<Diagnostic>) {
        let result = lex(source);
        let kinds = result
            .tokens
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| *k != TokenKind::Eof)
            .collect();
        (kinds, result.diagnostics)
    }

    // [lit-numeric] `1` Int, `1L` Long, `1.2` Double, `1.2f` Float;
    // underscores allowed.
    #[test]
    fn numeric_literal_suffixes() {
        let (kinds, diags) = kinds("1 1L 1.2 1.2f 1_000L 3.5");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![
                TokenKind::Int { value: 1, long: false },
                TokenKind::Int { value: 1, long: true },
                TokenKind::Float { value: 1.2, single: false },
                TokenKind::Float { value: 1.2, single: true },
                TokenKind::Int { value: 1000, long: true },
                TokenKind::Float { value: 3.5, single: false },
            ]
        );
    }

    // [lit-numeric] `f` requires a decimal point; `L` forbids one; and a
    // literal must not run into identifier characters.
    #[test]
    fn invalid_numeric_suffixes() {
        let (_, diags) = kinds("1f");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("requires a decimal point"));

        let (_, diags) = kinds("1.2L");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("not valid on the float literal"));

        let (_, diags) = kinds("10x");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("invalid numeric literal `10x`"));

        let (_, diags) = kinds("1.2fx");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("invalid numeric literal `1.2fx`"));
    }

    // The float rule is unchanged by suffixes: `1.size()` lexes as an
    // int, `.`, ident — not a float.
    #[test]
    fn dot_method_call_on_int_is_not_a_float() {
        let (kinds, diags) = kinds("1.size");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![
                TokenKind::Int { value: 1, long: false },
                TokenKind::Dot,
                TokenKind::Ident("size".into()),
            ]
        );
    }
}
