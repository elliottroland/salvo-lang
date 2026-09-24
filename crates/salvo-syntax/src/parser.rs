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

/// [actor-self-send] The contextual selector that names the enclosing handler:
/// `k@self(args)`. Contextual, not reserved — `self` stays an ordinary name
/// everywhere else, and it is only special immediately after `@`.
pub const SELF_SELECTOR: &str = "self";

/// [mod-export] The contextual modifier that makes a declaration visible to
/// other modules: `export fn size(...)`. Declarations are module-private by
/// default (user decision 2026-09-18), and this is the only way out. Contextual
/// like the rest — a variable may still be called `export`; only the start of a
/// top-level declaration makes it a modifier, where a bare identifier would be
/// a parse error anyway.
pub const EXPORT_MODIFIER: &str = "export";

/// [assert-fn] The two compiler-owned assertion forms, spelled with a trailing
/// `!`: `assert!(cond, "why")` and `unreachable!("why")`. Contextual — both
/// names stay ordinary identifiers everywhere else.
pub const ASSERT_FORM: &str = "assert";
pub const UNREACHABLE_FORM: &str = "unreachable";

/// [actor-effect-kind] The contextual modifier that makes an effect an actor
/// protocol: `actor effect E { … }`. Contextual, not reserved — and the only
/// place the word appears in the language, since the phase decided against
/// colouring functions.
pub const ACTOR_MODIFIER: &str = "actor";

/// [actor-mailbox] The contextual name of an actor handler's settings slot:
/// `mailbox { capacity: 16 }` (user decision 2026-09-16). Contextual like every
/// other word this phase added — a state field may still be called `mailbox`;
/// only a brace after it makes the slot.
pub const MAILBOX_SLOT: &str = "mailbox";

/// [actor-mailbox] The std struct the slot's braces build: the block is that
/// struct's literal with the type elided, which is what gives the fields their
/// names, types, defaults and diagnostics for free.
pub const MAILBOX_TYPE: &str = "Mailbox";

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
    /// reserving it anywhere else (`send fn`, `spawn`, `mailbox`, `on`).
    fn at_word(&self, word: &str) -> bool {
        matches!(self.kind(), TokenKind::Ident(name) if name == word)
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

    /// [mod-export] A declaration, with the optional `export` modifier in
    /// front of it.
    ///
    /// **Contextual**, like `iter`, `send` and `actor`: `export` stays an
    /// ordinary identifier everywhere else, and there is nothing to
    /// disambiguate here because a bare identifier at item level is a parse
    /// error anyway. It comes first, before `intrinsic` / `linear` / `actor` /
    /// `platform` / `provenance`, so a declaration reads
    /// "who can see it, what kind it is, what it is called".
    fn parse_item(&mut self) -> Option<Item> {
        let export_span = match self.kind() {
            TokenKind::Ident(name) if name == EXPORT_MODIFIER => {
                let span = self.peek().span;
                self.bump();
                Some(span)
            }
            _ => None,
        };
        let item = self.parse_declaration()?;
        let Some(span) = export_span else {
            return Some(item);
        };
        // Only declarations carry visibility. The three items that are not
        // declarations each get their own reason, since "expected item" would
        // hide what is actually wrong.
        match item {
            Item::Type(mut d) => {
                d.exported = true;
                Some(Item::Type(d))
            }
            Item::Struct(mut d) => {
                d.exported = true;
                Some(Item::Struct(d))
            }
            Item::Qualifier(mut d) => {
                d.exported = true;
                Some(Item::Qualifier(d))
            }
            Item::Effect(mut d) => {
                d.exported = true;
                Some(Item::Effect(d))
            }
            Item::Handler(mut d) => {
                d.exported = true;
                Some(Item::Handler(d))
            }
            Item::Params(mut d) => {
                d.exported = true;
                Some(Item::Params(d))
            }
            Item::Fn(mut d) => {
                d.exported = true;
                Some(Item::Fn(d))
            }
            Item::Import(_) => {
                self.error(
                    "`export` cannot precede an `import`: a module states what *it* \
                     declares, and there is no re-export — import the original name \
                     where you need it [mod-export]",
                    span,
                );
                Some(item)
            }
            Item::Refn(_) => {
                self.error(
                    "`export` cannot precede a `refn`: a refinement travels with the \
                     qualifier whose claim it is about, so it is visible wherever that \
                     qualifier is [mod-export]",
                    span,
                );
                Some(item)
            }
            Item::Rename(_) => {
                self.error(
                    "`export` cannot precede a `rename`: a rename is a name for this \
                     file's own use, not a declaration — export the function it names \
                     instead [mod-export]",
                    span,
                );
                Some(item)
            }
            Item::Test(_) => {
                self.error(
                    "`export` cannot precede a `test`: a test is run, never referenced, \
                     so there is nothing to make visible [test-decl]",
                    span,
                );
                Some(item)
            }
        }
    }

    fn parse_declaration(&mut self) -> Option<Item> {
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
            // [free-send-fn] `send fn parse_row(out: Reply<User>, row: Row)` —
            // the send kind extended to a *free* function (user decision
            // 2026-09-17, FC-1(a)): a unit of work that runs by being
            // scheduled rather than called, and what a `replyto` may target
            // outside a handler. Contextual for the same reasons as the member
            // form: `send` is an ordinary name, and `r.send(v)` is how a reply
            // token is discharged.
            TokenKind::Ident(name)
                if name == "send" && matches!(self.peek_at(1).kind, TokenKind::KwFn) =>
            {
                self.bump();
                self.parse_fn_flavored(false, FnFlavor::Send).map(Item::Fn)
            }
            // [cmp-auto] `auto fn cmp@Person(a: Person, b: Person) -> Int` — a
            // bodiless declaration whose body the *compiler* writes,
            // structurally, from the type's fields. Contextual for the same
            // reason `iter fn` is: `auto` is an ordinary name everywhere else,
            // and at item level a bare identifier is otherwise a parse error.
            TokenKind::Ident(name)
                if name == "auto" && matches!(self.peek_at(1).kind, TokenKind::KwFn) =>
            {
                self.bump();
                let mut f = self.parse_fn(false)?;
                f.structural = true;
                // A body is refused by the *checker*, beside the other `auto`
                // rules: it is a semantic mistake about what `auto` means, and
                // keeping the family of diagnostics in one place is what lets
                // them share wording.
                Some(Item::Fn(f))
            }
            // [test-decl] `test "an empty heap pops nothing" { … }` — a test,
            // named by a string literal. Contextual for the same reason
            // `iter fn` is: `test` is an ordinary name everywhere else (a
            // variable, a module, `std.test` itself), and the string literal
            // in the next position is what makes the form unambiguous with no
            // lookahead beyond it.
            TokenKind::Ident(name)
                if name == "test" && matches!(self.peek_at(1).kind, TokenKind::Str(_)) =>
            {
                self.parse_test().map(Item::Test)
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
            // [actor-effect-kind] `actor effect E { … }`: an actor protocol.
            // Contextual — at item level a bare identifier is otherwise a
            // parse error, which is what makes one token of lookahead enough
            // (the `iter fn` precedent).
            TokenKind::Ident(name)
                if name == ACTOR_MODIFIER && matches!(self.peek_at(1).kind, TokenKind::KwEffect) =>
            {
                self.bump();
                self.parse_effect_kinded(false, true).map(Item::Effect)
            }
            // [actor-effect-kind] The word every other language uses for this,
            // and the one this kind was spelled with until 2026-09-16. Worth a
            // diagnostic of its own rather than "expected item": `async` is
            // what an author will reach for, and the rename exists partly to
            // stop implying the colouring `async` carries elsewhere.
            TokenKind::Ident(name)
                if name == "async" && matches!(self.peek_at(1).kind, TokenKind::KwEffect) =>
            {
                let span = self.peek().span;
                self.error(
                    "an asynchronous protocol is declared `actor effect`, not `async \
                     effect`: what runs one is an **actor** — and there is no `async` \
                     anywhere in Salvo, since nothing is coloured",
                    span,
                );
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

    /// [test-decl] `test "an empty heap pops nothing" { … }`.
    ///
    /// The name is a **literal**: the runner enumerates and filters tests
    /// (`salvo test --list`, a substring filter) without running anything, so
    /// an interpolated name has nothing to be known by and is refused here
    /// rather than half-supported.
    fn parse_test(&mut self) -> Option<TestDecl> {
        let docs = self.docs_here();
        let start = self.peek().span;
        self.bump(); // `test`
        let name_token = self.peek().clone();
        let TokenKind::Str(parts) = &name_token.kind else {
            // Unreachable: the caller only dispatches here on a string
            // literal. Kept honest rather than panicking.
            let found = self.kind().describe();
            self.error(
                format!("a test's name must be a string literal, found {found} [test-decl]"),
                name_token.span,
            );
            return None;
        };
        let name = match parts.as_slice() {
            [] => String::new(),
            [StrPart::Text(text)] => text.clone(),
            _ => {
                self.error(
                    "a test's name must be a plain string literal: `salvo test` lists \
                     and filters tests without running them, so an interpolated name \
                     could not be known [test-decl]",
                    name_token.span,
                );
                return None;
            }
        };
        let name_span = name_token.span;
        self.bump(); // the name
        let body = self.parse_block()?;
        Some(TestDecl {
            docs,
            name,
            name_span,
            span: start.to(body.span),
            body,
        })
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
        // [cmp-carry] [qual-value-arg] And value slots in a `(…)` block after
        // the generics, which is how a keyed container names the ordering or
        // hash its keys are kept by:
        // `intrinsic type SortedSet<T>(?cmp: (T, T) -> Int = cmp)`.
        let (generics, generic_canbe, mut fn_slots) = self.parse_generics_slots();
        let (block_slots, value_slots) = self.parse_slot_block();
        fn_slots.extend(block_slots);
        for v in &value_slots {
            // [qual-depend] Value slots belong to qualifiers alone: a type's
            // values carry identities [cmp-carry], not claims about other
            // values.
            self.error(
                "a value slot belongs to a qualifier, not a type: only a \
                 claim can depend on another value",
                v.span,
            );
        }
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            intrinsic,
            linear,
            name,
            generics,
            generic_canbe,
            fn_slots,
            auto_qualifiers,
            alias,
            span: start.to(end),
        })
    }

    /// `<A, B, C>` — declaration-site generic parameters.
    fn parse_generics(&mut self) -> Vec<Ident> {
        let (generics, canbe, slots) = self.parse_generics_all();
        for (ident, _) in &canbe {
            self.error(
                "`canbe` on a type parameter is only supported on functions and structs",
                ident.span,
            );
        }
        for slot in &slots {
            // [cmp-carry] A fn slot belongs to a declaration whose *values*
            // can carry an identity — a qualifier or an intrinsic type. On a
            // fn the binder binds bare in the signature instead (the ordering round
            // decision 11): it is an indirect way of declaring a fn in the
            // parameter scope, so it does not belong in the generics list.
            let (name, span) = slot_name_span(slot);
            self.error(
                format!(
                    "`?{name}` declares a function slot, which only a qualifier or an \
                     `intrinsic type` may have: on a function, write the implicit \
                     parameter (`?{name}: …`) in the parameter list, and use `?{name}` \
                     in the types it applies to"
                ),
                span,
            );
        }
        generics
    }

    /// [cmp-carry] A generics list that may declare **fn slots**:
    /// `<T, ?cmp: (T, T) -> Int = cmp>` — for a qualifier or an
    /// `intrinsic type`.
    fn parse_generics_slots(&mut self) -> (Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<SlotDecl>) {
        self.parse_generics_all()
    }

    /// Type parameters with optional per-parameter `canbe` opt-ins
    /// [linear-generics] [canbe-optin]: `<T canbe linear, U>` — one
    /// qualifier per `canbe` (the comma separates parameters).
    fn parse_generics_canbe(&mut self) -> (Vec<Ident>, Vec<(Ident, TypeRef)>) {
        let (generics, canbe, slots) = self.parse_generics_all();
        for slot in &slots {
            let (name, span) = slot_name_span(slot);
            self.error(
                format!(
                    "`?{name}` declares a function slot, which only a qualifier or an \
                     `intrinsic type` may have [cmp-carry]"
                ),
                span,
            );
        }
        (generics, canbe)
    }

    fn parse_generics_all(&mut self) -> (Vec<Ident>, Vec<(Ident, TypeRef)>, Vec<SlotDecl>) {
        let mut generics = Vec::new();
        let mut canbe = Vec::new();
        let mut slots: Vec<SlotDecl> = Vec::new();
        if self.at(&TokenKind::Lt) {
            self.group_depth += 1;
            self.bump();
            loop {
                if self.eat(&TokenKind::Gt).is_some() || self.at_eof() {
                    break;
                }
                // [qual-value-arg] `?` in a `<…>` list is the old mixed
                // spelling: slots are declared in a `(…)` block after the
                // type generics (user decision 2026-09-23) —
                // `qualifier Sorted<T>(?cmp: (T, T) -> Int) of List<T>`.
                // Parsed as before for recovery; the error stops the build.
                if let Some(q) = self.eat(&TokenKind::Question).map(|t| t.span) {
                    self.error(
                        "a slot is declared in a `(…)` block after the type generics \
                         — `Sorted<T>(?cmp: (T, T) -> Int)` — not in the `<…>` list",
                        q,
                    );
                    if self.at_type_name() {
                        match self.parse_type_ref() {
                            Some(group) => slots.push(SlotDecl::Group(group)),
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
                        continue;
                    }
                    match self.parse_fn_slot(q) {
                        Some(slot) => slots.push(SlotDecl::One(slot)),
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
                    continue;
                }
                match self.ident_type("generic parameter") {
                    Some(id) => {
                        if !slots.is_empty() {
                            // [cmp-carry] Slots trail the type parameters, as
                            // an implicit parameter trails the ordinary ones.
                            self.error(
                                "a type parameter cannot follow a function slot: \
                                 slots come last, so the types they are written \
                                 over are already in scope",
                                id.span,
                            );
                        }
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
        (generics, canbe, slots)
    }

    /// [qual-value-arg] The declaration-side **slot block** (user decision
    /// 2026-09-23): a round-bracket list after the type generics holding the
    /// declaration's value slots — `(?cmp: (T, T) -> Int)`, `(?Ordered<T>)` —
    /// for a qualifier or an `intrinsic type`. A `?`-led entry is an
    /// implicit (a fn slot or a group spread); an unprefixed `name: Type`
    /// entry is a **value slot** [qual-depend]
    /// (`qualifier KeyOf<K, V>(map: Map<K, V>) of K`), which only a
    /// qualifier may declare; constants join in a later step of the
    /// refinement-types sequence.
    fn parse_slot_block(&mut self) -> (Vec<SlotDecl>, Vec<ValueSlot>) {
        let mut slots = Vec::new();
        let mut values = Vec::new();
        if !self.at(&TokenKind::LParen) || !self.same_line() {
            return (slots, values);
        }
        self.group_depth += 1;
        self.bump(); // (
        loop {
            if self.eat(&TokenKind::RParen).is_some() || self.at_eof() {
                break;
            }
            let Some(q) = self.eat(&TokenKind::Question).map(|t| t.span) else {
                // [qual-depend] An unprefixed entry declares a value slot:
                // `map: Map<K, V>`.
                let Some(name) = self.ident_value("slot") else {
                    self.bump();
                    continue;
                };
                if self.expect(&TokenKind::Colon).is_none() {
                    continue;
                }
                let Some(ty) = self.parse_type() else {
                    continue;
                };
                let span = name.span.to(ty.span());
                values.push(ValueSlot { name, ty, span });
                if self.eat(&TokenKind::Comma).is_none() {
                    if self.expect(&TokenKind::RParen).is_none() {
                        break;
                    }
                    break;
                }
                continue;
            };
            // [cmp-carry] `?Ordered<T>` — a group spread, recognised by its
            // casing exactly as a parameter list's is [implicit-group].
            if self.at_type_name() {
                match self.parse_type_ref() {
                    Some(group) => slots.push(SlotDecl::Group(group)),
                    None => {
                        self.bump();
                        continue;
                    }
                }
            } else {
                match self.parse_fn_slot(q) {
                    Some(slot) => slots.push(SlotDecl::One(slot)),
                    None => {
                        self.bump();
                        continue;
                    }
                }
            }
            if self.eat(&TokenKind::Comma).is_none() {
                if self.expect(&TokenKind::RParen).is_none() {
                    break;
                }
                break;
            }
        }
        self.group_depth -= 1;
        (slots, values)
    }

    /// [cmp-carry] One fn slot, after its `?`: `cmp: (T, T) -> Int = cmp`.
    fn parse_fn_slot(&mut self, question: Span) -> Option<FnSlot> {
        let name = self.ident_value("function slot")?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let ty_span = ty.span();
        if !matches!(ty, Type::Fn { .. }) {
            // The same requirement an implicit parameter has, for the same
            // reason: what fills it is a function [implicit-param].
            self.error(
                format!(
                    "a function slot must have a function type, but `{ty}` is not \
                     one: write `?{}: (T, T) -> Int`",
                    name.name
                ),
                ty.span(),
            );
        }
        // [cmp-carry] No `= default`: the slot's **name** is what an unwritten
        // one resolves by, exactly as an implicit parameter's is
        // [implicit-resolve]. Writing one would restate the mechanism, so the
        // form does not exist.
        Some(FnSlot {
            name,
            ty,
            span: question.to(ty_span),
        })
    }

    /// A selector after `@`: a type name (`Person`) or a dotted module path
    /// (`core.list`), kept as one identifier whose name holds the dots.
    fn ident_dotted_path(&mut self) -> Option<Ident> {
        let first = self.ident()?;
        let mut name = first.name.clone();
        let mut span = first.span;
        while self.at(&TokenKind::Dot) {
            self.bump();
            let next = self.ident()?;
            name.push('.');
            name.push_str(&next.name);
            span = span.to(next.span);
        }
        Some(Ident { name, span })
    }

    fn parse_struct(&mut self, linear: bool) -> Option<StructDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwStruct)?.span;
        let name = self.ident_decl_dotted("struct")?;
        // [linear-generics] Structs take per-parameter `canbe` opt-ins too
        // (user decision 2026-09-12): `struct Box<T canbe linear>` is the
        // conditional-container declaration.
        let (generics, generic_canbe) = self.parse_generics_canbe();
        // `: Yield<self, Str>` — obligation groups this type satisfies
        // [group-obligation]. Before `canbe`, because `:` states what the
        // type must *provide* while `canbe` states what it may be qualified
        // as.
        let mut obligations = Vec::new();
        if self.eat(&TokenKind::Colon).is_some() {
            loop {
                // [cmp-auto] `: auto Ordered<self>` — sugar for one bodiless
                // `auto fn` per member of the group. Contextual, like
                // `iter fn` (`auto` is not a reserved word): inside an
                // obligation clause a bare `auto` followed by a capitalized
                // name can only be this.
                let is_auto = self.at_word("auto")
                    && matches!(
                        &self.peek_at(1).kind,
                        TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase())
                    );
                if is_auto {
                    self.bump();
                }
                obligations.push(Obligation {
                    auto: is_auto,
                    group: self.parse_type_ref()?,
                });
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
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
        // [cmp-carry] [qual-value-arg] `qualifier Sorted<T>(?cmp: (T, T) ->
        // Int)`: type generics in `<…>`, value slots in the `(…)` block —
        // which is how a structure *holds* an ordering, and [qual-depend]
        // how a claim names the value it depends on.
        let (generics, canbe, mut fn_slots) = self.parse_generics_slots();
        let (block_slots, value_slots) = self.parse_slot_block();
        fn_slots.extend(block_slots);
        for (ident, _) in &canbe {
            self.error(
                "`canbe` on a type parameter is only supported on functions and structs",
                ident.span,
            );
        }
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            intrinsic,
            subject,
            name,
            generics,
            fn_slots,
            value_slots,
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
    /// plain (exhaustive) name or `Never` is rejected — a refinement
    /// never decides whether a parameter is kept.
    fn parse_refn_deduction_list(&mut self) -> Option<Vec<RefnDeduction>> {
        let mut entries: Vec<RefnDeduction> = Vec::new();
        loop {
            let param = self.ident()?;
            let mut end = param.span;
            let mut add: Vec<TypeRef> = Vec::new();
            let mut remove: Vec<TypeRef> = Vec::new();
            let mut preserve: Vec<TypeRef> = Vec::new();
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
                // [qual-preserve] `preserve Q`: the call does not invalidate
                // the dependent claims other values hold about this
                // parameter. Contextual, like `defer` in a deduction entry.
                if self.at_word("preserve")
                    && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
                {
                    self.bump();
                    while self.at_type_name() {
                        let Some(r) = self.parse_type_ref() else { break };
                        end = r.span;
                        preserve.push(r);
                    }
                    continue;
                }
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
                preserve,
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

    /// [actor-effect-kind] `actor effect E { … }` — an actor protocol. The
    /// modifier is **contextual** (`actor` followed by `effect`), like every
    /// other word this phase added: nothing is reserved, so `actor` stays a
    /// legal name. There is deliberately no `actor fn` — the phase decided
    /// against colouring — so this is the only place the word appears.
    fn parse_effect_kinded(&mut self, platform: bool, is_actor: bool) -> Option<EffectDecl> {
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            platform,
            is_actor,
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
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
        // [effect-handler-multi] One effect or several, comma-separated: the
        // handler wears one face per effect over one piece of state.
        let mut of = vec![self.parse_type()?];
        while self.eat(&TokenKind::Comma).is_some() {
            of.push(self.parse_type()?);
        }
        let mut state = Vec::new();
        let mut fns = Vec::new();
        let mut mailbox = None;
        let mut end = of.last().map(|t| t.span()).unwrap_or(start);
        if self.at(&TokenKind::LBrace) && self.same_line() {
            self.bump();
            while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                // [actor-send-fn] `send fn` is a member too; anything else
                // that is not a `fn` — or the `mailbox` slot — is a state
                // field.
                let is_send_member =
                    self.at_word("send") && matches!(self.peek_at(1).kind, TokenKind::KwFn);
                let is_mailbox = self.at_word(MAILBOX_SLOT)
                    && matches!(self.peek_at(1).kind, TokenKind::LBrace);
                if self.at(&TokenKind::KwFn) || is_send_member {
                    fns.push(self.parse_member_fn()?);
                } else if is_mailbox {
                    // [actor-mailbox] The slot is a struct literal with its
                    // type elided: `mailbox { capacity: 16 }` *is*
                    // `Mailbox { capacity: 16 }`, so every field rule, default
                    // and diagnostic is the struct machinery's. Contextual, so
                    // `mailbox` stays an ordinary field name — a state field
                    // called `mailbox` is `mailbox: T = …`, and only a brace
                    // makes it the slot.
                    let start = self.bump().span;
                    let lit = self.parse_struct_lit_body(Some(Type::Named {
                        qualifiers: Vec::new(),
                        base: TypeRef {
                            alias: None,
                            at: None,
                            binder: false,
                            established: false,
                            value_args: Vec::new(),
                            name: Ident {
                                name: MAILBOX_TYPE.to_string(),
                                span: start,
                            },
                            args: Vec::new(),
                            from: Vec::new(),
                            span: start,
                        },
                    }))?;
                    if mailbox.is_some() {
                        self.error("a handler declares one `mailbox` slot", start);
                    }
                    mailbox = Some(lit);
                } else {
                    state.push(self.parse_field_decl()?);
                    self.eat(&TokenKind::Comma);
                }
            }
            end = self.expect(&TokenKind::RBrace)?.span;
        }
        Some(HandlerDecl {
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            intrinsic,
            platform,
            name,
            generics,
            params,
            effects,
            of,
            mailbox,
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
    /// [actor-send-fn] A member of an effect or a handler, with the
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
        // [cmp-canonical] `fn cmp@Person(…)`: the **canonical** implementation
        // of a capability for a type, `@`-scoped to it. The same selector the
        // language already reads on a *reference* ([fn-overload-at]'s
        // `size@core.list`, [effect-at]'s `close@Fs`), now also written at the
        // declaration — so the declaration spelling and the disambiguation
        // spelling are one token (user decision 2026-09-21).
        //
        // Capitalized only: a type is uppercase [name-casing], and a lowercase
        // name after `@` is a module path everywhere else, which here would be
        // someone reaching for a module-scoping form that does not exist (a fn
        // is already scoped to its module).
        let mut scoped_to = None;
        if self.at(&TokenKind::At) && self.same_line() {
            let at = self.bump().span;
            let target = self.ident()?;
            if !target.name.starts_with(|c: char| c.is_uppercase()) {
                self.error(
                    "`@` on a declaration scopes a function to a *type* \
                     (`fn cmp@Person(…)`), and a type name is capitalized \
                     [name-casing]; a function is already scoped to its module",
                    at,
                );
            }
            scoped_to = Some(target);
        }
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
            // [qual-ctor-fn] `-> +Qualifier T` marks a constructive-qualifier
            // constructor: the qualifier is applied *by construction*, and
            // the `+` is the same establishment marker deductions use
            // [deduce-reapply] (user decision 2026-09-23, replacing the old
            // trailing `-> T as Qualifier`).
            if self.at(&TokenKind::Plus) && self.same_line() {
                self.bump();
                constructs = Some(self.parse_type_ref()?);
            }
            let ty = self.parse_type()?;
            // [proj-anywhere] The derived-return summary the checker and the
            // emitters consume: the parameter named by the *first* `proj` in
            // the return type. `proj(p) T` is an ordinary qualifier
            // on `T`, so the old prefix spelling reads identically.
            derived_return = first_proj_source(&ty);
            return_type = Some(ty);
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
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            intrinsic,
            is_iter,
            is_send,
            iter_state,
            name,
            scoped_to,
            structural: false,
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
                // [actor-spawn-effect] `[spawn]`: the actor-creation
                // capability, lowercase and compiler-owned like `use`.
                // Contextual, so `spawn` stays available as a name.
                TokenKind::Ident(name) if name == "spawn" => {
                    let tok = self.bump();
                    effects.push(EffectRef::Spawn(tok.span));
                }
                // `waitfor` in an effect list was the deleted capability
                // (SH-5(d), user decision 2026-09-19): occupancy is inferred,
                // not declared, so the word here is a plain unknown-effect
                // error like any other — named, because the fix is deletion.
                TokenKind::Ident(name) if name == "waitfor" => {
                    let tok = self.bump();
                    self.error(
                        "`waitfor` is not a declarable effect: occupancy is inferred \
                         (a wait needs no capability since 2026-09-19) — delete it \
                         from this list",
                        tok.span,
                    );
                }
                // [effect-local] `local E` (contextual, user decision
                // 2026-09-20): the requirement that accepts a scope-local
                // binding of `E`, disclaiming seam rights. `local` followed
                // by anything but a type name stays an ordinary effect
                // name, so an effect called `local` (unwise) still parses
                // alone.
                TokenKind::Ident(name)
                    if name == "local"
                        && matches!(self.peek_at(1).kind, TokenKind::Ident(_)) =>
                {
                    self.bump();
                    match self.parse_type_ref() {
                        Some(r) => effects.push(EffectRef::LocalEffect(r)),
                        None => {
                            self.group_depth -= 1;
                            return None;
                        }
                    }
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

    /// Whether the cursor sits on a **contextual modifier that starts the next
    /// declaration** — `export fn`, `iter fn`, `send fn`, `actor effect`.
    ///
    /// These four words are ordinary identifiers [mod-export]
    /// [actor-effect-kind], which makes them indistinguishable from a
    /// qualifier name to any parser loop that runs "while the next token is an
    /// identifier". A trailing qualifier list is exactly such a loop, and a
    /// declaration whose last thing is one — `=> data: Mut` on a bodiless
    /// `intrinsic fn` — has nothing after it to stop at, so the *following*
    /// declaration's modifier got absorbed as a qualifier. Recognizing the
    /// pair by shape, with one token of lookahead, is how the item parser
    /// itself recognizes them.
    fn at_contextual_decl_modifier(&self) -> bool {
        let TokenKind::Ident(name) = &self.kind() else {
            return false;
        };
        let name: &str = name;
        let next = &self.peek_at(1).kind;
        match name {
            EXPORT_MODIFIER => matches!(
                next,
                TokenKind::KwFn
                    | TokenKind::KwStruct
                    | TokenKind::KwQualifier
                    | TokenKind::KwEffect
                    | TokenKind::KwHandler
                    | TokenKind::KwType
                    | TokenKind::KwIntrinsic
                    | TokenKind::KwPlatform
                    | TokenKind::KwProvenance
                    | TokenKind::KwLinear
                    | TokenKind::KwParams
            ) || matches!(next, TokenKind::Ident(w) if w == "iter" || w == "send" || w == ACTOR_MODIFIER),
            "iter" | "send" => matches!(next, TokenKind::KwFn),
            ACTOR_MODIFIER => matches!(next, TokenKind::KwEffect),
            _ => false,
        }
    }

    /// One entry [deduce-syntax]: `!elem`, `elem`, `elem: Qual…`,
    /// `elem: None`, `elem: Never`, `elem: -Qual…`, `elem: +Qual…`,
    /// `x.f: proj(a)`, `.f: proj(a)`, or a bare `proj(a)`.
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
        // [defer-deduction] `defer elem`: consumed, and the obligation may
        // outlive the frame. Contextual — a parameter named `defer` stays
        // writable as a bare keep (`=> defer`), since the form here needs a
        // name after the word.
        if self.at_word("defer")
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(_))
        {
            self.bump();
            let name = self.ident()?;
            return Some(Deduction {
                span: start.to(name.span),
                target: DeductionTarget::Param { name, path: Vec::new() },
                kind: DeductionKind::Deferred,
            });
        }
        // Bare `proj(…)`: opaque.
        if matches!(&self.peek().kind, TokenKind::KwProj) {
            let r = self.parse_type_ref()?;
            if r.from.is_empty() {
                self.error("a bare `proj` deduction needs its sources: `proj(c)`", r.span);
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
            // `: proj(…)`
            if matches!(&self.peek().kind, TokenKind::KwProj) {
                let r = self.parse_type_ref()?;
                if r.from.is_empty() {
                    self.error("a projection entry needs its sources: `proj(c)`", r.span);
                }
                end = r.span;
                DeductionKind::Proj(r.from)
            } else if self.at_word("preserve")
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
            {
                // [qual-preserve] `=> map: preserve KeyOf`: the fn does not
                // invalidate the named dependent claims other values hold
                // about this parameter [qual-depend]. Contextual, like
                // `defer`; a separate entry, so it may accompany the
                // parameter's ordinary one (it is about other values'
                // claims, not this parameter's own list).
                self.bump();
                let mut quals: Vec<TypeRef> = Vec::new();
                while self.at_type_name() {
                    let Some(r) = self.parse_type_ref() else { break };
                    end = r.span;
                    quals.push(r);
                }
                DeductionKind::Preserve(quals)
            } else {
                // Plain names are *exhaustive* (only these survive); `-`-
                // prefixed names are a delta (drop these, keep the rest);
                // `+`-prefixed names are **re-applied** — claims this function
                // establishes afresh, trusted, and only in the qualifier's own
                // file [deduce-reapply]. A delta may not mix with either, since
                // an exhaustive list already drops what it does not name
                // [deduce-syntax].
                let mut plain: Vec<TypeRef> = Vec::new();
                let mut removed: Vec<TypeRef> = Vec::new();
                let mut reapplied: Vec<TypeRef> = Vec::new();
                loop {
                    let sign = if self.at(&TokenKind::Minus) {
                        end = self.bump().span;
                        Some(false)
                    } else if self.at(&TokenKind::Plus) {
                        end = self.bump().span;
                        Some(true)
                    } else {
                        None
                    };
                    if !self.at_ident() || self.at_contextual_decl_modifier() {
                        break;
                    }
                    let Some(r) = self.parse_type_ref() else { break };
                    end = r.span;
                    match sign {
                        Some(false) => removed.push(r),
                        Some(true) => reapplied.push(r),
                        None => plain.push(r),
                    }
                }
                let exhaustive = |quals: Vec<TypeRef>, reapplied: Vec<TypeRef>| {
                    DeductionKind::Exhaustive { quals, reapplied }
                };
                match (
                    plain.is_empty() && reapplied.is_empty(),
                    removed.is_empty(),
                ) {
                    (true, true) => {
                        self.error(
                            "expected qualifiers after `:` — `None` to strip every \
                             qualifier, `Never` to consume the value",
                            end,
                        );
                        exhaustive(Vec::new(), Vec::new())
                    }
                    (false, true) => {
                        let lone = |what: &str| {
                            reapplied.is_empty()
                                && plain.len() == 1
                                && plain[0].name.name == what
                                && plain[0].args.is_empty()
                        };
                        if lone("Never") {
                            DeductionKind::Moved
                        } else if lone("None") {
                            exhaustive(Vec::new(), Vec::new())
                        } else {
                            exhaustive(plain, reapplied)
                        }
                    }
                    (true, false) => DeductionKind::Remove(removed),
                    (false, false) => {
                        self.error(
                            "a deduction entry is either exhaustive (plain \
                             qualifier names, `+Qual` for one this function \
                             re-establishes) or a delta (`-Qual`), not both: \
                             an exhaustive list already drops everything it \
                             does not name",
                            start.to(end),
                        );
                        exhaustive(plain, reapplied)
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
                "a result path (`.field`) can only state a projection: `.field: proj(p)`",
                start.to(end),
            );
        }
        if let DeductionTarget::Param { path, .. } = &target {
            if !path.is_empty() && !matches!(kind, DeductionKind::Proj(_)) {
                self.error(
                    "a parameter's field path can only state a projection: `v.field: proj(p)`",
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
        // [deduce-reapply] A `+` before a qualifier marks it **established**
        // (`-> (+Idx(list) Int)?`): trusted like a constructor's head `+Q`,
        // and validated in return position by the checker — anywhere else a
        // `+` in a type is meaningless and the checker reports it.
        let mut established_first = false;
        if self.at(&TokenKind::Plus)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            self.bump();
            established_first = true;
        }
        let mut first_ref = self.parse_type_ref()?;
        first_ref.established = established_first;
        let mut refs = vec![first_ref];
        loop {
            let established = if self.at(&TokenKind::Plus)
                && self.same_line()
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
            {
                self.bump();
                true
            } else {
                false
            };
            if !established
                && !((self.at_ident()
                    || matches!(
                        self.kind(),
                        TokenKind::KwProj | TokenKind::KwOnce | TokenKind::KwLinear
                    ))
                    && self.same_line())
            {
                break;
            }
            let mut r = self.parse_type_ref()?;
            r.established = established;
            refs.push(r);
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
        // [cmp-carry] `cmp@Person` / `size@core.list` — a function **identity**
        // in a type-argument position, which is what a structure that holds an
        // ordering carries. The selector is what tells an identity from a type
        // without knowing the slot; a bare name is decided by the slot at
        // lowering, and where no slot expects one it stays an ordinary type.
        let mut at = None;
        if self.at(&TokenKind::At) && self.same_line() {
            self.bump();
            let sel = self.ident_dotted_path()?;
            end = sel.span;
            at = Some(sel);
        }
        // [proj-anywhere] `proj(param)`: the borrow's source, written on
        // the obligation itself so it can sit anywhere a type does — a union
        // arm, a type argument, a tuple element — and so a type borrowing
        // from two parameters names each (`proj(a, b)`). Only `proj` reads
        // a paren this way; the sources are value names (lowercase, by
        // [name-casing]), which is what tells them from a qualified group
        // (`proj (Ok Str | Err Int)` — a type inside).
        let mut from = Vec::new();
        if name.name == "proj"
            && self.at(&TokenKind::LParen)
            && self.same_line()
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
            && matches!(self.peek_at(2).kind, TokenKind::Comma | TokenKind::RParen)
        {
            self.bump(); // (
            loop {
                from.push(self.ident()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            end = self.expect(&TokenKind::RParen)?.span;
        }
        if self.at(&TokenKind::Lt) {
            self.group_depth += 1;
            self.bump();
            loop {
                if self.at(&TokenKind::Gt) || self.at_eof() {
                    break;
                }
                // [qual-value-arg] `?` in a `<…>` list is the old mixed
                // spelling: value arguments moved to the round-bracket block
                // (user decision 2026-09-23).
                if let Some(q) = self.eat(&TokenKind::Question).map(|t| t.span) {
                    self.error(
                        "`?` names a value argument, which lives in a `(…)` block \
                         after the type generics: `Sorted(?cmp)`, `Heap<Person>(?cmp)`",
                        q,
                    );
                    let _ = self.ident();
                    if self.eat(&TokenKind::Colon).is_some() {
                        let _ = self.ident();
                    }
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                    continue;
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
        // [qual-value-arg] The value-argument block: `Sorted(?cmp)`,
        // `Heap(min_by_age)`, `SortedSet<Str>(by_len)` — implicit binders and
        // fn identities, after the type generics, in round brackets (user
        // decision 2026-09-23). Recognised by its first entry: `?`, or a
        // value name (lowercase) followed by `,`, `)` or `@` — so a
        // qualified group (`Ok (Ok Str | Err Int)`) and a fn type's named
        // parameter (`(v: Int) -> R`) never match.
        let mut value_args = Vec::new();
        if name.name != "proj"
            && self.at(&TokenKind::LParen)
            && self.same_line()
            && (matches!(self.peek_at(1).kind, TokenKind::Question)
                || (matches!(self.peek_at(1).kind, TokenKind::Int { .. })
                    && matches!(self.peek_at(2).kind, TokenKind::Comma | TokenKind::RParen))
                || (matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
                    && matches!(
                        self.peek_at(2).kind,
                        TokenKind::Comma | TokenKind::RParen | TokenKind::At | TokenKind::Dot
                    )))
        {
            self.group_depth += 1;
            self.bump(); // (
            loop {
                if self.at(&TokenKind::RParen) || self.at_eof() {
                    break;
                }
                // [cmp-carry] `?cmp`: the signature's binder. [cmp-binder]
                // `?cmp: cmp2` — the binder introduced under another name.
                if let Some(q) = self.eat(&TokenKind::Question).map(|t| t.span) {
                    let Some(id) = self.ident_value("function slot") else {
                        self.group_depth -= 1;
                        return None;
                    };
                    let alias = if self.eat(&TokenKind::Colon).is_some() {
                        match self.ident_value("binder alias") {
                            Some(a) => Some(a),
                            None => {
                                self.group_depth -= 1;
                                return None;
                            }
                        }
                    } else {
                        None
                    };
                    let span = q.to(alias.as_ref().map(|a| a.span).unwrap_or(id.span));
                    value_args.push(Type::Named {
                        qualifiers: Vec::new(),
                        base: TypeRef {
                            alias,
                            name: id,
                            args: Vec::new(),
                            value_args: Vec::new(),
                            from: Vec::new(),
                            at: None,
                            binder: true,
                            established: false,
                            span,
                        },
                    });
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                    continue;
                }
                // [qual-const] A **constant** filling a value slot:
                // `InRange(0, 65535) Int`. Carried as a digit-named ref —
                // the AST's type language has no literal node, and the
                // lowering reads the digits back — a recorded shortcut.
                if let TokenKind::Int { value, .. } = self.peek().kind {
                    let span = self.bump().span;
                    value_args.push(Type::Named {
                        qualifiers: Vec::new(),
                        base: TypeRef {
                            alias: None,
                            name: Ident {
                                name: value.to_string(),
                                span,
                            },
                            args: Vec::new(),
                            value_args: Vec::new(),
                            from: Vec::new(),
                            at: None,
                            binder: false,
                            established: false,
                            span,
                        },
                    });
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                    continue;
                }
                // [qual-depend] A **place** filling a value slot:
                // `KeyOf(m)`, `ValidFor(state.data)` — a lowercase name,
                // possibly a field chain. Parsed here because
                // `parse_type` reads a dot after a lowercase name as
                // nothing (types are uppercase [name-casing]).
                if matches!(&self.peek().kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
                    && matches!(self.peek_at(1).kind, TokenKind::Dot)
                {
                    let head = self.ident()?;
                    let mut path = head.name.clone();
                    let mut end_span = head.span;
                    while self.eat(&TokenKind::Dot).is_some() {
                        let part = self.ident()?;
                        path.push('.');
                        path.push_str(&part.name);
                        end_span = part.span;
                    }
                    let span = head.span.to(end_span);
                    value_args.push(Type::Named {
                        qualifiers: Vec::new(),
                        base: TypeRef {
                            alias: None,
                            name: Ident { name: path, span },
                            args: Vec::new(),
                            value_args: Vec::new(),
                            from: Vec::new(),
                            at: None,
                            binder: false,
                            established: false,
                            span,
                        },
                    });
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                    continue;
                }
                let Some(ty) = self.parse_type() else {
                    self.group_depth -= 1;
                    return None;
                };
                value_args.push(ty);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.group_depth -= 1;
            end = self.expect(&TokenKind::RParen)?.span;
        }
        let span = name.span.to(end);
        Some(TypeRef {
            alias: None,
            name,
            args,
            value_args,
            from,
            at,
            binder: false,
            established: false,
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
            TokenKind::KwUse => {
                let start = self.bump().span;
                // [use-local] `use local H(args)` — the scope-local opt-out
                // (user decision 2026-09-20): lock-free, unshareable, the
                // pre-2026-09-20 default. Contextual: `use local` followed
                // by nothing an expression can start with would be a plain
                // parse error either way, and a *binding named* `local` is
                // still reachable as `use (local)`.
                let local = if self.at_word("local")
                    && self.same_line()
                    && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                {
                    self.bump();
                    true
                } else {
                    false
                };
                let handler = self.parse_expr()?;
                // [with-clause] The dependency clause, on the statement as
                // on the spawn (user decision 2026-09-20): the instances to
                // supply instead of the scope's resolution.
                let with_items = self.parse_with_clause()?;
                // [actor-use-addr] `use H(args) on POOL` — the sugar for
                // `let __a = spawn H(args) on POOL` then `use __a` (SH-7,
                // user decision 2026-09-19): the dominant case, one shared
                // instance serving an effect in a scope, in one line. Parsed
                // as a `use` whose handler is a spawn expression, so the
                // spawn machinery and the addr-binding machinery each do
                // their own half. Same-line, like every trailing clause.
                if self.at_word("on") && self.same_line() {
                    self.bump();
                    let pool = self.parse_expr()?;
                    let span = start.to(pool.span());
                    let spawn_span = handler.span().to(pool.span());
                    if local {
                        self.error(
                            "`use local … on POOL` contradicts itself: the `on` clause \
                             spawns a shared servant, and `local` is the scope-local \
                             opt-out — drop one of them",
                            span,
                        );
                    }
                    // The clause belongs to the *spawn* here: the sugar's
                    // instance is the child's, so its dependencies are
                    // supplied where the child is built.
                    let handler = Expr::Spawn {
                        handler: Box::new(handler),
                        with_items,
                        pool: Some(Box::new(pool)),
                        span: spawn_span,
                    };
                    return Some(Stmt::Use {
                        handler,
                        local: false,
                        with_items: Vec::new(),
                        span,
                    });
                }
                let end = with_items
                    .last()
                    .map(|i| i.span())
                    .unwrap_or_else(|| handler.span());
                let span = start.to(end);
                Some(Stmt::Use {
                    handler,
                    local,
                    with_items,
                    span,
                })
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
                // [op-compound] `x += e` is `x = x + e`, desugared here: the
                // arithmetic rules [op-arith], the place rules and the
                // narrowing reset all come from the two forms it is made of,
                // so no checker or emitter knows this spelling exists. Safe to
                // duplicate the target because a Salvo place is an identifier
                // or a field path — there is nothing in one to evaluate twice.
                if let Some(op) = compound_op(self.kind()) {
                    if self.same_line() {
                        let op_span = self.bump().span;
                        let rhs = self.parse_expr()?;
                        let span = expr.span().to(rhs.span());
                        let value = Expr::Binary {
                            op,
                            lhs: Box::new(expr.clone()),
                            rhs: Box::new(rhs),
                            span: op_span.to(span),
                        };
                        return Some(Stmt::Assign {
                            target: expr,
                            value,
                            span,
                        });
                    }
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
        let mut lhs = self.parse_elvis()?;
        loop {
            match self.kind() {
                TokenKind::KwIs if self.same_line() => {
                    self.bump();
                    let (check, binding, lifted) = self.parse_is_check()?;
                    let end = binding
                        .as_ref()
                        .map(|b| b.span)
                        .or_else(|| check.last().map(|r| r.span))
                        .unwrap_or_else(|| lhs.span());
                    let span = lhs.span().to(end);
                    lhs = self.is_or_widen(lhs, check, binding, lifted, span)?;
                }
                // [is-not] `x !is Q` is sugar for `!(x is Q)` (user decision
                // 2026-09-22): one `Expr::Is` under a `Not`, so narrowing,
                // `when` heads and the guard rule all reach it unchanged — the
                // `Not` arm of `analyze_cond` already swaps the two narrow sets
                // [is-narrow-guard]. The postfix tier leaves this `!` alone.
                TokenKind::Bang
                    if self.same_line() && matches!(self.peek_at(1).kind, TokenKind::KwIs) =>
                {
                    let start = self.bump().span;
                    self.bump();
                    let (check, binding, lifted) = self.parse_is_check()?;
                    let end = binding
                        .as_ref()
                        .map(|b| b.span)
                        .or_else(|| check.last().map(|r| r.span))
                        .unwrap_or_else(|| lhs.span());
                    // A negated test narrows nothing in its *then* branch, so
                    // there is no value for a binding to name.
                    if let Some(b) = &binding {
                        self.error(
                            "`!is` binds nothing: the value is only known on the \
                             branch the test *fails*, so there is nothing to \
                             name — drop the binding, or write the positive test \
                             with an `else`",
                            b.span,
                        );
                    }
                    if lifted > 0 {
                        self.error(
                            "`!is ^Q` is not a check: `^` widens the value it \
                             tested [qual-lift], and a negated test produces no \
                             value to widen — write `!(x is ^Q)` if the boolean \
                             is all you want",
                            start.to(end),
                        );
                    }
                    let span = lhs.span().to(end);
                    let inner = self.is_or_widen(lhs, check, binding, lifted, span)?;
                    lhs = Expr::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(inner),
                        span: start.to(end),
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

    /// [qual-lift] Classifies a parsed `is` check: every term `^`-marked is a
    /// **widening** check (the qualifiers are tested and removed), none marked
    /// is an ordinary narrowing `is`, and a mix is refused — the two say
    /// opposite things about the same check, and no existing form needed it
    /// (the standalone `^ Q` operator this replaces took qualifiers only).
    fn is_or_widen(
        &mut self,
        subject: Expr,
        check: Vec<TypeRef>,
        binding: Option<Ident>,
        lifted: usize,
        span: Span,
    ) -> Option<Expr> {
        if lifted == 0 {
            return Some(Expr::Is {
                subject: Box::new(subject),
                check,
                binding,
                span,
            });
        }
        if lifted != check.len() {
            self.error(
                "an `is` check either lifts every qualifier or none: mark them \
                 all with `^` (`is ^Mut ^NonEmpty`), or split the check",
                span,
            );
        }
        Some(Expr::Widen {
            subject: Box::new(subject),
            quals: check,
            binding,
            span,
        })
    }

    /// The type-ref sequence after `is`, with an optional trailing binding.
    /// Type names are uppercase by convention; a trailing lowercase
    /// identifier is a binding: `is Str s`, `is Err Str`, `is NonEmpty`.
    fn parse_is_check(&mut self) -> Option<(Vec<TypeRef>, Option<Ident>, usize)> {
        let mut refs = Vec::new();
        let mut binding = None;
        let mut lifted = 0usize;
        loop {
            if !self.same_line() {
                break;
            }
            // [qual-lift] `is ^Ok` — the qualifier is tested *and removed*
            // (user decision 2026-09-21, superseding the standalone `^ Ok`
            // operator). The mark is on the qualifier, not on the check, so a
            // reader learns one rule: `^Q` is "Q, lifted".
            if matches!(self.kind(), TokenKind::Caret) {
                self.bump();
                lifted += 1;
                refs.push(self.parse_type_ref()?);
                continue;
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
        Some((refs, binding, lifted))
    }

    /// [elvis] `subject ?: rhs`, **right-associative**, sitting where Kotlin's
    /// elvis sits: tighter than `is`/comparison/equality, looser than
    /// additive. So `m[k] ?: 0 > 3` is `(m[k] ?: 0) > 3` and `a ?: b + 1` is
    /// `a ?: (b + 1)`, and a reader who knows Kotlin's `?:` does not have to
    /// learn a different binding.
    ///
    /// The right side is an ordinary expression — which since step 2 includes
    /// `return`/`break`/`continue`, so `x ?: return _` needs no exception
    /// [expr-escape].
    fn parse_elvis(&mut self) -> Option<Expr> {
        let lhs = self.parse_additive()?;
        if !self.same_line() {
            return Some(lhs);
        }
        // [pick] A qualifier may be named before the operator: `x Ok?: r`,
        // `x ^Ok?: r`. Recognised by scanning ahead for the `?:` — an
        // expression is never followed by a bare type name otherwise, and the
        // two-token lookahead keeps that unambiguous.
        let pick = self.peek_elvis_pick();
        if pick.is_none() && !matches!(self.kind(), TokenKind::QuestionColon) {
            return Some(lhs);
        }
        let pick = match pick {
            Some(()) => {
                let start = self.peek().span;
                let lift = self.eat(&TokenKind::Caret).is_some();
                let mut quals = vec![self.parse_type_ref()?];
                let mut end = quals[0].span;
                // Further qualifiers, each carrying its own `^` when lifted.
                while !matches!(self.kind(), TokenKind::QuestionColon) {
                    let marked = self.eat(&TokenKind::Caret).is_some();
                    if marked != lift {
                        self.error(
                            "a pick either lifts every qualifier or none: mark \
                             them all with `^`, or split the pick",
                            self.peek().span,
                        );
                    }
                    let r = self.parse_type_ref()?;
                    end = r.span;
                    quals.push(r);
                }
                Some(ElvisPick {
                    quals,
                    lift,
                    span: start.to(end),
                })
            }
            None => None,
        };
        self.expect(&TokenKind::QuestionColon)?;
        let rhs = self.parse_elvis()?;
        let span = lhs.span().to(rhs.span());
        Some(Expr::Elvis {
            subject: Box::new(lhs),
            pick,
            rhs: Box::new(rhs),
            span,
        })
    }

    /// [pick] Whether a qualifier pick starts here: `[^]Name+` followed by
    /// `?:`, each name optionally carrying a value-argument block
    /// (`Idx(heap)?:` [qual-depend]) or type arguments. Pure lookahead —
    /// nothing is consumed.
    fn peek_elvis_pick(&self) -> Option<()> {
        let mut i = 0usize;
        if matches!(self.peek_at(i).kind, TokenKind::Caret) {
            i += 1;
        }
        let mut names = 0usize;
        loop {
            match &self.peek_at(i).kind {
                TokenKind::Ident(n) if n.chars().next().is_some_and(|c| c.is_uppercase()) => {
                    i += 1;
                    names += 1;
                    // `<…>` type arguments and `(…)` value arguments ride
                    // with the name; skip each balanced group.
                    for (open, close) in
                        [(TokenKind::Lt, TokenKind::Gt), (TokenKind::LParen, TokenKind::RParen)]
                    {
                        if self.peek_at(i).kind == open {
                            let mut depth = 0usize;
                            loop {
                                let k = &self.peek_at(i).kind;
                                if *k == open {
                                    depth += 1;
                                } else if *k == close {
                                    depth -= 1;
                                    if depth == 0 {
                                        i += 1;
                                        break;
                                    }
                                } else if matches!(k, TokenKind::Eof) {
                                    return None;
                                }
                                i += 1;
                            }
                        }
                    }
                }
                TokenKind::Caret if names > 0 => i += 1,
                _ => break,
            }
        }
        if names == 0 {
            return None;
        }
        matches!(self.peek_at(i).kind, TokenKind::QuestionColon).then_some(())
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
                // [safe-call] `?.` reads a field or calls a dot-notation
                // function on the non-`None` side. Same postfix tier as `.`, so
                // it chains with everything else.
                TokenKind::QuestionDot if self.same_line() => {
                    self.bump();
                    let field = self.ident()?;
                    let (args, end) = if self.at(&TokenKind::LParen) {
                        let (args, named, end) = self.parse_call_args()?;
                        if let Some(first) = named.first() {
                            self.error(
                                "a `?.` call takes its arguments positionally",
                                first.span,
                            );
                        }
                        (Some(args), end)
                    } else {
                        (None, field.span)
                    };
                    let span = expr.span().to(end);
                    let base = expr.clone();
                    let access = Expr::Field {
                        base: Box::new(expr),
                        field,
                        span,
                    };
                    let inner = match args {
                        Some(args) => Expr::Call {
                            callee: Box::new(access),
                            type_args: Vec::new(),
                            args,
                            named: Vec::new(),
                            span,
                        },
                        None => access,
                    };
                    expr = Expr::SafeField {
                        base: Box::new(base),
                        inner: Box::new(inner),
                        span,
                    };
                }
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
                    // [actor-self-send] `k@self(args)`: the enclosing
                    // *handler*'s member — a message to this actor. One
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
                // [is-not] `!` directly before `is` is the **negated check**,
                // not an assert: `s !is Str` used to parse as `(s!) is Str`,
                // which type-checked and meant the opposite of what it reads
                // like (the assert makes the value present, then the test
                // passes) — so the spelling the demo's author reached for
                // silently inverted the guard. The comparison tier takes it.
                TokenKind::Bang
                    if self.same_line()
                        && !matches!(self.peek_at(1).kind, TokenKind::KwIs) =>
                {
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
            // [placeholder] `_` is the value the enclosing construct left
            // unnamed — today, the `None` side inside an `?:` right-hand side.
            // A contextual keyword rather than a lexer token: `_` lexes as an
            // identifier, it appears in no `.sv` source, and it cannot be
            // *declared* (the checker refuses that), so reading it here is
            // never ambiguous with a variable somebody named `_`.
            TokenKind::Ident(name) if name == "_" => {
                let span = self.bump().span;
                Some(Expr::Placeholder { span })
            }
            // [assert-fn] `assert!(cond, "why")` and `unreachable!("why")`: the
            // two assertion forms, spelled with a `!` because a bang in Salvo
            // marks a place that can fail (user decision 2026-09-23).
            // Contextual, like every other keyword-shaped name here — a
            // variable may still be called `assert`; only `assert!(` is the
            // form, which no other reading of those three tokens has (a
            // non-null assertion on a *name* followed by a call would be
            // `assert!` applied to a fn value, which is not callable).
            TokenKind::Ident(name)
                if (name == ASSERT_FORM || name == UNREACHABLE_FORM)
                    && matches!(self.peek_at(1).kind, TokenKind::Bang)
                    && matches!(self.peek_at(2).kind, TokenKind::LParen) =>
            {
                let assert = name == ASSERT_FORM;
                let start = self.bump().span;
                self.bump(); // `!`
                self.expect(&TokenKind::LParen)?;
                let saved_depth = self.group_depth;
                self.group_depth += 1;
                let cond = if assert {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                let message = if assert {
                    if self.eat(&TokenKind::Comma).is_some() {
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    }
                } else if self.at(&TokenKind::RParen) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                let end = self.expect(&TokenKind::RParen)?.span;
                self.group_depth = saved_depth;
                let span = start.to(end);
                Some(match cond {
                    Some(cond) => Expr::Assert {
                        cond,
                        message,
                        span,
                    },
                    None => Expr::Unreachable { message, span },
                })
            }
            // [expr-escape] The three escapes are **expressions** of type
            // `Never` (user decision 2026-09-21), so a tail position can hold
            // one with no grammar exception — which is what `?:`'s right side
            // needs. A value follows only on the same line and only when
            // something can start one, so `return` alone before a `}` or a
            // newline is still the value-less form.
            TokenKind::KwReturn | TokenKind::KwBreak => {
                let is_break = matches!(self.kind(), TokenKind::KwBreak);
                let start = self.bump().span;
                let value = if self.stmt_value_follows() {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                let end = value.as_ref().map(|e| e.span()).unwrap_or(start);
                let span = start.to(end);
                Some(if is_break {
                    Expr::Break { value, span }
                } else {
                    Expr::Return { value, span }
                })
            }
            TokenKind::KwContinue => {
                let span = self.bump().span;
                Some(Expr::Continue { span })
            }
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
                // [actor-self-send] The old self-send spelling. `self.k(…)`
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
                // [actor-spawn-expr] `spawn H(...)` — a *name* follows.
                if self.at_word("spawn") && matches!(self.peek_at(1).kind, TokenKind::Ident(_)) {
                    return self.parse_spawn();
                }
                // [actor-replyto] `replyto k(...)` / `replyto! k(...)`.
                if self.at_word("replyto")
                    && (matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                        || (matches!(self.peek_at(1).kind, TokenKind::Bang)
                            && matches!(self.peek_at(2).kind, TokenKind::Ident(_))))
                {
                    return self.parse_replyto();
                }
                // [actor-waitfor] [waitfor-infer] `waitfor out { … }`, or
                // `waitfor out: Reply<T> { … }` with the type written. A name
                // followed by a `:` or a `{` is the tell, and neither can be a
                // call of a fn named `waitfor`: an argument list opens with `(`.
                if self.at_word("waitfor")
                    && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                    && matches!(
                        self.peek_at(2).kind,
                        TokenKind::Colon | TokenKind::LBrace
                    )
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

    /// [actor-spawn-expr] `spawn H(args) with D1(...), addr on POOL` — the
    /// asynchronous binding of a handler. Both clauses are optional: a
    /// handler with no dependencies needs no `use`, and an omitted `on` means
    /// the pool current at the spawn [main-pool] — which in `main` is the
    /// main pool. The mailbox bound is the *handler's* [actor-mailbox].
    ///
    /// No clause takes a `{ ... }` body, so struct-literal speculation stays
    /// on throughout: a constructor argument may be a struct literal like
    /// any other argument.
    fn parse_spawn(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `spawn`
        let handler = self.parse_expr()?;
        let with_items = self.parse_with_clause()?;
        let mut end = handler.span();
        let mut pool = None;
        if self.at_word("on") && self.same_line() {
            self.bump();
            let expr = self.parse_expr()?;
            end = expr.span();
            pool = Some(Box::new(expr));
        } else if let Some(last) = with_items.last() {
            end = last.span();
        }
        Some(Expr::Spawn {
            handler: Box::new(handler),
            with_items,
            pool,
            span: start.to(end),
        })
    }

    /// [with-clause] The `with` clause of a `use` or `spawn`: the dependency
    /// instances to supply instead of the scope's resolution
    /// ([spawn-inherit]), as a comma-separated list of handler constructions
    /// and `Addr` values — the parser keeps both as expressions, as the
    /// `use` *statement* does, and the checker tells them apart.
    ///
    /// Same line as its binding form, like the `on` clause: without the
    /// guard a bare spawn followed by a `use` *statement* swallowed the next
    /// line as its clause (the defect that retired the clause's original
    /// `use` spelling — 2026-09-19, and the reason the word is now `with`,
    /// user decision 2026-09-20). `with` is a keyword already — the
    /// qualifier-compatibility clause uses it (`qualifier Q of T with A`) —
    /// and the two positions cannot be confused: one follows a qualifier's
    /// `of` type, the other a `use`/`spawn` handler expression.
    fn parse_with_clause(&mut self) -> Option<Vec<Expr>> {
        let mut items = Vec::new();
        // The clause's old spelling is a plain parse error naming the new
        // one (no dual-accepting grammar — the standing invariant).
        if self.at(&TokenKind::KwUse) && self.same_line() {
            let span = self.peek().span;
            self.error(
                "the dependency clause of a `use`/`spawn` is spelled `with` now \
                 (`spawn H(args) with D(), addr on POOL`): `use` is the statement's \
                 word",
                span,
            );
            self.bump();
            loop {
                items.push(self.parse_expr()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            return Some(items);
        }
        if self.at(&TokenKind::KwWith) && self.same_line() {
            self.bump();
            loop {
                items.push(self.parse_expr()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        Some(items)
    }

    /// [actor-replyto] `replyto k(captures)` — mint a parked one-shot
    /// continuation targeting member `k` of the enclosing handler, yielding
    /// its linear `Reply<T>`. `replyto! k(captures)` is the gated mint: the
    /// actor serves nothing else until the answer arrives.
    ///
    /// [task-pool-inherit] A trailing `on POOL` names where the continuation
    /// runs, and is optional: omitted, it runs on the pool current at the
    /// mint. Same line as the mint, so a following statement that happens to
    /// call a function named `on` cannot be swallowed.
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
        // [task-pool-inherit] The optional placement clause.
        let mut end = end;
        let mut pool = None;
        if self.at_word("on") && self.same_line() {
            self.bump();
            let expr = self.parse_expr()?;
            end = expr.span();
            pool = Some(Box::new(expr));
        }
        Some(Expr::ReplyTo {
            member,
            captures,
            gated,
            pool,
            span: start.to(end),
        })
    }

    /// [actor-waitfor] `waitfor out: Reply<T> { ... }` — `main`'s bridge
    /// into the asynchronous world. The binder's type is written out, since
    /// nothing else in the block says what answer is being waited for.
    fn parse_waitfor(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `waitfor`
        let binding = self.ident()?;
        // [waitfor-infer] The type is optional: `waitfor out { p.total(out) }`
        // reads it off the send the block makes (user decision 2026-09-21).
        let ty = if self.eat(&TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        if !self.at(&TokenKind::LBrace) {
            let span = self.peek().span;
            self.error(
                "`waitfor` takes a binder and a block that sends the token \
                 somewhere: `waitfor out { p.total(out) }`, or \
                 `waitfor out: Reply<Int> { … }` to write the type out",
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
            // [qual-lift] A branch head is always `is`; `^` on its qualifiers
            // is what makes it a widening branch (user decision 2026-09-21).
            let head_span = self.expect(&TokenKind::KwIs)?.span;
            let (check, binding, lifted) = self.parse_is_check()?;
            let widen = lifted > 0;
            if widen && lifted != check.len() {
                self.error(
                    "a branch head either lifts every qualifier or none: mark \
                     them all with `^`, or split the branch",
                    head_span,
                );
            }
            let body = self.parse_block()?;
            let span = head_span.to(body.span);
            branches.push(WhenBranch {
                check,
                binding,
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
    /// [actor-send-fn] `send fn`: an asynchronous member of an actor
    /// protocol — legal only inside an effect or a handler.
    Send,
}

/// [proj-anywhere] The source parameter of the first `proj(p)` in a
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

/// [cmp-carry] How a slot-list entry reads back in a diagnostic: the slot's own
/// name, or the group's.
fn slot_name_span(slot: &SlotDecl) -> (String, Span) {
    match slot {
        SlotDecl::One(s) => (s.name.name.clone(), s.span),
        SlotDecl::Group(g) => (g.name.name.clone(), g.span),
    }
}

/// [op-compound] The arithmetic operator a compound assignment carries, or
/// `None` for anything else. Four of them, matching the four arithmetic
/// operators that take two operands: `%=` is deliberately absent, since a
/// remainder-in-place has no reading a reader would guess.
fn compound_op(kind: &TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::PlusEq => Some(BinaryOp::Add),
        TokenKind::MinusEq => Some(BinaryOp::Sub),
        TokenKind::StarEq => Some(BinaryOp::Mul),
        TokenKind::SlashEq => Some(BinaryOp::Div),
        _ => None,
    }
}
