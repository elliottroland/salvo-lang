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
    /// [implicit-with] Entries a **plural** `with` subject desugared to
    /// (one entry per subject), taken by the clause loop after the entry
    /// that produced them.
    pending_deductions: Vec<Deduction>,
    /// Depth of explicit grouping (parens/brackets); newlines are ignored
    /// inside groups.
    group_depth: u32,
    /// When true, do not speculate struct literals / trailing braces
    /// (condition position).
    no_struct: bool,
    /// [comptime-inline] Inside a `compfn` body: the comptime syntax (`inline`,
    /// `refuse`, `v.[field]`, `[field]:`, `T.fields`, `[field.type]` in a type
    /// position) is legal. Anywhere else each form is a parse error naming the
    /// scope it belongs to.
    in_compfn: bool,
    /// [comptime-pattern] Inside the type after a comptime `is`: `*` is a
    /// wildcard type there, and nowhere else.
    in_comp_pattern: bool,
    /// [terminal-scope] True while parsing a statement's own expression at the
    /// top of a block, where a body-taking construct (`if`/`while`/`for`/
    /// `try`/`waitfor`) may write its body as a terminal `...` that runs to the
    /// end of the enclosing block instead of a braced block. Cleared the moment
    /// parsing descends into any sub-expression (a condition, an operand, a
    /// `let` value, a call argument, a parenthesised group), so `...` is a body
    /// only for the construct that *is* the statement.
    allow_terminal: bool,
    /// [terminal-lambda] True while parsing a statement's own expression tree —
    /// including through a `return`/`let`/assignment value and call arguments,
    /// where `allow_terminal` is cleared — but false inside a nested block or a
    /// lambda body. The gate for a **trailing-lambda terminal scope**
    /// (`map(xs, (e) -> ...)`): the last argument's lambda body may be written
    /// `...` and becomes the rest of the enclosing block.
    in_stmt_expr: bool,
    /// [terminal-lambda] Set when a trailing `(params) -> ...` lambda has been
    /// parsed in the current statement; its body is filled with the rest of the
    /// block once the statement closes. At most one per statement.
    pending_terminal_lambda: bool,
    /// [elvis-wrap] Suppresses the forward `?:`'s line-wrap: set while parsing
    /// a nested right side (a `return`/escape value) so a next-line `?:` is
    /// left for the *enclosing* elvis chain to pick up, bracketing everything
    /// before the newline on the left (user decision 2026-10-12).
    elvis_no_wrap: bool,
}

struct Snapshot {
    pos: usize,
    diag_len: usize,
}

/// [expr-infix] What the current token is, as an infix operator, for the
/// precedence-climbing loop (`parse_binary`). Paired with a left binding power
/// by `infix_bp`.
#[derive(Clone, Copy)]
enum InfixKind {
    /// A plain `lhs OP rhs` binary that builds `Expr::Binary`.
    Binary(BinaryOp),
    /// `is` — a type/qualifier check (`Expr::Is`/`Expr::Widen`).
    Is,
    /// `!is` — the negated check, sugar for `!(x is Q)`.
    IsNot,
    /// The positional-pick elvis (`?…?:` / `:?…?`), handled by
    /// `parse_elvis_tail`.
    Elvis,
    /// [expr-pipe] `|>` — the pipe, handled by `parse_pipe_tail`.
    Pipe,
    /// [expr-safe-pipe] `?>` — the null-safe pipe, handled by
    /// `parse_safe_pipe_tail`.
    SafePipe,
}

/// [actor-self-send] The contextual selector that names the enclosing handler:
/// `k@self(args)`. Contextual, not reserved — `self` stays an ordinary name
/// everywhere else, and it is only special immediately after `@`.
pub const SELF_SELECTOR: &str = "self";
/// [handler-init] The contextual name of a handler's `init { … }` block.
pub const INIT_BLOCK: &str = "init";

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

/// [assert-fn] `todo!()` / `todo!("why")`: a place not yet implemented. Like
/// `unreachable!()` it is type `Never` and traps at runtime, but it also
/// **warns at every site** — the compiler flags unfinished code during a
/// build. Contextual, like the other two.
pub const TODO_FORM: &str = "todo";

/// [actor-effect-kind] The contextual modifier that makes an effect an actor
/// protocol: `actor effect E { … }`. Contextual, not reserved — and the only
/// place the word appears in the language, since the phase decided against
/// colouring functions.
pub const ACTOR_MODIFIER: &str = "actor";

/// [threadsafe-platform] The contextual modifier that states a platform
/// handler's thread-safety contract: `threadsafe platform handler H of E`
/// (user decision 2026-09-26). The host class may be entered concurrently
/// from any thread; without it the compiler serializes the instance on both
/// backends. Contextual like `actor` — an ordinary identifier everywhere
/// else — and the only word it may precede is `platform`.
pub const THREADSAFE_MODIFIER: &str = "threadsafe";

/// [platform-iterable] The contextual modifier declaring that a platform
/// type's host loops natively: `iterable platform type List<T> : Iter<self,
/// T>` (user decision 2026-10-05). Only `platform type` may follow it.
pub const ITERABLE_MODIFIER: &str = "iterable";

/// [noremote] The contextual modifier that keeps a type off the wire:
/// `noremote struct Canvas { … }`, `noremote intrinsic type Pool` (user
/// decision 2026-09-26). Every type is serializable by default; this is the
/// opt-out, and it is transitive — a struct holding a `noremote` field is
/// `noremote` itself. Contextual like the others: an ordinary identifier
/// everywhere but directly before a type-declaring keyword.
pub const NOREMOTE_MODIFIER: &str = "noremote";

/// [struct-opaque] The contextual modifier that keeps a struct's fields in its
/// module: `opaque struct Map<K, V> { … }`.
pub const OPAQUE_MODIFIER: &str = "opaque";

/// [comptime-bound] [comptime-fields] The contextual modifier of the whole
/// compile-time facility: `comptime fn cmp<T is Struct>(…)`, `comptime struct
/// Field { … }`, `comptime type Type = …` (user decision 2026-09-29, replacing
/// the standalone `compfn`). Contextual like the other declaration words: at
/// item level, and inside a struct body, a bare identifier is otherwise a
/// parse error.
pub const COMPTIME_WORD: &str = "comptime";

/// [namespace] The contextual word opening a `namespace X { … }` grouping. A
/// **contextual** keyword, not reserved: it is read as a keyword only at item
/// position when a capitalized name follows (`namespace AttributeValue`),
/// which is what lets `namespace` stay an ordinary identifier (a variable, a
/// field, a call) everywhere else — talking *about* namespaces is common.
pub const NAMESPACE_WORD: &str = "namespace";

/// [effect-alias] The contextual word opening an `effects IO = [...]` group.
/// Contextual like `namespace`: read as a keyword only at item position before
/// a capitalized name, so `effects` stays an ordinary identifier.
pub const EFFECTS_WORD: &str = "effects";

/// [eff-handle] The internal nominal type an `eff[…]` handle lowers to — the
/// name `Addr<E>` used before the surface rename. Kept as the internal
/// spelling so the compiler's actor/handle machinery (keyed on this name) is
/// untouched by the surface change; only the written syntax moved to `eff[…]`.
pub const ADDR_TYPE_NAME: &str = "Addr";

/// [eff-handle] The contextual type former `eff[A, B, …]`: the type of a
/// first-class handle over a flat set of effects (what `Addr<E>` was spelled
/// before — the mailbox analogy no longer fit once a handle covers any effect,
/// actor or not). Lowercase and compiler-owned like `proj`/`iter`/`ref`, read
/// as the former only before `[`, so `eff` stays an ordinary identifier.
pub const EFF_WORD: &str = "eff";

/// [obligation-by] [fn-by] The contextual word naming where a body is stamped
/// from: `: Ordered<self> by auto`, `fn cmp(a: P, b: P) -> Int by auto`.
/// Contextual — `by` is an ordinary value name elsewhere, and both positions
/// (after an obligation's type, after a fn signature on the same line) are
/// otherwise the end of the declaration.
pub const BY_WORD: &str = "by";

/// [comptime-refuse] `refuse!("…")`: an error at the instantiation site, with
/// the bang and parens of `assert!`/`unreachable!` [assert-fn].
pub const REFUSE_FORM: &str = "refuse";

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
            pending_deductions: Vec::new(),
            group_depth: 0,
            no_struct: false,
            in_compfn: false,
            in_comp_pattern: false,
            allow_terminal: false,
            in_stmt_expr: false,
            pending_terminal_lambda: false,
            elvis_no_wrap: false,
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

    /// [doc-module] The module's own documentation: the **first** comment run
    /// in the file — leading blank lines allowed — provided it documents no
    /// declaration. It is the module's when a blank line follows it, or when
    /// the file's first token is `import` on the very next line (an import
    /// carries no docs, so the run cannot be anyone else's). A run sitting
    /// directly above a declaration is that declaration's [doc-comment], and
    /// the module then has no docs of its own (user decision 2026-09-29,
    /// option (b): the header-then-imports layout needs no blank line).
    fn module_docs(&self) -> Vec<String> {
        // The first line holding code; every comment before it is a
        // candidate. A file of nothing but comments has no such line.
        let first_code = self.tokens.first().filter(|t| !matches!(t.kind, TokenKind::Eof))
            .map(|t| self.line_of(t.span.start));
        let limit = first_code.unwrap_or(u32::MAX);
        let Some(start) = (0..limit).find(|line| self.comments.contains_key(line)) else {
            return Vec::new();
        };
        let mut end = start;
        while end + 1 < limit && self.comments.contains_key(&(end + 1)) {
            end += 1;
        }
        let documents_code = first_code.is_some_and(|code| end + 1 == code);
        if documents_code
            && !matches!(self.tokens.first().map(|t| &t.kind), Some(TokenKind::KwImport))
        {
            return Vec::new();
        }
        (start..=end).map(|line| self.comments[&line].clone()).collect()
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
        // [doc-module] Read before any item is parsed: the rule looks at the
        // file's first token, which is still where the parser stands.
        let docs = self.module_docs();
        let mut items = Vec::new();
        let mut uses = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            // [mod-use] A module-level `use H()`; whether this module may
            // have one is the checker's call.
            if self.at(&TokenKind::KwUse) {
                let start = self.bump().span;
                if let Some(mut handler) = self.parse_expr() {
                    let mut span = start.to(handler.span());
                    // `use H() on POOL`, the spawn sugar a `use` statement has.
                    if self.at_word("on") && self.same_line() {
                        self.bump();
                        if let Some(pool) = self.parse_expr() {
                            span = start.to(pool.span());
                            let spawn_span = handler.span().to(pool.span());
                            handler = Expr::Spawn {
                                handler: Box::new(handler),
                                with_items: Vec::new(),
                                pool: Some(Box::new(pool)),
                                join: None,
                                span: spawn_span,
                            };
                        }
                    }
                    uses.push(ModuleUse { handler, span });
                } else {
                    self.recover_to_item_start();
                }
                continue;
            }
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.recover_to_item_start(),
            }
            if self.pos == before {
                // Safety net: never loop without progress.
                self.bump();
            }
        }
        Module { docs, items, uses, namespaces: Vec::new() }
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
            Item::Namespace(mut d) => {
                // [namespace] [name-dot] `export namespace X` exports every
                // member (D6); the flag is distributed when the body is
                // flattened.
                d.exported = true;
                Some(Item::Namespace(d))
            }
            Item::EffectAlias(mut d) => {
                d.exported = true;
                Some(Item::EffectAlias(d))
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

    /// [namespace] `namespace X { items }`, or the terminal form
    /// `namespace X ...` whose body runs to the end of the enclosing item list
    /// [terminal-scope]. The caller is positioned on `namespace`.
    ///
    /// Namespaces nest, so the body is parsed with the same `parse_item` loop
    /// the module uses. A braced body ends at its `}`; a terminal body ends at
    /// EOF or the `}` of an enclosing braced namespace — nothing may follow a
    /// terminal scope at the same level.
    fn parse_namespace(&mut self) -> Option<NamespaceDecl> {
        let docs = self.docs_here();
        let start = self.bump().span; // `namespace`
        let name = self.ident_type("namespace")?;
        let terminal = self.at(&TokenKind::Ellipsis);
        if terminal {
            self.bump();
        } else if !self.at(&TokenKind::LBrace) {
            let found = self.kind().describe();
            let span = self.peek().span;
            self.error(
                format!(
                    "expected `{{` or `...` after the namespace name `{}`, found {found} \
                     [namespace]",
                    name.name
                ),
                span,
            );
            return None;
        } else {
            self.bump(); // `{`
        }
        let mut items = Vec::new();
        loop {
            if terminal {
                // Runs to the end of the enclosing item list: EOF at the top
                // level, or the `}` that closes an enclosing braced namespace.
                if self.at_eof() || self.at(&TokenKind::RBrace) {
                    break;
                }
            } else if self.at(&TokenKind::RBrace) {
                self.bump();
                break;
            } else if self.at_eof() {
                self.error(
                    format!("unterminated `namespace {}`: expected `}}` [namespace]", name.name),
                    start,
                );
                break;
            }
            let before = self.pos;
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.recover_to_item_start(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        let end = self.tokens[self.pos.saturating_sub(1)].span;
        let span = start.to(end);
        Some(NamespaceDecl {
            docs,
            exported: false,
            name,
            items,
            terminal,
            span,
        })
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
                             (`type {} = ...`)",
                            t.name.name, t.name.name
                        ),
                        t.span,
                    );
                }
                Some(Item::Type(t))
            }
            // [effect-alias] `effects IO = [Console, Fs, Streams]` — a named
            // effect group. A **contextual** keyword, recognised only before a
            // capitalized name (and an `=` follows), so `effects` stays an
            // ordinary identifier.
            TokenKind::Ident(name)
                if name == EFFECTS_WORD
                    && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase())) =>
            {
                self.parse_effect_alias().map(Item::EffectAlias)
            }
            // [namespace] `namespace X { … }` or `namespace X ...` — a
            // grouping of declarations, flattened to dotted names before
            // resolution [name-dot]. A **contextual** keyword: recognised only
            // when a capitalized name follows, so `namespace` stays an
            // ordinary identifier (a variable, a call) everywhere else. At item
            // level a bare identifier is otherwise a parse error, which is what
            // makes this unambiguous with one token of lookahead.
            TokenKind::Ident(name)
                if name == NAMESPACE_WORD
                    && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase())) =>
            {
                self.parse_namespace().map(Item::Namespace)
            }
            // [iter-fn] `iter fn range(start: Int, end: Int) -> Emitted T |
            // Finished` — a hand-written `next` whose iterator struct is
            // generated; the declaration is the minter.
            //
            // A **contextual** keyword, and it has to be: `iter` is also the name
            // of every container's minter and of the `iter T` type former
            // [iter-type], so it must stay callable and declarable. At item level
            // a bare identifier is otherwise a parse error, which is what makes
            // `iter fn` unambiguous with no lookahead beyond the next token.
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
            // [comptime-bound] `compfn cmp<struct T>(a: T, b: T) -> Int { … }` — a
            // compile-time function, the one scope the comptime syntax is legal
            // in. Contextual for the reason `iter fn` is: at item level a bare
            // identifier is otherwise a parse error, and `compfn` is not a
            // reserved word (a value may be called that).
            // [comptime-bound] [comptime-fields] `comptime fn`, `comptime struct`,
            // `comptime type`: the compile-time facility's one modifier.
            TokenKind::Ident(name) if name == COMPTIME_WORD => {
                let span = self.peek().span;
                match self.peek_at(1).kind {
                    TokenKind::KwFn => self.parse_compfn().map(Item::Fn),
                    TokenKind::KwStruct => {
                        self.bump();
                        let mut s = self.parse_struct(false)?;
                        s.comptime = true;
                        Some(Item::Struct(s))
                    }
                    TokenKind::KwType => {
                        self.bump();
                        let mut t = self.parse_type_decl(false)?;
                        t.comptime = true;
                        Some(Item::Type(t))
                    }
                    _ => {
                        let found = self.peek_at(1).kind.describe();
                        self.error(
                            format!(
                                "expected `fn`, `struct` or `type` after `comptime`, found \
                                 {found} [comptime-bound]"
                            ),
                            span,
                        );
                        None
                    }
                }
            }
            // The first spelling (2026-09-28), replaced the next day once the
            // model gained `comptime struct`: one word for the facility.
            TokenKind::Ident(name) if name == "compfn" => {
                let span = self.peek().span;
                self.error(
                    "`compfn` is spelled `comptime fn` [comptime-bound]",
                    span,
                );
                self.bump();
                self.parse_fn(false).map(Item::Fn)
            }
            // [obligation-by] The word this replaced (user decision 2026-09-28): the
            // structural implementations are `compfn`s in `core.auto`, asked
            // for with `by auto`, so `auto fn` has nothing left to mean.
            TokenKind::Ident(name)
                if name == "auto" && matches!(self.peek_at(1).kind, TokenKind::KwFn) =>
            {
                let span = self.peek().span;
                self.error(
                    "`auto fn` no longer exists: write `fn cmp(a: T, b: T) -> Int by auto` \
                     — the structural implementation is stamped from `core.auto`'s \
                     `compfn` [fn-by]",
                    span,
                );
                self.bump();
                self.parse_fn(false).map(Item::Fn)
            }
            // [test-decl] `test "an empty heap pops nothing" { … }` — a test,
            // named by a string literal. Contextual for the same reason
            // `iter fn` is: `test` is an ordinary name everywhere else (a
            // variable, a module, `std.test` itself), and the string literal
            // in the next position is what makes the form unambiguous with no
            // lookahead beyond it.
            TokenKind::Ident(name)
                if name == "test"
                    && (matches!(self.peek_at(1).kind, TokenKind::Str(_))
                        || (matches!(&self.peek_at(1).kind, TokenKind::Ident(k) if k == "actor")
                            && matches!(self.peek_at(2).kind, TokenKind::Str(_) | TokenKind::LParen))) =>
            {
                self.parse_test().map(Item::Test)
            }
            TokenKind::KwStruct => self.parse_struct(false).map(Item::Struct),
            // [noremote] `noremote struct S`, `noremote linear struct S`,
            // `noremote intrinsic type T`, `noremote type A = …`: the type
            // stays off the wire. The word is peeled and the declaration
            // parsed as usual, then flagged; anything but a type declaration
            // after it is an error naming what it is for.
            TokenKind::Ident(name)
                if name == NOREMOTE_MODIFIER
                    && matches!(
                        self.peek_at(1).kind,
                        TokenKind::KwStruct
                            | TokenKind::KwLinear
                            | TokenKind::KwIntrinsic
                            | TokenKind::KwType
                            | TokenKind::KwFn
                            | TokenKind::KwHandler
                            | TokenKind::KwEffect
                            | TokenKind::KwQualifier
                    ) || (name == NOREMOTE_MODIFIER
                        && matches!(&self.peek_at(1).kind, TokenKind::Ident(w) if w == OPAQUE_MODIFIER)) =>
            {
                let span = self.peek().span;
                self.bump();
                let item = self.parse_declaration()?;
                match item {
                    Item::Struct(mut d) => {
                        d.noremote = true;
                        Some(Item::Struct(d))
                    }
                    Item::Type(mut d) => {
                        d.noremote = true;
                        Some(Item::Type(d))
                    }
                    other => {
                        self.error(
                            "`noremote` keeps a *type* off the wire, so it precedes a \
                             `struct` or `type` declaration — a function, handler or \
                             effect has no wire form to refuse [noremote]",
                            span,
                        );
                        Some(other)
                    }
                }
            }
            // [struct-opaque] `opaque struct S`, `opaque linear struct S`: the
            // fields are the declaring module's. Contextual, like `noremote`.
            TokenKind::Ident(name)
                if name == OPAQUE_MODIFIER
                    && matches!(self.peek_at(1).kind, TokenKind::KwStruct | TokenKind::KwLinear) =>
            {
                let span = self.peek().span;
                self.bump();
                let item = self.parse_declaration()?;
                match item {
                    Item::Struct(mut d) => {
                        d.opaque = true;
                        Some(Item::Struct(d))
                    }
                    other => {
                        self.error(
                            "`opaque` hides a struct's fields outside its module, so it \
                             precedes a `struct` declaration [struct-opaque]",
                            span,
                        );
                        Some(other)
                    }
                }
            }
            // [linear-group] [obligation-spelling] `linear struct X { … }`:
            // the exactly-once obligation as a declaration modifier.
            TokenKind::KwLinear if matches!(self.peek_at(1).kind, TokenKind::KwStruct) => {
                self.bump();
                self.parse_struct(true).map(Item::Struct)
            }
            // [platform-type] `linear platform type HostIn`: a host handle with
            // an obligation.
            TokenKind::KwLinear if matches!(self.peek_at(1).kind, TokenKind::KwPlatform) => {
                self.bump();
                self.bump();
                if !self.at(&TokenKind::KwType) {
                    let found = self.kind().describe();
                    let span = self.peek().span;
                    self.error(
                        format!(
                            "`linear platform` declares a linear host handle, so `type` must \
                             follow: `linear platform type HostIn` (found {found}) [platform-type]"
                        ),
                        span,
                    );
                    return None;
                }
                self.parse_platform_type(false, true)
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
            // [platform-handler] `platform handler H of E`: a host
            // implementation of a Salvo effect.
            TokenKind::KwPlatform => {
                self.bump();
                self.parse_platform_rest(false)
            }
            // [threadsafe-platform] `threadsafe platform handler H of E`: the
            // host class is safe to enter concurrently, so the compiler shares
            // it without a lock on either backend. Contextual: `threadsafe`
            // is an ordinary name everywhere but directly before `platform`.
            TokenKind::Ident(name)
                if name == THREADSAFE_MODIFIER
                    && matches!(self.peek_at(1).kind, TokenKind::KwPlatform) =>
            {
                let span = self.peek().span;
                self.bump();
                self.bump();
                match self.kind() {
                    TokenKind::KwHandler | TokenKind::KwType => self.parse_platform_rest(true),
                    other => {
                        let found = other.describe();
                        self.error(
                            format!(
                                "expected `handler` or `type` after `threadsafe platform`, \
                                 found {found}: the thread-safety contract is a claim about a \
                                 host class, so it belongs on a `platform handler` or a \
                                 `platform type` [threadsafe-platform]"
                            ),
                            span,
                        );
                        None
                    }
                }
            }
            // [platform-iterable] `iterable platform type List<T> : Iter<self,
            // T>`. Contextual, like `threadsafe`.
            TokenKind::Ident(name)
                if name == ITERABLE_MODIFIER
                    && matches!(self.peek_at(1).kind, TokenKind::KwPlatform) =>
            {
                let span = self.peek().span;
                self.bump();
                self.bump();
                if !self.at(&TokenKind::KwType) {
                    let found = self.kind().describe();
                    self.error(
                        format!(
                            "expected `type` after `iterable platform`, found {found}: \
                             native iteration is a claim about a host type, so it belongs \
                             on a `platform type` [platform-iterable]"
                        ),
                        span,
                    );
                    return None;
                }
                let item = self.parse_platform_type(false, false)?;
                match item {
                    Item::Type(mut t) => {
                        t.iterable = true;
                        Some(Item::Type(t))
                    }
                    other => Some(other),
                }
            }
            TokenKind::KwEffect => self.parse_effect().map(Item::Effect),
            // [actor-effect-kind] `actor effect E { … }`: an actor protocol.
            // Contextual — at item level a bare identifier is otherwise a
            // parse error, which is what makes one token of lookahead enough
            // (the `iter fn` precedent).
            TokenKind::Ident(name)
                if name == ACTOR_MODIFIER && matches!(self.peek_at(1).kind, TokenKind::KwEffect) =>
            {
                self.bump();
                self.parse_effect_kinded(true).map(Item::Effect)
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
                self.parse_effect_kinded(true).map(Item::Effect)
            }
            TokenKind::KwHandler => self.parse_handler(false).map(Item::Handler),
            // [qual-refn-scope] A refinement lives in the qualifier whose claim
            // it is about — always (user decision 2026-09-26). The top-level
            // form existed to *reconcile* two qualifiers that disagreed, and
            // disagreement can no longer be expressed [qual-no-with], so the
            // form has no job left.
            TokenKind::KwRefn => {
                let sp = self.peek().span;
                let decl = self.parse_refn()?;
                self.error(
                    format!(
                        "a refinement belongs to the qualifier whose claim it is \
                         about: move `refn {}` into that qualifier's body \
                         [qual-refn-scope]",
                        decl.name.name
                    ),
                    sp,
                );
                Some(Item::Refn(decl))
            }
            // [fn-rename] A module-scoped name for one overload.
            TokenKind::KwRename => self.parse_rename().map(Item::Rename),
            TokenKind::KwParams => self.parse_params_group().map(Item::Params),
            TokenKind::KwFn => {
                let f = self.parse_fn(false)?;
                // [decl-body] [platform-fn] A top-level `fn` without a body:
                // the host's to implement only when it says so.
                if f.body.is_none() && f.by.is_none() {
                    self.error(
                        format!(
                            "`fn {}` has no body: write one, or declare it `platform fn {}` \
                             for the host to implement [platform-fn]",
                            f.name.name, f.name.name
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
        // [test-kind] `test actor(seed: 7) "…"`: the kind, then its
        // arguments, before the name. The parentheses may be left off when
        // there are none.
        let mut kind = TestKind::Plain;
        if matches!(&self.kind(), TokenKind::Ident(k) if k == "actor") {
            self.bump(); // `actor`
            let mut seed = 0;
            if self.eat(&TokenKind::LParen).is_some() {
                while !self.at(&TokenKind::RParen) {
                    let arg = self.ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let negative = self.eat(&TokenKind::Minus).is_some();
                    let value_token = self.peek().clone();
                    let TokenKind::Int { value, .. } = value_token.kind else {
                        self.error(
                            format!(
                                "an actor test's `{}` must be an integer literal [test-actor]",
                                arg.name
                            ),
                            value_token.span,
                        );
                        return None;
                    };
                    self.bump();
                    match arg.name.as_str() {
                        "seed" => seed = if negative { -value } else { value },
                        other => {
                            self.error(
                                format!(
                                    "an actor test takes `seed`, not `{other}` [test-actor]"
                                ),
                                arg.span,
                            );
                            return None;
                        }
                    }
                    if self.eat(&TokenKind::Comma).is_none() {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
            }
            kind = TestKind::Actor { seed };
        }
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
            kind,
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
        // [group-obligation] [obligation-by] The clause closes a type
        // declaration (comptime round 2, R-1): `type Source = A | B : ToStr<self>
        // by auto`, `intrinsic type List<T> canbe Mut : Hashed<self>`. Last,
        // because an alias has to be read before what it promises.
        let obligations = self.parse_obligations()?;
        if linear && alias.is_some() {
            self.error(
                "an alias cannot be linear: the obligation belongs to the type \
                 itself, so write `linear intrinsic type` (or `linear struct`) \
                 where it is declared",
                name.span,
            );
        }
        let end = obligations
            .last()
            .map(|o| o.by.as_ref().map(|b| b.span).unwrap_or(o.group.span))
            .or_else(|| alias.as_ref().map(|t| t.span()))
            .or_else(|| auto_qualifiers.last().map(|q| q.span))
            .unwrap_or(name.span);
        Some(TypeDecl {
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            // [noremote] Set by `parse_declaration`, which peels the word.
            noremote: false,
            comptime: false,
            platform: false,
            threadsafe: false,
            iterable: false,
            docs,
            intrinsic,
            linear,
            name,
            generics,
            generic_canbe,
            fn_slots,
            auto_qualifiers,
            alias,
            obligations,
            span: start.to(end),
        })
    }

    /// `<A, B, C>` — declaration-site generic parameters.
    fn parse_generics(&mut self) -> Vec<Ident> {
        let (generics, canbe, slots) = self.parse_generics_all();
        for (ident, _) in &canbe {
            self.error(
                "`canbe` on a type parameter is only supported on functions, structs and qualifiers",
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
                    "`?{name}` declares a function slot, which only a qualifier, a \
                     struct or an `intrinsic type` may have [cmp-carry]"
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
        // [struct-slot] And fn slots, as a type declaration has them
        // [cmp-carry]: `struct Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) ->
        // Bool)`.
        let (generics, generic_canbe, mut fn_slots) = self.parse_generics_slots();
        let (block_slots, value_slots) = self.parse_slot_block();
        fn_slots.extend(block_slots);
        for v in &value_slots {
            self.error(
                "a value slot belongs to a qualifier, not a type: only a \
                 claim can depend on another value",
                v.span,
            );
        }
        // `: Yield<self, Str>` — obligation groups this type satisfies
        // [group-obligation]. Before `canbe`, because `:` states what the
        // type must *provide* while `canbe` states what it may be qualified
        // as.
        let obligations = self.parse_obligations()?;
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
        let mut fns = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            // [fn-attached] A `fn` inside the body is **declared on** the
            // struct: it travels with the type, and its visibility is the
            // type's. `auto fn` is the generated form of the same thing.
            // [fn-attached] An inner fn's visibility *is* the type's, so
            // writing `export` on one says something the language does not let
            // it say.
            if self.at_word(EXPORT_MODIFIER)
                && matches!(self.peek_at(1).kind, TokenKind::KwFn)
            {
                let sp = self.peek().span;
                self.error(
                    "a function declared inside a struct takes the struct's own \
                     visibility: drop the `export` (export the struct instead) \
                     [fn-attached]",
                    sp,
                );
                self.bump();
            }
            // [comptime-bound] A concrete `compfn` inside the body: its own
            // single instantiation, declared on the type like any inner fn
            // (comptime round 2, 12.1).
            if self.at_word(COMPTIME_WORD) && matches!(self.peek_at(1).kind, TokenKind::KwFn) {
                fns.push(self.parse_compfn()?);
                continue;
            }
            if self.at_word("auto") && matches!(self.peek_at(1).kind, TokenKind::KwFn) {
                let sp = self.peek().span;
                self.error(
                    "`auto fn` no longer exists: write `fn cmp(a: T, b: T) -> Int by auto` \
                     — the structural implementation is stamped from `core.auto`'s \
                     `compfn` [fn-by]",
                    sp,
                );
                self.bump();
            }
            if self.at(&TokenKind::KwFn) {
                fns.push(self.parse_fn(false)?);
                continue;
            }
            fields.push(self.parse_field_decl(false)?);
            self.eat(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(StructDecl {
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            // [noremote] Set by `parse_declaration`, which peels the word.
            noremote: false,
            // [comptime-fields] Set by `parse_declaration` too.
            comptime: false,
            docs,
            name,
            fns,
            generics,
            generic_canbe,
            fn_slots,
            obligations,
            auto_qualifiers,
            fields,
            linear,
            // [struct-opaque] Set by `parse_declaration`, which peels the word.
            opaque: false,
            span: start.to(end),
        })
    }

    /// `: Yield<self, Str>, Ordered<self> by auto` — the obligation clause a
    /// struct or type declaration carries [group-obligation], each entry
    /// optionally saying where its members are stamped from [obligation-by].
    /// Answers an empty list when there is no `:`.
    fn parse_obligations(&mut self) -> Option<Vec<Obligation>> {
        let mut obligations = Vec::new();
        if self.eat(&TokenKind::Colon).is_none() {
            return Some(obligations);
        }
        loop {
            // [obligation-by] The pre-comptime spelling, worth its own message: the
            // word moved to the other side of the group.
            if self.at_word("auto")
                && matches!(
                    &self.peek_at(1).kind,
                    TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase())
                )
            {
                let sp = self.peek().span;
                self.error(
                    "`: auto Group<self>` no longer exists: write `: Group<self> by auto` — \
                     the members are stamped from `core.auto`'s `compfn`s [obligation-by]",
                    sp,
                );
                self.bump();
            }
            let group = self.parse_type_ref()?;
            let by = self.parse_by()?;
            obligations.push(Obligation { by, group });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        Some(obligations)
    }

    /// [obligation-by] [fn-by] The optional `by auto` / `by json` / `by
    /// my_compfn` clause: a lowercase dotted path. Same-line, like every
    /// trailing clause.
    fn parse_by(&mut self) -> Option<Option<ByRef>> {
        if !(self.at_word(BY_WORD) && self.same_line()) {
            return Some(None);
        }
        let start = self.bump().span;
        // [obligation-by] `by @auto`: a leading `@` says the path is a module
        // (a module stands on the right of `@`, as in `size@list`).
        let mut at = None;
        if self.at(&TokenKind::At) && self.same_line() {
            let sp = self.bump().span;
            at = Some(ByAt::Module(sp));
        }
        let mut path = vec![self.ident_value("scope after `by`")?];
        while self.at(&TokenKind::Dot)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
        {
            self.bump();
            path.push(self.ident()?);
        }
        let mut end = path.last().map(|p| p.span).unwrap_or(start);
        // `by auto@mymod`: the comptime fn `auto` of module `mymod`.
        if self.at(&TokenKind::At) && self.same_line() {
            let at_span = self.bump().span;
            if at.is_some() {
                self.error(
                    "`by @x@y` says both readings: `by @x` is the module `x`, `by x@y` the \
                     comptime fn `x` of module `y` [obligation-by]",
                    at_span,
                );
                return None;
            }
            let mut sel = vec![self.ident_value("module after `@`")?];
            while self.at(&TokenKind::Dot)
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
            {
                self.bump();
                sel.push(self.ident()?);
            }
            end = sel.last().map(|p| p.span).unwrap_or(end);
            at = Some(ByAt::Fn(sel));
        }
        let span = start.to(end);
        Some(Some(ByRef { path, at, span }))
    }

    /// [comptime-bound] `compfn cmp<struct T>(a: T, b: T) -> Int => a, b { … }`:
    /// a `fn` in every respect but two — the generics take a **kind bound**
    /// (`<struct T>` or `<union T>`, at most one parameter), and the body is
    /// parsed with the comptime syntax enabled. A `compfn` with no generics is
    /// concrete: its own single instantiation.
    fn parse_compfn(&mut self) -> Option<FnDecl> {
        let docs = self.docs_here();
        let start = self.expect_word(COMPTIME_WORD)?;
        self.expect(&TokenKind::KwFn)?;
        let name = self.ident_value("comptime fn")?;
        let mut bound = None;
        // `<T is Struct>` / `<T is Union>` is a kind bound — a stamped fn.
        // Any other `<…>` is an ordinary generics list, read with the rest
        // of the signature: an expanded-in-place fn may be generic, and
        // stays so (user decision 2026-10-20) [comptime-generic-fn].
        if self.at(&TokenKind::Lt) && matches!(self.peek_at(2).kind, TokenKind::KwIs) {
            self.group_depth += 1;
            self.bump();
            // `<T is Struct>` / `<T is Union>`: the bound names an arm of
            // `core.comptime`'s `Kind`, spelled as the narrowing it is — the
            // same `is` a `[when field.type]` arm tests (user decision
            // 2026-09-29).
            let param = self.ident_type("type parameter")?;
            if self.expect(&TokenKind::KwIs).is_none() {
                self.group_depth -= 1;
                return None;
            }
            let kind_span = self.peek().span;
            let kind = match self.kind() {
                TokenKind::Ident(n) if n == "Struct" => CompKind::Struct,
                TokenKind::Ident(n) if n == "Union" => CompKind::Union,
                other => {
                    let found = other.describe();
                    self.error(
                        format!(
                            "a `comptime fn`'s type parameter is bound to a kind: write \
                             `<T is Struct>` or `<T is Union>` (found {found}) [comptime-bound]"
                        ),
                        kind_span,
                    );
                    self.group_depth -= 1;
                    return None;
                }
            };
            self.bump();
            if self.at(&TokenKind::Comma) {
                let sp = self.peek().span;
                self.error(
                    "a `comptime fn` takes exactly one bounded type parameter: the type \
                     it is stamped at [comptime-bound]",
                    sp,
                );
            }
            self.group_depth -= 1;
            self.expect(&TokenKind::Gt)?;
            bound = Some((kind, param));
        }
        // The rest is a fn signature and body, with the comptime syntax on.
        let saved = std::mem::replace(&mut self.in_compfn, true);
        let fake_fn_span = start;
        let rest = self.parse_fn_rest(fake_fn_span, name, None, Vec::new(), Vec::new(), docs, false, FnFlavor::Plain);
        self.in_compfn = saved;
        let mut f = rest?;
        if let Some((_, param)) = &bound {
            f.generics = vec![param.clone()];
        }
        if f.body.is_none() && f.by.is_none() {
            self.error(
                format!(
                    "`comptime fn {}` has no body: it is the code that is stamped, so \
                     it always has one [comptime-bound]",
                    f.name.name
                ),
                f.span,
            );
        }
        if f.by.is_some() {
            self.error(
                format!(
                    "`comptime fn {}` cannot itself say `by`: it is what `by` stamps \
                     from [comptime-bound]",
                    f.name.name
                ),
                f.span,
            );
        }
        f.compfn = Some(CompFn {
            bound,
            span: start,
        });
        Some(f)
    }

    /// Consumes a contextual word, answering its span.
    fn expect_word(&mut self, word: &str) -> Option<Span> {
        if self.at_word(word) {
            return Some(self.bump().span);
        }
        let found = self.kind().describe();
        let span = self.peek().span;
        self.error(format!("expected `{word}`, found {found}"), span);
        None
    }

    /// `name: Type (= default)?`, or — when `allow_inferred` — `name = value`
    /// with the type inferred from the initializer [state-infer]. The inferred
    /// form is for `state { … }` blocks only (handlers and `iter fn`s); a
    /// struct field or qualifier override always writes its type.
    fn parse_field_decl(&mut self, allow_inferred: bool) -> Option<FieldDecl> {
        let docs = self.docs_here();
        // [comptime-fields] `type` is a keyword, and `Field.type` is the one
        // field spelled with it — the projection a comptime body reads as
        // `field.type`. Accepted as a field name here so `core.comptime` can
        // declare what the body reads.
        let name = if self.at(&TokenKind::KwType) && matches!(self.peek_at(1).kind, TokenKind::Colon) {
            let span = self.bump().span;
            Ident {
                name: "type".to_string(),
                span,
            }
        } else {
            self.ident_value("field")?
        };
        // [state-infer] `name = value` in a state block: no annotation, the
        // type is the initializer's. Recognised by an `=` where a `:` would
        // otherwise be.
        if allow_inferred && self.at(&TokenKind::Eq) {
            self.bump(); // `=`
            let value = self.parse_expr()?;
            let span = name.span.to(value.span());
            return Some(FieldDecl {
                docs,
                name,
                ty: None,
                canbe_mut: false,
                default: Some(value),
                span,
            });
        }
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        // [field-canbe-mut] `name: T canbe Mut` (user decision 2026-10-20; it
        // was `name: canbe Mut T`): written after the type, as a declaration's
        // own `canbe Mut` is after its name. Only `Mut` may follow, since it is
        // the one auto-qualifier a struct value carries.
        let canbe_mut = if self.at(&TokenKind::KwCanbe) && self.same_line() {
            self.bump();
            let q = self.ident_type("`Mut`")?;
            if q.name != "Mut" {
                self.error(
                    format!("only `Mut` may follow `canbe` on a field type, found `{}`", q.name),
                    q.span,
                );
            }
            true
        } else {
            false
        };
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
            ty: Some(ty),
            canbe_mut,
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
        let (generics, generic_canbe, mut fn_slots) = self.parse_generics_slots();
        let (block_slots, value_slots) = self.parse_slot_block();
        fn_slots.extend(block_slots);
        self.expect(&TokenKind::KwOf)?;
        let of = self.parse_type()?;
        // [qual-no-with] The `with` compatibility clause is retired (user
        // decision 2026-10-12): qualifiers compose freely, so there is no
        // allowlist to declare. A `with` here is no longer a qualifier
        // clause (the keyword survives for `use … with …`).
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
                    field_overrides.push(self.parse_field_decl(false)?);
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
            generic_canbe,
            fn_slots,
            value_slots,
            of,
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
        // [qual-refn-match] An optional `@module` selector scopes the match.
        let at = if self.at(&TokenKind::At) && self.same_line() {
            self.bump();
            self.ident_dotted_path()
        } else {
            None
        };
        let generics = self.parse_generics();
        // [qual-refn-match] A refn's parameter list may end with `*`, which
        // matches all remaining parameters of a candidate. Parsed here (not
        // via `parse_params`) so the wildcard is allowed.
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut params = Vec::new();
        let mut wildcard = false;
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            if self.at(&TokenKind::Star) {
                self.bump();
                wildcard = true;
                break;
            }
            let pname = self.ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let pspan = pname.span.to(ty.span());
            params.push(Param {
                name: pname,
                ty,
                variadic: false,
                implicit: false,
                default: None,
                span: pspan,
            });
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        self.expect(&TokenKind::RParen)?;
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
        // [deduce-conditional] A refn may carry a `-> subject is Q` response
        // (6c): it applies the conditional claim to each matched Bool fn.
        // Any *other* `->` (a return type) is still refused.
        let mut response = None;
        if self.at(&TokenKind::Arrow) {
            let is_cond = self.same_line()
                && {
                    // `-> ident (is | !is) …`
                    matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                        && (matches!(self.peek_at(2).kind, TokenKind::KwIs)
                            || (matches!(self.peek_at(2).kind, TokenKind::Bang)
                                && matches!(self.peek_at(3).kind, TokenKind::KwIs)))
                };
            if !is_cond {
                let span = self.peek().span;
                self.error(
                    format!(
                        "a refinement cannot declare a return type: `refn {}` \
                         refines an existing function's deductions, and the \
                         function decides what it returns; write `=> {}: +Qual` \
                         (or `-> {} is Qual` for a Bool predicate)",
                        name.name,
                        params.first().map(|p| p.name.name.as_str()).unwrap_or("param"),
                        params.first().map(|p| p.name.name.as_str()).unwrap_or("param")
                    ),
                    span,
                );
                return None;
            }
            self.bump();
            let mut responses = Vec::new();
            loop {
                let subject = self.ident()?;
                let negated = if self.at(&TokenKind::Bang) {
                    self.bump();
                    true
                } else {
                    false
                };
                self.expect(&TokenKind::KwIs)?;
                let qual = self.parse_type_ref()?;
                let span = subject.span.to(qual.span);
                responses.push(ConditionalResponse { subject, negated, qual, span });
                if !(self.at(&TokenKind::AmpAmp) && self.same_line()) {
                    break;
                }
                self.bump();
            }
            response = Some(responses);
        }
        // [qual-refn] The `=>` deduction clause is optional when a response
        // clause is present; otherwise a refn must state something.
        let deductions = if self.at(&TokenKind::FatArrow) {
            self.bump();
            self.parse_refn_deduction_list()?
        } else if response.is_some() {
            Vec::new()
        } else {
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
        };
        let end = deductions
            .last()
            .map(|d| d.span)
            .or_else(|| response.as_ref().and_then(|r| r.last()).map(|r| r.span))
            .unwrap_or(start);
        Some(RefnDecl {
            docs,
            name,
            at,
            generics,
            params,
            wildcard,
            deductions,
            response,
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

    /// [effect-alias] `effects IO = [Console, Fs, Streams]` — the caller is
    /// positioned on the `effects` identifier.
    fn parse_effect_alias(&mut self) -> Option<EffectAliasDecl> {
        let docs = self.docs_here();
        let start = self.bump().span; // `effects`
        let name = self.ident_type("effect alias")?;
        self.expect(&TokenKind::Eq)?;
        let effects = self.parse_effect_list()?;
        let end = effects.last().map(|e| match e {
            EffectRef::Use(s) | EffectRef::Spawn(s) => *s,
            EffectRef::Effect(r) | EffectRef::AnyEffect(r) => r.span,
        }).unwrap_or(name.span);
        Some(EffectAliasDecl {
            exported: false,
            docs,
            name,
            effects,
            span: start.to(end),
        })
    }

    fn parse_effect(&mut self) -> Option<EffectDecl> {
        self.parse_effect_kinded(false)
    }

    /// [actor-effect-kind] `actor effect E { … }` — an actor protocol. The
    /// modifier is **contextual** (`actor` followed by `effect`), like every
    /// other word this phase added: nothing is reserved, so `actor` stays a
    /// legal name. There is deliberately no `actor fn` — the phase decided
    /// against colouring — so this is the only place the word appears.
    fn parse_effect_kinded(&mut self, is_actor: bool) -> Option<EffectDecl> {
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwEffect)?.span;
        let name = self.ident_type("effect")?;
        let generics = self.parse_generics();
        // [effect-prereq] `effect Fs [Streams] { … }` — the dependency-list
        // spelling a handler already uses for "needs this from the scope".
        let prereqs = if self.at(&TokenKind::LBracket) {
            self.parse_effect_list()?
        } else {
            Vec::new()
        };
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
            is_actor,
            name,
            generics,
            prereqs,
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
        // [implicit-with] The group's own clause, before the body: `params
        // Hashed<T> => eq with hash { … }`. Spreading a group opts into its
        // deductions along with its members (user decision 2026-09-26), so the
        // clause is written where the members are.
        let mut deductions: Vec<Deduction> = Vec::new();
        if self.at(&TokenKind::FatArrow) {
            let arrow = self.bump().span;
            loop {
                let Some(entry) = self.parse_deduction_entry() else {
                    return None;
                };
                if !matches!(entry.kind, DeductionKind::With { .. }) {
                    self.error(
                        "a params group's clause states only which of its members are \
                         filled together (`=> eq with hash`): it has no body and no \
                         parameters of its own",
                        entry.span,
                    );
                    return None;
                }
                deductions.push(entry);
                deductions.extend(std::mem::take(&mut self.pending_deductions));
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            if deductions.is_empty() {
                self.error("expected a deduction entry after `=>`", arrow);
                return None;
            }
        }
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
            deductions,
            span: start.to(end),
        })
    }

    fn parse_handler(&mut self, intrinsic: bool) -> Option<HandlerDecl> {
        self.parse_handler_flavored(intrinsic, false, false)
    }

    /// What follows `platform` (and `threadsafe platform`): `handler`, or
    /// `fn` [platform-fn].
    fn parse_platform_rest(&mut self, threadsafe: bool) -> Option<Item> {
        match self.kind() {
            // [platform-fn] `platform fn f(…) -> T`: a host implementation of
            // one function. Bodiless; `threadsafe` belongs to a handler.
            TokenKind::KwFn if !threadsafe => {
                let mut f = self.parse_fn(false)?;
                f.platform = true;
                if f.body.is_some() || f.by.is_some() {
                    self.error(
                        format!(
                            "`platform fn {}` has a body: a platform fn is implemented by \
                             the host, so it is a signature only [platform-fn]",
                            f.name.name
                        ),
                        f.name.span,
                    );
                }
                Some(Item::Fn(f))
            }
            // [platform-handler] `platform handler HostRawFs of RawFs`: a
            // host implementation of an *ordinary* Salvo effect, registered
            // with `use` like any handler.
            TokenKind::KwHandler => self
                .parse_handler_flavored(false, true, threadsafe)
                .map(Item::Handler),
            // [platform-type] `platform type Client`: an opaque host handle.
            TokenKind::KwType => self.parse_platform_type(threadsafe, false),
            other => {
                let span = self.peek().span;
                let found = other.describe();
                self.error(
                    format!(
                        "expected `handler`, `fn` or `type` after `platform`, found {found}: a \
                         platform declaration is a host implementation of a Salvo effect \
                         (`platform handler`), of one function (`platform fn`), or a host \
                         object's handle (`platform type`)"
                    ),
                    span,
                );
                None
            }
        }
    }

    /// [platform-type] `platform type Name<T>` (after `platform`, and after
    /// `threadsafe` or `linear` where written): the ordinary type grammar, then
    /// the flags. An alias is refused — a platform type *is* its host class.
    fn parse_platform_type(&mut self, threadsafe: bool, linear: bool) -> Option<Item> {
        let mut t = self.parse_type_decl_linear(false, linear)?;
        t.platform = true;
        t.threadsafe = threadsafe;
        t.noremote = true;
        if t.alias.is_some() {
            self.error(
                format!(
                    "`platform type {}` cannot be an alias: a platform type is an opaque \
                     handle to the host class of the same name [platform-type]",
                    t.name.name
                ),
                t.name.span,
            );
        }
        Some(Item::Type(t))
    }

    /// `handler`, `intrinsic handler` [backend-intrinsic] and
    /// `platform handler` [platform-handler] share every piece of grammar:
    /// the two modifiers differ only in who supplies the members.
    /// `threadsafe` [threadsafe-platform] rides on the platform form.
    fn parse_handler_flavored(
        &mut self,
        intrinsic: bool,
        platform: bool,
        threadsafe: bool,
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
        // [effect-any] `of any E`: this face forwards to many instances.
        let mut of_any = vec![self.eat_any_prefix()];
        let mut of = vec![self.parse_type()?];
        while self.eat(&TokenKind::Comma).is_some() {
            of_any.push(self.eat_any_prefix());
            of.push(self.parse_type()?);
        }
        let mut state = Vec::new();
        let mut fns = Vec::new();
        let mut mailbox = None;
        let mut init: Option<FnDecl> = None;
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
                // [handler-init] `init { … }`: contextual like `mailbox` — a
                // state field named `init` is `init: T = …`.
                let is_init = self.at_word(INIT_BLOCK)
                    && matches!(self.peek_at(1).kind, TokenKind::LBrace);
                if self.at(&TokenKind::KwFn) || is_send_member {
                    fns.push(self.parse_member_fn()?);
                } else if is_init {
                    let name_tok = self.bump();
                    let body = self.parse_block()?;
                    if init.is_some() {
                        self.error("a handler declares one `init` block", name_tok.span);
                    }
                    let name = Ident { name: INIT_BLOCK.to_string(), span: name_tok.span };
                    let span = name_tok.span.to(body.span);
                    init = Some(FnDecl {
                        docs: Vec::new(),
                        exported: false,
                        intrinsic: false,
                        platform: false,
                        is_iter: false,
                        iter_state: Vec::new(),
                        is_send: true,
                        name,
                        type_name: None,
                        scoped_to: None,
                        compfn: None,
                        by: None,
                        stamped: None,
                        generics: Vec::new(),
                        generic_canbe: Vec::new(),
                        derived_return: None,
                        params: Vec::new(),
                        implicit_groups: Vec::new(),
                        effects: None,
                        deductions: None,
                        return_type: None,
                        response: None,
                        constructs: None,
                        body: Some(body),
                        span,
                    });
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
                } else if self.at(&TokenKind::KwState) {
                    // [effect-handler] [state-infer] A handler's state lives in
                    // a `state { … }` block, the same shape an `iter fn` uses
                    // (user decision 2026-10-20, replacing the free field decls
                    // that used to sit in the handler body). Each field is
                    // `name: Type = value` or the inferred `name = value`; the
                    // type, when inferred, is the initializer's.
                    let state_kw = self.bump().span;
                    if !state.is_empty() {
                        self.error("a handler declares one `state` block", state_kw);
                    }
                    if self.expect(&TokenKind::LBrace).is_some() {
                        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                            let before = self.pos;
                            match self.parse_field_decl(true) {
                                Some(field) => {
                                    if field.default.is_none() {
                                        self.error(
                                            format!(
                                                "state field `{}` needs an initializer: a \
                                                 handler's state is built once, at \
                                                 construction",
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
                } else {
                    // [effect-handler] A bare field declaration in the handler
                    // body is no longer state — state moved into the `state {
                    // … }` block (user decision 2026-10-20). Name the block so
                    // the fix is obvious.
                    let fspan = self.peek().span;
                    self.error(
                        "a handler's state goes in a `state { … }` block now, not \
                         as a bare field in the body: wrap it (`state { count: \
                         Int = 0 }`, or `state { count = 0 }` to infer the type)",
                        fspan,
                    );
                    // Recover by parsing it as a field anyway, so one stray
                    // field is one diagnostic rather than a cascade.
                    let before = self.pos;
                    if let Some(field) = self.parse_field_decl(true) {
                        state.push(field);
                    } else {
                        self.recover_in_block();
                    }
                    // A token neither a field nor recovery consumes (a
                    // keyword such as `is` at the start of a line) would
                    // otherwise loop here forever, one diagnostic per turn.
                    if self.pos == before {
                        self.bump();
                    }
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
            threadsafe,
            name,
            generics,
            params,
            effects,
            of,
            of_any,
            init,
            mailbox,
            state,
            fns,
            span: start.to(end),
        })
    }

    /// [effect-any] `any` before a type name, contextual: an
    /// effect called `any` (unwise) still parses alone.
    fn eat_any_prefix(&mut self) -> bool {
        if self.at_word("any") && matches!(self.peek_at(1).kind, TokenKind::Ident(_)) {
            self.bump();
            true
        } else {
            false
        }
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
        // The docs sit above the whole declaration; `external`/`intrinsic`
        // is on the same line as `fn`, so the line lookup finds them
        // whether or not the modifier was already consumed [doc-comment].
        let docs = self.docs_here();
        let start = self.expect(&TokenKind::KwFn)?.span;
        // [fn-type-named] A capitalized name is a **type**: `fn Mut List<T>(…)`,
        // `fn Ok<T>(…)`. The type's generics become the fn's; `name` holds the
        // canonical key derived from the type. Otherwise an ordinary lowercase
        // fn name.
        if self.at_type_name() {
            let (ty, generics, canbe) = self.parse_type_fn_name()?;
            let (key, _) = type_fn_key(&ty);
            let name = Ident { name: key, span: ty.span() };
            return self.parse_fn_rest(start, name, Some(ty), generics, canbe, docs, intrinsic, flavor);
        }
        let name = self.ident_value("fn")?;
        self.parse_fn_rest(start, name, None, Vec::new(), Vec::new(), docs, intrinsic, flavor)
    }

    /// Everything after the declaration word and the name: generics,
    /// parameters, effects, return type, deductions, then a body — or a `by`
    /// clause [fn-by] — or nothing (a signature).
    fn parse_fn_rest(
        &mut self,
        start: Span,
        name: Ident,
        type_name: Option<Type>,
        type_generics: Vec<Ident>,
        type_canbe: Vec<(Ident, TypeRef)>,
        docs: Vec<String>,
        intrinsic: bool,
        flavor: FnFlavor,
    ) -> Option<FnDecl> {
        let is_iter = flavor == FnFlavor::Iter;
        let is_send = flavor == FnFlavor::Send;
        // [fn-attached] A function is **declared on** a type by being written
        // inside its body, or by being the file's fulfilment of one of the
        // type's obligations — never by a selector on the declaration (user
        // decision 2026-09-26, replacing `fn cmp@Person(…)`). Both spellings
        // are "declare something on the struct", which is the same shape
        // actors and qualifiers already use.
        let scoped_to = None;
        if self.at(&TokenKind::At) && self.same_line() {
            let at = self.bump().span;
            let target = self.ident();
            let target = target.map(|t| t.name).unwrap_or_else(|| "Type".to_string());
            self.error(
                format!(
                    "a declaration does not take a `@` selector: write the function \
                     inside `{target}`'s body to declare it on the type, or let it \
                     fulfil one of `{target}`'s obligations (`: Hashed<self>`) in \
                     this file [fn-attached]"
                ),
                at,
            );
        }
        // [fn-type-named] A type-named fn's generics came from the type
        // (`fn Mut List<T>` → `T`, `fn Ok<T canbe linear>` → the opt-in);
        // only an ordinary fn reads `<…>` here.
        let (generics, generic_canbe) = if type_name.is_some() {
            (type_generics, type_canbe)
        } else {
            self.parse_generics_canbe()
        };
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
        let mut response = None;
        // [proj-infer] `-> T holds proj(a, b)`: the opaque lends, held for
        // synthesis into the deduction list once the clause is parsed.
        let mut opaque_lends: Option<(Vec<Ident>, Span)> = None;
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
            // [deduce-conditional] `-> list is NonEmpty` / `-> list !is Q`
            // (chainable with `&&`): a qualifier-conditional response. The fn
            // returns `Bool`; the clause says what a `true` result proves.
            // Recognised by a lowercase identifier followed by `is` or `!is`.
            if constructs.is_none()
                && self.at_ident()
                && (matches!(self.peek_at(1).kind, TokenKind::KwIs)
                    || (matches!(self.peek_at(1).kind, TokenKind::Bang)
                        && matches!(self.peek_at(2).kind, TokenKind::KwIs)))
            {
                let mut responses = Vec::new();
                loop {
                    let subject = self.ident()?;
                    let negated = if self.at(&TokenKind::Bang) {
                        self.bump();
                        true
                    } else {
                        false
                    };
                    self.expect(&TokenKind::KwIs)?;
                    let qual = self.parse_type_ref()?;
                    let span = subject.span.to(qual.span);
                    responses.push(ConditionalResponse { subject, negated, qual, span });
                    // `&&` chains more entries; anything else ends the clause.
                    if !(self.at(&TokenKind::AmpAmp) && self.same_line()) {
                        break;
                    }
                    self.bump();
                }
                response = Some(responses);
                // The real return type is `Bool`.
                return_type = Some(Type::Named {
                    qualifiers: Vec::new(),
                    base: TypeRef {
                        name: Ident { name: "Bool".to_string(), span: start },
                        args: Vec::new(),
                        value_args: Vec::new(),
                        from: Vec::new(),
                        at: None,
                        binder: false,
                        established: false,
                        alias: None,
                        span: start,
                    },
                });
            } else {
            let ty = self.parse_type()?;
            // [proj-infer] `-> T holds proj(a, b)`: the result is owned and
            // holds borrows of the named parameters somewhere inside.
            if self.at_holds_proj() {
                let (sources, span) = self.parse_holds_proj()?;
                opaque_lends = Some((sources, span));
            }
            // [proj-anywhere] The derived-return summary the checker and the
            // emitters consume: the parameter named by the *first* `proj` in
            // the return type. `proj(p) T` is an ordinary qualifier
            // on `T`, so the old prefix spelling reads identically.
            derived_return = first_proj_source(&ty);
            return_type = Some(ty);
            }
        }
        // [deduce-syntax] `=> …` groups: the unnamed ones are the fn's own
        // clause; `=>[f] …` belongs to the fn-typed parameter `f`.
        let mut deductions = self.parse_deduction_clause(&mut params)?;
        // [proj-infer] The opaque return annotation *is* a written lends
        // entry: synthesized here so the checker's written-entry machinery
        // (validation, `declared_lends`, the hover's `lent` flags) is
        // untouched by the respelling.
        if let Some((sources, span)) = opaque_lends {
            deductions.get_or_insert_with(Vec::new).push(Deduction {
                target: DeductionTarget::Opaque,
                kind: DeductionKind::Proj(sources),
                span,
            });
        }

        // [fn-by] `fn cmp(a: P, b: P) -> Int by auto`: the body is stamped,
        // so none is written. Same-line, where a body would open.
        let by = self.parse_by()?;
        // [iter-fn] An `iter fn`'s body opens with the pass's own fields. It is
        // parsed here rather than as a statement so the body that follows is an
        // ordinary block: `state` declares data, it does not run.
        let mut iter_state = Vec::new();
        let body = if by.is_none() && self.at(&TokenKind::LBrace) && self.same_line() {
            if is_iter {
                Some(self.parse_iter_body(&mut iter_state)?)
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        if by.is_some() && self.at(&TokenKind::LBrace) && self.same_line() {
            let sp = self.peek().span;
            self.error(
                format!(
                    "`fn {}` says `by`, so its body is stamped: drop the body, or drop \
                     the `by` to write it yourself [fn-by]",
                    name.name
                ),
                sp,
            );
            self.parse_block()?;
        }

        let end = body
            .as_ref()
            .map(|b| b.span)
            .or_else(|| by.as_ref().map(|b| b.span))
            .or_else(|| constructs.as_ref().map(|c| c.span))
            .or_else(|| return_type.as_ref().map(|t| t.span()))
            .unwrap_or(name.span);
        Some(FnDecl {
            // [mod-export] Set by `parse_item`, which reads the modifier.
            exported: false,
            docs,
            intrinsic,
            platform: false,
            is_iter,
            is_send,
            iter_state,
            name,
            type_name,
            scoped_to,
            compfn: None,
            by,
            stamped: None,
            generics,
            generic_canbe,
            params,
            implicit_groups,
            derived_return,
            effects,
            deductions,
            return_type,
            response,
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
            // [fn-param-default] `name: T = expr`. A variadic tail and an
            // implicit parameter each already fill themselves, so neither
            // takes a default.
            let default = if let Some(eq) = self.eat(&TokenKind::Eq) {
                if variadic {
                    self.error("a variadic parameter cannot have a default value", eq.span);
                    self.group_depth -= 1;
                    return None;
                }
                if implicit_at.is_some() {
                    self.error(
                        "an implicit parameter cannot have a default value: it is \
                         resolved automatically, or overridden with `name = value`",
                        eq.span,
                    );
                    self.group_depth -= 1;
                    return None;
                }
                let Some(expr) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                Some(expr)
            } else {
                None
            };
            let span = name.span.to(default.as_ref().map(|e| e.span()).unwrap_or_else(|| ty.span()));
            params.push(Param {
                name,
                ty,
                variadic,
                implicit: implicit_at.is_some(),
                default,
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
                // [effect-any] `any E` (contextual): the requirement that
                // accepts a binding spread over many instances. `any`
                // followed by anything but a type name stays an ordinary
                // effect name, so an effect called `any` (unwise) still
                // parses alone. (`local E`, the form that accepted a
                // scope-local binding, was removed 2026-09-28 — every
                // binding is a handle [effect-handle].)
                TokenKind::Ident(name)
                    if name == "any" && matches!(self.peek_at(1).kind, TokenKind::Ident(_)) =>
                {
                    self.bump();
                    match self.parse_type_ref() {
                        Some(r) => effects.push(EffectRef::AnyEffect(r)),
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
    /// [proj-infer] The **opaque projection** annotation, which follows the
    /// return type: `-> T holds proj(a, b)` — the returned value is *owned*
    /// and holds borrows of the named parameters somewhere inside (a view),
    /// where the written type cannot show them. `proj(x) T` says the result
    /// **is** a borrow of `x`; `T holds proj(x)` says it *holds* one, and
    /// reading the type first is the point of the spelling (user decision
    /// 2026-09-25, replacing the prefix `T holds proj(a, b)` of the day
    /// before — which read as though the result were the projection).
    /// `holds` is a reserved word rather than contextual: a type is a chain
    /// of space-separated qualifiers, so a bare word after one is read as
    /// another qualifier (`Mut List<proj T> holds …` parsed `holds` as a
    /// qualifier and `proj(it)` as the base type before it was reserved).
    /// The sources need no parentheses of their own, because `proj(…)`
    /// already carries them — which is also what keeps the form
    /// unambiguous inside a fn type's parameter list.
    ///
    /// Answers the sources and the annotation's span; the caller synthesizes
    /// the `DeductionTarget::Opaque` entry, which is how everything
    /// downstream ([proj-infer]'s written-lends machinery) stays unchanged.
    fn parse_holds_proj(&mut self) -> Option<(Vec<Ident>, Span)> {
        let start = self.bump().span; // `holds`
        self.expect(&TokenKind::KwProj)?;
        self.expect(&TokenKind::LParen)?;
        let mut sources: Vec<Ident> = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                sources.push(self.ident()?);
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        let close = self.expect(&TokenKind::RParen)?.span;
        if sources.is_empty() {
            self.error(
                "`holds` needs the sources the result borrows: `T holds proj(a)`",
                start.to(close),
            );
        }
        Some((sources, start.to(close)))
    }

    /// [proj-infer] Whether an opaque-projection annotation follows: `holds`
    /// on the same line as the type it is about.
    fn at_holds_proj(&self) -> bool {
        self.at(&TokenKind::KwHolds) && self.same_line()
    }

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
                // [implicit-with] A plural subject's extra entries.
                entries.extend(std::mem::take(&mut self.pending_deductions));
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
            ) || matches!(next, TokenKind::Ident(w) if w == "iter" || w == "send" || w == ACTOR_MODIFIER || w == THREADSAFE_MODIFIER || w == NOREMOTE_MODIFIER),
            "iter" | "send" => matches!(next, TokenKind::KwFn),
            ACTOR_MODIFIER => matches!(next, TokenKind::KwEffect),
            THREADSAFE_MODIFIER => matches!(next, TokenKind::KwPlatform),
            NOREMOTE_MODIFIER => matches!(
                next,
                TokenKind::KwStruct | TokenKind::KwLinear | TokenKind::KwIntrinsic | TokenKind::KwType
            ),
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
            // [deduce-field] `!h.tags`: the **field** is replaced — its
            // storage identity is destroyed, which at field level is what
            // consumption means (the caller keeps the field, not the
            // object that was in it).
            let mut path: Vec<Ident> = Vec::new();
            while self.at(&TokenKind::Dot) {
                self.bump();
                path.push(self.ident()?);
            }
            let end = path.last().map(|i| i.span).unwrap_or(name.span);
            return Some(Deduction {
                span: start.to(end),
                target: DeductionTarget::Param { name, path },
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
        // (The bare `proj(…)` opaque entry moved to the return type —
        // `-> T holds proj(a, b)`, user decision 2026-09-24 [proj-infer]; a
        // `proj` here now falls through to the identifier expectation and is
        // a plain parse error.)
        // The target: `.f.g` (result path) or `x` / `x.f` (parameter path).
        let mut path: Vec<Ident> = Vec::new();
        // Extra subjects of a plural `with` entry.
        let mut plural_subjects: Vec<Ident> = Vec::new();
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
            // A **plural subject** — `a|b|c with hash` — is sugar for one
            // entry per subject; the extra subjects are collected here,
            // before the relation is read.
            let mut extra_subjects: Vec<Ident> = Vec::new();
            while self.at(&TokenKind::Pipe)
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(_))
            {
                self.bump();
                extra_subjects.push(self.ident()?);
            }
            if !extra_subjects.is_empty() {
                plural_subjects = extra_subjects;
            }
            while self.at(&TokenKind::Dot) {
                self.bump();
                path.push(self.ident()?);
            }
            Some(name)
        };
        let mut end = path.last().map(|i| i.span).or(target.as_ref().map(|n| n.span)).unwrap_or(start);
        // [implicit-with] `=> eq with hash`, and chains (`a with b with c`) —
        // implicit parameters that are only meaningful together (user decision
        // 2026-09-26). Symmetric because the entry states a relation between parameters rather than a
        // property of one.
        if self.at(&TokenKind::KwWith) {
            let mut others: Vec<Ident> = Vec::new();
            while self.eat(&TokenKind::KwWith).is_some() {
                let other = self.ident()?;
                end = other.span;
                others.push(other);
            }
            let Some(name) = target else {
                self.error(
                    "a `with` entry names implicit parameters, not a result path",
                    start.to(end),
                );
                return None;
            };
            if !path.is_empty() {
                self.error(
                    "a `with` entry's subject is a parameter, not a field path",
                    start.to(end),
                );
            }
            for subject in plural_subjects {
                self.pending_deductions.push(Deduction {
                    span: start.to(end),
                    target: DeductionTarget::Param {
                        name: subject,
                        path: Vec::new(),
                    },
                    kind: DeductionKind::With {
                        others: others.clone(),
                    },
                });
            }
            return Some(Deduction {
                span: start.to(end),
                target: DeductionTarget::Param { name, path: Vec::new() },
                kind: DeductionKind::With { others },
            });
        }
        let kind = if self.eat(&TokenKind::Colon).is_some() {
            // `: proj(…)` / `: ref(…)`
            if matches!(&self.peek().kind, TokenKind::KwProj | TokenKind::KwRef) {
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
            // [deduce-field] A parameter's **field path** may state a
            // projection (`v.items: proj(p)`) or — since 2026-09-25 — a
            // *field-granular mutation*: `=> h.tags: Mut` says the call
            // mutates only that field, narrowing the whole-value event the
            // parameter's `Mut` would otherwise produce, and `=> !h.tags`
            // says the field is **replaced** (its storage identity
            // destroyed — consumption, one level down). Nothing else.
            let field_ok = matches!(
                kind,
                DeductionKind::Proj(_) | DeductionKind::Exhaustive { .. } | DeductionKind::Moved
            );
            if !path.is_empty() && !field_ok {
                self.error(
                    "a parameter's field path can state a projection                      (`v.field: proj(p)`), a field-granular mutation                      (`v.field: Mut`) or a replacement (`!v.field`)",
                    start.to(end),
                );
            }
        }
        Some(Deduction { span: start.to(end), target, kind })
    }

    // --- Types ---

    pub fn parse_type(&mut self) -> Option<Type> {
        // [union-spread] A leading `...` makes the first arm a spread; it is
        // only meaningful in a union, so a lone `...T` with no `|` is still
        // built as a `Spread` and refused by the checker outside a union.
        let first = self.parse_union_arm()?;
        if !self.at(&TokenKind::Pipe) {
            return Some(first);
        }
        let mut arms = vec![first];
        while self.eat(&TokenKind::Pipe).is_some() {
            arms.push(self.parse_union_arm()?);
        }
        let span = arms.first().unwrap().span().to(arms.last().unwrap().span());
        Some(Type::Union { arms, span })
    }

    /// [union-spread] One arm of a union: a `...T` spread, or an ordinary
    /// postfix type. The spread's inner type is a full postfix type, but the
    /// checker requires it to be a bare generic name.
    fn parse_union_arm(&mut self) -> Option<Type> {
        if self.at(&TokenKind::Ellipsis) {
            let start = self.bump().span;
            let inner = self.parse_type_postfix()?;
            let span = start.to(inner.span());
            return Some(Type::Spread { inner: Box::new(inner), span });
        }
        self.parse_type_postfix()
    }

    /// [type-literal] A literal in type position: a plain string (no
    /// interpolation), an integer (optionally negative, `L` for a `Long`), or
    /// `true`/`false`. A float is refused here rather than parsed, since it
    /// is never a literal type.
    fn parse_type_literal(&mut self) -> Option<Type> {
        let start = self.peek().span;
        let negative = self.at(&TokenKind::Minus) && matches!(self.peek_at(1).kind, TokenKind::Int { .. } | TokenKind::Float { .. });
        let at = if negative { 1 } else { 0 };
        let value = match &self.peek_at(at).kind {
            TokenKind::Str(parts) if !negative => {
                let mut text = String::new();
                for part in parts {
                    match part {
                        StrPart::Text(t) => text.push_str(t),
                        StrPart::Interp { .. } => {
                            self.error("a literal type cannot interpolate", start);
                            self.bump();
                            return Some(Type::Literal { value: TypeLit::Str(String::new()), span: start });
                        }
                    }
                }
                TypeLit::Str(text)
            }
            TokenKind::Int { value, long } => {
                let v = if negative { -*value } else { *value };
                if *long { TypeLit::Long(v) } else { TypeLit::Int(v) }
            }
            TokenKind::KwTrue if !negative => TypeLit::Bool(true),
            TokenKind::KwFalse if !negative => TypeLit::Bool(false),
            TokenKind::Float { .. } => {
                if negative {
                    self.bump();
                }
                let span = self.bump().span;
                self.error(
                    "a float cannot be a literal type: equality on floats does not name a value [type-literal]",
                    start.to(span),
                );
                return Some(Type::Literal { value: TypeLit::Int(0), span: start.to(span) });
            }
            _ => return None,
        };
        if negative {
            self.bump();
        }
        let end = self.bump().span;
        Some(Type::Literal { value, span: start.to(end) })
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

    /// [fn-type-named] Parse a function's **type name** — a run of qualifier
    /// refs followed by a base (`Mut List<T>`, `Heap List<T>`, `Ok<T>`,
    /// `Ok<T canbe linear>`) — stopping before the parameter list `(`. The
    /// base's generics are parsed with `canbe` awareness, returned alongside
    /// the type and its generic names.
    /// [fn-type-named] The type name of a type-fn **call** — qualifiers then a
    /// base, stopping before the `(` — whose `<…>` are type *arguments*, so
    /// any type may stand there (`Mut Set<(Int, Str)>()`, `Mut List<Int?>()`).
    /// The declaration side reads generic *parameters* instead
    /// ([`Self::parse_type_fn_name`]); sharing that reader is what refused a
    /// tuple argument at a qualified call.
    fn parse_type_call_name(&mut self) -> Option<Type> {
        // A selector (`Ok@m`) or a borrow source is not part of a type-fn
        // name: leave those to the expression forms that read them.
        let plain = |r: &TypeRef| r.at.is_none() && r.from.is_empty();
        // A ref's span is its name's, as the declaration reader gives it: the
        // callee's span (hover, references) covers the name, not its `<…>`.
        let named = |mut r: TypeRef| {
            r.span = r.name.span;
            r
        };
        let mut quals: Vec<TypeRef> = Vec::new();
        let mut base = self.parse_type_ref_opts(false).filter(plain).map(named)?;
        while self.at_type_name() && self.same_line() {
            quals.push(base);
            base = self.parse_type_ref_opts(false).filter(plain).map(named)?;
        }
        Some(Type::Named { qualifiers: quals, base })
    }

    fn parse_type_fn_name(&mut self) -> Option<(Type, Vec<Ident>, Vec<(Ident, TypeRef)>)> {
        // Parse a run of refs; each is a name with optional (canbe-aware)
        // `<…>` generics. The last is the base, the rest qualifiers. Value
        // args (`(…)`) are never consumed — those are the fn's parameters.
        let parse_one = |p: &mut Self| -> Option<(TypeRef, Vec<Ident>, Vec<(Ident, TypeRef)>)> {
            let name = p.type_ref_name()?;
            let (gen_idents, canbe) = if p.at(&TokenKind::Lt) {
                p.parse_generics_canbe()
            } else {
                (Vec::new(), Vec::new())
            };
            let args: Vec<Type> = gen_idents
                .iter()
                .map(|g| Type::Named {
                    qualifiers: Vec::new(),
                    base: TypeRef {
                        name: g.clone(),
                        args: Vec::new(),
                        value_args: Vec::new(),
                        from: Vec::new(),
                        at: None,
                        binder: false,
                        established: false,
                        alias: None,
                        span: g.span,
                    },
                })
                .collect();
            let span = name.span;
            let r = TypeRef {
                name,
                args,
                value_args: Vec::new(),
                from: Vec::new(),
                at: None,
                binder: false,
                established: false,
                alias: None,
                span,
            };
            Some((r, gen_idents, canbe))
        };
        let mut quals: Vec<TypeRef> = Vec::new();
        let (mut base, mut generics, mut canbe) = parse_one(self)?;
        while self.at_type_name() && self.same_line() {
            quals.push(base);
            let (r, g, c) = parse_one(self)?;
            base = r;
            generics = g;
            canbe = c;
        }
        let ty = Type::Named { qualifiers: quals, base };
        Some((ty, generics, canbe))
    }

    /// [eff-handle] `eff[A, B, …]` → the internal `Addr` type carrying the
    /// effect set as its arguments. One effect reproduces the old `Addr<E>`
    /// exactly (`args: [E]`); several is a multi-effect handle (`args: [A,
    /// B]`). Members are plain effect refs — `use`/`spawn`/`any` are
    /// capabilities, not handle contents, so they are not accepted here.
    fn parse_eff_handle(&mut self) -> Option<Type> {
        let start = self.bump().span; // `eff`
        self.expect(&TokenKind::LBracket)?;
        self.group_depth += 1;
        let mut args: Vec<Type> = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            let r = match self.parse_type_ref() {
                Some(r) => r,
                None => {
                    self.group_depth -= 1;
                    return None;
                }
            };
            let span = r.span;
            args.push(Type::Named { qualifiers: Vec::new(), base: r });
            let _ = span;
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RBracket)?.span;
        if args.is_empty() {
            self.error("`eff[…]` names at least one effect", start.to(end));
            return None;
        }
        let base = TypeRef {
            name: Ident { name: ADDR_TYPE_NAME.to_string(), span: start },
            args,
            value_args: Vec::new(),
            from: Vec::new(),
            at: None,
            binder: false,
            established: false,
            alias: None,
            span: start.to(end),
        };
        Some(Type::Named { qualifiers: Vec::new(), base })
    }

    /// [comptime-inline] Whether a type splice `[binder.type]` starts here:
    /// exactly `[`, a lowercase name, `.`, `type`, `]`.
    fn at_type_splice(&self) -> bool {
        self.at(&TokenKind::LBracket)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
            && matches!(self.peek_at(2).kind, TokenKind::Dot)
            && matches!(self.peek_at(3).kind, TokenKind::KwType)
            && matches!(self.peek_at(4).kind, TokenKind::RBracket)
    }

    /// [comptime-inline] [comptime-fields] `[field.type]`: a binder's type
    /// spliced into a type position (ARG_PARSE.md §13 (B), user decision
    /// 2026-10-20). Carried as a ref named `field.type` — a name with a dot
    /// in it, which no written type can spell — that the expansion replaces
    /// per copy; the span covers the brackets.
    fn parse_type_splice(&mut self) -> Option<TypeRef> {
        let open = self.expect(&TokenKind::LBracket)?.span;
        self.require_compfn(open, "a type splice `[field.type]`");
        let binder = self.ident()?;
        self.bump(); // .
        self.bump(); // type
        let close = self.expect(&TokenKind::RBracket)?.span;
        let span = open.to(close);
        Some(TypeRef {
            name: Ident { name: format!("{}.type", binder.name), span },
            args: Vec::new(),
            value_args: Vec::new(),
            from: Vec::new(),
            at: None,
            binder: false,
            established: false,
            alias: None,
            span,
        })
    }

    fn parse_type_atom(&mut self) -> Option<Type> {
        if self.at(&TokenKind::LParen) {
            return self.parse_paren_type();
        }
        if self.at_type_splice() {
            let base = self.parse_type_splice()?;
            return Some(Type::Named { qualifiers: Vec::new(), base });
        }
        // [comptime-pattern] `*` — any type, in a comptime type pattern only.
        // Carried as a named type spelled `*`, which no declaration can spell;
        // the expansion matches it and it never survives into checked code.
        if self.in_comp_pattern && self.at(&TokenKind::Star) {
            let span = self.bump().span;
            return Some(Type::Named {
                qualifiers: Vec::new(),
                base: TypeRef {
                    name: Ident { name: "*".to_string(), span },
                    args: Vec::new(),
                    value_args: Vec::new(),
                    from: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    span,
                },
            });
        }
        // [eff-handle] `eff[A, B, …]`: a first-class handle over a flat set of
        // effects. Contextual — only before `[` — so `eff` stays a name. It
        // lowers to the internal `Addr` type (unchanged machinery): a single
        // effect is `Addr<E>` as before, several is a multi-effect handle.
        if self.at_word(EFF_WORD) && matches!(self.peek_at(1).kind, TokenKind::LBracket) {
            return self.parse_eff_handle();
        }
        // [eff-handle] The old handle spelling `Addr<…>` is no longer the
        // language: a plain parse error naming the replacement. (`Addr` is the
        // compiler's internal name for the handle; the surface is `eff[…]`.)
        if matches!(&self.kind(), TokenKind::Ident(n) if n == ADDR_TYPE_NAME)
            && matches!(self.peek_at(1).kind, TokenKind::Lt)
        {
            let span = self.peek().span;
            self.error(
                "`Addr<…>` is not a type you write: a first-class effect handle is \
                 spelled `eff[E]` (several effects: `eff[A, B]`) [eff-handle]",
                span,
            );
            return None;
        }
        if let Some(lit) = self.parse_type_literal() {
            return Some(lit);
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
                        TokenKind::KwProj | TokenKind::KwRef | TokenKind::KwOnce | TokenKind::KwLinear
                    ))
                    && self.same_line())
            {
                break;
            }
            // [fn-by] [obligation-by] `-> Long by auto`, `: Ordered<self> by
            // auto`: the clause word closes the type. It cannot be a type or a
            // qualifier, which are uppercase by rule [name-casing].
            if !established && self.at_word(BY_WORD) {
                break;
            }
            let mut r = self.parse_type_ref()?;
            r.established = established;
            refs.push(r);
        }
        // [comptime-inline] Qualifiers applied to a type splice: `Mut
        // [field.type]`. Every ref read so far is a qualifier; the expansion
        // puts them in front of the binder's own.
        if self.at_type_splice() && self.same_line() {
            let base = self.parse_type_splice()?;
            return Some(Type::Named { qualifiers: refs, base });
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
        // [type-literal-qualified] Qualifiers applied to a literal type:
        // `Short "port"`. Every ref read so far is a qualifier, so none is
        // popped as the base. (It used to stop here, leaving the literal
        // to fail as "expected item"; only the alias spelling worked.)
        if self.same_line() {
            if let Some(lit) = self.parse_type_literal() {
                let start = refs.first().unwrap().span;
                let span = start.to(lit.span());
                return Some(Type::QualifiedGroup {
                    qualifiers: refs,
                    base: Box::new(lit),
                    span,
                });
            }
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
            // [proj-infer] `-> Mut It holds proj(c)` on a fn type: the
            // value's result holds borrows of its own parameter `c` —
            // written inline, where the pre-2026-09-24 spelling was a remote
            // `=>[f] proj(c)` group on the enclosing declaration. The
            // synthesized entry lands in the fn type's own contract list,
            // which an `=>[f]` group extends rather than replaces. The
            // sources sit inside `proj(…)`, so the form cannot be confused
            // with the next parameter of the enclosing list.
            if self.at_holds_proj() {
                let (sources, dspan) = self.parse_holds_proj()?;
                let entry = Deduction {
                    target: DeductionTarget::Opaque,
                    kind: DeductionKind::Proj(sources),
                    span: dspan,
                };
                return Some(Type::Fn {
                    params: elems,
                    param_names: names,
                    effects,
                    deductions: Some(vec![entry]),
                    ret: Box::new(ret),
                    span: start.to(dspan),
                });
            }
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
            TokenKind::KwRef => {
                let span = self.bump().span;
                return Some(Ident { name: "ref".to_string(), span });
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
        // [comptime-inline] A bare `field.type` in a type position is not the
        // language: a binder's type is spliced in brackets, `[field.type]`
        // (ARG_PARSE.md §13 (B)). Reported here and still read as the splice,
        // so a speculative parse that rolls back (a generic call's `<…>`)
        // does not turn it into a misleading error elsewhere.
        if matches!(&self.kind(), TokenKind::Ident(n) if n.starts_with(|c: char| c.is_lowercase()))
            && matches!(self.peek_at(1).kind, TokenKind::Dot)
            && matches!(self.peek_at(2).kind, TokenKind::KwType)
        {
            let binder = self.ident()?;
            self.bump();
            let end = self.bump().span;
            let span = binder.span.to(end);
            self.error(
                format!(
                    "a binder's type is spliced into a type position in brackets: write \
                     `[{0}.type]`, not `{0}.type` [comptime-inline]",
                    binder.name
                ),
                span,
            );
            return Some(Ident { name: format!("{}.type", binder.name), span });
        }
        let head = self.ident()?;
        // [iter-fn] A leading `_` is the compiler's namespace: a generated
        // iterator struct is `__Iter_<fn>_…`, and it exists as a real
        // declaration after the desugaring, so without this a program could
        // *name* it — and the whole point of generating it is that an iterator
        // struct you must name is written by hand. Its spelling is `iter T`
        // [iter-type].
        if head.name.starts_with('_') {
            self.error(
                format!(
                    "`{}` is a compiler-generated name and cannot be written: an \
                     iterator struct a program needs to name is declared as a \
                     struct of its own, with its own `next` — or spelled `iter T` \
                     where a pattern is allowed",
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
        // [namespace] A dotted type reference may have any number of
        // uppercase segments (`Outer.Inner.Thing`), since namespaces nest
        // [name-dot] [name-casing]. Each `.Uppercase` extends the name; the
        // whole dotted path is one key, as the two-segment form always was.
        let mut name = head.name;
        let mut span = head.span;
        while self.at(&TokenKind::Dot)
            && self.same_line()
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            self.bump();
            let seg = self.ident()?;
            name = format!("{name}.{}", seg.name);
            span = span.to(seg.span);
        }
        Some(Ident { name, span })
    }

    fn parse_type_ref(&mut self) -> Option<TypeRef> {
        self.parse_type_ref_opts(true)
    }

    /// [fn-type-named] `parse_type_ref` with a flag to suppress the
    /// value-argument block (`Heap(?cmp)`, `Buffer(8)`): a type-fn *name*
    /// never carries value args — a following `(` there is the function's
    /// parameter / argument list, not part of the type.
    fn parse_type_ref_opts(&mut self, allow_value_args: bool) -> Option<TypeRef> {
        // [namespace] `.S` / `.S.T` — the leading-dot sugar: a reference whose
        // namespace is left to the expected type (D3/D4). Parsed as a name
        // that keeps its leading dot (`.S`), which no ordinary type can spell,
        // so the checker recognises and resolves it against the expected
        // type's exposed namespaces. Only uppercase segments follow the dot
        // (a type-like member); a lowercase one would be a field access.
        if self.at(&TokenKind::Dot)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
        {
            let dot = self.bump().span;
            let first = self.ident()?;
            let mut name_str = format!(".{}", first.name);
            let mut span = dot.to(first.span);
            while self.at(&TokenKind::Dot)
                && self.same_line()
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
            {
                self.bump();
                let seg = self.ident()?;
                name_str = format!("{name_str}.{}", seg.name);
                span = span.to(seg.span);
            }
            return Some(TypeRef {
                name: Ident { name: name_str, span },
                args: Vec::new(),
                value_args: Vec::new(),
                from: Vec::new(),
                at: None,
                binder: false,
                established: false,
                alias: None,
                span,
            });
        }
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
        // [proj-anywhere] [ref-handle] `proj(param)` / `ref(param)`: the
        // borrow's source, written on the obligation itself so it can sit
        // anywhere a type does — a union arm, a type argument, a tuple
        // element — and so a type borrowing from two parameters names each
        // (`proj(a, b)`). `proj` is a read-only view's borrow; `ref` is a
        // handle into the named container (mutable when it carries `Mut`).
        // Both read a paren this way; the sources are value names (lowercase,
        // by [name-casing]), which is what tells them from a qualified group
        // (`proj (Ok Str | Err Int)` — a type inside).
        let mut from = Vec::new();
        if (name.name == "proj" || name.name == "ref")
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
        if allow_value_args
            && name.name != "proj"
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
    /// every field needs an initializer. The type may be written
    /// (`i: Int = 0`) or inferred from the initializer (`i = 0`) [state-infer].
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
                    match self.parse_field_decl(true) {
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

    /// [terminal-scope] Runs `f` with terminal-`...` bodies **disallowed** —
    /// used for every sub-expression (a condition, an operand, a call
    /// argument, a `let` value), so `...` is a body only for the construct
    /// that *is* a statement. Restores the flag afterwards.
    fn sub_expr<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::replace(&mut self.allow_terminal, false);
        let out = f(self);
        self.allow_terminal = saved;
        out
    }

    /// [terminal-scope] The remaining statements of the enclosing block as a
    /// `Block` — a terminal scope's body. The enclosing `parse_block` loop sees
    /// `}` next and ends, so nothing may follow at the same level.
    fn parse_terminal_rest(&mut self) -> Block {
        let start = self.peek().span;
        let saved_depth = std::mem::replace(&mut self.group_depth, 0);
        let saved_no_struct = std::mem::replace(&mut self.no_struct, false);
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            match self.parse_block_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.recover_in_block(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.group_depth = saved_depth;
        self.no_struct = saved_no_struct;
        let end = self.peek().span;
        Block { stmts, span: start.to(end) }
    }

    /// [terminal-scope] Parses a body-taking construct's body for the simple
    /// constructs (`try`/`while`/`for`/`waitfor`, which take no inline `else`
    /// after a terminal `...`): a braced block, or — only when the construct is
    /// itself a statement — a terminal `...` whose body is the rest of the
    /// enclosing block. The second element is `true` for the terminal form.
    fn parse_body(&mut self, what: &str) -> Option<(Block, bool)> {
        if self.at(&TokenKind::Ellipsis) {
            if !self.allow_terminal || self.group_depth != 0 {
                let span = self.peek().span;
                self.error(
                    format!(
                        "`...` is a terminal scope — the rest of the enclosing \
                         block becomes {what}'s body — so it is only a statement, \
                         never a value or a nested expression: write a `{{ … }}` \
                         block here"
                    ),
                    span,
                );
                return None;
            }
            self.bump(); // `...`
            return Some((self.parse_terminal_rest(), true));
        }
        Some((self.parse_block()?, false))
    }

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.expect(&TokenKind::LBrace)?.span;
        // Blocks reset grouping: statements inside are newline-terminated
        // even when the block itself appears inside parentheses.
        let saved_depth = std::mem::replace(&mut self.group_depth, 0);
        let saved_no_struct = std::mem::replace(&mut self.no_struct, false);
        // [terminal-lambda] A nested block is not the enclosing statement's
        // expression tree; `parse_block_stmt` re-arms the flag per statement.
        let saved_in_stmt = std::mem::replace(&mut self.in_stmt_expr, false);
        let mut stmts = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            match self.parse_block_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.recover_in_block(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.group_depth = saved_depth;
        self.no_struct = saved_no_struct;
        self.in_stmt_expr = saved_in_stmt;
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Block {
            stmts,
            span: start.to(end),
        })
    }

    /// [terminal-scope] [terminal-lambda] One statement of a block, with the
    /// terminal-scope flags set for its own expression and the trailing-lambda
    /// capture run afterwards. Shared by `parse_block` and
    /// `parse_terminal_rest`.
    fn parse_block_stmt(&mut self) -> Option<Stmt> {
        // [terminal-scope] A statement's own expression may end in a terminal
        // `...` body; a sub-expression may not. [terminal-lambda] And its last
        // call argument may be a `(…) -> ...` lambda, allowed through the whole
        // statement expression tree (`in_stmt_expr`).
        self.allow_terminal = true;
        self.in_stmt_expr = true;
        let stmt = self.parse_stmt();
        self.allow_terminal = false;
        self.in_stmt_expr = false;
        // [terminal-lambda] If a trailing `(…) -> ...` lambda was parsed, the
        // rest of the enclosing block is now its body.
        if self.pending_terminal_lambda {
            self.pending_terminal_lambda = false;
            let rest = self.parse_terminal_rest();
            if let Some(mut s) = stmt {
                fill_terminal_lambda(&mut s, rest);
                return Some(s);
            }
            return stmt;
        }
        stmt
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
            // [comptime-inline] `[for …]` / `[if …]` / `[when …]`: a bracketed
            // head is a compile-time construct (user decision 2026-09-29,
            // replacing the `inline` word) — brackets appear exactly where the
            // same text without them would be runtime Salvo.
            TokenKind::LBracket
                if matches!(
                    self.peek_at(1).kind,
                    TokenKind::KwFor | TokenKind::KwIf | TokenKind::KwWhen
                ) =>
            {
                self.parse_inline().map(Stmt::Comp)
            }
            // [comptime-refuse] `refuse!("…")`.
            TokenKind::Ident(w)
                if w == REFUSE_FORM && matches!(self.peek_at(1).kind, TokenKind::Bang) =>
            {
                let start = self.bump().span;
                self.bump();
                self.require_compfn(start, "`refuse!`");
                self.expect(&TokenKind::LParen)?;
                self.group_depth += 1;
                let message = self.parse_expr();
                self.group_depth -= 1;
                let message = message?;
                let end = self.expect(&TokenKind::RParen)?.span;
                Some(Stmt::Comp(CompStmt::Refuse {
                    message,
                    span: start.to(end),
                }))
            }
            // [fn-rename] In force from here to the end of the block.
            TokenKind::KwRename => self.parse_rename().map(Stmt::Rename),
            TokenKind::KwUse => {
                let start = self.bump().span;
                // [eff-handle] Share one parser with the expression form; the
                // statement is the discarded-value case.
                let Expr::Use { handler, with_items, span } = self.parse_use_expr(start)? else {
                    return None;
                };
                Some(Stmt::Use {
                    handler: *handler,
                    with_items,
                    span,
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.at(&TokenKind::Eq) && self.same_line() {
                    self.bump();
                    let value = self.sub_expr(|p| p.parse_expr())?;
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
                        let rhs = self.sub_expr(|p| p.parse_expr())?;
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

    /// [comptime-inline] Each comptime form is legal only inside a `compfn`;
    /// elsewhere the error names the scope rather than reporting a bare
    /// "expected item".
    fn require_compfn(&mut self, span: Span, what: &str) {
        if !self.in_compfn {
            self.error(
                format!(
                    "{what} is comptime syntax and belongs inside a `comptime fn`: an \
                     ordinary fn has no fields to walk or instantiations to refuse \
                     [comptime-inline]"
                ),
                span,
            );
        }
    }

    /// `[for field in T.fields] { … }`, `[if <cond>] { … } else { … }`,
    /// `[when field.kind] { is Struct { … } … }`, `[when value] { [arm] { … }
    /// }`. [terminal-scope] `[for …]` and `[if …]` also take a terminal `...`
    /// body, as `for` and `if` do: the rest of the enclosing block.
    fn parse_inline(&mut self) -> Option<CompStmt> {
        let start = self.expect(&TokenKind::LBracket)?.span;
        self.require_compfn(start, "a `[for …]`/`[if …]`/`[when …]` head");
        match self.kind() {
            TokenKind::KwFor => {
                self.bump();
                let binder = self.ident_value("binder")?;
                self.expect(&TokenKind::KwIn)?;
                let seq = self.parse_comp_seq()?;
                self.expect(&TokenKind::RBracket)?;
                // [terminal-scope] `[for f in T.fields] ...`: the rest of the
                // block, once per field.
                let (body, _terminal) = self.parse_body("the `[for …]`")?;
                let span = start.to(body.span);
                Some(CompStmt::For {
                    binder,
                    seq,
                    body,
                    span,
                })
            }
            TokenKind::KwIf => {
                self.bump();
                let cond = self.parse_comp_cond()?;
                self.expect(&TokenKind::RBracket)?;
                // [terminal-scope] `[if cond] ...` keeps the rest of the block
                // when the condition holds; an optional braced `else { … }`
                // written inline right after the `...` is kept instead when it
                // does not — the runtime `if cond ...` shape.
                if self.at(&TokenKind::Ellipsis) {
                    if !self.allow_terminal || self.group_depth != 0 {
                        let span = self.peek().span;
                        self.error(
                            "`...` is a terminal scope — the rest of the enclosing block \
                             becomes the `[if …]`'s body — so it is only a statement: write \
                             a `{ … }` block",
                            span,
                        );
                        return None;
                    }
                    self.bump();
                    let mut else_ = None;
                    if self.at(&TokenKind::KwElse) {
                        self.bump();
                        if self.at(&TokenKind::Ellipsis) {
                            let span = self.peek().span;
                            self.error(
                                "`else ...` is not allowed on a terminal `[if …]`: only the \
                                 `[if]` branch is the terminal scope, and its `else` is an \
                                 ordinary `{ … }` block",
                                span,
                            );
                            return None;
                        }
                        else_ = Some(self.parse_block()?);
                    }
                    let then = self.parse_terminal_rest();
                    // The rest follows the `else`, so it is what the statement ends with.
                    let span = start.to(then.span);
                    return Some(CompStmt::If { cond, then, else_, span });
                }
                let then = self.parse_block()?;
                let mut end = then.span;
                let else_ = if self.at(&TokenKind::KwElse) {
                    self.bump();
                    let b = self.parse_block()?;
                    end = b.span;
                    Some(b)
                } else {
                    None
                };
                Some(CompStmt::If {
                    cond,
                    then,
                    else_,
                    span: start.to(end),
                })
            }
            _ => {
                self.expect(&TokenKind::KwWhen)?;
                // [comptime-name] `[when field.name] { is "a" | "b" { … } … }`.
                if let TokenKind::Ident(b) = self.kind().clone() {
                    if b.starts_with(|c: char| c.is_lowercase())
                        && matches!(self.peek_at(1).kind, TokenKind::Dot)
                        && matches!(&self.peek_at(2).kind, TokenKind::Ident(p) if p == "name")
                        && matches!(self.peek_at(3).kind, TokenKind::RBracket)
                    {
                        let binder = self.ident()?;
                        self.bump();
                        self.bump();
                        self.expect(&TokenKind::RBracket)?;
                        return self.parse_inline_when_name(start, binder);
                    }
                    // [comptime-kind] `[when field.kind] { is Struct { … } … }`:
                    // the kind of the binder's type.
                    if b.starts_with(|c: char| c.is_lowercase())
                        && matches!(self.peek_at(1).kind, TokenKind::Dot)
                        && matches!(&self.peek_at(2).kind, TokenKind::Ident(p) if p == "kind")
                        && matches!(self.peek_at(3).kind, TokenKind::RBracket)
                    {
                        let root = self.ident()?;
                        self.bump();
                        let end = self.bump().span;
                        self.expect(&TokenKind::RBracket)?;
                        let ty = CompTy { span: root.span.to(end), root, via_type: true, args: Vec::new() };
                        return self.parse_inline_when_kind(start, ty);
                    }
                }
                // A type subject (`T`, `field.type`) with `.kind` opens the
                // kind dispatch; a value subject the arm dispatch. The brace
                // decides which was meant: `{ is Kind` against `{ [arm]`.
                let snap = self.snapshot();
                if let Some(ty) = self.try_parse_comp_ty() {
                    if self.at(&TokenKind::Dot)
                        && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n == "kind")
                        && matches!(self.peek_at(2).kind, TokenKind::RBracket)
                    {
                        self.bump();
                        self.bump();
                        self.bump();
                        return self.parse_inline_when_kind(start, ty);
                    }
                    if self.at(&TokenKind::RBracket)
                        && matches!(self.peek_at(1).kind, TokenKind::LBrace)
                        && matches!(self.peek_at(2).kind, TokenKind::KwIs | TokenKind::KwElse | TokenKind::RBrace)
                    {
                        // [comptime-pattern] A dispatch over the type itself,
                        // by pattern. (A kind dispatch is over `.kind`
                        // [comptime-kind]: a kind word here is a type name, and
                        // the expansion says so when none is declared.)
                        self.bump();
                        return self.parse_inline_when_type(start, ty);
                    }
                }
                self.rollback(snap);
                let saved_no_struct = std::mem::replace(&mut self.no_struct, true);
                let value = self.parse_expr();
                self.no_struct = saved_no_struct;
                let value = value?;
                self.expect(&TokenKind::RBracket)?;
                self.expect(&TokenKind::LBrace)?;
                self.expect(&TokenKind::LBracket)?;
                let binder = self.ident_value("arm binder")?;
                self.expect(&TokenKind::RBracket)?;
                let body = self.parse_block()?;
                if !self.at(&TokenKind::RBrace) {
                    let sp = self.peek().span;
                    self.error(
                        "a `[when value]` over a union has exactly one written arm, \
                         `[arm] { … }`, stamped once per declared arm [comptime-inline]",
                        sp,
                    );
                }
                let end = self.expect(&TokenKind::RBrace)?.span;
                Some(CompStmt::WhenArms {
                    value,
                    binder,
                    body,
                    span: start.to(end),
                })
            }
        }
    }

    /// [comptime-name] The arms of a `[when field.name]`: `is "a" | "b" { … }`,
    /// and an optional `else`.
    fn parse_inline_when_name(&mut self, start: Span, binder: Ident) -> Option<CompStmt> {
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::KwElse) {
                self.comp_when_else("name the rest in an arm — `is \"a\" | \"b\" { … }` — since the arms cover every field (or arm) name")?;
                continue;
            }
            let head = self.expect(&TokenKind::KwIs)?.span;
            let lits = self.comp_name_lits()?;
            let body = self.parse_block()?;
            let span = head.to(body.span);
            arms.push(NameArm { lits, body, span });
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(CompStmt::WhenName {
            binder,
            arms,
            span: start.to(end),
        })
    }

    /// [when-exhaustive] An `else` in a comptime `[when]` with a subject: refused,
    /// as an ordinary `when` refuses one — the arms are the cases, and covering
    /// them is what is checked (user decision 2026-10-20). The block is read
    /// and dropped, so the rest of the dispatch still parses.
    fn comp_when_else(&mut self, remedy: &str) -> Option<()> {
        let span = self.bump().span;
        self.error(
            format!(
                "a `[when]` with a subject takes no `else`, as a `when` with one does not: \
                 {remedy} [when-exhaustive]"
            ),
            span,
        );
        self.parse_block()?;
        Some(())
    }

    fn parse_inline_when_kind(&mut self, start: Span, ty: CompTy) -> Option<CompStmt> {
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::KwElse) {
                self.comp_when_else("name the rest in an arm — `is Opaque` covers `Basic` and `Generic` — since the arms cover every kind")?;
                continue;
            }
            let head = self.expect(&TokenKind::KwIs)?.span;
            let kind = self.parse_kind_word()?;
            let body = self.parse_block()?;
            let span = head.to(body.span);
            arms.push(KindArm { kind, body, span });
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(CompStmt::WhenKind {
            ty,
            arms,
            span: start.to(end),
        })
    }

    /// An arm of `core.comptime`'s `Kind`: `Struct`, `Union`, `Tuple`,
    /// `FnType`, `Opaque` (or its halves `Basic`, `Generic`).
    fn parse_kind_word(&mut self) -> Option<TypeKindWord> {
        let span = self.peek().span;
        let word = match self.kind() {
            TokenKind::Ident(n) => TypeKindWord::parse(n),
            _ => None,
        };
        let Some(word) = word else {
            let found = self.kind().describe();
            self.error(
                format!(
                    "expected an arm of `Kind` — `Struct`, `Union`, `Tuple`, `FnType`, \
                     `Opaque` (or its halves `Basic`, `Generic`) — found {found} \
                     [comptime-inline]"
                ),
                span,
            );
            return None;
        };
        self.bump();
        Some(word)
    }

    /// `T`, `Reading`, or `field.type` — with no lookahead commitment: answers
    /// `None` (having consumed nothing durable; callers roll back) when what
    /// follows is not one of those shapes.
    fn try_parse_comp_ty(&mut self) -> Option<CompTy> {
        let TokenKind::Ident(head) = self.kind().clone() else {
            return None;
        };
        let root_span = self.peek().span;
        if head.starts_with(|c: char| c.is_uppercase()) {
            // [comptime-generic-fn] `Arg<T>` — an instantiated root.
            if matches!(self.peek_at(1).kind, TokenKind::Lt) {
                let r = self.parse_type_ref_opts(false)?;
                return Some(CompTy {
                    root: r.name,
                    via_type: false,
                    args: r.args,
                    span: r.span,
                });
            }
            self.bump();
            return Some(CompTy {
                root: Ident { name: head, span: root_span },
                via_type: false,
                args: Vec::new(),
                span: root_span,
            });
        }
        if self.at(&TokenKind::Ident(head.clone()))
            && matches!(self.peek_at(1).kind, TokenKind::Dot)
            && matches!(self.peek_at(2).kind, TokenKind::KwType)
        {
            self.bump();
            self.bump();
            let end = self.bump().span;
            return Some(CompTy {
                root: Ident { name: head, span: root_span },
                via_type: true,
                args: Vec::new(),
                span: root_span.to(end),
            });
        }
        None
    }

    fn parse_comp_ty(&mut self) -> Option<CompTy> {
        let span = self.peek().span;
        match self.try_parse_comp_ty() {
            Some(t) => Some(t),
            None => {
                let found = self.kind().describe();
                self.error(
                    format!(
                        "expected a compile-time type — the bound parameter (`T`), a type \
                         name, or a binder's type (`field.type`) — found {found} \
                         [comptime-fields]"
                    ),
                    span,
                );
                None
            }
        }
    }

    /// `T.fields` / `T.arms` / `field.type.arms`.
    fn parse_comp_seq(&mut self) -> Option<CompSeq> {
        let ty = self.parse_comp_ty()?;
        self.expect(&TokenKind::Dot)?;
        let what = self.ident_value("`fields` or `arms`")?;
        let arms = match what.name.as_str() {
            "fields" => false,
            "arms" => true,
            other => {
                self.error(
                    format!(
                        "an `inline for` walks `.fields` (of a struct) or `.arms` (of a \
                         union), not `.{other}` [comptime-fields]"
                    ),
                    what.span,
                );
                return None;
            }
        };
        let span = ty.span.to(what.span);
        Some(CompSeq { ty, arms, span })
    }

    /// The compile-time conditions [comptime-inline] [comptime-logic]: leaves
    /// joined by `||`, `&&`, `!` and parentheses, with the precedence the same
    /// operators have in ordinary expressions (user decision 2026-10-20).
    fn parse_comp_cond(&mut self) -> Option<CompCond> {
        let mut lhs = self.parse_comp_cond_and()?;
        while self.at(&TokenKind::PipePipe) {
            self.bump();
            let rhs = self.parse_comp_cond_and()?;
            let span = lhs.span().to(rhs.span());
            lhs = CompCond::Or(Box::new(lhs), Box::new(rhs), span);
        }
        Some(lhs)
    }

    fn parse_comp_cond_and(&mut self) -> Option<CompCond> {
        let mut lhs = self.parse_comp_cond_unary()?;
        while self.at(&TokenKind::AmpAmp) {
            self.bump();
            let rhs = self.parse_comp_cond_unary()?;
            let span = lhs.span().to(rhs.span());
            lhs = CompCond::And(Box::new(lhs), Box::new(rhs), span);
        }
        Some(lhs)
    }

    fn parse_comp_cond_unary(&mut self) -> Option<CompCond> {
        if self.at(&TokenKind::Bang) {
            let start = self.bump().span;
            let inner = self.parse_comp_cond_unary()?;
            let span = start.to(inner.span());
            return Some(CompCond::Not(Box::new(inner), span));
        }
        if self.at(&TokenKind::LParen) {
            self.bump();
            self.group_depth += 1;
            let inner = self.parse_comp_cond();
            self.group_depth -= 1;
            let inner = inner?;
            self.expect(&TokenKind::RParen)?;
            return Some(inner);
        }
        self.parse_comp_cond_leaf()
    }

    /// `is` or `!is`, answering whether it was negated.
    fn comp_is_or_not_is(&mut self) -> Option<bool> {
        if self.at(&TokenKind::Bang) && matches!(self.peek_at(1).kind, TokenKind::KwIs) {
            self.bump();
            self.bump();
            return Some(true);
        }
        self.expect(&TokenKind::KwIs)?;
        Some(false)
    }

    /// A plain string literal (no interpolation) in a comptime head: a
    /// binder's name is compared to one [comptime-name].
    fn comp_name_lit(&mut self) -> Option<(String, Span)> {
        let lit_span = self.peek().span;
        let TokenKind::Str(parts) = self.kind().clone() else {
            let found = self.kind().describe();
            self.error(
                format!(
                    "expected a string literal naming a field or arm, found {found} \
                     [comptime-name]"
                ),
                lit_span,
            );
            return None;
        };
        self.bump();
        let mut lit = String::new();
        for part in parts {
            match part {
                StrPart::Text(t) => lit.push_str(&t),
                StrPart::Interp { .. } => {
                    self.error(
                        "the name a binder is compared to is a plain literal, with no \
                         interpolation [comptime-name]",
                        lit_span,
                    );
                    return None;
                }
            }
        }
        Some((lit, lit_span))
    }

    /// `"a" | "b" | …` — the literals a `field.name is` test or a `[when
    /// field.name]` arm names [comptime-name].
    fn comp_name_lits(&mut self) -> Option<Vec<(String, Span)>> {
        let mut lits = vec![self.comp_name_lit()?];
        while self.at(&TokenKind::Pipe) {
            self.bump();
            lits.push(self.comp_name_lit()?);
        }
        Some(lits)
    }

    /// One compile-time test [comptime-inline]: `X.kind is <Kind>`, `X is
    /// <Type>`, `X.mutable`, `b.name == "lit"` / `b.name is "a" | "b"`,
    /// `b.first`, `b.last`, `x.index == y.index`, each `is` also as `!is`.
    fn parse_comp_cond_leaf(&mut self) -> Option<CompCond> {
        let start = self.peek().span;
        // `binder.name …`, `binder.kind is …`, `binder.first`, `binder.last`,
        // `x.index == y.index`
        if let TokenKind::Ident(b) = self.kind().clone() {
            if b.starts_with(|c: char| c.is_lowercase())
                && matches!(self.peek_at(1).kind, TokenKind::Dot)
                && matches!(&self.peek_at(2).kind, TokenKind::Ident(p) if matches!(p.as_str(), "name" | "first" | "last" | "index" | "kind"))
            {
                let binder = self.ident()?;
                self.bump();
                let proj = self.ident()?;
                match proj.name.as_str() {
                    "index" => {
                        let op = match self.kind() {
                            TokenKind::EqEq => BinaryOp::Eq,
                            TokenKind::BangEq => BinaryOp::NotEq,
                            TokenKind::Lt => BinaryOp::Lt,
                            TokenKind::Gt => BinaryOp::Gt,
                            TokenKind::LtEq => BinaryOp::LtEq,
                            TokenKind::GtEq => BinaryOp::GtEq,
                            other => {
                                let found = other.describe();
                                let sp = self.peek().span;
                                self.error(
                                    format!(
                                        "a position is compared to another binder's position: \
                                         `x.index == y.index` (found {found}) [comptime-inline]"
                                    ),
                                    sp,
                                );
                                return None;
                            }
                        };
                        self.bump();
                        let other = self.ident_value("binder")?;
                        self.expect(&TokenKind::Dot)?;
                        let what = self.ident_value("`index`")?;
                        if what.name != "index" {
                            self.error(
                                "a position compares to a position: write `y.index` \
                                 [comptime-inline]",
                                what.span,
                            );
                            return None;
                        }
                        return Some(CompCond::IndexCmp {
                            a: binder,
                            b: other,
                            op,
                            span: start.to(what.span),
                        });
                    }
                    "first" | "last" => {
                        return Some(CompCond::Flag {
                            binder,
                            last: proj.name == "last",
                            span: start.to(proj.span),
                        });
                    }
                    "kind" => {
                        // [comptime-kind] `field.kind` is the kind of `field.type`.
                        let ty = CompTy {
                            root: binder.clone(),
                            via_type: true,
                            args: Vec::new(),
                            span: start.to(proj.span),
                        };
                        return self.comp_kind_test(start, ty);
                    }
                    _ => {
                        // `name`: `==`/`!=` one literal, or `is`/`!is` a
                        // union of them [comptime-name].
                        let negated;
                        let lits = if self.eat(&TokenKind::EqEq).is_some() {
                            negated = false;
                            vec![self.comp_name_lit()?]
                        } else if self.eat(&TokenKind::BangEq).is_some() {
                            negated = true;
                            vec![self.comp_name_lit()?]
                        } else if self.at(&TokenKind::KwIs)
                            || (self.at(&TokenKind::Bang) && matches!(self.peek_at(1).kind, TokenKind::KwIs))
                        {
                            negated = self.comp_is_or_not_is()?;
                            self.comp_name_lits()?
                        } else {
                            let sp = self.peek().span;
                            self.error(
                                "a binder's name is compared to a literal: `field.name == \
                                 \"value\"`, or tested against a union of them: `field.name \
                                 is \"a\" | \"b\"` [comptime-name]",
                                sp,
                            );
                            return None;
                        };
                        let end = lits.last().map(|(_, s)| *s).unwrap_or(proj.span);
                        let cond = CompCond::NameIs {
                            binder,
                            lits: lits.into_iter().map(|(l, _)| l).collect(),
                            span: start.to(end),
                        };
                        return Some(if negated {
                            CompCond::Not(Box::new(cond), start.to(end))
                        } else {
                            cond
                        });
                    }
                }
            }
        }
        let ty = self.parse_comp_ty()?;
        // `T.mutable`: the `Struct.mutable` projection as a condition.
        if self.at(&TokenKind::Dot)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n == "mutable")
        {
            self.bump();
            let end = self.bump().span;
            return Some(CompCond::Mutable {
                ty,
                span: start.to(end),
            });
        }
        // `T.kind is Struct`, `field.type.kind is Union` [comptime-kind].
        if self.at(&TokenKind::Dot)
            && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n == "kind")
        {
            self.bump();
            self.bump();
            return self.comp_kind_test(start, ty);
        }
        if self.at(&TokenKind::KwCanbe) {
            let sp = self.peek().span;
            self.error(
                "a type's mutability is read as `T.mutable` in a comptime condition \
                 (`core.comptime`'s `Struct.mutable`), not with `canbe` [comptime-inline]",
                sp,
            );
            return None;
        }
        // A written type: equality, never a kind test [comptime-kind] — a
        // pattern, with `*` and declared binders [comptime-pattern].
        let negated = self.comp_is_or_not_is()?;
        let (binders, target) = self.comp_type_pattern()?;
        let span = start.to(target.span());
        let cond = CompCond::Is { ty, binders, target, span };
        Some(if negated { CompCond::Not(Box::new(cond), span) } else { cond })
    }

    /// [comptime-pattern] `<E, K> Map<K, List<E>>` / `List<*>` / `Str`: the
    /// binders a pattern declares (before it, like a fn's generics), then
    /// the pattern, in which `*` is any type.
    fn comp_type_pattern(&mut self) -> Option<(Vec<Ident>, Type)> {
        let mut binders = Vec::new();
        if self.at(&TokenKind::Lt) {
            self.bump();
            self.group_depth += 1;
            loop {
                match self.ident_type("pattern binder") {
                    Some(b) => binders.push(b),
                    None => {
                        self.group_depth -= 1;
                        return None;
                    }
                }
                if self.eat(&TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.group_depth -= 1;
            self.expect(&TokenKind::Gt)?;
        }
        let saved = std::mem::replace(&mut self.in_comp_pattern, true);
        let target = self.parse_type();
        self.in_comp_pattern = saved;
        Some((binders, target?))
    }

    /// [comptime-pattern] The arms of a `[when field.type]`: `is <E> List<E> {
    /// … }`, `is Str { … }`, and the `else` the expansion requires.
    fn parse_inline_when_type(&mut self, start: Span, ty: CompTy) -> Option<CompStmt> {
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::KwElse) {
                self.comp_when_else("end with `is * { … }`, which matches every type the arms above it did not")?;
                continue;
            }
            let head = self.expect(&TokenKind::KwIs)?.span;
            let (binders, pattern) = self.comp_type_pattern()?;
            let body = self.parse_block()?;
            let span = head.to(body.span);
            arms.push(TypeArm { binders, pattern, body, span });
        }
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(CompStmt::WhenType {
            ty,
            arms,
            span: start.to(end),
        })
    }

    /// `… is <Kind>` / `… !is <Kind>` after `X.kind` [comptime-kind].
    fn comp_kind_test(&mut self, start: Span, ty: CompTy) -> Option<CompCond> {
        let negated = self.comp_is_or_not_is()?;
        let kind = self.parse_kind_word()?;
        let span = start.to(self.tokens[self.pos - 1].span);
        let cond = CompCond::Kind { ty, kind, span };
        Some(if negated { CompCond::Not(Box::new(cond), span) } else { cond })
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
        // [terminal-scope] A `let` value is a sub-expression: no terminal `...`.
        let value = self.sub_expr(|p| p.parse_expr())?;
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
        self.parse_binary(0).and_then(|e| self.newline_postfix(e))
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
            // [terminal-lambda] The body is captured after the statement; use
            // the span so far.
            LambdaBody::Terminal => start,
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
            // [terminal-lambda] The body is captured after the statement; use
            // the span so far.
            LambdaBody::Terminal => start,
        };
        Some(Expr::Lambda {
            params,
            body,
            span: start.to(end),
        })
    }

    fn parse_lambda_body(&mut self) -> Option<LambdaBody> {
        // [terminal-lambda] `(params) -> ...` — the lambda's body is the rest
        // of the enclosing block, captured after the statement closes. Allowed
        // only as the trailing argument of a statement's expression tree
        // (`in_stmt_expr`), and only one per statement.
        if self.at(&TokenKind::Ellipsis) {
            if !self.in_stmt_expr {
                let span = self.peek().span;
                self.error(
                    "a `(…) -> ...` lambda body is a terminal scope — the rest of the \
                     enclosing block becomes it — so it is only the last argument of a \
                     statement's call, never a nested or stored expression: write a \
                     `{ … }` body here",
                    span,
                );
                return None;
            }
            if self.pending_terminal_lambda {
                let span = self.peek().span;
                self.error(
                    "only one `(…) -> ...` terminal lambda per statement: the rest of \
                     the block cannot be two bodies — write `{ … }` on all but one",
                    span,
                );
                return None;
            }
            self.bump(); // `...`
            self.pending_terminal_lambda = true;
            return Some(LambdaBody::Terminal);
        }
        let saved_in_stmt = std::mem::replace(&mut self.in_stmt_expr, false);
        let body = if self.at(&TokenKind::LBrace) {
            Some(LambdaBody::Block(self.parse_block()?))
        } else {
            Some(LambdaBody::Expr(Box::new(self.parse_expr()?)))
        };
        self.in_stmt_expr = saved_in_stmt;
        body
    }

    /// [expr-infix] Every infix operator, parsed by one **precedence-climbing**
    /// loop over a single binding-power table (`infix_bp`), replacing the
    /// former tier-per-operator cascade (`parse_or`/`parse_and`/… — user
    /// decision 2026-10-14). `min_bp` is the lowest left binding power this
    /// call will consume: the top-level entry passes 0, and each operator
    /// recurses for its right side at its own *right* binding power, so a
    /// left-associative operator (every one Salvo has) folds left.
    ///
    /// The table is the one source of truth for precedence; see `infix_bp` for
    /// the levels. Three operators are not plain `lhs OP rhs` binaries and are
    /// dispatched specially from the loop at their tier: `is`/`!is` (a
    /// type-ref check, no expression right side), the positional-pick elvis
    /// (`?…?:` / `:?…?`, a `?`-run with its own wrap rule), and the pipe
    /// `|>` (binds `_` in its right side). The rest build `Expr::Binary`.
    ///
    /// [expr-wrap] Most operators require their line (`same_line`); the
    /// *flattening* operators (`|>`, both elvis forms, and — in the postfix
    /// tier — `.`/`replyto`) may instead begin a continuation line, where the
    /// newline brackets the whole expression so far on its left (the loop's
    /// left fold *is* that bracketing, since a flattening operator sits at a
    /// loose tier and `lhs` already holds everything to its left).
    fn parse_binary(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let Some((lbp, kind)) = self.infix_bp() else { break };
            if lbp <= min_bp {
                break;
            }
            // `is`/`!is`: a check, not a binary. Same tier as comparison.
            if matches!(kind, InfixKind::Is | InfixKind::IsNot) {
                lhs = self.parse_is_tail(lhs, matches!(kind, InfixKind::IsNot))?;
                continue;
            }
            // Elvis: the positional pick, with its own `?`-run scan and
            // [elvis-wrap] continuation rule.
            if matches!(kind, InfixKind::Elvis) {
                match self.parse_elvis_tail(lhs.clone())? {
                    Some(e) => {
                        lhs = e;
                        continue;
                    }
                    // Not actually an elvis operator here (a lone `?`/`:`, or
                    // a wrap left for an enclosing chain): stop.
                    None => break,
                }
            }
            // [expr-pipe] The pipe `|>`: evaluate the left side once, bind it
            // to `_`, read it in the right side.
            if matches!(kind, InfixKind::Pipe) {
                lhs = self.parse_pipe_tail(lhs)?;
                continue;
            }
            // [expr-safe-pipe] The null-safe pipe `?>`.
            if matches!(kind, InfixKind::SafePipe) {
                lhs = self.parse_safe_pipe_tail(lhs)?;
                continue;
            }
            // A plain binary operator. `rbp = lbp + 1` makes it left-associative.
            let InfixKind::Binary(op) = kind else { break };
            self.bump();
            let rhs = self.sub_expr(|p| p.parse_binary(lbp + 1))?;
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

    /// [expr-infix] The binding-power table: the current token's **left**
    /// binding power and what kind of infix operator it is, or `None` when it
    /// is not an infix operator here. The single definition of operator
    /// precedence — loosest (lowest) first:
    ///
    /// | bp | operators |
    /// |----|-----------|
    /// |  2 | `\|>` (pipe) |
    /// |  4 | `\|\|` |
    /// |  6 | `&&` |
    /// |  8 | `==` `!=` |
    /// | 10 | `<` `>` `<=` `>=`, `is`, `!is` |
    /// | 12 | elvis `?…?:` / `:?…?` |
    /// | 14 | `+` `-` |
    /// | 16 | `*` `/` `%` |
    ///
    /// Only the flattening operators (`|>` and elvis) may begin a continuation
    /// line; the rest return `None` across a newline so the statement ends
    /// (the `same_line` gate). Even-numbered levels leave the odd `lbp + 1`
    /// for each left-associative operator's right side.
    fn infix_bp(&self) -> Option<(u8, InfixKind)> {
        let same = self.same_line();
        Some(match self.kind() {
            // The flattening operators may start a continuation line
            // [expr-wrap]; a newline before them brackets the whole expression
            // so far on its left. (`|>` is unambiguous line-leading — nothing
            // in Salvo starts a statement with it.)
            TokenKind::PipeGt => (2, InfixKind::Pipe),
            // [expr-safe-pipe] `?>` — a `Question` glued to a `Gt` (not a
            // lexer token, like `?.`): the null-safe pipe, at the pipe tier,
            // detected before the elvis `?`-run so a `?>` is never read as an
            // elvis operator.
            TokenKind::Question
                if matches!(self.peek_at(1).kind, TokenKind::Gt) =>
            {
                (2, InfixKind::SafePipe)
            }
            // [elvis] `?:` may start a continuation line [elvis-wrap]; the
            // wrap decision is in `parse_elvis_tail`. Only `?` begins a pick
            // now (the backward `:?` form was retired), so a lone `:` is left
            // unconsumed.
            TokenKind::Question => (12, InfixKind::Elvis),
            // Same-line-only operators below.
            _ if !same => return None,
            TokenKind::PipePipe => (4, InfixKind::Binary(BinaryOp::Or)),
            TokenKind::AmpAmp => (6, InfixKind::Binary(BinaryOp::And)),
            TokenKind::EqEq => (8, InfixKind::Binary(BinaryOp::Eq)),
            TokenKind::BangEq => (8, InfixKind::Binary(BinaryOp::NotEq)),
            TokenKind::Lt => (10, InfixKind::Binary(BinaryOp::Lt)),
            TokenKind::Gt => (10, InfixKind::Binary(BinaryOp::Gt)),
            TokenKind::LtEq => (10, InfixKind::Binary(BinaryOp::LtEq)),
            TokenKind::GtEq => (10, InfixKind::Binary(BinaryOp::GtEq)),
            TokenKind::KwIs => (10, InfixKind::Is),
            TokenKind::Bang if matches!(self.peek_at(1).kind, TokenKind::KwIs) => {
                (10, InfixKind::IsNot)
            }
            TokenKind::Plus => (14, InfixKind::Binary(BinaryOp::Add)),
            TokenKind::Minus => (14, InfixKind::Binary(BinaryOp::Sub)),
            TokenKind::Star => (16, InfixKind::Binary(BinaryOp::Mul)),
            TokenKind::Slash => (16, InfixKind::Binary(BinaryOp::Div)),
            TokenKind::Percent => (16, InfixKind::Binary(BinaryOp::Rem)),
            _ => return None,
        })
    }

    /// [is-check] An `is`/`!is` tail, folded onto `lhs` as the subject. `not`
    /// is the `!is` form, sugar for `!(x is Q)` (user decision 2026-09-22):
    /// one `Expr::Is` under a `Not`, so narrowing, `when` heads and the guard
    /// rule all reach it unchanged.
    fn parse_is_tail(&mut self, lhs: Expr, not: bool) -> Option<Expr> {
        let start = if not { self.bump().span } else { lhs.span() };
        self.bump(); // `is`
        let (check, binding, lifted) = self.parse_is_check()?;
        let end = binding
            .as_ref()
            .map(|b| b.span)
            .or_else(|| check.last().map(|r| r.span))
            .unwrap_or_else(|| lhs.span());
        if not {
            // A negated test narrows nothing in its *then* branch, so there is
            // no value for a binding to name.
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
            return Some(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(inner),
                span: start.to(end),
            });
        }
        let span = lhs.span().to(end);
        self.is_or_widen(lhs, check, binding, lifted, span)
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
            // [type-literal] `is "STANDARD"`, `is 3`, `is true`: a literal
            // tests the value. It rides the check as a type ref spelled as
            // the literal ([`TypeLit::from_ref_name`] reads it back), and
            // takes no qualifiers or binding of its own.
            if refs.is_empty() {
                if let Some(Type::Literal { value, span }) = self.parse_type_literal() {
                    refs.push(value.as_type_ref(span));
                    break;
                }
            }
            // [namespace] `is .S` — the leading-dot sugar: the namespace is
            // left to the subject type [namespace]. A `.` then an uppercase
            // name opens a dot-ref, which `parse_type_ref` reads.
            if self.at(&TokenKind::Dot)
                && matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
            {
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

    /// [elvis] The **positional pick** operator tail, folded onto an
    /// already-parsed `subject`. Dispatched from `parse_binary` at the elvis
    /// [elvis] A `?:` tail. The subject's first arm is the value when it holds
    /// it; otherwise `rhs` runs with `_` bound to the arms left behind.
    /// Returns `Some(expr)` when a `?:` was consumed, `Some(None)` when the
    /// current token is not a `?:` here (a lone `?`/`:`, or a next-line `?:`
    /// left for an enclosing chain) — the loop then stops at this tier.
    ///
    /// Precedence is Kotlin's (tighter than comparison, looser than
    /// additive), and the chain is **left-associative** — the loop folds it,
    /// and the right side is parsed one tier up (additive, bp 13+), not
    /// recursively, which is what makes `a ?: b ?: c` read `(a ?: b) ?: c`.
    ///
    /// The right side is an ordinary expression — which since [expr-escape]
    /// includes `return`/`break`/`continue`, so `x ?: return _` needs no
    /// exception.
    fn parse_elvis_tail(&mut self, subject: Expr) -> Option<Option<Expr>> {
        // `?:` is a `?` immediately followed by a `:` (two separate tokens
        // [elvis]). Anything else at this tier is not a pick.
        if !matches!(self.kind(), TokenKind::Question)
            || !matches!(self.peek_at(1).kind, TokenKind::Colon)
        {
            return Some(None);
        }
        // [elvis-wrap] `?:` may start a continuation line, so a chain can wrap
        // (`a ?: b\n    ?: c`); a line beginning with `?` is unambiguous
        // (nothing in Salvo starts a statement with `?`). But inside a nested
        // escape right side (`elvis_no_wrap`) the operator belongs to the
        // *enclosing* chain: a `return`/escape value parses a full expression,
        // and left-associativity means a following `?:` brackets the whole
        // chain so far. A parenthesised sub-expression (`group_depth > 0`) is
        // self-contained, so the flag does not reach into it.
        if self.elvis_no_wrap && self.group_depth == 0 {
            return Some(None);
        }
        self.bump(); // `?`
        self.expect(&TokenKind::Colon)?;
        // The right side is an operand one tier up (additive, bp 13+), *not* a
        // recursive elvis: that is what makes the chain left-associative. It
        // may still be a `return`/escape, whose value must not wrap a next-line
        // `?:` — that operator brackets the whole chain so far on its left
        // (`elvis_no_wrap`), not the escape's value.
        let saved = std::mem::replace(&mut self.elvis_no_wrap, true);
        let rhs = self.sub_expr(|p| p.parse_binary(13));
        self.elvis_no_wrap = saved;
        let rhs = rhs?;
        let span = subject.span().to(rhs.span());
        Some(Some(Expr::Elvis {
            subject: Box::new(subject),
            rhs: Box::new(rhs),
            span,
        }))
    }

    /// [expr-pipe] A pipe `|>` tail, folded onto an already-parsed `lhs`. The
    /// left side is evaluated once and bound to `_`, read in the right side.
    /// Dispatched from `parse_binary` at the pipe tier (bp 2, the loosest), so
    /// `a + b |> f(_)` pipes the whole sum and `x |> f(_) |> g(_)` folds left
    /// into `g(f(x))`.
    ///
    /// The right side is parsed one tier up (bp 3), which — since the pipe is
    /// the loosest operator — is every tighter operator, i.e. a full
    /// expression stopping only at the next `|>`. That is what makes the chain
    /// left-associative. The `_` it must contain is checked later
    /// ([placeholder]); here it is an ordinary expression, so `_` parses as a
    /// placeholder exactly as it does in an elvis right side.
    /// [expr-safe-pipe] A null-safe pipe `?>` tail, folded onto `lhs`. `v ?>
    /// f(_)` runs `f(_)` only when `v` is present (`T` of a `T?`), binding the
    /// narrowed value to `_`, and rewraps the result `X` as `X?`; when `v` is
    /// absent the result is `None` and the right side never runs — the pipe
    /// analogue of `?.`. Same tier and newline behaviour as `|>`: the right
    /// side is parsed at bp 3 (every tighter operator).
    fn parse_safe_pipe_tail(&mut self, lhs: Expr) -> Option<Expr> {
        self.bump(); // `?`
        self.bump(); // `>`
        let rhs = self.sub_expr(|p| p.parse_binary(3))?;
        let span = lhs.span().to(rhs.span());
        Some(Expr::SafePipe {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        })
    }

    fn parse_pipe_tail(&mut self, lhs: Expr) -> Option<Expr> {
        self.bump(); // `|>`
        // The right side is parsed at bp 3 — every operator tighter than the
        // pipe, i.e. a full expression stopping only at the next `|>`. It is
        // *not* under `elvis_no_wrap`: unlike an elvis right side (parsed at
        // additive level, so a following elvis must be left for the chain),
        // the pipe is the loosest operator, so a tighter operator in its right
        // side (`x |> f(_) ?: g(_)` is `x |> (f(_) ?: g(_))`) belongs to the
        // right side and the bp-3 recursion absorbs it correctly.
        let rhs = self.sub_expr(|p| p.parse_binary(3))?;
        let span = lhs.span().to(rhs.span());
        Some(Expr::Pipe {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span,
        })
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        match self.kind() {
            // [inc-dec] `++i` / `--i`: the step happens first, so the
            // expression's value is the *new* one.
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let down = matches!(self.kind(), TokenKind::MinusMinus);
                let start = self.bump().span;
                let operand = self.sub_expr(|p| p.parse_unary())?;
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
                let operand = self.sub_expr(|p| p.parse_unary())?;
                let span = start.to(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Bang => {
                let start = self.bump().span;
                let operand = self.sub_expr(|p| p.parse_unary())?;
                let span = start.to(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Ellipsis => {
                let start = self.bump().span;
                let operand = self.sub_expr(|p| p.parse_unary())?;
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
        let expr = self.parse_primary()?;
        self.parse_postfix_from(expr)
    }

    /// The trailer loop (`.field`, `(call)`, `[index]`, `?.`, `!`, `@`),
    /// starting from an already-parsed base. Same-line only: a trailer that
    /// begins a continuation line is left for [`Self::newline_postfix`],
    /// which brackets it onto the whole expression (user decision
    /// 2026-10-12, [expr-wrap]).
    fn parse_postfix_from(&mut self, mut expr: Expr) -> Option<Expr> {
        loop {
            match self.kind() {
                TokenKind::Dot if self.same_line() => {
                    self.bump();
                    // `t.0` — a tuple element by constant index
                    // [expr-tuple-index].
                    if let Some(indexed) = self.tuple_index_suffix(&expr) {
                        expr = indexed;
                        continue;
                    }
                    // [comptime-access] `v.[field]` — a read by compile-time
                    // name, the binder in brackets; the expansion rewrites it
                    // to the field the copy is for.
                    if self.at(&TokenKind::LBracket) {
                        let open = self.bump().span;
                        self.require_compfn(open, "`.[field]`");
                        let binder = self.ident_value("binder")?;
                        let close = self.expect(&TokenKind::RBracket)?.span;
                        let field = Ident {
                            name: format!("[{}]", binder.name),
                            span: open.to(close),
                        };
                        let span = expr.span().to(close);
                        expr = Expr::Field {
                            base: Box::new(expr),
                            field,
                            span,
                        };
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
                    // [handler-init] `self@Face`: the enclosing handler's own
                    // address as the face named — a type ref, so a generic
                    // face (`self@ActorGroup<E>`) parses with its arguments.
                    if matches!(&expr, Expr::Ident(id) if id.name == SELF_SELECTOR)
                        && matches!(self.kind(), TokenKind::Ident(n) if n.chars().next().is_some_and(|c| c.is_uppercase()))
                    {
                        let face = self.parse_type_ref()?;
                        let span = expr.span().to(face.span);
                        expr = Expr::SelfAddr { face, span };
                        continue;
                    }
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
                    // [eff-handle] `e[A, B]` / `e[IO]` — projection of an
                    // effect handle: the bracket holds capitalised effect/alias
                    // **names**, each followed by `,` or `]`. Anything else is
                    // array indexing (a value expression). The first token
                    // being a capitalised ident that `,`/`]` follows is the
                    // tell — a value expression would continue (`.`, `(`, `[`,
                    // an operator) or be lowercase/a literal.
                    let is_projection = matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase()))
                        && matches!(self.peek_at(2).kind, TokenKind::Comma | TokenKind::RBracket);
                    if is_projection {
                        let start = self.bump().span; // `[`
                        self.group_depth += 1;
                        let mut effects: Vec<TypeRef> = Vec::new();
                        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
                            match self.parse_type_ref() {
                                Some(r) => effects.push(r),
                                None => {
                                    self.group_depth -= 1;
                                    return None;
                                }
                            }
                            if self.eat(&TokenKind::Comma).is_none() {
                                break;
                            }
                        }
                        self.group_depth -= 1;
                        let end = self.expect(&TokenKind::RBracket)?.span;
                        let _ = start;
                        let pspan = expr.span().to(end);
                        expr = Expr::Project { base: Box::new(expr), effects, span: pspan };
                        continue;
                    }
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
                // [safe-nav] `base?.field` / `base?.method(args)`: a `?`
                // immediately followed by `.`. A single `?` run ending in
                // `:` is the elvis operator and belongs to the lower
                // precedence tier (`parse_elvis`), so only `?.` is taken
                // here. One `?.` guards one link: the field, and a call on
                // it when `(` follows on the same line.
                TokenKind::Question
                    if self.same_line()
                        && matches!(self.peek_at(1).kind, TokenKind::Dot) =>
                {
                    self.bump(); // `?`
                    self.bump(); // `.`
                    let field = self.ident()?;
                    let (access, end) = if self.at(&TokenKind::LParen) && self.same_line() {
                        let (args, named, end) = self.parse_call_args()?;
                        (SafeAccess::Call { field, type_args: Vec::new(), args, named }, end)
                    } else {
                        let fspan = field.span;
                        (SafeAccess::Field(field), fspan)
                    };
                    let span = expr.span().to(end);
                    expr = Expr::SafeNav {
                        base: Box::new(expr),
                        access,
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
                // [actor-replyto] [expr-replyto-flat] A trailing `replyto k(…)`
                // (or `replyto!`) after a call fills that call's final
                // `Reply<T>` argument — the flattened form of
                // `send(…, replyto k(…))` (user decision 2026-10-20). Same
                // line here; a newline-leading one is taken by
                // `newline_postfix`, bracketing left like `.`.
                TokenKind::Ident(name)
                    if self.same_line()
                        && name == "replyto"
                        && self.replyto_follows() =>
                {
                    expr = self.append_replyto(expr)?;
                }
                _ => break,
            }
        }
        Some(expr)
    }

    /// [expr-wrap] A trailer that begins a *continuation line* — a `.field`
    /// or `.method(…)` after a newline, outside any brackets — brackets the
    /// **whole** expression so far on its left (user decision 2026-10-12),
    /// the same rule the elvis operator follows [elvis-wrap]. So
    /// `a ?: b\n    .m()` is `(a ?: b).m()`, and a wrapped method chain
    /// `x\n    .a()\n    .b()` is `((x).a()).b()`. Nothing in Salvo starts a
    /// statement with `.`, so a leading dot is unambiguous. Applied once the
    /// full expression is parsed.
    fn newline_postfix(&mut self, mut expr: Expr) -> Option<Expr> {
        // Each pass takes one continuation line: a newline-leading `.` or a
        // newline-leading `replyto` outside any brackets. Its own same-line
        // trailers (`.a().b[0]`) attach normally; the next newline trailer is
        // a fresh pass, so every newline brackets what precedes it on the left.
        while self.group_depth == 0 && !self.elvis_no_wrap && !self.same_line() {
            // [actor-replyto] [expr-replyto-flat] A newline-leading `replyto`
            // fills the preceding call's final `Reply<T>` argument, bracketing
            // the whole call chain so far on its left.
            if self.at_word("replyto") && self.replyto_follows() {
                expr = self.append_replyto(expr)?;
                continue;
            }
            if !self.at(&TokenKind::Dot) {
                break;
            }
            // Consume the leading `.member` (and any same-line trailers after
            // it) with the whole expression as the base. The newline barrier
            // is lifted only for this first dot by stepping over it here and
            // reading the member directly, then handing back to the same-line
            // trailer loop.
            self.bump(); // the continuation `.`
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
            // Same-line trailers after the member (`()`, `[i]`, further
            // `.x`) attach here as usual.
            expr = self.parse_postfix_from(expr)?;
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
    /// [struct-lit-condition] Inside parentheses a `{` cannot open the
    /// condition's block, so a struct literal is allowed again there:
    /// `for n in notices(Path { text: t }, s) { … }`.
    fn parse_call_args(&mut self) -> Option<(Vec<Expr>, Vec<NamedArg>, Span)> {
        let saved = std::mem::replace(&mut self.no_struct, false);
        let out = self.parse_call_args_inner();
        self.no_struct = saved;
        out
    }

    fn parse_call_args_inner(&mut self) -> Option<(Vec<Expr>, Vec<NamedArg>, Span)> {
        self.expect(&TokenKind::LParen)?;
        self.group_depth += 1;
        let mut args = Vec::new();
        let mut named: Vec<NamedArg> = Vec::new();
        // [col-pairs] Whether this call uses the `k: v` pair sugar. Each pair
        // becomes a 2-tuple `(k, v)` filling a `...pairs: (K, V)[]` variadic
        // (so `Map("a": 1)` is `Map(("a", 1))`). Pairs are all-or-nothing
        // in one call (user decision 2026-10-12, Q6): no mixing with
        // positional or `=`-named arguments.
        let mut saw_pair = false;
        let mut saw_plain = false;
        while !self.at(&TokenKind::RParen) && !self.at_eof() {
            let Some(arg) = self.parse_expr() else {
                self.group_depth -= 1;
                return None;
            };
            if self.at(&TokenKind::Colon) {
                // [col-pairs] `k: v` — a key-value pair argument.
                let colon = self.bump().span;
                if saw_plain || !named.is_empty() {
                    self.error(
                        "a `k: v` pair argument cannot be mixed with positional \
                         or named arguments: a call uses pairs for every argument \
                         or none",
                        colon,
                    );
                    self.group_depth -= 1;
                    return None;
                }
                let Some(value) = self.parse_expr() else {
                    self.group_depth -= 1;
                    return None;
                };
                saw_pair = true;
                let span = arg.span().to(value.span());
                args.push(Expr::Tuple {
                    elems: vec![arg, value],
                    span,
                });
            } else if self.at(&TokenKind::Eq) {
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
                if saw_pair {
                    let span = arg.span();
                    self.error(
                        "a positional argument cannot be mixed with `k: v` pair \
                         arguments: a call uses pairs for every argument or none",
                        span,
                    );
                    self.group_depth -= 1;
                    return None;
                }
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
                saw_plain = true;
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
        // [comptime-inline] `[arm.type] { … }`: a struct literal whose head is
        // a binder's type, spliced per copy (ARG_PARSE.md §13 (B)).
        if self.at_type_splice() && !self.no_struct {
            let snap = self.snapshot();
            if let Some(ty) = self.parse_type_atom() {
                let shorthand = matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                    && matches!(self.peek_at(2).kind, TokenKind::Comma | TokenKind::RBrace);
                if self.at(&TokenKind::LBrace)
                    && self.same_line()
                    && (self.brace_is_struct_lit() || shorthand || matches!(self.peek_at(1).kind, TokenKind::RBrace))
                {
                    return self.parse_struct_lit_body(Some(ty));
                }
            }
            self.rollback(snap);
            let span = self.peek().span;
            self.error(
                "a type splice in an expression is the head of a struct literal: \
                 `[arm.type] { … }` [comptime-inline]",
                span,
            );
            return None;
        }
        match self.kind().clone() {
            // [eff-handle] `use H(...)` as an expression — the value-producing
            // form, yielding the registered handle. `use` is a real keyword,
            // so no contextual guard.
            TokenKind::KwUse => {
                let start = self.bump().span;
                self.parse_use_expr(start)
            }
            // [namespace] `.S { … }` / `.S(…)` / `.S` — the leading-dot
            // shorthand in expression position: a namespaced type whose
            // namespace is left to the expected type (D3/D4). A `.` followed
            // by an uppercase name opens a dot-type atom (`parse_type_atom`
            // reads the `.S`/`.S.T` name); a `{` after it is a struct literal,
            // a `(` a type-fn call, otherwise the bare type value. The checker
            // resolves the `.`-prefixed name against the expected type.
            TokenKind::Dot
                if matches!(&self.peek_at(1).kind, TokenKind::Ident(n) if n.starts_with(|c: char| c.is_uppercase())) =>
            {
                let snap = self.snapshot();
                if let Some(ty) = self.parse_type_atom() {
                    // Struct literal: `.S { … }`.
                    if self.at(&TokenKind::LBrace) && self.same_line() && self.brace_is_struct_lit() {
                        return self.parse_struct_lit_body(Some(ty));
                    }
                    // Type-fn call: `.S(…)`.
                    if self.at(&TokenKind::LParen) && self.same_line() {
                        if let Some((args, named, end)) = self.parse_call_args() {
                            let (key, _) = type_fn_key(&ty);
                            let type_args = match &ty {
                                Type::Named { base, .. } => base.args.clone(),
                                _ => Vec::new(),
                            };
                            let callee = Expr::Ident(Ident { name: key, span: ty.span() });
                            return Some(Expr::Call {
                                callee: Box::new(callee),
                                type_args,
                                args,
                                named,
                                span: ty.span().to(end),
                            });
                        }
                    }
                    // A bare dot-type value (`.S` as a fieldless literal, or
                    // where the expected type makes it a value): a fieldless
                    // struct literal carries the type only.
                    return Some(Expr::StructLit { ty: Some(ty.clone()), fields: Vec::new(), span: ty.span() });
                }
                self.rollback(snap);
                let found = self.kind().describe();
                let span = self.peek().span;
                self.error(format!("expected expression, found {found}"), span);
                None
            }
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
                if (name == ASSERT_FORM || name == UNREACHABLE_FORM || name == TODO_FORM)
                    && matches!(self.peek_at(1).kind, TokenKind::Bang)
                    && matches!(self.peek_at(2).kind, TokenKind::LParen) =>
            {
                let assert = name == ASSERT_FORM;
                let is_todo = name == TODO_FORM;
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
                    None if is_todo => Expr::Todo { message, span },
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
                // [terminal-scope] An escape's value is a sub-expression.
                let value = if self.stmt_value_follows() {
                    Some(Box::new(self.sub_expr(|p| p.parse_expr())?))
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
                // [eff-handle] `eff[a, b]` — **merge**: combine already-bound
                // handle values into one. Contextual (`eff` before `[`), like
                // the type former; the operands are value expressions, and
                // `eff[*]` is every handle in scope.
                if self.at_word(EFF_WORD) && matches!(self.peek_at(1).kind, TokenKind::LBracket) {
                    return self.parse_eff_merge();
                }
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
            TokenKind::LParen => {
                // [struct-lit-condition] As in call arguments.
                let saved = std::mem::replace(&mut self.no_struct, false);
                let e = self.parse_paren_expr();
                self.no_struct = saved;
                e
            }
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
            // [fn-type-named] A capitalized type in call position names a
            // type-fn: `Mut List(1, 2, 3)`, `List(…)`, `Heap List(…)`,
            // `Ok(x)`. Parse the type *name* (stopping before `(`) first; if
            // a `(` follows it is the call's argument list (Q1 — a
            // capitalized callee is a type-fn lookup). Otherwise roll back to
            // the struct-literal / bare-ident handling below.
            if self.at_type_name() {
                let snap = self.snapshot();
                if let Some(ty) = self.parse_type_call_name() {
                    // A `(` here is the call's argument list — UNLESS a `{`
                    // follows the parens, in which case this was a struct
                    // literal with value-args (`Ranked<T>(?cmp) { … }`), or
                    // the parens hold value-args (`?`-binders / identities)
                    // rather than call arguments. Both roll back to the
                    // struct-literal / value-args handling below.
                    if self.at(&TokenKind::LParen) && self.same_line() {
                        if let Some((args, named, end)) = self.parse_call_args() {
                            if !(self.at(&TokenKind::LBrace) && self.same_line()) {
                                let (key, _) = type_fn_key(&ty);
                                let type_args = match &ty {
                                    Type::Named { base, .. } => base.args.clone(),
                                    _ => Vec::new(),
                                };
                                let callee = Expr::Ident(Ident { name: key, span: ty.span() });
                                return Some(Expr::Call {
                                    callee: Box::new(callee),
                                    type_args,
                                    args,
                                    named,
                                    span: ty.span().to(end),
                                });
                            }
                        }
                    }
                }
                self.rollback(snap);
            }
            let snap = self.snapshot();
            if let Some(ty) = self.parse_type_atom() {
                // [col-no-literal] With a type name in front, `{}` is a
                // fieldless struct literal (`Finished {}`). A bare `{}` with
                // no name builds nothing — collection literals are retired.
                // [struct-lit-shorthand] `Name { x, y }`: with a type name in
                // front, a bare field name is `x: x`.
                let shorthand = matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                    && matches!(self.peek_at(2).kind, TokenKind::Comma | TokenKind::RBrace);
                let braced_body = self.at(&TokenKind::LBrace)
                    && self.same_line()
                    && (self.brace_is_struct_lit()
                        || shorthand
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
            // [col-no-literal] A bare `{}` is not a struct literal (a
            // fieldless struct is written with its name, `Finished {}`), and
            // collection literals are retired — so `parse_brace_expr` reports
            // it rather than reading it as an empty collection.
            TokenKind::RBrace => false,
            TokenKind::Ellipsis => true,
            // [comptime-access] `{ [field]: … }` and `{ inline for … }` open a
            // struct literal only inside a compfn; elsewhere a `[` after `{`
            // has no meaning now that collection literals are retired
            // [col-no-literal].
            TokenKind::LBracket => {
                self.in_compfn
                    && (matches!(self.peek_at(2).kind, TokenKind::KwFor)
                        || (matches!(self.peek_at(2).kind, TokenKind::Ident(_))
                            && matches!(self.peek_at(3).kind, TokenKind::RBracket)
                            && matches!(self.peek_at(4).kind, TokenKind::Colon)))
            }
            TokenKind::Ident(_) => matches!(self.peek_at(2).kind, TokenKind::Colon),
            // [comptime-fields] `type` is a field name a struct may declare
            // (`parse_field_decl`'s carve-out for `core.comptime`'s
            // `Field.type`), so `{ type: … }` must be recognised as a struct
            // literal even when `type:` is its *first* field — the ordinary
            // `Ident(_)` arm above never matches `KwType`, so without this
            // a literal spelled `P { type: 1 }` fell through to `_ => false`
            // and was misread as "neither a struct literal nor a lambda"
            // (2026-10-19).
            TokenKind::KwType => matches!(self.peek_at(2).kind, TokenKind::Colon),
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
        let fields = self.parse_struct_lit_entries();
        self.group_depth -= 1;
        let fields = fields?;
        let end = self.expect(&TokenKind::RBrace)?.span;
        Some(Expr::StructLit {
            ty,
            fields,
            span: start.to(end),
        })
    }

    /// The entries of a struct literal up to its closing brace: `name: expr`,
    /// `...spread`, and inside a compfn `[field]: expr` and `inline for field in
    /// T.fields { entries }` [comptime-access] [comptime-inline].
    fn parse_struct_lit_entries(&mut self) -> Option<Vec<StructLitField>> {
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if self.at(&TokenKind::Ellipsis) {
                let spread_start = self.bump().span;
                let value = self.parse_expr()?;
                let span = spread_start.to(value.span());
                fields.push(StructLitField {
                    kind: StructLitFieldKind::Spread(value),
                    span,
                });
            } else if self.at(&TokenKind::LBracket) && matches!(self.peek_at(1).kind, TokenKind::KwFor)
            {
                let start = self.bump().span;
                self.require_compfn(start, "a `[for …]` head");
                self.bump();
                let binder = self.ident_value("binder")?;
                self.expect(&TokenKind::KwIn)?;
                let seq = self.parse_comp_seq()?;
                self.expect(&TokenKind::RBracket)?;
                self.expect(&TokenKind::LBrace)?;
                let entries = self.parse_struct_lit_entries()?;
                let end = self.expect(&TokenKind::RBrace)?.span;
                fields.push(StructLitField {
                    kind: StructLitFieldKind::InlineFor {
                        binder,
                        seq,
                        entries,
                    },
                    span: start.to(end),
                });
            } else {
                let name = if self.at(&TokenKind::LBracket) {
                    let open = self.bump().span;
                    self.require_compfn(open, "`[field]:`");
                    let binder = self.ident_value("binder")?;
                    let close = self.expect(&TokenKind::RBracket)?.span;
                    Ident {
                        name: format!("[{}]", binder.name),
                        span: open.to(close),
                    }
                } else if self.at(&TokenKind::KwType) && matches!(self.peek_at(1).kind, TokenKind::Colon) {
                    // [comptime-fields] `type` is a keyword, but `Field.type`
                    // (and any struct declaring a field spelled that way,
                    // following the same carve-out `parse_field_decl` makes)
                    // must still be constructible as a struct literal — the
                    // field label and the ordinary `type` item/type-position
                    // keyword never collide, since this only fires with a
                    // `:` immediately after.
                    let span = self.bump().span;
                    Ident { name: "type".to_string(), span }
                } else {
                    self.ident()?
                };
                // [struct-lit-shorthand] `{ x }` is `{ x: x }`, the inverse
                // of destructuring.
                let value = if !name.name.starts_with('[')
                    && matches!(self.kind(), TokenKind::Comma | TokenKind::RBrace)
                {
                    Expr::Ident(name.clone())
                } else {
                    self.expect(&TokenKind::Colon)?;
                    self.parse_expr()?
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
        Some(fields)
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
    /// A `{` in expression position: either a bare struct literal
    /// (`{name: ..., age: ...}`) or a brace lambda (`{ i: Int -> 0 }`).
    /// Collection literals are retired [col-no-literal]: a collection is
    /// built with its type-named constructor (`List(1, 2)`, `Set("a")`,
    /// `Map("k": v)`), so a brace that is neither a lambda nor a struct
    /// literal is a parse error here.
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
        // [col-no-literal] Neither a lambda nor a struct literal. The old
        // brace-collection reading is gone; point at the constructor form.
        let span = self.peek().span;
        self.error(
            "a `{...}` here is neither a struct literal nor a lambda. Collection \
             literals are retired: build a collection with its constructor — \
             `List(1, 2, 3)`, `Set(\"a\", \"b\")`, `Map(\"k\": v)` (or the `Mut`, \
             `Sorted…` variants) [col-no-literal]",
            span,
        );
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
        // [terminal-scope] A condition is a sub-expression: no terminal `...`
        // in it, but the construct's *body* keeps the statement's flag.
        let result = self.sub_expr(|p| p.parse_expr());
        self.no_struct = saved;
        result
    }

    fn parse_if(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwIf)?.span;
        let cond = self.parse_condition()?;
        // [terminal-scope] `if cond ...` — the then-body is the rest of the
        // enclosing block. An optional braced `else` is written inline right
        // after the `...`, before that rest; no `elif`, no `else ...`.
        if self.at(&TokenKind::Ellipsis) {
            if !self.allow_terminal || self.group_depth != 0 {
                let span = self.peek().span;
                self.error(
                    "`...` is a terminal scope — the rest of the enclosing block \
                     becomes the `if`'s then-body — so it is only a statement, \
                     never a value or a nested expression: write a `{ … }` block",
                    span,
                );
                return None;
            }
            self.bump(); // `...`
            let mut else_block = None;
            if self.at(&TokenKind::KwElif) {
                let span = self.peek().span;
                self.error(
                    "a terminal `if … ...` cannot have `elif`: the `...` body is \
                     the rest of the block, so there is nowhere for an `elif` to \
                     go — write the braced `if { … } elif { … }` form",
                    span,
                );
                return None;
            }
            if self.at(&TokenKind::KwElse) {
                self.bump();
                if self.at(&TokenKind::Ellipsis) {
                    let span = self.peek().span;
                    self.error(
                        "`else ...` is not allowed on a terminal `if`: only the \
                         `if` branch is the terminal scope, and its `else` is an \
                         ordinary `{ … }` block",
                        span,
                    );
                    return None;
                }
                else_block = Some(self.parse_block()?);
            }
            // Now the rest of the enclosing block is the then-body.
            let then_block = self.parse_terminal_rest();
            let end = else_block
                .as_ref()
                .map(|b| b.span)
                .unwrap_or(then_block.span);
            return Some(Expr::If {
                branches: vec![(cond, then_block)],
                else_block,
                span: start.to(end),
            });
        }
        let block = self.parse_block()?;
        let mut branches = vec![(cond, block)];
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

    /// `try { ... }` / `try ...` — the throw delimiter [try]. A braced block,
    /// or a terminal `...` scope [terminal-scope] when `try` is a statement.
    fn parse_try(&mut self) -> Option<Expr> {
        let start = self.expect(&TokenKind::KwTry)?.span;
        if !self.at(&TokenKind::LBrace) && !self.at(&TokenKind::Ellipsis) {
            let span = self.peek().span;
            self.error("`try` takes a block: `try { ... }` (or `try ...`)", span);
            return None;
        }
        let (body, _terminal) = self.parse_body("the `try`")?;
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
        // [actor-group] `in GROUP`: join the spawned actor to an actor group.
        // After `on`, since a placement reads before a membership; a real
        // keyword (`for x in xs`), so no contextual guard is needed.
        let mut join = None;
        if self.at(&TokenKind::KwIn) && self.same_line() {
            self.bump();
            let expr = self.parse_expr()?;
            end = expr.span();
            join = Some(Box::new(expr));
        }
        Some(Expr::Spawn {
            handler: Box::new(handler),
            with_items,
            pool,
            join,
            span: start.to(end),
        })
    }

    /// [eff-handle] `use H(args) [with …] [on POOL]` as an **expression**:
    /// the `use` keyword is already consumed; `start` is its span. Registers
    /// the handler and yields its handle. The `on POOL` form keeps its old
    /// sugar — a `spawn` bound as an addr — so the `pool` field is passed
    /// through and the checker/IR handle it as they did for the statement.
    /// [eff-handle] `eff[a, b]` / `eff[*]` — the merge constructor in value
    /// position: `eff` then `[`, with value operands (or a lone `*` for every
    /// in-scope handle). Distinct from the type former by context (this is
    /// `parse_primary`).
    fn parse_eff_merge(&mut self) -> Option<Expr> {
        let start = self.bump().span; // `eff`
        self.expect(&TokenKind::LBracket)?;
        self.group_depth += 1;
        // `eff[*]`: every handle in scope.
        if self.at(&TokenKind::Star) {
            self.bump();
            self.group_depth -= 1;
            let end = self.expect(&TokenKind::RBracket)?.span;
            return Some(Expr::Merge { items: Vec::new(), all: true, span: start.to(end) });
        }
        let mut items: Vec<Expr> = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_eof() {
            match self.parse_expr() {
                Some(e) => items.push(e),
                None => {
                    self.group_depth -= 1;
                    return None;
                }
            }
            if self.eat(&TokenKind::Comma).is_none() {
                break;
            }
        }
        self.group_depth -= 1;
        let end = self.expect(&TokenKind::RBracket)?.span;
        Some(Expr::Merge { items, all: false, span: start.to(end) })
    }

    fn parse_use_expr(&mut self, start: Span) -> Option<Expr> {
        let handler = self.parse_expr()?;
        let with_items = self.parse_with_clause()?;
        // [actor-use-addr] `use H(args) on POOL` — the sugar: lower to a
        // `spawn` handler bound as an addr, exactly as the statement form
        // does, so one code path serves both. The clause belongs to the
        // spawn (the instance is the child's).
        if self.at_word("on") && self.same_line() {
            self.bump();
            let pool = self.parse_expr()?;
            let span = start.to(pool.span());
            let spawn_span = handler.span().to(pool.span());
            let handler = Expr::Spawn {
                handler: Box::new(handler),
                with_items,
                pool: Some(Box::new(pool)),
                join: None,
                span: spawn_span,
            };
            return Some(Expr::Use {
                handler: Box::new(handler),
                with_items: Vec::new(),
                span,
            });
        }
        let end = with_items.last().map(|i| i.span()).unwrap_or_else(|| handler.span());
        Some(Expr::Use {
            handler: Box::new(handler),
            with_items,
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

    /// [expr-replyto-flat] Whether a `replyto` word at the current position is
    /// the flattened reply operator: `replyto k(…)` or `replyto! k(…)` — a
    /// member name (or `!` then a name) follows. The same lookahead the
    /// primary-position mint uses, so a variable or call named `replyto` is
    /// untouched.
    fn replyto_follows(&self) -> bool {
        matches!(self.peek_at(1).kind, TokenKind::Ident(_))
            || (matches!(self.peek_at(1).kind, TokenKind::Bang)
                && matches!(self.peek_at(2).kind, TokenKind::Ident(_)))
    }

    /// [expr-replyto-flat] Append a flattened `replyto k(…)` to the preceding
    /// call, filling its final `Reply<T>` argument (user decision 2026-10-20).
    /// `send(a, b) replyto k(c)` is exactly `send(a, b, replyto k(c))`: the
    /// minted continuation becomes the call's last positional argument, so no
    /// checker or backend rule knows the flattened spelling exists. The base
    /// must be a call — a `replyto` after anything else has no argument slot
    /// to fill, which is an error naming the shape.
    ///
    /// A **chain** nests rightward: `a() replyto b() replyto c()` is
    /// `a(replyto b(replyto c()))` — each further `replyto` fills the previous
    /// mint's own reply slot, not the outer call's (user decision 2026-10-20).
    /// `parse_replyto_chain` builds that nested mint; this only appends it.
    fn append_replyto(&mut self, base: Expr) -> Option<Expr> {
        let reply = self.parse_replyto_chain()?;
        match base {
            Expr::Call { callee, type_args, mut args, named, span } => {
                let span = span.to(reply.span());
                args.push(reply);
                Some(Expr::Call { callee, type_args, args, named, span })
            }
            // Nesting a chain: the previous mint's own `Reply<T>` slot is
            // filled by this one. The mint carries its continuation's args as
            // `captures`, so the fill is appended there.
            Expr::ReplyTo { member, mut captures, gated, pool, span } => {
                let span = span.to(reply.span());
                captures.push(reply);
                Some(Expr::ReplyTo { member, captures, gated, pool, span })
            }
            other => {
                self.error(
                    "a trailing `replyto` fills the final `Reply<T>` argument of \
                     the call before it (`send(x) replyto k(…)`), so it must \
                     follow a call",
                    other.span().to(reply.span()),
                );
                // Recover with the base unchanged; the reply is dropped.
                Some(other)
            }
        }
    }

    /// [expr-replyto-flat] One mint, with any further trailing `replyto`
    /// nested into *its* captures (right-associative). A trailing `replyto`
    /// may be same-line or begin a continuation line (bracketing left like the
    /// other flattening operators); either way it fills the mint it follows.
    fn parse_replyto_chain(&mut self) -> Option<Expr> {
        let reply = self.parse_replyto()?;
        // Another `replyto` after this mint — same line, or a newline-leading
        // one outside brackets — fills this mint's own `Reply<T>` slot.
        let follows = (self.same_line()
            || (self.group_depth == 0 && !self.elvis_no_wrap))
            && self.at_word("replyto")
            && self.replyto_follows();
        if follows {
            return self.append_replyto(reply);
        }
        Some(reply)
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
        if !self.at(&TokenKind::LBrace) && !self.at(&TokenKind::Ellipsis) {
            let span = self.peek().span;
            self.error(
                "`waitfor` takes a binder and a block that sends the token \
                 somewhere: `waitfor out { p.total(out) }`, or \
                 `waitfor out: Reply<Int> { … }` to write the type out",
                span,
            );
            return None;
        }
        let (body, _terminal) = self.parse_body("the `waitfor`")?;
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
        // [when-bind] `when name = subject { … }`: a fresh local bound to the
        // subject, scoped to the `when`. Recognized by an identifier followed
        // by a single `=` (not `==`, which is a comparison in the subject).
        let binding = if matches!(self.kind(), TokenKind::Ident(_))
            && matches!(self.peek_at(1).kind, TokenKind::Eq)
        {
            let name = self.ident()?;
            self.expect(&TokenKind::Eq)?;
            Some(name)
        } else {
            None
        };
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
            binding,
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
        let (body, terminal) = self.parse_body("the loop")?;
        let mut end = body.span;
        // [terminal-scope] A terminal loop body is the rest of the block, so
        // nothing — not even an `else` — can follow it.
        let else_block = if !terminal && self.at(&TokenKind::KwElse) {
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
        let (body, terminal) = self.parse_body("the loop")?;
        let mut end = body.span;
        let else_block = if !terminal && self.at(&TokenKind::KwElse) {
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
                    let (expr, diags) = parse_interpolated_expr(&source, offset, self.in_compfn);
                    self.diagnostics.extend(diags);
                    StrExprPart::Interp(Box::new(expr))
                }
            })
            .collect()
    }
}


/// Parses a `${...}` fragment as an expression, shifting all spans by
/// `offset` so they point back into the original file.
fn parse_interpolated_expr(source: &str, offset: u32, in_compfn: bool) -> (Expr, Vec<Diagnostic>) {
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
    // [comptime-inline] An interpolation inside a compfn body may read
    // `value.[field]` or `field.name` like the body around it.
    parser.in_compfn = in_compfn;
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

/// [fn-type-named] The canonical key and generic parameters of a type-named
/// function. The key is the qualifier names (in written order) then the base
/// name, space-joined (`Mut List<T>` → `"Mut List"`, `Ok<T>` → `"Ok"`,
/// `Heap List<T>` → `"Heap List"`); the generics are the base's bare
/// type-variable arguments (`<T>` → `[T]`). Qualifier arguments and nested
/// type arguments are not part of the key — the key names the *shape* of the
/// type being constructed, which is what callers and overloads agree on.
pub fn type_fn_key(ty: &Type) -> (String, Vec<Ident>) {
    match ty {
        Type::Named { qualifiers, base } => {
            let mut parts: Vec<String> = qualifiers.iter().map(|q| q.name.name.clone()).collect();
            parts.push(base.name.name.clone());
            let generics: Vec<Ident> = base
                .args
                .iter()
                .filter_map(|a| match a {
                    Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() => {
                        Some(base.name.clone())
                    }
                    _ => None,
                })
                .collect();
            // Join with `_`, not a space, so the key is a valid target-language
            // identifier (the backends spell a fn by this key directly) and
            // still cannot collide with a lowercase value-fn [name-casing].
            (parts.join("_"), generics)
        }
        // A non-`Named` type name (a group, union, …) is not a valid
        // type-fn name; a later check reports it, so the key only needs to
        // be stable and non-colliding.
        _ => ("<type-fn>".to_string(), Vec::new()),
    }
}

/// [proj-anywhere] The source parameter of the first `proj(p)` in a
/// type, searching arms, arguments and elements in order.
pub fn first_proj_source(ty: &Type) -> Option<Ident> {
    // [proj-infer] Only a *wholesale* `proj` — on the result itself, an
    // arm, a tuple element — makes a derived return. A `proj` inside a type
    // argument (`List<proj T>`) is a borrow the result *holds*, tracked as a
    // lend, so type arguments are not descended into.
    fn in_ref(r: &TypeRef) -> Option<Ident> {
        if r.name.name == "proj" || r.name.name == "ref" {
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

/// [terminal-lambda] Fills the single `LambdaBody::Terminal` placeholder in a
/// just-parsed statement with the captured rest-of-block, in place. There is at
/// most one per statement (the parser refuses a second).
fn fill_terminal_lambda(stmt: &mut Stmt, body: Block) {
    use crate::visit_mut::{self, MutVisitor};
    struct Fill {
        body: Option<Block>,
    }
    impl MutVisitor for Fill {
        fn visit_expr(&mut self, e: &mut Expr) {
            // The framework's `walk_expr` descends into children after this
            // returns, so we only act on the placeholder and never recurse
            // (calling `walk_expr` here would re-enter `visit_expr` on `e`).
            if let Expr::Lambda { body: lb, .. } = e {
                if matches!(lb, LambdaBody::Terminal) {
                    if let Some(b) = self.body.take() {
                        *lb = LambdaBody::Block(b);
                    }
                }
            }
        }
    }
    let mut fill = Fill { body: Some(body) };
    visit_mut::walk_stmt(&mut fill, stmt);
}
