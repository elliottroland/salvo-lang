//! [ir-obligation] The obligation-annotation pass: after the IR is built, walk
//! every fn body and insert `Obligation` directives at the four points where a
//! fn's outstanding linear-obligation set changes (IR_CHECKER.md §3 D3):
//!
//! - `GainParams` at the top of the body: linear parameters moved into the fn.
//! - `Gain` after a binding (`Let`/`Unpack`/`ForEach`) of a linear value.
//! - `Release` after a statement whose expression consumes a linear value
//!   (`Read { consume: true }`): the move hands the obligation on.
//! - `Discharge` where a `Narrow` proves a linear arm absent on a path (the
//!   value is narrowed to a type that drops the linear arm).
//!
//! The directives are what the dump shows and what `ir::check` balances. The
//! pass runs on the built program before the linear check and before emission.

use std::collections::HashSet;

use crate::linear::{linear_arms, linear_type_name, ty_own_linear};
use crate::program::Symbols;
use crate::types::Ty;

use super::nodes::*;

/// Annotate every fn body in `program` with obligation directives.
pub fn annotate(program: &mut Program, symbols: &Symbols) {
    for module in &mut program.modules {
        for decl in &mut module.decls {
            match decl {
                Decl::Fn(f) => annotate_fn(f, symbols),
                Decl::Impl(h) => {
                    if let Some(init) = &mut h.init {
                        annotate_fn(init, symbols);
                    }
                    for m in &mut h.members {
                        annotate_fn(m, symbols);
                    }
                }
                Decl::Static(_) | Decl::Struct(_) | Decl::Union(_) | Decl::Interface(_) | Decl::PlatformType(_) | Decl::Enum(_) => {}
            }
        }
    }
}

fn annotate_fn(f: &mut FnDecl, symbols: &Symbols) {
    let own: HashSet<String> = f.type_params.iter().filter(|p| p.canbe_linear).map(|p| p.name.clone()).collect();
    let Some(body) = &mut f.body else { return };
    // [ref-handle] Parameters not moved in (lent/kept) are borrows: a narrow
    // of one carries no obligation, however linear its type.
    let borrowed: HashSet<String> = f.params.iter().filter(|p| p.mode != PassMode::Moved).map(|p| p.local.0.clone()).collect();
    let tys = std::cell::RefCell::new(f.params.iter().map(|p| (p.local.0.clone(), p.ty.clone())).collect());
    let a = Annotator { symbols, own: &own, borrowed: &borrowed, assigns: &f.fate.assign_spans, state: &f.fate.linear_state, binds: &f.fate.binds, tys };

    // [ir-obligation] GainParams: declared params (not effects/implicits)
    // that are linear and moved into the fn. A lent/kept parameter's
    // obligation stays with the caller.
    let end = f.params.len().saturating_sub(f.implicit_params);
    let declared = &f.params[f.effect_params.min(end)..end];
    let gained: Vec<ObligationRef> = declared
        .iter()
        .filter(|p| p.mode == PassMode::Moved && a.is_linear(&p.ty))
        .map(|p| a.ref_of(Place { root: p.local.clone(), steps: Vec::new() }, &p.ty))
        .collect();
    if !gained.is_empty() {
        body.stmts.insert(0, Stmt::Obligation(Obligation { kind: ObligationKind::GainParams, refs: gained, span: body.span }));
    }

    a.block(body);
}

struct Annotator<'a, 'p> {
    symbols: &'a Symbols<'p>,
    own: &'a HashSet<String>,
    borrowed: &'a HashSet<String>,
    /// The written assignments of a whole local, by the value's span.
    assigns: &'a std::collections::HashMap<(u32, u32), Span>,
    /// [fate-link] The borrow-mode bindings' fate links, by statement id.
    binds: &'a std::collections::HashMap<u32, Vec<crate::check::FlowLink>>,
    /// [linear-state] In a handler member: the state fields holding obligations.
    state: &'a [String],
    /// Each local's type, as its binding declares it (met in order).
    tys: std::cell::RefCell<std::collections::HashMap<String, Ty>>,
}

impl Annotator<'_, '_> {
    fn is_linear(&self, ty: &Ty) -> bool {
        // [proj-type] [ref-handle] A borrow carries no obligation — neither a
        // whole `ref`/`proj` value nor a union all of whose linear arms are
        // borrows (`ref X?` = `ref X | None`).
        if ty.is_proj() {
            return false;
        }
        if matches!(ty.strip_quals(), Ty::Union(_)) {
            return linear_arms(self.symbols, self.own, ty).is_some();
        }
        ty_own_linear(self.symbols, self.own, ty)
    }

    fn ref_of(&self, place: Place, ty: &Ty) -> ObligationRef {
        let arms = linear_arms(self.symbols, self.own, ty);
        // Only a whole-value (non-projection, non-union) linear struct is
        // settled by decomposition; a projection or union carries its own key.
        let container_fields = if arms.is_none() && place.steps.is_empty() {
            crate::linear::linear_fields(self.symbols, self.own, ty)
        } else {
            Vec::new()
        };
        ObligationRef { place, arms, container_fields, ty_name: linear_type_name(self.symbols, self.own, ty), arm_names: crate::linear::linear_arm_names(self.symbols, self.own, ty) }
    }

    /// Rewrite a block's statements, inserting `Gain`/`Release`/`Discharge`
    /// directives around the statements that cause them, and recursing into
    /// nested blocks.
    fn block(&self, block: &mut Block) {
        let mut out: Vec<Stmt> = Vec::with_capacity(block.stmts.len());
        for mut s in std::mem::take(&mut block.stmts) {
            // Recurse into nested blocks first.
            self.recurse(&mut s);

            // A statement whose expression(s) consume a linear value releases
            // it: collect the released places before the statement is moved.
            let releases = self.releases_in(&s);

            match &s {
                Stmt::Let { local, ty, .. } | Stmt::ForEach { local, ty, .. } | Stmt::Narrow { local, ty, .. } | Stmt::Alias { local, ty, .. } => {
                    self.tys.borrow_mut().insert(local.0.clone(), ty.clone());
                }
                _ => {}
            }
            let gain = self.gain_of(&s);
            let span = stmt_span(&s);

            // [ir-obligation] A `Narrow` of a linear source transfers its
            // obligation to the narrowed binding: the source leaves the path's
            // set (its un-narrowed arms are proven absent here), and the
            // narrowed local gains whatever is still linear. A narrow to a
            // wholly non-linear type is a pure discharge of the source.
            if let Stmt::Narrow { local, ty, from, from_ty, .. } = &s {
                // A narrow of a borrowed value (a lent/kept parameter, or a
                // view) transfers no obligation: it is a borrow of a borrow.
                let from_linear = self.is_linear(from_ty) && !self.borrowed.contains(&from.root.0);
                out.push(s.clone());
                if from_linear {
                    let dref = self.ref_of(from.clone(), from_ty);
                    out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Discharge, refs: vec![dref], span }));
                    if self.is_linear(ty) {
                        let gref = self.ref_of(Place { root: local.clone(), steps: Vec::new() }, ty);
                        out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Gain, refs: vec![gref], span }));
                    }
                }
                continue;
            }

            // A leaving statement (`return`/`break`/`continue`) consumes
            // *before* it leaves; and a consuming read inside a branch
            // condition or switch subject is evaluated before the fork — so a
            // statement's releases always precede it, while a `Gain` (a new
            // binding) follows it.
            let leaves = matches!(s, Stmt::Return(..) | Stmt::Break | Stmt::Continue);
            if !releases.is_empty() {
                out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Release, refs: releases, span }));
            }
            out.push(s);
            if !leaves {
                if let Some(refs) = gain {
                    out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Gain, refs, span }));
                }
            }
        }
        block.stmts = out;
        if let Some(v) = &mut block.value {
            self.recurse_expr(v);
            // A consuming read in the block's tail value releases here — the
            // value (and its obligation) flows out to whatever uses the block.
            let mut refs = Vec::new();
            walk_expr(v, &mut |e| self.collect_consumes(e, &mut refs));
            if !refs.is_empty() {
                let span = v.span;
                block.stmts.push(Stmt::Obligation(Obligation { kind: ObligationKind::Release, refs, span }));
            }
        }
    }

    /// Recurse into the nested blocks a statement contains.
    fn recurse(&self, s: &mut Stmt) {
        match s {
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => self.block(body),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value), _) => {
                self.recurse_expr(value)
            }
            _ => {}
        }
    }

    fn recurse_expr(&self, e: &mut Expr) {
        match &mut e.kind {
            ExprKind::Branch { arms, otherwise, .. } => {
                for (cond, body) in arms.iter_mut() {
                    self.recurse_expr(cond);
                    self.block(body);
                }
                if let Some(b) = otherwise {
                    self.block(b);
                }
            }
            ExprKind::Switch { subject, arms, .. } => {
                self.recurse_expr(subject);
                for arm in arms.iter_mut() {
                    self.block(&mut arm.body);
                }
            }
            ExprKind::Test { subject, .. } => self.recurse_expr(subject),
            ExprKind::Try { body } => self.block(body),
            ExprKind::Lambda { body, .. } => self.block(body),
            // [actor-waitfor] A `waitfor` mints a linear reply token the body
            // must send to; it is gained at the body's start and must be
            // settled before the body ends.
            ExprKind::WaitFor { local, token_ty, body } => {
                self.block(body);
                if self.is_linear(token_ty) {
                    let r = self.ref_of(Place { root: local.clone(), steps: Vec::new() }, token_ty);
                    body.stmts.insert(0, Stmt::Obligation(Obligation { kind: ObligationKind::Gain, refs: vec![r], span: body.span }));
                }
            }
            _ => {}
        }
    }

    /// Whether a binding's value is a **borrow** of a linear place rather than
    /// an owned value: a non-consuming `Read` whose read type is a `proj`
    /// borrow, or whose place is rooted in a lent/kept local (`self.borrowed`)
    /// — reading a field of a borrowed linear struct yields a borrow, which
    /// owes nothing. A consuming read (`!read`, a move out) owns; a call or
    /// construction produces an owned value.
    fn value_is_borrow(&self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Read { place, consume: false, .. } => value.ty.is_proj() || self.borrowed.contains(&place.root.0),
            _ => false,
        }
    }

    /// [linear-obligation] A borrow-mode binding fate-linked to the value it
    /// was bound from (`let alias = h`, `FateFacts::binds`) is an
    /// alias, and owes nothing: the root keeps the obligation.
    fn derived(&self, id: NodeId) -> bool {
        self.binds.get(&id.0).is_some_and(|links| !links.is_empty())
    }

    /// `Gain` for a binding of a linear value.
    fn gain_of(&self, s: &Stmt) -> Option<Vec<ObligationRef>> {
        match s {
            // A binding of a linear value gains its obligation — unless the
            // value is a *borrow* of a linear place (a non-consuming read of a
            // field of a lent/kept root, or a `proj` read): a borrow owns
            // nothing, the same rule a `Narrow` of a borrowed source obeys.
            Stmt::Let { id, local, ty, value } if self.is_linear(ty) && !self.value_is_borrow(value) && !self.derived(*id) => {
                Some(vec![self.ref_of(Place { root: local.clone(), steps: Vec::new() }, ty)])
            }
            // [linear-obligation] A written assignment of an owned linear
            // value to a whole local gains it, as its binding did (the
            // obligation is the local's type's: a `T?` local owes as `T`).
            Stmt::Assign { place, value }
                if place.steps.is_empty()
                    && self.assigns.contains_key(&(value.span.start, value.span.end))
                    // [linear-state] A state field's value belongs to the
                    // handler, which owes it across activations.
                    && !self.state.contains(&place.root.0)
                    && self.is_linear(&value.ty)
                    && !self.value_is_borrow(value) =>
            {
                let ty = self.tys.borrow().get(&place.root.0).cloned().unwrap_or_else(|| value.ty.clone());
                let ty = if self.is_linear(&ty) { ty } else { value.ty.clone() };
                Some(vec![self.ref_of(place.clone(), &ty)])
            }
            Stmt::ForEach { id, local, ty, .. } if self.is_linear(ty) && !self.derived(*id) => {
                Some(vec![self.ref_of(Place { root: local.clone(), steps: Vec::new() }, ty)])
            }
            Stmt::Unpack { locals, .. } => {
                let refs: Vec<ObligationRef> = locals
                    .iter()
                    .filter(|(_, t)| self.is_linear(t))
                    .map(|(l, t)| self.ref_of(Place { root: l.clone(), steps: Vec::new() }, t))
                    .collect();
                (!refs.is_empty()).then_some(refs)
            }
            _ => None,
        }
    }

    /// The linear places a statement's expressions consume (`Read { consume:
    /// true }` of a linear value). These release their obligation.
    fn releases_in(&self, s: &Stmt) -> Vec<ObligationRef> {
        let mut refs = Vec::new();
        each_expr(s, &mut |e| self.collect_consumes(e, &mut refs));
        refs
    }

    fn collect_consumes(&self, e: &Expr, out: &mut Vec<ObligationRef>) {
        if let ExprKind::Read { place, consume: true, .. } = &e.kind {
            if self.is_linear(&e.ty) {
                out.push(self.ref_of(place.clone(), &e.ty));
            }
        }
    }
}

/// A rough span for a statement (for the directive's own span).
fn stmt_span(s: &Stmt) -> Span {
    match s {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value), _) => value.span,
        Stmt::ForEach { iterable, .. } => iterable.span,
        _ => Span::default(),
    }
}

/// Visit the top-level expressions a statement holds (not recursing into
/// nested blocks — those are handled by `block`). The consuming reads that
/// matter for a Release sit in the statement's own expression tree.
fn each_expr(s: &Stmt, f: &mut impl FnMut(&Expr)) {
    match s {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value), _) => walk_expr(value, f),
        Stmt::ForEach { iterable, .. } => walk_expr(iterable, f),
        _ => {}
    }
}

/// Walk an expression tree, visiting every sub-expression, but NOT descending
/// into nested blocks (branch/switch/lambda bodies) — their reads belong to
/// their own path and are released there.
fn walk_expr(e: &Expr, f: &mut impl FnMut(&Expr)) {
    f(e);
    match &e.kind {
        ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::Tuple(args) | ExprKind::List(args) | ExprKind::Array(args) | ExprKind::Concat(args) => {
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::MemberCall { instance, args, .. } => {
            walk_expr(instance, f);
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                walk_expr(v, f);
            }
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::Rewrap { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::BorrowAs { value, .. }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value } => walk_expr(value, f),
        // Actor forms carry consuming reads of linear values (a `Reply` token
        // sent, a message's payload) in their operands.
        ExprKind::Send { addr, args, .. } => {
            walk_expr(addr, f);
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::SelfSend { args, .. } => {
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::HandlerCall { instance, args, .. } => {
            walk_expr(instance, f);
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::ReplyTo { captures, pool, .. } => {
            for c in captures {
                walk_expr(c, f);
            }
            if let Some(p) = pool {
                walk_expr(p, f);
            }
        }
        ExprKind::Spawn { handler, deps, pool, join, .. } => {
            walk_expr(handler, f);
            for d in deps {
                walk_expr(d, f);
            }
            if let Some(p) = pool {
                walk_expr(p, f);
            }
            if let Some(j) = join {
                walk_expr(j, f);
            }
        }
        ExprKind::UnionToStr { value, .. } | ExprKind::AddrInstance { addr: value } | ExprKind::Handle { instance: value } => walk_expr(value, f),
        // A branch's conditions and a switch's subject are evaluated on the
        // *current* path (before the fork), so their consuming reads release
        // here. Arm bodies are separate paths, handled by `block` recursion —
        // do not descend into them.
        ExprKind::Branch { arms, .. } => {
            for (cond, _body) in arms {
                walk_expr(cond, f);
            }
        }
        ExprKind::Switch { subject, .. } => walk_expr(subject, f),
        _ => {}
    }
}
