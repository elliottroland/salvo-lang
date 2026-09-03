//! Deduction inference and validation [deduce-syntax] [deduce-infer].
//!
//! A function's deduction list (`-> [list: Mut] T`) states what a call
//! does to each parameter: a listed parameter is given back to the caller
//! (borrowed) with the stated qualifiers still known; a parameter omitted
//! from a *written* list is moved.
//!
//! Each entry has a polarity [deduce-syntax]: plain qualifier names are
//! *exhaustive* (only those survive — including qualifiers the callee
//! never declared), `-Q` is a *delta* (drop `Q`, keep the rest), a bare
//! entry keeps everything, and `Nothing` means moved. The removal set is
//! computed at the call site against the qualifiers the *argument*
//! carries, which is what makes the exhaustive form sound: a function
//! that mutates a value can invalidate claims about its contents that its
//! signature never mentions, so such a parameter may not keep
//! "everything else" — the bare and delta forms are rejected there. A
//! bodyless fn has no inference at all: it must declare its effects,
//! deductions, and return type [decl-explicit], so there is nothing to
//! approximate.
//!
//! An unwritten list is inferred as the strictest deduction over all uses
//! of each parameter in the body (including moves), iterated to a
//! fixpoint over the call graph: inference starts optimistic (keep-all)
//! and facts only shrink along `KeepAll` → `Remove` (growing) →
//! `Exhaustive` (shrinking), so the iteration terminates. Exhaustiveness
//! is contagious: handing a parameter to an exhaustive callee makes the
//! caller's own entry exhaustive. Written lists are validated against the
//! same body facts: promising a parameter back that the body moves, or a
//! qualifier the body may remove, is an error.
//!
//! The Kotlin backend ignores deductions; they are the Rust backend's
//! ownership/borrow contract, computed and stored here so the typed IR
//! carries them.
//!
//! Lenient like the rest of the checker: uses that cannot be resolved
//! (backend interop, effect-member calls, non-`fn` callees) are treated
//! as plain borrows that preserve all qualifiers. Value flow of a bare
//! parameter out of a branch/loop tail is not tracked as a move yet.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{
    Block, Deduction, DeductionKind, Expr, FnDecl, Item, LambdaBody, Param, Stmt,
    StrExprPart, StructLitFieldKind, Type,
};
use salvo_syntax::Span;

use crate::check::{Checked, Key};
use crate::types::QualEffect;
use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::resolve::FnKey;

/// The deduction facts for one parameter [deduce-syntax].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDeduction {
    pub param: String,
    /// False when a call moves the parameter (omitted from a written
    /// deduction list, written as `[p: Nothing]`, or inferred as moved).
    pub kept: bool,
    /// What a call does to the *argument's* known qualifiers. Only
    /// meaningful when `kept`.
    pub effect: QualEffect,
}

/// One function participating in inference.
struct FnInfo<'p> {
    key: FnKey,
    decl: &'p FnDecl,
}

/// Computes (or validates) the deduction list of every top-level `fn` and
/// stores the result in `checked.deductions`. `claims` carries parameters
/// the checker saw claimed by move-mode bindings and moved-position
/// projections [fate-move-mode]: the binding takes ownership of data
/// reached through the parameter, so the fn demands ownership from its
/// callers — the claimed parameter is moved. Claims only exist for fns
/// without a written list, and are applied after every body inference
/// (facts only disappear, so the fixpoint still terminates).
pub(crate) fn infer(
    program: &Program,
    checked: &mut Checked,
    claims: &HashMap<FnKey, HashSet<String>>,
    mutations: &HashMap<FnKey, HashSet<String>>,
) {
    let no_mutations: HashSet<String> = HashSet::new();
    let mut fns: Vec<FnInfo<'_>> = Vec::new();
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for (item_idx, item) in ast.items.iter().enumerate() {
            if let Item::Fn(f) = item {
                fns.push(FnInfo {
                    key: FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    decl: f,
                });
            }
        }
    }
    let fn_decls: HashMap<FnKey, &FnDecl> = fns.iter().map(|f| (f.key, f.decl)).collect();

    // Initial state: written lists as declared (validating their shape),
    // unwritten lists optimistic (everything kept with declared quals).
    let mut errors: Vec<FileDiagnostic> = Vec::new();
    let mut states: HashMap<FnKey, Vec<ParamDeduction>> = HashMap::new();
    for f in &fns {
        let mutated = mutations.get(&f.key).unwrap_or(&no_mutations);
        let state = match &f.decl.deductions {
            Some(list) => from_written(f.decl, list, mutated, |span, msg| {
                errors.push(FileDiagnostic::error(f.key.file, span, msg));
            }),
            None => {
                let mut state = optimistic(f.decl);
                // [deduce-syntax] A mutated parameter cannot keep
                // everything: state its declared set exhaustively.
                exhaustive_for_mutated(f.decl, &mut state, mutated);
                apply_claims(&mut state, claims.get(&f.key));
                state
            }
        };
        states.insert(f.key, state);
    }

    // Fixpoint over the call graph for the inferred (unwritten) lists.
    // Constraints are monotone (facts only disappear), so this terminates.
    loop {
        let mut changed = false;
        for f in &fns {
            if f.decl.deductions.is_some() {
                continue;
            }
            let Some(body) = &f.decl.body else { continue };
            let mut new = infer_body(f, body, &checked.call_fn, &fn_decls, &states);
            exhaustive_for_mutated(
                f.decl,
                &mut new,
                mutations.get(&f.key).unwrap_or(&no_mutations),
            );
            apply_claims(&mut new, claims.get(&f.key));
            if states.get(&f.key) != Some(&new) {
                states.insert(f.key, new);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Validate written lists against the body facts [deduce-infer]: the
    // contract may be *stricter* than the body (dropping qualifiers or
    // moving parameters the body gives back), never looser.
    for f in &fns {
        let (Some(list), Some(body)) = (&f.decl.deductions, &f.decl.body) else {
            continue;
        };
        let inferred = infer_body(f, body, &checked.call_fn, &fn_decls, &states);
        let written = &states[&f.key];
        for ((w, i), p) in written.iter().zip(&inferred).zip(&f.decl.params) {
            let Some(entry) = list.iter().find(|d| d.param.name == w.param) else {
                continue;
            };
            if w.kept && !i.kept {
                errors.push(FileDiagnostic::error(
                    f.key.file,
                    entry.span,
                    format!(
                        "deduction promises `{}` back to the caller, but the \
                         body moves it",
                        w.param
                    ),
                ));
                continue;
            }
            let declared = declared_quals(&p.ty);
            let promised = w.effect.kept_quals(&declared);
            let survives = i.effect.kept_quals(&declared);
            for q in &promised {
                if w.kept && !survives.contains(q) {
                    errors.push(FileDiagnostic::error(
                        f.key.file,
                        entry.span,
                        format!(
                            "deduction promises qualifier `{q}` on `{}`, but \
                             the body may remove it",
                            w.param
                        ),
                    ));
                }
            }
        }
    }

    checked.errors.extend(errors);
    checked.deductions = states;
}

/// [deduce-syntax] Forces the exhaustive form on every parameter the body
/// invalidates: keeping "everything else" is exactly the unsound claim,
/// because mutation can falsify qualifiers the caller has and this
/// signature never mentions. The surviving set is what inference already
/// computed from the declared qualifiers.
fn exhaustive_for_mutated(
    decl: &FnDecl,
    state: &mut [ParamDeduction],
    mutated: &HashSet<String>,
) {
    for (d, p) in state.iter_mut().zip(&decl.params) {
        if !d.kept || !mutated.contains(&d.param) {
            continue;
        }
        let declared = declared_quals(&p.ty);
        let keep = d.effect.kept_quals(&declared);
        d.effect = QualEffect::Exhaustive(keep);
    }
}

/// The qualifier names written on the outside of a parameter type, in
/// declaration order (`Mut NonEmpty List<T>` -> `["Mut", "NonEmpty"]`).
pub fn declared_quals(ty: &Type) -> Vec<String> {
    match ty {
        Type::Named { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        Type::Nullable { inner, .. } => declared_quals(inner),
        _ => Vec::new(),
    }
}

/// Everything kept with nothing stripped: the optimistic starting point of
/// inference. Bodyless fns never reach it — they must declare their lists
/// [decl-explicit].
pub(crate) fn optimistic(decl: &FnDecl) -> Vec<ParamDeduction> {
    decl.params
        .iter()
        .map(|p| ParamDeduction {
            param: p.name.name.clone(),
            kept: true,
            effect: QualEffect::KeepAll,
        })
        .collect()
}

/// A written deduction list, validated for shape: entries must name a
/// parameter (once), exhaustive entries may only keep qualifiers declared
/// on that parameter, and `Nothing` is the only type form [deduce-syntax].
/// `mutated` names the parameters the body invalidates; for those, only
/// the exhaustive form is sound (see the soundness rule under D1).
pub(crate) fn from_written(
    decl: &FnDecl,
    list: &[Deduction],
    mutated: &HashSet<String>,
    mut error: impl FnMut(Span, String),
) -> Vec<ParamDeduction> {
    for (i, d) in list.iter().enumerate() {
        if !decl.params.iter().any(|p| p.name.name == d.param.name) {
            error(
                d.span,
                format!("deduction names unknown parameter `{}`", d.param.name),
            );
        }
        if list[..i].iter().any(|prev| prev.param.name == d.param.name) {
            error(
                d.span,
                format!("duplicate deduction for parameter `{}`", d.param.name),
            );
        }
    }
    decl.params
        .iter()
        .map(|p| {
            let declared = declared_quals(&p.ty);
            let name = p.name.name.clone();
            let Some(d) = list.iter().find(|d| d.param.name == name) else {
                return ParamDeduction {
                    param: name,
                    kept: false,
                    effect: QualEffect::Exhaustive(Vec::new()),
                };
            };
            let invalidates = mutated.contains(&name);
            let effect = match &d.kind {
                DeductionKind::Moved => {
                    return ParamDeduction {
                        param: name,
                        kept: false,
                        effect: QualEffect::Exhaustive(Vec::new()),
                    };
                }
                DeductionKind::KeepAll => {
                    if invalidates {
                        error(
                            d.span,
                            format!(
                                "`{name}` is mutated by this function, so the \
                                 deduction cannot keep every qualifier: mutation \
                                 can invalidate qualifiers the caller has and \
                                 this signature never mentions. List exactly \
                                 what survives (`[{name}: {}]`)",
                                declared.join(" ")
                            ),
                        );
                    }
                    QualEffect::KeepAll
                }
                DeductionKind::Remove(items) => {
                    if invalidates {
                        error(
                            d.span,
                            format!(
                                "`{name}` is mutated by this function, so the \
                                 deduction cannot drop qualifiers selectively: \
                                 mutation can invalidate qualifiers the caller \
                                 has and this signature never mentions. List \
                                 exactly what survives (`[{name}: {}]`)",
                                declared.join(" ")
                            ),
                        );
                    }
                    let mut drop = Vec::new();
                    for q in items {
                        if !q.args.is_empty() {
                            error(
                                q.span,
                                "a deduction entry may only name qualifiers (and \
                                 `Nothing`): type narrowing in deductions is not \
                                 supported yet"
                                    .to_string(),
                            );
                            continue;
                        }
                        drop.push(q.name.name.clone());
                    }
                    QualEffect::Remove(drop)
                }
                DeductionKind::Exhaustive(items) => {
                    let mut keep = Vec::new();
                    for q in items {
                        if !q.args.is_empty() {
                            error(
                                q.span,
                                "a deduction entry may only name qualifiers (and \
                                 `Nothing`): type narrowing in deductions is not \
                                 supported yet"
                                    .to_string(),
                            );
                            continue;
                        }
                        if !declared.contains(&q.name.name) {
                            error(
                                q.span,
                                format!(
                                    "deduction keeps qualifier `{}`, which is not \
                                     declared on parameter `{name}` (a deduction \
                                     may preserve or drop qualifiers, not add \
                                     them)",
                                    q.name.name
                                ),
                            );
                        } else {
                            keep.push(q.name.name.clone());
                        }
                    }
                    QualEffect::Exhaustive(keep)
                }
            };
            ParamDeduction {
                param: name,
                kept: true,
                effect,
            }
        })
        .collect()
}

/// Re-derives one fn's deduction facts from its body, given the current
/// state of every other fn [deduce-infer].
fn infer_body(
    f: &FnInfo<'_>,
    body: &Block,
    call_fn: &HashMap<Key, FnKey>,
    fn_decls: &HashMap<FnKey, &FnDecl>,
    states: &HashMap<FnKey, Vec<ParamDeduction>>,
) -> Vec<ParamDeduction> {
    let mut walk = Walk {
        file: f.key.file,
        call_fn,
        fn_decls,
        states,
        params: optimistic(f.decl),
        decl: f.decl,
    };
    walk.block(body);
    walk.params
}

/// The body walker applying use-constraints to the parameter facts.
struct Walk<'a, 'p> {
    file: usize,
    call_fn: &'a HashMap<Key, FnKey>,
    fn_decls: &'a HashMap<FnKey, &'p FnDecl>,
    states: &'a HashMap<FnKey, Vec<ParamDeduction>>,
    params: Vec<ParamDeduction>,
    /// The declaration being walked (for its fn-typed parameters'
    /// written contracts [fn-contract]).
    decl: &'p FnDecl,
}

/// The parameter an argument passes *itself* (spreads forward the value
/// whole). Shadowing is rejected by the checker, so a name match is the
/// parameter.
fn bare_ident(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident(id) => Some(&id.name),
        Expr::Spread { operand, .. } => bare_ident(operand),
        _ => None,
    }
}

/// Marks every claimed parameter as moved [fate-move-mode]: a move-mode
/// binding (or moved-position projection) took ownership of data reached
/// through it, so the fn demands ownership from its callers.
fn apply_claims(state: &mut [ParamDeduction], claims: Option<&HashSet<String>>) {
    let Some(claims) = claims else { return };
    for p in state.iter_mut() {
        if claims.contains(&p.param) {
            p.kept = false;
            p.effect = QualEffect::Exhaustive(Vec::new());
        }
    }
}

/// The callee parameter an argument position binds to (trailing arguments
/// bind the variadic parameter) [fn-variadic].
fn param_for_arg(params: &[Param], i: usize) -> Option<usize> {
    if i < params.len() {
        return Some(i);
    }
    match params.last() {
        Some(p) if p.variadic => Some(params.len() - 1),
        _ => None,
    }
}

impl Walk<'_, '_> {
    fn mark_moved(&mut self, name: &str) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.kept = false;
            p.effect = QualEffect::Exhaustive(Vec::new());
        }
    }

    /// A use that drops specific qualifiers: the parameter's effect keeps
    /// its polarity and grows the removal set [deduce-syntax].
    fn remove_quals(&mut self, name: &str, removed: &[String]) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.effect = match &p.effect {
                QualEffect::KeepAll => QualEffect::Remove(removed.to_vec()),
                QualEffect::Remove(have) => {
                    let mut have = have.clone();
                    for q in removed {
                        if !have.contains(q) {
                            have.push(q.clone());
                        }
                    }
                    QualEffect::Remove(have)
                }
                QualEffect::Exhaustive(keep) => QualEffect::Exhaustive(
                    keep.iter().filter(|q| !removed.contains(q)).cloned().collect(),
                ),
            };
        }
    }

    /// A use that leaves *only* `allowed` known (the callee's list is
    /// exhaustive): exhaustiveness is contagious through the call graph —
    /// a fn that hands its parameter to an exhaustive callee can no longer
    /// promise qualifiers of its own caller either [deduce-syntax].
    fn restrict_to(&mut self, name: &str, allowed: &[String]) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            let keep: Vec<String> = match &p.effect {
                QualEffect::KeepAll => allowed.to_vec(),
                QualEffect::Exhaustive(keep) => keep
                    .iter()
                    .filter(|q| allowed.contains(q))
                    .cloned()
                    .collect(),
                QualEffect::Remove(dropped) => allowed
                    .iter()
                    .filter(|q| !dropped.contains(q))
                    .cloned()
                    .collect(),
            };
            p.effect = QualEffect::Exhaustive(keep);
        }
    }

    fn block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // Binding a bare parameter (or a projection of one) to a
            // variable *links* the two — shared fate [fate-link] — it
            // does not move the parameter. Sound because a derived
            // variable is read-only [fate-derived-readonly]: every way
            // its value could escape the function is a checker error
            // until `copy` makes it independent.
            Stmt::Let { value, .. } => {
                if bare_ident(value).is_none() {
                    self.expr(value);
                }
            }
            Stmt::Assign { target, value, .. } => {
                self.expr(target);
                if bare_ident(value).is_none() {
                    self.expr(value);
                }
            }
            Stmt::Return { value: Some(v), .. }
            | Stmt::Break { value: Some(v), .. }
            | Stmt::Yield { value: v, .. } => self.moving_expr(v),
            Stmt::Use { handler, .. } => {
                // Handler constructor arguments are stored in the handler.
                if let Expr::Call { args, .. } = handler {
                    for a in args {
                        self.moving_expr(a);
                    }
                } else {
                    self.expr(handler);
                }
            }
            Stmt::Expr(e) => self.expr(e),
            _ => {}
        }
    }

    /// An expression whose value flows somewhere the caller keeps:
    /// passing a bare parameter here moves it.
    fn moving_expr(&mut self, e: &Expr) {
        match bare_ident(e) {
            Some(name) => {
                let name = name.to_string();
                self.mark_moved(&name);
            }
            None => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Call {
                callee, args, span, ..
            } => self.call(callee, args, *span),
            // Struct/array/tuple construction stores the value.
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { value, .. } => self.moving_expr(value),
                        StructLitFieldKind::Spread(v) => self.moving_expr(v),
                    }
                }
            }
            Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
                for el in elems {
                    self.moving_expr(el);
                }
            }
            Expr::ArrayInit { size, init, .. } => {
                self.expr(size);
                self.expr(init);
            }
            Expr::Str { parts, .. } => {
                for p in parts {
                    if let StrExprPart::Interp(i) = p {
                        self.expr(i);
                    }
                }
            }
            Expr::Field { base, .. } => self.expr(base),
            Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            Expr::Unary { operand, .. }
            | Expr::NonNull { operand, .. }
            | Expr::PostIncrement { operand, .. }
            | Expr::Spread { operand, .. } => self.expr(operand),
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Is { subject, .. } => self.expr(subject),
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.block(b);
                }
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::When {
                subject, branches, ..
            } => {
                self.expr(subject);
                for b in branches {
                    self.block(&b.body);
                }
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                self.expr(cond);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                self.expr(iterable);
                self.block(body);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            // Uses inside a lambda body count like any other use (the
            // lambda may run any number of times).
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(e) => self.expr(e),
                LambdaBody::Block(b) => self.block(b),
            },
            _ => {}
        }
    }

    /// A call site: bare-parameter arguments take the callee's deduction —
    /// moved when the callee moves them, otherwise stripped of exactly the
    /// callee's removal set (declared − kept) [deduce-syntax]. Unresolved
    /// callees (interop, effect members) borrow and preserve everything.
    fn call(&mut self, callee: &Expr, args: &[Expr], span: Span) {
        // Dot notation: the receiver is argument 0 [fn-dot].
        let mut arg_exprs: Vec<&Expr> = Vec::new();
        match callee {
            Expr::Field { base, .. } => arg_exprs.push(base),
            Expr::Ident(_) => {}
            other => self.expr(other),
        }
        arg_exprs.extend(args.iter());

        let resolved = self
            .call_fn
            .get(&(self.file, span))
            .and_then(|key| Some((self.fn_decls.get(key)?, self.states.get(key)?)));
        let Some((callee_decl, callee_state)) = resolved else {
            // [fn-contract] A call through a fn-typed *parameter* of the
            // walking fn applies that parameter's written fn-type
            // contract: consumed positions move bare-parameter
            // arguments. Default (no written list): keeps everything.
            if let Expr::Ident(callee_id) = callee {
                let fn_param_ty = self
                    .decl
                    .params
                    .iter()
                    .find(|p| p.name.name == callee_id.name)
                    .map(|p| &p.ty);
                if let Some(Type::Fn {
                    param_names,
                    deductions: Some(list),
                    ..
                }) = fn_param_ty
                {
                    for (i, a) in arg_exprs.into_iter().enumerate() {
                        let Some(name) = bare_ident(a) else {
                            self.expr(a);
                            continue;
                        };
                        let kept = param_names
                            .get(i)
                            .and_then(|n| n.as_ref())
                            .map(|n| list.iter().any(|d| d.param.name == n.name))
                            .unwrap_or(true);
                        if !kept {
                            let name = name.to_string();
                            self.mark_moved(&name);
                        }
                    }
                    return;
                }
            }
            for a in arg_exprs {
                self.expr(a);
            }
            return;
        };
        for (i, a) in arg_exprs.into_iter().enumerate() {
            let Some(name) = bare_ident(a) else {
                self.expr(a);
                continue;
            };
            let name = name.to_string();
            let Some(pidx) = param_for_arg(&callee_decl.params, i) else {
                continue;
            };
            let ded = &callee_state[pidx];
            if !ded.kept {
                self.mark_moved(&name);
                continue;
            }
            match ded.effect.clone() {
                QualEffect::KeepAll => {}
                QualEffect::Remove(dropped) => self.remove_quals(&name, &dropped),
                QualEffect::Exhaustive(keep) => self.restrict_to(&name, &keep),
            }
        }
    }
}
