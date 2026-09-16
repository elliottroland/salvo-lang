//! Recursive-descent parser for Salvo.
//!
//! Design notes:
//! * Salvo has no statement terminators; statements end at newlines. Tokens
//!   carry a `newline_before` flag, and infix/postfix continuation across a
//!   newline is only allowed inside parentheses/brackets (`group_depth > 0`).
//! * Struct literals (`Person { ... }`) are ambiguous with blocks in
//!   condition position (`if x is Person { ... }`), so struct-literal
//!   speculation is disabled while parsing conditions (`no_struct`)
//!   [cond-bool].
//! * Type names and qualifiers are uppercase by convention; `is` checks use
//!   this to distinguish the checked type from an optional binding
//!   (`if x is Str s`) [is-binding].

use std::collections::HashMap;

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::lexer;
use crate::lexer::Comment;
use crate::span::Span;
use crate::token::{StrPart, Token, TokenKind};

pub struct Parser<'s> {
    #[allow(dead_code)]
    source: &'s str,
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    /// Own-line `//` comments by the line they sit on [doc-comment],
    /// used to attach the run above a declaration as its docs.
    comments: HashMap<u32, String>,
    /// Byte offset of the start of each line, for offset -> line lookup.
    line_starts: Vec<u32>,
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

/// [async-self-send] The contextual selector that names the enclosing handler:
/// `k@self(args)`. Contextual, not reserved — `self` stays an ordinary name
/// everywhere else, and it is only special immediately after `@`.
pub const SELF_SELECTOR: &str = "self";

/// [async-effect-kind] The contextual modifier that makes an effect a process
/// protocol: `async effect E { … }`. Contextual, not reserved — and the only
/// place the word appears in the language, since the phase decided against
/// colouring functions.
pub const ASYNC_MODIFIER: &str = "async";

impl<'s> Parser<'s> {
    pub fn new(source: &'s str, tokens: Vec<Token>, comments: Vec<Comment>) -> Self {
        let mut line_starts = vec![0u32];
        line_starts.extend(
            source
                .char_indices()
                .filter(|&(_, c)| c == '\n')
                .map(|(i, _)| i as u32 + 1),
        );
        let line_of = |offset: u32| match line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let comments = comments
            .into_iter()
            .filter(|c| c.own_line)
            .map(|c| (line_of(c.span.start) as u32, c.text))
            .collect();
        Parser {
            source,
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            comments,
            line_starts,
            group_depth: 0,
            no_struct: false,
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    // --- Doc comments ---

    /// The line a byte offset falls on (0-based).
    fn line_of(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line as u32,
            Err(next) => next as u32 - 1,
        }
    }

    /// The doc comment of the declaration starting at `offset`
    /// [doc-comment]: the maximal run of own-line `//` comments on the
    /// lines *immediately* above it. One blank line — or any code — ends
    /// the run, which is what makes an unrelated comment earlier in the
    /// file stay unrelated.
    fn docs_before(&self, offset: u32) -> Vec<String> {
        let mut line = self.line_of(offset);
        let mut docs = Vec::new();
        while line > 0 {
            line -= 1;
            match self.comments.get(&line) {
                Some(text) => docs.push(text.clone()),
                None => break,
            }
        }
        docs.reverse();
        docs
    }

    /// The docs of the declaration the parser is positioned at.
    fn docs_here(&self) -> Vec<String> {
        self.docs_before(self.peek().span.start)
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

    /// An identifier that must name a *type* — struct, qualifier, type,
    /// effect, handler, generic parameter — and therefore starts with an
    /// uppercase letter [name-casing].
    fn ident_type(&mut self, what: &str) -> Option<Ident> {
        let id = self.ident()?;
        if !id.name.starts_with(|c: char| c.is_uppercase()) {
            self.error(
                format!("{what} names must start with an uppercase letter [name-casing]"),
                id.span,
            );
        }
        Some(id)
    }

    /// An identifier that must name a *value* — fn, parameter, field,
    /// variable — and therefore does not start with an uppercase letter
    /// [name-casing]. Values must stay distinguishable from type paths:
    /// `Environment.Id { … }` is a struct literal, `person.name` a field
    /// read [name-dot].
    fn ident_value(&mut self, what: &str) -> Option<Ident> {
        let id = self.ident()?;
        if id.name.starts_with(|c: char| c.is_uppercase()) {
            self.error(
                format!("{what} names must not start with an uppercase letter [name-casing]"),
                id.span,
            );
        }
        Some(id)
    }

    /// The name of a struct or qualifier declaration: a plain `Name` or a
    /// dot-name `Ns.Name` [name-dot]. Both segments are type names, and
    /// the pair is kept as one dotted name — the key everything else uses.
    fn ident_decl_dotted(&mut self, what: &str) -> Option<Ident> {
        let head = self.ident_type(what)?;
        if !self.at(&TokenKind::Dot) || !self.same_line() {
            return Some(head);
        }
        if !matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            return Some(head);
        }
        self.bump();
        let tail = self.ident_type(what)?;
        let mut span = head.span.to(tail.span);
        // Names cap at two segments; consume any extra so the declaration
        // still parses [name-dot].
        while self.at(&TokenKind::Dot)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            self.bump();
            let extra = self.ident()?;
            span = span.to(extra.span);
            self.error(
                "a dot-name has exactly two segments (`Ns.Name`) [name-dot]",
                span,
            );
        }
        Some(Ident {
            name: format!("{}.{}", head.name, tail.name),
            span,
        })
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

    /// True when the current token is the *contextual* keyword `word` — an
    /// identifier the grammar reads as a keyword in one position without
    /// reserving it anywhere else (`send fn`, `spawn`, `capacity`, `on`).
    fn at_word(&self, word: &str) -> bool {
        matches!(self.kind(), TokenKind::Ident(name) if name == word)
    }

    /// Consumes the contextual keyword `word`, or reports `message` against
    /// the token that stands where it should have been.
    fn expect_word(&mut self, word: &str, message: &str) -> Option<Span> {
        if self.at_word(word) {
            return Some(self.bump().span);
        }
        let span = self.peek().span;
        self.error(message.to_string(), span);
        None
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
                | TokenKind::KwIntrinsic
                | TokenKind::KwPlatform
                | TokenKind::KwProvenance
                | TokenKind::KwRefn
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
            // [intrinsic-fn] [intrinsic-std-only] `intrinsic` marks a
            // declaration the *compiler* implements. It is the only backing
            // modifier there is, now that `external`/`define` are gone, and
            // only the standard library may write it — which the checker
            // enforces, since the parser does not know which file it is in.
            TokenKind::KwIntrinsic => {
                self.bump();
                match self.kind() {
                    TokenKind::KwType => self.parse_type_decl(true).map(Item::Type),
                    TokenKind::KwFn => self.parse_fn(true).map(Item::Fn),
                    TokenKind::KwQualifier => self
                        .parse_qualifier(true, QualSubject::State)
                        .map(Item::Qualifier),
                    TokenKind::KwHandler => self.parse_handler(true).map(Item::Handler),
                    _ => {
                        let found = self.kind().describe();
                        let span = self.peek().span;
                        self.error(
                            format!(
                                "expected `type`, `fn`, `qualifier`, or `handler` after \
                                 `intrinsic`, found {found}"
                            ),
                            span,
                        );
                        None
                    }
                }
            }
            TokenKind::KwType => {
                let t = self.parse_type_decl(false)?;
                // [decl-body] A bodiless `type` was only ever meaningful as
                // `external type`; with `external` gone there is nothing for
                // one to mean, so it is a parse error naming both forms that
                // do exist.
                if t.alias.is_none() {
                    self.error(
                        format!(
                            "`type {}` declares nothing: give it a definition \
                             (`type {} = ...`), or declare what the target \
                             language provides with a `platform effect`",
                            t.name.name, t.name.name
                        ),
                        t.span,
                    );
                }
                Some(Item::Type(t))
            }
            // [iter-fn] `iter fn next(c: Countdown) -> Emitted T | Finished` —
            // a hand-written `next` whose pass struct is generated, named after
            // the `iter` it generates.
            //
            // A **contextual** keyword, and it has to be: `iter` is the name of
            // the function this form generates, so it must stay callable and
            // declarable. At item level a bare identifier is otherwise a parse
            // error, which is what makes `iter fn` unambiguous with no lookahead
            // beyond the next token.
            TokenKind::Ident(name)
                if name == "iter" && matches!(self.peek_at(1).kind, TokenKind::KwFn) =>
            {
                self.bump();
                self.parse_fn_flavored(false, FnFlavor::Iter).map(Item::Fn)
            }
            TokenKind::KwStruct => self.parse_struct(false).map(Item::Struct),
            // [linear-group] [obligation-spelling] `linear struct X { … }`:
            // the exactly-once obligation as a declaration modifier.
            TokenKind::KwLinear if matches!(self.peek_at(1).kind, TokenKind::KwStruct) => {
                self.bump();
                self.parse_struct(true).map(Item::Struct)
            }
            // [linear-group] `linear intrinsic type Reply<T>` — a linear
            // **opaque** type, the same modifier on the other declaration
            // form that can carry an obligation (user decision 2026-09-15).
            // The token's representation is the backend's business, so it has
            // no fields to make a `linear struct` of.
            TokenKind::KwLinear if matches!(self.peek_at(1).kind, TokenKind::KwIntrinsic) => {
                self.bump();
                self.bump();
                if !self.at(&TokenKind::KwType) {
                    let found = self.kind().describe();
                    let span = self.peek().span;
                    self.error(
                        format!(
                            "`linear` before `intrinsic` declares an opaque linear \
                             type, so `type` must follow: `linear intrinsic type \
                             Reply<T>` (found {found})"
                        ),
                        span,
                    );
                    return None;
                }
                self.parse_type_decl_linear(true, true).map(Item::Type)
            }
            TokenKind::KwQualifier => self
                .parse_qualifier(false, QualSubject::State)
                .map(Item::Qualifier),
            // [qual-subject] `provenance qualifier Q of T`: a claim about
            // where the handle came from, not about its contents.
            TokenKind::KwProvenance => {
                self.bump();
                if !self.at(&TokenKind::KwQualifier) {
                    let found = self.kind().describe();
                    let span = self.peek().span;
                    self.error(
                        format!("expected `qualifier` after `provenance`, found {found}"),
                        span,
                    );
                    return None;
                }
                self.parse_qualifier(false, QualSubject::Provenance)
                    .map(Item::Qualifier)
            }
            // [platform-effect] `platform effect E { ... }`: the members are
            // implemented by the host in the target language. `effect` is
            // the only declaration this modifier takes — a platform *type*
            // and a platform *handler* are deferred, and the diagnostic
            // says so rather than reporting a bare parse error.
            TokenKind::KwPlatform => {
                self.bump();
                match self.kind() {
                    TokenKind::KwEffect => self.parse_effect(true).map(Item::Effect),
                    // [platform-handler] `platform handler HostRawFs of RawFs`:
                    // a host implementation of an *ordinary* Salvo effect,
                    // registered with `use` like any handler.
                    TokenKind::KwHandler => self
                        .parse_handler_flavored(false, true)
                        .map(Item::Handler),
                    other => {
                        let span = self.peek().span;
                        let found = other.describe();
                        self.error(
                            format!(
                                "expected `effect` or `handler` after `platform`, found \
                                 {found}: a platform declaration is either the group of \
                                 functions the host implements (`platform effect`) or a \
                                 host implementation of a Salvo effect (`platform \
                                 handler`)"
                            ),
                            span,
                        );
                        None
                    }
                }
            }
            TokenKind::KwEffect => self.parse_effect(false).map(Item::Effect),
            // [async-effect-kind] `async effect E { … }`: a process protocol.
            // Contextual — at item level a bare identifier is otherwise a
            // parse error, which is what makes one token of lookahead enough
            // (the `iter fn` precedent).
            TokenKind::Ident(name)
                if name == ASYNC_MODIFIER && matches!(self.peek_at(1).kind, TokenKind::KwEffect) =>
            {
                self.bump();
                self.parse_effect_kinded(false, true).map(Item::Effect)
            }
            TokenKind::KwHandler => self.parse_handler(false).map(Item::Handler),
            // [qual-refn] A top-level refinement: the consumer's own
            // statement about a function, which is how conflicting
            // refinements from two qualifiers get reconciled.
            TokenKind::KwRefn => self.parse_refn().map(Item::Refn),
            // [fn-rename] A module-scoped name for one overload.
            TokenKind::KwRename => self.parse_rename().map(Item::Rename),
            TokenKind::KwParams => self.parse_params_group().map(Item::Params),
            TokenKind::KwFn => {
                let f = self.parse_fn(false)?;
                // [decl-body] A top-level `fn` without a body was
                // `external fn`'s shape. The two things it could have meant
                // now have their own spellings, so the error names both
                // rather than reporting a bare "expected `{`".
                if f.body.is_none() {
                    self.error(
                        format!(
                            "`fn {}` has no body: write one, or — if the target \
                             language implements it — declare it as a member of a \
                             `platform effect`",
                            f.name.name
                        ),
                        f.span,
                    );
                }
                Some(Item::Fn(f))
            }
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

    fn parse_type_decl(&mut self, intrinsic: bool) -> Option<TypeDecl> {
        self.parse_type_decl_linear(intrinsic, false)
    }

    /// [linear-group] `linear intrinsic type Reply<T>`: the obligation
    /// modifier on an opaque type. Only an `intrinsic type` may carry it —
    /// an alias is a name for another type, and the obligation belongs to
    /// the type itself.
    fn parse_type_decl_linear(&mut self, intrinsic: bool, linear: bool) -> Option<TypeDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwType)?.span;
        let name = self.ident_type("type")?;
        // [linear-container] A type declaration takes per-parameter `canbe`
        // opt-ins like a struct's: `intrinsic type List<T canbe linear>` is
        // what makes `List<Reply<T>>` a linear type and `List<Int>` a plain
        // one (user decision 2026-09-16).
        let (generics, generic_canbe) = self.parse_generics_canbe();
        // `canbe Mut` — auto-qualifiers the type opts into
        // [type-canbe-mut] [canbe-optin].
        let mut auto_qualifiers = Vec::new();
        if self.eat(&TokenKind::KwCanbe).is_some() {
            loop {
                auto_qualifiers.push(self.parse_type_ref()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        let alias = if self.eat(&TokenKind::Eq).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        if linear && alias.is_some() {
            self.error(
                "an alias cannot be linear: the obligation belongs to the type \
                 itself, so write `linear intrinsic type` (or `linear struct`) \
                 where it is declared",
                name.span,
            );
        }
        let end = alias
            .as_ref()
            .map(|t| t.span())
            .or_else(|| auto_qualifiers.last().map(|q| q.span))
            .unwrap_or(name.span);
        Some(TypeDecl {
            docs,
            intrinsic,
            linear,
            name,
            generics,
            generic_canbe,
            auto_qualifiers,
            alias,
            span: start.to(end),
        })
    }

    /// `<A, B, C>` — declaration-site generic parameters.
    fn parse_generics(&mut self) -> Vec<Ident> {
        let (generics, canbe) = self.parse_generics_canbe();
        for (ident, _) in &canbe {
            self.error(
                "`canbe` on a type parameter is only supported on functions and structs",
                ident.span,
            );
        }
        generics
    }

    /// Type parameters with optional per-parameter `canbe` opt-ins
    /// [linear-generics] [canbe-optin]: `<T canbe linear, U>` — one
    /// qualifier per `canbe` (the comma separates parameters).
    fn parse_generics_canbe(&mut self) -> (Vec<Ident>, Vec<(Ident, TypeRef)>) {
        let mut generics = Vec::new();
        let mut canbe = Vec::new();
        if self.at(&TokenKind::Lt) {
            self.group_depth += 1;
            self.bump();
            loop {
                if self.eat(&TokenKind::Gt).is_some() || self.at_eof() {
                    break;
                }
                match self.ident_type("generic parameter") {
                    Some(id) => {
                        if self.eat(&TokenKind::KwCanbe).is_some() {
                            if let Some(q) = self.parse_type_ref() {
                                canbe.push((id.clone(), q));
                            }
                        }
                        generics.push(id);
                    }
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
        (generics, canbe)
    }

    fn parse_struct(&mut self, linear: bool) -> Option<StructDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwStruct)?.span;
        let name = self.ident_decl_dotted("struct")?;
        // [linear-generics] Structs take per-parameter `canbe` opt-ins too
        // (user decision 2026-09-12): `struct Box<T canbe linear>` is the
        // conditional-container declaration.
        let (generics, generic_canbe) = self.parse_generics_canbe();
        // `: Linear, Yield<Str>` — obligation groups this type satisfies
        // [group-obligation]. Before `canbe`, because `:` states what the
        // type must *provide* while `canbe` states what it may be qualified
        // as.
        let mut obligations = Vec::new();
        if self.eat(&TokenKind::Colon).is_some() {
            loop {
                obligations.push(self.parse_type_ref()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        // `canbe Mut` — auto-qualifiers the struct opts into
        // [struct-mut] [canbe-optin].
        let mut auto_qualifiers = Vec::new();
        if self.eat(&TokenKind::KwCanbe).is_some() {
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
            docs,
            name,
            generics,
            generic_canbe,
            obligations,
            auto_qualifiers,
            fields,
            linear,
            span: start.to(end),
        })
    }

    /// `name: Type (= default)?`
    fn parse_field_decl(&mut self) -> Option<FieldDecl> {
        let docs = self.docs_here();
        let name = self.ident_value("field")?;
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
            docs,
            name,
            ty,
            default,
            span,
        })
    }

    fn parse_qualifier(
        &mut self,
        intrinsic: bool,
        subject: QualSubject,
    ) -> Option<QualifierDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwQualifier)?.span;
        let name = self.ident_decl_dotted("qualifier")?;
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
        let mut refns = Vec::new();
        let mut has_body = false;
        let mut end = of.span();
        if self.at(&TokenKind::LBrace) && self.same_line() {
            has_body = true;
            self.bump();
            while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                if self.at(&TokenKind::KwFn) {
                    fns.push(self.parse_fn(false)?);
                } else if self.at(&TokenKind::KwRefn) {
                    // [qual-refn] A refinement of a function this
                    // qualifier does not own.
                    refns.push(self.parse_refn()?);
                } else {
                    field_overrides.push(self.parse_field_decl()?);
                    self.eat(&TokenKind::Comma);
                }
            }
            end = self.expect(&TokenKind::RBrace)?.span;
        }
        Some(QualifierDecl {
            docs,
            intrinsic,
            subject,
            name,
            generics,
            of,
            with,
            field_overrides,
            fns,
            refns,
            has_body,
            span: start.to(end),
        })
    }

    /// `refn add(list: Mut List<T>, elem: T) => list: +NonEmpty`
    /// [qual-refn].
    ///
    /// Narrower than a `fn` by construction: no body, no effect list, no
    /// return type, and a deduction list that can only add and remove
    /// qualifiers. Each of the three omissions is a diagnostic naming the
    /// reason rather than a bare parse error, since each is a plausible
    /// thing to try.
    /// [fn-rename] `rename fn add2 = add(a: Int, b: Int)`.
    ///
    /// The parameter list is there to pick one overload, so it repeats the
    /// parameters exactly — same names, same types [qual-refn-match]. What it
    /// may *not* repeat is anything that plays no part in selection: an
    /// effect list, a deduction list, or a return type. Each is reported by
    /// name, since writing one is a reasonable guess about how this works.
    fn parse_rename(&mut self) -> Option<RenameDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwRename)?.span;
        self.expect(&TokenKind::KwFn)?;
        let name = self.ident_value("renamed function")?;
        self.expect(&TokenKind::Eq)?;
        let target = self.ident_value("function")?;
        let generics = self.parse_generics();
        let (params, implicit_groups) = self.parse_params()?;
        // [implicit-param] Implicits play no part in selection either, so a
        // group spread here is the same mistake as an effect list.
        if !implicit_groups.is_empty() {
            let span = name.span;
            self.error(
                format!(
                    "`rename fn {}` names an overload by its ordinary \
                     parameters: implicit parameters take no part in choosing \
                     one, so `?Group` does not belong here",
                    name.name
                ),
                span,
            );
            return None;
        }
        let params_end = params
            .last()
            .map(|p| p.span)
            .unwrap_or(target.span);
        if (self.at(&TokenKind::LBracket) || self.at(&TokenKind::FatArrow)) && self.same_line() {
            let span = self.peek().span;
            self.error(
                format!(
                    "`rename fn {}` names an overload by its parameters, so it \
                     takes no effect or deduction clause: neither takes part in \
                     choosing an overload",
                    name.name
                ),
                span,
            );
            return None;
        }
        if self.at(&TokenKind::Arrow) && self.same_line() {
            let span = self.peek().span;
            self.error(
                format!(
                    "`rename fn {}` names an overload by its parameters, so it \
                     takes no return type: overloads are never chosen by what \
                     they return",
                    name.name
                ),
                span,
            );
            return None;
        }
        Some(RenameDecl {
            docs,
            name,
            target,
            generics,
            params,
            span: start.to(params_end),
        })
    }

    fn parse_refn(&mut self) -> Option<RefnDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwRefn)?.span;
        let name = self.ident_value("refinement")?;
        let generics = self.parse_generics();
        let (params, _) = self.parse_params()?;
        // An effect list here is the most likely mistake: a refinement
        // states what is *known* after a call, and a function's effects
        // belong to the function.
        if self.at(&TokenKind::LBracket) {
            let span = self.peek().span;
            self.error(
                format!(
                    "a refinement cannot declare effects: `refn {}` only states \
                     what is known about the arguments afterwards, so write \
                     `=> param: +Qual` here",
                    name.name
                ),
                span,
            );
            return None;
        }
        if self.at(&TokenKind::Arrow) {
            let span = self.peek().span;
            self.error(
                format!(
                    "a refinement cannot declare a return type: `refn {}` \
                     refines an existing function's deductions, and the \
                     function decides what it returns; write `=> {}: +Qual`",
                    name.name,
                    params.first().map(|p| p.name.name.as_str()).unwrap_or("param")
                ),
                span,
            );
            return None;
        }
        if !self.at(&TokenKind::FatArrow) {
            let span = self.peek().span;
            self.error(
                format!(
                    "expected a deduction clause in `refn {}`, e.g. \
                     `=> {}: +Qual`: a refinement exists to state one",
                    name.name,
                    params
                        .first()
                        .map(|p| p.name.name.as_str())
                        .unwrap_or("param")
                ),
                span,
            );
            return None;
        }
        self.bump();
        let deductions = self.parse_refn_deduction_list()?;
        // A return type would claim the refinement changes what the
        // function produces, which is exactly what it may not do. Only a
        // token that could *start* a type is reported, so a `}` closing the
        // qualifier body on the same line is not mistaken for one.
        if self.same_line() && (self.at_ident() || self.at(&TokenKind::LParen)) {
            let span = self.peek().span;
            self.error(
                format!(
                    "a refinement cannot declare a return type: `refn {}` \
                     refines an existing function's deductions, and the \
                     function decides what it returns",
                    name.name
                ),
                span,
            );
            return None;
        }
        let end = deductions.last().map(|d| d.span).unwrap_or(start);
        Some(RefnDecl {
            docs,
            name,
            generics,
            params,
            deductions,
            span: start.to(end),
        })
    }

    /// `=> list: +NonEmpty`, `=> list: -Sorted`, `=> a: +P, b: -Q`
    /// [qual-refn]: every entry is a set of additions and removals, so a
    /// plain (exhaustive) name or `Nothing` is rejected — a refinement
    /// never decides whether a parameter is kept.
    fn parse_refn_deduction_list(&mut self) -> Option<Vec<RefnDeduction>> {
        let mut entries: Vec<RefnDeduction> = Vec::new();
        loop {
            let param = self.ident()?;
            let mut end = param.span;
            let mut add: Vec<TypeRef> = Vec::new();
            let mut remove: Vec<TypeRef> = Vec::new();
            if self.eat(&TokenKind::Colon).is_none() {
                self.error(
                    format!(
                        "`{}` states nothing: a refinement entry adds or removes \
                         qualifiers, e.g. `[{}: +Qual]`",
                        param.name, param.name
                    ),
                    param.span,
                );
            }
            loop {
                let plus = self.at(&TokenKind::Plus);
                let minus = self.at(&TokenKind::Minus);
                if !plus && !minus {
                    if self.at_ident() {
                        // A plain name would be the exhaustive form, which
                        // decides keptness — not a refinement's business.
                        let r = self.parse_type_ref()?;
                        self.error(
                            format!(
                                "a refinement's qualifiers need a sign: write \
                                 `+{}` if the call establishes it or `-{}` if it \
                                 invalidates it (a plain name would mean \"only \
                                 this survives\", which is the function's own \
                                 deduction to make)",
                                r.name.name, r.name.name
                            ),
                            r.span,
                        );
                        end = r.span;
                        continue;
                    }
                    break;
                }
                self.bump();
                if !self.at_ident() {
                    break;
                }
                let Some(r) = self.parse_type_ref() else { break };
                end = r.span;
                if plus {
                    add.push(r);
                } else {
                    remove.push(r);
                }
            }
            entries.push(RefnDeduction {
                span: param.span.to(end),
                param,
                add,
                remove,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        Some(entries)
    }

    fn parse_effect(&mut self, platform: bool) -> Option<EffectDecl> {
        self.parse_effect_kinded(platform, false)
    }

    /// [async-effect-kind] `async effect E { … }` — a process protocol. The
    /// modifier is **contextual** (`async` followed by `effect`), like every
    /// other word this phase added: nothing is reserved, so `async` stays a
    /// legal name. There is deliberately no `async fn` — the phase decided
    /// against colouring — so this is the only place the word appears.
    fn parse_effect_kinded(&mut self, platform: bool, is_async: bool) -> Option<EffectDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwEffect)?.span;
        let name = self.ident_type("effect")?;
        let generics = self.parse_generics();
        self.expect(&TokenKind::LBrace)?;
        let mut fns = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            fns.push(self.parse_member_fn()?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(EffectDecl {
            docs,
            platform,
            is_async,
            name,
            generics,
            fns,
            span: start.to(end),
        })
    }

    /// `params Field<T> { fn add(a: T, b: T) -> T ... }` [implicit-group]: a
    /// named bundle of implicit parameters. Shaped like an effect
    /// declaration, because it is the same thing — a set of function
    /// signatures — but supplied by *resolution* rather than by a handler.
    fn parse_params_group(&mut self) -> Option<ParamsDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwParams)?.span;
        let name = self.ident_type("params group")?;
        let generics = self.parse_generics();
        self.expect(&TokenKind::LBrace)?;
        let mut fns = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            fns.push(self.parse_fn(false)?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(ParamsDecl {
            docs,
            name,
            generics,
            fns,
            span: start.to(end),
        })
    }

    fn parse_handler(&mut self, intrinsic: bool) -> Option<HandlerDecl> {
        self.parse_handler_flavored(intrinsic, false)
    }

    /// `handler`, `intrinsic handler` [backend-intrinsic] and
    /// `platform handler` [platform-handler] share every piece of grammar:
    /// the two modifiers differ only in who supplies the members.
    fn parse_handler_flavored(
        &mut self,
        intrinsic: bool,
        platform: bool,
    ) -> Option<HandlerDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwHandler)?.span;
        let name = self.ident_type("handler")?;
        let generics = self.parse_generics();
        let mut params = Vec::new();
        if self.at(&TokenKind::LParen) {
            // A handler constructor takes no implicit parameters; the checker
            // reports one written here [implicit-fn-only].
            params = self.parse_params()?.0;
        }
        // [effect-handler-deps] The effects the handler depends on, written
        // exactly as a fn's: `handler Stamped [Logger, Clock] of Logger`.
        // They have no names because nothing can refer to them — a member
        // body calls their members like any other code.
        let effects = if self.at(&TokenKind::LBracket) {
            Some(self.parse_effect_list()?)
        } else {
            None
        };
        self.expect(&TokenKind::KwOf)?;
        let of = self.parse_type()?;
        let mut state = Vec::new();
        let mut fns = Vec::new();
        let mut end = of.span();
        if self.at(&TokenKind::LBrace) && self.same_line() {
            self.bump();
            while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                // [async-send-fn] `send fn` is a member too; anything else
                // that is not a `fn` is a state field.
                let is_send_member =
                    self.at_word("send") && matches!(self.peek_at(1).kind, TokenKind::KwFn);
                if self.at(&TokenKind::KwFn) || is_send_member {
                    fns.push(self.parse_member_fn()?);
                } else {
                    state.push(self.parse_field_decl()?);
                    self.eat(&TokenKind::Comma);
                }
            }
            end = self.expect(&TokenKind::RBrace)?.span;
        }
        Some(HandlerDecl {
            docs,
            intrinsic,
            platform,
            name,
            generics,
            params,
            effects,
            of,
            state,
            fns,
            span: start.to(end),
        })
    }

    // --- Functions ---

    fn parse_fn(&mut self, intrinsic: bool) -> Option<FnDecl> {
        self.parse_fn_flavored(intrinsic, FnFlavor::Plain)
    }

    /// [iter-fn] `iter fn` is the generated-pass form; the flavour is carried on
    /// the declaration rather than inferred from the body.
    /// [async-send-fn] A member of an effect or a handler, with the
    /// contextual `send` modifier: `send fn bump(n: Int)`. Contextual for
    /// the reason `iter fn` is (a member named `send` must stay declarable,
    /// and `r.send(v)` is how a reply token is discharged), and unambiguous
    /// with no lookahead beyond the next token: inside a member list a bare
    /// identifier is otherwise a parse error.
    fn parse_member_fn(&mut self) -> Option<FnDecl> {
        if self.at_word("send") && matches!(self.peek_at(1).kind, TokenKind::KwFn) {
            self.bump();
            return self.parse_fn_flavored(false, FnFlavor::Send);
        }
        self.parse_fn(false)
    }

    fn parse_fn_flavored(&mut self, intrinsic: bool, flavor: FnFlavor) -> Option<FnDecl> {
        let is_iter = flavor == FnFlavor::Iter;
        let is_send = flavor == FnFlavor::Send;
        // The docs sit above the whole declaration; `external`/`intrinsic`
        // is on the same line as `fn`, so the line lookup finds them
        // whether or not the modifier was already consumed [doc-comment].
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwFn)?.span;
        let name = self.ident_value("fn")?;
        let (generics, generic_canbe) = self.parse_generics_canbe();
        let (mut params, implicit_groups) = self.parse_params()?;

        // Effects: `[Random<Int>, Console, use]`
        let effects = if self.at(&TokenKind::LBracket) && self.same_line() {
            Some(self.parse_effect_list()?)
        } else {
            None
        };

        // `-> return_type (as Qualifier)?`, then the deduction clause
        // `=> entries` [deduce-syntax].
        let mut return_type = None;
        let mut constructs = None;
        let mut derived_return = None;
        if self.at(&TokenKind::Arrow) && self.same_line() {
            self.bump();
            if self.at(&TokenKind::LBracket) {
                let span = self.peek().span;
                self.error(
                    "deductions are written after the return type, behind `=>` \
                     (`-> Int => list: Mut`), not in brackets after `->`",
                    span,
                );
                return None;
            }
            let ty = self.parse_type()?;
            // [proj-anywhere] The derived-return summary the checker and the
            // emitters consume: the parameter named by the *first* `proj` in
            // the return type. `proj[from: p] T` is now an ordinary qualifier
            // on `T`, so the old prefix spelling reads identically.
            derived_return = first_proj_source(&ty);
            return_type = Some(ty);
            // `-> T as Qualifier` marks a constructive-qualifier constructor.
            if self.at(&TokenKind::KwAs) && self.same_line() {
                self.bump();
                constructs = Some(self.parse_type_ref()?);
            }
        }
        // [deduce-syntax] `=> …` groups: the unnamed ones are the fn's own
        // clause; `=>[f] …` belongs to the fn-typed parameter `f`.
        let deductions = self.parse_deduction_clause(&mut params)?;

        // [iter-fn] An `iter fn`'s body opens with the pass's own fields. It is
        // parsed here rather than as a statement so the body that follows is an
        // ordinary block: `state` declares data, it does not run.
        let mut iter_state = Vec::new();
        let body = if self.at(&TokenKind::LBrace) && self.same_line() {
            if is_iter {
                Some(self.parse_iter_body(&mut iter_state)?)
            } else {
                Some(self.parse_block()?)
            }
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
            docs,
            intrinsic,
            is_iter,
            is_send,
            iter_state,
            name,
            generics,
            generic_canbe,
            params,
            implicit_groups,
            derived_return,
            effects,
            deductions,
            return_type,
            constructs,
            body,
            span: start.to(end),
        })
    }

    /// A parameter list, with the implicit parameters it declares
    /// [implicit-param] [implicit-group]. Two spellings share the `?`:
    ///
    /// - `?cmp: (T, T) -> Int` — one implicit parameter, named and typed.
    /// - `?Field<T>` — a *spread* of a `params` group, with no binder: its
    ///   members become implicit parameters in their own right.
    ///
    /// [name-casing] decides which, from the first token after `?`: values
    /// are lowercase and types uppercase, so no lookahead is needed.
    /// Implicit parameters trail the ordinary ones — an implicit followed by
    /// a normal parameter is a parse error, since a caller could not then
    /// pass the normal one positionally.
    fn parse_params(&mut self) -> Option<(Vec<Param>, Vec<TypeRef>)> {
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut params = Vec::new();
        let mut implicit_groups = Vec::new();
        let mut seen_implicit: Option<Span> = None;
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let variadic = self.eat(&TokenKind::Ellipsis).is_some();
            let implicit_at = self.eat(&TokenKind::Question).map(|t| t.span);
            if let Some(span) = implicit_at {
                if variadic {
                    self.error("a variadic parameter cannot be implicit", span);
                    self.group_depth -= 1;
                    return None;
                }
                seen_implicit = Some(span);
                // `?Field<T>`: a group spread, recognised by its casing.
                if self.at_type_name() {
                    let Some(group) = self.parse_type_ref() else {
                        self.group_depth -= 1;
                        return None;
                    };
                    implicit_groups.push(group);
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                    continue;
                }
            } else if let Some(prev) = seen_implicit {
                self.error(
                    "implicit parameters must come last: a parameter after one \
                     could not be passed positionally",
                    prev,
                );
                self.group_depth -= 1;
                return None;
            }
            let Some(name) = self.ident_value("parameter") else {
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
                implicit: implicit_at.is_some(),
                span,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        self.expect(&TokenKind::RParen)?;
        Some((params, implicit_groups))
    }

    /// Whether the next token is an identifier naming a *type*
    /// [name-casing] — which is what tells `?Field<T>` from `?cmp: …`.
    fn at_type_name(&self) -> bool {
        matches!(&self.kind(), TokenKind::Ident(name)
            if name.starts_with(|c: char| c.is_uppercase()))
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
                // [async-spawn-effect] `[spawn]`: the process-creation
                // capability, lowercase and compiler-owned like `use`.
                // Contextual, so `spawn` stays available as a name.
                TokenKind::Ident(name) if name == "spawn" => {
                    let tok = self.bump();
                    effects.push(EffectRef::Spawn(tok.span));
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

    /// [deduce-syntax] The deduction clause of a declaration: zero or more
    /// groups, each `=> entry, entry, …` (the fn's own) or `=>[f] entry, …`
    /// (the contract of the fn-typed parameter `f`, attached to its type).
    /// A group may start on the line after the return type; the body's `{`
    /// follows the last entry on its line. Returns the fn's own entries,
    /// `None` when no unnamed group was written.
    fn parse_deduction_clause(&mut self, params: &mut [Param]) -> Option<Option<Vec<Deduction>>> {
        let mut own: Option<Vec<Deduction>> = None;
        while self.at(&TokenKind::FatArrow) {
            let arrow = self.bump().span;
            let group_of: Option<Ident> = if self.at(&TokenKind::LBracket) {
                self.bump();
                let name = self.ident()?;
                self.expect(&TokenKind::RBracket)?;
                Some(name)
            } else {
                None
            };
            let mut entries: Vec<Deduction> = Vec::new();
            loop {
                let Some(entry) = self.parse_deduction_entry() else {
                    return None;
                };
                entries.push(entry);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            if entries.is_empty() {
                self.error("expected a deduction entry after `=>`", arrow);
                return None;
            }
            match group_of {
                None => own.get_or_insert_with(Vec::new).extend(entries),
                Some(name) => {
                    let Some(param) = params.iter_mut().find(|p| p.name.name == name.name) else {
                        self.error(
                            format!("`=>[{}]` names no parameter of this function", name.name),
                            name.span,
                        );
                        return None;
                    };
                    // [fn-contract] [once-fn] Through a qualifier group: a
                    // `once (t: T) -> None` parameter parses as a
                    // `QualifiedGroup` wrapping the fn type, so matching
                    // `Type::Fn` alone rejected exactly the parameter that
                    // most wants a contract — a callback that consumes what
                    // it is given can only be called once.
                    let target = match &mut param.ty {
                        Type::QualifiedGroup { base, .. } => base.as_mut(),
                        other => other,
                    };
                    match target {
                        Type::Fn { deductions, .. } => {
                            deductions.get_or_insert_with(Vec::new).extend(entries);
                        }
                        _ => {
                            self.error(
                                format!(
                                    "`=>[{}]` scopes deductions to a fn-typed parameter, but `{}` \
                                     is not one",
                                    name.name, name.name
                                ),
                                name.span,
                            );
                            return None;
                        }
                    }
                }
            }
        }
        Some(own)
    }

    /// One entry [deduce-syntax]: `!elem`, `elem`, `elem: Qual…`,
    /// `elem: None`, `elem: Nothing`, `elem: -Qual…`, `elem: +Qual…`,
    /// `x.f: proj[from: a]`, `.f: proj[from: a]`, or a bare `proj[from: a]`.
    fn parse_deduction_entry(&mut self) -> Option<Deduction> {
        let start = self.peek().span;
        // `!elem`: consumed.
        if self.at(&TokenKind::Bang) {
            self.bump();
            let name = self.ident()?;
            return Some(Deduction {
                span: start.to(name.span),
                target: DeductionTarget::Param { name, path: Vec::new() },
                kind: DeductionKind::Moved,
            });
        }
        // Bare `proj[from: …]`: opaque.
        if matches!(&self.peek().kind, TokenKind::KwProj) {
            let r = self.parse_type_ref()?;
            if r.from.is_empty() {
                self.error("a bare `proj` deduction needs its sources: `proj[from: c]`", r.span);
            }
            return Some(Deduction {
                span: start.to(r.span),
                target: DeductionTarget::Opaque,
                kind: DeductionKind::Proj(r.from),
            });
        }
        // The target: `.f.g` (result path) or `x` / `x.f` (parameter path).
        let mut path: Vec<Ident> = Vec::new();
        let target = if self.at(&TokenKind::Dot) {
            self.bump();
            path.push(self.ident()?);
            while self.at(&TokenKind::Dot) {
                self.bump();
                path.push(self.ident()?);
            }
            None
        } else {
            let name = self.ident()?;
            while self.at(&TokenKind::Dot) {
                self.bump();
                path.push(self.ident()?);
            }
            Some(name)
        };
        let mut end = path.last().map(|i| i.span).or(target.as_ref().map(|n| n.span)).unwrap_or(start);
        let kind = if self.eat(&TokenKind::Colon).is_some() {
            // `: proj[from: …]`
            if matches!(&self.peek().kind, TokenKind::KwProj) {
                let r = self.parse_type_ref()?;
                if r.from.is_empty() {
                    self.error("a projection entry needs its sources: `proj[from: c]`", r.span);
                }
                end = r.span;
                DeductionKind::Proj(r.from)
            } else {
                // Plain names are *exhaustive* (only these survive); `-`-
                // prefixed names are a delta (drop these, keep the rest).
                // Mixing the two in one entry is an error [deduce-syntax].
                let mut plain: Vec<TypeRef> = Vec::new();
                let mut removed: Vec<TypeRef> = Vec::new();
                loop {
                    let negated = if self.at(&TokenKind::Minus) {
                        end = self.bump().span;
                        true
                    } else if self.at(&TokenKind::Plus) {
                        let span = self.bump().span;
                        self.error(
                            "adding qualifiers in a deduction (`+Qual`) is not \
                             supported yet: a deduction may only preserve or \
                             drop qualifiers",
                            span,
                        );
                        if self.at_ident() {
                            if let Some(r) = self.parse_type_ref() {
                                end = r.span;
                            }
                        }
                        continue;
                    } else {
                        false
                    };
                    if !self.at_ident() {
                        break;
                    }
                    let Some(r) = self.parse_type_ref() else { break };
                    end = r.span;
                    if negated {
                        removed.push(r);
                    } else {
                        plain.push(r);
                    }
                }
                match (plain.is_empty(), removed.is_empty()) {
                    (true, true) => {
                        self.error(
                            "expected qualifiers after `:` — `None` to strip every \
                             qualifier, `Nothing` to consume the value",
                            end,
                        );
                        DeductionKind::Exhaustive(Vec::new())
                    }
                    (false, true) => {
                        let lone = |what: &str| {
                            plain.len() == 1 && plain[0].name.name == what && plain[0].args.is_empty()
                        };
                        if lone("Nothing") {
                            DeductionKind::Moved
                        } else if lone("None") {
                            DeductionKind::Exhaustive(Vec::new())
                        } else {
                            DeductionKind::Exhaustive(plain)
                        }
                    }
                    (true, false) => DeductionKind::Remove(removed),
                    (false, false) => {
                        self.error(
                            "a deduction entry is either exhaustive (plain \
                             qualifier names) or a delta (`-Qual`), not both: \
                             an exhaustive list already drops everything it \
                             does not name",
                            start.to(end),
                        );
                        DeductionKind::Exhaustive(plain)
                    }
                }
            }
        } else {
            DeductionKind::KeepAll
        };
        let target = match target {
            Some(name) => DeductionTarget::Param { name, path },
            None => DeductionTarget::Result { path },
        };
        if matches!(target, DeductionTarget::Result { .. }) && !matches!(kind, DeductionKind::Proj(_)) {
            self.error(
                "a result path (`.field`) can only state a projection: `.field: proj[from: p]`",
                start.to(end),
            );
        }
        if let DeductionTarget::Param { path, .. } = &target {
            if !path.is_empty() && !matches!(kind, DeductionKind::Proj(_)) {
                self.error(
                    "a parameter's field path can only state a projection: `v.field: proj[from: p]`",
                    start.to(end),
                );
            }
        }
        Some(Deduction { span: start.to(end), target, kind })
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
        while (self.at_ident()
            || matches!(self.kind(), TokenKind::KwProj | TokenKind::KwOnce | TokenKind::KwLinear))
            && self.same_line()
        {
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
        let mut names: Vec<Option<Ident>> = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            // A named fn-type parameter: `v: List<Int>` [fn-contract].
            let name = if matches!(self.peek().kind, TokenKind::Ident(_))
                && matches!(self.peek_at(1).kind, TokenKind::Colon)
            {
                let id = self.ident()?;
                self.bump(); // :
                Some(id)
            } else {
                None
            };
            let Some(ty) = self.parse_type() else {
                self.group_depth -= 1;
                return None;
            };
            names.push(name);
            elems.push(ty);
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RParen)?.span;

        let effects = if self.at(&TokenKind::LBracket) && self.same_line() {
            // [type-tuple] A `[` here is ambiguous: it opens a fn type's
            // effect list (`(Int) [] -> Str`) *or* it is the array suffix on
            // a tuple type (`(Str, Int)[]`). Only a fn type continues with
            // `->`, so speculate and roll back — otherwise the suffix is
            // eaten as an empty effect list and a tuple array is rejected
            // with a misleading "expected `->` after effect list".
            let snap = self.snapshot();
            match self.parse_effect_list() {
                Some(list) if self.at(&TokenKind::Arrow) && self.same_line() => Some(list),
                _ => {
                    self.rollback(snap);
                    None
                }
            }
        } else {
            None
        };
        if self.at(&TokenKind::Arrow) && self.same_line() {
            self.bump();
            if self.at(&TokenKind::LBracket) && self.same_line() {
                let span = self.peek().span;
                self.error(
                    "a fn type's deductions are written on the enclosing declaration as a \
                     group, `=>[name] …`, not in brackets after its `->`",
                    span,
                );
                return None;
            }
            let ret = self.parse_type()?;
            let span = start.to(ret.span());
            // The contract [fn-contract] is filled in by the enclosing fn
            // declaration's `=>[name]` group, if one is written.
            return Some(Type::Fn {
                params: elems,
                param_names: names,
                effects,
                deductions: None,
                ret: Box::new(ret),
                span,
            });
        }
        if names.iter().any(|n| n.is_some()) {
            self.error(
                "named parameters are only meaningful in function types \
                 (expected `->` after `)`)",
                start,
            );
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
    /// The name in a type reference: `Name`, or a dot-name `Ns.Name`
    /// [name-dot]. The dot is only taken when *both* segments are type
    /// names, so `person.name` (a field read) and `list.size()` (a
    /// dot-call) are untouched — the casing rule [name-casing] is what
    /// makes that decidable.
    fn type_ref_name(&mut self) -> Option<Ident> {
        // [obligation-spelling] The obligation keywords stand in qualifier
        // position: `proj NonEmpty List<T>`, `once (A) -> B`. They carry
        // their keyword spelling as the name, so the checker and displays
        // agree with the source. `linear` parses here too — `canbe linear`
        // bounds arrive through this path — and the *checker* refuses it in
        // use-site type positions ([linear-group]: a per-value spelling
        // could be forgotten), keeping the diagnostic better than a parse
        // error.
        match self.kind() {
            TokenKind::KwProj => {
                let span = self.bump().span;
                return Some(Ident { name: "proj".to_string(), span });
            }
            TokenKind::KwOnce => {
                let span = self.bump().span;
                return Some(Ident { name: "once".to_string(), span });
            }
            TokenKind::KwLinear => {
                let span = self.bump().span;
                return Some(Ident { name: "linear".to_string(), span });
            }
            _ => {}
        }
        let head = self.ident()?;
        // [iter-fn] A leading `_` is the compiler's namespace: a generated pass
        // struct is `__Pass_<Subject>`, and it exists as a real declaration
        // after the desugaring, so without this a program could *name* it —
        // and the whole point of generating it is that a pass you must name is
        // written by hand.
        if head.name.starts_with('_') {
            self.error(
                format!(
                    "`{}` is a compiler-generated name and cannot be written: a \
                     pass a program needs to name is declared as a struct of its \
                     own, with its own `next`",
                    head.name
                ),
                head.span,
            );
            return Some(head);
        }
        if !head.name.starts_with(|c: char| c.is_uppercase()) {
            return Some(head);
        }
        if !self.at(&TokenKind::Dot) || !self.same_line() {
            return Some(head);
        }
        if !matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            return Some(head);
        }
        self.bump();
        let tail = self.ident()?;
        Some(Ident {
            name: format!("{}.{}", head.name, tail.name),
            span: head.span.to(tail.span),
        })
    }

    fn parse_type_ref(&mut self) -> Option<TypeRef> {
        let name = self.type_ref_name()?;
        let mut args = Vec::new();
        let mut end = name.span;
        // [proj-anywhere] `proj[from: param]`: the borrow's source, written on
        // the obligation itself so it can sit anywhere a type does — a union
        // arm, a type argument, a tuple element — and so a type borrowing
        // from two parameters names each. Only `proj` takes the bracket; a
        // `[` after any other name is the array postfix `T[]`, handled by
        // the caller, so it is only consumed here when followed by an ident.
        let mut from = Vec::new();
        if name.name == "proj"
            && self.at(&TokenKind::LBracket)
            && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
        {
            self.bump(); // [
            let key = self.ident()?;
            if key.name != "from" {
                self.error("expected `from:` in `proj[from: param]`", key.span);
            }
            self.expect(&TokenKind::Colon)?;
            // `proj[from: a, b]`: several sources at once [proj-anywhere].
            loop {
                from.push(self.ident()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            end = self.expect(&TokenKind::RBracket)?.span;
        }
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
        Some(TypeRef {
            name,
            args,
            from,
            span,
        })
    }

    // --- Blocks and statements ---

    /// [iter-fn] An `iter fn`'s body: an optional `state { … }` field block,
    /// then ordinary statements. The block is *declarations only* (user
    /// decision 2026-09-09) — it is the pass's shape, not code that runs — so
    /// it is parsed with the same `parse_field_decl` a struct body uses, and
    /// every field needs an annotation and an initializer.
    fn parse_iter_body(&mut self, state: &mut Vec<FieldDecl>) -> Option<Block> {
        let start = self.expect(&TokenKind::LBrace)?.span;
        let saved_depth = std::mem::replace(&mut self.group_depth, 0);
        let saved_no_struct = std::mem::replace(&mut self.no_struct, false);
        if self.at(&TokenKind::KwState) {
            let state_span = self.peek().span;
            self.bump();
            if self.expect(&TokenKind::LBrace).is_some() {
                while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                    let before = self.pos;
                    match self.parse_field_decl() {
                        Some(field) => {
                            if field.default.is_none() {
                                self.error(
                                    format!(
                                        "`state` field `{}` needs an initializer: it is \
                                         evaluated once, when the pass is minted",
                                        field.name.name
                                    ),
                                    field.span,
                                );
                            }
                            state.push(field);
                        }
                        None => self.recover_in_block(),
                    }
                    if self.pos == before {
                        self.bump();
                    }
                    self.eat(&TokenKind::Comma);
                }
                self.expect(&TokenKind::RBrace)?;
            }
            if state.is_empty() {
                self.error(
                    "an empty `state` block declares nothing: drop it, or give \
                     the pass a field",
                    state_span,
                );
            }
        }
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
            // [fn-rename] In force from here to the end of the block.
            TokenKind::KwRename => self.parse_rename().map(Stmt::Rename),
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
                    let Some(field) = self.ident_value("field") else {
                        self.group_depth -= 1;
                        return None;
                    };
                    let binding = if self.eat(&TokenKind::Colon).is_some() {
                        let Some(b) = self.ident_value("binding") else {
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
            _ => self.ident_value("binding").map(Pattern::Ident),
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
        let name = self.ident_value("lambda parameter")?;
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
            let Some(name) = (if self.at_ident() {
                self.ident_value("lambda parameter")
            } else {
                None
            }) else {
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
                // [qual-widen] `expr ^ Qual...`, same precedence tier as `is`.
                TokenKind::Caret if self.same_line() => {
                    self.bump();
                    let (quals, binding) = self.parse_is_check()?;
                    if let Some(b) = &binding {
                        self.error(
                            "`^` takes no binding: the subject itself reads \
                             without the qualifier in the checked branch",
                            b.span,
                        );
                    }
                    let end = quals.last().map(|r| r.span).unwrap_or_else(|| lhs.span());
                    let span = lhs.span().to(end);
                    lhs = Expr::Widen {
                        subject: Box::new(lhs),
                        quals,
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
            // [obligation-spelling] `is once (A) -> B`, `is proj Str`: the
            // obligation keywords open a type ref like any qualifier.
            if matches!(
                self.kind(),
                TokenKind::KwProj | TokenKind::KwOnce | TokenKind::KwLinear
            ) {
                refs.push(self.parse_type_ref()?);
                continue;
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
            // [inc-dec] `++i` / `--i`: the step happens first, so the
            // expression's value is the *new* one.
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let down = matches!(self.kind(), TokenKind::MinusMinus);
                let start = self.bump().span;
                let operand = self.parse_unary()?;
                let span = start.to(operand.span());
                Some(Expr::IncDec {
                    operand: Box::new(operand),
                    down,
                    prefix: true,
                    span,
                })
            }
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

    /// After a `.`, an integer token is a **tuple index**
    /// [expr-tuple-index]: `t.0`, and `t.0.1` for a nested one (the lexer
    /// never reads a fraction after a dot, so these are two tokens).
    /// Returns `None` when the next token is not an integer, leaving the
    /// field-name path to the caller.
    fn tuple_index_suffix(&mut self, base: &Expr) -> Option<Expr> {
        let TokenKind::Int { value, long } = *self.kind() else {
            return None;
        };
        let token = self.bump();
        if long {
            self.error(
                "a tuple index has no `L` suffix [expr-tuple-index]",
                token.span,
            );
        }
        let index = match usize::try_from(value) {
            Ok(i) => i,
            Err(_) => {
                self.error(
                    format!("`{value}` is not a valid tuple index [expr-tuple-index]"),
                    token.span,
                );
                0
            }
        };
        Some(Expr::TupleIndex {
            base: Box::new(base.clone()),
            index,
            span: base.span().to(token.span),
        })
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.kind() {
                TokenKind::Dot => {
                    self.bump();
                    // `t.0` — a tuple element by constant index
                    // [expr-tuple-index].
                    if let Some(indexed) = self.tuple_index_suffix(&expr) {
                        expr = indexed;
                        continue;
                    }
                    let field = self.ident()?;
                    let span = expr.span().to(field.span);
                    expr = Expr::Field {
                        base: Box::new(expr),
                        field,
                        span,
                    };
                }
                // [fn-overload-at] `name@core.list(...)`: the module whose
                // overload is meant. Written on the *name*, so it attaches
                // to an identifier or to the field of a dot-notation call.
                // [effect-at] `name@Fs(...)`: a *capitalized* name after
                // `@` is an effect instead — the member's owner, where two
                // effects declare the same member name
                // [effect-member-overload]. A generic instance is pinned by
                // the call's type arguments (`next_random@Random<Int>()`),
                // which parse exactly as they do without the `@`.
                TokenKind::At if self.same_line() => {
                    let at = self.bump().span;
                    let first = self.ident()?;
                    // [async-self-send] `k@self(args)`: the enclosing
                    // *handler*'s member — a message to this process. One
                    // selector family with `k@E` (an effect's member) and
                    // `k@module` (a module's overload), which is why the
                    // spelling is a selector and not a `self.` receiver: a
                    // reader learns one rule for "the call says which it
                    // means". `self` is lowercase, where [effect-at] reads a
                    // lowercase name as a module path, so it is recognised
                    // here as a contextual selector keyword — a *module*
                    // named `self` is therefore unreachable this way, which
                    // costs nothing (module names are file paths).
                    if first.name == SELF_SELECTOR {
                        let span = expr.span().to(first.span);
                        expr = match expr {
                            Expr::Ident(name) => Expr::SelfScoped { name, span },
                            other => {
                                self.error(
                                    "`@self` selects a member of the enclosing handler, \
                                     so it follows a plain member name (`k@self(…)`)",
                                    at,
                                );
                                other
                            }
                        };
                        continue;
                    }
                    let is_effect = first
                        .name
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_uppercase());
                    if is_effect {
                        let end = first.span;
                        expr = match expr {
                            Expr::Ident(name) => {
                                let span = name.span.to(end);
                                Expr::EffectScoped {
                                    base: None,
                                    name,
                                    effect: first,
                                    span,
                                }
                            }
                            Expr::Field { base, field, .. } => {
                                let span = base.span().to(end);
                                Expr::EffectScoped {
                                    base: Some(base),
                                    name: field,
                                    effect: first,
                                    span,
                                }
                            }
                            other => {
                                self.error(
                                    "`@` selects which effect's member a *name* \
                                     means, so it follows a member name \
                                     (`close@Fs(s)`) or a dot-notation call \
                                     (`s.close@Fs()`)",
                                    at,
                                );
                                other
                            }
                        };
                        continue;
                    }
                    let mut module = vec![first];
                    while self.at(&TokenKind::Dot) && self.same_line() {
                        // A dot after the module path may belong to the path
                        // (`core.list`) or to a following field access; a
                        // module segment is always a lowercase name, and so
                        // is a field, so the path simply takes them all —
                        // `f@a.b.c(x)` names module `a.b.c`.
                        self.bump();
                        module.push(self.ident()?);
                    }
                    let end = module.last().unwrap().span;
                    expr = match expr {
                        Expr::Ident(name) => {
                            let span = name.span.to(end);
                            Expr::Scoped {
                                base: None,
                                name,
                                module,
                                span,
                            }
                        }
                        Expr::Field { base, field, .. } => {
                            let span = base.span().to(end);
                            Expr::Scoped {
                                base: Some(base),
                                name: field,
                                module,
                                span,
                            }
                        }
                        other => {
                            self.error(
                                "`@` selects which module's overload a *name* \
                                 means, so it follows a function name \
                                 (`add@core.list(x)`) or a dot-notation call \
                                 (`xs.add@core.list(x)`)",
                                at,
                            );
                            other
                        }
                    };
                }
                TokenKind::LParen if self.same_line() => {
                    let (args, named, end) = self.parse_call_args()?;
                    let span = expr.span().to(end);
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        type_args: Vec::new(),
                        args,
                        named,
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
                }
                TokenKind::Bang if self.same_line() => {
                    let end = self.bump().span;
                    let span = expr.span().to(end);
                    expr = Expr::NonNull {
                        operand: Box::new(expr),
                        span,
                    };
                }
                // [inc-dec] Postfix, same line so a leading `++`/`--` on the
                // next line is not swallowed as this expression's suffix.
                TokenKind::PlusPlus | TokenKind::MinusMinus if self.same_line() => {
                    let down = matches!(self.kind(), TokenKind::MinusMinus);
                    let end = self.bump().span;
                    let span = expr.span().to(end);
                    expr = Expr::IncDec {
                        operand: Box::new(expr),
                        down,
                        prefix: false,
                        span,
                    };
                }
                _ => break,
            }
        }
        Some(expr)
    }

    /// Parses `(arg, arg, ...)`; returns the args and the closing-paren span.
    /// The arguments of a call: positional, then any `name = value`
    /// overrides of implicit parameters [implicit-override].
    ///
    /// The name is recognised *after* parsing the expression, by the `=`
    /// that follows it — unambiguous because assignment is a statement in
    /// Salvo, never an expression, so `=` cannot otherwise appear here.
    fn parse_call_args(&mut self) -> Option<(Vec<Expr>, Vec<NamedArg>, Span)> {
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut args = Vec::new();
        let mut named: Vec<NamedArg> = Vec::new();
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(arg) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            if self.at(&TokenKind::Eq) {
                self.bump();
                let Expr::Ident(name) = arg else {
                    let span = arg.span();
                    self.error(
                        "only an implicit parameter can be given by name here: \
                         write `name = value`",
                        span,
                    );
                    self.group_depth -= 1;
                    return None;
                };
                let Some(value) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                let span = name.span.to(value.span());
                named.push(NamedArg { name, value, span });
            } else {
                if let Some(prev) = named.first() {
                    let span = arg.span();
                    let prev_span = prev.span;
                    let _ = prev_span;
                    self.error(
                        "a positional argument cannot follow a named one",
                        span,
                    );
                    self.group_depth -= 1;
                    return None;
                }
                args.push(arg);
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RParen)?.span;
        Some((args, named, end))
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
        let (args, named, end) = match self.parse_call_args() {
            Some(a) => a,
            None => {
                self.rollback(snap);
                return None;
            }
        };
        let span = callee.span().to(end);
        Some(Expr::Call {
            callee: Box::new(callee.clone()),
            type_args,
            args,
            named,
            span,
        })
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.kind().clone() {
            TokenKind::Int { value, long } => {
                let tok = self.bump();
                Some(Expr::Int {
                    value,
                    long,
                    span: tok.span,
                })
            }
            TokenKind::Float { value, single } => {
                let tok = self.bump();
                Some(Expr::Float {
                    value,
                    single,
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
            TokenKind::Ident(_) => {
                // [async-self-send] The old self-send spelling. `self.k(…)`
                // was the first-pass form for a matter of hours; it is a plain
                // parse error now, naming the selector that replaced it (user
                // decision 2026-09-15). No transitional accept — nothing
                // outside this repository writes Salvo.
                if self.at_word(SELF_SELECTOR) && matches!(self.peek_at(1).kind, TokenKind::Dot) {
                    let span = self.peek().span;
                    self.error(
                        "`self.k(…)` is not the self-send form: write `k@self(…)`, the \
                         selector that names the enclosing handler",
                        span,
                    );
                    self.bump();
                    return Some(Expr::Error { span });
                }
                // The asynchronous forms are **contextual**: each is
                // recognised from its word plus what follows, so `spawn`,
                // `replyto` and `waitfor` all stay usable as ordinary names.
                // [async-spawn-expr] `spawn H(...)` — a *name* follows.
                if self.at_word("spawn") && matches!(self.peek_at(1).kind, TokenKind::Ident(_)) {
                    return self.parse_spawn();
                }
                // [async-replyto] `replyto k(...)` / `replyto! k(...)`.
                if self.at_word("replyto")
                    && (matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                        || (matches!(self.peek_at(1).kind, TokenKind::Bang)
                            && matches!(self.peek_at(2).kind, TokenKind::Ident(_))))
                {
                    return self.parse_replyto();
                }
                // [async-waitfor] `waitfor out: Reply<T> { ... }` — a name
                // and a `:` follow, which no call of a fn named `waitfor`
                // can look like.
                if self.at_word("waitfor")
                    && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                    && matches!(self.peek_at(2).kind, TokenKind::Colon)
                {
                    return self.parse_waitfor();
                }
                self.parse_ident_expr()
            }
            TokenKind::LParen => self.parse_paren_expr(),
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::LBrace => self.parse_brace_expr(),
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwWhen => self.parse_when(),
            TokenKind::KwWhile => self.parse_while(),
            TokenKind::KwFor => self.parse_for(),
            TokenKind::KwTry => self.parse_try(),
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
                // [col-literal] With a type name in front, `{}` is a
                // fieldless struct literal (`Finished {}`) — the empty-brace
                // collection reading applies only to a *bare* `{}`, which has
                // no name to say what it builds.
                let braced_body = self.at(&TokenKind::LBrace)
                    && self.same_line()
                    && (self.brace_is_struct_lit()
                        || matches!(self.peek_at(1).kind, TokenKind::RBrace));
                if braced_body {
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
            // [col-literal] `{}` is an **empty collection literal**, not an
            // empty struct literal: the collections are what people write
            // empty, and a fieldless struct is written with its name
            // (`Finished {}`). Its kind — Set or Map — comes from the
            // expected type, and is an error where nothing supplies one.
            TokenKind::RBrace => false,
            TokenKind::Ellipsis => true,
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
        // [col-literal] A brace collection: `{1, 2}` is a Set and
        // `{"a": 1}` a Map. Reached only after the two older forms have been
        // ruled out, which is what keeps `{x: 1}` a bare struct literal and
        // `{ i: Int -> 0 }` a lambda — a map key is therefore an expression
        // that is not a bare identifier.
        self.parse_brace_collection()
    }

    /// `{1, 2, 3}` (Set) or `{"a": 1, "b": 2}` (Map), told apart by whether
    /// a `:` follows the first element.
    fn parse_brace_collection(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::LBrace)?.span;
        // [col-literal] `{}` — the kind is the expected type's to decide, so
        // it parses as an empty Set literal and the checker reads the
        // position (a `Map` there is equally an empty map).
        if self.at(&TokenKind::RBrace) {
            let end = self.bump().span;
            return Some(Expr::SetLit {
                elems: Vec::new(),
                span: start.to(end),
            });
        }
        self.group_depth += 1;
        let first = match self.parse_expr() {
            Some(e) => e,
            None => {
                self.group_depth -= 1;
                return None;
            }
        };
        if self.eat(&TokenKind::Colon).is_some() {
            // A map: the first `:` decides, and every entry needs one.
            let Some(value) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            let mut entries = vec![(first, value)];
            while self.eat(&TokenKind::Comma).is_some() {
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let Some(k) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                if self.eat(&TokenKind::Colon).is_none() {
                    let span = self.peek().span;
                    self.error(
                        "expected `:` after a map literal's key — a `{...}` \
                         whose first entry has one is a map, so every entry \
                         needs a value",
                        span,
                    );
                    self.group_depth -= 1;
                    return None;
                }
                let Some(v) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                entries.push((k, v));
            }
            self.group_depth -= 1;
            let end = self.expect(&TokenKind::RBrace)?.span;
            return Some(Expr::MapLit {
                entries,
                span: start.to(end),
            });
        }
        let mut elems = vec![first];
        while self.eat(&TokenKind::Comma).is_some() {
            if self.at(&TokenKind::RBrace) {
                break;
            }
            let Some(e) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            elems.push(e);
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Expr::SetLit {
            elems,
            span: start.to(end),
        })
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

    /// `try { ... }` — the throw delimiter [try]. Always a block, like
    /// every other body-taking construct.
    fn parse_try(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwTry)?.span;
        if !self.at(&TokenKind::LBrace) {
            let span = self.peek().span;
            self.error("`try` takes a block: `try { ... }`", span);
            return None;
        }
        let body = self.parse_block()?;
        let span = start.to(body.span);
        Some(Expr::Try { body, span })
    }

    /// [async-spawn-expr] `spawn H(args) use D1(...), addr capacity N on POOL`
    /// — the asynchronous binding of a handler. The `use` clause is optional
    /// (a handler with no dependencies needs none); `capacity` and `on` are
    /// not, since neither the mailbox bound nor the pool has a default.
    ///
    /// No clause takes a `{ ... }` body, so struct-literal speculation stays
    /// on throughout: a constructor argument may be a struct literal like
    /// any other argument.
    fn parse_spawn(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `spawn`
        let handler = self.parse_expr()?;
        // The spawn-site `use` clause: what the child's declared
        // dependencies are bound to [effect-handler-deps]. Each item is a
        // handler construction or an `Addr` value — the parser keeps both as
        // expressions, as the `use` *statement* does, and the checker tells
        // them apart.
        let mut uses = Vec::new();
        if self.at(&TokenKind::KwUse) {
            self.bump();
            loop {
                uses.push(self.parse_expr()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect_word(
            "capacity",
            "a spawn states its mailbox bound: `spawn H(...) capacity 16 on pool(2)`",
        )?;
        let capacity = self.parse_expr()?;
        self.expect_word(
            "on",
            "a spawn states where it runs: `on pool(2)` (`pool` is an ordinary function)",
        )?;
        let pool = self.parse_expr()?;
        let span = start.to(pool.span());
        Some(Expr::Spawn {
            handler: Box::new(handler),
            uses,
            capacity: Box::new(capacity),
            pool: Box::new(pool),
            span,
        })
    }

    /// [async-replyto] `replyto k(captures)` — mint a parked one-shot
    /// continuation targeting member `k` of the enclosing handler, yielding
    /// its linear `Reply<T>`. `replyto! k(captures)` is the gated mint: the
    /// process serves nothing else until the answer arrives.
    fn parse_replyto(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `replyto`
        let gated = self.eat(&TokenKind::Bang).is_some();
        let member = self.ident()?;
        // The parentheses are part of the form even when there is nothing to
        // capture — `replyto k()` reads as the continuation it is.
        if !self.at(&TokenKind::LParen) {
            let span = self.peek().span;
            self.error(
                "`replyto` names a member and its captures: `replyto k()`",
                span,
            );
            return None;
        }
        let (captures, named, end) = self.parse_call_args()?;
        if let Some(first) = named.first() {
            self.error(
                "a `replyto` takes the continuation's captures positionally",
                first.span,
            );
        }
        Some(Expr::ReplyTo {
            member,
            captures,
            gated,
            span: start.to(end),
        })
    }

    /// [async-waitfor] `waitfor out: Reply<T> { ... }` — `main`'s bridge
    /// into the asynchronous world. The binder's type is written out, since
    /// nothing else in the block says what answer is being waited for.
    fn parse_waitfor(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `waitfor`
        let binding = self.ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        if !self.at(&TokenKind::LBrace) {
            let span = self.peek().span;
            self.error(
                "`waitfor` takes a block that sends the token somewhere: \
                 `waitfor out: Reply<Int> { p.total(out) }`",
                span,
            );
            return None;
        }
        let body = self.parse_block()?;
        let span = start.to(body.span);
        Some(Expr::WaitFor {
            binding,
            ty,
            body,
            span,
        })
    }

    fn parse_when(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwWhen)?.span;
        // [when-condition] `when {` is the subject-less form: a subject is
        // always a plain variable, so a brace here cannot be one.
        if self.at(&TokenKind::LBrace) {
            return self.parse_when_cond(start);
        }
        let subject = self.parse_condition()?;
        self.expect(&TokenKind::LBrace)?;
        let mut branches = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            // [when-union-subject] A subject `when` is exhaustive over the
            // union's arms, so it has no default branch. Name the two forms
            // rather than reporting a missing `is`.
            if self.at(&TokenKind::KwElse) {
                let span = self.bump().span;
                self.error(
                    "`when` over a subject is exhaustive over the union's arms, so \
                     it takes no `else`: drop the subject to write a condition chain \
                     (`when { cond { … } else { … } }`)",
                    span,
                );
                self.parse_block()?;
                continue;
            }
            // [qual-widen] A branch head is `is ...` (narrow) or `^ ...`
            // (widen): the same arm test, opposite effect on the type.
            let widen = self.at(&TokenKind::Caret);
            let head_span = if widen {
                self.bump().span
            } else {
                self.expect(&TokenKind::KwIs)?.span
            };
            let (check, binding) = self.parse_is_check()?;
            if widen {
                if let Some(b) = &binding {
                    self.error(
                        "a `^` branch takes no binding: the subject itself \
                         reads without the qualifier inside the branch",
                        b.span,
                    );
                }
            }
            let body = self.parse_block()?;
            let span = head_span.to(body.span);
            branches.push(WhenBranch {
                check,
                binding: if widen { None } else { binding },
                widen,
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

    /// [when-condition] The subject-less `when`: bare boolean branch heads
    /// followed by blocks, closed by a mandatory `else`. `when` is always
    /// exhaustive — with no subject there are no arms to be exhaustive
    /// over, so the `else` is what supplies it.
    fn parse_when_cond(&mut self, start: Span) -> Option<Expr> {
        self.expect(&TokenKind::LBrace)?;
        let mut branches: Vec<(Expr, Block)> = Vec::new();
        let mut else_block: Option<Block> = None;
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::KwElse) {
                let else_span = self.bump().span;
                let block = self.parse_block()?;
                if !self.at(&TokenKind::RBrace) && !self.at_eof() {
                    self.error(
                        "`else` is the last branch of a `when`: it runs when no \
                         condition above it held",
                        else_span,
                    );
                }
                else_block = Some(block);
                continue;
            }
            let cond = self.parse_condition()?;
            let block = self.parse_block()?;
            branches.push((cond, block));
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        let span = start.to(end);
        let Some(else_block) = else_block else {
            self.error(
                "a subject-less `when` must end with an `else`: `when` is always \
                 exhaustive, and with no subject the `else` is what makes it so \
                 (use `if`/`elif` when there is nothing to fall back to)",
                span,
            );
            return Some(Expr::Error { span });
        };
        if branches.is_empty() {
            self.error(
                "a `when` with only an `else` has nothing to decide: write the \
                 block on its own",
                span,
            );
            return Some(Expr::Error { span });
        }
        Some(Expr::WhenCond {
            branches,
            else_block,
            span,
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
    // Interpolations hold expressions, never declarations, so they carry
    // no docs.
    let mut parser = Parser::new(source, tokens, Vec::new());
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


/// Which flavour of `fn` is being parsed. A modifier is carried on the
/// declaration rather than inferred from the body: [iter-fn] for
/// `yield fn`, [iter-fn] for `iter fn`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FnFlavor {
    Plain,
    Iter,
    /// [async-send-fn] `send fn`: an asynchronous member of a process
    /// protocol — legal only inside an effect or a handler.
    Send,
}

/// [proj-anywhere] The source parameter of the first `proj[from: p]` in a
/// type, searching arms, arguments and elements in order.
pub fn first_proj_source(ty: &Type) -> Option<Ident> {
    // [proj-infer] Only a *wholesale* `proj` — on the result itself, an
    // arm, a tuple element — makes a derived return. A `proj` inside a type
    // argument (`List<proj T>`) is a borrow the result *holds*, tracked as a
    // lend, so type arguments are not descended into.
    fn in_ref(r: &TypeRef) -> Option<Ident> {
        if r.name.name == "proj" {
            if let Some(from) = r.from.first() {
                return Some(from.clone());
            }
        }
        None
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers
            .iter()
            .find_map(in_ref)
            .or_else(|| in_ref(base)),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers
            .iter()
            .find_map(in_ref)
            .or_else(|| first_proj_source(base)),
        Type::Nullable { inner, .. } | Type::Array { elem: inner, .. } => first_proj_source(inner),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().find_map(first_proj_source)
        }
        _ => None,
    }
}
