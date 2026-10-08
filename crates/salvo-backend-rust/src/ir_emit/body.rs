//! Statements, expressions and ownership for the Rust IR emitter.
//!
//! The ownership rule [rs-ir]: a local is `Owned`, or holds a reference
//! (`Ref`, `RefMut`: a lent parameter, a `proj` value, a narrowing that
//! reads in place), or is a field of the handler (`SelfField`). A read
//! that needs its own value moves it when the IR marks it consumed and the
//! storage is owned, and clones it otherwise; a lent argument borrows.

use std::collections::{HashMap, HashSet};

use salvo_core::types::Ty;
use salvo_ir::{ArmTest, Block, Expr, ExprKind, FnKind, FnRef, Justification, Lit, Local, Op, Param, PassMode, Place, Stmt, Step};

use super::decls::{fn_ty_param_mode, FnPos};
use super::{is_copy_ty, is_mut, is_proj, ModuleEmitter};
use crate::emit::{escape_char, escape_format_text, escape_string, rs_ident};
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
    /// [rs-loc] Loop locals bound to a position of the anchor.
    pub loc_index: HashMap<String, String>,
    /// The type parameters in scope.
    pub generics: HashSet<String>,
    /// [rs-elem-mut] Elem locals: the anchor (or place) text, and the
    /// position variable when it is an element.
    pub elem: HashMap<String, (String, Option<String>)>,
    /// [rs-proj-generic] Those whose `proj` renders as the variable.
    pub plain_vars: HashSet<String>,
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
            if is_mut(&p.ty) { Kind::RefMut } else { Kind::Ref }
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

pub(crate) fn walk_block(b: &Block, fs: &mut impl FnMut(&Stmt), fe: &mut impl FnMut(&Expr)) {
    for s in &b.stmts {
        walk_stmt(s, fs, fe);
    }
    if let Some(v) = &b.value {
        walk_expr(v, fs, fe);
    }
}

fn walk_stmt(s: &Stmt, fs: &mut impl FnMut(&Stmt), fe: &mut impl FnMut(&Expr)) {
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
        Stmt::Expr(e) | Stmt::Return(Some(e)) => walk_expr(e, fs, fe),
        _ => {}
    }
}

fn walk_place(p: &Place, fs: &mut impl FnMut(&Stmt), fe: &mut impl FnMut(&Expr)) {
    for s in &p.steps {
        if let Step::Index(e) = s {
            walk_expr(e, fs, fe);
        }
    }
}

pub(crate) fn walk_expr(e: &Expr, fs: &mut impl FnMut(&Stmt), fe: &mut impl FnMut(&Expr)) {
    fe(e);
    let mut sub = |x: &Expr| walk_expr(x, fs, fe);
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
        ExprKind::Construct { fields } => fields.iter().for_each(|(_, v)| sub(v)),
        ExprKind::MakeUnion { value, .. } | ExprKind::Rewrap { value, .. } | ExprKind::DropMut { value } | ExprKind::Widen { value } | ExprKind::Spread { value } => sub(value),
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
pub(crate) fn diverges(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Throw { .. } | ExprKind::Unreachable { .. } => true,
        ExprKind::Branch { arms, otherwise, .. } => {
            let blk = |b: &Block| b.value.as_ref().map(|v| diverges(v)).unwrap_or_else(|| matches!(b.stmts.last(), Some(Stmt::Return(_) | Stmt::Break | Stmt::Continue)) || matches!(b.stmts.last(), Some(Stmt::Expr(x)) if diverges(x)));
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

fn bare(e: &Expr) -> Option<&Local> {
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

    fn fresh(&mut self, base: &str) -> String {
        self.f.tmp += 1;
        format!("__{base}{}", self.f.tmp)
    }

    /// `module:line:col` of a span in this module's source.
    pub fn location(&self, span: salvo_syntax::Span) -> String {
        match self.s.program.files.iter().find(|f| f.module == self.module.path) {
            Some(file) => {
                let (line, col) = salvo_syntax::span::line_col(&file.content, span.start);
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
            ExprKind::Read { place, consume: true } => {
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

    pub fn member_decl(&self, m: &salvo_ir::MemberRef) -> Option<&'p salvo_ir::Member> {
        match self.s.decls.get(&m.interface) {
            Some(salvo_ir::Decl::Interface(i)) => i.members.get(m.index),
            _ => None,
        }
    }

    /// A fn body at `indent`.
    pub fn fn_body(&mut self, b: &Block, indent: usize) -> String {
        self.scan(b);
        let mut out = self.stmts(&b.stmts, indent);
        if let (Some(v), Some((_, opt))) = (&b.value, self.f.loc.clone()) {
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
            out.push_str(&self.stmt(s, indent));
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
            Stmt::Return(None) if self.f.throws.is_some() => format!("{pad}return std::ops::ControlFlow::Continue(());\n"),
            Stmt::Return(Some(e)) if self.f.throws.is_some() => {
                let ret = self.f.ret.clone().unwrap_or(Ty::Unknown);
                if ret.is_none_ty() {
                    let code = self.expr_stmt(e, indent);
                    return format!("{pad}{code};\n{pad}return std::ops::ControlFlow::Continue(());\n");
                }
                let code = self.value_into(e, &ret, indent);
                format!("{pad}return std::ops::ControlFlow::Continue({code});\n")
            }
            Stmt::Return(None) => format!("{pad}return;\n"),
            Stmt::Return(Some(e)) if self.f.loc.is_some() => {
                let opt = self.f.loc.as_ref().is_some_and(|(_, o)| *o);
                let code = self.loc_expr(e, opt, indent);
                format!("{pad}return {code};\n")
            }
            Stmt::Return(Some(e)) => {
                let ret = self.f.ret.clone().unwrap_or(Ty::Unknown);
                if ret.is_none_ty() || matches!(ret, Ty::Never) {
                    if matches!(e.kind, ExprKind::Unit | ExprKind::MakeNone) {
                        return format!("{pad}return;\n");
                    }
                    let code = self.expr_stmt(e, indent);
                    return format!("{pad}{code};\n{pad}return;\n");
                }
                let code = self.value_into(e, &ret, indent);
                format!("{pad}return {code};\n")
            }
            Stmt::Break => format!("{pad}break;\n"),
            Stmt::Continue => format!("{pad}continue;\n"),
        }
    }

    /// An expression in statement position (its value dropped).
    fn expr_stmt(&mut self, e: &Expr, indent: usize) -> String {
        match &e.kind {
            ExprKind::Branch { .. } | ExprKind::Switch { .. } => self.value(e, indent),
            _ => self.value(e, indent),
        }
    }

    fn let_stmt(&mut self, local: &Local, ty: &Ty, value: &Expr, indent: usize) -> String {
        let pad = Self::pad(indent);
        let n = rs_local(&local.0);
        self.f.tys.insert(local.0.clone(), ty.clone());
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
        if is_proj(ty) && is_mut(ty) {
            if let Some((anchor, pos)) = self.loc_of(value, indent) {
                let p = self.fresh("h");
                self.f.kinds.insert(local.0.clone(), Kind::Elem);
                self.f.elem.insert(local.0.clone(), (anchor, Some(p.clone())));
                return format!("{pad}let {p}: usize = {pos};\n");
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
        let kind = if is_proj(ty) { if is_mut(ty) { Kind::RefMut } else { Kind::Ref } } else { Kind::Owned };
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
        let annot = if self.annotatable(ty) && !self.nested_proj(ty) { format!(": {}", self.ty(ty)) } else { String::new() };
        format!("{pad}let mut {n}{annot} = {code};\n")
    }

    fn borrow_local(&mut self, local: &Local, ty: &Ty, value: &Expr, indent: usize) -> Option<String> {
        let ExprKind::Read { place, consume } = &value.kind else { return None };
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
        let ExprKind::Call { target: FnRef::Decl(id), type_args, args } = &e.kind else { return (String::new(), e.clone()) };
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
                new_args.push(Expr { ty: a.ty.clone(), span: a.span, kind: ExprKind::Read { place: Place { root: Local(t), steps: Vec::new() }, consume: false } });
            } else {
                new_args.push(a.clone());
            }
        }
        (pre, Expr { ty: e.ty.clone(), span: e.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: type_args.clone(), args: new_args } })
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
    fn place_text(&mut self, p: &Place, indent: usize) -> (String, Kind) {
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
        let (text, kind) = self.place_text(p, indent);
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
            let (text, kind) = self.place_text(place, indent);
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
            ExprKind::Read { place, consume } => {
                let (text, kind) = self.place_text(place, indent);
                let bare = place.steps.is_empty();
                if is_copy_ty(&e.ty) {
                    // A scalar stored as a borrow (a `proj T` field at `T = Int`).
                    let stored_ref = !bare && self.place_ty_raw(place).is_some_and(|t| is_proj(&t));
                    return if (bare && matches!(kind, Kind::Ref | Kind::RefMut)) || stored_ref { format!("*{text}") } else { text };
                }
                if is_proj(&e.ty) {
                    if kind == Kind::Elem {
                        return if is_mut(&e.ty) { format!("&mut {text}") } else { format!("&{text}") };
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
            ExprKind::Call { target, type_args, args } => self.call(e, target, type_args, args, indent),
            ExprKind::MemberCall { instance, member, args, .. } => {
                let inst = self.raw(instance, indent);
                let Some(m) = self.member_decl(member) else {
                    self.error("a member call to an unknown member");
                    return "()".to_string();
                };
                let ps = self.s.unalias_params(&m.params);
                let a = self.args(&ps, args, FnPos::DynParam, indent);
                format!("{inst}.{}({})", rs_ident(&m.emitted_name), a.join(", "))
            }
            ExprKind::Op { op, args } => self.op(e, *op, args, indent),
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
            ExprKind::Branch { id, arms, otherwise } => self.branch(e, *id, arms, otherwise.as_ref(), indent),
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
        // The sign test of a primitive `cmp` is the primitive operator.
        if matches!(op, Op::Lt | Op::Gt | Op::LtEq | Op::GtEq) && args.len() == 2 {
            if let (ExprKind::Call { target: FnRef::Decl(id), args: inner, .. }, ExprKind::Int(0)) = (&args[0].kind, &args[1].kind) {
                let prim = self.s.ast_fn(id).is_some_and(|f| f.intrinsic && f.name.name == "cmp") && inner.len() == 2 && is_copy_ty(&inner[0].ty);
                if prim {
                    let a = self.value(&inner[0], indent);
                    let b = self.value(&inner[1], indent);
                    return format!("({a} {} {b})", sym(op));
                }
            }
        }
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
            Op::Lt | Op::Gt | Op::LtEq | Op::GtEq => format!("({} {} {})", a[0], sym(op), a[1]),
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
                self.s.platform_hosts.insert(h.id.module.clone());
                format!("{}{}", self.s.prefix(&h.id.module), crate::emit::platform_adapter_name(&rs_ident(&h.name)))
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
    fn rewrap(&mut self, code: &str, from: &Ty, to: &Ty) -> String {
        let fa: Vec<Ty> = from.strip_quals().value_arms().into_iter().cloned().collect();
        let ta: Vec<Ty> = to.strip_quals().value_arms().into_iter().cloned().collect();
        let (fo, to_opt) = (from.strip_quals().has_none_arm(), to.strip_quals().has_none_arm());
        let pat = |s: &mut Self, arms: &[Ty], i: usize, opt: bool, inner: &str| -> String {
            let p = if arms.len() >= 2 {
                s.s.union_sizes.insert(arms.len());
                format!("crate::unions::Union{}::U{}({inner})", arms.len(), i + 1)
            } else {
                inner.to_string()
            };
            if opt { format!("Some({p})") } else { p }
        };
        let mut arms = Vec::new();
        for (i, a) in fa.iter().enumerate() {
            let j = ta.iter().position(|b| b == a).or_else(|| {
                let same: Vec<usize> = ta.iter().enumerate().filter(|(_, b)| alike(b, a)).map(|(j, _)| j).collect();
                (same.len() == 1).then(|| same[0])
            });
            let lhs = pat(self, &fa, i, fo, "__v");
            match j {
                Some(j) => {
                    let rhs = pat(self, &ta, j, to_opt, "__v");
                    arms.push(format!("{lhs} => {rhs}"));
                }
                None => {
                    // The arm is itself a union the target spreads.
                    let inner_arms: Vec<Ty> = a.strip_quals().value_arms().into_iter().cloned().collect();
                    if inner_arms.len() >= 2 {
                        let inner = self.rewrap("__v", a, to);
                        arms.push(format!("{lhs} => {inner}"));
                    } else {
                        arms.push(format!("{lhs} => unreachable!(\"salvo: unreachable union arm\")"));
                    }
                }
            }
        }
        if fo {
            arms.push(if to_opt { "None => None".to_string() } else { "None => unreachable!(\"salvo: unreachable union arm\")".to_string() });
        }
        if fa.len() < 2 && !fo {
            // A plain value placed into the target.
            let j = ta.iter().position(|b| alike(b, from)).unwrap_or(0);
            return pat(self, &ta, j, to_opt, code);
        }
        arms.push("#[allow(unreachable_patterns)] _ => unreachable!(\"salvo: unreachable union arm\")".to_string());
        format!("(match {code} {{ {} }})", arms.join(", "))
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

    fn branch(&mut self, e: &Expr, id: salvo_ir::NodeId, arms: &[(Expr, Block)], otherwise: Option<&Block>, indent: usize) -> String {
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

    fn switch(&mut self, e: &Expr, id: salvo_ir::NodeId, subject: &Expr, arms: &[salvo_ir::SwitchArm], indent: usize) -> String {
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

    fn narrow(&mut self, local: &Local, ty: &Ty, from: &Place, from_ty: &Ty, because: &Justification, indent: usize) -> String {
        let pad = Self::pad(indent);
        let n = rs_local(&local.0);
        self.f.tys.insert(local.0.clone(), ty.clone());
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
            if is_mut(ty) {
                let code = if nn >= 2 { format!("match &mut {src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_mut().unwrap()") };
                (code, Kind::RefMut)
            } else {
                let code = if nn >= 2 { format!("match &{src} {{ {} => __v, _ => unreachable!() }}", pat(i, "__v")) } else { format!("{src}.as_ref().unwrap()") };
                (code, Kind::Ref)
            }
        } else if copy_or_ref {
            let k = if is_copy_ty(ty) { Kind::Owned } else if is_mut(ty) { Kind::RefMut } else { Kind::Ref };
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
        // [rs-loc] Over the anchor, a locator loops by position.
        if let (Some((anchor, _)), Some(r)) = (self.f.loc.clone(), bare(iterable)) {
            if r.0 == anchor && is_proj(ty) {
                let i = self.fresh("li");
                let a = rs_local(&anchor);
                let saved = self.f.kinds.clone();
                self.f.kinds.insert(local.0.clone(), Kind::Ref);
                self.f.loc_index.insert(local.0.clone(), i.clone());
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
    fn args(&mut self, params: &[Param], args: &[Expr], fn_pos: FnPos, indent: usize) -> Vec<String> {
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
        // [rs-elem-mut] [elem-distinct] Two mutable lends of one anchor,
        // proven apart: one split borrow.
        // Elem handles of one anchor, passed mutably together.
        let elems: Vec<(usize, String, String)> = args
            .iter()
            .enumerate()
            .filter(|(i, _)| params.get(*i).is_some_and(|p| p.mode == PassMode::LentMut))
            .filter_map(|(i, a)| bare(a).and_then(|r| self.f.elem.get(&r.0)).and_then(|(an, p)| p.clone().map(|p| (i, an.clone(), p))))
            .collect();
        if elems.len() == 2 && elems[0].1 == elems[1].1 {
            self.s.needs_seq = true;
            let (p0, p1) = (self.fresh("pm"), self.fresh("pm"));
            let lets = vec![format!("let ({p0}, {p1}) = crate::seq::salvo_pair_mut(&mut {}[..], {}, {}).expect(\"salvo: value is absent\");", elems[0].1, elems[0].2, elems[1].2)];
            let mut out = Vec::new();
            for (i, a) in args.iter().enumerate() {
                if i == elems[0].0 {
                    out.push(p0.clone());
                } else if i == elems[1].0 {
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
        // Two mutable lends of one anchor (direct or `x!`), proven apart.
        let cands: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| self.s.mut_lend(a).is_some() || matches!(&a.kind, ExprKind::Branch { arms, .. } if arms.len() == 1 && arms[0].1.stmts.len() == 1 && matches!(&arms[0].1.stmts[0], Stmt::Let { value, .. } if self.s.mut_lend(value).is_some())))
            .map(|(i, _)| i)
            .collect();
        if cands.len() == 2 {
            let saved_tmp = self.f.tmp;
            let l0 = self.loc_of(&args[cands[0]], indent);
            let l1 = self.loc_of(&args[cands[1]], indent);
            if let (Some((a0, p0)), Some((a1, p1))) = (l0, l1) {
                if a0 == a1 {
                    self.s.needs_seq = true;
                    let (x0, x1) = (self.fresh("l"), self.fresh("l"));
                    let (m0, m1) = (self.fresh("pm"), self.fresh("pm"));
                    let lets = vec![
                        format!("let {x0} = {p0};"),
                        format!("let {x1} = {p1};"),
                        format!("let ({m0}, {m1}) = crate::seq::salvo_pair_mut(&mut {a0}[..], {x0}, {x1}).expect(\"salvo: value is absent\");"),
                    ];
                    let mut out = Vec::new();
                    for (i, a) in args.iter().enumerate() {
                        if i == cands[0] {
                            out.push(m0.clone());
                        } else if i == cands[1] {
                            out.push(m1.clone());
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
            if reads_root && by_value {
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

    /// [canbe-entry] [rs-loc] A covered call: positions for the covered
    /// arguments (computed first), the shared anchor where none is named.
    fn covered_args(&mut self, params: &[Param], args: &[Expr], covered: &HashMap<usize, String>, indent: usize) -> (Vec<String>, Vec<String>) {
        let mut lets = Vec::new();
        let mut out = Vec::new();
        let mut anchor_done = false;
        for (i, a) in args.iter().enumerate() {
            if let Some(an) = covered.get(&i) {
                let (anchor, pos) = match bare(a).and_then(|r| self.f.elem.get(&r.0).cloned()) {
                    Some((anchor, Some(p))) => (anchor, p),
                    _ => match self.loc_of(a, indent) {
                        Some(x) => x,
                        None => {
                            self.error("a covered argument that is not an element handle [canbe-entry]");
                            ("()".to_string(), "0".to_string())
                        }
                    },
                };
                if an == "__anchor" && !anchor_done {
                    anchor_done = true;
                    out.push(format!("&mut {anchor}"));
                }
                let t = self.fresh("c");
                lets.push(format!("let {t} = {pos};"));
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
        if matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
            let pos = if p.mode == PassMode::Moved && fn_pos == FnPos::Param { FnPos::Owned } else { fn_pos };
            return self.fn_arg(a, &p.ty, pos, indent);
        }
        if is_copy_ty(&p.ty) || p.variadic || is_proj(&p.ty) {
            return self.value_into(a, &p.ty, indent);
        }
        if is_copy_ty(&a.ty) && p.mode != PassMode::LentMut && p.mode != PassMode::Lent {
            return self.value(a, indent);
        }
        match p.mode {
            PassMode::Moved => self.value_into(a, &p.ty, indent),
            PassMode::Lent => self.borrow(a, indent),
            PassMode::LentMut => self.borrow_mut(a, indent),
        }
    }

    /// A fn value passed to a fn-typed slot at `pos`.
    pub fn fn_arg(&mut self, a: &Expr, slot: &Ty, pos: FnPos, indent: usize) -> String {
        if let ExprKind::Call { target: FnRef::Decl(id), args, .. } = &a.kind {
            if args.len() == 1 && self.s.ast_fn(id).is_some_and(|f| f.intrinsic && f.name.name == "copy") && matches!(self.s.unalias(&args[0].ty).strip_quals(), Ty::Fn { .. }) {
                return self.fn_arg(&args[0], slot, pos, indent);
            }
        }
        if let ExprKind::FnValue(FnRef::Local(l)) = &a.kind {
            let read = Expr { ty: a.ty.clone(), span: a.span, kind: ExprKind::Read { place: Place { root: l.clone(), steps: Vec::new() }, consume: false } };
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
                // differ from the slot's is adapted.
                if place.steps.is_empty() && matches!(pos, FnPos::Param | FnPos::DynParam) {
                    if let Some(lt) = self.f.tys.get(&place.root.0).cloned() {
                        let lt = self.s.unalias(&lt);
                        let (_, lm) = self.closure_params(&lt);
                        let (sp, sm) = self.closure_params(slot);
                        if lm.len() == sm.len() && lm != sm && !self.s.fn_ty_lends_mut(slot) {
                            let names: Vec<String> = sp.iter().map(|(n, _)| n.clone()).collect();
                            let conv: Vec<String> = names
                                .iter()
                                .zip(sm.iter().zip(&lm))
                                .map(|(n, (s, l))| match (s, l) {
                                    (PassMode::Moved, PassMode::Moved) | (PassMode::Lent, PassMode::Lent) | (PassMode::LentMut, PassMode::LentMut) => n.clone(),
                                    (PassMode::Lent | PassMode::LentMut, PassMode::Moved) => format!("(*{n}).clone()"),
                                    (PassMode::Moved, PassMode::Lent) => format!("&{n}"),
                                    (PassMode::Moved, PassMode::LentMut) => format!("&mut {n}"),
                                    (PassMode::LentMut, PassMode::Lent) => format!("&*{n}"),
                                    (PassMode::Lent, PassMode::LentMut) => n.clone(),
                                })
                                .collect();
                            let callee = if kind == Kind::SelfField { format!("({text})") } else { text.clone() };
                            return format!("&mut |{}| {callee}({})", names.join(", "), conv.join(", "));
                        }
                    }
                }
                match (pos, kind) {
                    (FnPos::Param | FnPos::DynParam, Kind::RefMut) if place.steps.is_empty() => format!("&mut *{text}"),
                    (FnPos::Param | FnPos::DynParam, Kind::Owned) if place.steps.is_empty() => format!("&mut {text}"),
                    (FnPos::Param | FnPos::DynParam, _) => {
                        // A stored callback, called through.
                        let (ps, _) = self.closure_params(slot);
                        let names: Vec<String> = ps.iter().map(|(n, _)| n.clone()).collect();
                        let typed: Vec<String> = ps.iter().map(|(n, t)| format!("{n}: {t}")).collect();
                        format!("&mut |{}| ({text})({})", typed.join(", "), names.join(", "))
                    }
                    (FnPos::Stored, Kind::Owned) if place.steps.is_empty() => format!("std::sync::Arc::new({text})"),
                    (FnPos::Owned, Kind::Owned) if place.steps.is_empty() => text,
                    (_, _) => format!("{text}.clone()"),
                }
            }
            _ => {
                let v = self.value(a, indent);
                match pos {
                    FnPos::Param | FnPos::DynParam => format!("&mut {v}"),
                    _ => v,
                }
            }
        }
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
        for (i, e) in effects.iter().enumerate() {
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
            Ty::Fn { params, effects, .. } => effects.iter().chain(params.iter()).cloned().collect(),
            _ => Vec::new(),
        };
        let saved_kinds = self.f.kinds.clone();
        let saved_ret = self.f.ret.replace(ret.clone());
        // Closures do not throw [rs-throw-controlflow].
        let saved_throws = self.f.throws.take();
        let saved_tries = std::mem::take(&mut self.f.tries);
        let saved_lt = self.f.ret_lt;
        self.f.ret_lt = false;
        let slot_effects = match slot.strip_quals() {
            Ty::Fn { effects, .. } => effects.len(),
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
                peel.push_str(&format!("{}let {n} = {}*{n};\n", Self::pad(indent + 1), if is_mut(&p.ty) { "&mut *" } else { "" }));
                k = if is_mut(&p.ty) { Kind::RefMut } else { Kind::Ref };
                self.f.tys.insert(p.local.0.clone(), inst_params.get(i).cloned().filter(is_proj).unwrap_or_else(|| p.ty.clone()));
            }
            self.f.kinds.insert(p.local.0.clone(), k);
            self.f.tys.insert(p.local.0.clone(), p.ty.clone());
            match cps.get(i) {
                Some((_, t)) if annotate && !foreign_var(&p.ty, &self.f.generics) => ps.push(format!("mut {n}: {t}")),
                _ => ps.push(format!("mut {n}")),
            }
        }
        // [rs-loc] A lambda in a lending slot is a locator: it answers the
        // position in its anchor parameter.
        let lends = self.s.fn_ty_lends_mut(slot);
        let saved_loc = self.f.loc.take();
        let saved_li = std::mem::take(&mut self.f.loc_index);
        if lends {
            let anchor = params.iter().zip(&modes).find(|(p, m)| !is_copy_ty(&p.ty) && **m != PassMode::Moved).map(|(p, _)| p.local.0.clone()).unwrap_or_default();
            self.f.loc = Some((anchor, ret.strip_quals().has_none_arm()));
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
            b.push_str(&format!("{}{code}\n", Self::pad(indent + 1)));
        }
        self.f.loc = saved_loc;
        self.f.loc_index = saved_li;
        self.f.kinds = saved_kinds;
        self.f.ret = saved_ret;
        self.f.ret_lt = saved_lt;
        self.f.throws = saved_throws;
        self.f.tries = saved_tries;
        let r = if lends {
            if ret.strip_quals().has_none_arm() { " -> Option<usize>".to_string() } else { " -> usize".to_string() }
        } else if ret.is_none_ty() || foreign_var(ret, &self.f.generics) {
            String::new()
        } else {
            format!(" -> {}", self.ty(ret))
        };
        let mv = if matches!(pos, FnPos::Owned | FnPos::Stored) { "move " } else { "" };
        let _ = &cps;
        let closure = format!("{mv}|{}|{r} {{\n{b}{}}}", ps.join(", "), Self::pad(indent));
        if mv.is_empty() {
            return closure;
        }
        // A kept closure owns clones of what it captures.
        let mut caps: Vec<String> = Vec::new();
        walk_block(body, &mut |_| {}, &mut |e| {
            if let ExprKind::Read { place, .. } = &e.kind {
                caps.push(place.root.0.clone());
            }
        });
        caps.sort();
        caps.dedup();
        let own: HashSet<String> = params.iter().map(|p| p.local.0.clone()).collect();
        let mut pre = String::new();
        for c in caps {
            if own.contains(&c) || !self.f.kinds.contains_key(&c) {
                continue;
            }
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
        let saved = self.f.kinds.clone();
        let inst_params: Vec<Ty> = match super::decls::subst_ty(slot, &self.f.call_subst).strip_quals() {
            Ty::Fn { params, effects, .. } => effects.iter().chain(params.iter()).cloned().collect(),
            _ => Vec::new(),
        };
        let mut args = Vec::new();
        let mut peel = String::new();
        for (i, ((n, _), m)) in ps.iter().zip(&modes).enumerate() {
            let mut t = if i < effects.len() { effects[i].clone() } else { params[i - effects.len()].clone() };
            let mut k = match m {
                PassMode::Moved => Kind::Owned,
                PassMode::Lent => Kind::Ref,
                PassMode::LentMut => Kind::RefMut,
            };
            // [rs-proj-generic] A borrowed element arrives one reference deeper.
            if let Some(it) = inst_params.get(i).filter(|t| is_proj(t)) {
                if *m != PassMode::Moved && !is_proj(&t) {
                    peel.push_str(&format!("let {n} = *{n}; "));
                    t = it.clone();
                    k = Kind::Ref;
                }
            }
            self.f.kinds.insert(n.clone(), k);
            self.f.tys.insert(n.clone(), t.clone());
            args.push(Expr { ty: t, span: e.span, kind: ExprKind::Read { place: Place { root: Local(n.clone()), steps: Vec::new() }, consume: *m == PassMode::Moved } });
        }
        let want = match super::decls::subst_ty(slot, &self.f.call_subst).strip_quals() {
            Ty::Fn { ret: r, .. } if !super::body::foreign_var(r, &self.f.generics) || self.f.call_subst.is_empty() => (**r).clone(),
            _ => (*ret).clone(),
        };
        let call = Expr { ty: want, span: e.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: Vec::new(), args } };
        let body = if self.s.fn_ty_lends_mut(slot) {
            // [rs-loc] The callee's locator.
            let ExprKind::Call { args, .. } = &call.kind else { unreachable!() };
            match self.s.fn_decl(id) {
                Some(f) => self.loc_call(id, f, args, indent),
                None => "0".to_string(),
            }
        } else {
            self.value(&call, indent)
        };
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

    /// [copy-scalar-free] A borrow of a Copy scalar is the scalar: a call
    /// whose declared result is a `proj` the instantiation made a scalar
    /// reads it out.
    fn deref_scalar(&mut self, code: String, decl: &Ty, inst: &Ty) -> String {
        // A borrow the instantiation reads as a value.
        let owned = |d: &Ty, i: &Ty| is_proj(d) && !is_proj(i);
        if owned(decl, inst) {
            return if is_copy_ty(inst) { format!("(*{code})") } else { format!("({code}).clone()") };
        }
        let (da, ia) = (decl.strip_quals().value_arms(), inst.strip_quals().value_arms());
        if da.len() != ia.len() || !da.iter().zip(&ia).any(|(d, i)| owned(d, i)) {
            if let (Ty::Named { name: dn, args: dargs }, Ty::Named { name: inn, args: iargs }) = (decl.strip_quals(), inst.strip_quals()) {
                if dn == "List" && inn == "List" && dargs.len() == 1 && iargs.len() == 1 && owned(&dargs[0], &iargs[0]) {
                    let m = if is_copy_ty(&iargs[0]) { "copied" } else { "cloned" };
                    return format!("{code}.into_iter().{m}().collect::<Vec<_>>()");
                }
            }
            return code;
        }
        let opt = decl.strip_quals().has_none_arm() && inst.strip_quals().has_none_arm();
        if da.len() == 1 {
            let m = if is_copy_ty(ia[0]) { "copied" } else { "cloned" };
            return if opt { format!("{code}.{m}()") } else if is_copy_ty(ia[0]) { format!("(*{code})") } else { format!("({code}).clone()") };
        }
        let n = da.len();
        let mut arms = Vec::new();
        for (i, (d, a)) in da.iter().zip(&ia).enumerate() {
            let p = format!("crate::unions::Union{n}::U{}(__v)", i + 1);
            let v = if is_proj(d) && !is_proj(a) { if is_copy_ty(a) { "*__v" } else { "__v.clone()" } } else { "__v" };
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

    /// [rs-loc] A returned borrow, as a position in the anchor.
    fn loc_expr(&mut self, e: &Expr, opt: bool, indent: usize) -> String {
        let wrap = |code: String, is_opt: bool| -> String {
            match (opt, is_opt) {
                (true, false) => format!("Some({code})"),
                (false, true) => format!("{code}.expect(\"salvo: value is absent\")"),
                _ => code,
            }
        };
        match &e.kind {
            ExprKind::Present { value } => {
                let c = self.loc_expr(value, false, indent);
                wrap(c, false)
            }
            ExprKind::MakeNone => "None".to_string(),
            ExprKind::Read { place, .. } if place.steps.is_empty() && self.f.loc_index.contains_key(&place.root.0) => {
                let i = self.f.loc_index[&place.root.0].clone();
                wrap(i, false)
            }
            ExprKind::Call { target: FnRef::Decl(id), args, .. } => {
                let is_opt = e.ty.strip_quals().has_none_arm();
                if let Some(af) = self.s.ast_fn(id).filter(|af| af.intrinsic) {
                    let name = af.name.name.clone();
                    let recv = af.params.first().and_then(|p| salvo_backend::emit_util::type_base_name(&p.ty));
                    let a: Vec<String> = args.iter().map(|x| self.raw(x, indent)).collect();
                    return match crate::intrinsics::fn_call(&name, recv, &a, Spread::None, true) {
                        Some(c) => wrap(c, is_opt),
                        None => {
                            self.error(format!("intrinsic `{name}` has no locator form [rs-loc]"));
                            "0".to_string()
                        }
                    };
                }
                let Some(f) = self.s.fn_decl(id) else { return "0".to_string() };
                let c = self.loc_call(id, f, args, indent);
                wrap(c, is_opt)
            }
            _ => {
                self.error("a locator returns something other than a position of its anchor [rs-loc]");
                "0".to_string()
            }
        }
    }

    /// [rs-loc] A mutable lend as (anchor text, position code), when it
    /// has one: a lending call, a lending callback, or `x!` of either.
    fn loc_of(&mut self, e: &Expr, indent: usize) -> Option<(String, String)> {
        let anchor_of = |s: &mut Self, a: &Expr| -> Option<String> {
            let p = match &a.kind {
                ExprKind::Read { place, .. } => place.clone(),
                ExprKind::DropMut { value } => match &value.kind {
                    ExprKind::Read { place, .. } => place.clone(),
                    _ => return None,
                },
                _ => return None,
            };
            Some(s.place_text(&p, indent).0)
        };
        match &e.kind {
            ExprKind::Call { target: FnRef::Decl(id), args, .. } => {
                let f = self.s.fn_decl(id)?;
                let k = match self.s.mut_lend(e) {
                    Some((_, k)) => k,
                    None if self.s.locs.contains(id) && is_mut(&f.ret.strip_quals().without_none()) => self.s.lend_param(f)?,
                    None => return None,
                };
                let anchor = anchor_of(self, args.get(k)?)?;
                let pos = self.loc_call(id, f, args, indent);
                Some((anchor, pos))
            }
            // `x!`: the position, or the trap.
            ExprKind::Branch { arms, otherwise: None, .. } if arms.len() == 1 && matches!(arms[0].0.kind, ExprKind::Bool(true)) => {
                let b = &arms[0].1;
                let [Stmt::Let { local, value, .. }] = b.stmts.as_slice() else { return None };
                let Some(v) = &b.value else { return None };
                let ExprKind::Switch { subject, arms: sarms, .. } = &v.kind else { return None };
                if bare(subject) != Some(local) {
                    return None;
                }
                let at = sarms.iter().find_map(|a| {
                    let from_stmt = a.body.stmts.iter().find_map(|s| match s {
                        Stmt::Expr(Expr { kind: ExprKind::Unreachable { at, .. }, .. }) => Some(at.clone()),
                        _ => None,
                    });
                    match a.body.value.as_deref().map(|x| &x.kind) {
                        Some(ExprKind::Unreachable { at, .. }) => Some(at.clone()),
                        _ => from_stmt,
                    }
                })?;
                let (anchor, pos) = self.loc_of(value, indent)?;
                Some((anchor, format!("{pos}.expect(\"salvo: value is absent at {at}\")")))
            }
            _ => None,
        }
    }

    /// `g__loc(args)`: the anchor argument read.
    fn loc_call(&mut self, id: &salvo_ir::DeclId, f: &salvo_ir::FnDecl, args: &[Expr], indent: usize) -> String {
        let mut ps = self.s.unalias_params(&f.params);
        if let Some(a) = self.s.lend_param(f) {
            ps[a].mode = PassMode::Lent;
        }
        let a = self.args(&ps, args, FnPos::Param, indent);
        format!("{}__loc({})", self.fn_path(id), a.join(", "))
    }

    /// [rs-loc] A mutable lend: the position, then the element of the anchor.
    fn mut_lend_call(&mut self, e: &Expr, id: &salvo_ir::DeclId, k: usize, args: &[Expr], indent: usize) -> String {
        let Some(f) = self.s.fn_decl(id) else { return "()".to_string() };
        let Some(anchor) = args.get(k).and_then(|a| match &a.kind {
            ExprKind::Read { place, .. } => Some(place.clone()),
            ExprKind::DropMut { value } => match &value.kind {
                ExprKind::Read { place, .. } => Some(place.clone()),
                _ => None,
            },
            _ => None,
        }) else {
            self.error("a mutable lend from something that is not a place [rs-loc]");
            return "()".to_string();
        };
        let loc = self.loc_call(id, f, args, indent);
        let (a, _) = self.place_text(&anchor, indent);
        let i = self.fresh("l");
        if e.ty.strip_quals().has_none_arm() {
            format!("{{ match {loc} {{ Some({i}) => Some(&mut {a}[{i}]), None => None }} }}")
        } else {
            format!("{{ let {i} = {loc}; &mut {a}[{i}] }}")
        }
    }

    fn call(&mut self, e: &Expr, target: &FnRef, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        if let Some((id, k)) = self.s.mut_lend(e) {
            return self.mut_lend_call(e, &id, k, args, indent);
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
                        Param { local: Local(String::new()), ty: pt, mode, variadic: false, check: None }
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
                if !lends {
                    return code;
                }
                // [rs-loc] A locator callback: the element at its position.
                let anchor = ps.iter().zip(args).find(|(p, _)| !is_copy_ty(&p.ty)).and_then(|(_, a)| match &a.kind {
                    ExprKind::Read { place, .. } => Some(place.clone()),
                    ExprKind::DropMut { value } => match &value.kind {
                        ExprKind::Read { place, .. } => Some(place.clone()),
                        _ => None,
                    },
                    _ => None,
                });
                let Some(anchor) = anchor else {
                    self.error("a lending callback called on something that is not a place [rs-loc]");
                    return code;
                };
                let (a, _) = self.place_text(&anchor, indent);
                let i = self.fresh("l");
                if e.ty.strip_quals().has_none_arm() {
                    format!("{{ match {code} {{ Some({i}) => Some(&mut {a}[{i}]), None => None }} }}")
                } else {
                    format!("{{ let {i} = {code}; &mut {a}[{i}] }}")
                }
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
                let covered = self.covered(f);
                let saved_subst = std::mem::take(&mut self.f.call_subst);
                if type_args.len() == f.type_params.len() {
                    self.f.call_subst = f.type_params.iter().map(|t| t.name.clone()).zip(type_args.iter().cloned()).collect();
                }
                let (lets, mut a) = if covered.is_empty() { self.call_args(&ps, args, indent) } else { self.covered_args(&ps, args, &covered, indent) };
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
                    let fi = self.s.program.files.iter().position(|x| x.module == id.module).unwrap_or(0);
                    self.s.erased.fns.contains(&(fi, af.name.span.start))
                });
                let impl_params = self.stores_callbacks_pub(f);
                let ta = if !erased && tps > 0 && type_args.len() == tps && type_args.iter().all(|t| !t.is_unknown() && !super::body::foreign_var(t, &self.f.generics) && (f.kind == FnKind::Plain || !self.s.holds_proj(t))) && !impl_params {
                    let ts: Vec<String> = type_args.iter().map(|t| self.ty(t)).collect();
                    // A borrowed argument (to a Salvo fn: a host's answers
                    // owned) names no lifetime of its own here (`&Fighter`,
                    // `ListYield<'_, Fighter>`), or none is said.
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
                let vars = self.s.plain_proj_vars(&f.type_params, &f.params, &f.borrows);
                let tps: HashSet<String> = f.type_params.iter().map(|t| t.name.clone()).collect();
                let ret = super::strip_nested_var_proj_in(&super::strip_plain_proj(&f.ret, &vars), false, &tps);
                self.deref_scalar(code, &ret, &e.ty)
            }
        }
    }

    fn intrinsic_call(&mut self, e: &Expr, id: &salvo_ir::DeclId, af: &salvo_syntax::ast::FnDecl, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        let name = af.name.name.as_str();
        let recv = af.params.first().and_then(|p| salvo_backend::emit_util::type_base_name(&p.ty));
        let _ = type_args;
        if name == "copy" {
            let v = self.raw(&args[0], indent);
            if is_copy_ty(&args[0].ty) {
                return v;
            }
            return format!("({v}).clone()");
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
                    let vars = self.s.plain_proj_vars(&f.type_params, &f.params, &f.borrows);
                    let tps: HashSet<String> = f.type_params.iter().map(|t| t.name.clone()).collect();
                    let ret = super::strip_nested_var_proj_in(&super::strip_plain_proj(&f.ret, &vars), false, &tps);
                    self.deref_scalar(code, &ret, &e.ty)
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
        _ => ">=",
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

