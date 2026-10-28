//! Statements, expressions and ownership for the Rust IR emitter.
//!
//! The ownership rule [rs-ir]: a local is `Owned`, or holds a reference
//! (`Ref`, `RefMut`: a lent parameter, a `proj` value, a narrowing that
//! reads in place), or is a field of the handler (`SelfField`). A read
//! that needs its own value moves it when the IR marks it consumed and the
//! storage is owned, and clones it otherwise; a lent argument borrows.

use std::collections::{HashMap, HashSet};

use salvo_core::types::Ty;
use salvo_core::ir::{ArmTest, Block, Expr, ExprKind, FnKind, FnRef, Justification, Lit, Local, Op, Param, PassMode, Place, Stmt, Step};

use super::decls::{fn_ty_param_mode, FnPos};
use super::{is_copy_ty, is_mut, is_mut_borrow, is_proj, is_ref, ModuleEmitter};
use crate::emit::{escape_char, escape_format_text, escape_string, rs_ident, stateful_trait_name, stateless_trait_name};
use crate::intrinsics::Spread;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Owned,
    Ref,
    RefMut,
    SelfField,
    /// [rs-elem-mut] [deduce-field] A name for a place, re-rendered at every
    /// use: a field path, or an element of an anchor at a position.
    Elem,
}

/// Per-fn state.
#[derive(Default)]
pub(crate) struct FnState {
    pub kinds: HashMap<String, Kind>,
    pub tys: HashMap<String, Ty>,
    /// The lifetime a `proj` type renders with, while one is being spelled.
    pub lt: Option<String>,
    pub ret: Option<Ty>,
    pub ret_lt: bool,
    pub tmp: usize,
    pub test_arms: HashMap<(u32, usize), ArmTest>,
    /// Locals some read consumes.
    pub consumed: HashSet<String>,
    /// Locals passed where a `LentMut` parameter is.
    pub lent_mut: HashSet<String>,
    /// Handler-instance locals and how many handles they back.
    pub handles: HashMap<String, usize>,
    /// Handler-instance locals shared behind an `Arc` (several faces).
    pub shared_inst: HashSet<String>,
    /// [rs-loc] Rendering a locator variant: the anchor parameter, and
    /// whether the position is optional.
    pub loc: Option<(String, bool)>,
    /// [rs-loc] Loop locals bound to a position of the anchor: the handle
    /// the element is.
    pub loc_index: HashMap<String, super::paths::Handle>,
    /// [rs-path] The path type a locator answers.
    pub loc_pt: Option<super::paths::PathTy>,
    /// [rs-path] Handle locals: the container's root and the path.
    pub ref_handles: HashMap<String, super::paths::HandleBinding>,
    /// [rs-path] A place being rendered for a write (an enum path is then
    /// walked `&mut`).
    pub place_mut: bool,
    /// [rs-path] Rendering a lending callback's call for its path alone.
    pub want_path: bool,
    /// [rs-path] The fn's path type parameters: (container, element) key,
    /// and the Rust type parameter standing for it.
    pub gen_paths: Vec<((String, String), String)>,
    /// The type parameters in scope.
    pub generics: HashSet<String>,
    /// [rs-elem-mut] Elem locals: the anchor (or place) text, and the
    /// position variable when it is an element.
    pub elem: HashMap<String, (String, Option<String>)>,
    /// [rs-proj-generic] Those whose `proj` renders as the variable.
    pub plain_vars: HashSet<String>,
    /// [rs-proj-generic] The fn's type parameters a source parameter holds:
    /// their `proj`, nested too, is the definition's borrow (`&'a T`).
    pub held_vars: HashSet<String>,
    /// [rs-repoint] The statements (by address) that start a fresh Rust
    /// variable for a re-pointed view local.
    pub repoint: HashMap<usize, super::repoint::Split>,
    /// [rs-throw-controlflow] The fn's message type, when it throws.
    pub throws: Option<Ty>,
    /// The enclosing `try`s: label and outcome type.
    pub tries: Vec<(String, Ty)>,
    /// The impl whose member is being rendered (its key).
    pub current_impl: Option<String>,
    /// The callee's type parameters as instantiated, while its arguments
    /// render: what a fn-typed slot means at this call.
    pub call_subst: HashMap<String, Ty>,
    /// [rs-mixed] Rendering a façade member: a self-send always enqueues.
    pub in_facade: bool,
    /// Type variables rendered as these types (a generic trait's member
    /// implemented at an instantiation: its conventions are the
    /// declaration's, its types the instance's).
    pub render_subst: HashMap<String, Ty>,
    /// [rs-fn-lend] Parameter types a callback's result may borrow from,
    /// with the named lifetime they share in the signature.
    pub tie: Vec<(Ty, String)>,
    /// Locals some `Assign` writes.
    pub assigned: HashSet<String>,
    /// A branch expression being rendered as a borrow: its arms' tails are
    /// borrowed (`Some(true)`: mutably) rather than owned.
    pub borrow_tail: Option<bool>,
    /// Set when a tail rendered under `borrow_tail` is owned by its own
    /// block, so the borrow would dangle: the branch is rendered again owned.
    pub borrow_tail_failed: bool,
    /// Block-local bindings of a place that a mutable borrow of their
    /// branch reaches through: bound `&mut`.
    pub mut_alias: HashSet<String>,
    /// Locals a lambda or a `replyto` captures.
    pub captured: HashSet<String>,
    /// [rs-fn-field] Fn-typed locals bound to a stored fn value (a call's result, a
    /// field read): they hold an `Arc<dyn Fn>`, which is called through
    /// rather than coerced to a `&mut dyn FnMut`.
    pub arc_fns: HashSet<String>,
}

pub(crate) fn strip_all_pub(t: &Ty) -> Ty {
    strip_all(t)
}

/// Whether `t` names a type variable outside `scope` (an annotation would
/// not resolve).
pub(crate) fn foreign_var(t: &Ty, scope: &HashSet<String>) -> bool {
    match t {
        Ty::Var(v) => !scope.contains(v),
        Ty::Named { args, .. } | Ty::Union(args) | Ty::Tuple(args) => args.iter().any(|a| foreign_var(a, scope)),
        Ty::Qualified { base, .. } => foreign_var(base, scope),
        Ty::Array(e) => foreign_var(e, scope),
        Ty::Fn { params, ret, effects, .. } => params.iter().chain(effects).any(|a| foreign_var(a, scope)) || foreign_var(ret, scope),
        Ty::Unknown | Ty::Any => true,
        _ => false,
    }
}

impl FnState {
    pub fn bind_param(&mut self, name: &str, p: &Param) -> Kind {
        let k = if is_copy_ty(&p.ty) || p.variadic {
            Kind::Owned
        } else if is_proj(&p.ty) {
            if is_mut_borrow(&p.ty) { Kind::RefMut } else { Kind::Ref }
        } else {
            match p.mode {
                PassMode::Moved => Kind::Owned,
                PassMode::Lent if matches!(p.ty.strip_quals(), Ty::Fn { .. }) => Kind::RefMut,
                PassMode::Lent => Kind::Ref,
                PassMode::LentMut => Kind::RefMut,
            }
        };
        self.kinds.insert(name.to_string(), k);
        self.tys.insert(name.to_string(), p.ty.clone());
        k
    }
}

/// Whether every arm of a branch ends in a read of a place that outlives
/// the arm (not a local the arm owns), or does not end at all.
fn tails_lendable(e: &Expr) -> bool {
    let blocks: Vec<&Block> = match &e.kind {
        ExprKind::Branch { arms, otherwise, .. } => arms.iter().map(|(_, b)| b).chain(otherwise.iter()).collect(),
        ExprKind::Switch { arms, .. } => arms.iter().map(|a| &a.body).collect(),
        _ => return false,
    };
    blocks.into_iter().all(|b| match b.value.as_deref() {
        None => true,
        Some(v) if diverges(v) => true,
        Some(v) => match &v.kind {
            ExprKind::Read { place, .. } => !b.stmts.iter().any(|s| matches!(s, Stmt::Let { local, ty, .. } if *local == place.root && !is_proj(ty))),
            ExprKind::Branch { .. } | ExprKind::Switch { .. } => tails_lendable(v),
            _ => false,
        },
    })
}

/// A read's place as the program wrote it (`h.items`), for a diagnostic.
fn read_place_text(e: &Expr) -> Option<String> {
    let ExprKind::Read { place, .. } = &e.kind else { return None };
    let mut out = place.root.0.split('~').next().unwrap_or("").to_string();
    for s in &place.steps {
        match s {
            Step::Field(f) => out.push_str(&format!(".{f}")),
            Step::Tuple(i) => out.push_str(&format!(".{i}")),
            _ => return None,
        }
    }
    Some(out)
}

pub(crate) fn rs_local(l: &str) -> String {
    let s = l.replace('~', "_");
    if s.starts_with("__") { s } else { rs_ident(&s) }
}

/// [safe-nav] If `value` is a plain field read of the narrowed receiver `nav`
/// (`Read(nav.field)`, possibly wrapped in `Present`), the field name; `None`
/// for a call link or any other shape.
fn safe_nav_field<'a>(value: &'a Expr, nav: &Local) -> Option<&'a str> {
    let read = match &value.kind {
        ExprKind::Present { value } => value,
        _ => value,
    };
    match &read.kind {
        ExprKind::Read { place, .. } if place.root == *nav && place.steps.len() == 1 => match &place.steps[0] {
            Step::Field(f) => Some(f.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// The roots every `Assign` in the body writes.
pub(crate) fn assigned_roots(b: Option<&Block>) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(b) = b {
        walk_block(b, &mut |s| {
            if let Stmt::Assign { place, .. } = s {
                out.insert(place.root.0.clone());
            }
        }, &mut |_| {});
    }
    out
}

pub(crate) fn walk_block<'b>(b: &'b Block, fs: &mut impl FnMut(&'b Stmt), fe: &mut impl FnMut(&'b Expr)) {
    for s in &b.stmts {
        walk_stmt(s, fs, fe);
    }
    if let Some(v) = &b.value {
        walk_expr(v, fs, fe);
    }
}

pub(crate) fn walk_stmt<'b>(s: &'b Stmt, fs: &mut impl FnMut(&'b Stmt), fe: &mut impl FnMut(&'b Expr)) {
    fs(s);
    match s {
        Stmt::Loop { body, .. } => walk_block(body, fs, fe),
        Stmt::ForEach { iterable, body, .. } => {
            walk_expr(iterable, fs, fe);
            walk_block(body, fs, fe);
        }
        Stmt::Let { value, .. } => walk_expr(value, fs, fe),
        Stmt::Narrow { from, .. } | Stmt::Alias { place: from, .. } => walk_place(from, fs, fe),
        Stmt::Assign { place, value } => {
            walk_place(place, fs, fe);
            walk_expr(value, fs, fe);
        }
        Stmt::Expr(e) | Stmt::Return(Some(e), _) => walk_expr(e, fs, fe),
        _ => {}
    }
}

fn walk_place<'b>(p: &'b Place, fs: &mut impl FnMut(&'b Stmt), fe: &mut impl FnMut(&'b Expr)) {
    for s in &p.steps {
        if let Step::Index(e) = s {
            walk_expr(e, fs, fe);
        }
    }
}

pub(crate) fn walk_expr<'b>(e: &'b Expr, fs: &mut impl FnMut(&'b Stmt), fe: &mut impl FnMut(&'b Expr)) {
    fe(e);
    let mut sub = |x: &'b Expr| walk_expr(x, fs, fe);
    match &e.kind {
        ExprKind::Read { place, .. } => {
            for s in &place.steps {
                if let Step::Index(i) = s {
                    sub(i);
                }
            }
        }
        ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::Tuple(args) | ExprKind::List(args) | ExprKind::Array(args) | ExprKind::Concat(args) | ExprKind::SelfSend { args, .. } => {
            args.iter().for_each(&mut sub)
        }
        ExprKind::MemberCall { instance, args, .. } => {
            sub(instance);
            args.iter().for_each(&mut sub);
        }
        ExprKind::UnionToStr { value, arms } => {
            sub(value);
            arms.iter().for_each(&mut sub);
        }
        ExprKind::HandlerCall { instance, args, .. } => {
            sub(instance);
            args.iter().for_each(&mut sub);
        }
        ExprKind::Construct { fields } => fields.iter().for_each(|(_, v)| sub(v)),
        ExprKind::MakeUnion { value, .. } | ExprKind::Rewrap { value, .. } | ExprKind::DropMut { value } | ExprKind::BorrowAs { value, .. } | ExprKind::Widen { value } | ExprKind::Spread { value } => sub(value),
        ExprKind::Handle { instance } => sub(instance),
        ExprKind::AddrInstance { addr } => sub(addr),
        ExprKind::Throw { message } => sub(message),
        ExprKind::Send { addr, args, .. } => {
            sub(addr);
            args.iter().for_each(&mut sub);
        }
        ExprKind::Assert { cond, message, .. } => {
            sub(cond);
            if let Some(m) = message {
                sub(m);
            }
        }
        ExprKind::Unreachable { message: Some(m), .. } => sub(m),
        ExprKind::Test { subject, .. } => sub(subject),
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                walk_expr(c, fs, fe);
                walk_block(b, fs, fe);
            }
            if let Some(o) = otherwise {
                walk_block(o, fs, fe);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            walk_expr(subject, fs, fe);
            for a in arms {
                walk_block(&a.body, fs, fe);
            }
        }
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => walk_block(body, fs, fe),
        ExprKind::Spawn { handler, deps, pool, join, .. } => {
            sub(handler);
            deps.iter().for_each(&mut sub);
            if let Some(p) = pool {
                walk_expr(p, fs, fe);
            }
            if let Some(j) = join {
                walk_expr(j, fs, fe);
            }
        }
        ExprKind::ReplyTo { captures, pool, .. } => {
            captures.iter().for_each(&mut sub);
            if let Some(p) = pool {
                walk_expr(p, fs, fe);
            }
        }
        _ => {}
    }
}

/// Whether an expression never yields (a block ending in a jump).
/// [rs-never-call] A call whose Salvo type is `Never`: a `-> Never` fn, an
/// effect member such as `Process.exit`. Rust sees most of them as `-> ()`
/// (the program's own fns and effect traits keep `()`), so in value position
/// the call needs an `unreachable!()` after it to have a value's type.
pub(crate) fn never_call(e: &Expr) -> bool {
    matches!(e.ty, Ty::Never) && matches!(e.kind, ExprKind::Call { .. } | ExprKind::MemberCall { .. } | ExprKind::HandlerCall { .. })
}

/// [rs-loop-temp] `e` (a block-valued `Branch` or `Switch`) with `wrap`
/// applied to the value of every arm that has one and does not diverge,
/// typed `ty`; `None` when an arm has no value to wrap.
fn copy_into_tails(e: &Expr, wrap: &dyn Fn(Expr) -> Expr, ty: &Ty) -> Option<Expr> {
    let block = |b: &Block| -> Option<Block> {
        let v = b.value.as_deref()?;
        let value = if diverges(v) || matches!(v.ty, Ty::Never) {
            v.clone()
        } else {
            match copy_into_tails(v, wrap, ty) {
                Some(inner) => inner,
                None => wrap(v.clone()),
            }
        };
        Some(Block { stmts: b.stmts.clone(), value: Some(Box::new(value)), span: b.span })
    };
    let kind = match &e.kind {
        ExprKind::Branch { id, arms, otherwise } => {
            let arms = arms.iter().map(|(c, b)| block(b).map(|b| (c.clone(), b))).collect::<Option<Vec<_>>>()?;
            let otherwise = match otherwise {
                Some(b) => Some(block(b)?),
                None => None,
            };
            ExprKind::Branch { id: *id, arms, otherwise }
        }
        ExprKind::Switch { id, subject, arms } => {
            let arms = arms
                .iter()
                .map(|a| block(&a.body).map(|body| salvo_core::ir::SwitchArm { test: a.test.clone(), body }))
                .collect::<Option<Vec<_>>>()?;
            ExprKind::Switch { id: *id, subject: subject.clone(), arms }
        }
        _ => return None,
    };
    Some(Expr { ty: ty.clone(), span: e.span, kind })
}

pub(crate) fn diverges(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Throw { .. } | ExprKind::Unreachable { .. } => true,
        // [rs-never-call] A call answering `Never`, which `value` spells as a
        // block Rust can see diverge.
        _ if never_call(e) => true,
        // Every arm of a choice ends in an escape (a `when` whose arms all
        // `return`).
        ExprKind::Switch { arms, .. } => {
            let blk = |b: &Block| b.value.as_ref().map(|v| diverges(v)).unwrap_or_else(|| matches!(b.stmts.last(), Some(Stmt::Return(..) | Stmt::Break | Stmt::Continue)) || matches!(b.stmts.last(), Some(Stmt::Expr(x)) if diverges(x)));
            !arms.is_empty() && arms.iter().all(|a| blk(&a.body))
        }
        ExprKind::Branch { arms, otherwise, .. } => {
            let blk = |b: &Block| b.value.as_ref().map(|v| diverges(v)).unwrap_or_else(|| matches!(b.stmts.last(), Some(Stmt::Return(..) | Stmt::Break | Stmt::Continue)) || matches!(b.stmts.last(), Some(Stmt::Expr(x)) if diverges(x)));
            arms.iter().all(|(_, b)| blk(b)) && otherwise.as_ref().map_or(arms.len() == 1 && matches!(arms[0].0.kind, ExprKind::Bool(true)), blk)
        }
        _ => false,
    }
}

fn read_root(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Read { place, .. } => Some(place.root.0.clone()),
        ExprKind::DropMut { value } => read_root(value),
        _ => None,
    }
}

pub(crate) fn bare(e: &Expr) -> Option<&Local> {
    match &e.kind {
        ExprKind::Read { place, .. } if place.steps.is_empty() => Some(&place.root),
        ExprKind::DropMut { value } => bare(value),
        _ => None,
    }
}

/// Two types alike once their qualifiers go (a narrowing's arm by type).
fn alike(a: &Ty, b: &Ty) -> bool {
    strip_all(a) == strip_all(b)
}

fn strip_all(t: &Ty) -> Ty {
    match t {
        Ty::Qualified { base, .. } => strip_all(base),
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(strip_all).collect() },
        Ty::Union(a) => Ty::Union(a.iter().map(strip_all).collect()),
        Ty::Tuple(a) => Ty::Tuple(a.iter().map(strip_all).collect()),
        Ty::Array(e) => Ty::Array(Box::new(strip_all(e))),
        other => other.clone(),
    }
}

/// The arm of `from` a narrowed type `to` is: exact first, then alike.
fn arm_of(from: &Ty, to: &Ty) -> Option<usize> {
    let arms: Vec<&Ty> = from.strip_quals().value_arms();
    if let Some(i) = arms.iter().position(|a| *a == to) {
        return Some(i);
    }
    let same: Vec<usize> = arms.iter().enumerate().filter(|(_, a)| alike(a, to)).map(|(i, _)| i).collect();
    (same.len() == 1).then(|| same[0])
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    fn pad(indent: usize) -> String {
        "    ".repeat(indent)
    }

    pub(crate) fn fresh(&mut self, base: &str) -> String {
        self.f.tmp += 1;
        format!("__{base}{}", self.f.tmp)
    }

    /// `module:line:col` of a span in this module's source.
    pub fn location(&self, span: salvo_syntax::Span) -> String {
        match self.s.program.files.iter().position(|f| f.module == self.module.path) {
            Some(i) => {
                let file = &self.s.program.files[i];
                // [origin] Copied code is named where it lands.
                let span = salvo_core::origin::landing(&self.s.program.modules[i], span);
                let (line, col) = salvo_syntax::span::span_line_col(&file.content, span);
                format!("{}:{line}:{col}", file.module)
            }
            None => format!("{}", self.module.path),
        }
    }

    /// Pre-scans a body for what its bindings need to know.
    fn scan(&mut self, b: &Block) {
        let mut consumed = HashSet::new();
        let mut handles: HashMap<String, usize> = HashMap::new();
        let mut calls: Vec<Expr> = Vec::new();
        let mut assigned = HashSet::new();
        let mut captured = HashSet::new();
        walk_block(b, &mut |s| {
            if let Stmt::Assign { place, .. } = s {
                assigned.insert(place.root.0.clone());
            }
        }, &mut |e| match &e.kind {
            ExprKind::Read { place, consume: true, .. } => {
                consumed.insert(place.root.0.clone());
            }
            ExprKind::Lambda { captures, .. } => {
                captured.extend(captures.iter().map(|c| c.local.0.clone()));
            }
            ExprKind::ReplyTo { captures, .. } => {
                for c in captures {
                    if let Some(r) = bare(c) {
                        captured.insert(r.0.clone());
                    }
                }
            }
            ExprKind::Handle { instance } => {
                if let Some(r) = bare(instance) {
                    *handles.entry(r.0.clone()).or_default() += 1;
                }
            }
            ExprKind::Call { .. } | ExprKind::MemberCall { .. } => calls.push(e.clone()),
            _ => {}
        });
        for c in &calls {
            let (params, args): (Vec<Param>, &Vec<Expr>) = match &c.kind {
                ExprKind::Call { target: FnRef::Decl(id), args, .. } => (self.s.fn_decl(id).map(|f| self.s.unalias_params(&f.params)).unwrap_or_default(), args),
                ExprKind::MemberCall { member, args, .. } => (self.member_decl(member).map(|m| self.s.unalias_params(&m.params)).unwrap_or_default(), args),
                _ => continue,
            };
            for (p, a) in params.iter().zip(args) {
                if p.mode == PassMode::LentMut {
                    if let Some(r) = bare(a) {
                        self.f.lent_mut.insert(r.0.clone());
                    }
                }
            }
        }
        self.f.consumed.extend(consumed);
        self.f.assigned.extend(assigned);
        self.f.captured.extend(captured);
        for (k, v) in handles {
            *self.f.handles.entry(k).or_default() += v;
        }
    }

    pub fn member_decl(&self, m: &salvo_core::ir::MemberRef) -> Option<&'p salvo_core::ir::Member> {
        match self.s.decls.get(&m.interface) {
            Some(salvo_core::ir::Decl::Interface(i)) => i.members.get(m.index),
            _ => None,
        }
    }

    /// A fn body at `indent`.
    pub fn fn_body(&mut self, b: &Block, indent: usize) -> String {
        self.scan(b);
        // [rs-repoint] A re-pointed view gets a variable per live range.
        let is_view = |t: &Ty| self.s.holds_proj(t) && !salvo_core::ir::build::refs::has_ref(t);
        let plan = super::repoint::plan(b, &is_view, &self.f.captured);
        self.f.repoint.extend(plan);
        let mut out = self.stmts(&b.stmts, indent);
        if let (Some(v), Some((_, opt))) = (&b.value, self.f.loc.clone()) {
            // A tail that escapes in every arm answers through its returns.
            if diverges(v) {
                let code = self.value(v, indent);
                out.push_str(&format!("{}{code};\n", Self::pad(indent)));
                return out;
            }
            let code = self.loc_expr(v, opt, indent);
            out.push_str(&format!("{}return {code};\n", Self::pad(indent)));
        } else if let Some(v) = &b.value {
            let ret = self.f.ret.clone().unwrap_or(Ty::Unknown);
            let code = self.value_into(v, &ret, indent);
            if self.f.throws.is_some() {
                out.push_str(&format!("{}return std::ops::ControlFlow::Continue({code});\n", Self::pad(indent)));
            } else {
                out.push_str(&format!("{}return {code};\n", Self::pad(indent)));
            }
        }
        out
    }

    pub fn block_stmts_pub(&mut self, b: &Block, indent: usize) -> String {
        self.stmts(&b.stmts, indent)
    }

    fn stmts(&mut self, stmts: &[Stmt], indent: usize) -> String {
        let mut out = String::new();
        for s in stmts {
            match self.f.repoint.get(&super::repoint::key(s)).cloned() {
                // [rs-repoint] The value stored here is a new variable's.
                Some(super::repoint::Split::Shadow(_, ty)) => {
                    let Stmt::Assign { place, value } = s else { unreachable!() };
                    out.push_str(&self.let_stmt(&place.root, &ty, value, indent));
                }
                Some(super::repoint::Split::Declare(vs)) => {
                    for (name, ty) in vs {
                        let annot = if self.annotatable(&ty) && (!self.nested_proj(&ty) || self.concrete(&ty)) { format!(": {}", self.ty(&ty)) } else { String::new() };
                        out.push_str(&format!("{}let mut {}{annot};\n", Self::pad(indent), rs_local(&name)));
                    }
                    out.push_str(&self.stmt(s, indent));
                }
                None => out.push_str(&self.stmt(s, indent)),
            }
        }
        out
    }

    fn stmt(&mut self, s: &Stmt, indent: usize) -> String {
        let pad = Self::pad(indent);
        match s {
            Stmt::Loop { body, .. } => {
                let b = self.stmts(&body.stmts, indent + 1);
                format!("{pad}loop {{\n{b}{pad}}}\n")
            }
            Stmt::ForEach { local, ty, iterable, body, .. } => self.for_each(local, ty, iterable, body, indent),
            Stmt::Let { local, ty, value, .. } => self.let_stmt(local, ty, value, indent),
            Stmt::Narrow { local, ty, from, from_ty, because, .. } => self.narrow(local, ty, from, from_ty, because, indent),
            Stmt::Unpack { locals, from, from_ty, variant, .. } => {
                // [actor-dispatch] The payload of the variant just tested.
                let (text, _) = self.place_text(from, indent);
                let Some((e, m)) = (match from_ty.strip_quals() {
                    Ty::Named { name, .. } => self.s.generated_enum(name),
                    _ => None,
                }) else {
                    self.error("an unpack of something that is not a generated enum");
                    return String::new();
                };
                let v = &e.variants[*variant];
                let names: Vec<String> = locals.iter().map(|(l, _)| rs_local(&l.0)).collect();
                for (l, t) in locals {
                    self.f.tys.insert(l.0.clone(), t.clone());
                    self.f.kinds.insert(l.0.clone(), if is_proj(t) { Kind::Ref } else { Kind::Owned });
                }
                let pat = if names.is_empty() { String::new() } else { format!("({})", names.join(", ")) };
                format!("{pad}let {}{}::{}{pat} = {text} else {{ unreachable!() }};\n", self.s.prefix(&m), rs_ident(&e.name), v.name)
            }
            Stmt::Alias { local, ty, place, .. } => {
                // [deduce-field] No binding: the place, at every use.
                let (text, _) = self.place_text(place, indent);
                self.f.tys.insert(local.0.clone(), ty.clone());
                self.f.kinds.insert(local.0.clone(), Kind::Elem);
                self.f.elem.insert(local.0.clone(), (text, None));
                String::new()
            }
            Stmt::Assign { place, value } => {
                let v = match self.place_ty(place) {
                    Some(t) => self.value_into(value, &t, indent),
                    None => self.value(value, indent),
                };
                let p = self.lvalue(place, indent);
                format!("{pad}{p} = {v};\n")
            }
            Stmt::Expr(e) => {
                let code = self.expr_stmt(e, indent);
                format!("{pad}{code};\n")
            }
            Stmt::Return(None, _) if self.f.throws.is_some() => format!("{pad}return std::ops::ControlFlow::Continue(());\n"),
            Stmt::Return(Some(e), _) if self.f.throws.is_some() => {
                let ret = self.f.ret.clone().unwrap_or(Ty::Unknown);
                if ret.is_none_ty() {
                    let code = self.expr_stmt(e, indent);
                    return format!("{pad}{code};\n{pad}return std::ops::ControlFlow::Continue(());\n");
                }
                let code = self.value_into(e, &ret, indent);
                format!("{pad}return std::ops::ControlFlow::Continue({code});\n")
            }
            Stmt::Return(None, _) => format!("{pad}return;\n"),
            Stmt::Return(Some(e), _) if self.f.loc.is_some() => {
                let opt = self.f.loc.as_ref().is_some_and(|(_, o)| *o);
                let code = self.loc_expr(e, opt, indent);
                format!("{pad}return {code};\n")
            }
            Stmt::Return(Some(e), _) => {
                let ret = self.f.ret.clone().unwrap_or(Ty::Unknown);
                if ret.is_none_ty() || matches!(ret, Ty::Never) {
                    if matches!(e.kind, ExprKind::Unit | ExprKind::MakeNone) {
                        return format!("{pad}return;\n");
                    }
                    let code = self.expr_stmt(e, indent);
                    return format!("{pad}{code};\n{pad}return;\n");
                }
                // [rs-fn-field] A fn value returned is stored: the fn answers
                // an `Arc<dyn Fn>`.
                let code = if matches!(ret.strip_quals(), Ty::Fn { .. }) { self.fn_arg(e, &ret, FnPos::Stored, indent) } else { self.value_into(e, &ret, indent) };
                format!("{pad}return {code};\n")
            }
            Stmt::Break => format!("{pad}break;\n"),
            Stmt::Continue => format!("{pad}continue;\n"),
            // [ir-obligation] A ledger directive: metadata for the IR-level
            // linear check, no runtime effect.
            Stmt::Obligation(_) => String::new(),
        }
    }

    /// An expression in statement position (its value dropped).
    fn expr_stmt(&mut self, e: &Expr, indent: usize) -> String {
        match &e.kind {
            ExprKind::Branch { .. } | ExprKind::Switch { .. } => self.value(e, indent),
            // A statement needs no value, so a `Never` call stands bare.
            _ if never_call(e) => self.value_of(e, indent),
            _ => self.value(e, indent),
        }
    }

    fn let_stmt(&mut self, local: &Local, ty: &Ty, value: &Expr, indent: usize) -> String {
        // [rs-path] A new binding of the name ends any handle it was.
        self.f.ref_handles.remove(&local.0);
        let pad = Self::pad(indent);
        let n = rs_local(&local.0);
        self.f.tys.insert(local.0.clone(), ty.clone());
        // [rs-fn-field] A fn value a call answers, or a field holds, is an
        // `Arc<dyn Fn>`.
        let arc_fn = matches!(self.s.unalias(ty).strip_quals(), Ty::Fn { .. })
            && match &value.kind {
                ExprKind::Call { target: FnRef::Decl(id), .. } => !self.s.ast_fn(id).is_some_and(|f| f.intrinsic),
                ExprKind::Read { place, .. } => !place.steps.is_empty() || self.f.arc_fns.contains(&place.root.0),
                _ => false,
            };
        if arc_fn {
            self.f.arc_fns.insert(local.0.clone());
        } else {
            self.f.arc_fns.remove(&local.0);
        }
        // [effect-handle] A handle over a handler instance.
        let is_face = matches!(ty.strip_quals(), Ty::Named { name, .. } if self.s.symbols.effects.contains_key(name.as_str()));
        let handler_read = bare(value).filter(|r| {
            let t = self.f.tys.get(&r.0).cloned().unwrap_or_else(|| value.ty.clone());
            is_face && matches!(t.strip_quals(), Ty::Named { name, .. } if self.s.symbols.handlers.contains_key(name.as_str()))
        });
        if handler_read.is_some() || matches!(value.kind, ExprKind::Handle { .. }) {
            let code = self.handle(ty, value, indent);
            self.f.kinds.insert(local.0.clone(), Kind::Owned);
            return format!("{pad}let {n} = {code};\n");
        }
        // A handler instance several faces share sits behind an `Arc`.
        if self.f.handles.get(&local.0).copied().unwrap_or(0) > 1 {
            let code = self.value(value, indent);
            let stateful = self.handler_stateful(&value.ty);
            self.f.shared_inst.insert(local.0.clone());
            self.f.kinds.insert(local.0.clone(), Kind::Owned);
            return if stateful {
                format!("{pad}let {n} = std::sync::Arc::new(std::sync::Mutex::new({code}));\n")
            } else {
                format!("{pad}let {n} = std::sync::Arc::new({code});\n")
            };
        }
        // [rs-elem-mut] A mutable handle on an element is its position, the
        // element re-rendered at every use [rs-loc].
        if salvo_core::ir::build::refs::has_ref(ty) {
            // [rs-path] Another name for a handle: the same binding.
            if let Some(r) = bare(value) {
                if let Some(b) = self.handle_of(&r.0) {
                    self.bind_handle(&local.0, b);
                    return String::new();
                }
            }
            if let Some(m) = self.mint(value, indent) {
                let p = self.fresh("h");
                let pt_text = self.path_rust(&m.pt);
                let mut out: String = m.pre.iter().map(|l| format!("{pad}{l}\n")).collect();
                if m.opt {
                    out.push_str(&format!("{pad}let {p}: Option<{pt_text}> = {};\n", m.code));
                    self.bind_handle(&local.0, super::paths::HandleBinding::Optional { var: p, prefix: m.prefix, pt: m.pt });
                } else {
                    out.push_str(&format!("{pad}let {p}: {pt_text} = {};\n", m.code));
                    let mut steps = m.prefix.steps.clone();
                    steps.extend(self.steps_of_value(&m.pt, &p));
                    self.bind_handle(&local.0, super::paths::HandleBinding::Total(super::paths::Handle { root: m.prefix.root, steps }));
                }
                return out;
            }
        }
        // [rs-opt-borrow] Another name for a union of borrows: a copy of
        // it (`Option<&T>`, `Union2<&A, &B>` are `Copy`) — unless the name is
        // reassigned later, which stores an owned value of the IR type in it:
        // such a local owns its value from the start [rs-borrow-locals]
        // (ROADMAP §2d item 11: `let k = key` then `k = up(here)` was E0308).
        let reassigned = self.f.assigned.contains(&local.0);
        if bare(value).is_some() && !super::is_borrowed_union(ty) && reassigned {
            let st = self.storage_ty(value);
            if super::is_borrowed_union(&st) {
                let src = self.value(value, indent);
                let sarms: Vec<Ty> = st.value_arms().into_iter().cloned().collect();
                let code = self.own_borrowed_union(&src, &sarms, st.has_none_arm(), ty);
                self.f.kinds.insert(local.0.clone(), Kind::Owned);
                let annot = if self.annotatable(ty) { format!(": {}", self.ty(ty)) } else { String::new() };
                return format!("{pad}let mut {n}{annot} = {code};\n");
            }
        }
        if bare(value).is_some() && !super::is_borrowed_union(ty) && !reassigned {
            let st = self.storage_ty(value);
            if super::is_borrowed_union(&st) {
                let code = self.value(value, indent);
                self.f.tys.insert(local.0.clone(), st);
                self.f.kinds.insert(local.0.clone(), Kind::Owned);
                return format!("{pad}let mut {n} = {code};\n");
            }
        }
        // [rs-borrow-locals] A borrow-mode binding of a pure place, never
        // reassigned, mutated through, captured or consumed, is a borrow
        // of the place: the checker poisons it before any write the
        // borrow would outlive [fate-poison].
        if let Some(code) = self.borrow_local(local, ty, value, indent) {
            let k = if self.f.mut_alias.contains(&local.0) { Kind::RefMut } else { Kind::Ref };
            self.f.kinds.insert(local.0.clone(), k);
            return format!("{pad}let mut {n} = {code};\n");
        }
        let kind = if is_proj(ty) { if is_mut_borrow(ty) { Kind::RefMut } else { Kind::Ref } } else { Kind::Owned };
        // [rs-loop-temp] A borrow the binding keeps must not be of a
        // temporary: such an argument is bound first.
        let (pre, value) = self.hoist_borrowed_temps(value, indent);
        let value = &value;
        let code = self.value_into(value, ty, indent);
        if !pre.is_empty() {
            self.f.kinds.insert(local.0.clone(), kind);
            let annot = if self.annotatable(ty) { format!(": {}", self.ty(ty)) } else { String::new() };
            return format!("{pre}{pad}let mut {n}{annot} = {code};\n");
        }
        self.f.kinds.insert(local.0.clone(), kind);
        // [rs-repoint] A view split into several variables has no later
        // assignment left to infer its type from: said when it can be.
        let split = self.f.repoint.values().any(|x| x.names(&local.0));
        let annot = if self.annotatable(ty) && (!self.nested_proj(ty) || (split && self.concrete(ty))) { format!(": {}", self.ty(ty)) } else { String::new() };
        format!("{pad}let mut {n}{annot} = {code};\n")
    }

    /// Whether `t` names no type variable: its rendering is the value's.
    fn concrete(&self, t: &Ty) -> bool {
        !foreign_var(t, &HashSet::new()) && !self.f.generics.iter().any(|g| super::mentions_var(t, g))
    }

    fn borrow_local(&mut self, local: &Local, ty: &Ty, value: &Expr, indent: usize) -> Option<String> {
        let ExprKind::Read { place, consume, .. } = &value.kind else { return None };
        // A consuming read of a borrowed root cannot move: it borrows too.
        if *consume && !matches!(self.f.kinds.get(&place.root.0), Some(Kind::Ref | Kind::RefMut)) {
            return None;
        }
        if is_proj(ty) || is_copy_ty(ty) || is_proj(&value.ty) || matches!(ty.strip_quals(), Ty::Fn { .. }) || self.s.holds_proj(ty) {
            return None;
        }
        let l = &local.0;
        if self.f.assigned.contains(l) || self.f.captured.contains(l) || self.f.consumed.contains(l) || self.f.lent_mut.contains(l) {
            return None;
        }
        if !place.steps.iter().all(|s| matches!(s, Step::Field(_) | Step::Tuple(_))) || self.f.loc.is_some() {
            return None;
        }
        match self.f.kinds.get(&place.root.0) {
            Some(Kind::Owned | Kind::Ref | Kind::RefMut | Kind::SelfField) => {}
            _ => return None,
        }
        if self.f.loc_index.contains_key(&place.root.0) {
            return None;
        }
        if self.f.mut_alias.contains(l) {
            return Some(self.borrow_mut(value, indent));
        }
        Some(self.borrow(value, indent))
    }

    /// A container type over borrowed elements: its Rust type is the
    /// value's to say (a generic producer may hand back owned ones).
    fn nested_proj(&self, t: &Ty) -> bool {
        match t.strip_quals() {
            Ty::Named { args, .. } => args.iter().any(|a| self.s.holds_proj(a)),
            _ => false,
        }
    }

    /// The lent, non-place arguments of a call whose result keeps a borrow,
    /// as `let`s before it.
    fn hoist_borrowed_temps(&mut self, e: &Expr, indent: usize) -> (String, Expr) {
        let ExprKind::Call { target: FnRef::Decl(id), type_args, args, .. } = &e.kind else { return (String::new(), e.clone()) };
        if !self.s.holds_proj(&e.ty) {
            return (String::new(), e.clone());
        }
        let Some(f) = self.s.fn_decl(id) else { return (String::new(), e.clone()) };
        let mut pre = String::new();
        let mut new_args = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let lent = f.params.get(i).is_some_and(|p| p.mode != PassMode::Moved && !is_copy_ty(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. }));
            let place = matches!(a.kind, ExprKind::Read { .. } | ExprKind::DropMut { .. } | ExprKind::FnValue(_) | ExprKind::Lambda { .. });
            if lent && !place && !is_proj(&a.ty) {
                let t = self.fresh("tmp");
                let v = self.value(a, indent);
                pre.push_str(&format!("{}let mut {t} = {v};\n", Self::pad(indent)));
                self.f.kinds.insert(t.clone(), Kind::Owned);
                self.f.tys.insert(t.clone(), a.ty.clone());
                new_args.push(Expr { ty: a.ty.clone(), span: a.span, kind: ExprKind::Read { place: Place { root: Local(t), steps: Vec::new() }, consume: false, moved: false } });
            } else {
                new_args.push(a.clone());
            }
        }
        (pre, Expr { ty: e.ty.clone(), span: e.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: type_args.clone(), args: new_args, moved: false } })
    }

    fn annotatable(&self, t: &Ty) -> bool {
        fn has_fn(t: &Ty) -> bool {
            match t {
                Ty::Fn { .. } => true,
                Ty::Qualified { base, .. } => has_fn(base),
                Ty::Named { args, .. } | Ty::Union(args) | Ty::Tuple(args) => args.iter().any(has_fn),
                Ty::Array(e) => has_fn(e),
                Ty::Unknown | Ty::Any | Ty::Never => true,
                _ => false,
            }
        }
        !has_fn(t)
    }

    fn handler_stateful(&self, t: &Ty) -> bool {
        match t.strip_quals() {
            Ty::Named { name, .. } => match self.s.impl_decl(name) {
                Some(h) if h.platform => !h.threadsafe,
                Some(h) => h.stateful,
                None => true,
            },
            _ => true,
        }
    }

    /// [effect-handle] [rs-handle] The face's handle over an instance.
    fn handle(&mut self, face: &Ty, value: &Expr, indent: usize) -> String {
        let inst = match &value.kind {
            ExprKind::Handle { instance } => instance.as_ref(),
            _ => value,
        };
        let face_path = match face.strip_quals() {
            Ty::Named { name, .. } => self.type_path(name),
            _ => self.ty(face),
        };
        let inst_ty = bare(inst).and_then(|r| self.f.tys.get(&r.0).cloned()).unwrap_or_else(|| inst.ty.clone());
        let stateful = self.handler_stateful(&inst_ty);
        if let Some(r) = bare(inst) {
            if self.f.shared_inst.contains(&r.0) {
                let n = rs_local(&r.0);
                return if stateful { format!("{face_path}::share_locked({n}.clone())") } else { format!("{face_path}::share_shared({n}.clone())") };
            }
            let n = rs_local(&r.0);
            return if stateful { format!("{face_path}::locked({n})") } else { format!("{face_path}::shared({n})") };
        }
        let code = self.value(inst, indent);
        if stateful { format!("{face_path}::locked({code})") } else { format!("{face_path}::shared({code})") }
    }

    // ------------------------------------------------------------ places --

    /// The place's text and the kind of its root.
    pub(crate) fn place_text(&mut self, p: &Place, indent: usize) -> (String, Kind) {
        // [mod-use] A module's static, through its accessor.
        if !self.f.kinds.contains_key(&p.root.0) && self.s.statics.get(&self.module.path).is_some_and(|s| s.contains(&p.root.0)) {
            let mut out = format!("{}{}()", self.s.prefix(&self.module.path), rs_local(&p.root.0));
            for s in &p.steps {
                match s {
                    Step::Field(f) => {
                        out.push('.');
                        out.push_str(&rs_ident(f));
                    }
                    Step::Tuple(i) => out.push_str(&format!(".{i}")),
                    Step::Index(e) => {
                        let i = self.value(e, indent);
                        out.push_str(&format!("[({i}) as usize]"));
                    }
                }
            }
            return (out, Kind::Ref);
        }
        // [rs-path] A handle local: its container's root, walked along the path.
        if let Some(b) = self.f.ref_handles.get(&p.root.0).cloned() {
            match b {
                super::paths::HandleBinding::Total(h) => {
                    let mut steps = h.steps.clone();
                    for s in &p.steps {
                        steps.push(match s {
                            Step::Field(f) => super::paths::SStep::Field(f.clone()),
                            Step::Tuple(i) => super::paths::SStep::Tuple(*i),
                            Step::Index(e) => {
                                let i = self.value(e, indent);
                                super::paths::SStep::Dyn(super::paths::PStep::Index, format!("(({i}) as usize)"))
                            }
                        });
                    }
                    let m = self.f.place_mut;
                    return (self.render_path(&h.root, &steps, m), Kind::Elem);
                }
                super::paths::HandleBinding::Optional { var, prefix, pt } => {
                    if !p.steps.is_empty() {
                        self.error("a field of an optional handle read before it was tested [rs-path]");
                    }
                    // [rs-path] Used as a value, an optional handle is the
                    // optional element: its path, walked if present.
                    let x = "__x".to_string();
                    let mut steps = prefix.steps.clone();
                    steps.extend(self.steps_of_value(&pt, &x));
                    let m = self.f.place_mut;
                    let place = self.render_path(&prefix.root, &steps, m);
                    let r = if m { "&mut " } else { "&" };
                    return (format!("{var}.map(|{x}| {r}{place})"), Kind::Owned);
                }
            }
        }
        let kind = self.f.kinds.get(&p.root.0).copied().unwrap_or(Kind::Owned);
        let mut out = match kind {
            Kind::SelfField => format!("self.{}", rs_local(&p.root.0)),
            Kind::Elem => match self.f.elem.get(&p.root.0) {
                Some((a, Some(i))) => format!("{a}[{i}]"),
                Some((a, None)) => a.clone(),
                None => rs_local(&p.root.0),
            },
            _ => rs_local(&p.root.0),
        };
        for s in &p.steps {
            match s {
                Step::Field(f) => {
                    out.push('.');
                    out.push_str(&rs_ident(f));
                }
                Step::Tuple(i) => out.push_str(&format!(".{i}")),
                Step::Index(e) => {
                    let i = self.value(e, indent);
                    out.push_str(&format!("[({i}) as usize]"));
                }
            }
        }
        (out, kind)
    }

    /// The type stored at a place, when it can be told.
    fn place_ty(&self, p: &Place) -> Option<Ty> {
        let t = self.place_ty_raw(p)?;
        if super::body::foreign_var(&t, &self.f.generics) && !matches!(t, Ty::Var(_)) {
            return None;
        }
        Some(t)
    }

    /// The declared type at a place, type variables and all.
    fn place_ty_raw(&self, p: &Place) -> Option<Ty> {
        let mut t = self.f.tys.get(&p.root.0).cloned()?;
        for s in &p.steps {
            t = match (s, t.strip_quals()) {
                (Step::Field(f), Ty::Named { name, .. }) => self.s.struct_decl(name)?.fields.iter().find(|x| &x.name == f)?.ty.clone(),
                (Step::Tuple(i), Ty::Tuple(es)) => es.get(*i)?.clone(),
                (Step::Index(_), Ty::Named { args, .. }) => args.first()?.clone(),
                (Step::Index(_), Ty::Array(e)) => (**e).clone(),
                _ => return None,
            };
        }
        Some(t)
    }

    fn lvalue(&mut self, p: &Place, indent: usize) -> String {
        let saved = std::mem::replace(&mut self.f.place_mut, true);
        let (text, kind) = self.place_text(p, indent);
        self.f.place_mut = saved;
        if p.steps.is_empty() && matches!(kind, Kind::Ref | Kind::RefMut) {
            return format!("*{text}");
        }
        text
    }

    // ------------------------------------------------------- expressions --

    /// The value of `e` for a slot of type `slot`: a borrow where the slot
    /// is a `proj` the value is not.
    pub fn value_into(&mut self, e: &Expr, slot: &Ty, indent: usize) -> String {
        // What a read actually holds: the place's stored type.
        let ety = match &e.kind {
            ExprKind::Read { place, .. } => self.place_ty(place).filter(|t| !matches!(t, Ty::Var(_)) && !t.is_unknown()).map(|t| self.s.unalias(&t)).unwrap_or_else(|| e.ty.clone()),
            // A call answers its declaration's type, when that names no variable.
            ExprKind::Call { target: FnRef::Decl(id), .. } => match self.s.fn_decl(id) {
                Some(f) if !foreign_var(&f.ret, &HashSet::new()) && f.throws.is_none() => f.ret.clone(),
                _ => e.ty.clone(),
            },
            _ => e.ty.clone(),
        };
        // [type-none-unit] A `None`-typed expression into an *optional* slot
        // (`T?`, a union with both value arms and `None`): the value is
        // absent, but the expression may have an effect (`return ignore(_)`
        // for a `Str?`), so evaluate it, then yield the absent optional. Not
        // for a bare `None` slot (that is unit `()`, handled elsewhere), and
        // not for a plain `None`/`MakeNone`/`Unit` expression.
        if slot.strip_quals().has_none_arm()
            && !slot.strip_quals().value_arms().is_empty()
            && ety.is_none_ty()
            && !matches!(e.kind, ExprKind::MakeNone | ExprKind::Unit)
            && !matches!(e.ty, Ty::Never)
            && !diverges(e)
        {
            let v = self.expr_stmt(e, indent);
            return format!("{{ {v}; None }}");
        }
        // A present value into an optional slot.
        if slot.strip_quals().has_none_arm() && !ety.strip_quals().has_none_arm() && !e.ty.strip_quals().without_none().is_none_ty() && !matches!(ety, Ty::Never | Ty::Unknown) && !ety.is_none_ty() && !matches!(e.kind, ExprKind::MakeNone | ExprKind::Unreachable { .. }) && !diverges(e) {
            let inner = slot.strip_quals().without_none();
            let v = self.value_into(e, &inner, indent);
            return format!("Some({v})");
        }
        if is_proj(slot) && !is_proj(&e.ty) && !matches!(e.kind, ExprKind::MakeNone) {
            return if is_mut(slot) { self.borrow_mut(e, indent) } else { self.borrow(e, indent) };
        }
        self.value(e, indent)
    }

    /// `&T` of `e`.
    pub fn borrow(&mut self, e: &Expr, indent: usize) -> String {
        if let ExprKind::DropMut { value } = &e.kind {
            return self.borrow(value, indent);
        }
        if diverges(e) {
            return self.value(e, indent);
        }
        if let ExprKind::Read { place, .. } = &e.kind {
            if self.f.kinds.get(&place.root.0) == Some(&Kind::Elem) {
                let (text, _) = self.place_text(place, indent);
                return format!("&{text}");
            }
        }
        if is_proj(&e.ty) {
            let v = self.value(e, indent);
            return if is_mut(&e.ty) { format!("&*{v}") } else { v };
        }
        // A branch's value borrowed: each arm's tail borrowed where it stands.
        if matches!(e.kind, ExprKind::Branch { .. } | ExprKind::Switch { .. }) && !is_copy_ty(&e.ty) && tails_lendable(e) {
            let (tmp, errs, failed) = (self.f.tmp, self.s.errors.len(), self.f.borrow_tail_failed);
            self.f.borrow_tail = Some(false);
            self.f.borrow_tail_failed = false;
            let v = self.value(e, indent);
            self.f.borrow_tail = None;
            if !std::mem::replace(&mut self.f.borrow_tail_failed, failed) {
                return v;
            }
            self.f.tmp = tmp;
            self.s.errors.truncate(errs);
            let v = self.value(e, indent);
            return format!("&{v}");
        }
        if let ExprKind::Read { place, .. } = &e.kind {
            let (text, kind) = self.place_text(place, indent);
            if place.steps.is_empty() {
                return match kind {
                    Kind::Ref => text,
                    Kind::RefMut => format!("&*{text}"),
                    _ => format!("&{text}"),
                };
            }
            return format!("&{text}");
        }
        let v = self.value(e, indent);
        format!("&{v}")
    }

    /// `&mut T` of `e`.
    pub fn borrow_mut(&mut self, e: &Expr, indent: usize) -> String {
        if let ExprKind::DropMut { value } = &e.kind {
            return self.borrow_mut(value, indent);
        }
        if diverges(e) {
            return self.value(e, indent);
        }
        // [rs-narrow-mut] A branch's value lent mutably (`add(xs!, 3)`): each
        // arm's tail is the storage reached `&mut`, never a clone of it.
        if matches!(e.kind, ExprKind::Branch { .. } | ExprKind::Switch { .. }) && !is_copy_ty(&e.ty) && tails_lendable(e) {
            let mut narrows: Vec<(String, String)> = Vec::new();
            walk_expr(e, &mut |st| {
                if let Stmt::Narrow { local, from, .. } = st {
                    narrows.push((local.0.clone(), from.root.0.clone()));
                }
            }, &mut |_| {});
            for (l, from) in narrows {
                self.f.lent_mut.insert(l);
                self.f.mut_alias.insert(from);
            }
            let (tmp, errs, failed) = (self.f.tmp, self.s.errors.len(), self.f.borrow_tail_failed);
            self.f.borrow_tail = Some(true);
            self.f.borrow_tail_failed = false;
            let v = self.value(e, indent);
            self.f.borrow_tail = None;
            if !std::mem::replace(&mut self.f.borrow_tail_failed, failed) {
                return v;
            }
            self.f.tmp = tmp;
            self.s.errors.truncate(errs);
            self.error("cannot lower this mutable use: the value it would mutate is a temporary of its own expression [backend-never-wrong]");
            return self.value(e, indent);
        }
        if let ExprKind::Read { place, .. } = &e.kind {
            let saved = std::mem::replace(&mut self.f.place_mut, true);
            let (text, kind) = self.place_text(place, indent);
            self.f.place_mut = saved;
            if place.steps.is_empty() {
                return match kind {
                    Kind::Ref | Kind::RefMut => format!("&mut *{text}"),
                    _ => format!("&mut {text}"),
                };
            }
            return format!("&mut {text}");
        }
        if is_proj(&e.ty) {
            return self.value(e, indent);
        }
        let v = self.value(e, indent);
        format!("&mut {v}")
    }

    /// `e` as written, for a method receiver or an intrinsic's argument
    /// that borrows natively.
    pub fn raw(&mut self, e: &Expr, indent: usize) -> String {
        if let ExprKind::DropMut { value } = &e.kind {
            return self.raw(value, indent);
        }
        if is_copy_ty(&e.ty) {
            return self.value(e, indent);
        }
        if let ExprKind::Read { place, .. } = &e.kind {
            return self.place_text(place, indent).0;
        }
        self.value(e, indent)
    }

    /// An owned value of `e`'s type (a reference, for a `proj` type).
    pub fn value(&mut self, e: &Expr, indent: usize) -> String {
        // [rs-never-call] `{ exit(9); unreachable!() }` has every type, which
        // the bare call (`()` to Rust) does not.
        if never_call(e) {
            let call = self.value_of(e, indent);
            return format!("{{ {call}; unreachable!() }}");
        }
        self.value_of(e, indent)
    }

    /// `value` without the [rs-never-call] wrapping.
    fn value_of(&mut self, e: &Expr, indent: usize) -> String {
        match &e.kind {
            ExprKind::Int(v) => match e.ty.strip_quals() {
                Ty::Named { name, .. } if name == "Long" => format!("{v}i64"),
                Ty::Named { name, .. } if name == "Byte" => format!("{v}u8"),
                _ => format!("{v}i32"),
            },
            ExprKind::Long(v) => format!("{v}i64"),
            ExprKind::Float(v) => format!("{v:?}f32"),
            ExprKind::Double(v) => format!("{v:?}f64"),
            ExprKind::Bool(v) => v.to_string(),
            ExprKind::Char(c) => format!("'{}'", escape_char(*c)),
            ExprKind::Str(s) => format!("String::from(\"{}\")", escape_string(s)),
            ExprKind::Unit => "()".to_string(),
            ExprKind::MakeNone => "None".to_string(),
            ExprKind::Read { place, consume, .. } => {
                let saved = std::mem::replace(&mut self.f.place_mut, is_proj(&e.ty) && is_mut_borrow(&e.ty));
                let (text, kind) = self.place_text(place, indent);
                self.f.place_mut = saved;
                let bare = place.steps.is_empty();
                if is_copy_ty(&e.ty) {
                    // A scalar stored as a borrow (a `proj T` field at `T = Int`).
                    let stored_ref = !bare && self.place_ty_raw(place).is_some_and(|t| is_proj(&t));
                    return if (bare && matches!(kind, Kind::Ref | Kind::RefMut)) || stored_ref { format!("*{text}") } else { text };
                }
                if is_proj(&e.ty) {
                    if kind == Kind::Elem {
                        return if is_mut_borrow(&e.ty) { format!("&mut {text}") } else { format!("&{text}") };
                    }
                    return if bare && kind == Kind::RefMut { format!("&mut *{text}") } else { text };
                }
                if matches!(self.s.unalias(&e.ty).strip_quals(), Ty::Fn { .. }) {
                    // An owned callback moves; a stored one is a shared `Arc`.
                    return if bare && kind == Kind::Owned { text } else if bare && kind == Kind::RefMut { text } else { format!("{text}.clone()") };
                }
                if (*consume || self.uncloneable(&e.ty)) && kind == Kind::Owned && !self.through_ref(place) {
                    text
                } else if *consume && kind == Kind::SelfField && bare {
                    // [state-take] a state field handed over, refilled before use.
                    format!("std::mem::take(&mut {text})")
                } else {
                    format!("{text}.clone()")
                }
            }
            ExprKind::Call { target, type_args, args, .. } => self.call(e, target, type_args, args, indent),
            ExprKind::MemberCall { instance, member, args, .. } => {
                let inst = self.raw(instance, indent);
                let Some(m) = self.member_decl(member) else {
                    self.error("a member call to an unknown member");
                    return "()".to_string();
                };
                // [rs-path] A lending member used as a value: its path, walked once.
                if self.s.interface_by_id(&member.interface).is_some_and(|i| self.s.member_lends_mut(i, m)) {
                    if let Some(mint) = self.mint(e, indent) {
                        return self.materialize(mint);
                    }
                    self.error("a lending effect member called on something that is not a place [rs-path]");
                }
                let ps = self.s.unalias_params(&m.params);
                let a = self.args(&ps, args, FnPos::DynParam, indent);
                format!("{inst}.{}({})", rs_ident(&m.emitted_name), a.join(", "))
            }
            ExprKind::Op { op, args } => self.op(e, *op, args, indent),
            ExprKind::HandlerCall { instance, member, args } => self.handler_call(instance, member, args, indent),
            // [interp-union] `UnionN::to_str`, given the function of each arm.
            ExprKind::UnionToStr { value, arms } => {
                let v = self.raw(value, indent);
                // The method is generic in each arm, so every arm function
                // takes its payload by reference, whatever the arm's type.
                let slot = Ty::Fn { params: vec![Ty::Var("T".into())], ret: Box::new(Ty::named("Str")), contract: None, effects: Vec::new() };
                let fs: Vec<String> = arms.iter().map(|a| self.fn_arg(a, &slot, FnPos::DynParam, indent)).collect();
                format!("{v}.to_str({})", fs.join(", "))
            }
            ExprKind::Construct { fields } => self.construct(e, fields, indent),
            ExprKind::MakeUnion { arm, value } => {
                let arms: Vec<Ty> = e.ty.strip_quals().value_arms().into_iter().cloned().collect();
                let at = arms.get(*arm).cloned().unwrap_or(Ty::Unknown);
                let v = self.value_into(value, &at, indent);
                let wrapped = if arms.len() >= 2 {
                    self.s.union_sizes.insert(arms.len());
                    format!("crate::unions::Union{}::U{}({v})", arms.len(), arm + 1)
                } else {
                    v
                };
                if e.ty.strip_quals().has_none_arm() { format!("Some({wrapped})") } else { wrapped }
            }
            ExprKind::Rewrap { from, value } => {
                let v = self.value(value, indent);
                self.rewrap(&v, from, &e.ty)
            }
            ExprKind::DropMut { value } => self.value(value, indent),
            // [rs-opt-borrow] Outside an argument slot (which lends it, see
            // `lend_value`): the union of borrows of what it reads.
            ExprKind::BorrowAs { value, .. } => {
                let st = self.storage_ty(value);
                let to = super::borrowed_arms(&e.ty, false);
                if st == to {
                    return self.value(value, indent);
                }
                let v = self.value(value, indent);
                self.lend_union(&format!("&({v})"), &st, &to, false)
            }
            ExprKind::Present { value } => {
                let inner = e.ty.strip_quals().without_none();
                let v = self.value_into(value, &inner, indent);
                format!("Some({v})")
            }
            ExprKind::Widen { value } => {
                let v = self.value(value, indent);
                let t = self.ty(&e.ty);
                format!("(({v}) as {t})")
            }
            ExprKind::Tuple(es) => {
                let mut a = Vec::new();
                let elems: Vec<Ty> = match e.ty.strip_quals() {
                    Ty::Tuple(t) => t.clone(),
                    _ => Vec::new(),
                };
                for (i, x) in es.iter().enumerate() {
                    let slot = elems.get(i).cloned().unwrap_or_else(|| x.ty.clone());
                    a.push(self.value_into(x, &slot, indent));
                }
                if a.len() == 1 { format!("({},)", a[0]) } else { format!("({})", a.join(", ")) }
            }
            ExprKind::List(es) | ExprKind::Array(es) => {
                if es.iter().any(|x| matches!(x.kind, ExprKind::Spread { .. })) {
                    let mut parts = String::from("{ let mut __v = Vec::new(); ");
                    for x in es {
                        match &x.kind {
                            ExprKind::Spread { value } => {
                                let v = self.raw(value, indent);
                                parts.push_str(&format!("__v.extend({v}.iter().cloned()); "));
                            }
                            _ => {
                                let v = self.value(x, indent);
                                parts.push_str(&format!("__v.push({v}); "));
                            }
                        }
                    }
                    parts.push_str("__v }");
                    return parts;
                }
                let elem = match e.ty.strip_quals() {
                    Ty::Array(t) => (**t).clone(),
                    Ty::Named { args, .. } if !args.is_empty() => args[0].clone(),
                    _ => Ty::Unknown,
                };
                let a: Vec<String> = es.iter().map(|x| self.value_into(x, &elem, indent)).collect();
                format!("vec![{}]", a.join(", "))
            }
            ExprKind::Spread { value } => self.value(value, indent),
            ExprKind::Concat(parts) => self.concat(parts, indent),
            ExprKind::Branch { id, arms, otherwise } => {
                // [safe-nav] [rs-safe-nav] A `?.` whose links are all field
                // reads renders as an `.as_ref().and_then(..).map(..)`
                // combinator chain; anything else (a call link, or a shape the
                // chain cannot express) falls back to the generic branch,
                // which is correct if less idiomatic.
                if let Some(code) = self.safe_nav_chain(e, indent) {
                    code
                } else {
                    self.branch(e, *id, arms, otherwise.as_ref(), indent)
                }
            }
            ExprKind::Switch { id, subject, arms } => self.switch(e, *id, subject, arms, indent),
            ExprKind::Test { id, subject, test } => {
                self.f.test_arms.insert((id.0, usize::MAX), test.clone());
                let s = self.raw_subject(subject, indent);
                self.test_code(&s, &subject.ty, test)
            }
            ExprKind::Lambda { params, ret, body, .. } => self.lambda(params, ret, body, &e.ty, FnPos::Local, indent),
            ExprKind::FnValue(r) => self.fn_value(e, r, &e.ty, FnPos::Local, indent),
            ExprKind::Assert { cond, message, at } => {
                let c = self.value(cond, indent);
                match message {
                    Some(m) => {
                        let text = self.raw(m, indent);
                        format!("if !({c}) {{ panic!(\"salvo: {{}} at {at}\", {text}) }}")
                    }
                    None => format!("if !({c}) {{ panic!(\"salvo: assertion failed at {at}\") }}"),
                }
            }
            ExprKind::Unreachable { message, at } => match message {
                Some(m) => match &m.kind {
                    ExprKind::Str(s) => format!("panic!(\"salvo: {} at {at}\")", escape_format_text(s)),
                    _ => {
                        let text = self.raw(m, indent);
                        format!("panic!(\"salvo: {{}} at {at}\", {text})")
                    }
                },
                None => format!("panic!(\"salvo: unreachable at {at}\")"),
            },
            ExprKind::Handle { .. } => self.handle(&e.ty.clone(), e, indent),
            ExprKind::Spawn { handler, pool, join, .. } => self.spawn(handler, pool.as_deref(), join.as_deref(), indent),
            ExprKind::Send { addr, member, args } => self.send(addr, member, args, indent),
            ExprKind::ReplyTo { target, captures, gated, pool } => self.replyto(target, captures, *gated, pool.as_deref(), indent),
            ExprKind::WaitFor { local, token_ty, body } => self.waitfor(local, token_ty, body, indent),
            ExprKind::SelfAddr => "self.__addr.expect(\"an actor's own addr\")".to_string(),
            ExprKind::SelfSend { member, args } => self.self_send(member, args, indent),
            ExprKind::AddrInstance { addr } => self.addr_instance(e, addr, indent),
            ExprKind::Throw { message } => {
                let mt = message.ty.clone();
                let m = self.value(message, indent);
                self.propagate(&m, &mt)
            }
            ExprKind::Try { body } => {
                let label = format!("'try_{}", { self.f.tmp += 1; self.f.tmp });
                self.f.tries.push((label.clone(), e.ty.clone()));
                let arms: Vec<Ty> = e.ty.strip_quals().value_arms().into_iter().cloned().collect();
                let ok = arms.first().cloned().unwrap_or(Ty::none());
                let n = arms.len().max(2);
                self.s.union_sizes.insert(n);
                let saved = self.f.kinds.clone();
                let mut out = self.stmts(&body.stmts, indent + 1);
                let v = match &body.value {
                    Some(v) => self.value_into(v, &ok, indent + 1),
                    None => "()".to_string(),
                };
                self.f.kinds = saved;
                self.f.tries.pop();
                out.push_str(&format!("{}crate::unions::Union{n}::U1({v})\n", Self::pad(indent + 1)));
                format!("({label}: {{\n{out}{}}})", Self::pad(indent))
            }
            ExprKind::Unsupported(what) => {
                self.error(format!("unsupported IR node: {what}"));
                "()".to_string()
            }
        }
    }

    /// [rs-host-fields] A value with no `Clone` (a reply token, a linear
    /// host value): a read of it is a move.
    pub fn uncloneable(&self, t: &Ty) -> bool {
        // An opaque host object is moved, never cloned.
        let opaque_host = matches!(t.strip_quals(), Ty::Named { name, .. } if self.s.symbols.intrinsic_types.get(name.as_str()).is_some_and(|d| d.platform && (d.linear || !d.auto_qualifiers.iter().any(|q| q.name.name == "Mut"))));
        opaque_host || self.host_limits_pub(t).1
    }

    /// Whether a place's path passes through a reference (so a move out of
    /// it is impossible).
    fn through_ref(&self, p: &Place) -> bool {
        let kind = self.f.kinds.get(&p.root.0).copied().unwrap_or(Kind::Owned);
        if kind != Kind::Owned {
            return true;
        }
        // A field of a local holding a borrowing struct.
        self.f.tys.get(&p.root.0).is_some_and(|t| is_proj(t))
    }

    fn op(&mut self, e: &Expr, op: Op, args: &[Expr], indent: usize) -> String {
        let a: Vec<String> = args.iter().map(|x| self.value(x, indent)).collect();
        let int = match e.ty.strip_quals() {
            Ty::Named { name, .. } if name == "Int" => Some("i32"),
            Ty::Named { name, .. } if name == "Long" => Some("i64"),
            Ty::Named { name, .. } if name == "Byte" => Some("u8"),
            _ => None,
        };
        let wrap = |f: &str| -> Option<String> { int.map(|t| format!("{t}::{f}({}, {})", a[0], a[1])) };
        match op {
            Op::Add => wrap("wrapping_add").unwrap_or_else(|| format!("({} + {})", a[0], a[1])),
            Op::Sub => wrap("wrapping_sub").unwrap_or_else(|| format!("({} - {})", a[0], a[1])),
            Op::Mul => wrap("wrapping_mul").unwrap_or_else(|| format!("({} * {})", a[0], a[1])),
            Op::Div => wrap("wrapping_div").unwrap_or_else(|| format!("({} / {})", a[0], a[1])),
            Op::Rem => wrap("wrapping_rem").unwrap_or_else(|| format!("({} % {})", a[0], a[1])),
            Op::And => format!("({} && {})", a[0], a[1]),
            Op::Or => format!("({} || {})", a[0], a[1]),
            Op::Not => format!("!({})", a[0]),
            Op::Neg => match int {
                Some(t) => format!("{t}::wrapping_neg({})", a[0]),
                None => format!("(-{})", a[0]),
            },
            Op::Lt | Op::Gt | Op::LtEq | Op::GtEq | Op::Eq | Op::NotEq => format!("({} {} {})", a[0], sym(op), a[1]),
        }
    }

    fn concat(&mut self, parts: &[Expr], indent: usize) -> String {
        if parts.iter().all(|p| matches!(p.kind, ExprKind::Str(_))) {
            let text: String = parts.iter().map(|p| if let ExprKind::Str(s) = &p.kind { s.clone() } else { String::new() }).collect();
            return format!("String::from(\"{}\")", escape_string(&text));
        }
        // [rs-mut-arg-hoist] `format!` borrows every argument at once, so a
        // part that lends mutably evaluates the parts into temps, in order.
        let lends_mut = parts.iter().any(|p| {
            let mut hit = false;
            walk_expr(p, &mut |_| {}, &mut |e| {
                if let ExprKind::Call { target: FnRef::Decl(id), args, .. } = &e.kind {
                    if let Some(f) = self.s.fn_decl(id) {
                        hit |= f.params.iter().zip(args).any(|(p, _)| p.mode == PassMode::LentMut);
                    }
                }
            });
            hit
        });
        let mut fmt = String::new();
        let mut args = Vec::new();
        let mut lets = String::new();
        for p in parts {
            if lends_mut && !matches!(p.kind, ExprKind::Str(_)) {
                fmt.push_str("{}");
                let scalar = self.scalar_to_str_arg(p);
                let v = match scalar {
                    Some(x) => self.value(x, indent),
                    None => self.value(p, indent),
                };
                let t = self.fresh("part");
                lets.push_str(&format!("let {t} = {v}; "));
                args.push(t);
                continue;
            }
            match &p.kind {
                ExprKind::Str(s) => fmt.push_str(&escape_format_text(s)),
                _ => {
                    fmt.push_str("{}");
                    let scalar = self.scalar_to_str_arg(p);
                    args.push(match scalar {
                        Some(x) => self.value(x, indent),
                        None => self.raw(p, indent),
                    });
                }
            }
        }
        if !lets.is_empty() {
            return format!("{{ {lets}format!(\"{fmt}\", {}) }}", args.join(", "));
        }
        format!("format!(\"{fmt}\", {})", args.join(", "))
    }

    /// The scalar of a `to_str` call whose text is `Display`'s.
    fn scalar_to_str_arg<'e>(&self, e: &'e Expr) -> Option<&'e Expr> {
        let ExprKind::Call { target: FnRef::Decl(id), args, .. } = &e.kind else { return None };
        let [arg] = args.as_slice() else { return None };
        let is_to_str = self.s.ast_fn(id).is_some_and(|f| f.intrinsic && f.name.name == "to_str");
        let scalar = matches!(arg.ty.strip_quals(), Ty::Named { name, .. } if matches!(name.as_str(), "Int" | "Long" | "Byte" | "Char" | "Bool" | "Str"));
        (is_to_str && scalar).then_some(arg)
    }

    fn construct(&mut self, e: &Expr, fields: &[(String, Expr)], indent: usize) -> String {
        let Ty::Named { name, args } = e.ty.strip_quals().without_none().strip_quals().clone() else {
            self.error("a construct of a non-named type");
            return "()".to_string();
        };
        if let Some(h) = self.s.impl_decl(&name) {
            // A handler: `H::new(params…, deps…)`.
            let mut a = Vec::new();
            for p in &h.ctor_params {
                match fields.iter().find(|(n, _)| *n == p.local.0) {
                    Some((_, v)) if matches!(p.ty.strip_quals(), Ty::Fn { .. }) => {
                        let code = self.fn_arg(v, &p.ty, FnPos::Owned, indent);
                        a.push(code);
                    }
                    Some((_, v)) => a.push(self.value_into(v, &p.ty, indent)),
                    None => {
                        self.error(format!("handler `{}` constructed without `{}`", h.name, p.local.0));
                    }
                }
            }
            for i in 0..h.deps.len() {
                if let Some((_, v)) = fields.iter().find(|(n, _)| *n == format!("__dep{i}")) {
                    a.push(self.value(v, indent));
                }
            }
            let path = if h.platform {
                self.s.platform_hosts.insert(h.id.module().clone());
                format!("{}{}", self.s.prefix(h.id.module()), crate::emit::platform_adapter_name(&rs_ident(&h.name)))
            } else {
                self.type_path(&name)
            };
            return format!("{path}::new({})", a.join(", "));
        }
        let decl = self.s.struct_decl(&name);
        let path = self.type_path(&name);
        let _ = args;
        let mut fs = Vec::new();
        for (n, v) in fields {
            let fty = decl.and_then(|d| d.fields.iter().find(|f| f.name == *n)).map(|f| f.ty.clone()).unwrap_or_else(|| v.ty.clone());
            let code = if matches!(fty.strip_quals(), Ty::Fn { .. }) {
                self.fn_arg(v, &fty, FnPos::Stored, indent)
            } else {
                self.value_into(v, &fty, indent)
            };
            fs.push(format!("{}: {code}", rs_ident(n)));
        }
        if fs.is_empty() {
            format!("{path} {{}}")
        } else {
            format!("{path} {{ {} }}", fs.join(", "))
        }
    }

    /// A value moved between two union representations.
    /// [rs-union-convert] The one primitive that converts a value of union
    /// type `from` into union type `to`. Every reshape — a permutation
    /// (`Str | Bool` → `Bool | Str`), a widen or narrow of arms, dropping or
    /// keeping `None`, owning a borrowed arm, and the flat↔nested spread a
    /// union-spanning generic needs — is this one `match`, mapping each
    /// source arm to the target arm of the same **arm identity**
    /// [union-arm-identity].
    ///
    /// `own` clones a borrowed payload into an owned target slot (the former
    /// `own_borrowed_union`); without it a payload passes through unchanged
    /// (the former `rewrap`). Arm identity is matched by exact type, then by
    /// `alike` (qualifiers/borrow erased). Three arm shapes beyond a plain
    /// one-to-one map:
    ///   * a source arm that is itself a bare union the target *spreads*
    ///     (its inner arms appear directly in `to`): recurse, so the inner
    ///     value is re-wrapped into `to`;
    ///   * a target arm that is itself a bare union the source *collapses
    ///     into* (several source arms land in one `to` arm): wrap the payload
    ///     into that inner union first;
    ///   * no match: `unreachable!` (the runtime never produces it).
    fn convert_union(&mut self, code: &str, from: &Ty, to: &Ty, own: bool) -> String {
        let fa: Vec<Ty> = from.strip_quals().value_arms().into_iter().cloned().collect();
        let ta: Vec<Ty> = to.strip_quals().value_arms().into_iter().cloned().collect();
        let (fo, to_opt) = (from.strip_quals().has_none_arm(), to.strip_quals().has_none_arm());
        // [rs-union-convert] An identity reshape — same rendered type, same
        // `None`, and (when owning) no borrowed arm to clone — is a no-op.
        // (Flattening means a nested single-arm wrapper renders identically
        // to its inner union, so this also absorbs that case.)
        if self.ty(from) == self.ty(to) && fo == to_opt && !(own && fa.iter().any(is_proj)) {
            return code.to_string();
        }
        let pat = |s: &mut Self, arms: &[Ty], i: usize, opt: bool, inner: &str| -> String {
            let p = if arms.len() >= 2 {
                s.s.union_sizes.insert(arms.len());
                format!("crate::unions::Union{}::U{}({inner})", arms.len(), i + 1)
            } else {
                inner.to_string()
            };
            if opt { format!("Some({p})") } else { p }
        };
        // The payload of source arm `a`, owned if asked and the arm is a
        // borrow.
        let own_payload = |a: &Ty, inner: &str| -> String {
            if own && !is_copy_ty(a) && is_proj(a) { format!("({inner}).clone()") } else { inner.to_string() }
        };
        // Where source arm `a` lands in the target: exact arm, then unique
        // `alike` arm.
        let land = |ta: &[Ty], a: &Ty| -> Option<usize> {
            ta.iter().position(|b| b == a).or_else(|| {
                let same: Vec<usize> = ta.iter().enumerate().filter(|(_, b)| alike(b, a)).map(|(j, _)| j).collect();
                (same.len() == 1).then(|| same[0])
            })
        };
        let mut arms = Vec::new();
        for (i, a) in fa.iter().enumerate() {
            let lhs = pat(self, &fa, i, fo, "__v");
            match land(&ta, a) {
                Some(j) => {
                    let payload = own_payload(a, "__v");
                    let rhs = pat(self, &ta, j, to_opt, &payload);
                    arms.push(format!("{lhs} => {rhs}"));
                }
                None => {
                    // The source arm is itself a bare union the target
                    // spreads across its own arms: re-wrap the inner value
                    // into the target.
                    let inner_arms: Vec<Ty> = a.strip_quals().value_arms().into_iter().cloned().collect();
                    if inner_arms.len() >= 2 {
                        let inner = self.convert_union("__v", a, to, own);
                        arms.push(format!("{lhs} => {inner}"));
                        continue;
                    }
                    // Or the target has one arm that is itself a bare union
                    // containing this source arm (`A` into `Ok (A | B)` /
                    // the nested span of a generic): wrap into the inner
                    // union, then into the outer arm.
                    if let Some((oj, inner_ty)) = ta.iter().enumerate().find_map(|(k, t)| {
                        let ia: Vec<&Ty> = t.strip_quals().value_arms();
                        (ia.len() >= 2 && land(&ia.iter().map(|x| (*x).clone()).collect::<Vec<_>>(), a).is_some()).then(|| (k, t.clone()))
                    }) {
                        let payload = own_payload(a, "__v");
                        let inner = self.convert_union(&payload, a, &inner_ty, own);
                        let rhs = pat(self, &ta, oj, to_opt, &inner);
                        arms.push(format!("{lhs} => {rhs}"));
                        continue;
                    }
                    arms.push(format!("{lhs} => unreachable!(\"salvo: unreachable union arm\")"));
                }
            }
        }
        if fo {
            arms.push(if to_opt { "None => None".to_string() } else { "None => unreachable!(\"salvo: unreachable union arm\")".to_string() });
        }
        if fa.len() < 2 && !fo {
            // A plain value placed into the target: its one arm, owned if
            // asked, into the arm it lands on.
            let payload = own_payload(fa.first().unwrap_or(&Ty::Unknown), code);
            let j = land(&ta, &from.strip_quals()).or_else(|| fa.first().and_then(|a| land(&ta, a))).unwrap_or(0);
            return pat(self, &ta, j, to_opt, &payload);
        }
        arms.push("#[allow(unreachable_patterns)] _ => unreachable!(\"salvo: unreachable union arm\")".to_string());
        format!("(match {code} {{ {} }})", arms.join(", "))
    }

    /// The plain union reshape: payloads pass through unchanged
    /// [rs-union-convert].
    fn rewrap(&mut self, code: &str, from: &Ty, to: &Ty) -> String {
        self.convert_union(code, from, to, false)
    }

    /// [generic-union-span] The *nested* slot a declared union type becomes
    /// when a type parameter in it is bound to a span of ≥2 union arms, or
    /// `None` when no such nesting arises. The caller's value is physically
    /// flat (`union_of` never nests), so when a declared arm is a type
    /// variable the call bound to a multi-arm union, the callee's
    /// monomorphised slot nests (`Union2<Union2<A,B>, C>`) and the argument
    /// must be rewrapped. Only a *bare* type-variable arm counts — a tagged
    /// payload (`Ok T`) stays one arm whatever `T` is, so it never nests.
    fn span_nested_slot(&self, declared: &Ty) -> Option<Ty> {
        if self.f.call_subst.is_empty() {
            return None;
        }
        let d = declared.strip_quals();
        let Ty::Union(arms) = d else { return None };
        // A bare type-variable arm bound to a multi-arm union is the trigger.
        let nests = arms.iter().any(|arm| {
            matches!(arm, Ty::Var(g) if self.f.call_subst.get(g).is_some_and(|b| b.strip_quals().value_arms().len() >= 2))
        });
        if !nests {
            return None;
        }
        Some(super::decls::subst_ty(d, &self.f.call_subst))
    }

    // ----------------------------------------------------- control flow --

    fn block(&mut self, b: &Block, slot: Option<&Ty>, indent: usize) -> String {
        let pad = Self::pad(indent);
        let saved = self.f.kinds.clone();
        let tail = self.f.borrow_tail.take();
        let mut out = self.stmts(&b.stmts, indent + 1);
        if tail.is_some() {
            if let Some(r) = b.value.as_deref().and_then(read_root) {
                let here = b.stmts.iter().any(|s| matches!(s, Stmt::Let { local, .. } | Stmt::Narrow { local, .. } | Stmt::Alias { local, .. } if local.0 == r));
                if here && matches!(self.f.kinds.get(&r), Some(Kind::Owned) | None) {
                    self.f.borrow_tail_failed = true;
                }
            }
        }
        if let Some(v) = &b.value {
            let code = match (tail, slot) {
                (Some(true), _) => self.borrow_mut(v, indent + 1),
                (Some(false), _) => self.borrow(v, indent + 1),
                (None, Some(t)) => self.value_into(v, t, indent + 1),
                (None, None) => self.value(v, indent + 1),
            };
            out.push_str(&format!("{}{code}\n", Self::pad(indent + 1)));
        }
        self.f.kinds = saved;
        format!("{{\n{out}{pad}}}")
    }

    /// [safe-nav] [rs-safe-nav] Render a `?.` lowering as an
    /// `.and_then(..)`/`.map(..)` combinator chain when every link is a field
    /// read. Returns `None` for a call link, a non-field link, or a non-`?.`
    /// branch, so the caller falls back to the generic (correct) block.
    ///
    /// Each link's body reuses `self.value` on the link's own `Read` node, so
    /// the per-field clone/copy decision is the emitter's own ([rs-read-mode])
    /// rather than re-derived here. The closure parameter *is* the link's
    /// `nav` local (a fresh name, so no collision), bound `Owned`: `.and_then`
    /// on an owned `Option<T>` gives `T`, and on an `Option<&T>` gives `&T` —
    /// a non-`consume` field read `.clone()`s / copies through either, so the
    /// body is valid for both.
    fn safe_nav_chain(&mut self, e: &Expr, indent: usize) -> Option<String> {
        use salvo_core::ir::safe_nav::match_safe_nav;
        // Collect the links, innermost receiver last.
        let mut links: Vec<(&Local, &Expr, bool)> = Vec::new(); // (nav, field-read value, already_opt)
        let mut cur = e;
        let receiver;
        loop {
            let nav = match_safe_nav(cur)?;
            // The link must be a plain field read of the narrowed receiver.
            if safe_nav_field(nav.value, nav.nav).is_none() {
                return None;
            }
            links.push((nav.nav, nav.value, nav.already_opt));
            // Recurse into a nested `?.` receiver; stop at the base.
            if match_safe_nav(nav.receiver).is_some() {
                cur = nav.receiver;
            } else {
                receiver = nav.receiver;
                break;
            }
        }
        links.reverse(); // base-first
        let mut code = self.value(receiver, indent);
        for (nav, value, already_opt) in links {
            let p = rs_local(&nav.0);
            // The closure parameter is the link's own `nav`, bound owned; the
            // field read renders through `self.value`.
            let saved = self.f.kinds.insert(nav.0.clone(), Kind::Owned);
            let body = self.value(value, indent);
            match saved {
                Some(k) => {
                    self.f.kinds.insert(nav.0.clone(), k);
                }
                None => {
                    self.f.kinds.remove(&nav.0);
                }
            }
            // An already-optional link chains with `and_then` (its body is an
            // `Option`); a plain-value link with `map` (its body is the value,
            // which `map` lifts into the `Option`).
            let combinator = if already_opt { "and_then" } else { "map" };
            code = format!("{code}.{combinator}(|{p}| {body})");
        }
        Some(code)
    }

    fn branch(&mut self, e: &Expr, id: salvo_core::ir::NodeId, arms: &[(Expr, Block)], otherwise: Option<&Block>, indent: usize) -> String {
        let _ = id;
        let slot = (!e.ty.is_none_ty()).then(|| e.ty.clone());
        let tail = self.f.borrow_tail.take();
        // A block expression: one `true` arm.
        if arms.len() == 1 && otherwise.is_none() && matches!(arms[0].0.kind, ExprKind::Bool(true)) {
            self.f.borrow_tail = tail;
            return self.block(&arms[0].1, slot.as_ref(), indent);
        }
        let mut out = String::new();
        for (i, (c, b)) in arms.iter().enumerate() {
            let cond = self.value(c, indent);
            self.f.borrow_tail = tail;
            let body = self.block(b, slot.as_ref(), indent);
            if i > 0 {
                out.push_str(" else ");
            }
            out.push_str(&format!("if {cond} {body}"));
        }
        match otherwise {
            Some(o) => {
                self.f.borrow_tail = tail;
                let body = self.block(o, slot.as_ref(), indent);
                out.push_str(&format!(" else {body}"));
            }
            None if slot.is_some() => out.push_str(" else { unreachable!() }"),
            None => {}
        }
        out
    }

    /// [actor-dispatch] A member of a handler *instance*, called directly:
    /// a face's through its trait (the instance is the implementing struct),
    /// the handler's own as an inherent method.
    fn handler_call(&mut self, instance: &Expr, member: &salvo_core::ir::HandlerMember, args: &[Expr], indent: usize) -> String {
        let Ty::Named { name, .. } = instance.ty.strip_quals().clone() else {
            self.error("a handler call on something that is not a handler");
            return "()".to_string();
        };
        let Some(h) = self.s.impl_decl(&name) else {
            self.error(format!("a handler call on `{name}`, which is not a handler"));
            return "()".to_string();
        };
        match member {
            salvo_core::ir::HandlerMember::Face(m) => {
                let Some(mem) = self.member_decl(m) else {
                    self.error("a handler call of an unknown member");
                    return "()".to_string();
                };
                let Some(iface) = self.s.interface_by_id(&m.interface) else { return "()".to_string() };
                let tr = if h.stateful { stateful_trait_name(&iface.name) } else { stateless_trait_name(&iface.name) };
                let path = self.path_in(m.interface.module(), &tr);
                let inst = self.borrow_mut(instance, indent);
                let mut ps = self.s.unalias_params(&mem.params);
                if mem.send {
                    // A message is delivered by value.
                    ps.iter_mut().for_each(|p| p.mode = PassMode::Moved);
                }
                let a = self.args(&ps, args, FnPos::DynParam, indent);
                let all: Vec<String> = std::iter::once(inst).chain(a).collect();
                format!("{path}::{}({})", rs_ident(&mem.emitted_name), all.join(", "))
            }
            salvo_core::ir::HandlerMember::Own(n) => {
                let inst = self.raw(instance, indent);
                let own = h.members.iter().chain(h.init.as_ref()).find(|f| &f.name == n);
                let mut ps: Vec<Param> = own
                    .map(|f| self.s.unalias_params(&f.params).into_iter().skip(f.effect_params).take(f.params.len() - f.effect_params - f.implicit_params).collect())
                    .unwrap_or_default();
                if own.is_some_and(|f| f.send) {
                    // A message is delivered by value.
                    ps.iter_mut().for_each(|p| p.mode = PassMode::Moved);
                }
                let a = self.args(&ps, args, FnPos::Param, indent);
                format!("{inst}.{}({})", rs_ident(n), a.join(", "))
            }
        }
    }

    /// The text a test reads its subject through.
    fn raw_subject(&mut self, subject: &Expr, indent: usize) -> String {
        match &subject.kind {
            ExprKind::Read { place, .. } => self.place_text(place, indent).0,
            _ => {
                let v = self.value(subject, indent);
                format!("({v})")
            }
        }
    }

    fn switch(&mut self, e: &Expr, id: salvo_core::ir::NodeId, subject: &Expr, arms: &[salvo_core::ir::SwitchArm], indent: usize) -> String {
        let slot = (!e.ty.is_none_ty()).then(|| e.ty.clone());
        let tail = self.f.borrow_tail.take();
        let (subj, prelude) = match &subject.kind {
            ExprKind::Read { .. } => (self.raw_subject(subject, indent), None),
            _ => {
                let t = self.fresh("subj");
                let v = self.value(subject, indent);
                (t.clone(), Some(format!("let {t} = {v};")))
            }
        };
        let mut out = String::new();
        for (i, a) in arms.iter().enumerate() {
            self.f.test_arms.insert((id.0, i), a.test.clone());
            self.f.borrow_tail = tail;
            let body = self.block(&a.body, slot.as_ref(), indent);
            let last = i + 1 == arms.len();
            if i > 0 {
                out.push_str(" else ");
            }
            if last && i > 0 {
                out.push_str(&body);
            } else if last && matches!(a.test, ArmTest::Else) {
                out.push_str(&body);
            } else {
                let cond = self.test_code(&subj, &subject.ty, &a.test);
                out.push_str(&format!("if {cond} {body}"));
                if last {
                    out.push_str(if slot.is_some() { " else { unreachable!() }" } else { "" });
                }
            }
        }
        match prelude {
            Some(p) => format!("{{ {p} {out} }}"),
            None => out,
        }
    }

    fn test_code(&mut self, subject: &str, st: &Ty, test: &ArmTest) -> String {
        // [actor-dispatch] A generated enum: its variant.
        if let (Ty::Named { name, .. }, ArmTest::Arm(i)) = (st.strip_quals(), test) {
            if let Some((e, m)) = self.s.generated_enum(name) {
                let v = &e.variants[*i];
                let rest = if v.fields.is_empty() { "" } else { "(..)" };
                return format!("matches!({subject}, {}{}::{}{rest})", self.s.prefix(&m), rs_ident(&e.name), v.name);
            }
        }
        let arms: Vec<Ty> = st.strip_quals().value_arms().into_iter().cloned().collect();
        let n = arms.len();
        let opt = st.strip_quals().has_none_arm();
        let pat = |s: &mut Self, i: usize, inner: &str| -> String {
            let p = if n >= 2 {
                s.s.union_sizes.insert(n);
                format!("crate::unions::Union{n}::U{}({inner})", i + 1)
            } else {
                inner.to_string()
            };
            if opt { format!("Some({p})") } else { p }
        };
        match test {
            ArmTest::None => format!("{subject}.is_none()"),
            ArmTest::Else => "true".to_string(),
            ArmTest::Arm(i) => {
                if n < 2 {
                    if opt { format!("{subject}.is_some()") } else { "true".to_string() }
                } else {
                    let p = pat(self, *i, "_");
                    format!("matches!({subject}, {p})")
                }
            }
            ArmTest::Arms(is) => {
                let ps: Vec<String> = is.iter().map(|i| pat(self, *i, "_")).collect();
                format!("matches!({subject}, {})", ps.join(" | "))
            }
            ArmTest::Lit(lits) => {
                let mut parts = Vec::new();
                for la in lits {
                    let p = if n >= 2 || opt { pat(self, la.arm, "__v") } else { "__v".to_string() };
                    if la.lits.is_empty() {
                        let p = if n >= 2 || opt { pat(self, la.arm, "_") } else { "_".to_string() };
                        parts.push(format!("matches!({subject}, {p})"));
                        continue;
                    }
                    let op = if la.negate { "!=" } else { "==" };
                    let cmp: Vec<String> = la.lits.iter().map(|l| format!("__v {op} &{}", lit_code(l))).collect();
                    let joined = cmp.join(if la.negate { " && " } else { " || " });
                    parts.push(format!("(match &{subject} {{ {p} => {joined}, #[allow(unreachable_patterns)] _ => false }})"));
                }
                match parts.len() {
                    0 => "false".to_string(),
                    1 => parts.pop().unwrap(),
                    _ => format!("({})", parts.join(" || ")),
                }
            }
        }
    }

    /// [rs-opt-borrow] Own a stored union of borrows into the target union
    /// `to`, cloning each borrowed payload into its owned slot. A thin
    /// wrapper over [`Self::convert_union`] with `own = true`; `sarms`/`sopt`
    /// are the storage's value arms and whether it is `Option<…>`.
    fn own_borrowed_union(&mut self, src: &str, sarms: &[Ty], sopt: bool, to: &Ty) -> String {
        let mut from_arms: Vec<Ty> = sarms.to_vec();
        if sopt {
            from_arms.push(Ty::none());
        }
        let from = Ty::union_of(from_arms);
        self.convert_union(src, &from, to, true)
    }

    fn narrow(&mut self, local: &Local, ty: &Ty, from: &Place, from_ty: &Ty, because: &Justification, indent: usize) -> String {
        let pad = Self::pad(indent);
        let n = rs_local(&local.0);
        self.f.tys.insert(local.0.clone(), ty.clone());
        self.f.ref_handles.remove(&local.0);
        // [rs-path] Narrowing a handle: an optional one, once present, is
        // its path; any other narrowing (a claim) is the same handle.
        if from.steps.is_empty() {
            match self.handle_of(&from.root.0) {
                Some(super::paths::HandleBinding::Optional { var, prefix, pt }) if !ty.strip_quals().has_none_arm() => {
                    let p = self.fresh("h");
                    let mut steps = prefix.steps.clone();
                    steps.extend(self.steps_of_value(&pt, &p));
                    self.bind_handle(&local.0, super::paths::HandleBinding::Total(super::paths::Handle { root: prefix.root, steps }));
                    return format!("{pad}let {p} = {var}.expect(\"salvo: value is absent\");\n");
                }
                Some(b) => {
                    self.bind_handle(&local.0, b);
                    return String::new();
                }
                None => {}
            }
        }
        // [rs-opt-borrow] Storage holding a union of borrows narrows to the
        // borrow its arm holds.
        let stored = if from.steps.is_empty() { self.f.tys.get(&from.root.0).filter(|t| super::is_borrowed_union(t) && !super::is_borrowed_union(from_ty)).cloned() } else { None };
        if let Some(st) = &stored {
            let (src, _) = self.place_text(from, indent);
            let sarms: Vec<Ty> = st.value_arms().into_iter().cloned().collect();
            let sopt = st.has_none_arm();
            let still_union = ty.strip_quals().has_none_arm() || ty.strip_quals().value_arms().len() >= 2;
            if still_union {
                // [rs-opt-borrow] The narrow keeps a union (≥2 arms, or one
                // arm plus `None`). A bare alias is right only when nothing
                // has to change: every stored arm is kept (same count), the
                // `None` arm stays (or the target has none to drop), and the
                // target keeps the arms as the stored borrows. Otherwise own
                // the storage into the target union — unwrap the optional
                // when the target drops `None`, map each kept arm, clone each
                // borrowed payload into its owned slot, and send dropped arms
                // to `unreachable!`.
                let drops_none = sopt && !ty.strip_quals().has_none_arm();
                let owns_arms = !is_proj(ty) && sarms.iter().any(is_proj);
                let drops_arms = ty.strip_quals().value_arms().len() != sarms.len();
                if !drops_none && !owns_arms && !drops_arms {
                    self.f.tys.insert(local.0.clone(), st.clone());
                    self.f.kinds.insert(local.0.clone(), Kind::Owned);
                    return format!("{pad}let mut {n} = {src};\n");
                }
                let code = self.own_borrowed_union(&src, &sarms, sopt, ty);
                self.f.tys.insert(local.0.clone(), ty.clone());
                self.f.kinds.insert(local.0.clone(), Kind::Owned);
                return format!("{pad}let mut {n} = {code};\n");
            }
            if !still_union {
                let test = match because {
                    Justification::Test { test } => self.f.test_arms.get(&(test.0, usize::MAX)).cloned(),
                    Justification::Arm { switch, arm } => self.f.test_arms.get(&(switch.0, *arm)).cloned(),
                    _ => None,
                };
                let i = match test {
                    Some(ArmTest::Arm(i)) if sarms.len() >= 2 => i,
                    _ if sarms.len() >= 2 => arm_of(from_ty, ty).unwrap_or(0),
                    _ => 0,
                };
                let arm = sarms.get(i).cloned().unwrap_or(Ty::Unknown);
                let mutable = is_mut_borrow(&arm);
                let k = if is_copy_ty(&arm) { Kind::Owned } else if mutable { Kind::RefMut } else { Kind::Ref };
                self.f.kinds.insert(local.0.clone(), k);
                if sarms.len() < 2 {
                    // A mutable borrow is not `Copy`: reborrowed, so the
                    // optional stays usable after the narrowing.
                    let take = if mutable { ".as_deref_mut()" } else { "" };
                    return format!("{pad}let mut {n} = {src}{take}.unwrap();\n");
                }
                self.s.union_sizes.insert(sarms.len());
                let mut p = format!("crate::unions::Union{}::U{}(__v)", sarms.len(), i + 1);
                if sopt {
                    p = format!("Some({p})");
                }
                let code = if mutable {
                    format!("match &mut {src} {{ {p} => &mut **__v, _ => unreachable!() }}")
                } else if is_copy_ty(&arm) || !is_proj(&arm) {
                    format!("match {src} {{ {p} => __v, _ => unreachable!() }}")
                } else {
                    format!("match {src} {{ {p} => __v, _ => unreachable!() }}")
                };
                return format!("{pad}let mut {n} = {code};\n");
            }
        }
        let (src, root_kind) = self.place_text(from, indent);
        let test = match because {
            Justification::Test { test } => self.f.test_arms.get(&(test.0, usize::MAX)).cloned(),
            Justification::Arm { switch, arm } => self.f.test_arms.get(&(switch.0, *arm)).cloned(),
            _ => None,
        };
        let from_arms: Vec<Ty> = from_ty.strip_quals().value_arms().into_iter().cloned().collect();
        let nn = from_arms.len();
        let opt = from_ty.strip_quals().has_none_arm();
        if ty.is_none_ty() {
            self.f.kinds.insert(local.0.clone(), Kind::Owned);
            return format!("{pad}let mut {n} = ();\n");
        }
        // No narrowing at all (a value loop that may not assign): the
        // storage, read in place.
        if alike(ty, from_ty) {
            if is_copy_ty(ty) {
                self.f.kinds.insert(local.0.clone(), Kind::Owned);
                return format!("{pad}let mut {n} = {src};\n");
            }
            if root_kind == Kind::Owned && from.steps.is_empty() && self.f.consumed.contains(&local.0) {
                self.f.kinds.insert(local.0.clone(), Kind::Owned);
                return format!("{pad}let mut {n} = {src};\n");
            }
            self.f.kinds.insert(local.0.clone(), Kind::Ref);
            let r = if from.steps.is_empty() && matches!(root_kind, Kind::Ref | Kind::RefMut) { format!("&*{src}") } else { format!("&{src}") };
            return format!("{pad}let mut {n} = {r};\n");
        }
        let arm = match test {
            Some(ArmTest::Arm(i)) if nn >= 2 => Some(i),
            _ if nn >= 2 => arm_of(from_ty, ty),
            _ => None,
        };
        // A sub-union of the storage: rewrapped, owned.
        if nn >= 2 && arm.is_none() {
            let v = if root_kind == Kind::Owned && from.steps.is_empty() && self.f.consumed.contains(&local.0) { src.clone() } else { format!("{src}.clone()") };
            let code = self.rewrap(&v, from_ty, ty);
            self.f.kinds.insert(local.0.clone(), Kind::Owned);
            return format!("{pad}let mut {n} = {code};\n");
        }
        // Same representation: an alias of the storage.
        if nn < 2 && !opt {
            let kind = if is_copy_ty(ty) { Kind::Owned } else { root_kind };
            if is_copy_ty(ty) || kind == Kind::Owned {
                let code = if is_copy_ty(ty) { src } else { format!("&{src}") };
                self.f.kinds.insert(local.0.clone(), if is_copy_ty(ty) { Kind::Owned } else { Kind::Ref });
                return format!("{pad}let mut {n} = {code};\n");
            }
            self.f.kinds.insert(local.0.clone(), Kind::Ref);
            let r = if from.steps.is_empty() && matches!(root_kind, Kind::Ref | Kind::RefMut) { format!("&*{src}") } else { format!("&{src}") };
            return format!("{pad}let mut {n} = {r};\n");
        }
        let pat = |i: usize, inner: &str| -> String {
            let p = format!("crate::unions::Union{nn}::U{}({inner})", i + 1);
            if opt { format!("Some({p})") } else { p }
        };
        if nn >= 2 {
            self.s.union_sizes.insert(nn);
        }
        let i = arm.unwrap_or(0);
        let copy_or_ref = is_copy_ty(ty) || is_proj(ty);
        let owned_root = root_kind == Kind::Owned && !self.through_ref(from);
        let moves = !copy_or_ref && owned_root && self.f.consumed.contains(&local.0);
        // Whether the storage's arm holds a borrow itself (an `Option<&T>`),
        // or the borrow comes from reading owned storage in place.
        let arm_holds_ref = from_arms.get(i).is_some_and(|a| is_proj(a));
        let (code, kind) = if is_proj(ty) && !arm_holds_ref && !is_copy_ty(ty) {
            if is_mut_borrow(ty) {
                let code = if nn >= 2 { format!("match &mut {src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_mut().unwrap()") };
                (code, Kind::RefMut)
            } else {
                let code = if nn >= 2 { format!("match &{src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_ref().unwrap()") };
                (code, Kind::Ref)
            }
        } else if copy_or_ref {
            let k = if is_copy_ty(ty) { Kind::Owned } else if is_mut_borrow(ty) { Kind::RefMut } else { Kind::Ref };
            let code = if nn >= 2 {
                if is_proj(ty) && is_mut(ty) && owned_root {
                    format!("match {src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v"))
                } else {
                    format!("match &{src} {{ {} => *__v, _ => unreachable!() }}", pat(i, "__v"))
                }
            } else if is_proj(ty) && is_mut(ty) {
                if owned_root { format!("{src}.unwrap()") } else { format!("{src}.as_deref_mut().unwrap()") }
            } else if root_kind == Kind::Ref || root_kind == Kind::RefMut || !from.steps.is_empty() {
                format!("{src}.unwrap()")
            } else {
                format!("{src}.unwrap()")
            };
            (code, k)
        } else if moves {
            let code = if nn >= 2 { format!("match {src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.unwrap()") };
            (code, Kind::Owned)
        } else if self.f.lent_mut.contains(&local.0) {
            if root_kind == Kind::Ref {
                let r = rs_local(&from.root.0);
                self.error(format!("`{r}` is read-only here, so a `Mut` value cannot be mutated through its union arm: the arm's `Mut` is a claim about the arm, not about `{r}`, so this frame received it borrowed. Take the payload as its own parameter (`Mut List<T>`) and check the arm at the call site."));
            }
            let code = if nn >= 2 { format!("match &mut {src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_mut().unwrap()") };
            (code, Kind::RefMut)
        } else {
            let code = if nn >= 2 { format!("match &{src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_ref().unwrap()") };
            (code, Kind::Ref)
        };
        self.f.kinds.insert(local.0.clone(), kind);
        format!("{pad}let mut {n} = {code};\n")
    }

    fn for_each(&mut self, local: &Local, ty: &Ty, iterable: &Expr, body: &Block, indent: usize) -> String {
        let pad = Self::pad(indent);
        let n = rs_local(&local.0);
        self.f.tys.insert(local.0.clone(), ty.clone());
        self.f.ref_handles.remove(&local.0);
        // [rs-loc] [rs-path] Over a list in the anchor, a locator loops by
        // position: each element is the path to it.
        let over_list = matches!(iterable.ty.strip_quals(), Ty::Named { name, .. } if name == "List" || name == "Deque") || matches!(iterable.ty.strip_quals(), Ty::Array(_));
        if self.f.loc.is_some() && is_proj(ty) && over_list {
            let mut pre = Vec::new();
            if let Some(h) = self.anchor_handle(iterable, indent, &mut pre).filter(|_| pre.is_empty()) {
                let i = self.fresh("li");
                let a = self.render_path(&h.root, &h.steps, false);
                let saved = self.f.kinds.clone();
                self.f.kinds.insert(local.0.clone(), Kind::Ref);
                let mut steps = h.steps.clone();
                steps.push(super::paths::SStep::Dyn(super::paths::PStep::Index, i.clone()));
                self.f.loc_index.insert(local.0.clone(), super::paths::Handle { root: h.root.clone(), steps });
                let b = self.stmts(&body.stmts, indent + 1);
                self.f.kinds = saved;
                return format!("{pad}for {i} in 0..{a}.len() {{\n{}let {n} = &{a}[{i}];\n{b}{pad}}}\n", Self::pad(indent + 1));
            }
        }
        let it_ty = iterable.ty.strip_quals().clone();
        let consumed = matches!(&iterable.kind, ExprKind::Read { consume: true, .. }) || !matches!(iterable.kind, ExprKind::Read { .. } | ExprKind::DropMut { .. });
        let (head, kind) = match &it_ty {
            Ty::Named { name, .. } if name == "Str" => {
                let r = self.raw(iterable, indent);
                (format!("{r}.chars()"), Kind::Owned)
            }
            Ty::Named { name, .. } if name == "List" || name == "Deque" => self.vec_iter(iterable, ty, consumed, indent),
            Ty::Array(_) => self.vec_iter(iterable, ty, consumed, indent),
            Ty::Named { name, .. } => {
                // [platform-iterable] a host type's own iteration.
                match self.s.symbols.key_modules.get(name.as_str()).map(|m| (*m).clone()) {
                    Some(m) => {
                        let suffix = salvo_core::naming::each_suffix(self.s.program, &m, name);
                        let host = format!("crate::{}", crate::emit::host_mod_name(&m));
                        if is_proj(ty) && is_mut(ty) {
                            let b = self.borrow_mut(iterable, indent);
                            (format!("{host}::each_mut{suffix}({b})"), Kind::RefMut)
                        } else if is_proj(ty) {
                            let b = self.borrow(iterable, indent);
                            (format!("{host}::each{suffix}({b})"), Kind::Ref)
                        } else if consumed {
                            let v = self.value(iterable, indent);
                            (format!("{host}::into_each{suffix}({v})"), Kind::Owned)
                        } else {
                            // The host's items may be borrows or values.
                            let b = self.borrow(iterable, indent);
                            (format!("{host}::each{suffix}({b}).map(|__x| __x.clone())"), Kind::Owned)
                        }
                    }
                    None => {
                        self.error(format!("`for` over `{name}`, which has no rust iteration"));
                        ("std::iter::empty()".to_string(), Kind::Owned)
                    }
                }
            }
            other => {
                self.error(format!("`for` over `{other}`, which has no rust iteration"));
                ("std::iter::empty()".to_string(), Kind::Owned)
            }
        };
        let saved = self.f.kinds.clone();
        self.f.kinds.insert(local.0.clone(), kind);
        let b = self.stmts(&body.stmts, indent + 1);
        self.f.kinds = saved;
        format!("{pad}for mut {n} in {head} {{\n{b}{pad}}}\n")
    }

    fn vec_iter(&mut self, iterable: &Expr, elem: &Ty, consumed: bool, indent: usize) -> (String, Kind) {
        if is_proj(elem) && is_mut(elem) {
            let b = self.borrow_mut(iterable, indent);
            return (format!("({b}).iter_mut()"), Kind::RefMut);
        }
        if is_proj(elem) {
            let r = self.raw(iterable, indent);
            return (format!("{r}.iter()"), Kind::Ref);
        }
        if is_copy_ty(elem) {
            let r = self.raw(iterable, indent);
            return (format!("{r}.iter().copied()"), Kind::Owned);
        }
        if consumed {
            let v = self.value(iterable, indent);
            return (format!("{v}.into_iter()"), Kind::Owned);
        }
        let r = self.raw(iterable, indent);
        (format!("{r}.iter().cloned()"), Kind::Owned)
    }

    // ------------------------------------------------------------- calls --

    /// The arguments of a call, by the callee's parameters.
    pub(crate) fn args(&mut self, params: &[Param], args: &[Expr], fn_pos: FnPos, indent: usize) -> Vec<String> {
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let Some(p) = params.get(i) else {
                out.push(self.value(a, indent));
                continue;
            };
            out.push(self.arg(a, p, fn_pos, indent));
        }
        out
    }

    /// A call's arguments, with [rs-mut-arg-hoist]: a value read of a place
    /// a `LentMut` argument also borrows is evaluated first, into a temp.
    fn call_args(&mut self, params: &[Param], args: &[Expr], indent: usize) -> (Vec<String>, Vec<String>) {
        self.call_args_with(params, args, &[], indent)
    }

    fn call_args_with(&mut self, params: &[Param], args: &[Expr], extra: &[String], indent: usize) -> (Vec<String>, Vec<String>) {
        // [rs-elem-mut] [rs-path] Two mutable handles into one container,
        // proven apart: one split borrow, where their paths diverge.
        let lent: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(i, _)| params.get(*i).is_some_and(|p| p.mode == PassMode::LentMut))
            .filter(|(_, a)| {
                bare(a).is_some_and(|r| matches!(self.handle_of(&r.0), Some(super::paths::HandleBinding::Total(_))))
                    || self.lends_loc(a)
                    || matches!(&a.kind, ExprKind::Branch { arms, .. } if arms.len() == 1 && arms[0].1.stmts.len() == 1 && matches!(&arms[0].1.stmts[0], Stmt::Let { value, .. } if self.lends_loc(value)))
            })
            .map(|(i, _)| i)
            .collect();
        if lent.len() == 2 {
            let saved_tmp = self.f.tmp;
            let mut pre = Vec::new();
            let mut hs = Vec::new();
            for &i in &lent {
                let h = match bare(&args[i]).and_then(|r| self.handle_of(&r.0)) {
                    Some(super::paths::HandleBinding::Total(h)) => Some(h),
                    _ => self.anchor_handle(&args[i], indent, &mut pre),
                };
                hs.push(h);
            }
            if let (Some(h0), Some(h1)) = (hs[0].clone(), hs[1].clone()) {
                if h0.root == h1.root {
                    let split = self.split_pair(&h0.root, &h0.steps, &h1.steps);
                    let (p0, p1) = (self.fresh("pm"), self.fresh("pm"));
                    let mut lets = pre;
                    lets.push(format!("let ({p0}, {p1}) = {split};"));
                    let mut out = Vec::new();
                    for (i, a) in args.iter().enumerate() {
                        if i == lent[0] {
                            out.push(p0.clone());
                        } else if i == lent[1] {
                            out.push(p1.clone());
                        } else {
                            match params.get(i) {
                                Some(p) => out.push(self.arg(a, p, FnPos::Param, indent)),
                                None => out.push(self.value(a, indent)),
                            }
                        }
                    }
                    return (lets, out);
                }
            }
            self.f.tmp = saved_tmp;
        }
        let mut mut_roots: Vec<String> = params.iter().zip(args).filter(|(p, _)| p.mode == PassMode::LentMut && !matches!(p.ty.strip_quals(), Ty::Fn { .. })).filter_map(|(_, a)| read_root(a)).collect();
        mut_roots.extend(extra.iter().cloned());
        // A mutable lend nested in an argument counts too.
        for a in args {
            let mut found: Vec<String> = Vec::new();
            walk_expr(a, &mut |_| {}, &mut |e| {
                if let ExprKind::Call { target: FnRef::Decl(id), args: inner, .. } = &e.kind {
                    if let Some(f) = self.s.fn_decl(id) {
                        for (p, x) in f.params.iter().zip(inner) {
                            if p.mode == PassMode::LentMut && !matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
                                found.extend(read_root(x));
                            }
                        }
                    }
                }
            });
            if read_root(a).is_none() || !found.is_empty() {
                mut_roots.extend(found);
            }
        }
        mut_roots.sort();
        mut_roots.dedup();
        if mut_roots.is_empty() {
            return (Vec::new(), self.args(params, args, FnPos::Param, indent));
        }
        let mut lets = Vec::new();
        let mut out: Vec<String> = Vec::new();
        let mut pending: Vec<(usize, &Expr, Option<&Param>)> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let p = params.get(i);
            let lent_mut = p.is_some_and(|p| p.mode == PassMode::LentMut);
            let reads_root = !lent_mut && {
                let mut hit = false;
                walk_expr(a, &mut |_| {}, &mut |e| {
                    match &e.kind {
                        ExprKind::Read { place, .. } if mut_roots.contains(&place.root.0) => hit = true,
                        ExprKind::FnValue(FnRef::Local(l)) if mut_roots.contains(&l.0) => hit = true,
                        ExprKind::Call { target: FnRef::Local(l), .. } if mut_roots.contains(&l.0) => hit = true,
                        _ => {}
                    }
                });
                hit
            };
            let code = match p {
                Some(p) => self.arg(a, p, FnPos::Param, indent),
                None => self.value(a, indent),
            };
            let by_value = p.is_some_and(|p| p.mode == PassMode::Moved || is_copy_ty(&p.ty)) || is_copy_ty(&a.ty);
            let hoisting = reads_root && (by_value || p.is_some_and(|p| p.mode == PassMode::Lent && !is_proj(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. })));
            // An argument hoisted ahead of the call must not overtake an
            // earlier one with effects of its own: those are bound first,
            // in order [deduce-same-call].
            if hoisting {
                for (idx, b, bp) in std::mem::take(&mut pending) {
                    let by_value_b = bp.is_some_and(|q| q.mode == PassMode::Moved || is_copy_ty(&q.ty)) || is_copy_ty(&b.ty);
                    if let Some(q) = bp.filter(|q| q.mode == PassMode::Moved && super::is_borrowed_union(&q.ty)) {
                        let (l, u) = self.hoist_lend(b, &q.ty, &out[idx], indent);
                        lets.push(l);
                        out[idx] = u;
                        continue;
                    }
                    let t = self.fresh("arg");
                    if by_value_b {
                        lets.push(format!("let {t} = {};", out[idx]));
                        out[idx] = t;
                    } else {
                        let v = self.value(b, indent);
                        lets.push(format!("let mut {t} = {v};"));
                        out[idx] = if bp.is_some_and(|q| q.mode == PassMode::LentMut) { format!("&mut {t}") } else { format!("&{t}") };
                    }
                }
            }
            if reads_root && by_value && p.is_some_and(|q| q.mode == PassMode::Moved && super::is_borrowed_union(&q.ty)) {
                let q = p.unwrap();
                let (l, u) = self.hoist_lend(a, &q.ty, &code, indent);
                lets.push(l);
                out.push(u);
            } else if reads_root && by_value {
                let t = self.fresh("arg");
                lets.push(format!("let {t} = {code};"));
                out.push(t);
            } else if reads_root && p.is_some_and(|p| p.mode == PassMode::Lent && !is_proj(&p.ty) && !matches!(p.ty.strip_quals(), Ty::Fn { .. })) {
                // A read before a mutable lend of its place sees the old
                // value: an owned copy, lent. Only of what copies
                // unobservably — a scalar or immutable data: a copy of
                // mutable data would be a snapshot here and a live handle
                // on Kotlin [backend-never-wrong].
                let root = read_root(a);
                let later = args.iter().enumerate().skip(i + 1).any(|(j, b)| {
                    let mut roots: Vec<String> = Vec::new();
                    if params.get(j).is_some_and(|p| p.mode == PassMode::LentMut) {
                        roots.extend(read_root(b));
                    }
                    walk_expr(b, &mut |_| {}, &mut |e| {
                        if let ExprKind::Call { target: FnRef::Decl(id), args: inner, .. } = &e.kind {
                            if let Some(f) = self.s.fn_decl(id) {
                                for (p, x) in f.params.iter().zip(inner) {
                                    if p.mode == PassMode::LentMut {
                                        roots.extend(read_root(x));
                                    }
                                }
                            }
                        }
                    });
                    root.as_ref().is_some_and(|r| roots.contains(r))
                });
                let inner = match &a.kind { ExprKind::DropMut { value } => value.as_ref(), _ => a };
                if later && !is_copy_ty(&a.ty) && super::decls::ty_carries_mut(&inner.ty) {
                    if let Some(place) = read_place_text(inner) {
                        let at = self.location(a.span);
                        self.error(format!(
                            "cannot lower this call: `{place}` is read here while a later argument mutates it, and it is mutable data — a copy would be a snapshot on one backend and a live handle on the other, so the program has to choose: `copy({place})` for the snapshot, or give the mutating call its own statement first [rs-mut-arg-hoist] (at {at})"
                        ));
                    }
                }
                let v = self.value(a, indent);
                let t = self.fresh("arg");
                lets.push(format!("let {t} = {v};"));
                out.push(format!("&{t}"));
            } else {
                let mut effects = false;
                walk_expr(a, &mut |_| {}, &mut |e| {
                    if matches!(e.kind, ExprKind::Call { .. } | ExprKind::MemberCall { .. } | ExprKind::Send { .. } | ExprKind::SelfSend { .. }) {
                        effects = true;
                    }
                });
                if effects {
                    pending.push((out.len(), a, p));
                }
                out.push(code);
            }
        }
        (lets, out)
    }

    /// [ref-anchor] [rs-loc] A call to a fn with handle parameters: each
    /// handle argument is a position in its container, computed first (a
    /// bound handle's own position, or the mint's locator), so the container
    /// argument's `&mut` is the only borrow the call takes.
    fn anchored_args(&mut self, f: &salvo_core::ir::FnDecl, params: &[Param], args: &[Expr], indent: usize) -> (Vec<String>, Vec<String>) {
        // Call arguments align with the IR params (effects first).
        let anchored: HashMap<usize, usize> = f.ref_anchors.iter().copied().collect();
        let mut lets = Vec::new();
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            if let Some(&k) = anchored.get(&i) {
                let mut pre = Vec::new();
                let handle = match bare(a).and_then(|r| self.handle_of(&r.0)) {
                    Some(super::paths::HandleBinding::Total(h)) => Some(h),
                    _ => self.anchor_handle(a, indent, &mut pre),
                };
                let container = args.get(k).and_then(|c| self.anchor_handle(c, indent, &mut pre));
                // The callee's parameter at this call's type arguments: a
                // container that is the callee's type parameter is this
                // call's container.
                let pty = super::decls::subst_ty(&f.params[i].ty, &self.f.call_subst);
                let Some(pt) = self.handle_path_ty(&pty) else {
                    // Reported with the path type: one mistake, one diagnostic.
                    out.push("panic!()".to_string());
                    continue;
                };
                let value = match (handle, container, Some(pt)) {
                    (Some(h), Some(c), Some(pt)) => match self.strip_prefix(&h, &c) {
                        Some(rest) => self.path_value(&rest, &pt),
                        None => None,
                    },
                    _ => None,
                };
                let Some(value) = value else {
                    self.error("a `ref(c)` argument that is not a handle into the argument passed as `c` [ref-anchor] [backend-never-wrong]");
                    out.push("panic!()".to_string());
                    continue;
                };
                lets.extend(pre);
                let t = self.fresh("c");
                lets.push(format!("let {t} = {value};"));
                out.push(t);
                continue;
            }
            match params.get(i) {
                Some(p) => out.push(self.arg(a, p, FnPos::Param, indent)),
                None => out.push(self.value(a, indent)),
            }
        }
        (lets, out)
    }

    fn arg(&mut self, a: &Expr, p: &Param, fn_pos: FnPos, indent: usize) -> String {
        // [generic-union-span] A type parameter bound to a *span* of ≥2
        // union arms makes the callee's slot nested (`Union2<Union2<A,B>,
        // C>`) where the argument is flat (`Union3<A,B,C>`): the arg is
        // rewrapped flat→nested here, and the result nested→flat in `call`.
        if let Some(nested) = self.span_nested_slot(&p.ty) {
            let flat = a.ty.strip_quals().clone();
            let v = self.value(a, indent);
            return self.convert_union(&v, &flat, &nested, false);
        }
        if matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
            let pos = if p.mode == PassMode::Moved && fn_pos == FnPos::Param { FnPos::Owned } else { fn_pos };
            return self.fn_arg(a, &p.ty, pos, indent);
        }
        if is_copy_ty(&p.ty) || p.variadic || is_proj(&p.ty) {
            return self.value_into(a, &p.ty, indent);
        }
        if p.mode == PassMode::Moved && super::is_borrowed_union(&p.ty) {
            return self.lend_value(a, &p.ty, indent);
        }
        if is_copy_ty(&a.ty) && p.mode != PassMode::LentMut && p.mode != PassMode::Lent {
            return self.value(a, indent);
        }
        // [rs-proj-arm] [fn-kept-borrow] An owned argument at a kept slot the
        // call instantiates as a borrow (`key: K` at `K = proj Str`, so
        // `&&String`): the borrow of a borrow of it.
        if p.mode == PassMode::Lent && !is_proj(&p.ty) && !is_copy_ty(&a.ty) && !is_proj(&self.storage_ty(a)) && !matches!(a.kind, ExprKind::BorrowAs { .. }) {
            let inst = super::decls::subst_ty(&p.ty, &self.f.call_subst);
            if is_proj(&inst) && !is_ref(&inst) {
                let b = self.borrow(a, indent);
                return format!("&{b}");
            }
        }
        match p.mode {
            PassMode::Moved => self.value_into(a, &p.ty, indent),
            PassMode::Lent => self.borrow(a, indent),
            PassMode::LentMut => self.borrow_mut(a, indent),
        }
    }

    /// [rs-opt-borrow] What a value is stored as: a read of a local the
    /// IR types as an owned union that holds a union of borrows (a kept
    /// union parameter, or a binding of one) is the union of borrows.
    pub(crate) fn storage_ty(&self, e: &Expr) -> Ty {
        let mut inner = e;
        while let ExprKind::DropMut { value } = &inner.kind {
            inner = value;
        }
        if let ExprKind::Read { place, .. } = &inner.kind {
            if place.steps.is_empty() {
                if let Some(t) = self.f.tys.get(&place.root.0) {
                    if super::is_borrowed_union(t) && !super::is_borrowed_union(&e.ty) {
                        return t.clone();
                    }
                }
            }
        }
        e.ty.clone()
    }

    /// [rs-opt-borrow] [proj-opt-slot] An argument for a union-of-borrows
    /// slot (`Option<&T>`, `Union2<&A, &mut B>`): each arm borrowed where
    /// it is owned, passed on where it is a borrow already — the IR's
    /// `BorrowAs` says which arm lands where.
    fn lend_value(&mut self, a: &Expr, slot: &Ty, indent: usize) -> String {
        let mutable = matches!(slot, Ty::Union(arms) if arms.iter().any(|x| is_proj(x) && super::is_mut_borrow(x)));
        match &a.kind {
            ExprKind::MakeNone => return "None".to_string(),
            ExprKind::DropMut { value } => return self.lend_value(value, slot, indent),
            ExprKind::BorrowAs { value, .. } => return self.lend_value(value, slot, indent),
            ExprKind::Present { value } if slot.has_none_arm() => {
                let inner = Ty::union_of(slot.value_arms().into_iter().cloned().collect());
                let v = if matches!(inner, Ty::Union(_)) { self.lend_value(value, &inner, indent) } else { self.lend_one(value, mutable, indent) };
                return format!("Some({v})");
            }
            ExprKind::MakeUnion { value, .. } => {
                let sa: Vec<Ty> = slot.value_arms().into_iter().cloned().collect();
                if let Some(Some(j)) = salvo_core::types::read_arm_map(&value.ty, &Ty::Union(sa.clone())).map(|m| m.iter().position(|(f, _)| f.is_some())) {
                    let v = self.lend_one(value, mutable, indent);
                    self.s.union_sizes.insert(sa.len());
                    let c = format!("crate::unions::Union{}::U{}({v})", sa.len(), j + 1);
                    return if slot.has_none_arm() { format!("Some({c})") } else { c };
                }
            }
            _ => {}
        }
        let st = self.storage_ty(a);
        if !mutable && st == *slot && bare(a).is_some() {
            return self.value(a, indent);
        }
        // What the value is in Rust, and a place to match on.
        let saved = std::mem::replace(&mut self.f.place_mut, mutable);
        // A union whose every arm is a borrow (or `Copy`) is matched by
        // value: it owns nothing, so a temporary may go.
        let all_borrowed = super::is_borrowed_union(&st) && st.value_arms().into_iter().all(|x| is_proj(x) || is_copy_ty(x));
        // A binding read to mutate: its own place, reborrowed (a `&mut`
        // is not `Copy`), and an optional handle walked mutably.
        if mutable && super::is_borrowed_union(&st) {
            if let Some(r) = bare(a) {
                let place = Place { root: r.clone(), steps: Vec::new() };
                let (text, _) = self.place_text(&place, indent);
                self.f.place_mut = saved;
                if matches!(self.handle_of(&r.0), Some(super::paths::HandleBinding::Optional { .. })) {
                    return self.lend_union_by_value(&text, &st, slot, mutable);
                }
                return self.lend_union(&format!("&mut {text}"), &st, slot, mutable);
            }
        }
        if all_borrowed {
            let v = self.value(a, indent);
            self.f.place_mut = saved;
            return self.lend_union_by_value(&v, &st, slot, mutable);
        }
        let code = if super::is_borrowed_union(&st) || !matches!(a.kind, ExprKind::Read { .. }) {
            let v = self.value(a, indent);
            format!("{}({v})", if mutable { "&mut " } else { "&" })
        } else if mutable {
            self.borrow_mut(a, indent)
        } else {
            self.borrow(a, indent)
        };
        self.f.place_mut = saved;
        self.lend_union(&code, &st, slot, mutable)
    }

    /// [rs-opt-borrow] [rs-mut-arg-hoist] An argument for a union-of-borrows
    /// slot bound ahead of its call: what owns its arms is bound, and the
    /// call lends from the binding — a borrow of a temporary would not
    /// outlive the `let`. A union holding only borrows is bound as lent.
    fn hoist_lend(&mut self, a: &Expr, slot: &Ty, code: &str, indent: usize) -> (String, String) {
        let mut src = a;
        loop {
            match &src.kind {
                ExprKind::DropMut { value } | ExprKind::BorrowAs { value, .. } => src = value,
                _ => break,
            }
        }
        let st = self.storage_ty(src);
        let t = self.fresh("arg");
        let owns = !(super::is_borrowed_union(&st) && st.value_arms().into_iter().all(|x| is_proj(x) || is_copy_ty(x)));
        if !owns || matches!(src.kind, ExprKind::MakeNone) {
            return (format!("let {t} = {code};"), t);
        }
        let mutable = matches!(slot, Ty::Union(arms) if arms.iter().any(|x| is_proj(x) && super::is_mut_borrow(x)));
        let v = self.value(src, indent);
        let r = if mutable { "&mut " } else { "&" };
        let u = self.lend_union(&format!("{r}{t}"), &st, slot, mutable);
        (format!("let mut {t} = {v};"), u)
    }

    /// [rs-opt-borrow] A union of borrows `v`, by value, as the union of
    /// borrows `to`: each borrow reborrowed (`&*`), shared where `to` reads.
    fn lend_union_by_value(&mut self, v: &str, from: &Ty, to: &Ty, mutable: bool) -> String {
        let fa: Vec<Ty> = from.value_arms().into_iter().cloned().collect();
        let ta: Vec<Ty> = to.value_arms().into_iter().cloned().collect();
        let same_shape = fa.len() == ta.len() && fa.iter().zip(&ta).all(|(f, t)| is_mut_borrow(f) == is_mut_borrow(t) && is_proj(f) == is_proj(t));
        if same_shape {
            return v.to_string();
        }
        let (fo, topt) = (from.has_none_arm(), to.has_none_arm());
        let Some(map) = salvo_core::types::read_arm_map(from, to) else {
            self.error(format!("cannot lend a `{from}` as a `{to}` [rs-opt-borrow] [backend-never-wrong]"));
            return "panic!()".to_string();
        };
        let mut out = Vec::new();
        for (j, (f, _)) in map.iter().enumerate() {
            let Some(f) = f else { continue };
            let Some(i) = fa.iter().position(|x| x == f) else { continue };
            let val = if is_copy_ty(f) { "__v" } else if mutable { "&mut *__v" } else { "&*__v" };
            let wrap = |arms: &[Ty], k: usize, opt: bool, inner: &str, s: &mut Self| {
                let p = if arms.len() >= 2 {
                    s.s.union_sizes.insert(arms.len());
                    format!("crate::unions::Union{}::U{}({inner})", arms.len(), k + 1)
                } else {
                    inner.to_string()
                };
                if opt { format!("Some({p})") } else { p }
            };
            let lhs = wrap(&fa, i, fo, "__v", self);
            let rhs = wrap(&ta, j, topt, val, self);
            out.push(format!("{lhs} => {rhs}"));
        }
        if fo {
            out.push("None => None".to_string());
        }
        format!("match {v} {{ {} }}", out.join(", "))
    }

    /// One value for a borrowed arm: a borrow already is passed on, a
    /// `Copy` value by value, anything else borrowed.
    fn lend_one(&mut self, v: &Expr, mutable: bool, indent: usize) -> String {
        let st = self.storage_ty(v);
        if is_copy_ty(&st) {
            return self.value(v, indent);
        }
        if is_proj(&st) {
            let saved = std::mem::replace(&mut self.f.place_mut, mutable);
            let c = self.value(v, indent);
            self.f.place_mut = saved;
            return c;
        }
        if mutable { self.borrow_mut(v, indent) } else { self.borrow(v, indent) }
    }

    /// [rs-opt-borrow] `code` — a reference to a value stored as `from` —
    /// as the union of borrows `to`, arm by arm.
    fn lend_union(&mut self, code: &str, from: &Ty, to: &Ty, mutable: bool) -> String {
        let fa: Vec<Ty> = salvo_core::types::spread_union_quals(from).value_arms().into_iter().cloned().collect();
        let ta: Vec<Ty> = to.value_arms().into_iter().cloned().collect();
        let (fo, topt) = (from.has_none_arm(), to.has_none_arm());
        let Some(map) = salvo_core::types::read_arm_map(from, to) else {
            self.error(format!("cannot lend a `{from}` as a `{to}` [rs-opt-borrow] [backend-never-wrong]"));
            return "panic!()".to_string();
        };
        let pat = |s: &mut Self, arms: &[Ty], i: usize, opt: bool, inner: &str| -> String {
            let p = if arms.len() >= 2 {
                s.s.union_sizes.insert(arms.len());
                format!("crate::unions::Union{}::U{}({inner})", arms.len(), i + 1)
            } else {
                inner.to_string()
            };
            if opt { format!("Some({p})") } else { p }
        };
        // An owned optional of one arm: the standard library's own lend.
        let m = if mutable { "as_mut" } else { "as_ref" };
        if fa.len() == 1 && ta.len() == 1 && fo && topt && !is_proj(&fa[0]) && !is_copy_ty(&fa[0]) {
            return format!("({code}).{m}()");
        }
        // An owned union lent whole, arm for arm: the union's own lend.
        let whole = fa.len() >= 2
            && fa.len() == ta.len()
            && fa.iter().all(|f| !is_proj(f) && !is_copy_ty(f))
            && map.iter().enumerate().all(|(j, (f, _))| f.as_ref().is_some_and(|f| fa.iter().position(|x| x == f) == Some(j)))
            && ta.iter().all(|t| is_proj(t) && super::is_mut_borrow(t) == mutable)
            && (topt || !fo);
        if whole {
            self.s.union_sizes.insert(fa.len());
            return if fo {
                format!("({code}).{m}().map(|__u| __u.{m}())")
            } else if topt {
                format!("Some(({code}).{m}())")
            } else {
                format!("({code}).{m}()")
            };
        }
        let mut out = Vec::new();
        for (j, (f, _)) in map.iter().enumerate() {
            let Some(f) = f else { continue };
            let Some(i) = fa.iter().position(|x| x == f) else { continue };
            let val = if is_copy_ty(f) {
                "*__v"
            } else if is_proj(f) {
                if mutable { "&mut **__v" } else { "&**__v" }
            } else {
                "__v"
            };
            let lhs = pat(self, &fa, i, fo, "__v");
            let rhs = pat(self, &ta, j, topt, val);
            out.push(format!("{lhs} => {rhs}"));
        }
        if fo {
            out.push("None => None".to_string());
        }
        if fa.len() < 2 && !fo {
            // Not a union: the one value, lent.
            let val = out.pop().map(|a| a.split(" => ").nth(1).unwrap_or("__v").to_string()).unwrap_or_default();
            return format!("{{ let __v = {code}; {val} }}");
        }
        if fa.len() >= 2 && out.len() < fa.len() + usize::from(fo) {
            out.push("_ => unreachable!(\"salvo: unreachable union arm\")".to_string());
        }
        format!("match {code} {{ {} }}", out.join(", "))
    }

    /// A fn value passed to a fn-typed slot at `pos`.
    pub fn fn_arg(&mut self, a: &Expr, slot: &Ty, pos: FnPos, indent: usize) -> String {
        if let ExprKind::Call { target: FnRef::Decl(id), args, .. } = &a.kind {
            if args.len() == 1 && self.s.ast_fn(id).is_some_and(|f| f.intrinsic && f.name.name == "copy") && matches!(self.s.unalias(&args[0].ty).strip_quals(), Ty::Fn { .. }) {
                return self.fn_arg(&args[0], slot, pos, indent);
            }
        }
        if let ExprKind::FnValue(FnRef::Local(l)) = &a.kind {
            let read = Expr { ty: a.ty.clone(), span: a.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false, moved: false } };
            return self.fn_arg(&read, slot, pos, indent);
        }
        match &a.kind {
            ExprKind::Lambda { params, ret, body, .. } => {
                let c = self.lambda(params, ret, body, slot, pos, indent);
                match pos {
                    FnPos::Param | FnPos::DynParam => format!("&mut {c}"),
                    FnPos::Stored => format!("std::sync::Arc::new({c})"),
                    FnPos::Owned | FnPos::Local => c,
                }
            }
            ExprKind::FnValue(r) => {
                let c = self.fn_value(a, r, slot, pos, indent);
                match pos {
                    FnPos::Param | FnPos::DynParam => format!("&mut {c}"),
                    FnPos::Stored => format!("std::sync::Arc::new({c})"),
                    FnPos::Owned | FnPos::Local => c,
                }
            }
            ExprKind::Read { place, .. } => {
                let (text, kind) = self.place_text(place, indent);
                // [rs-fn-param-convention] A callback whose own conventions
                // differ from the slot's is adapted; [rs-proj-arm] so is one
                // whose slot the call retagged to a borrow. A stored callback
                // (a field, an `Arc` binding) is always called through.
                if matches!(pos, FnPos::Param | FnPos::DynParam) && !self.s.fn_ty_lends_mut(slot) {
                    let direct = place.steps.is_empty() && matches!(kind, Kind::RefMut | Kind::Owned) && !self.f.arc_fns.contains(&place.root.0);
                    let own = if place.steps.is_empty() { self.f.tys.get(&place.root.0).cloned() } else { None }.unwrap_or_else(|| a.ty.clone());
                    let callee = if direct { text.clone() } else { format!("({text})") };
                    if let Some(c) = self.fn_adapter(&own, slot, &callee, !direct) {
                        return format!("&mut {c}");
                    }
                }
                match (pos, kind) {
                    (FnPos::Param | FnPos::DynParam, Kind::RefMut) if place.steps.is_empty() => format!("&mut *{text}"),
                    (FnPos::Param | FnPos::DynParam, Kind::Owned) if place.steps.is_empty() => format!("&mut {text}"),
                    (FnPos::Param | FnPos::DynParam, _) => {
                        // A stored callback into a lending slot, called through.
                        let (ps, _) = self.closure_params(slot);
                        let names: Vec<String> = ps.iter().map(|(n, _)| n.clone()).collect();
                        let typed: Vec<String> = ps.iter().map(|(n, t)| format!("{n}: {t}")).collect();
                        format!("&mut |{}| ({text})({})", typed.join(", "), names.join(", "))
                    }
                    (FnPos::Stored, Kind::Owned) if place.steps.is_empty() && !self.f.arc_fns.contains(&place.root.0) => format!("std::sync::Arc::new({text})"),
                    (FnPos::Owned, Kind::Owned) if place.steps.is_empty() => text,
                    (_, _) => format!("{text}.clone()"),
                }
            }
            _ => {
                let v = self.value(a, indent);
                match pos {
                    // [rs-fn-field] A fn value a call answers (an `Arc<dyn Fn>`, as a fn
                    // returning one renders) is bound once and called through.
                    FnPos::Param | FnPos::DynParam if matches!(a.kind, ExprKind::Call { .. }) && !self.s.fn_ty_lends_mut(slot) => match self.fn_adapter(&a.ty, slot, "__f", true) {
                        Some(c) => format!("&mut {{ let __f = {v}; move {c} }}"),
                        None => format!("&mut {v}"),
                    },
                    FnPos::Param | FnPos::DynParam => format!("&mut {v}"),
                    _ => v,
                }
            }
        }
    }

    /// [rs-fn-param-convention] [rs-proj-arm] A closure adapting a fn value
    /// of Salvo type `own`, called as `callee`, to the fn-typed `slot`: each
    /// argument is converted from the slot's pass mode to the value's own,
    /// and an argument the call retagged to a borrow (the instantiated slot
    /// has `proj` where the value's own parameter does not) arrives one
    /// reference deeper and is peeled first (`|__a0: &&String| keep(*__a0)`).
    /// `None` when the value fits the slot as it is, unless `always` (a
    /// stored callback, which is called through in any case).
    fn fn_adapter(&mut self, own: &Ty, slot: &Ty, callee: &str, always: bool) -> Option<String> {
        let own = self.s.unalias(own);
        let (_, lm) = self.closure_params(&own);
        let (sp, sm) = self.closure_params(slot);
        let inst_slot = super::decls::subst_ty(slot, &self.f.call_subst);
        let fn_params = |t: &Ty| -> Vec<Ty> {
            match t.strip_quals() {
                Ty::Fn { params, effects, .. } => salvo_core::threaded_effects(effects).iter().chain(params.iter()).cloned().collect(),
                _ => Vec::new(),
            }
        };
        let (inst_ps, own_ps) = (fn_params(&inst_slot), fn_params(&own));
        let fits = lm.len() == sm.len();
        let retag: Vec<bool> = (0..sm.len())
            .map(|i| fits && sm[i] != PassMode::Moved && inst_ps.get(i).is_some_and(is_proj) && !own_ps.get(i).is_some_and(is_proj))
            .collect();
        let retagged = retag.iter().any(|r| *r);
        if !always && !(fits && (lm != sm || retagged)) {
            return None;
        }
        let names: Vec<String> = sp.iter().map(|(n, _)| n.clone()).collect();
        let conv: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if !fits {
                    return n.clone();
                }
                let base = if retag[i] {
                    let m = if inst_ps.get(i).is_some_and(is_mut_borrow) { "&mut *" } else { "" };
                    format!("{m}*{n}")
                } else {
                    n.clone()
                };
                match (sm[i], lm[i]) {
                    (PassMode::Moved, PassMode::Moved) | (PassMode::Lent, PassMode::Lent) | (PassMode::LentMut, PassMode::LentMut) => base,
                    (PassMode::Lent | PassMode::LentMut, PassMode::Moved) => format!("(*{base}).clone()"),
                    (PassMode::Moved, PassMode::Lent) => format!("&{base}"),
                    (PassMode::Moved, PassMode::LentMut) => format!("&mut {base}"),
                    (PassMode::LentMut, PassMode::Lent) => format!("&*{base}"),
                    (PassMode::Lent, PassMode::LentMut) => base,
                }
            })
            .collect();
        // Typed where the instantiated slot is concrete (a stored callback
        // or a retagged one says what arrives); otherwise the coercion to
        // the slot's `dyn FnMut` types the parameters.
        let concrete = !foreign_var(&inst_slot, &self.f.generics);
        let params: Vec<String> = if concrete && (always || retagged) {
            self.closure_params_at(slot, &inst_slot).0.iter().map(|(n, t)| format!("{n}: {t}")).collect()
        } else {
            names.clone()
        };
        Some(format!("|{}| {callee}({})", params.join(", "), conv.join(", ")))
    }

    /// The parameter names and Rust types a closure of fn type `t` takes.
    fn closure_params(&mut self, t: &Ty) -> (Vec<(String, String)>, Vec<PassMode>) {
        self.closure_params_at(t, t)
    }

    /// The parameters of a closure for the fn-typed slot `slot` (whose
    /// declared types decide what arrives by reference), typed at `inst`.
    /// A mode is `Moved` for a by-value argument.
    fn closure_params_at(&mut self, slot: &Ty, inst: &Ty) -> (Vec<(String, String)>, Vec<PassMode>) {
        let Ty::Fn { params: sp, effects, .. } = slot.strip_quals() else { return (Vec::new(), Vec::new()) };
        let ip: Vec<Ty> = match inst.strip_quals() {
            Ty::Fn { params, .. } if params.len() == sp.len() => params.clone(),
            _ => sp.clone(),
        };
        let mut out = Vec::new();
        let mut modes = Vec::new();
        // [throw] A throw is not threaded in [rs-throw-controlflow].
        for (i, e) in salvo_core::threaded_effects(effects).iter().enumerate() {
            let h = self.ty(e);
            out.push((format!("__e{i}"), format!("&{h}")));
            modes.push(PassMode::Lent);
        }
        let lends = self.s.fn_ty_lends_mut(slot);
        for (i, p) in sp.iter().enumerate() {
            let mut mode = fn_ty_param_mode(slot, i);
            if lends && mode == PassMode::LentMut {
                mode = PassMode::Lent;
            }
            let inner = self.ty(&ip[i]);
            let by_value = is_copy_ty(p) || is_proj(p) || matches!(p.strip_quals(), Ty::Fn { .. });
            let (rt, m) = if by_value {
                (inner, PassMode::Moved)
            } else {
                match mode {
                    PassMode::Moved => (inner, PassMode::Moved),
                    PassMode::Lent => (format!("&{inner}"), PassMode::Lent),
                    PassMode::LentMut => (format!("&mut {inner}"), PassMode::LentMut),
                }
            };
            out.push((format!("__a{i}"), rt));
            modes.push(m);
        }
        (out, modes)
    }

    fn lambda(&mut self, params: &[Param], ret: &Ty, body: &Block, slot: &Ty, pos: FnPos, indent: usize) -> String {
        let inst_slot = super::decls::subst_ty(slot, &self.f.call_subst);
        let inst_params: Vec<Ty> = match inst_slot.strip_quals() {
            Ty::Fn { params, effects, .. } => salvo_core::threaded_effects(effects).iter().chain(params.iter()).cloned().collect(),
            _ => Vec::new(),
        };
        let saved_kinds = self.f.kinds.clone();
        // [rs-path] A parameter shadows a handle of the same name.
        let saved_handles = self.f.ref_handles.clone();
        for p in params {
            self.f.ref_handles.remove(&p.local.0);
        }
        let saved_ret = self.f.ret.replace(ret.clone());
        // [rs-throw-controlflow] A closure throws only where its slot's fn
        // type says it may: it answers a `ControlFlow`, as a named fn that
        // may throw does, and a throw in it leaves it with `Break`.
        let lambda_throws = salvo_core::fn_type_throws(slot);
        let saved_throws = std::mem::replace(&mut self.f.throws, lambda_throws.clone());
        let saved_tries = std::mem::take(&mut self.f.tries);
        let saved_lt = self.f.ret_lt;
        self.f.ret_lt = false;
        let slot_effects = match slot.strip_quals() {
            Ty::Fn { effects, .. } => salvo_core::threaded_effects(effects).len(),
            _ => 0,
        };
        let inst = Ty::Fn { params: params.iter().skip(slot_effects).map(|p| p.ty.clone()).collect(), ret: Box::new(ret.clone()), contract: None, effects: Vec::new() };
        let (cps, modes) = self.closure_params_at(slot, &inst);
        let annotate = !matches!(pos, FnPos::Param | FnPos::DynParam);
        let mut ps = Vec::new();
        let mut peel = String::new();
        for (i, p) in params.iter().enumerate() {
            let n = rs_local(&p.local.0);
            let mode = modes.get(i).copied().unwrap_or(PassMode::Moved);
            let mut k = match mode {
                PassMode::Moved => Kind::Owned,
                PassMode::Lent => Kind::Ref,
                PassMode::LentMut => Kind::RefMut,
            };
            // [rs-proj-generic] A borrowed element arriving by reference is
            // peeled to the borrow itself.
            let p_proj = is_proj(&p.ty) || inst_params.get(i).is_some_and(is_proj);
            if p_proj && mode != PassMode::Moved {
                peel.push_str(&format!("{}let {n} = {}*{n};\n", Self::pad(indent + 1), if is_mut_borrow(&p.ty) { "&mut *" } else { "" }));
                k = if is_mut_borrow(&p.ty) { Kind::RefMut } else { Kind::Ref };
                self.f.tys.insert(p.local.0.clone(), inst_params.get(i).cloned().filter(is_proj).unwrap_or_else(|| p.ty.clone()));
            }
            self.f.kinds.insert(p.local.0.clone(), k);
            self.f.tys.insert(p.local.0.clone(), p.ty.clone());
            match cps.get(i) {
                Some((_, t)) if annotate && !foreign_var(&p.ty, &self.f.generics) => ps.push(format!("mut {n}: {t}")),
                _ => ps.push(format!("mut {n}")),
            }
        }
        // A kept closure owns clones of what it captures, so inside it a
        // capture of a borrowed local is an owned value: passed on as `&x`,
        // not as the reference it was outside.
        let mv = if matches!(pos, FnPos::Owned | FnPos::Stored) { "move " } else { "" };
        let mut caps: Vec<String> = Vec::new();
        if !mv.is_empty() {
            walk_block(body, &mut |_| {}, &mut |e| {
                if let ExprKind::Read { place, .. } = &e.kind {
                    caps.push(place.root.0.clone());
                }
            });
            caps.sort();
            caps.dedup();
            let own: HashSet<String> = params.iter().map(|p| p.local.0.clone()).collect();
            caps.retain(|c| !own.contains(c) && saved_kinds.contains_key(c));
            for c in &caps {
                if saved_kinds[c] == Kind::Ref {
                    self.f.kinds.insert(c.clone(), Kind::Owned);
                }
            }
        }
        // [rs-loc] A lambda in a lending slot is a locator: it answers the
        // position in its anchor parameter.
        let lends = self.s.fn_ty_lends_mut(slot);
        let saved_loc = self.f.loc.take();
        let saved_li = std::mem::take(&mut self.f.loc_index);
        let saved_pt = self.f.loc_pt.take();
        if lends {
            let anchor = params.iter().zip(&modes).find(|(p, m)| !is_copy_ty(&p.ty) && **m != PassMode::Moved).map(|(p, _)| p.local.0.clone()).unwrap_or_default();
            self.f.loc = Some((anchor, ret.strip_quals().has_none_arm()));
            self.f.loc_pt = self.handle_path_ty(ret);
        }
        let mut b = peel;
        b.push_str(&self.stmts(&body.stmts, indent + 1));
        if let Some(v) = &body.value {
            let code = if lends {
                let opt = ret.strip_quals().has_none_arm();
                self.loc_expr(v, opt, indent + 1)
            } else {
                self.value_into(v, ret, indent + 1)
            };
            if lambda_throws.is_some() && !lends {
                b.push_str(&format!("{}std::ops::ControlFlow::Continue({code})\n", Self::pad(indent + 1)));
            } else {
                b.push_str(&format!("{}{code}\n", Self::pad(indent + 1)));
            }
        } else if lambda_throws.is_some() && ret.is_none_ty() {
            b.push_str(&format!("{}std::ops::ControlFlow::Continue(())\n", Self::pad(indent + 1)));
        }
        self.f.loc = saved_loc;
        self.f.loc_index = saved_li;
        self.f.ref_handles = saved_handles;
        let lambda_pt = std::mem::replace(&mut self.f.loc_pt, saved_pt);
        self.f.kinds = saved_kinds;
        self.f.ret = saved_ret;
        self.f.ret_lt = saved_lt;
        self.f.throws = saved_throws;
        self.f.tries = saved_tries;
        let r = if lends {
            let p = lambda_pt.clone().map(|pt| self.path_rust(&pt)).unwrap_or_else(|| "usize".to_string());
            if ret.strip_quals().has_none_arm() { format!(" -> Option<{p}>") } else { format!(" -> {p}") }
        } else if lambda_throws.is_some() {
            // The slot's own type names it; inference does the rest.
            String::new()
        } else if ret.is_none_ty() || foreign_var(ret, &self.f.generics) {
            String::new()
        } else {
            format!(" -> {}", self.ty(ret))
        };
        let _ = &cps;
        let closure = format!("{mv}|{}|{r} {{\n{b}{}}}", ps.join(", "), Self::pad(indent));
        if mv.is_empty() {
            return closure;
        }
        let mut pre = String::new();
        for c in caps {
            let k = self.f.kinds[&c];
            let n = rs_local(&c);
            match k {
                Kind::SelfField => pre.push_str(&format!("let mut {n} = self.{n}.clone(); ")),
                _ => pre.push_str(&format!("let mut {n} = {n}.clone(); ")),
            }
        }
        if pre.is_empty() { closure } else { format!("{{ {pre}{closure} }}") }
    }

    /// A fn declaration used as a value: a closure forwarding to it.
    fn fn_value(&mut self, e: &Expr, r: &FnRef, slot: &Ty, pos: FnPos, indent: usize) -> String {
        let FnRef::Decl(id) = r else {
            let FnRef::Local(l) = r else { unreachable!() };
            return rs_local(&l.0);
        };
        let (ps, modes) = self.closure_params_at(slot, &e.ty);
        let Ty::Fn { params, ret, effects, .. } = e.ty.strip_quals().clone() else {
            self.error("a fn value of a non-fn type");
            return "()".to_string();
        };
        // [throw] A throw is not threaded in [rs-throw-controlflow].
        let effects = salvo_core::threaded_effects(&effects);
        let saved = self.f.kinds.clone();
        let inst_params: Vec<Ty> = match super::decls::subst_ty(slot, &self.f.call_subst).strip_quals() {
            Ty::Fn { params, effects, .. } => salvo_core::threaded_effects(effects).iter().chain(params.iter()).cloned().collect(),
            _ => Vec::new(),
        };
        let mut args = Vec::new();
        let mut peel = String::new();
        let slot_params: Vec<Ty> = match slot.strip_quals() {
            Ty::Fn { params, effects, .. } => salvo_core::threaded_effects(effects).iter().chain(params.iter()).cloned().collect(),
            _ => Vec::new(),
        };
        for (i, ((n, _), m)) in ps.iter().zip(&modes).enumerate() {
            let mut t = if i < effects.len() { effects[i].clone() } else { params[i - effects.len()].clone() };
            let mut k = match m {
                PassMode::Moved => Kind::Owned,
                PassMode::Lent => Kind::Ref,
                PassMode::LentMut => Kind::RefMut,
            };
            // [rs-proj-generic] A borrowed element arrives one reference deeper.
            let mut consume = *m == PassMode::Moved;
            if let Some(it) = inst_params.get(i).filter(|t| is_proj(t)) {
                if *m != PassMode::Moved && !is_proj(&t) {
                    peel.push_str(&format!("let {n} = *{n}; "));
                    t = it.clone();
                    k = Kind::Ref;
                } else if *m == PassMode::Moved && !is_proj(&t) && !is_copy_ty(&t) && slot_params.get(i).is_some_and(is_proj) {
                    // [fn-kept-borrow] A slot whose parameter is written
                    // `proj` passes the borrow itself (`&String`), which is
                    // what the value's kept parameter takes.
                    t = it.clone();
                    k = Kind::Ref;
                    consume = false;
                }
            }
            self.f.kinds.insert(n.clone(), k);
            self.f.tys.insert(n.clone(), t.clone());
            args.push(Expr { ty: t, span: e.span, kind: ExprKind::Read { place: Place { root: Local(n.clone()), steps: Vec::new() }, consume, moved: false } });
        }
        let want = match super::decls::subst_ty(slot, &self.f.call_subst).strip_quals() {
            Ty::Fn { ret: r, .. } if !super::body::foreign_var(r, &self.f.generics) || self.f.call_subst.is_empty() => (**r).clone(),
            _ => (*ret).clone(),
        };
        let call = Expr { ty: want, span: e.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: Vec::new(), args, moved: false } };
        // [rs-throw-controlflow] Into a slot that may throw, the forwarding
        // closure answers the callee's `ControlFlow`: a throw leaves the
        // closure (not the enclosing fn), and a value is `Continue`.
        let slot_throws = salvo_core::fn_type_throws(slot);
        let saved_throws = std::mem::replace(&mut self.f.throws, slot_throws.clone());
        let saved_tries = std::mem::take(&mut self.f.tries);
        let body = if self.s.fn_ty_lends_mut(slot) {
            // [rs-loc] The callee's locator.
            let ExprKind::Call { args, .. } = &call.kind else { unreachable!() };
            match self.s.fn_decl(id) {
                Some(f) => self.loc_call(id, f, args, indent),
                None => "0".to_string(),
            }
        } else if slot_throws.is_some() {
            let v = self.value(&call, indent);
            format!("std::ops::ControlFlow::Continue({v})")
        } else {
            self.value(&call, indent)
        };
        self.f.throws = saved_throws;
        self.f.tries = saved_tries;
        self.f.kinds = saved;
        // Annotated whenever the instantiated slot is concrete: inference
        // through `&mut *` needs the type.
        let inst_slot = super::decls::subst_ty(slot, &self.f.call_subst);
        let concrete = !foreign_var(&inst_slot, &self.f.generics);
        let annotate = !matches!(pos, FnPos::Param | FnPos::DynParam) || concrete;
        let ps: Vec<(String, String)> = if concrete && matches!(pos, FnPos::Param | FnPos::DynParam) { self.closure_params_at(slot, &inst_slot).0 } else { ps };
        let typed: Vec<String> = ps.iter().map(|(n, t)| if annotate { format!("{n}: {t}") } else { n.clone() }).collect();
        let mv = if matches!(pos, FnPos::Owned | FnPos::Stored) { "move " } else { "" };
        if peel.is_empty() {
            format!("{mv}|{}| {body}", typed.join(", "))
        } else {
            format!("{mv}|{}| {{ {peel}{body} }}", typed.join(", "))
        }
    }

    /// [rs-proj-generic] A callee's result as its definition renders it:
    /// `proj` over a plain variable is the variable (the instantiation
    /// carries any borrow), and a nested `proj` over a variable a source
    /// parameter holds stays the callee's own borrow (`Vec<&'a T>`).
    fn callee_ret(&self, f: &salvo_core::ir::FnDecl) -> Ty {
        let vars = self.s.plain_proj_vars(&f.type_params, &f.params, &f.borrows);
        let tps: HashSet<String> = f.type_params.iter().map(|t| t.name.clone()).collect();
        let held: HashSet<String> = tps.difference(&vars).cloned().collect();
        super::strip_nested_var_proj_in(&super::strip_plain_proj(&f.ret, &vars), false, &tps, &held)
    }

    /// [copy-scalar-free] A borrow of a Copy scalar is the scalar: a call
    /// whose declared result is a `proj` the instantiation made a scalar
    /// reads it out.
    ///
    /// [rs-proj-flat] And a borrow of a borrow is the borrow: a declared
    /// `proj T` (rendered `&T`) instantiated at a `T` that is itself a
    /// borrow (`proj Str`, `&String`) answers `&&String`, where Salvo's type
    /// is `proj Str` (`proj proj Str` is `proj Str`, [proj-type]); a shared
    /// reference is `Copy`, so the outer one is copied out (`*`,
    /// `.copied()`, `U1(*__v)`). `subst` is the call's instantiation.
    fn deref_scalar(&mut self, code: String, decl: &Ty, inst: &Ty, subst: &HashMap<String, Ty>) -> String {
        // A borrow the instantiation reads as a value.
        let owned = |d: &Ty, i: &Ty| is_proj(d) && !is_proj(i);
        // A borrow of a variable the instantiation made a borrow.
        let flat = |d: &Ty, i: &Ty| {
            let var = match d.strip_quals() {
                Ty::Var(v) => Some(v),
                Ty::Named { name, args } if args.is_empty() => Some(name),
                _ => None,
            };
            is_proj(d) && is_proj(i) && var.is_some_and(|v| subst.get(v).is_some_and(|t| is_proj(t) && !is_ref(t)))
        };
        if owned(decl, inst) {
            return if is_copy_ty(inst) { format!("(*{code})") } else { format!("({code}).clone()") };
        }
        if flat(decl, inst) {
            return format!("(*{code})");
        }
        let (da, ia) = (decl.strip_quals().value_arms(), inst.strip_quals().value_arms());
        if da.len() != ia.len() || !da.iter().zip(&ia).any(|(d, i)| owned(d, i) || flat(d, i)) {
            if let (Ty::Named { name: dn, args: dargs }, Ty::Named { name: inn, args: iargs }) = (decl.strip_quals(), inst.strip_quals()) {
                if dn == "List" && inn == "List" && dargs.len() == 1 && iargs.len() == 1 {
                    if owned(&dargs[0], &iargs[0]) {
                        let m = if is_copy_ty(&iargs[0]) { "copied" } else { "cloned" };
                        return format!("{code}.into_iter().{m}().collect::<Vec<_>>()");
                    }
                    if flat(&dargs[0], &iargs[0]) {
                        return format!("{code}.into_iter().copied().collect::<Vec<_>>()");
                    }
                }
            }
            return code;
        }
        let opt = decl.strip_quals().has_none_arm() && inst.strip_quals().has_none_arm();
        if da.len() == 1 {
            if flat(da[0], ia[0]) {
                return if opt { format!("{code}.copied()") } else { format!("(*{code})") };
            }
            let m = if is_copy_ty(ia[0]) { "copied" } else { "cloned" };
            return if opt { format!("{code}.{m}()") } else if is_copy_ty(ia[0]) { format!("(*{code})") } else { format!("({code}).clone()") };
        }
        let n = da.len();
        let mut arms = Vec::new();
        for (i, (d, a)) in da.iter().zip(&ia).enumerate() {
            let p = format!("crate::unions::Union{n}::U{}(__v)", i + 1);
            let v = if flat(d, a) { "*__v" } else if is_proj(d) && !is_proj(a) { if is_copy_ty(a) { "*__v" } else { "__v.clone()" } } else { "__v" };
            let r = format!("crate::unions::Union{n}::U{}({v})", i + 1);
            if opt {
                arms.push(format!("Some({p}) => Some({r})"));
            } else {
                arms.push(format!("{p} => {r}"));
            }
        }
        if opt {
            arms.push("None => None".to_string());
        }
        format!("(match {code} {{ {} }})", arms.join(", "))
    }

    /// [rs-throw-controlflow] A thrown message of type `mt`: to the
    /// innermost `try`, or out of the fn.
    fn propagate(&mut self, m: &str, mt: &Ty) -> String {
        if let Some((label, outcome)) = self.f.tries.last().cloned() {
            let arms: Vec<Ty> = outcome.strip_quals().value_arms().into_iter().cloned().collect();
            let msg = arms.get(1).cloned().unwrap_or(Ty::Unknown);
            let conv = self.into_union(m, mt, &msg);
            return format!("break {label} crate::unions::Union{}::U2({conv})", arms.len().max(2));
        }
        match self.f.throws.clone() {
            Some(t) => {
                let conv = self.into_union(m, mt, &t);
                format!("return std::ops::ControlFlow::Break({conv})")
            }
            None => {
                self.error("a throw outside a `try` in a fn that does not throw");
                "unreachable!()".to_string()
            }
        }
    }

    /// A value of type `from` placed into `to`: an arm of a union, or
    /// itself.
    fn into_union(&mut self, code: &str, from: &Ty, to: &Ty) -> String {
        let ta: Vec<Ty> = to.strip_quals().value_arms().into_iter().cloned().collect();
        if ta.len() < 2 || alike(from, to) {
            return code.to_string();
        }
        if from.strip_quals().value_arms().len() >= 2 {
            return self.rewrap(code, from, to);
        }
        match ta.iter().position(|a| alike(a, from)) {
            Some(i) => {
                self.s.union_sizes.insert(ta.len());
                format!("crate::unions::Union{}::U{}({code})", ta.len(), i + 1)
            }
            None => code.to_string(),
        }
    }

    /// [rs-loc] [rs-path] A returned handle, as the path this locator
    /// answers: a path in its anchor parameter, of the locator's path type.
    fn loc_expr(&mut self, e: &Expr, opt: bool, indent: usize) -> String {
        let wrap = |code: String, is_opt: bool| -> String {
            match (opt, is_opt) {
                (true, false) => format!("Some({code})"),
                (false, true) => format!("{code}.expect(\"salvo: value is absent\")"),
                _ => code,
            }
        };
        let Some(pt) = self.f.loc_pt.clone() else {
            self.error("internal: a locator with no path type [rs-path]");
            return "None".to_string();
        };
        let anchor_root = match self.f.loc.clone() {
            Some((a, _)) => self.place_text(&Place { root: Local(a), steps: Vec::new() }, indent).0,
            None => String::new(),
        };
        // A handle of the anchor, as this locator's path value.
        let value_of = |s: &mut Self, h: &super::paths::Handle| -> Option<String> {
            if h.root != anchor_root {
                s.error("a locator answers a handle into something other than its anchor [rs-loc] [backend-never-wrong]");
                return None;
            }
            s.path_value(&h.steps, &pt)
        };
        match &e.kind {
            ExprKind::MakeNone => "None".to_string(),
            ExprKind::Present { value } => {
                let c = self.loc_expr(value, false, indent);
                wrap(c, false)
            }
            ExprKind::Read { place, .. } if place.steps.is_empty() && self.f.loc_index.contains_key(&place.root.0) => {
                let h = self.f.loc_index[&place.root.0].clone();
                let v = value_of(self, &h).unwrap_or_else(|| "0".to_string());
                wrap(v, false)
            }
            ExprKind::Read { place, .. } if place.steps.is_empty() && self.handle_of(&place.root.0).is_some() => match self.handle_of(&place.root.0).unwrap() {
                super::paths::HandleBinding::Total(h) => {
                    let v = value_of(self, &h).unwrap_or_else(|| "0".to_string());
                    wrap(v, false)
                }
                super::paths::HandleBinding::Optional { var, prefix, pt: inner } => {
                    let x = self.fresh("x");
                    let mut steps = prefix.steps.clone();
                    steps.extend(self.steps_of_value(&inner, &x));
                    let h = super::paths::Handle { root: prefix.root, steps };
                    let v = value_of(self, &h).unwrap_or_else(|| "0".to_string());
                    wrap(format!("{var}.map(|{x}| {v})"), true)
                }
            },
            _ => {
                let Some(m) = self.mint(e, indent) else {
                    self.error("a locator returns something other than a handle of its anchor [rs-loc]");
                    return "None".to_string();
                };
                let q = self.fresh("q");
                let mut steps = m.prefix.steps.clone();
                steps.extend(self.steps_of_value(&m.pt, &q));
                let h = super::paths::Handle { root: m.prefix.root.clone(), steps };
                let v = value_of(self, &h).unwrap_or_else(|| "0".to_string());
                // The callee's path is already this one: forward it as is.
                if v == q && m.pre.is_empty() {
                    return match (m.opt, opt) {
                        (true, false) => format!("{}.expect(\"salvo: value is absent\")", m.code),
                        (false, true) => format!("Some({})", m.code),
                        _ => m.code,
                    };
                }
                let pre = if m.pre.is_empty() { String::new() } else { format!("{} ", m.pre.join(" ")) };
                match (m.opt, opt) {
                    (true, true) => format!("{{ {pre}let {q} = {}?; Some({v}) }}", m.code),
                    (true, false) => format!("{{ {pre}let {q} = {}.expect(\"salvo: value is absent\"); {v} }}", m.code),
                    (false, true) => format!("{{ {pre}let {q} = {}; Some({v}) }}", m.code),
                    (false, false) => format!("{{ {pre}let {q} = {}; {v} }}", m.code),
                }
            }
        }
    }

    /// [rs-path] Whether `e` is a call to a mint, which renders as a
    /// position (its path fn).
    pub(crate) fn lends_loc(&self, e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::Call { target: FnRef::Decl(id), .. } if self.s.fn_decl(id).is_some_and(|f| self.s.is_mint(f)))
    }

    /// A mint's call as its path fn: the anchor argument read.
    pub(crate) fn loc_call(&mut self, id: &salvo_core::ir::DeclId, f: &salvo_core::ir::FnDecl, args: &[Expr], indent: usize) -> String {
        let mut ps = self.s.unalias_params(&f.params);
        if let Some(a) = self.s.lend_param(f) {
            ps[a].mode = PassMode::Lent;
        }
        let a = self.args(&ps, args, FnPos::Param, indent);
        // [rs-path] A mint is its path fn, under its own name.
        format!("{}({})", self.fn_path(id), a.join(", "))
    }

    pub(crate) fn call(&mut self, e: &Expr, target: &FnRef, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        // [rs-path] A mint used as a value — a path fn: the path, walked once.
        if matches!(target, FnRef::Decl(id) if self.s.fn_decl(id).is_some_and(|f| self.s.is_mint(f))) {
            if let Some(m) = self.mint(e, indent) {
                return self.materialize(m);
            }
        }
        match target {
            FnRef::Local(l) => {
                let ty = self.s.unalias(&self.f.tys.get(&l.0).cloned().unwrap_or(Ty::Unknown));
                let lends = self.s.fn_ty_lends_mut(&ty);
                let (_, mut modes) = self.closure_params(&ty);
                if lends {
                    for m in modes.iter_mut() {
                        if *m == PassMode::LentMut {
                            *m = PassMode::Lent;
                        }
                    }
                }
                let ptys = match ty.strip_quals() {
                    Ty::Fn { params, effects, .. } => {
                        let mut v = effects.clone();
                        v.extend(params.iter().cloned());
                        v
                    }
                    _ => Vec::new(),
                };
                let ps: Vec<Param> = args
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        let pt = ptys.get(i).cloned().unwrap_or_else(|| x.ty.clone());
                        let mode = modes.get(i).copied().unwrap_or(PassMode::Lent);
                        // A by-value slot of a non-copy type moves.
                        Param { local: Local(String::new()), ty: pt, mode, variadic: false, check: None, span: x.span }
                    })
                    .collect();
                let (lets, a) = self.call_args_with(&ps, args, &[l.0.clone()], indent);
                let kind = self.f.kinds.get(&l.0).copied().unwrap_or(Kind::Owned);
                let callee = match kind {
                    Kind::SelfField => format!("(self.{})", rs_local(&l.0)),
                    _ => rs_local(&l.0),
                };
                let code = if lets.is_empty() {
                    format!("{callee}({})", a.join(", "))
                } else {
                    format!("{{ {} {callee}({}) }}", lets.join(" "), a.join(", "))
                };
                // [rs-throw-controlflow] A fn value that may throw answers a
                // `ControlFlow`: a throw goes to the innermost `try`, or out
                // of the fn, as a named callee's does.
                let code = match salvo_core::fn_type_throws(&ty) {
                    Some(mt) => {
                        let prop = self.propagate("__m", &mt);
                        format!("(match {code} {{ std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => {prop} }})")
                    }
                    None => code,
                };
                if !lends || self.f.want_path {
                    return code;
                }
                // [rs-loc] [rs-path] A locator callback: the element at the
                // path it answers.
                let mut pre = Vec::new();
                let anchor = ps.iter().zip(args).find(|(p, _)| !is_copy_ty(&p.ty)).and_then(|(_, a)| self.anchor_handle(a, indent, &mut pre));
                let (Some(anchor), Some(pt)) = (anchor, self.handle_path_ty(&e.ty)) else {
                    self.error("a lending callback called on something that is not a place [rs-loc]");
                    return code;
                };
                let m = super::paths::Mint { pre, prefix: anchor, pt, code, opt: e.ty.strip_quals().has_none_arm() };
                self.materialize(m)
            }
            FnRef::Decl(id) => {
                if let Some(af) = self.s.ast_fn(id) {
                    if af.intrinsic {
                        return self.intrinsic_call(e, id, af, type_args, args, indent);
                    }
                }
                let Some(f) = self.s.fn_decl(id) else {
                    self.error(format!("a call to `{}`, which has no declaration in the IR", self.s.ir.ref_name(id)));
                    return "()".to_string();
                };
                let mut ps = self.s.unalias_params(&f.params);
                // [rs-fn-field] A callee keeping its callbacks takes them owned.
                if self.stores_callbacks_only(f) {
                    for p in ps.iter_mut() {
                        if matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
                            p.mode = PassMode::Moved;
                        }
                    }
                }
                let saved_subst = std::mem::take(&mut self.f.call_subst);
                if type_args.len() == f.type_params.len() {
                    self.f.call_subst = f.type_params.iter().map(|t| t.name.clone()).zip(type_args.iter().cloned()).collect();
                }
                // [generic-union-span] If the return type nests under the
                // span binding, the result comes back nested and is
                // flattened to `e.ty` after the call. Captured while the
                // substitution is live.
                let span_ret = self.span_nested_slot(&f.ret);
                let (lets, mut a) = if f.ref_anchors.is_empty() { self.call_args(&ps, args, indent) } else { self.anchored_args(f, &ps, args, indent) };
                // [runtime-kept-fn] A kept callback: boxed and owned, or a
                // plain fn.
                for i in 0..args.len() {
                    if let Some((boxed, _)) = self.s.kept_param(f, i) {
                        let c = self.fn_arg(&args[i], &ps[i].ty, if boxed { FnPos::Owned } else { FnPos::Local }, indent);
                        a[i] = if boxed { format!("std::boxed::Box::new({c})") } else { c };
                    }
                }
                self.f.call_subst = saved_subst;
                let path = self.fn_path(id);
                let tps = f.type_params.len();
                let erased = self.s.ast_fn(id).is_some_and(|af| {
                    let fi = self.s.file_of(id).unwrap_or(0);
                    self.s.erased.fns.contains(&(fi, af.name.span))
                });
                let impl_params = self.stores_callbacks_pub(f);
                let ta = if !erased && tps > 0 && type_args.len() == tps && type_args.iter().all(|t| !t.is_unknown() && !super::body::foreign_var(t, &self.f.generics) && (f.kind == FnKind::Plain || !self.s.holds_proj(t))) && !impl_params {
                    let mut ts: Vec<String> = type_args.iter().map(|t| self.ty(t)).collect();
                    // [rs-path] The path type parameters are inferred.
                    ts.extend(std::iter::repeat_n("_".to_string(), Self::generic_path_pairs(f).len()));
                    // A borrowed argument (to a Salvo fn: a host's answers
                    // owned) names no lifetime of its own here (`&Fighter`,
                    // `ListNext<'_, Fighter>`), or none is said.
                    if ts.iter().all(|t| !t.replace("'_", "").contains('\'')) {
                        format!("::<{}>", ts.join(", "))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let code = if lets.is_empty() {
                    format!("{path}{ta}({})", a.join(", "))
                } else {
                    format!("{{ {} {path}{ta}({}) }}", lets.join(" "), a.join(", "))
                };
                let code = match f.throws.clone() {
                    Some(mt) => {
                        let prop = self.propagate("__m", &mt);
                        format!("(match {code} {{ std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => {prop} }})")
                    }
                    None => code,
                };
                let ret = self.callee_ret(f);
                // [generic-union-span] Flatten a nested span result back to
                // the call's own (flat) type before anything else uses it.
                let code = match span_ret {
                    Some(nested) => self.convert_union(&code, &nested, &e.ty, false),
                    None => code,
                };
                let subst = call_subst_of(f, type_args, args);
                self.deref_scalar(code, &ret, &e.ty, &subst)
            }
        }
    }

    fn intrinsic_call(&mut self, e: &Expr, id: &salvo_core::ir::DeclId, af: &salvo_syntax::ast::FnDecl, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        let name = af.name.name.as_str();
        let recv = af.params.first().and_then(|p| salvo_backend::emit_util::type_base_name(&p.ty));
        let _ = type_args;
        if name == "copy" {
            // [rs-loop-temp] `copy` of a block-valued expression — an `?:`,
            // an `if` value — is applied to each arm's value inside it: the
            // value may borrow a temporary the block binds
            // (`copy(get(make().items, 0) ?: …)` binds `make().items` there),
            // which dies at the block's end, before a clone outside could
            // reach it (E0597, ROADMAP §2d item 9). Not for a union: an arm's
            // value is then wrapped into it, and a copy of the wrap alone
            // leaves rustc nothing to name the union's other arms by.
            if matches!(args[0].kind, ExprKind::Branch { .. } | ExprKind::Switch { .. }) && !matches!(args[0].ty, Ty::Union(_)) {
                let wrap = |v: Expr| Expr { ty: e.ty.clone(), span: v.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: type_args.to_vec(), args: vec![v], moved: false } };
                if let Some(pushed) = copy_into_tails(&args[0], &wrap, &e.ty) {
                    return self.raw(&pushed, indent);
                }
            }
            let v = self.raw(&args[0], indent);
            if is_copy_ty(&args[0].ty) {
                return v;
            }
            // [proj-opt-slot] A union of borrows copies into an owned one,
            // arm by arm.
            let st = self.storage_ty(&args[0]);
            if super::is_borrowed_union(&st) {
                let arms: Vec<Ty> = st.value_arms().into_iter().cloned().collect();
                if arms.len() < 2 {
                    let mutable = arms.first().is_some_and(super::is_mut_borrow);
                    return if mutable { format!("({v}).as_deref().cloned()") } else { format!("({v}).cloned()") };
                }
                let opt = st.has_none_arm();
                self.s.union_sizes.insert(arms.len());
                let mut out = Vec::new();
                for (i, a) in arms.iter().enumerate() {
                    let p = format!("crate::unions::Union{}::U{}(__v)", arms.len(), i + 1);
                    let val = if is_proj(a) { "(**__v).clone()" } else { "(*__v).clone()" };
                    let r = format!("crate::unions::Union{}::U{}({val})", arms.len(), i + 1);
                    out.push(if opt { format!("Some({p}) => Some({r})") } else { format!("{p} => {r}") });
                }
                if opt {
                    out.push("None => None".to_string());
                }
                return format!("match &({v}) {{ {} }}", out.join(", "));
            }
            return format!("({v}).clone()");
        }
        // [ref-notsame] Same storage: the same address. Both sides borrowed,
        // so a handle and a parameter reference compare alike.
        if name == "same" && args.len() == 2 {
            let a = self.borrow(&args[0], indent);
            let b = self.borrow(&args[1], indent);
            return format!("std::ptr::eq({a}, {b})");
        }
        if name == "discard" {
            let v = self.value(&args[0], indent);
            return if is_copy_ty(&args[0].ty) { format!("{{ let _ = {v}; }}") } else { format!("std::mem::drop({v})") };
        }
        if let Some(code) = self.actor_intrinsic(e, name, recv, type_args, args, indent) {
            return code;
        }
        if name == "to_str" && matches!(recv, Some("Double" | "Float")) {
            self.s.needs_str = true;
        }
        let params: Vec<Param> = self.s.fn_decl(id).map(|f| f.params.clone()).unwrap_or_default();
        let variadic = af.params.iter().any(|p| p.variadic);
        let mut a: Vec<String> = Vec::new();
        let mut spread = Spread::None;
        for (i, x) in args.iter().enumerate() {
            let p = params.get(i);
            if variadic && p.is_some_and(|p| p.variadic) {
                match &x.kind {
                    ExprKind::Array(es) if !es.iter().any(|y| matches!(y.kind, ExprKind::Spread { .. })) => {
                        for el in es {
                            a.push(self.value(el, indent));
                        }
                    }
                    ExprKind::Array(es) if es.len() == 1 => {
                        if let ExprKind::Spread { value } = &es[0].kind {
                            a.push(self.raw(value, indent));
                            spread = Spread::Borrowed;
                        }
                    }
                    _ => {
                        a.push(self.value(x, indent));
                        spread = Spread::Owned;
                    }
                }
                continue;
            }
            let code = match p.map(|p| p.mode) {
                Some(PassMode::Moved) | None => self.value(x, indent),
                _ => self.raw(x, indent),
            };
            a.push(code);
        }
        match crate::intrinsics::fn_call(name, recv, &a, spread, false) {
            Some(code) => match self.s.fn_decl(id) {
                Some(f) => {
                    let ret = self.callee_ret(&f);
                    let subst = call_subst_of(&f, type_args, args);
                    self.deref_scalar(code, &ret, &e.ty, &subst)
                }
                None => code,
            },
            None => {
                self.error(format!("intrinsic fn `{name}` has no rust lowering"));
                let _ = e;
                "()".to_string()
            }
        }
    }
}

fn sym(op: Op) -> &'static str {
    match op {
        Op::Lt => "<",
        Op::Gt => ">",
        Op::LtEq => "<=",
        Op::GtEq => ">=",
        Op::Eq => "==",
        Op::NotEq => "!=",
        _ => unreachable!("not a comparison"),
    }
}

fn lit_code(l: &Lit) -> String {
    match l {
        Lit::Str(s) => format!("\"{}\"", escape_string(s)),
        Lit::Int(i) => format!("{i}i32"),
        Lit::Long(i) => format!("{i}i64"),
        Lit::Bool(b) => b.to_string(),
    }
}

/// [rs-proj-flat] A call's instantiation: the callee's type parameters to
/// the call's type arguments, or, for a call the IR states none for (a
/// `for` loop's `next`, a forwarding closure's call), to what the
/// arguments' types bind them to, as rustc infers them: `Mut ListNext<T>`
/// at a `Mut ListNext<proj Str>` binds `T` to `proj Str`, and `proj T` at
/// a `proj Str` binds it to `Str` (the parameter's own qualifiers are not
/// the variable's).
fn call_subst_of(f: &salvo_core::ir::FnDecl, type_args: &[Ty], args: &[Expr]) -> HashMap<String, Ty> {
    if !type_args.is_empty() {
        if type_args.len() != f.type_params.len() {
            return HashMap::new();
        }
        return f.type_params.iter().map(|t| t.name.clone()).zip(type_args.iter().cloned()).collect();
    }
    let tps: HashSet<String> = f.type_params.iter().map(|t| t.name.clone()).collect();
    let mut map = HashMap::new();
    if !tps.is_empty() && args.len() == f.params.len() {
        for (p, a) in f.params.iter().zip(args) {
            bind_type_vars(&p.ty, &a.ty, &tps, &mut map);
        }
    }
    map
}

/// The variables of `tps` in `pat` bound to the parts of `conc` they stand
/// for, qualifiers included except the ones `pat` itself writes there.
fn bind_type_vars(pat: &Ty, conc: &Ty, tps: &HashSet<String>, map: &mut HashMap<String, Ty>) {
    if conc.is_unknown() {
        return;
    }
    let var = match pat.strip_quals() {
        Ty::Var(v) => Some(v),
        Ty::Named { name, args } if args.is_empty() && tps.contains(name) => Some(name),
        _ => None,
    };
    if let Some(v) = var.filter(|v| tps.contains(*v)) {
        let own: HashSet<String> = pat.quals().iter().map(|q| q.name.clone()).collect();
        map.entry(v.clone()).or_insert_with(|| conc.clone().remove_quals(&own));
        return;
    }
    match (pat.strip_quals(), conc.strip_quals()) {
        (Ty::Named { name: pn, args: pa }, Ty::Named { name: cn, args: ca }) if pn == cn && pa.len() == ca.len() => {
            for (p, c) in pa.iter().zip(ca) {
                bind_type_vars(p, c, tps, map);
            }
        }
        (Ty::Union(ps), Ty::Union(cs)) | (Ty::Tuple(ps), Ty::Tuple(cs)) if ps.len() == cs.len() => {
            for (p, c) in ps.iter().zip(cs) {
                bind_type_vars(p, c, tps, map);
            }
        }
        (Ty::Array(p), Ty::Array(c)) => bind_type_vars(p, c, tps, map),
        _ => {}
    }
}
