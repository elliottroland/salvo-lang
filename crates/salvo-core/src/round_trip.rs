//! [round-trip] [dump-type-check] A checked file printed back as Salvo with
//! every choice the checker made written out (the pipeline record B4 and D4, §II.7;
//! user decisions P5 (b) and P24). One writer, two readings:
//!
//! * **The round trip** (`write_out`, `compare`): only what Salvo can say,
//!   so the written-out file is a program. Each call's callee as
//!   `name@module` (`name@Effect` for an effect's member), its inferred type
//!   arguments, its implicit arguments by name (`cmp = cmp@main`); an
//!   operator as the call it resolved to (`plus@std.core.basic(a, b)`,
//!   `cmp@…(a, b) < 0`); an interpolated part as its `to_str` call; and the
//!   types the checker inferred where Salvo writes one (an unannotated
//!   `let`, a lambda's parameters and block return, a generic struct
//!   literal's type arguments). Checked again, the program must make the
//!   same choices and infer the same types; a rule that decides differently
//!   on the implicit spelling than on the explicit one fails it, and so does
//!   a writer that writes the wrong thing.
//! * **The typed dump** (`salvo dump --phase type-check`, `dump_file`): the
//!   same, plus what has no surface spelling, marked `#{…}`: a coercion
//!   (`#{as T}`), a narrowing (`#{narrowed from T}`), the calls a `for`
//!   names (`#{next@std.core.list}`), and with `--expr-types` every
//!   expression's type (`#{: T}`). Types are spelled as the dumps spell
//!   them (`Point@app.shapes`) [dump-spelling].
//!
//! What has no surface spelling is skipped by the round trip: a call through
//! a local or an implicit of the enclosing fn, sends, a renamed name, a type
//! with no source form, an operator whose operand widens [op-promote] (a
//! written call widens nothing), and anything a desugaring or a stamp made.

use std::collections::BTreeMap;

use salvo_syntax::ast::{BinaryOp, Expr, FnDecl, Item, LambdaBody, Module, NamedArg, Pattern, Stmt, StrExprPart, Type};
use salvo_syntax::visit::{self, Visitor};
use salvo_syntax::Span;

use crate::check::{Callee, Checked, ConvertStep, FnValue, ImplicitArg, Key, NarrowBy};
use crate::program::Symbols;
use crate::types::Ty;
use crate::Program;

/// One file written back with its choices spelled out, and what the
/// re-check must find there.
pub struct Rewrite {
    /// The written-out source.
    pub text: String,
    /// Every call whose choice was written out, by its start in the
    /// rewritten text: the callee it must resolve to, and the type arguments
    /// it must infer (now written).
    pub calls: BTreeMap<u32, Vec<String>>,
    /// Every implicit argument written out: the call's start in the
    /// rewritten text, the argument's name, and the declaration (or member)
    /// its value must name.
    pub implicits: Vec<(u32, String, String)>,
    /// Every type written out, by where the node it is the type of starts in
    /// the rewritten text (a binding's name, a struct literal): the type the
    /// re-check must give it.
    pub types: BTreeMap<u32, String>,
}

/// What the writer is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The round trip: Salvo only.
    Source,
    /// The typed dump: marks too, and every expression's type when asked.
    Dump { expr_types: bool },
}

/// An edit of the original source: insert `text` at `at` (`end == at`), or
/// replace `at..end` with it. Edits at one offset are ordered by `class`
/// (what ends there, then a replacement, then what starts there) and then
/// `rank` (inner before outer for what ends, outer before inner for what
/// starts).
struct Edit {
    at: u32,
    end: u32,
    text: String,
    class: u8,
    rank: i64,
}

/// Where an expectation is in the rewritten text: at an original offset (a
/// written node's start), or where an edit's text begins (a call the writer
/// spelled, an operator's).
#[derive(Clone, Copy)]
enum Anchor {
    Orig(u32),
    /// An edit's text start, and how far into it.
    Edit(usize, u32),
}

enum Expect {
    Call(String),
    Implicit(String, String),
    Type(String),
}

const ENDS: u8 = 0;
const REPLACES: u8 = 1;
const STARTS: u8 = 2;

/// Writes `file` of a checked `program` back with its choices spelled out:
/// the round trip's source.
pub fn write_out(program: &Program, symbols: &Symbols<'_>, checked: &Checked, file: usize) -> Rewrite {
    crate::types::with_keys_shown(|| Writer::new(program, symbols, checked, file, Mode::Source).run())
}

/// [dump-type-check] File `file` of a checked `program` as the typed dump
/// prints it: the round trip's writing, with what Salvo cannot spell marked
/// `#{…}`, and every expression's type when `expr_types`.
pub fn dump_file(program: &Program, symbols: &Symbols<'_>, checked: &Checked, file: usize, expr_types: bool) -> String {
    crate::types::with_keys_shown(|| Writer::new(program, symbols, checked, file, Mode::Dump { expr_types }).run().text)
}

/// The nodes of one module's declarations: every expression and every `let`.
struct Nodes {
    exprs: Vec<*const Expr>,
    lets: Vec<*const Stmt>,
}

impl Visitor for Nodes {
    fn visit_expr(&mut self, e: &Expr) {
        self.exprs.push(e as *const Expr);
    }
    fn visit_stmt(&mut self, s: &Stmt) {
        if matches!(s, Stmt::Let { .. }) {
            self.lets.push(s as *const Stmt);
        }
    }
}

fn nodes_of(m: &Module) -> (Vec<&Expr>, Vec<&Stmt>) {
    let mut fns: Vec<&FnDecl> = Vec::new();
    collect_fns(&m.items, &mut fns);
    let mut v = Nodes { exprs: Vec::new(), lets: Vec::new() };
    for f in fns {
        visit::walk_fn(&mut v, f);
    }
    // The visitor sees each node behind a reference as long-lived as `m`,
    // which owns every node it reached.
    (v.exprs.into_iter().map(|e| unsafe { &*e }).collect(), v.lets.into_iter().map(|s| unsafe { &*s }).collect())
}

fn collect_fns<'a>(items: &'a [Item], out: &mut Vec<&'a FnDecl>) {
    for item in items {
        match item {
            Item::Fn(f) => out.push(f),
            Item::Handler(h) => {
                out.extend(h.fns.iter());
                out.extend(h.init.iter());
            }
            Item::Qualifier(q) => out.extend(q.fns.iter()),
            Item::Struct(s) => out.extend(s.fns.iter()),
            Item::Namespace(n) => collect_fns(&n.items, out),
            _ => {}
        }
    }
}

/// Whether a type has a source spelling the round trip can write: named
/// types (a `Mut` one too), unions, tuples and arrays of them, with no other
/// qualifier, borrow, identity or unknown part.
fn spellable(t: &Ty) -> bool {
    match t {
        // [dump-spelling] A keyed type is written `Name@module`, as the
        // dumps spell it; an internal name (`_`) has no spelling.
        Ty::Named { name, args } => !crate::typekey::plain(name).contains('_') && args.iter().all(spellable),
        Ty::Qualified { quals, base } => {
            quals.iter().all(|q| q.name == "Mut" && q.args.is_empty() && !q.effect) && matches!(**base, Ty::Named { .. }) && spellable(base)
        }
        Ty::Union(arms) => arms.iter().all(spellable),
        Ty::Tuple(elems) => !elems.is_empty() && elems.iter().all(spellable),
        Ty::Array(e) => spellable(e),
        _ => false,
    }
}

fn ident_at(source: &str, span: Span, name: &str) -> bool {
    source.get(span.start as usize..span.end as usize) == Some(name)
}

/// What a call's choice is, as the check compares it: the callee (a
/// declaration's id, or an effect member as `Effect#index`) and the type
/// arguments, when there are any. `None` for a call the round trip skips.
fn choice_at(checked: &Checked, key: Key) -> Option<String> {
    let (callee, targs) = match &checked.call(key)?.callee {
        Callee::Fn { decl, type_args } => (decl.to_string(), type_args.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")),
        Callee::Member { effect, index, .. } => (format!("{effect}#{index}"), String::new()),
        _ => return None,
    };
    Some(format!("{callee}<{targs}>"))
}

fn choice(checked: &Checked, file: usize, span: Span) -> Option<String> {
    choice_at(checked, checked.key_at(file, span))
}

struct Writer<'a, 'p> {
    program: &'a Program,
    symbols: &'a Symbols<'p>,
    checked: &'a Checked,
    file: usize,
    source: &'a str,
    mode: Mode,
    edits: Vec<Edit>,
    expects: Vec<(Anchor, Expect)>,
}

impl<'a, 'p> Writer<'a, 'p> {
    fn new(program: &'a Program, symbols: &'a Symbols<'p>, checked: &'a Checked, file: usize, mode: Mode) -> Self {
        Writer { program, symbols, checked, file, source: program.files[file].content.as_str(), mode, edits: Vec::new(), expects: Vec::new() }
    }

    fn dump(&self) -> bool {
        matches!(self.mode, Mode::Dump { .. })
    }

    /// A node as written in this file, which a fact is kept at.
    fn here(&self, span: Span) -> bool {
        span.is_numbered() && span.is_written() && (span.end as usize) <= self.source.len() && span.start < span.end
    }

    fn key(&self, span: Span) -> Key {
        self.checked.key_at(self.file, span)
    }

    fn edit(&mut self, at: u32, end: u32, text: String, class: u8, rank: i64) -> usize {
        self.edits.push(Edit { at, end, text, class, rank });
        self.edits.len() - 1
    }

    /// What ends at `at`, after the node starting at `start` (inner nodes
    /// first).
    fn suffix(&mut self, at: u32, start: u32, text: String) -> usize {
        self.edit(at, at, text, ENDS, -(start as i64) * 2 + 1)
    }

    /// A node's closing text at `at` (before a mark after the same node).
    fn closer(&mut self, at: u32, start: u32, text: String) -> usize {
        self.edit(at, at, text, ENDS, -(start as i64) * 2)
    }

    /// A node's opening text at `at`, the node ending at `end` (outer nodes
    /// first).
    fn opener(&mut self, at: u32, end: u32, text: String) -> usize {
        self.edit(at, at, text, STARTS, -(end as i64))
    }

    /// A type as this writing spells it: the round trip only one Salvo can
    /// write, the dump any it knows.
    fn ty_text(&self, t: &Ty) -> Option<String> {
        match self.mode {
            Mode::Source => spellable(t).then(|| t.to_string()),
            Mode::Dump { .. } => (!t.is_unknown()).then(|| t.to_string()),
        }
    }

    fn run(mut self) -> Rewrite {
        let module = &self.program.modules[self.file];
        let (exprs, lets) = nodes_of(module);
        for s in lets {
            self.let_type(s);
        }
        for e in exprs {
            match e {
                Expr::Call { .. } => self.call(e),
                Expr::Binary { .. } => self.operator(e),
                Expr::Str { parts, .. } => {
                    for p in parts {
                        if let StrExprPart::Interp(inner) = p {
                            self.interpolated(inner);
                        }
                    }
                }
                Expr::Lambda { .. } => self.lambda(e),
                Expr::StructLit { .. } => self.struct_lit(e),
                Expr::For { iterable, .. } if self.dump() => self.for_marks(iterable),
                _ => {}
            }
            if self.dump() {
                self.marks(e);
            }
        }
        self.render()
    }

    /// The callee spelled as a call writes it: `name@module`, `name@Effect`.
    fn callee_text(&self, callee: &Callee) -> Option<String> {
        match callee {
            Callee::Fn { decl, .. } => Some(format!("{}@{}", decl.name(), decl.module())),
            Callee::Member { effect, index, .. } => {
                let decl = self.symbols.effects.get(effect.as_str())?;
                if decl.name.name == crate::THROW_EFFECT {
                    return None;
                }
                Some(format!("{}@{}", decl.fns.get(*index)?.name.name, decl.name.name))
            }
            _ => None,
        }
    }

    /// [call-type-args] The inferred type arguments of a fn call, written
    /// `<…>`, where every one has a spelling.
    fn type_args_text(&self, callee: &Callee) -> Option<String> {
        let Callee::Fn { type_args, .. } = callee else { return None };
        if type_args.is_empty() {
            return None;
        }
        let texts: Option<Vec<String>> = type_args.iter().map(|t| self.ty_text(t)).collect();
        texts.map(|ts| format!("<{}>", ts.join(", ")))
    }

    /// [implicit-override] The implicit arguments a call was filled with, as
    /// `name = value` with the expectations the re-check must meet; `None`
    /// when they cannot all be written (two of one name, which a name
    /// cannot tell apart, or one the user already wrote).
    fn implicits_text(&self, key: Key, written: &[NamedArg]) -> Option<(Vec<String>, Vec<(String, String)>)> {
        let filled = self.checked.implicit_args.get(&key)?;
        let mut names: Vec<String> = Vec::new();
        let mut parts: Vec<String> = Vec::new();
        let mut expects: Vec<(String, String)> = Vec::new();
        for arg in filled {
            let (name, value, expect) = match arg {
                // [cmp-carry-override] What an argument's type carries is not
                // written: the argument already gives it.
                ImplicitArg::Resolved { carried: true, .. } => continue,
                ImplicitArg::Resolved { name, key, .. } => (name.clone(), format!("{}@{}", key.name(), key.module()), key.to_string()),
                ImplicitArg::Member { name, effect, index, .. } => {
                    let Some(decl) = self.symbols.effects.get(effect.as_str()) else { continue };
                    let Some(member) = decl.fns.get(*index) else { continue };
                    (name.clone(), format!("{}@{}", member.name.name, decl.name.name), format!("{effect}#{index}"))
                }
                _ => continue,
            };
            names.push(name.clone());
            parts.push(format!("{name} = {value}"));
            expects.push((name, expect));
        }
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != names.len() || names.iter().any(|n| written.iter().any(|a| a.name.name == *n)) {
            return None;
        }
        Some((parts, expects))
    }

    /// A written call: its callee's selector, inferred type arguments and
    /// implicits.
    fn call(&mut self, e: &Expr) {
        let Expr::Call { callee, type_args, args, named, span } = e else { return };
        if !self.here(*span) {
            return;
        }
        // The callee as written: a bare name, or a dot call's field.
        let (name, name_span) = match callee.as_ref() {
            Expr::Ident(id) => (id.name.as_str(), id.span),
            Expr::Field { field, grouped: false, .. } => (field.name.as_str(), field.span),
            _ => return,
        };
        if name_span.is_expanded() || !ident_at(self.source, name_span, name) {
            return;
        }
        let key = self.key(*span);
        let Some(call) = self.checked.call(key) else { return };
        if let Expr::Field { .. } = callee.as_ref() {
            if !call.dot {
                return;
            }
        }
        let Some(want) = choice_at(self.checked, key) else { return };
        // A renamed name answers to its new name only, and `@` on it is
        // refused [fn-rename].
        if let Callee::Fn { decl, .. } = &call.callee {
            if decl.name() != name {
                return;
            }
        }
        let Some(spelled) = self.callee_text(&call.callee) else { return };
        let selector = &spelled[spelled.find('@').unwrap_or(0)..];
        let mut text = selector.to_string();
        // [call-type-args] Where the call wrote none. A member's pin its
        // instance, so they are left to it.
        if type_args.is_empty() {
            if let Some(t) = self.type_args_text(&call.callee) {
                text.push_str(&t);
            }
        }
        self.suffix(name_span.end, name_span.start, text);
        self.expects.push((Anchor::Orig(span.start), Expect::Call(want)));
        // [implicit-override] Before the closing parenthesis, where the
        // source has one.
        let close = span.end.saturating_sub(1);
        if self.source.as_bytes().get(close as usize) != Some(&b')') || args.last().is_some_and(|a| matches!(a, Expr::ReplyTo { .. })) {
            return;
        }
        let Some((parts, expects)) = self.implicits_text(key, named) else { return };
        if parts.is_empty() {
            return;
        }
        let lead = if args.is_empty() && named.is_empty() { "" } else { ", " };
        self.closer(close, span.start, format!("{lead}{}", parts.join(", ")));
        for (n, want) in expects {
            self.expects.push((Anchor::Orig(span.start), Expect::Implicit(n, want)));
        }
    }

    /// The offsets of the `)`s written right after `at` (whitespace between).
    fn closes_after(&self, at: u32) -> Vec<u32> {
        let bytes = self.source.as_bytes();
        let mut out = Vec::new();
        let mut i = at as usize;
        while i < bytes.len() {
            match bytes[i] {
                b' ' | b'\t' | b'\n' | b'\r' => i += 1,
                b')' => {
                    out.push(i as u32);
                    i += 1;
                }
                _ => break,
            }
        }
        out
    }

    /// The offsets of the `(`s written right before `at`, nearest first.
    fn opens_before(&self, at: u32) -> Vec<u32> {
        let bytes = self.source.as_bytes();
        let mut out = Vec::new();
        let mut i = at as usize;
        while i > 0 {
            match bytes[i - 1] {
                b' ' | b'\t' | b'\n' | b'\r' => i -= 1,
                b'(' => {
                    out.push((i - 1) as u32);
                    i -= 1;
                }
                _ => break,
            }
        }
        out
    }

    /// The `(` matching the `)` at `close`.
    fn match_back(&self, close: u32) -> Option<u32> {
        let bytes = self.source.as_bytes();
        let mut depth = 0i32;
        let mut i = close as usize + 1;
        while i > 0 {
            i -= 1;
            match bytes[i] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i as u32);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The `)` matching the `(` at `open`.
    fn match_fwd(&self, open: u32) -> Option<u32> {
        let bytes = self.source.as_bytes();
        let mut depth = 0i32;
        for (i, b) in bytes.iter().enumerate().skip(open as usize) {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i as u32);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Where an expression's text starts, with the grouping parentheses
    /// written round its leading operand (the parser keeps a grouped
    /// expression's own span, so `(a + b) * c` starts at `a`).
    fn outer_start(&self, e: &Expr) -> Option<u32> {
        let first = match e {
            Expr::Binary { lhs, .. } => Some(lhs.as_ref()),
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => Some(base.as_ref()),
            Expr::Call { callee, .. } => Some(callee.as_ref()),
            Expr::Is { subject, .. } | Expr::Retype { subject, .. } | Expr::Elvis { subject, .. } => Some(subject.as_ref()),
            Expr::SafeNav { base, .. } => Some(base.as_ref()),
            Expr::Pipe { lhs, .. } | Expr::SafePipe { lhs, .. } => Some(lhs.as_ref()),
            _ => None,
        };
        let Some(first) = first.filter(|c| c.span().start == e.span().start) else { return Some(e.span().start) };
        match self.closes_after(first.span().end).last() {
            Some(close) => self.match_back(*close).filter(|o| *o <= first.span().start),
            None => self.outer_start(first),
        }
    }

    /// Where an expression's text ends, with the grouping parentheses
    /// written round its trailing operand.
    fn outer_end(&self, e: &Expr) -> Option<u32> {
        let last = match e {
            Expr::Binary { rhs, .. } => Some(rhs.as_ref()),
            Expr::Unary { operand, .. } => Some(operand.as_ref()),
            Expr::Elvis { rhs, .. } | Expr::Pipe { rhs, .. } | Expr::SafePipe { rhs, .. } => Some(rhs.as_ref()),
            _ => None,
        };
        let Some(last) = last.filter(|c| c.span().end == e.span().end) else { return Some(e.span().end) };
        match self.opens_before(last.span().start).last() {
            Some(open) => self.match_fwd(*open).filter(|c| *c >= last.span().end).map(|c| c + 1),
            None => self.outer_end(last),
        }
    }

    /// [op-equality] [op-order] [op-arith] [op-pow] An operator written as the
    /// call it resolved to: `plus@m(a, b)`, `eq@m(a, b)`, `!eq@m(a, b)`,
    /// `cmp@m(a, b) < 0`.
    fn operator(&mut self, e: &Expr) {
        let Expr::Binary { op, lhs, rhs, span } = e else { return };
        let (lspan, rspan) = (lhs.span(), rhs.span());
        if lspan.is_expanded() || rspan.is_expanded() || lspan.end > rspan.start || rspan.end as usize > self.source.len() {
            return;
        }
        let compare = matches!(op, BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq);
        if !(compare || matches!(op, BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow)) {
            return;
        }
        // The operator is the call the checker recorded at it.
        if !self.here(*span) {
            return;
        }
        let key = self.key(*span);
        if self.checked.arm_compares.contains_key(&key) {
            return;
        }
        let Some(call) = self.checked.call(key) else { return };
        let Some(want) = choice_at(self.checked, key) else { return };
        let Some(spelled) = self.callee_text(&call.callee) else { return };
        // [op-promote] A widened operand: the call it names is made at the
        // wide type, which a written call does not do.
        let promoted = |s: Span| self.here(s) && self.checked.converts.get(&self.key(s)).is_some_and(|c| c.steps.iter().any(|st| matches!(st, ConvertStep::Promote { .. })));
        if !self.dump() && (promoted(lspan) || promoted(rspan)) {
            return;
        }
        // The operator's own text between its operands: the grouping
        // parentheses, and the symbol.
        let sym = crate::check::op_symbol(*op);
        let gap = &self.source[lspan.end as usize..rspan.start as usize];
        let trimmed: String = gap.chars().filter(|c| !c.is_whitespace() && *c != '(' && *c != ')').collect();
        if trimmed != sym {
            return;
        }
        let Some(at) = gap.find(sym) else { return };
        let sym_at = lspan.end + at as u32;
        // The comma takes the symbol's place and the space before it.
        let ws = gap[..at].len() - gap[..at].trim_end().len();
        let from = sym_at - ws as u32;
        let (Some(start), Some(end)) = (self.outer_start(e), self.outer_end(e)) else { return };
        let mut type_args = String::new();
        if let Some(t) = self.type_args_text(&call.callee) {
            type_args = t;
        }
        let implicits = self.implicits_text(key, &[]);
        let bang = if matches!(op, BinaryOp::NotEq) { "!" } else { "" };
        let opener = self.opener(start, end, format!("{bang}{spelled}{type_args}("));
        self.edit(from, sym_at + sym.len() as u32, ",".to_string(), REPLACES, 0);
        let mut close = String::new();
        if let Some((parts, _)) = &implicits {
            for p in parts {
                close.push_str(&format!(", {p}"));
            }
        }
        close.push(')');
        if compare {
            close.push_str(&format!(" {sym} 0"));
        }
        self.closer(end, start, close);
        // `!eq(…)`: the call starts after the `!`.
        let anchor = Anchor::Edit(opener, bang.len() as u32);
        self.expects.push((anchor, Expect::Call(want)));
        if let Some((_, expects)) = implicits {
            for (n, w) in expects {
                self.expects.push((anchor, Expect::Implicit(n, w)));
            }
        }
    }

    /// [interp-to-str] An interpolated part as its `to_str` call.
    fn interpolated(&mut self, part: &Expr) {
        let span = part.span();
        if !self.here(span) {
            return;
        }
        let key = self.key(span).synthetic(Key::INTERP_TEXT);
        let Some(call) = self.checked.call(key) else { return };
        let Some(want) = choice_at(self.checked, key) else { return };
        let Some(spelled) = self.callee_text(&call.callee) else { return };
        let (Some(start), Some(end)) = (self.outer_start(part), self.outer_end(part)) else { return };
        let type_args = self.type_args_text(&call.callee).unwrap_or_default();
        let implicits = self.implicits_text(key, &[]);
        // Round the part, outside anything the part's own node writes there
        // (an operator's call).
        let opener = self.edit(start, start, format!("{spelled}{type_args}("), STARTS, -(end as i64) - 1);
        let mut close = String::new();
        if let Some((parts, _)) = &implicits {
            for p in parts {
                close.push_str(&format!(", {p}"));
            }
        }
        close.push(')');
        self.edit(end, end, close, ENDS, -(start as i64) * 2 + 2);
        self.expects.push((Anchor::Edit(opener, 0), Expect::Call(want)));
        if let Some((_, expects)) = implicits {
            for (n, w) in expects {
                self.expects.push((Anchor::Edit(opener, 0), Expect::Implicit(n, w)));
            }
        }
    }

    /// The next non-whitespace byte at or after `at`.
    fn next_byte(&self, at: u32) -> Option<u8> {
        self.source.as_bytes()[at as usize..].iter().copied().find(|b| !b.is_ascii_whitespace())
    }

    /// [var-let-infer] An unannotated `let x = e`'s type, written `let x: T`.
    fn let_type(&mut self, s: &Stmt) {
        let Stmt::Let { pattern: Pattern::Ident(id), ty: None, .. } = s else { return };
        if !self.here(id.span) || !ident_at(self.source, id.span, &id.name) || self.next_byte(id.span.end) != Some(b'=') {
            return;
        }
        let Some(ty) = self.checked.expr_ty.get(&self.key(id.span)).cloned() else { return };
        let Some(text) = self.ty_text(&ty) else { return };
        self.suffix(id.span.end, id.span.start, format!(": {text}"));
        self.expects.push((Anchor::Orig(id.span.start), Expect::Type(ty.to_string())));
    }

    /// [fn-lambda] A lambda's inferred parameter types, and a block body's
    /// return type when it wrote none.
    fn lambda(&mut self, e: &Expr) {
        let Expr::Lambda { params, ret, body, span, .. } = e else { return };
        if !self.here(*span) {
            return;
        }
        for p in params {
            if p.ty.is_some() || !self.here(p.name.span) || !ident_at(self.source, p.name.span, &p.name.name) {
                continue;
            }
            if !matches!(self.next_byte(p.name.span.end), Some(b',') | Some(b')')) {
                continue;
            }
            let Some(ty) = self.checked.expr_ty.get(&self.key(p.name.span)).cloned() else { continue };
            let Some(text) = self.ty_text(&ty) else { continue };
            self.suffix(p.name.span.end, p.name.span.start, format!(": {text}"));
            self.expects.push((Anchor::Orig(p.name.span.start), Expect::Type(ty.to_string())));
        }
        if ret.is_some() {
            return;
        }
        let LambdaBody::Block(block) = body else { return };
        if !block.span.is_written() || self.source.as_bytes().get(block.span.start as usize) != Some(&b'{') {
            return;
        }
        let Some(Ty::Fn { ret, .. }) = self.checked.expr_ty.get(&self.key(*span)).map(|t| t.strip_quals().clone()) else { return };
        let Some(text) = self.ty_text(&ret) else { return };
        self.opener(block.span.start, block.span.end, format!("{text} "));
    }

    /// [struct-lit] A generic struct literal's inferred type arguments,
    /// where it wrote none: `Box { v: 1 }` as `Box<Int> { v: 1 }`.
    fn struct_lit(&mut self, e: &Expr) {
        let Expr::StructLit { ty: Some(Type::Named { base, .. }), span, .. } = e else { return };
        if !self.here(*span) || !base.args.is_empty() || !base.name.span.is_written() || !ident_at(self.source, base.name.span, &base.name.name) {
            return;
        }
        if self.next_byte(base.name.span.end) != Some(b'{') {
            return;
        }
        let Some(ty) = self.checked.expr_ty.get(&self.key(*span)).cloned() else { return };
        let Ty::Named { args, .. } = ty.strip_quals() else { return };
        if args.is_empty() {
            return;
        }
        let texts: Option<Vec<String>> = args.iter().map(|t| self.ty_text(t)).collect();
        let Some(texts) = texts else { return };
        self.suffix(base.name.span.end, base.name.span.start, format!("<{}>", texts.join(", ")));
        self.expects.push((Anchor::Orig(span.start), Expect::Type(ty.to_string())));
    }

    /// [iter-for] The dump's mark of the calls a `for` names.
    fn for_marks(&mut self, iterable: &Expr) {
        let span = iterable.span();
        if !self.here(span) {
            return;
        }
        let key = self.key(span);
        let mut names = Vec::new();
        for k in [Key::FOR_ITER, Key::FOR_NEXT] {
            if let Some(c) = self.checked.call(key.synthetic(k)) {
                match self.callee_text(&c.callee) {
                    Some(t) => names.push(t),
                    None => names.push(match &c.callee {
                        Callee::Local { local, .. } => format!("local {local}"),
                        _ => "#{???}".to_string(),
                    }),
                }
            }
        }
        if !names.is_empty() {
            let end = self.outer_end(iterable).unwrap_or(span.end);
            self.suffix(end, span.start, format!(" #{{{}}}", names.join(", ")));
        }
    }

    /// The dump's marks after an expression: a coercion, a narrowing, and
    /// with `--expr-types` its type.
    fn marks(&mut self, e: &Expr) {
        let span = e.span();
        if !self.here(span) {
            return;
        }
        let key = self.key(span);
        let at = match e {
            Expr::Binary { .. } => self.outer_end(e).unwrap_or(span.end),
            _ => span.end,
        };
        let mut text = String::new();
        if let Some(n) = self.checked.narrowed_reads.get(&key) {
            if self.checked.expr_ty.get(&key) != Some(&n.stored) {
                let how = match n.by {
                    NarrowBy::Flow => "narrowed from",
                    NarrowBy::Claim => "claimed from",
                };
                text.push_str(&format!("#{{{how} {}}}", n.stored));
            }
        }
        if let Some(to) = self.checked.converts.get(&key).and_then(|c| c.to_ty()) {
            text.push_str(&format!("#{{as {to}}}"));
        }
        if let Mode::Dump { expr_types: true } = self.mode {
            if let Some(t) = self.checked.expr_ty.get(&key) {
                text.push_str(&format!("#{{: {t}}}"));
            }
        }
        if !text.is_empty() {
            self.suffix(at, span.start, text);
        }
    }

    fn render(self) -> Rewrite {
        let mut order: Vec<usize> = (0..self.edits.len()).collect();
        order.sort_by_key(|&i| {
            let e = &self.edits[i];
            (e.at, e.class, e.rank, i)
        });
        let source = self.source;
        let mut text = String::with_capacity(source.len() + self.edits.iter().map(|e| e.text.len()).sum::<usize>());
        let mut out_at: Vec<u32> = vec![0; self.edits.len()];
        let mut last = 0usize;
        // Every original offset an edit touches, with where it lands: what
        // `moved` reads.
        let mut shifts: Vec<(u32, i64)> = Vec::new();
        for &i in &order {
            let e = &self.edits[i];
            let at = e.at as usize;
            if at < last {
                // Overlapping a replacement already made: skipped.
                continue;
            }
            text.push_str(&source[last..at]);
            out_at[i] = text.len() as u32;
            text.push_str(&e.text);
            last = e.end as usize;
            shifts.push((e.end, e.text.len() as i64 - (e.end - e.at) as i64));
        }
        text.push_str(&source[last..]);
        // Where the original character at `at` lands: after every edit at
        // or before it.
        let moved = |at: u32| -> u32 { (at as i64 + shifts.iter().filter(|(end, _)| *end <= at).map(|(_, d)| d).sum::<i64>()) as u32 };
        let mut calls: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut implicits = Vec::new();
        let mut types = BTreeMap::new();
        for (anchor, expect) in self.expects {
            let at = match anchor {
                Anchor::Orig(o) => moved(o),
                Anchor::Edit(i, into) => out_at[i] + into,
            };
            match expect {
                Expect::Call(want) => calls.entry(at).or_default().push(want),
                Expect::Implicit(n, want) => implicits.push((at, n, want)),
                Expect::Type(want) => {
                    types.insert(at, want);
                }
            }
        }
        Rewrite { text, calls, implicits, types }
    }
}

/// What the re-check of a [`Rewrite`] found different: each call whose
/// choice changed, each implicit argument whose written value names
/// something else, and each written type the re-check gave another. Empty
/// when the round trip holds.
pub fn compare(program: &Program, checked: &Checked, file: usize, rewrite: &Rewrite) -> Vec<String> {
    // Spelled as the expectations were.
    crate::types::with_keys_shown(|| compare_spelled(program, checked, file, rewrite))
}

fn compare_spelled(program: &Program, checked: &Checked, file: usize, rewrite: &Rewrite) -> Vec<String> {
    let module = &program.modules[file];
    let source = program.files[file].content.as_str();
    let line = |at: u32| source[..at.min(source.len() as u32) as usize].matches('\n').count() + 1;
    let mut found: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut values: BTreeMap<(u32, String), String> = BTreeMap::new();
    let mut typed: BTreeMap<u32, String> = BTreeMap::new();
    let (exprs, lets) = nodes_of(module);
    for e in &exprs {
        match e {
            Expr::Call { named, span, .. } => {
                if span.is_expanded() || !rewrite.calls.contains_key(&span.start) {
                    continue;
                }
                if let Some(c) = choice(checked, file, *span) {
                    found.entry(span.start).or_default().push(c);
                }
                for a in named {
                    let id = match &a.value {
                        Expr::Scoped { name, .. } | Expr::EffectScoped { name, .. } => name.span,
                        _ => continue,
                    };
                    let key = checked.key_at(file, id);
                    let got = match (checked.fn_refs.get(&key), checked.fn_values.get(&key)) {
                        (Some(k), _) => k.to_string(),
                        (None, Some(FnValue::Member(ImplicitArg::Member { effect, index, .. }))) => format!("{effect}#{index}"),
                        _ => "nothing".to_string(),
                    };
                    values.insert((span.start, a.name.name.clone()), got);
                }
            }
            Expr::Lambda { params, .. } => {
                for p in params {
                    if p.name.span.is_written() && p.name.span.is_numbered() && rewrite.types.contains_key(&p.name.span.start) {
                        if let Some(t) = checked.expr_ty.get(&checked.key_at(file, p.name.span)) {
                            typed.insert(p.name.span.start, t.to_string());
                        }
                    }
                }
            }
            Expr::StructLit { span, .. } if span.is_written() && span.is_numbered() && rewrite.types.contains_key(&span.start) => {
                if let Some(t) = checked.expr_ty.get(&checked.key_at(file, *span)) {
                    typed.insert(span.start, t.to_string());
                }
            }
            _ => {}
        }
    }
    for s in lets {
        let Stmt::Let { pattern: Pattern::Ident(id), .. } = s else { continue };
        if id.span.is_written() && id.span.is_numbered() && rewrite.types.contains_key(&id.span.start) {
            if let Some(t) = checked.expr_ty.get(&checked.key_at(file, id.span)) {
                typed.insert(id.span.start, t.to_string());
            }
        }
    }
    let mut out = Vec::new();
    for (at, want) in &rewrite.calls {
        let mut want = want.clone();
        want.sort();
        let mut got = found.get(at).cloned().unwrap_or_default();
        got.sort();
        if want != got {
            out.push(format!("line {}: the call chose {:?}, its written-out form {:?}", line(*at), want, got));
        }
    }
    for (at, name, want) in &rewrite.implicits {
        let got = values.get(&(*at, name.clone())).cloned().unwrap_or_else(|| "nothing".to_string());
        if *want != got {
            out.push(format!("line {}: `{name}` was filled by {want}, its written-out value names {got}", line(*at)));
        }
    }
    for (at, want) in &rewrite.types {
        let got = typed.get(at).cloned().unwrap_or_else(|| "nothing".to_string());
        if *want != got {
            out.push(format!("line {}: the type was {want}, written out it is {got}", line(*at)));
        }
    }
    out
}
