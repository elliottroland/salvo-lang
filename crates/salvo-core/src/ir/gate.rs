//! [ir-analyze] Per-declaration gating (PIPELINE.md R8): the IR is built, and
//! the flow and linear checks run, for every declaration whose own checking
//! produced no error, whatever the rest of the program holds. A declaration
//! with an error of its own reports that error and nothing from the IR, so a
//! type error never brings a flow diagnostic with it; one that depends on a
//! broken interface is skipped too, since what it reads of it is not
//! reliable.
//!
//! The rule, as built:
//! 1. **Which declarations broke**, from the errors' spans. An error raised
//!    at a node a stamp copied (rendered by its origin at the stamp's `by`
//!    site, a field or an arm [origin]) carries the stamped fn's id, and
//!    belongs to that fn's body only. Otherwise every item containing the error is
//!    broken: inside a fn's body, or a handler member's, the body; anywhere
//!    else in a declaration, its interface (a fn's signature, a struct's,
//!    an effect's, a type's, a qualifier's, a handler's outside its member
//!    bodies). An error in an import, in no declaration (a module-level
//!    `use`), past the file's end, or any parse or expansion error breaks the
//!    whole file.
//! 2. **The builder** builds a broken declaration's interface only (no body,
//!    no default, no initializer) and a broken file not at all.
//! 3. **The checks skip**, in a program with any error, a broken
//!    declaration, one whose build reported an error or left an
//!    `Unsupported` node, and one that calls or names a declaration whose
//!    interface broke (a fn reference, a task target, an interface member,
//!    a type by its key). A `Ty::Unknown` is no reason: clean programs' IR
//!    holds them (84 of the suite's programs did, 2026-10-20). A program with
//!    no error checks every declaration, as before gating.

use std::collections::HashSet;

use salvo_syntax::ast::Item;
use salvo_syntax::Span;

use crate::diag::FileDiagnostic;
use crate::ir::*;
use crate::program::{Program as CoreProgram, Symbols};

/// What the program's errors broke, by file and item index.
#[derive(Default, Debug)]
pub struct Gate {
    /// Files none of whose declarations is built or checked.
    pub files: HashSet<usize>,
    /// Declarations whose interface broke: not built past it, not checked,
    /// and their users skipped.
    pub interfaces: HashSet<(usize, usize)>,
    /// Declarations whose body broke: built without it, not checked.
    pub bodies: HashSet<(usize, usize)>,
}

impl Gate {
    /// The gate the error diagnostics among `diags` set (warnings are
    /// ignored).
    pub fn new<'a>(program: &CoreProgram, decls: &crate::decl_id::DeclIndex, diags: impl IntoIterator<Item = &'a FileDiagnostic>) -> Gate {
        let mut gate = Gate::default();
        for d in diags.into_iter().filter(|d| d.is_error()) {
            gate.attribute(program, decls, d);
        }
        gate
    }

    /// Nothing broke: every declaration is built and checked.
    pub fn is_clean(&self) -> bool {
        self.files.is_empty() && self.interfaces.is_empty() && self.bodies.is_empty()
    }

    /// Whether the item's body is left out of the IR (and its checks).
    pub fn body_broken(&self, file: usize, item: usize) -> bool {
        self.files.contains(&file) || self.interfaces.contains(&(file, item)) || self.bodies.contains(&(file, item))
    }

    /// Whether the item's interface broke, so its users are skipped.
    pub fn interface_broken(&self, file: usize, item: usize) -> bool {
        self.files.contains(&file) || self.interfaces.contains(&(file, item))
    }

    /// Breaks a parse- or expansion-broken file wholesale.
    pub fn break_file(&mut self, file: usize) {
        self.files.insert(file);
    }

    fn attribute(&mut self, program: &CoreProgram, decls: &crate::decl_id::DeclIndex, d: &FileDiagnostic) {
        let file = d.file;
        let (Some(source), Some(module)) = (program.files.get(file), program.modules.get(file)) else {
            return;
        };
        // [origin] [comptime-instantiate] An error at a node a stamp copied
        // is rendered at the stamp's `by` site, a field or an arm — written
        // spans of another declaration (the struct the stamp is for) — and
        // names the stamped fn it was raised in.
        if let Some(loc) = d.decl.and_then(|id| decls.loc(id)) {
            self.bodies.insert((loc.file, loc.item));
            return;
        }
        if d.span.is_expanded() || d.span.start as usize >= source.content.len() {
            self.files.insert(file);
            return;
        }
        let within = |s: Span| d.span.start >= s.start && d.span.start < s.end.max(s.start + 1);
        let mut found = false;
        for (i, item) in module.items.iter().enumerate() {
            let Some(span) = item_span(item) else { continue };
            if !within(span) {
                continue;
            }
            found = true;
            match item {
                Item::Fn(f) if f.body.as_ref().is_some_and(|b| within(b.span)) => {
                    self.bodies.insert((file, i));
                }
                Item::Handler(h) if h.fns.iter().chain(h.init.iter()).any(|f| f.body.as_ref().is_some_and(|b| within(b.span))) => {
                    self.bodies.insert((file, i));
                }
                Item::Import(_) => {
                    self.files.insert(file);
                }
                _ => {
                    self.interfaces.insert((file, i));
                }
            }
        }
        if !found {
            self.files.insert(file);
        }
    }
}

fn item_span(item: &Item) -> Option<Span> {
    Some(match item {
        Item::Import(i) => i.span,
        Item::Type(t) => t.span,
        Item::Struct(s) => s.span,
        Item::Qualifier(q) => q.span,
        Item::Effect(e) => e.span,
        Item::Handler(h) => h.span,
        Item::Params(p) => p.span,
        Item::Fn(f) => f.span,
        Item::EffectAlias(a) => a.span,
        Item::Namespace(n) => n.span,
        Item::Refn(r) => r.span,
        Item::Rename(r) => r.span,
        Item::Test(t) => t.span,
    })
}

/// [ir-analyze] Which declarations of the built IR the checks run on: not a
/// broken one (`Gate`), not one whose build failed (`failed`, by file and
/// item), and not one whose IR calls or names a declaration whose interface
/// broke or holds a node the builder could not lower.
pub struct Checkable<'g> {
    gate: &'g Gate,
    failed: &'g HashSet<(usize, usize)>,
    decls: &'g crate::decl_id::DeclIndex,
    /// The keys of the types, effects and handlers whose interface broke.
    broken_keys: HashSet<String>,
}

impl<'g> Checkable<'g> {
    pub fn new(program: &'g CoreProgram, symbols: &Symbols<'_>, decls: &'g crate::decl_id::DeclIndex, gate: &'g Gate, failed: &'g HashSet<(usize, usize)>) -> Self {
        let mut broken_keys = HashSet::new();
        for (file, module) in program.modules.iter().enumerate() {
            for (i, item) in module.items.iter().enumerate() {
                if !gate.interface_broken(file, i) {
                    continue;
                }
                let key = match item {
                    Item::Struct(s) => symbols.key_or(s, &s.name.name).to_string(),
                    Item::Effect(e) => symbols.key_or(e, &e.name.name).to_string(),
                    Item::Handler(h) => symbols.key_or(h, &h.name.name).to_string(),
                    Item::Type(t) => symbols.key_or(t, &t.name.name).to_string(),
                    _ => continue,
                };
                broken_keys.insert(key);
            }
        }
        Checkable { gate, failed, decls, broken_keys }
    }

    /// [decl-id] The file and item `id` is declared at (a member's
    /// container's).
    fn addr(&self, id: &DeclId) -> Option<(usize, usize)> {
        self.decls.loc(*id).map(|l| (l.file, l.item))
    }

    /// Whether the flow and linear checks run on `decl`.
    pub fn check(&self, decl: &Decl) -> bool {
        // A program with no errors checks everything, as before gating.
        if self.gate.is_clean() {
            return true;
        }
        let Some((file, item)) = self.addr(decl.id()) else { return true };
        if self.gate.body_broken(file, item) || self.failed.contains(&(file, item)) {
            return false;
        }
        let ok = std::cell::Cell::new(true);
        let ty = |t: &Ty| {
            if ok.get() && mentions(t, &self.broken_keys) {
                ok.set(false);
            }
        };
        let id = |d: &DeclId| {
            if ok.get() && self.addr(d).is_some_and(|(f, i)| self.gate.interface_broken(f, i)) {
                ok.set(false);
            }
        };
        visit_decl(
            decl,
            &mut |e| {
                ty(&e.ty);
                match &e.kind {
                    ExprKind::Unsupported(_) => ok.set(false),
                    ExprKind::Call { target: FnRef::Decl(d), .. } | ExprKind::FnValue(FnRef::Decl(d)) => id(d),
                    ExprKind::ReplyTo { target: ReplyTarget::Task { target, .. }, .. } => id(target),
                    ExprKind::MemberCall { member, .. } | ExprKind::Send { member, .. } => id(&member.interface),
                    ExprKind::HandlerCall { member: HandlerMember::Face(member), .. } => id(&member.interface),
                    _ => {}
                }
            },
            &mut |t| ty(t),
        );
        ok.get()
    }
}

/// Whether `t` names one of `keys`.
fn mentions(t: &Ty, keys: &HashSet<String>) -> bool {
    if keys.is_empty() {
        return false;
    }
    match t {
        Ty::Named { name, args } => keys.contains(name) || args.iter().any(|a| mentions(a, keys)),
        Ty::Qualified { base, .. } => mentions(base, keys),
        Ty::Union(xs) | Ty::Tuple(xs) => xs.iter().any(|a| mentions(a, keys)),
        Ty::Array(e) => mentions(e, keys),
        Ty::Fn { params, ret, effects, .. } => params.iter().chain(effects).any(|a| mentions(a, keys)) || mentions(ret, keys),
        _ => false,
    }
}

/// Every expression of a declaration's lowered code, and every type its
/// signature and bindings carry.
fn visit_decl(decl: &Decl, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    match decl {
        Decl::Fn(f) => visit_fn(f, e, t),
        Decl::Impl(h) => {
            for p in &h.ctor_params {
                t(&p.ty);
            }
            for d in &h.deps {
                t(d);
            }
            for s in &h.state {
                t(&s.ty);
                if let Some(x) = &s.default {
                    expr(x, e, t);
                }
            }
            if let Some(x) = &h.mailbox {
                expr(x, e, t);
            }
            for f in h.members.iter().chain(h.init.iter()) {
                visit_fn(f, e, t);
            }
        }
        Decl::Struct(s) => {
            for f in &s.fields {
                t(&f.ty);
                if let Some(x) = &f.default {
                    expr(x, e, t);
                }
            }
        }
        Decl::Static(s) => {
            t(&s.ty);
            for st in &s.stmts {
                stmt(st, e, t);
            }
        }
        _ => {}
    }
}

fn visit_fn(f: &FnDecl, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    for p in &f.params {
        t(&p.ty);
    }
    t(&f.ret);
    if let Some(b) = &f.body {
        block(b, e, t);
    }
}

fn block(b: &Block, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    for s in &b.stmts {
        stmt(s, e, t);
    }
    if let Some(v) = &b.value {
        expr(v, e, t);
    }
}

fn place(p: &Place, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    for s in &p.steps {
        if let Step::Index(x) = s {
            expr(x, e, t);
        }
    }
}

fn stmt(s: &Stmt, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    match s {
        Stmt::Loop { body, .. } => block(body, e, t),
        Stmt::ForEach { ty, iterable, body, .. } => {
            t(ty);
            expr(iterable, e, t);
            block(body, e, t);
        }
        Stmt::Let { ty, value, .. } => {
            t(ty);
            expr(value, e, t);
        }
        Stmt::Alias { ty, place: p, .. } => {
            t(ty);
            place(p, e, t);
        }
        Stmt::Narrow { ty, from, from_ty, .. } => {
            t(ty);
            t(from_ty);
            place(from, e, t);
        }
        Stmt::Unpack { locals, from, from_ty, .. } => {
            for (_, ty) in locals {
                t(ty);
            }
            t(from_ty);
            place(from, e, t);
        }
        Stmt::Assign { place: p, value } => {
            place(p, e, t);
            expr(value, e, t);
        }
        Stmt::Expr(x) | Stmt::Return(Some(x), _) => expr(x, e, t),
        Stmt::Return(None, _) | Stmt::Break | Stmt::Continue | Stmt::Obligation(_) => {}
    }
}

fn all(xs: &[Expr], e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    for x in xs {
        expr(x, e, t);
    }
}

fn expr(x: &Expr, e: &mut impl FnMut(&Expr), t: &mut impl FnMut(&Ty)) {
    e(x);
    match &x.kind {
        ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Unit
        | ExprKind::MakeNone
        | ExprKind::FnValue(_)
        | ExprKind::SelfAddr
        | ExprKind::Unsupported(_) => {}
        ExprKind::Read { place: p, .. } => place(p, e, t),
        ExprKind::Call { args, type_args, .. } => {
            type_args.iter().for_each(&mut *t);
            all(args, e, t);
        }
        ExprKind::Op { args, .. } | ExprKind::SelfSend { args, .. } => all(args, e, t),
        ExprKind::MemberCall { instance, args, type_args, .. } => {
            type_args.iter().for_each(&mut *t);
            expr(instance, e, t);
            all(args, e, t);
        }
        ExprKind::HandlerCall { instance, args, .. } => {
            expr(instance, e, t);
            all(args, e, t);
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                expr(v, e, t);
            }
        }
        ExprKind::Rewrap { from, value } => {
            t(from);
            expr(value, e, t);
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::BorrowAs { value, .. }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value } => expr(value, e, t),
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::Array(xs) | ExprKind::Concat(xs) => all(xs, e, t),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                expr(c, e, t);
                block(b, e, t);
            }
            if let Some(o) = otherwise {
                block(o, e, t);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            expr(subject, e, t);
            for a in arms {
                block(&a.body, e, t);
            }
        }
        ExprKind::Test { subject, .. } => expr(subject, e, t),
        ExprKind::Lambda { params, ret, body, .. } => {
            for p in params {
                t(&p.ty);
            }
            t(ret);
            block(body, e, t);
        }
        ExprKind::Try { body } => block(body, e, t),
        ExprKind::WaitFor { token_ty, body, .. } => {
            t(token_ty);
            block(body, e, t);
        }
        ExprKind::Throw { message } => expr(message, e, t),
        ExprKind::Assert { cond, message, .. } => {
            expr(cond, e, t);
            if let Some(m) = message {
                expr(m, e, t);
            }
        }
        ExprKind::Unreachable { message, .. } => {
            if let Some(m) = message {
                expr(m, e, t);
            }
        }
        ExprKind::Spawn { handler, deps, pool, join, effects } => {
            effects.iter().for_each(&mut *t);
            expr(handler, e, t);
            all(deps, e, t);
            if let Some(p) = pool {
                expr(p, e, t);
            }
            if let Some(j) = join {
                expr(j, e, t);
            }
        }
        ExprKind::Send { addr, args, .. } => {
            expr(addr, e, t);
            all(args, e, t);
        }
        ExprKind::ReplyTo { target, captures, pool, .. } => {
            if let ReplyTarget::Task { effects, .. } = target {
                all(effects, e, t);
            }
            all(captures, e, t);
            if let Some(p) = pool {
                expr(p, e, t);
            }
        }
        ExprKind::Handle { instance } => expr(instance, e, t),
        ExprKind::AddrInstance { addr } => expr(addr, e, t),
        ExprKind::UnionToStr { value, arms } => {
            expr(value, e, t);
            all(arms, e, t);
        }
    }
}

