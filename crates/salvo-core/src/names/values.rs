//! [name-table] Value-position names (PIPELINE.md C1.3): every name a body
//! writes where a value goes — a variable read, a callee, a handler in a
//! `use`, a selector — bound by resolution where it is written: to the
//! binding it reads (a `let`, a parameter, a state field, an `is` binding, a
//! lambda's parameter …), or, when no binding is in scope, to what the name
//! means there as a declaration ([`Meaning`]: the fns, effect members,
//! handler, struct … of that name).
//!
//! The scopes are the checker's, stated once: a block is a scope; a `let`
//! binds after its value; a condition's `is` bindings reach the branch it
//! guards and the right side of a `&&` (not an `||`, a `!` or an
//! `assert!`); `when name = s` reaches every branch and a branch's binding
//! its body; a `for` pattern, a lambda's parameters and a `waitfor` token
//! their bodies. A fn body sees its parameters, its implicit groups'
//! members (bound at the group, which has no name of its own), and — a
//! handler member — the handler's constructor parameters and state; a
//! parameter's default sees the parameters before it, a state initialiser
//! and a mailbox the constructor's parameters. Locals never shadow
//! [var-no-shadow], so innermost-first finds the one binding a name can
//! mean.

use std::collections::HashMap;

use salvo_syntax::ast::{
    Block, Expr, FieldDecl, FnDecl, HandlerDecl, Ident, Item, LambdaBody, Module, Param, Pattern, SafeAccess, Stmt, StrExprPart,
    StructLitFieldKind,
};
use salvo_syntax::Span;

use super::{At, Meaning, Names};
use crate::resolve::ModuleScope;

/// [name-table] One written value-position name.
#[derive(Clone, Copy, Debug)]
pub struct WrittenValue {
    /// The name's address in the tree resolution walked.
    pub addr: usize,
    /// The file it is written in.
    pub file: u32,
    /// The scope it is resolved in.
    pub at: At,
    /// The name's span, its node [node-id] included.
    pub span: Span,
    /// The binding it reads, by the binding's span (its node): `None` for a
    /// name no binding in scope has, which means what [`Names::value_meaning`]
    /// says (for a bound one, it says nothing).
    pub local: Option<Span>,
    pub(super) name: u32,
    pub(super) meaning: u32,
}

/// The walk: a scope stack of bindings, by name.
pub(super) struct Binder<'b, 'p> {
    pub(super) names: &'b mut Names<'p>,
    pub(super) interned: &'b mut HashMap<(At, u32), u32>,
    pub(super) scopes: &'b [ModuleScope<'p>],
    pub(super) stamp_scopes: &'b [ModuleScope<'p>],
    pub(super) fn_stamps: &'b HashMap<usize, usize>,
    pub(super) file: u32,
    pub(super) at: At,
    /// Per scope, the bindings by spelling.
    frames: Vec<HashMap<u32, Span>>,
}

impl<'b, 'p> Binder<'b, 'p> {
    pub(super) fn new(
        names: &'b mut Names<'p>,
        interned: &'b mut HashMap<(At, u32), u32>,
        scopes: &'b [ModuleScope<'p>],
        stamp_scopes: &'b [ModuleScope<'p>],
        fn_stamps: &'b HashMap<usize, usize>,
        file: u32,
    ) -> Self {
        Binder { names, interned, scopes, stamp_scopes, fn_stamps, file, at: At::File(file), frames: Vec::new() }
    }

    fn scope(&self) -> &ModuleScope<'p> {
        match self.at {
            At::File(f) => &self.scopes[f as usize],
            At::Stamp(i) => &self.stamp_scopes[i as usize],
        }
    }

    fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn bind(&mut self, id: &Ident) {
        self.bind_at(&id.name, id.span);
    }

    fn bind_at(&mut self, name: &str, span: Span) {
        let spelling = self.names.spell(name);
        if let Some(f) = self.frames.last_mut() {
            f.insert(spelling, span);
        }
    }

    fn bound_here(&mut self, name: &str) -> bool {
        let spelling = self.names.spell(name);
        self.frames.last().is_some_and(|f| f.contains_key(&spelling))
    }

    /// Records one value-position name; `locals` says whether a binding may
    /// answer it (a selected name, `f@m`, is never a local's).
    fn name(&mut self, id: &Ident, locals: bool) {
        let spelling = self.names.spell(&id.name);
        let local = if locals { self.frames.iter().rev().find_map(|f| f.get(&spelling).copied()) } else { None };
        let at = self.at;
        // A binding answers it: what the name would mean as a declaration
        // is nothing anyone reads (the shared empty meaning).
        let meaning = match self.interned.get(&(at, spelling)) {
            _ if local.is_some() => self.names.empty_meaning(),
            Some(m) => *m,
            None => {
                let m = Meaning::of(self.scope(), &id.name);
                self.names.meanings.push(m);
                let m = self.names.meanings.len() as u32 - 1;
                self.interned.insert((at, spelling), m);
                m
            }
        };
        let addr = id as *const Ident as usize;
        let idx = self.names.values.len() as u32;
        self.names.values.push(WrittenValue { addr, file: self.file, at, span: id.span, local, name: spelling, meaning });
        self.names.value_by_addr.insert(addr, idx);
        if id.span.is_numbered() {
            let key = (id.span.node, id.span, spelling);
            match self.names.value_by_copy.get(&key).copied() {
                None => {
                    self.names.value_by_copy.insert(key, Some(idx));
                }
                Some(Some(prev)) => {
                    let prev = self.names.values[prev as usize];
                    let agree = prev.local.map(|s| (s.node, s.start)) == local.map(|s| (s.node, s.start))
                        && self.names.meanings[prev.meaning as usize].same(&self.names.meanings[meaning as usize]);
                    if !agree {
                        self.names.value_by_copy.insert(key, None);
                    }
                }
                Some(None) => {}
            }
        }
    }

    // ------------------------------------------------------------ items --

    pub(super) fn module(&mut self, module: &Module) {
        // [mod-use] A module-level binding sees no locals.
        for u in &module.uses {
            self.push();
            self.expr(&u.handler);
            self.pop();
        }
        for item in &module.items {
            self.item(item);
        }
    }

    fn item(&mut self, item: &Item) {
        match item {
            Item::Fn(f) => self.func(f, &[], &[]),
            Item::Struct(s) => {
                for field in &s.fields {
                    self.field_default(field, &[]);
                }
                for f in &s.fns {
                    self.func(f, &[], &[]);
                }
            }
            Item::Effect(e) => {
                for f in &e.fns {
                    self.func(f, &[], &[]);
                }
            }
            Item::Handler(h) => self.handler(h),
            Item::Qualifier(q) => {
                for field in &q.field_overrides {
                    self.field_default(field, &[]);
                }
                for f in &q.fns {
                    self.func(f, &[], &[]);
                }
            }
            Item::Fns(g) => {
                for f in &g.fns {
                    self.func(f, &[], &[]);
                }
            }
            Item::Namespace(ns) => {
                for i in &ns.items {
                    self.item(i);
                }
            }
            Item::Test(t) => {
                self.push();
                self.block(&t.body);
                self.pop();
            }
            Item::Type(_) | Item::Import(_) | Item::Refn(_) | Item::Rename(_) | Item::EffectAlias(_) => {}
        }
    }

    /// A field's default: the constructor's parameters `ctor`, when a
    /// handler's state initialiser, else nothing.
    fn field_default(&mut self, field: &FieldDecl, ctor: &[Param]) {
        if let Some(d) = &field.default {
            self.push();
            for p in ctor {
                self.bind(&p.name);
            }
            self.expr(d);
            self.pop();
        }
    }

    fn handler(&mut self, h: &HandlerDecl) {
        // [fn-param-default] A constructor parameter's default sees the
        // parameters before it.
        self.defaults(&h.params);
        // [handler-state] An initialiser, and the mailbox bound, see the
        // constructor's parameters.
        for field in &h.state {
            self.field_default(field, &h.params);
        }
        if let Some(m) = &h.mailbox {
            self.push();
            for p in &h.params {
                self.bind(&p.name);
            }
            self.expr(m);
            self.pop();
        }
        if let Some(f) = &h.init {
            self.func(f, &h.params, &h.state);
        }
        // [actor-mixed-handler] A mixed handler's sync member is the façade:
        // the state is the servant's, out of its scope.
        let mixed = h.fns.iter().any(|f| f.is_send)
            && !h.of.is_empty()
            && h.of.iter().all(|of| {
                let base = match of {
                    salvo_syntax::ast::Type::Named { base, .. } => base,
                    salvo_syntax::ast::Type::QualifiedGroup { base, .. } => match base.as_ref() {
                        salvo_syntax::ast::Type::Named { base, .. } => base,
                        _ => return false,
                    },
                    _ => return false,
                };
                self.scope().effects.get(base.name.name.as_str()).is_some_and(|e| !e.is_actor)
            });
        for f in &h.fns {
            let state: &[FieldDecl] = if mixed && !f.is_send { &[] } else { &h.state };
            self.func(f, &h.params, state);
        }
    }

    fn defaults(&mut self, params: &[Param]) {
        for (i, p) in params.iter().enumerate() {
            if let Some(d) = &p.default {
                self.push();
                for q in &params[..i] {
                    self.bind(&q.name);
                }
                self.expr(d);
                self.pop();
            }
        }
    }

    /// A fn: its defaults, then its body over its parameters, its implicit
    /// groups' members and (a handler member) the handler's parameters and
    /// state. A comptime-stamped body resolves in its stamp's scope.
    fn func(&mut self, f: &FnDecl, extra: &[Param], state: &[FieldDecl]) {
        let saved = self.at;
        if let Some(&i) = self.fn_stamps.get(&(f as *const FnDecl as usize)) {
            self.at = At::Stamp(i as u32);
        }
        self.defaults(&f.params);
        if let Some(body) = &f.body {
            self.push();
            for p in extra.iter().chain(&f.params) {
                self.bind(&p.name);
            }
            // [implicit-group] A group's members are bound by the spread.
            for g in &f.implicit_groups {
                let members: Vec<String> = self
                    .scope()
                    .fn_groups
                    .get(g.name.name.as_str())
                    .map(|d| d.fns.iter().map(|m| m.name.name.clone()).collect())
                    .unwrap_or_default();
                for m in members {
                    if !self.bound_here(&m) {
                        self.bind_at(&m, g.span);
                    }
                }
            }
            // [cmp-binder] A signature's `?name` binders, bound where the
            // checker's implicits say: the parameter (or return type,
            // constructed claim, reapplied claim) whose type writes it first.
            let mut binders: Vec<(String, Span)> = Vec::new();
            for p in f.params.iter().filter(|p| !p.implicit) {
                type_binders(&p.ty, p.span, &mut binders);
            }
            if let Some(rt) = &f.return_type {
                type_binders(rt, rt.span(), &mut binders);
            }
            if let Some(c) = &f.constructs {
                ref_binders(c, c.span, &mut binders);
            }
            for d in f.deductions.iter().flatten() {
                if let (salvo_syntax::ast::DeductionKind::Exhaustive { reapplied, .. }, Some(_)) = (&d.kind, d.param_name()) {
                    for q in reapplied {
                        ref_binders(q, q.span, &mut binders);
                    }
                }
            }
            for (name, span) in binders {
                if !self.bound_here(&name) {
                    self.bind_at(&name, span);
                }
            }
            for field in state.iter().chain(&f.iter_state) {
                self.bind(&field.name);
            }
            self.block(body);
            self.pop();
        }
        self.at = saved;
    }

    // ------------------------------------------------------------ bodies --

    fn block(&mut self, b: &Block) {
        self.push();
        for s in &b.stmts {
            self.stmt(s);
        }
        self.pop();
    }

    /// A block a construct binds names for: `bound` are in scope inside it.
    fn bound_block(&mut self, bound: &[(String, Span)], b: &Block) {
        self.push();
        for (n, s) in bound {
            self.bind_at(n, *s);
        }
        self.block(b);
        self.pop();
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let { pattern, value, .. } => {
                self.expr(value);
                self.pattern(pattern);
            }
            Stmt::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Stmt::Use { handler, with_items, .. } => {
                self.expr(handler);
                for w in with_items {
                    self.expr(w);
                }
            }
            // [fn-rename] A callable's new name, not a binding.
            Stmt::Rename(_) => {}
            Stmt::Expr(e) => self.expr(e),
            // Expanded before resolution.
            Stmt::Comp(_) => {}
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Ident(id) => self.bind(id),
            Pattern::Tuple { elems, .. } => {
                for e in elems {
                    self.pattern(e);
                }
            }
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    self.bind(&f.binding);
                }
            }
        }
    }

    fn pattern_names(p: &Pattern, out: &mut Vec<(String, Span)>) {
        match p {
            Pattern::Ident(id) => out.push((id.name.clone(), id.span)),
            Pattern::Tuple { elems, .. } => {
                for e in elems {
                    Self::pattern_names(e, out);
                }
            }
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    out.push((f.binding.name.clone(), f.binding.span));
                }
            }
        }
    }

    /// A condition, and the bindings its `is` tests give the branch it
    /// guards (`analyze_cond`'s).
    fn cond(&mut self, c: &Expr) -> Vec<(String, Span)> {
        match c {
            Expr::Is { subject, binding, .. } | Expr::Widen { subject, binding, .. } => {
                self.expr(subject);
                binding.iter().map(|b| (b.name.clone(), b.span)).collect()
            }
            Expr::Binary { op: salvo_syntax::ast::BinaryOp::And, lhs, rhs, .. } => {
                let mut l = self.cond(lhs);
                self.push();
                for (n, s) in &l {
                    self.bind_at(n, *s);
                }
                let r = self.cond(rhs);
                self.pop();
                l.extend(r);
                l
            }
            Expr::Binary { op: salvo_syntax::ast::BinaryOp::Or, lhs, rhs, .. } => {
                self.cond(lhs);
                self.cond(rhs);
                Vec::new()
            }
            Expr::Unary { op: salvo_syntax::ast::UnaryOp::Not, operand, .. } => {
                self.cond(operand);
                Vec::new()
            }
            other => {
                self.expr(other);
                Vec::new()
            }
        }
    }

    fn args(&mut self, args: &[Expr], named: &[salvo_syntax::ast::NamedArg]) {
        for a in args {
            self.expr(a);
        }
        for n in named {
            self.expr(&n.value);
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Placeholder { .. }
            | Expr::Continue { .. }
            | Expr::Error { .. }
            | Expr::FieldDefault { .. }
            | Expr::SelfScoped { .. }
            | Expr::SelfAddr { .. } => {}
            Expr::Str { parts, .. } => {
                for p in parts {
                    if let StrExprPart::Interp(inner) = p {
                        self.expr(inner);
                    }
                }
            }
            Expr::Ident(id) => {
                if id.name != "None" {
                    self.name(id, true);
                }
            }
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => self.expr(base),
            Expr::Scoped { base, name, .. } => {
                if let Some(b) = base {
                    self.expr(b);
                }
                self.name(name, false);
            }
            Expr::EffectScoped { base, name, effect, .. } => {
                if let Some(b) = base {
                    self.expr(b);
                }
                self.name(name, false);
                self.name(effect, false);
            }
            Expr::Call { callee, args, named, .. } => {
                self.expr(callee);
                self.args(args, named);
            }
            Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            Expr::Tuple { elems, .. } => {
                for x in elems {
                    self.expr(x);
                }
            }
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { value, .. } => self.expr(value),
                        StructLitFieldKind::Spread(x) => self.expr(x),
                        // Expanded before resolution.
                        StructLitFieldKind::InlineFor { .. } => {}
                    }
                }
            }
            Expr::Unary { operand, .. } | Expr::IncDec { operand, .. } | Expr::Spread { operand, .. } => self.expr(operand),
            Expr::Binary { lhs, rhs, .. }
            | Expr::Elvis { subject: lhs, rhs, .. }
            | Expr::Pipe { lhs, rhs, .. }
            | Expr::SafePipe { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            // An `is` outside a condition binds nothing.
            Expr::Is { subject, .. } | Expr::Widen { subject, .. } => self.expr(subject),
            Expr::SafeNav { base, access, .. } => {
                self.expr(base);
                if let SafeAccess::Call { args, named, .. } = access {
                    self.args(args, named);
                }
            }
            // [assert-narrow] An assertion narrows what follows; its
            // bindings are not declared.
            Expr::Assert { cond, message, .. } => {
                self.cond(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::WhyNot { test, .. } => self.expr(test),
            Expr::Unreachable { message, .. } | Expr::Todo { message, .. } => {
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::If { branches, else_block, .. } => {
                for (c, b) in branches {
                    let bound = self.cond(c);
                    self.bound_block(&bound, b);
                }
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::WhenCond { branches, else_block, .. } => {
                for (c, b) in branches {
                    let bound = self.cond(c);
                    self.bound_block(&bound, b);
                }
                self.block(else_block);
            }
            Expr::When { binding, subject, branches, .. } => {
                self.expr(subject);
                self.push();
                if let Some(b) = binding {
                    self.bind(b);
                }
                for br in branches {
                    let bound: Vec<(String, Span)> = br.binding.iter().map(|b| (b.name.clone(), b.span)).collect();
                    self.bound_block(&bound, &br.body);
                }
                self.pop();
            }
            Expr::While { cond, body, else_block, .. } => {
                let bound = self.cond(cond);
                self.bound_block(&bound, body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::For { pattern, iterable, body, else_block, .. } => {
                self.expr(iterable);
                let mut bound = Vec::new();
                Self::pattern_names(pattern, &mut bound);
                self.bound_block(&bound, body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::Lambda { params, body, .. } => {
                self.push();
                for p in params {
                    self.bind(&p.name);
                }
                match body {
                    LambdaBody::Expr(x) => self.expr(x),
                    LambdaBody::Block(b) => self.block(b),
                    LambdaBody::Terminal => {}
                }
                self.pop();
            }
            Expr::Try { body, .. } => self.block(body),
            Expr::Return { value, .. } | Expr::Break { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Expr::Spawn { handler, with_items, pool, join, .. } => {
                self.expr(handler);
                for w in with_items {
                    self.expr(w);
                }
                if let Some(p) = pool {
                    self.expr(p);
                }
                if let Some(j) = join {
                    self.expr(j);
                }
            }
            Expr::Use { handler, with_items, .. } => {
                self.expr(handler);
                for w in with_items {
                    self.expr(w);
                }
            }
            Expr::Project { base, .. } => self.expr(base),
            Expr::Merge { items, .. } => {
                for i in items {
                    self.expr(i);
                }
            }
            Expr::ReplyTo { captures, pool, .. } => {
                for c in captures {
                    self.expr(c);
                }
                if let Some(p) = pool {
                    self.expr(p);
                }
            }
            Expr::WaitFor { binding, body, .. } => {
                let bound = vec![(binding.name.clone(), binding.span)];
                self.bound_block(&bound, body);
            }
        }
    }
}

/// [cmp-binder] The `?name` binders a written type holds, each at `at` (the
/// first occurrence of a name wins).
fn type_binders(ty: &salvo_syntax::ast::Type, at: Span, out: &mut Vec<(String, Span)>) {
    use salvo_syntax::visit::Visitor;
    struct Collect<'o>(Span, &'o mut Vec<(String, Span)>);
    impl Visitor for Collect<'_> {
        fn visit_type_ref(&mut self, t: &salvo_syntax::ast::TypeRef) {
            if t.binder && !self.1.iter().any(|(n, _)| *n == t.name.name) {
                self.1.push((t.name.name.clone(), self.0));
            }
        }
    }
    salvo_syntax::visit::walk_type(&mut Collect(at, out), ty);
}

fn ref_binders(r: &salvo_syntax::ast::TypeRef, at: Span, out: &mut Vec<(String, Span)>) {
    use salvo_syntax::visit::Visitor;
    struct Collect<'o>(Span, &'o mut Vec<(String, Span)>);
    impl Visitor for Collect<'_> {
        fn visit_type_ref(&mut self, t: &salvo_syntax::ast::TypeRef) {
            if t.binder && !self.1.iter().any(|(n, _)| *n == t.name.name) {
                self.1.push((t.name.name.clone(), self.0));
            }
        }
    }
    salvo_syntax::visit::walk_type_ref(&mut Collect(at, out), r);
}
