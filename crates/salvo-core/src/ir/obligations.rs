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

use crate::linear::{linear_arms, ty_own_linear};
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
    let a = Annotator { symbols, own: &own };

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
}

impl Annotator<'_, '_> {
    fn is_linear(&self, ty: &Ty) -> bool {
        // [proj-type] [ref-handle] A borrow (`proj`/`ref`) carries no
        // obligation — it is a view into a value owned elsewhere, like the
        // checker's `links` fate.
        if ty.is_proj() {
            return false;
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
        ObligationRef { place, arms, container_fields }
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

            let gain = self.gain_of(&s);
            let span = stmt_span(&s);

            // [ir-obligation] A `Narrow` of a linear source transfers its
            // obligation to the narrowed binding: the source leaves the path's
            // set (its un-narrowed arms are proven absent here), and the
            // narrowed local gains whatever is still linear. A narrow to a
            // wholly non-linear type is a pure discharge of the source.
            if let Stmt::Narrow { local, ty, from, from_ty, .. } = &s {
                let from_linear = self.is_linear(from_ty);
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
            // *before* it leaves, so Release precedes it; every other
            // statement's effects follow it, in evaluation order.
            let leaves = matches!(s, Stmt::Return(_) | Stmt::Break | Stmt::Continue);
            if leaves {
                if !releases.is_empty() {
                    out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Release, refs: releases, span }));
                }
                out.push(s);
            } else {
                out.push(s);
                if let Some(refs) = gain {
                    out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Gain, refs, span }));
                }
                if !releases.is_empty() {
                    out.push(Stmt::Obligation(Obligation { kind: ObligationKind::Release, refs: releases, span }));
                }
            }
        }
        block.stmts = out;
        if let Some(v) = &mut block.value {
            self.recurse_expr(v);
        }
    }

    /// Recurse into the nested blocks a statement contains.
    fn recurse(&self, s: &mut Stmt) {
        match s {
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => self.block(body),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value)) => {
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
            _ => {}
        }
    }

    /// `Gain` for a binding of a linear value.
    fn gain_of(&self, s: &Stmt) -> Option<Vec<ObligationRef>> {
        match s {
            Stmt::Let { local, ty, .. } if self.is_linear(ty) => {
                Some(vec![self.ref_of(Place { root: local.clone(), steps: Vec::new() }, ty)])
            }
            Stmt::ForEach { local, ty, .. } if self.is_linear(ty) => {
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
        if let ExprKind::Read { place, consume: true } = &e.kind {
            if self.is_linear(&e.ty) {
                out.push(self.ref_of(place.clone(), &e.ty));
            }
        }
    }
}

/// A rough span for a statement (for the directive's own span).
fn stmt_span(s: &Stmt) -> Span {
    match s {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value)) => value.span,
        Stmt::ForEach { iterable, .. } => iterable.span,
        _ => Span::default(),
    }
}

/// Visit the top-level expressions a statement holds (not recursing into
/// nested blocks — those are handled by `block`). The consuming reads that
/// matter for a Release sit in the statement's own expression tree.
fn each_expr(s: &Stmt, f: &mut impl FnMut(&Expr)) {
    match s {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value)) => walk_expr(value, f),
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
        _ => {}
    }
}
