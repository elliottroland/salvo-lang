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
    Block, Deduction, DeductionKind, EffectDecl, Expr, FnDecl, Item, LambdaBody, Param,
    QualSubject, Stmt, StrExprPart, StructLitFieldKind, Type,
};
use salvo_syntax::Span;

use crate::check::{Checked, Key};
use crate::types::{QualEffect, Ty};
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
    let no_state: HashSet<String> = HashSet::new();
    // [qual-subject] Provenance qualifiers survive every call, so an
    // inferred entry must not claim to remove one. Collected program-wide:
    // the deduction machinery is qualifier-*name* keyed throughout, and
    // [mod-collision] already rejects one name declared by two visible
    // modules.
    let provenance: HashSet<String> = program
        .modules
        .iter()
        .flat_map(|ast| ast.items.iter())
        .filter_map(|item| match item {
            Item::Qualifier(q) if q.subject == QualSubject::Provenance => {
                Some(q.name.name.clone())
            }
            _ => None,
        })
        .collect();
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
    // [call-resolve] Effect members have no `FnKey` — the handler is picked
    // at run time — but they *declare* their deductions ([decl-explicit]),
    // and the checker recorded which effect instance each member call
    // dispatches through. Together those give the callee contract, so a
    // member that takes ownership is seen as moving its argument here too
    // (it already does at the call site, in `check_effect_call`).
    // Effect names are program-wide keys, like the qualifier names this
    // pass already uses: [mod-collision] rejects one name declared by two
    // visible modules.
    let effects: HashMap<String, &EffectDecl> = program
        .modules
        .iter()
        .flat_map(|ast| ast.items.iter())
        .filter_map(|item| match item {
            Item::Effect(e) => Some((e.name.name.clone(), e)),
            _ => None,
        })
        .collect();
    let effect_calls = checked.effect_calls.clone();

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
            let mut new = infer_body(
                f,
                body,
                &checked.call_fn,
                &fn_decls,
                &states,
                &provenance,
                &effects,
                &effect_calls,
                &no_state,
                &checked.refinements,
            );
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
        let inferred = infer_body(
            f,
            body,
            &checked.call_fn,
            &fn_decls,
            &states,
            &provenance,
            &effects,
            &effect_calls,
            &no_state,
            &checked.refinements,
        );
        let written = &states[&f.key];
        validate_written(f.key.file, f.decl, list, written, &inferred, &mut errors);
    }

    // [effect-handler] Handler members are declarations with bodies too, and
    // their contract is applied at call sites like a named call's
    // ([decl-explicit], [call-resolve]) — so a written list that promises
    // more than the body delivers is the same bug there. They carry no
    // `FnKey` (not top-level items), so they are validated here instead of
    // joining the fixpoint, which is sound because the dependency runs one
    // way: a member's body facts depend on other fns' contracts, and a fn's
    // contract depends on members' *declared* lists, never their bodies.
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for (item_idx, item) in ast.items.iter().enumerate() {
            let Item::Handler(h) = item else { continue };
            for m in &h.fns {
                let (Some(list), Some(body)) = (&m.deductions, &m.body) else {
                    continue;
                };
                let info = FnInfo {
                    key: FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    decl: m,
                };
                let written = from_written(m, list, &no_mutations, |_, _| {});
                let state_names: HashSet<String> =
                    h.state.iter().map(|s| s.name.name.clone()).collect();
                let inferred = infer_body(
                    &info,
                    body,
                    &checked.call_fn,
                    &fn_decls,
                    &states,
                    &provenance,
                    &effects,
                    &effect_calls,
                    &state_names,
                    &checked.refinements,
                );
                validate_written(file_idx, m, list, &written, &inferred, &mut errors);
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
#[allow(clippy::too_many_arguments)]
fn infer_body<'a, 'p>(
    f: &FnInfo<'p>,
    body: &'p Block,
    call_fn: &'a HashMap<Key, FnKey>,
    fn_decls: &'a HashMap<FnKey, &'p FnDecl>,
    states: &'a HashMap<FnKey, Vec<ParamDeduction>>,
    provenance: &'a HashSet<String>,
    effects: &'a HashMap<String, &'p EffectDecl>,
    effect_calls: &'a HashMap<Key, Ty>,
    state_names: &'a HashSet<String>,
    refinements: &'a crate::refine::Refinements,
) -> Vec<ParamDeduction> {
    let mut walk = Walk {
        file: f.key.file,
        call_fn,
        fn_decls,
        states,
        params: optimistic(f.decl),
        decl: f.decl,
        provenance,
        effects,
        effect_calls,
        state_names,
        refinements,
        cond_depth: 0,
    };
    walk.block(body);
    walk.params
}

/// Validates a *written* deduction list against the facts inferred from the
/// body [deduce-infer]: the contract may be stricter than the body (drop
/// qualifiers, move parameters the body gives back), never looser.
fn validate_written(
    file: usize,
    decl: &FnDecl,
    list: &[Deduction],
    written: &[ParamDeduction],
    inferred: &[ParamDeduction],
    errors: &mut Vec<FileDiagnostic>,
) {
    for ((w, i), p) in written.iter().zip(inferred).zip(&decl.params) {
        let Some(entry) = list.iter().find(|d| d.param.name == w.param) else {
            continue;
        };
        if w.kept && !i.kept {
            errors.push(FileDiagnostic::error(
                file,
                entry.span,
                format!(
                    "deduction promises `{}` back to the caller, but the body \
                     moves it",
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
                    file,
                    entry.span,
                    format!(
                        "deduction promises qualifier `{q}` on `{}`, but the \
                         body may remove it",
                        w.param
                    ),
                ));
            }
        }
    }
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
    /// Provenance qualifier names [qual-subject]: never removed.
    provenance: &'a HashSet<String>,
    /// Effect declarations by name, for effect-member call contracts
    /// [call-resolve].
    effects: &'a HashMap<String, &'p EffectDecl>,
    /// Which effect instance each member call dispatches through (checker
    /// table `effect_calls`), keyed by call span.
    effect_calls: &'a HashMap<Key, Ty>,
    /// Handler *state* field names when walking a handler member
    /// [effect-state-store]: assigning into one is a store (the handler
    /// takes ownership), not a binding. Empty for ordinary fns.
    state_names: &'a HashSet<String>,
    /// [qual-refn] Refinements visible where this body is written: a
    /// refined call re-establishes what the callee's own list had to drop.
    refinements: &'a crate::refine::Refinements,
    /// How many conditional or repeated blocks enclose the statement being
    /// walked (`if`/`when` branches, loop bodies, lambda and deferred
    /// bodies). A refinement's *addition* is honored only at depth 0
    /// [qual-refn-infer]: this walk is a meet over all uses rather than a
    /// flow analysis, so a call that may not run cannot establish a fact
    /// the signature then promises. Removals are unaffected — applying one
    /// unconditionally is the conservative direction.
    cond_depth: usize,
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

impl<'p> Walk<'_, 'p> {
    fn mark_moved(&mut self, name: &str) {
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.kept = false;
            p.effect = QualEffect::Exhaustive(Vec::new());
        }
    }

    /// A use that drops specific qualifiers: the parameter's effect keeps
    /// its polarity and grows the removal set [deduce-syntax].
    fn remove_quals(&mut self, name: &str, removed: &[String]) {
        // [qual-subject] Provenance survives; only state claims drop.
        let removed: Vec<String> = removed
            .iter()
            .filter(|q| !self.provenance.contains(*q))
            .cloned()
            .collect();
        let removed = removed.as_slice();
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
        // [qual-subject] An exhaustive callee still cannot invalidate a
        // provenance claim, so the parameter's own provenance qualifiers
        // stay in the allowed set.
        let mut allowed = allowed.to_vec();
        for q in self
            .decl
            .params
            .iter()
            .find(|p| p.name.name == name)
            .map(|p| declared_quals(&p.ty))
            .unwrap_or_default()
        {
            if self.provenance.contains(&q) && !allowed.contains(&q) {
                allowed.push(q);
            }
        }
        let allowed = allowed.as_slice();
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

    /// A block that may not run, or may run more than once: additions from
    /// refinements inside it do not reach the inferred contract
    /// [qual-refn-infer].
    fn cond_block(&mut self, block: &Block) {
        self.cond_depth += 1;
        self.block(block);
        self.cond_depth -= 1;
    }

    fn cond_expr(&mut self, e: &Expr) {
        self.cond_depth += 1;
        self.expr(e);
        self.cond_depth -= 1;
    }

    /// A use that *establishes* qualifiers [qual-refn]: a refined call puts
    /// back what the callee's own exhaustive list had to drop.
    ///
    /// Restricted to qualifiers the parameter itself declares. An inferred
    /// list still only ever preserves or drops what the signature names —
    /// promising a caller a qualifier the parameter never declared is
    /// `+Q` in a *function's* own deduction list, which is D2 and
    /// deliberately not in the language. So a refinement can cancel a
    /// removal, never invent a claim.
    fn add_quals(&mut self, name: &str, added: &[String]) {
        if self.cond_depth > 0 {
            return;
        }
        let declared = self
            .decl
            .params
            .iter()
            .find(|p| p.name.name == name)
            .map(|p| declared_quals(&p.ty))
            .unwrap_or_default();
        let added: Vec<String> = added
            .iter()
            .filter(|q| declared.contains(q))
            .cloned()
            .collect();
        if added.is_empty() {
            return;
        }
        if let Some(p) = self.params.iter_mut().find(|p| p.param == name) {
            p.effect = match &p.effect {
                // Nothing was dropped, so there is nothing to put back.
                QualEffect::KeepAll => QualEffect::KeepAll,
                QualEffect::Remove(dropped) => QualEffect::Remove(
                    dropped.iter().filter(|q| !added.contains(q)).cloned().collect(),
                ),
                QualEffect::Exhaustive(keep) => {
                    let mut keep = keep.clone();
                    for q in &added {
                        if !keep.contains(q) {
                            keep.push(q.clone());
                        }
                    }
                    QualEffect::Exhaustive(keep)
                }
            };
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
                // [effect-state-store] Assigning into a handler state field
                // stores the value for the handler's lifetime: that is a
                // move, where an ordinary local would only link
                // [fate-link].
                let into_state = bare_ident(target)
                    .is_some_and(|name| self.state_names.contains(name));
                if into_state {
                    self.moving_expr(value);
                } else if bare_ident(value).is_none() {
                    self.expr(value);
                }
            }
            Stmt::Return { value: Some(v), .. } | Stmt::Break { value: Some(v), .. } => {
                self.moving_expr(v)
            }
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
            // [defer] The body runs at the enclosing block's exits, so its
            // calls contribute to the enclosing fn's inferred contract
            // exactly as if written there. Where they run relative to the
            // rest is not modelled here, so a refinement's addition inside
            // one does not reach the contract [qual-refn-infer].
            Stmt::Defer { body, .. } => self.cond_block(body),
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
            // [fn-overload-at] A scope-selected callee only appears inside a
            // `Call` (or as a fn value, which moves nothing); the receiver of
            // its dot form is an ordinary read.
            Expr::Scoped { base, .. } => {
                if let Some(base) = base {
                    self.expr(base);
                }
            }
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
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => self.expr(base),
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
                    self.cond_block(b);
                }
                if let Some(b) = else_block {
                    self.cond_block(b);
                }
            }
            Expr::When {
                subject, branches, ..
            } => {
                self.expr(subject);
                for b in branches {
                    self.cond_block(&b.body);
                }
            }
            // [when-condition]
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.cond_block(b);
                }
                self.cond_block(else_block);
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                self.expr(cond);
                self.cond_block(body);
                if let Some(b) = else_block {
                    self.cond_block(b);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                self.expr(iterable);
                self.cond_block(body);
                if let Some(b) = else_block {
                    self.cond_block(b);
                }
            }
            // Uses inside a lambda body count like any other use (the
            // lambda may run any number of times).
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(e) => self.cond_expr(e),
                LambdaBody::Block(b) => self.cond_block(b),
            },
            // [qual-widen] The check reads its subject.
            Expr::Widen { subject, .. } => self.expr(subject),
            // [try] The delimiter's body is ordinary code: the calls in it
            // contribute to the enclosing fn's inferred contract. A throw
            // can leave it early, so an addition inside one does not reach
            // the contract [qual-refn-infer].
            Expr::Try { body, .. } => self.cond_block(body),
            // Leaves: nothing to walk into. Listed rather than defaulted so
            // a new expression form cannot hide a consuming call from the
            // inference [deduce-syntax].
            Expr::Ident(_)
            | Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Error { .. } => {}
        }
    }

    /// A call site: bare-parameter arguments take the callee's deduction —
    /// moved when the callee moves them, otherwise stripped of exactly the
    /// callee's removal set (declared − kept) [deduce-syntax]. Effect
    /// members resolve through their declared list [call-resolve]; a call
    /// through a fn-typed parameter uses its written contract
    /// [fn-contract]; anything still unresolved borrows and preserves
    /// everything.
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
            .and_then(|key| Some((*key, self.fn_decls.get(key)?, self.states.get(key)?)));
        let Some((callee_key, callee_decl, callee_state)) = resolved else {
            // [call-resolve] An effect-member call: no `FnKey`, but the
            // member's declared list is the contract the call site already
            // enforces, so apply it here too.
            if let Some((member, facts)) = self.effect_member_contract(callee, span) {
                self.apply_callee(&member.params, &facts, arg_exprs, None);
                return;
            }
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
        self.apply_callee(&callee_decl.params, callee_state, arg_exprs, Some(callee_key));
    }

    /// Applies a resolved callee's per-parameter facts to the arguments:
    /// a moved parameter moves a bare-identifier argument, a kept one
    /// applies its qualifier effect [deduce-infer]. `callee` is the
    /// resolved overload when there is one, which is what refinements key
    /// on [qual-refn] — effect members and fn-value calls pass `None`
    /// (a member has no `FnKey`, and refining one is deferred).
    fn apply_callee(
        &mut self,
        params: &[Param],
        state: &[ParamDeduction],
        arg_exprs: Vec<&Expr>,
        callee: Option<FnKey>,
    ) {
        for (i, a) in arg_exprs.into_iter().enumerate() {
            let Some(name) = bare_ident(a) else {
                self.expr(a);
                continue;
            };
            let name = name.to_string();
            let Some(pidx) = param_for_arg(params, i) else {
                continue;
            };
            let Some(ded) = state.get(pidx) else { continue };
            if !ded.kept {
                self.mark_moved(&name);
                continue;
            }
            match ded.effect.clone() {
                QualEffect::KeepAll => {}
                QualEffect::Remove(dropped) => self.remove_quals(&name, &dropped),
                QualEffect::Exhaustive(keep) => self.restrict_to(&name, &keep),
            }
            // [qual-refn] The refinements in scope here have the last word,
            // exactly as at the call site: `add`'s exhaustive list drops
            // `NonEmpty`, and `NonEmpty`'s own refinement puts it back — so
            // a fn whose parameter declares it can promise it onward.
            let Some(callee) = callee else { continue };
            let refined = self
                .refinements
                .for_param(self.file, callee, &params[pidx].name.name)
                .map(|g| (g.add.clone(), g.remove.clone()));
            let Some((add, remove)) = refined else { continue };
            if !remove.is_empty() {
                self.remove_quals(&name, &remove);
            }
            if !add.is_empty() {
                self.add_quals(&name, &add);
            }
        }
    }

    /// The effect member a call dispatches to, with its declared deduction
    /// facts [call-resolve]. `None` when the call is not an effect-member
    /// call, when the checker recorded no instance for it, or when the
    /// member declares no list (then the old lenient borrow stands).
    fn effect_member_contract(
        &self,
        callee: &Expr,
        span: Span,
    ) -> Option<(&'p FnDecl, Vec<ParamDeduction>)> {
        let name = match callee {
            Expr::Ident(id) => id.name.as_str(),
            Expr::Field { field, .. } => field.name.as_str(),
            _ => return None,
        };
        let instance = self.effect_calls.get(&(self.file, span))?;
        let Ty::Named { name: effect, .. } = instance.strip_quals() else {
            return None;
        };
        let decl = self.effects.get(effect.as_str())?;
        let member = decl.fns.iter().find(|f| f.name.name == name)?;
        let list = member.deductions.as_ref()?;
        let facts = from_written(member, list, &HashSet::new(), |_, _| {});
        Some((member, facts))
    }
}
