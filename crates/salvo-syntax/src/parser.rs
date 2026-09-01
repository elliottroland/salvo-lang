//! Recursive-descent parser for Salvo.
//!
//! Design notes:
//! * Salvo has no statement terminators; statements end at newlines. Tokens
//!   carry a `newline_before` flag, and infix/postfix continuation across a
//!   newline is only allowed inside parentheses/brackets (`group_depth > 0`).
//! * Struct literals (`Person { ... }`) are ambiguous with blocks in
//!   condition position (`if x is Person { ... }`), so struct-literal
//!   speculation is disabled while parsing conditions (`no_struct`)
//!   [if-bool].
//! * Type names and qualifiers are uppercase by convention; `is` checks use
//!   this to distinguish the checked type from an optional binding
//!   (`if x is Str s`) [is-binding].

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::lexer;
use crate::span::Span;
use crate::token::{StrPart, Token, TokenKind};

pub struct Parser<'s> {
    #[allow(dead_code)]
    source: &'s str,
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    /// Depth of explicit grouping (parens/brackets); newlines are ignored
    /// inside groups.
    group_depth: u32,
    /// When true, do not speculate struct literals / trailing braces
    /// (condition position).
    no_struct: bool,
}

struct Snapshot {
    pos: usize,
    diag_len: usize,
}

impl<'s> Parser<'s> {
    pub fn new(source: &'s str, tokens: Vec<Token>) -> Self {
        Parser {
            source,
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            group_depth: 0,
            no_struct: false,
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    // --- Token helpers ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.kind() == kind
    }

    fn at_eof(&self) -> bool {
        matches!(self.kind(), TokenKind::Eof)
    }

    fn bump(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn eat(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Option<Token> {
        if let Some(tok) = self.eat(kind) {
            Some(tok)
        } else {
            let found = self.kind().describe();
            let span = self.peek().span;
            self.error(format!("expected {}, found {found}", kind.describe()), span);
            None
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            pos: self.pos,
            diag_len: self.diagnostics.len(),
        }
    }

    fn rollback(&mut self, snap: Snapshot) {
        self.pos = snap.pos;
        self.diagnostics.truncate(snap.diag_len);
    }

    /// True when the current token may continue an expression (it is not
    /// preceded by a newline, or we are inside a group).
    fn same_line(&self) -> bool {
        self.group_depth > 0 || !self.peek().newline_before
    }

    fn ident(&mut self) -> Option<Ident> {
        match self.kind().clone() {
            TokenKind::Ident(name) => {
                let tok = self.bump();
                Some(Ident {
                    name,
                    span: tok.span,
                })
            }
            _ => {
                let found = self.kind().describe();
                let span = self.peek().span;
                self.error(format!("expected identifier, found {found}"), span);
                None
            }
        }
    }

    fn at_ident(&self) -> bool {
        matches!(self.kind(), TokenKind::Ident(_))
    }

    // --- Module / items ---

    pub fn parse_module(&mut self) -> Module {
        let mut items = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.recover_to_item_start(),
            }
            if self.pos == before {
                // Safety net: never loop without progress.
                self.bump();
            }
        }
        Module { items }
    }

    /// Skips tokens until something that can plausibly start a top-level item.
    fn recover_to_item_start(&mut self) {
        let mut depth = 0u32;
        while !self.at_eof() {
            match self.kind() {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth == 0 {
                        self.bump();
                        return;
                    }
                    depth -= 1;
                }
                TokenKind::KwFn
                | TokenKind::KwStruct
                | TokenKind::KwQualifier
                | TokenKind::KwEffect
                | TokenKind::KwHandler
                | TokenKind::KwType
                | TokenKind::KwInternal
                | TokenKind::KwExternal
                | TokenKind::KwDefine
                | TokenKind::KwImport
                    if depth == 0 =>
                {
                    return;
                }
                _ => {}
            }
            self.bump();
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        match self.kind() {
            TokenKind::KwImport => self.parse_import().map(Item::Import),
            TokenKind::KwInternal | TokenKind::KwExternal => {
                let backing = if matches!(self.kind(), TokenKind::KwInternal) {
                    BackingMod::Internal
                } else {
                    BackingMod::External
                };
                self.bump();
                match self.kind() {
                    TokenKind::KwType => self.parse_type_decl(Some(backing)).map(Item::Type),
                    TokenKind::KwFn => self.parse_fn(Some(backing)).map(Item::Fn),
                    TokenKind::KwQualifier => {
                        self.parse_qualifier(Some(backing)).map(Item::Qualifier)
                    }
                    TokenKind::KwHandler => self.parse_handler(Some(backing)).map(Item::Handler),
                    _ => {
                        let found = self.kind().describe();
                        let span = self.peek().span;
                        self.error(
                            format!(
                                "expected `type`, `fn`, `qualifier`, or `handler` after \
                                 backing modifier, found {found}"
                            ),
                            span,
                        );
                        None
                    }
                }
            }
            TokenKind::KwType => self.parse_type_decl(None).map(Item::Type),
            TokenKind::KwStruct => self.parse_struct().map(Item::Struct),
            TokenKind::KwQualifier => self.parse_qualifier(None).map(Item::Qualifier),
            TokenKind::KwEffect => self.parse_effect().map(Item::Effect),
            TokenKind::KwHandler => self.parse_handler(None).map(Item::Handler),
            TokenKind::KwFn => self.parse_fn(None).map(Item::Fn),
            TokenKind::KwDefine => self.parse_define(),
            _ => {
                let found = self.kind().describe();
                let span = self.peek().span;
                self.error(format!("expected item, found {found}"), span);
                None
            }
        }
    }

    fn parse_import(&mut self) -> Option<ImportDecl> {
        let start = self.expect(&TokenKind::KwImport)?.span;
        let mut path = vec![self.ident()?];
        while self.eat(&TokenKind::Dot).is_some() {
            path.push(self.ident()?);
        }
        let alias = if self.at(&TokenKind::KwAs) && self.same_line() {
            self.bump();
            Some(self.ident()?)
        } else {
            None
        };
        let end = alias
            .as_ref()
            .map(|a| a.span)
            .unwrap_or_else(|| path.last().unwrap().span);
        Some(ImportDecl {
            path,
            alias,
            span: start.to(end),
        })
    }

    fn parse_type_decl(&mut self, backing: Option<BackingMod>) -> Option<TypeDecl> {
        let start = self.expect(&TokenKind::KwType)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        let alias = if self.eat(&TokenKind::Eq).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let end = alias
            .as_ref()
            .map(|t| t.span())
            .unwrap_or(name.span);
        Some(TypeDecl {
            backing,
            name,
            generics,
            alias,
            span: start.to(end),
        })
    }

    /// `<A, B, C>` — declaration-site generic parameters.
    fn parse_generics(&mut self) -> Vec<Ident> {
        let mut generics = Vec::new();
        if self.at(&TokenKind::Lt) {
            self.group_depth += 1;
            self.bump();
            loop {
                if self.eat(&TokenKind::Gt).is_some() || self.at_eof() {
                    break;
                }
                match self.ident() {
                    Some(id) => generics.push(id),
                    None => {
                        self.bump();
                        continue;
                    }
                }
                if self.eat(&TokenKind::Comma).is_none() {
                    if self.expect(&TokenKind::Gt).is_none() {
                        break;
                    }
                    break;
                }
            }
            self.group_depth -= 1;
        }
        generics
    }

    fn parse_struct(&mut self) -> Option<StructDecl> {
        let start = self.expect(&TokenKind::KwStruct)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        let mut auto_qualifiers = Vec::new();
        if self.eat(&TokenKind::KwWith).is_some() {
            loop {
                auto_qualifiers.push(self.parse_type_ref()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            fields.push(self.parse_field_decl()?);
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(StructDecl {
            name,
            generics,
            auto_qualifiers,
            fields,
            span: start.to(end),
        })
    }

    /// `name: Type (= default)?`
    fn parse_field_decl(&mut self) -> Option<FieldDecl> {
        let name = self.ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let default = if self.eat(&TokenKind::Eq).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let end = default
            .as_ref()
            .map(|e| e.span())
            .unwrap_or_else(|| ty.span());
        let span = name.span.to(end);
        Some(FieldDecl {
            name,
            ty,
            default,
            span,
        })
    }

    fn parse_qualifier(&mut self, backing: Option<BackingMod>) -> Option<QualifierDecl> {
        let start = self.expect(&TokenKind::KwQualifier)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        self.expect(&TokenKind::KwOf)?;
        let of = self.parse_type()?;
        let mut with = Vec::new();
        if self.eat(&TokenKind::KwWith).is_some() {
            loop {
                with.push(self.parse_type_ref()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        let mut field_overrides = Vec::new();
        let mut fns = Vec::new();
        let mut has_body = false;
        let mut end = of.span();
        if self.at(&TokenKind::LBrace) && self.same_line() {
            has_body = true;
            self.bump();
            while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                if self.at(&TokenKind::KwFn) {
                    fns.push(self.parse_fn(None)?);
                } else {
                    field_overrides.push(self.parse_field_decl()?);
                    self.eat(&TokenKind::Comma);
                }
            }
            end = self.expect(&TokenKind::RBrace)?.span;
        }
        Some(QualifierDecl {
            backing,
            name,
            generics,
            of,
            with,
            field_overrides,
            fns,
            has_body,
            span: start.to(end),
        })
    }

    fn parse_effect(&mut self) -> Option<EffectDecl> {
        let start = self.expect(&TokenKind::KwEffect)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        self.expect(&TokenKind::LBrace)?;
        let mut fns = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            fns.push(self.parse_fn(None)?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(EffectDecl {
            name,
            generics,
            fns,
            span: start.to(end),
        })
    }

    fn parse_handler(&mut self, backing: Option<BackingMod>) -> Option<HandlerDecl> {
        let start = self.expect(&TokenKind::KwHandler)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        let mut params = Vec::new();
        if self.at(&TokenKind::LParen) {
            params = self.parse_params()?;
        }
        self.expect(&TokenKind::KwOf)?;
        let of = self.parse_type()?;
        let mut state = Vec::new();
        let mut fns = Vec::new();
        let mut end = of.span();
        if self.at(&TokenKind::LBrace) && self.same_line() {
            self.bump();
            while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                if self.at(&TokenKind::KwFn) {
                    fns.push(self.parse_fn(None)?);
                } else {
                    state.push(self.parse_field_decl()?);
                    self.eat(&TokenKind::Comma);
                }
            }
            end = self.expect(&TokenKind::RBrace)?.span;
        }
        Some(HandlerDecl {
            backing,
            name,
            generics,
            params,
            of,
            state,
            fns,
            span: start.to(end),
        })
    }

    // --- Functions ---

    fn parse_fn(&mut self, backing: Option<BackingMod>) -> Option<FnDecl> {
        let start = self.expect(&TokenKind::KwFn)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        let params = self.parse_params()?;

        // Effects: `[Random<Int>, Console, use]`
        let effects = if self.at(&TokenKind::LBracket) && self.same_line() {
            Some(self.parse_effect_list()?)
        } else {
            None
        };

        // `-> [deductions] return_type (as Qualifier)?`
        let mut deductions = None;
        let mut return_type = None;
        let mut constructs = None;
        if self.at(&TokenKind::Arrow) && self.same_line() {
            self.bump();
            if self.at(&TokenKind::LBracket) {
                deductions = Some(self.parse_deduction_list()?);
            }
            return_type = Some(self.parse_type()?);
            // `-> T as Qualifier` marks a constructive-qualifier constructor.
            if self.at(&TokenKind::KwAs) && self.same_line() {
                self.bump();
                constructs = Some(self.parse_type_ref()?);
            }
        }

        let body = if self.at(&TokenKind::LBrace) && self.same_line() {
            Some(self.parse_block()?)
        } else {
            None
        };

        let end = body
            .as_ref()
            .map(|b| b.span)
            .or_else(|| constructs.as_ref().map(|c| c.span))
            .or_else(|| return_type.as_ref().map(|t| t.span()))
            .unwrap_or(name.span);
        Some(FnDecl {
            backing,
            name,
            generics,
            params,
            effects,
            deductions,
            return_type,
            constructs,
            body,
            span: start.to(end),
        })
    }

    fn parse_params(&mut self) -> Option<Vec<Param>> {
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut params = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let variadic = self.eat(&TokenKind::Ellipsis).is_some();
            let Some(name) = self.ident() else {
                self.group_depth -= 1;
                return None;
            };
            if self.expect(&TokenKind::Colon).is_none() {
                self.group_depth -= 1;
                return None;
            }
            let Some(ty) = self.parse_type() else {
                self.group_depth -= 1;
                return None;
            };
            let span = name.span.to(ty.span());
            params.push(Param {
                name,
                ty,
                variadic,
                span,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        self.expect(&TokenKind::RParen)?;
        Some(params)
    }

    fn parse_effect_list(&mut self) -> Option<Vec<EffectRef>> {
        self.expect(&TokenKind::LBracket)?;
        self.group_depth += 1;
        let mut effects = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            match self.kind() {
                TokenKind::KwUse => {
                    let tok = self.bump();
                    effects.push(EffectRef::Use(tok.span));
                }
                _ => match self.parse_type_ref() {
                    Some(r) => effects.push(EffectRef::Effect(r)),
                    None => {
                        self.group_depth -= 1;
                        return None;
                    }
                },
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        self.expect(&TokenKind::RBracket)?;
        Some(effects)
    }

    /// `[person]`, `[list: Mut]`, `[]`
    fn parse_deduction_list(&mut self) -> Option<Vec<Deduction>> {
        self.expect(&TokenKind::LBracket)?;
        self.group_depth += 1;
        let mut deductions = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            let Some(param) = self.ident() else {
                self.group_depth -= 1;
                return None;
            };
            let mut qualifiers = Vec::new();
            if self.eat(&TokenKind::Colon).is_some() {
                // Space-separated qualifier list: `list: Mut NonEmpty`
                while self.at_ident() {
                    match self.parse_type_ref() {
                        Some(r) => qualifiers.push(r),
                        None => break,
                    }
                }
            }
            let end = qualifiers.last().map(|q| q.span).unwrap_or(param.span);
            let span = param.span.to(end);
            deductions.push(Deduction {
                param,
                qualifiers,
                span,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        self.expect(&TokenKind::RBracket)?;
        Some(deductions)
    }

    // --- define templates ---

    fn parse_define(&mut self) -> Option<Item> {
        let start = self.expect(&TokenKind::KwDefine)?.span;
        match self.kind() {
            TokenKind::KwFn => {
                let df = self.parse_define_fn(start)?;
                Some(Item::DefineFn(df))
            }
            TokenKind::KwType => {
                self.bump();
                let name = self.ident()?;
                let generics = self.parse_generics();
                let (body, end) = self.parse_define_body()?;
                Some(Item::DefineType(DefineType {
                    name,
                    generics,
                    body,
                    span: start.to(end),
                }))
            }
            TokenKind::KwHandler => {
                self.bump();
                let name = self.ident()?;
                let generics = self.parse_generics();
                self.expect(&TokenKind::KwOf)?;
                let of = self.parse_type()?;
                self.expect(&TokenKind::LBrace)?;
                let mut fns = Vec::new();
                while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                    let fn_start = self.expect(&TokenKind::KwDefine)?.span;
                    fns.push(self.parse_define_fn(fn_start)?);
                }
                let end = self.expect(&TokenKind::RBrace)?.span;
                Some(Item::DefineHandler(DefineHandler {
                    name,
                    generics,
                    of,
                    fns,
                    span: start.to(end),
                }))
            }
            _ => {
                let found = self.kind().describe();
                let span = self.peek().span;
                self.error(
                    format!("expected `fn`, `type`, or `handler` after `define`, found {found}"),
                    span,
                );
                None
            }
        }
    }

    fn parse_define_fn(&mut self, start: Span) -> Option<DefineFn> {
        let sig = self.parse_fn_signature_only()?;
        let (body, end) = self.parse_define_body()?;
        Some(DefineFn {
            sig,
            body,
            span: start.to(end),
        })
    }

    /// Parses a fn signature without a body (for `define fn`).
    fn parse_fn_signature_only(&mut self) -> Option<FnDecl> {
        let start = self.expect(&TokenKind::KwFn)?.span;
        let name = self.ident()?;
        let generics = self.parse_generics();
        let params = self.parse_params()?;
        let effects = if self.at(&TokenKind::LBracket) && self.same_line() {
            Some(self.parse_effect_list()?)
        } else {
            None
        };
        let mut deductions = None;
        let mut return_type = None;
        if self.at(&TokenKind::Arrow) && self.same_line() {
            self.bump();
            if self.at(&TokenKind::LBracket) {
                deductions = Some(self.parse_deduction_list()?);
            }
            return_type = Some(self.parse_type()?);
        }
        let end = return_type.as_ref().map(|t| t.span()).unwrap_or(name.span);
        Some(FnDecl {
            backing: None,
            name,
            generics,
            params,
            effects,
            deductions,
            return_type,
            constructs: None,
            body: None,
            span: start.to(end),
        })
    }

    /// `{ imports: `` ... `` inline: `` ... `` }`
    fn parse_define_body(&mut self) -> Option<(DefineBody, Span)> {
        self.expect(&TokenKind::LBrace)?;
        let mut body = DefineBody::default();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let section = self.ident()?;
            self.expect(&TokenKind::Colon)?;
            let template = self.parse_template()?;
            match section.name.as_str() {
                "imports" => {
                    if body.imports.replace(template).is_some() {
                        self.error("duplicate `imports:` section", section.span);
                    }
                }
                "inline" => {
                    if body.inline.replace(template).is_some() {
                        self.error("duplicate `inline:` section", section.span);
                    }
                }
                other => {
                    self.error(
                        format!("unknown define section `{other}` (expected `imports` or `inline`)"),
                        section.span,
                    );
                }
            }
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some((body, end))
    }

    fn parse_template(&mut self) -> Option<Template> {
        let TokenKind::Template(content) = self.kind().clone() else {
            let found = self.kind().describe();
            let span = self.peek().span;
            self.error(format!("expected `` template, found {found}"), span);
            return None;
        };
        let tok = self.bump();
        let parts = parse_template_parts(&content, tok.span);
        Some(Template {
            parts,
            span: tok.span,
        })
    }

    // --- Types ---

    pub fn parse_type(&mut self) -> Option<Type> {
        let first = self.parse_type_postfix()?;
        if !self.at(&TokenKind::Pipe) {
            return Some(first);
        }
        let mut arms = vec![first];
        while self.eat(&TokenKind::Pipe).is_some() {
            arms.push(self.parse_type_postfix()?);
        }
        let span = arms.first().unwrap().span().to(arms.last().unwrap().span());
        Some(Type::Union { arms, span })
    }

    /// A type atom followed by `[]` / `?` postfixes.
    fn parse_type_postfix(&mut self) -> Option<Type> {
        let mut ty = self.parse_type_atom()?;
        loop {
            if self.at(&TokenKind::LBracket)
                && matches!(self.peek_at(1).kind, TokenKind::RBracket)
            {
                self.bump();
                let end = self.bump().span;
                let span = ty.span().to(end);
                ty = Type::Array {
                    elem: Box::new(ty),
                    span,
                };
            } else if self.at(&TokenKind::Question) {
                let end = self.bump().span;
                let span = ty.span().to(end);
                ty = Type::Nullable {
                    inner: Box::new(ty),
                    span,
                };
            } else {
                break;
            }
        }
        Some(ty)
    }

    fn parse_type_atom(&mut self) -> Option<Type> {
        if self.at(&TokenKind::LParen) {
            return self.parse_paren_type();
        }
        // A sequence of type refs: qualifiers followed by a base type.
        // Only continue the sequence on the same line (or inside a group) so
        // a type at the end of a line never swallows the next line.
        let mut refs = vec![self.parse_type_ref()?];
        while self.at_ident() && self.same_line() {
            refs.push(self.parse_type_ref()?);
        }
        // Qualifiers applied to a parenthesized type: `Ok (Ok Str | Err Int)`.
        if self.at(&TokenKind::LParen) && self.same_line() {
            let base = self.parse_paren_type()?;
            let start = refs.first().unwrap().span;
            let span = start.to(base.span());
            return Some(Type::QualifiedGroup {
                qualifiers: refs,
                base: Box::new(base),
                span,
            });
        }
        let base = refs.pop().unwrap();
        Some(Type::Named {
            qualifiers: refs,
            base,
        })
    }

    /// `(A, B) -> R`, `(A) [E] -> R`, `(A, B, C)` tuple, or `(T)` grouping.
    fn parse_paren_type(&mut self) -> Option<Type> {
        let start = self.expect(&TokenKind::LParen)?.span;
        self.group_depth += 1;
        let mut elems = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(ty) = self.parse_type() else {
                self.group_depth -= 1;
                return None;
            };
            elems.push(ty);
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RParen)?.span;

        let effects = if self.at(&TokenKind::LBracket) && self.same_line() {
            Some(self.parse_effect_list()?)
        } else {
            None
        };
        if self.at(&TokenKind::Arrow) && self.same_line() {
            self.bump();
            let ret = self.parse_type()?;
            let span = start.to(ret.span());
            return Some(Type::Fn {
                params: elems,
                effects,
                ret: Box::new(ret),
                span,
            });
        }
        if let Some(effects) = effects {
            // `[...]` after a paren type only makes sense for fn types.
            let _ = effects;
            self.error("expected `->` after effect list in function type", start);
        }
        if elems.len() == 1 {
            return Some(elems.into_iter().next().unwrap());
        }
        let span = start.to(end);
        Some(Type::Tuple { elems, span })
    }

    /// `Name` or `Name<T, U>`.
    fn parse_type_ref(&mut self) -> Option<TypeRef> {
        let name = self.ident()?;
        let mut args = Vec::new();
        let mut end = name.span;
        if self.at(&TokenKind::Lt) {
            self.group_depth += 1;
            self.bump();
            loop {
                if self.at(&TokenKind::Gt) || self.at_eof() {
                    break;
                }
                let Some(ty) = self.parse_type() else {
                    self.group_depth -= 1;
                    return None;
                };
                args.push(ty);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.group_depth -= 1;
            end = self.expect(&TokenKind::Gt)?.span;
        }
        let span = name.span.to(end);
        Some(TypeRef { name, args, span })
    }

    // --- Blocks and statements ---

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.expect(&TokenKind::LBrace)?.span;
        // Blocks reset grouping: statements inside are newline-terminated
        // even when the block itself appears inside parentheses.
        let saved_depth = std::mem::replace(&mut self.group_depth, 0);
        let saved_no_struct = std::mem::replace(&mut self.no_struct, false);
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            match self.parse_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.recover_in_block(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.group_depth = saved_depth;
        self.no_struct = saved_no_struct;
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Block {
            stmts,
            span: start.to(end),
        })
    }

    /// Skips to the start of the next statement (a token on a new line) or
    /// the end of the block.
    fn recover_in_block(&mut self) {
        let mut depth = 0u32;
        while !self.at_eof() {
            match self.kind() {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                }
                _ if depth == 0 && self.peek().newline_before => return,
                _ => {}
            }
            self.bump();
        }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        match self.kind() {
            TokenKind::KwLet => self.parse_let(),
            TokenKind::KwReturn => {
                let start = self.bump().span;
                let value = if self.stmt_value_follows() {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                let end = value.as_ref().map(|e| e.span()).unwrap_or(start);
                Some(Stmt::Return {
                    value,
                    span: start.to(end),
                })
            }
            TokenKind::KwBreak => {
                let start = self.bump().span;
                let value = if self.stmt_value_follows() {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                let end = value.as_ref().map(|e| e.span()).unwrap_or(start);
                Some(Stmt::Break {
                    value,
                    span: start.to(end),
                })
            }
            TokenKind::KwContinue => {
                let span = self.bump().span;
                Some(Stmt::Continue { span })
            }
            TokenKind::KwYield => {
                let start = self.bump().span;
                let value = self.parse_expr()?;
                let span = start.to(value.span());
                Some(Stmt::Yield { value, span })
            }
            TokenKind::KwUse => {
                let start = self.bump().span;
                let handler = self.parse_expr()?;
                let span = start.to(handler.span());
                Some(Stmt::Use { handler, span })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.at(&TokenKind::Eq) && self.same_line() {
                    self.bump();
                    let value = self.parse_expr()?;
                    let span = expr.span().to(value.span());
                    return Some(Stmt::Assign {
                        target: expr,
                        value,
                        span,
                    });
                }
                Some(Stmt::Expr(expr))
            }
        }
    }

    /// True when a `return`/`break` is followed by a value expression on the
    /// same line.
    fn stmt_value_follows(&self) -> bool {
        if self.peek().newline_before {
            return false;
        }
        !matches!(self.kind(), TokenKind::RBrace | TokenKind::Eof)
    }

    fn parse_let(&mut self) -> Option<Stmt> {
        let start = self.expect(&TokenKind::KwLet)?.span;
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(&TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.to(value.span());
        Some(Stmt::Let {
            pattern,
            ty,
            value,
            span,
        })
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        match self.kind() {
            TokenKind::LParen => {
                let start = self.bump().span;
                self.group_depth += 1;
                let mut elems = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at_eof() {
                    let Some(pat) = self.parse_pattern() else {
                        self.group_depth -= 1;
                        return None;
                    };
                    elems.push(pat);
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                }
                self.group_depth -= 1;
                let end = self.expect(&TokenKind::RParen)?.span;
                Some(Pattern::Tuple {
                    elems,
                    span: start.to(end),
                })
            }
            TokenKind::LBrace => {
                let start = self.bump().span;
                self.group_depth += 1;
                let mut fields = Vec::new();
                while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                    let Some(field) = self.ident() else {
                        self.group_depth -= 1;
                        return None;
                    };
                    let binding = if self.eat(&TokenKind::Colon).is_some() {
                        let Some(b) = self.ident() else {
                            self.group_depth -= 1;
                            return None;
                        };
                        b
                    } else {
                        field.clone()
                    };
                    let span = field.span.to(binding.span);
                    fields.push(StructPatternField {
                        field,
                        binding,
                        span,
                    });
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                }
                self.group_depth -= 1;
                let end = self.expect(&TokenKind::RBrace)?.span;
                Some(Pattern::Struct {
                    fields,
                    span: start.to(end),
                })
            }
            _ => self.ident().map(Pattern::Ident),
        }
    }

    // --- Expressions ---

    pub fn parse_expr(&mut self) -> Option<Expr> {
        // Lambda shorthand: `i -> ...`
        if self.at_ident() && matches!(self.peek_at(1).kind, TokenKind::Arrow) {
            return self.parse_lambda_from_ident();
        }
        // Lambda with parenthesized params: `(a, b) -> ...`
        if self.at(&TokenKind::LParen) {
            if let Some(lambda) = self.try_parse_paren_lambda() {
                return Some(lambda);
            }
        }
        self.parse_or()
    }

    fn parse_lambda_from_ident(&mut self) -> Option<Expr> {
        let name = self.ident()?;
        let start = name.span;
        self.expect(&TokenKind::Arrow)?;
        let param = LambdaParam {
            span: name.span,
            name,
            ty: None,
        };
        let body = self.parse_lambda_body()?;
        let end = match &body {
            LambdaBody::Expr(e) => e.span(),
            LambdaBody::Block(b) => b.span,
        };
        Some(Expr::Lambda {
            params: vec![param],
            body,
            span: start.to(end),
        })
    }

    /// Attempts `(a, b) -> ...`; rolls back and returns `None` when the
    /// parens are not a lambda parameter list.
    fn try_parse_paren_lambda(&mut self) -> Option<Expr> {
        let snap = self.snapshot();
        let start = self.bump().span; // (
        self.group_depth += 1;
        let mut params = Vec::new();
        let mut ok = true;
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(name) = (if self.at_ident() { self.ident() } else { None }) else {
                ok = false;
                break;
            };
            let ty = if self.eat(&TokenKind::Colon).is_some() {
                match self.parse_type() {
                    Some(t) => Some(t),
                    None => {
                        ok = false;
                        break;
                    }
                }
            } else {
                None
            };
            let span = name.span;
            params.push(LambdaParam { name, ty, span });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        if !ok || self.eat(&TokenKind::RParen).is_none() || !self.at(&TokenKind::Arrow) {
            self.rollback(snap);
            return None;
        }
        self.bump(); // ->
        let body = match self.parse_lambda_body() {
            Some(b) => b,
            None => {
                self.rollback(snap);
                return None;
            }
        };
        let end = match &body {
            LambdaBody::Expr(e) => e.span(),
            LambdaBody::Block(b) => b.span,
        };
        Some(Expr::Lambda {
            params,
            body,
            span: start.to(end),
        })
    }

    fn parse_lambda_body(&mut self) -> Option<LambdaBody> {
        if self.at(&TokenKind::LBrace) {
            Some(LambdaBody::Block(self.parse_block()?))
        } else {
            Some(LambdaBody::Expr(Box::new(self.parse_expr()?)))
        }
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_and()?;
        while self.at(&TokenKind::PipePipe) && self.same_line() {
            self.bump();
            let rhs = self.parse_and()?;
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_equality()?;
        while self.at(&TokenKind::AmpAmp) && self.same_line() {
            self.bump();
            let rhs = self.parse_equality()?;
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_comparison()?;
        loop {
            let op = match self.kind() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::NotEq,
                _ => break,
            };
            if !self.same_line() {
                break;
            }
            self.bump();
            let rhs = self.parse_comparison()?;
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    /// Comparisons plus the `is` checks (same precedence tier).
    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_additive()?;
        loop {
            match self.kind() {
                TokenKind::KwIs if self.same_line() => {
                    self.bump();
                    let (check, binding) = self.parse_is_check()?;
                    let end = binding
                        .as_ref()
                        .map(|b| b.span)
                        .or_else(|| check.last().map(|r| r.span))
                        .unwrap_or_else(|| lhs.span());
                    let span = lhs.span().to(end);
                    lhs = Expr::Is {
                        subject: Box::new(lhs),
                        check,
                        binding,
                        span,
                    };
                }
                TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq
                    if self.same_line() =>
                {
                    let op = match self.kind() {
                        TokenKind::Lt => BinaryOp::Lt,
                        TokenKind::Gt => BinaryOp::Gt,
                        TokenKind::LtEq => BinaryOp::LtEq,
                        _ => BinaryOp::GtEq,
                    };
                    self.bump();
                    let rhs = self.parse_additive()?;
                    let span = lhs.span().to(rhs.span());
                    lhs = Expr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    };
                }
                _ => break,
            }
        }
        Some(lhs)
    }

    /// The type-ref sequence after `is`, with an optional trailing binding.
    /// Type names are uppercase by convention; a trailing lowercase
    /// identifier is a binding: `is Str s`, `is Err Str`, `is NonEmpty`.
    fn parse_is_check(&mut self) -> Option<(Vec<TypeRef>, Option<Ident>)> {
        let mut refs = Vec::new();
        let mut binding = None;
        loop {
            if !self.same_line() {
                break;
            }
            let TokenKind::Ident(name) = self.kind() else {
                break;
            };
            let lowercase = name.chars().next().is_some_and(|c| c.is_lowercase());
            if lowercase {
                if refs.is_empty() {
                    // Something like `is x` — still record it as a check so
                    // later phases can report a proper error.
                    refs.push(self.parse_type_ref()?);
                } else {
                    binding = self.ident();
                }
                break;
            }
            refs.push(self.parse_type_ref()?);
        }
        if refs.is_empty() {
            let span = self.peek().span;
            let found = self.kind().describe();
            self.error(format!("expected type after `is`, found {found}"), span);
            return None;
        }
        Some((refs, binding))
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            if !self.same_line() {
                break;
            }
            self.bump();
            let rhs = self.parse_multiplicative()?;
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => break,
            };
            if !self.same_line() {
                break;
            }
            self.bump();
            let rhs = self.parse_unary()?;
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        match self.kind() {
            TokenKind::Minus => {
                let start = self.bump().span;
                let operand = self.parse_unary()?;
                let span = start.to(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Bang => {
                let start = self.bump().span;
                let operand = self.parse_unary()?;
                let span = start.to(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Ellipsis => {
                let start = self.bump().span;
                let operand = self.parse_unary()?;
                let span = start.to(operand.span());
                Some(Expr::Spread {
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.kind() {
                TokenKind::Dot => {
                    self.bump();
                    let field = self.ident()?;
                    let span = expr.span().to(field.span);
                    expr = Expr::Field {
                        base: Box::new(expr),
                        field,
                        span,
                    };
                }
                TokenKind::LParen if self.same_line() => {
                    let args = self.parse_call_args()?;
                    let span = expr.span().to(args.1);
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        type_args: Vec::new(),
                        args: args.0,
                        span,
                    };
                }
                TokenKind::Lt if self.same_line() => {
                    // Possibly explicit generic args on a call:
                    // `next_random<Int>()`. Speculative — falls back to a
                    // comparison operator.
                    match self.try_parse_generic_call(&expr) {
                        Some(call) => expr = call,
                        None => break,
                    }
                }
                TokenKind::LBracket if self.same_line() => {
                    let start = self.bump().span;
                    self.group_depth += 1;
                    let index = self.parse_expr();
                    self.group_depth -= 1;
                    let index = index?;
                    let end = self.expect(&TokenKind::RBracket)?.span;
                    let span = expr.span().to(end);
                    let _ = start;
                    expr = Expr::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                    // `Int[5] { i -> ... }` — sized array initialization.
                    if !self.no_struct && self.at(&TokenKind::LBrace) && self.same_line() {
                        if let Expr::Index { base, index, span } = expr {
                            if let Expr::Ident(name) = base.as_ref() {
                                let snap = self.snapshot();
                                match self.parse_brace_lambda() {
                                    Some(init) => {
                                        let full = span.to(init.span());
                                        expr = Expr::ArrayInit {
                                            elem_type: TypeRef {
                                                name: name.clone(),
                                                args: Vec::new(),
                                                span: name.span,
                                            },
                                            size: index,
                                            init: Box::new(init),
                                            span: full,
                                        };
                                        continue;
                                    }
                                    None => {
                                        self.rollback(snap);
                                        expr = Expr::Index { base, index, span };
                                    }
                                }
                            } else {
                                expr = Expr::Index { base, index, span };
                            }
                        }
                    }
                }
                TokenKind::Bang if self.same_line() => {
                    let end = self.bump().span;
                    let span = expr.span().to(end);
                    expr = Expr::NonNull {
                        operand: Box::new(expr),
                        span,
                    };
                }
                TokenKind::PlusPlus if self.same_line() => {
                    let end = self.bump().span;
                    let span = expr.span().to(end);
                    expr = Expr::PostIncrement {
                        operand: Box::new(expr),
                        span,
                    };
                }
                _ => break,
            }
        }
        Some(expr)
    }

    /// Parses `(arg, arg, ...)`; returns the args and the closing-paren span.
    fn parse_call_args(&mut self) -> Option<(Vec<Expr>, Span)> {
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut args = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(arg) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            args.push(arg);
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RParen)?.span;
        Some((args, end))
    }

    /// Speculatively parses `<T, U>(args)` as a generic call.
    fn try_parse_generic_call(&mut self, callee: &Expr) -> Option<Expr> {
        let snap = self.snapshot();
        self.group_depth += 1;
        self.bump(); // <
        let mut type_args = Vec::new();
        loop {
            if self.at(&TokenKind::Gt) {
                break;
            }
            match self.parse_type() {
                Some(ty) => type_args.push(ty),
                None => {
                    self.group_depth -= 1;
                    self.rollback(snap);
                    return None;
                }
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        if self.eat(&TokenKind::Gt).is_none() || !self.at(&TokenKind::LParen) {
            self.rollback(snap);
            return None;
        }
        let args = match self.parse_call_args() {
            Some(a) => a,
            None => {
                self.rollback(snap);
                return None;
            }
        };
        let span = callee.span().to(args.1);
        Some(Expr::Call {
            callee: Box::new(callee.clone()),
            type_args,
            args: args.0,
            span,
        })
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.kind().clone() {
            TokenKind::Int(value) => {
                let tok = self.bump();
                Some(Expr::Int {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::Float(value) => {
                let tok = self.bump();
                Some(Expr::Float {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::KwTrue => {
                let tok = self.bump();
                Some(Expr::Bool {
                    value: true,
                    span: tok.span,
                })
            }
            TokenKind::KwFalse => {
                let tok = self.bump();
                Some(Expr::Bool {
                    value: false,
                    span: tok.span,
                })
            }
            TokenKind::Char(value) => {
                let tok = self.bump();
                Some(Expr::Char {
                    value,
                    span: tok.span,
                })
            }
            TokenKind::Str(parts) => {
                let tok = self.bump();
                let parts = self.parse_str_parts(parts);
                Some(Expr::Str {
                    parts,
                    span: tok.span,
                })
            }
            TokenKind::Ident(_) => self.parse_ident_expr(),
            TokenKind::LParen => self.parse_paren_expr(),
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::LBrace => self.parse_brace_expr(),
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwWhen => self.parse_when(),
            TokenKind::KwWhile => self.parse_while(),
            TokenKind::KwFor => self.parse_for(),
            _ => {
                let found = self.kind().describe();
                let span = self.peek().span;
                self.error(format!("expected expression, found {found}"), span);
                None
            }
        }
    }

    /// An identifier — possibly the start of a struct literal
    /// (`Person { ... }`, `Mut Person { ... }`, `Pair<Int, Str> { ... }`).
    fn parse_ident_expr(&mut self) -> Option<Expr> {
        if !self.no_struct {
            let snap = self.snapshot();
            if let Some(ty) = self.parse_type_atom() {
                if self.at(&TokenKind::LBrace) && self.same_line() && self.brace_is_struct_lit() {
                    let is_plain_ident = matches!(
                        &ty,
                        Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty()
                    );
                    // Only treat it as a struct literal if the name looks
                    // like a type (uppercase), or it is qualified/generic.
                    let looks_like_type = match &ty {
                        Type::Named { base, .. } => base
                            .name
                            .name
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_uppercase()),
                        _ => false,
                    };
                    let _ = is_plain_ident;
                    if looks_like_type {
                        return self.parse_struct_lit_body(Some(ty));
                    }
                }
            }
            self.rollback(snap);
        }
        let ident = self.ident()?;
        Some(Expr::Ident(ident))
    }

    /// Lookahead: does the `{` ahead open a struct literal body
    /// (`}`, `ident:`, or `...`) rather than a block?
    fn brace_is_struct_lit(&self) -> bool {
        debug_assert!(self.at(&TokenKind::LBrace));
        match &self.peek_at(1).kind {
            TokenKind::RBrace | TokenKind::Ellipsis => true,
            TokenKind::Ident(_) => matches!(self.peek_at(2).kind, TokenKind::Colon),
            _ => false,
        }
    }

    /// Parses `{ field: expr, ...spread, field: expr }` given an optional
    /// preceding type.
    fn parse_struct_lit_body(&mut self, ty: Option<Type>) -> Option<Expr> {
        let start = ty
            .as_ref()
            .map(|t| t.span())
            .unwrap_or_else(|| self.peek().span);
        self.expect(&TokenKind::LBrace)?;
        self.group_depth += 1;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::Ellipsis) {
                let spread_start = self.bump().span;
                let Some(value) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                let span = spread_start.to(value.span());
                fields.push(StructLitField {
                    kind: StructLitFieldKind::Spread(value),
                    span,
                });
            } else {
                let Some(name) = self.ident() else {
                    self.group_depth -= 1;
                    return None;
                };
                if self.expect(&TokenKind::Colon).is_none() {
                    self.group_depth -= 1;
                    return None;
                }
                let Some(value) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                let span = name.span.to(value.span());
                fields.push(StructLitField {
                    kind: StructLitFieldKind::Named { name, value },
                    span,
                });
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Expr::StructLit {
            ty,
            fields,
            span: start.to(end),
        })
    }

    /// `(expr)` grouping or `(a, b, c)` tuple literal.
    fn parse_paren_expr(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::LParen)?.span;
        self.group_depth += 1;
        let mut elems = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(e) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            elems.push(e);
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RParen)?.span;
        match elems.len() {
            1 => Some(elems.into_iter().next().unwrap()),
            _ => Some(Expr::Tuple {
                elems,
                span: start.to(end),
            }),
        }
    }

    /// `[1, 2, 3]`
    fn parse_array_literal(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::LBracket)?.span;
        self.group_depth += 1;
        let mut elems = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            let Some(e) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            elems.push(e);
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RBracket)?.span;
        Some(Expr::ArrayLit {
            elems,
            span: start.to(end),
        })
    }

    /// A `{` in expression position: either a bare struct literal
    /// (`{name: ..., age: ...}`) or a brace lambda (`{ i: Int -> 0 }`).
    fn parse_brace_expr(&mut self) -> Option<Expr> {
        if let Some(lambda) = {
            let snap = self.snapshot();
            match self.parse_brace_lambda() {
                Some(l) => Some(l),
                None => {
                    self.rollback(snap);
                    None
                }
            }
        } {
            return Some(lambda);
        }
        if self.brace_is_struct_lit() {
            return self.parse_struct_lit_body(None);
        }
        let span = self.peek().span;
        self.error("expected struct literal or lambda", span);
        None
    }

    /// `{ i: Int -> 0 }` — a Kotlin-style brace lambda. Returns `None`
    /// (without emitting diagnostics beyond rollback) if the contents do not
    /// form `params ->`.
    fn parse_brace_lambda(&mut self) -> Option<Expr> {
        let snap = self.snapshot();
        let start = match self.eat(&TokenKind::LBrace) {
            Some(t) => t.span,
            None => return None,
        };
        self.group_depth += 1;
        let mut params = Vec::new();
        loop {
            if !self.at_ident() {
                self.group_depth -= 1;
                self.rollback(snap);
                return None;
            }
            let name = self.ident()?;
            let ty = if self.eat(&TokenKind::Colon).is_some() {
                match self.parse_type() {
                    Some(t) => Some(t),
                    None => {
                        self.group_depth -= 1;
                        self.rollback(snap);
                        return None;
                    }
                }
            } else {
                None
            };
            let span = name.span;
            params.push(LambdaParam { name, ty, span });
            if self.eat(&TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        self.group_depth -= 1;
        if self.eat(&TokenKind::Arrow).is_none() {
            self.rollback(snap);
            return None;
        }
        // Body: statements until the closing brace.
        let saved_depth = std::mem::replace(&mut self.group_depth, 0);
        let saved_no_struct = std::mem::replace(&mut self.no_struct, false);
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            match self.parse_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.recover_in_block(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.group_depth = saved_depth;
        self.no_struct = saved_no_struct;
        let end = self.expect(&TokenKind::RBrace)?.span;
        let body_span = start.to(end);
        let body = LambdaBody::Block(Block {
            stmts,
            span: body_span,
        });
        Some(Expr::Lambda {
            params,
            body,
            span: start.to(end),
        })
    }

    fn parse_condition(&mut self) -> Option<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, true);
        let result = self.parse_expr();
        self.no_struct = saved;
        result
    }

    fn parse_if(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwIf)?.span;
        let mut branches = Vec::new();
        let cond = self.parse_condition()?;
        let block = self.parse_block()?;
        branches.push((cond, block));
        let mut else_block = None;
        let mut end = branches.last().unwrap().1.span;
        loop {
            if self.at(&TokenKind::KwElif) {
                self.bump();
                let cond = self.parse_condition()?;
                let block = self.parse_block()?;
                end = block.span;
                branches.push((cond, block));
            } else if self.at(&TokenKind::KwElse) {
                self.bump();
                let block = self.parse_block()?;
                end = block.span;
                else_block = Some(block);
                break;
            } else {
                break;
            }
        }
        Some(Expr::If {
            branches,
            else_block,
            span: start.to(end),
        })
    }

    fn parse_when(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwWhen)?.span;
        let subject = self.parse_condition()?;
        self.expect(&TokenKind::LBrace)?;
        let mut branches = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let is_span = self.expect(&TokenKind::KwIs)?.span;
            let (check, binding) = self.parse_is_check()?;
            let body = self.parse_block()?;
            let span = is_span.to(body.span);
            branches.push(WhenBranch {
                check,
                binding,
                body,
                span,
            });
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Expr::When {
            subject: Box::new(subject),
            branches,
            span: start.to(end),
        })
    }

    fn parse_while(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwWhile)?.span;
        let cond = self.parse_condition()?;
        let body = self.parse_block()?;
        let mut end = body.span;
        let else_block = if self.at(&TokenKind::KwElse) {
            self.bump();
            let b = self.parse_block()?;
            end = b.span;
            Some(b)
        } else {
            None
        };
        Some(Expr::While {
            cond: Box::new(cond),
            body,
            else_block,
            span: start.to(end),
        })
    }

    fn parse_for(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwFor)?.span;
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::KwIn)?;
        let iterable = self.parse_condition()?;
        let body = self.parse_block()?;
        let mut end = body.span;
        let else_block = if self.at(&TokenKind::KwElse) {
            self.bump();
            let b = self.parse_block()?;
            end = b.span;
            Some(b)
        } else {
            None
        };
        Some(Expr::For {
            pattern,
            iterable: Box::new(iterable),
            body,
            else_block,
            span: start.to(end),
        })
    }

    /// Converts lexer string parts into expression parts by parsing each
    /// `${...}` fragment.
    fn parse_str_parts(&mut self, parts: Vec<StrPart>) -> Vec<StrExprPart> {
        parts
            .into_iter()
            .map(|part| match part {
                StrPart::Text(text) => StrExprPart::Text(text),
                StrPart::Interp { source, offset } => {
                    let (expr, diags) = parse_interpolated_expr(&source, offset);
                    self.diagnostics.extend(diags);
                    StrExprPart::Interp(Box::new(expr))
                }
            })
            .collect()
    }
}

/// Parses a `${...}` fragment as an expression, shifting all spans by
/// `offset` so they point back into the original file.
fn parse_interpolated_expr(source: &str, offset: u32) -> (Expr, Vec<Diagnostic>) {
    let lexed = lexer::lex(source);
    let mut diagnostics = lexed.diagnostics;
    let tokens: Vec<Token> = lexed
        .tokens
        .into_iter()
        .map(|mut t| {
            t.span = Span::new(t.span.start + offset, t.span.end + offset);
            t
        })
        .collect();
    for d in &mut diagnostics {
        d.span = Span::new(d.span.start + offset, d.span.end + offset);
    }
    let mut parser = Parser::new(source, tokens);
    let expr = parser.parse_expr().unwrap_or(Expr::Error {
        span: Span::new(offset, offset + source.len() as u32),
    });
    if !parser.at_eof() {
        let span = parser.peek().span;
        let found = parser.kind().describe();
        parser.error(
            format!("unexpected {found} in string interpolation"),
            span,
        );
    }
    diagnostics.extend(parser.into_diagnostics());
    (expr, diagnostics)
}

/// Splits raw template text into literal text and `${name}` / `${...name}`
/// interpolations.
fn parse_template_parts(content: &str, span: Span) -> Vec<TemplatePart> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // {
            if !text.is_empty() {
                parts.push(TemplatePart::Text(std::mem::take(&mut text)));
            }
            let mut inner = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                inner.push(c);
            }
            let inner = inner.trim();
            let (variadic, name) = match inner.strip_prefix("...") {
                Some(rest) => (true, rest.trim().to_string()),
                None => (false, inner.to_string()),
            };
            let ident = Ident { name, span };
            parts.push(if variadic {
                TemplatePart::InterpVariadic(ident)
            } else {
                TemplatePart::Interp(ident)
            });
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        parts.push(TemplatePart::Text(text));
    }
    parts
}
