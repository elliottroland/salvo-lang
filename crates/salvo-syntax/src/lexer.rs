//! Hand-written lexer for Salvo source code.

use crate::diag::Diagnostic;
use crate::span::Span;
use crate::token::{StrPart, Token, TokenKind};

pub struct LexResult {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
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
        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
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
        let span = Span::new(start, self.offset());
        if is_float {
            match text.parse::<f64>() {
                Ok(v) => self.push(TokenKind::Float(v), span),
                Err(_) => self.error(format!("invalid float literal `{text}`"), span),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => self.push(TokenKind::Int(v), span),
                Err(_) => self.error(format!("invalid integer literal `{text}`"), span),
            }
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
