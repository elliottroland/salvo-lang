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
//! entry keeps everything, and `Never` means moved. The removal set is
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
    DeductionTarget, Ident,
    Block, Deduction, DeductionKind, EffectDecl, Expr, FnDecl, Item, LambdaBody, Param,
    QualSubject, SafeAccess, Stmt, StrExprPart, StructLitFieldKind, Type,
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
    /// deduction list, written as `[p: Never]`, or inferred as moved).
    pub kept: bool,
    /// What a call does to the *argument's* known qualifiers. Only
    /// meaningful when `kept`.
    pub effect: QualEffect,
    /// [proj-infer] A `proj(p)` entry names this parameter: the
    /// result holds a borrow of it. Only ever `true` from a written clause.
    pub lent: bool,
    /// [deduce-syntax] The entry was *written* (`=> p …`), as opposed to
    /// inferred from the body or defaulted: written entries are fixed points
    /// of inference and are validated against the body; the rest follow it.
    pub written: bool,
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
    skip_below: Option<usize>,
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
        // [std-cache] std fns are not re-inferred: their contracts are
        // written (all-written → the fixpoint would discard the walked
        // result anyway [deduce-std-explicit]) and already in
        // `checked.deductions`, seeded by the cache. Skipping them is what
        // makes the project check cheap.
        if skip_below.is_some_and(|n| file_idx < n) {
            continue;
        }
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
    // [std-cache] `fn_decls` covers **every** fn (std included): a project fn
    // may call a std fn, and the callee's declaration must be found by key
    // even though std is not re-inferred. It is only AST references, cheap.
    let fn_decls: HashMap<FnKey, &FnDecl> = program
        .modules
        .iter()
        .enumerate()
        .flat_map(|(file_idx, ast)| {
            ast.items.iter().enumerate().filter_map(move |(item_idx, item)| match item {
                Item::Fn(f) => Some((FnKey { file: file_idx, item: item_idx }, f)),
                _ => None,
            })
        })
        .collect();
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
    // [proj-field] Which struct fields are borrows: storing into one
    // lends the value, so the parameter it came from stays kept.
    let proj_fields: HashMap<String, HashSet<String>> = program
        .modules
        .iter()
        .flat_map(|ast| ast.items.iter())
        .filter_map(|item| match item {
            Item::Struct(sd) => {
                let names: HashSet<String> = sd
                    .fields
                    .iter()
                    .filter(|f| type_has_proj(&f.ty))
                    .map(|f| f.name.name.clone())
                    .collect();
                (!names.is_empty()).then(|| (sd.name.name.clone(), names))
            }
            _ => None,
        })
        .collect();
    let expr_ty = checked.expr_ty.clone();

    // [deduce-default] The contract of every fn is a function of its
    // signature alone (user decision 2026-10-11): the per-parameter default
    // (a `send fn` consumes, a plain `fn` keeps, a `Mut` parameter keeps only
    // `Mut`), with any written entry overriding its parameter. No body
    // inference, so no fixpoint over the call graph — the contract is known in
    // one pass. `claims` and `mutations` are no longer consulted here (a
    // move-mode binding reaching a kept parameter is reported by the fate
    // analysis and by the body validation below, not resolved by changing the
    // contract); the parameters are kept so the checker's call sites compile.
    let _ = (&claims, &mutations);
    let mut errors: Vec<FileDiagnostic> = Vec::new();
    let mut states: HashMap<FnKey, Vec<ParamDeduction>> = HashMap::new();
    // [std-cache] Seed std fns' deductions from the cache (in `checked`,
    // seeded by `check_once`): they are not re-derived, but a project fn
    // calling one reads its contract from `states` by key.
    if let Some(n) = skip_below {
        for (key, ded) in &checked.deductions {
            if key.file < n {
                states.insert(*key, ded.clone());
            }
        }
    }
    for f in &fns {
        let state = match &f.decl.deductions {
            Some(list) => from_written(f.decl, list, &no_mutations, |span, msg| {
                errors.push(FileDiagnostic::error(f.key.file, span, msg));
            }),
            None => defaults(f.decl),
        };
        states.insert(f.key, state);
    }

    // [deduce-default] Validate every bodied fn's contract — defaults and
    // written entries alike — against the body: the contract may be *stricter*
    // than the body (dropping qualifiers, or moving a parameter the body gives
    // back) but never looser, so a kept parameter the body moves is an error
    // naming the `!p` remedy. (Before 2026-10-11 an unwritten parameter was
    // inferred from the body and so could never fail this; now the default
    // keep is checked like a written one.)
    for f in &fns {
        let Some(body) = &f.decl.body else { continue };
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
            &proj_fields,
            &expr_ty,
        );
        let contract = &states[&f.key];
        validate_contract(f.key.file, f.decl, contract, &inferred, &mut errors);
    }

    // [effect-handler] Handler members are declarations with bodies too, and
    // their contract is applied at call sites like a named call's
    // ([decl-explicit], [call-resolve]) — so a contract that promises more
    // than the body delivers is the same bug there. They carry no `FnKey`
    // (not top-level items), so they are validated here.
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for (item_idx, item) in ast.items.iter().enumerate() {
            let Item::Handler(h) = item else { continue };
            for m in &h.fns {
                let Some(body) = &m.body else { continue };
                let info = FnInfo {
                    key: FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    decl: m,
                };
                let contract = match &m.deductions {
                    Some(list) => from_written(m, list, &no_mutations, |_, _| {}),
                    None => defaults(m),
                };
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
                    &proj_fields,
                    &expr_ty,
                );
                validate_contract(file_idx, m, &contract, &inferred, &mut errors);
            }
        }
    }

    checked.errors.extend(errors);
    checked.deductions = states;
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
            lent: false,
            written: false,
        })
        .collect()
}

/// A written deduction list, validated for shape: entries must name a
/// parameter (once), exhaustive entries may only keep qualifiers declared
/// on that parameter, and `Never` is the only type form [deduce-syntax].
/// `mutated` names the parameters the body invalidates; for those, only
/// the exhaustive form is sound (see the soundness rule under D1).
pub(crate) fn from_written(
    decl: &FnDecl,
    list: &[Deduction],
    mutated: &HashSet<String>,
    mut error: impl FnMut(Span, String),
) -> Vec<ParamDeduction> {
    for (i, d) in list.iter().enumerate() {
        // Every parameter an entry names — plainly, as a field path's root,
        // or as a projection source — must exist.
        let mut named: Vec<&Ident> = Vec::new();
        if let DeductionTarget::Param { name, .. } = &d.target {
            named.push(name);
        }
        if let Some(sources) = d.proj_sources() {
            named.extend(sources.iter());
        }
        for n in named {
            if !decl.params.iter().any(|p| p.name.name == n.name) {
                error(
                    n.span,
                    format!("deduction names unknown parameter `{}`", n.name),
                );
            }
        }
        if let Some(pn) = d.param_name() {
            // [qual-preserve] A `preserve` entry accompanies the parameter's
            // ordinary one — it speaks about *other* values' claims — so it
            // is exempt from the once-rule on both sides.
            let is_preserve = |x: &Deduction| matches!(x.kind, DeductionKind::Preserve(_));
            if !is_preserve(d)
                && list[..i].iter().any(|prev| {
                    !is_preserve(prev)
                        && prev.param_name().is_some_and(|q| q.name == pn.name)
                })
            {
                error(
                    d.span,
                    format!("duplicate deduction for parameter `{}`", pn.name),
                );
            }
        }
    }
    // [proj-infer] The parameters some `proj(…)` entry names.
    let lent_names: HashSet<&str> = list
        .iter()
        .filter_map(|d| d.proj_sources())
        .flatten()
        .map(|i| i.name.as_str())
        .collect();
    decl.params
        .iter()
        .map(|p| {
            let declared = declared_quals(&p.ty);
            let name = p.name.name.clone();
            let lent = lent_names.contains(name.as_str());
            let Some(d) = list.iter().find(|d| {
                // [qual-preserve] Not an ownership entry.
                !matches!(d.kind, DeductionKind::Preserve(_)) && d.param_name().is_some_and(|n| n.name == name)
            }) else {
                // [deduce-default] Unmentioned by the clause: the parameter
                // takes its default (a written clause is partial — the entries
                // it omits are not inferred, they default like a clauseless
                // fn's, per-parameter). The `lent` flag still comes from any
                // `proj(…)` entry naming this parameter.
                let mut d = default_param(decl, p);
                d.lent = lent;
                return d;
            };
            let invalidates = mutated.contains(&name);
            let effect = match &d.kind {
                // [implicit-with] The fill-together relation is a statement
                // about which *implicits* travel together, read where a
                // call fills them.
                DeductionKind::With { .. } => {
                    return ParamDeduction {
                        param: name,
                        kept: true,
                        effect: QualEffect::KeepAll,
                        lent: false,
                        written: false,
                    };
                }
                // [defer-deduction] Consumed either way: a deferred
                // obligation leaves the caller's hands exactly as a moved
                // one does — what differs is only what the *body* may do
                // with it, which is the checker's business, not the
                // contract-shape's.
                DeductionKind::Moved | DeductionKind::Deferred => {
                    return ParamDeduction {
                        param: name,
                        kept: false,
                        effect: QualEffect::Exhaustive(Vec::new()),
                        lent: false,
                        written: true,
                    };
                }
                // A projection kind never reaches here (`param_name` is
                // `None` for it); the exhaustive default is unreachable.
                DeductionKind::Proj(_) => QualEffect::KeepAll,
                // [qual-preserve] Not an ownership fact: the entry speaks
                // about *other* values' claims, and the parameter's own
                // entry (or its absence) decides keptness as ever.
                DeductionKind::Preserve(_) => QualEffect::KeepAll,
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
                                 `Never`): type narrowing in deductions is not \
                                 supported yet"
                                    .to_string(),
                            );
                            continue;
                        }
                        drop.push(q.name.name.clone());
                    }
                    QualEffect::Remove(drop)
                }
                DeductionKind::Exhaustive { quals, reapplied } => {
                    let mut keep = Vec::new();
                    // [deduce-reapply] A re-applied entry may carry the
                    // qualifier's arguments (`+Heap<T>(?cmp)`), which is the
                    // point of the spelling: it names the claim it establishes.
                    // A plain entry may not — there the identity travels with
                    // the type and restating it would be a second source of
                    // truth [cmp-binder]. The checker validates the arguments
                    // and the same-file rule, where the lowered types are.
                    for (q, is_reapplied) in quals
                        .iter()
                        .map(|q| (q, false))
                        .chain(reapplied.iter().map(|q| (q, true)))
                    {
                        if !q.args.is_empty() && !is_reapplied {
                            error(
                                q.span,
                                "a deduction entry may only name qualifiers (and \
                                 `Never`): type narrowing in deductions is not \
                                 supported yet"
                                    .to_string(),
                            );
                            continue;
                        }
                        // [deduce-gained] A plain entry naming a qualifier the
                        // parameter does not declare is legal when the *body*
                        // establishes it — through a refinement, so the claim's
                        // owner said so [qual-refn] — and that is only knowable
                        // once the body has been walked. With a body the
                        // judgement is `validate_written`'s; without one (an
                        // `intrinsic fn`, an effect member) there is nothing to
                        // check against, so it is refused here.
                        if !declared.contains(&q.name.name)
                            && !is_reapplied
                            && decl.body.is_none()
                        {
                            error(
                                q.span,
                                format!(
                                    "deduction keeps qualifier `{}`, which is not \
                                     declared on parameter `{name}` (a bodiless \
                                     declaration has nothing to establish it; \
                                     `+{}` is how a function says it \
                                     *establishes* one)",
                                    q.name.name, q.name.name
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
                lent,
                written: true,
            }
        })
        .collect()
}

/// [deduce-default] The **default** deduction for one parameter, used where no
/// written entry overrides it (user decision 2026-10-11, replacing
/// body-inference): a `send fn` consumes every parameter (the message owns its
/// payload); a plain `fn` keeps every parameter, and a parameter whose declared
/// type carries `Mut` keeps only `Mut` (the exhaustive form, since a mutable
/// parameter the body may change cannot promise other qualifiers back).
pub(crate) fn default_param(decl: &FnDecl, p: &Param) -> ParamDeduction {
    if decl.is_send {
        return ParamDeduction {
            param: p.name.name.clone(),
            kept: false,
            effect: QualEffect::Exhaustive(Vec::new()),
            lent: false,
            written: false,
        };
    }
    let declared = declared_quals(&p.ty);
    let effect = if declared.iter().any(|q| q == "Mut") {
        QualEffect::Exhaustive(vec!["Mut".to_string()])
    } else {
        QualEffect::KeepAll
    };
    ParamDeduction {
        param: p.name.name.clone(),
        kept: true,
        effect,
        lent: false,
        written: false,
    }
}

/// [deduce-default] Every parameter at its default: the contract of a fn with
/// no written clause.
pub(crate) fn defaults(decl: &FnDecl) -> Vec<ParamDeduction> {
    decl.params.iter().map(|p| default_param(decl, p)).collect()
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
    proj_fields: &'a HashMap<String, HashSet<String>>,
    expr_ty: &'a HashMap<Key, Ty>,
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
        proj_fields,
        expr_ty,
    };
    walk.block(body);
    walk.params
}

/// [deduce-default] Whether a parameter type is a **Copy scalar** by its
/// written name (`Int`, `Long`, …): such a value is duplicated for free on
/// both backends, so its deduction is nothing to decide — moving one is not a
/// move the caller can observe. Name-based (deduction runs without the symbol
/// table); an alias to a scalar is not caught, which only risks a spurious
/// move error on an alias and never unsound output.
fn is_copy_scalar_param(ty: &Type) -> bool {
    let base = match ty {
        Type::Named { qualifiers, base } if qualifiers.is_empty() && base.args.is_empty() => base,
        Type::Nullable { inner, .. } => return is_copy_scalar_param(inner),
        _ => return false,
    };
    matches!(
        base.name.name.as_str(),
        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
    )
}

/// [deduce-default] Validates a fn's **contract** (defaults, written entries,
/// or a mix) against the facts the body produces: the contract may be stricter
/// than the body (drop qualifiers, move a parameter the body gives back) but
/// never looser. A kept parameter the body moves, or a kept qualifier the body
/// may remove, is an error. Unlike the old `validate_written`, this checks
/// every parameter — a default `keep` is checked exactly as a written `=> p`
/// is, so a clauseless fn that moves a parameter is caught (the remedy is to
/// write `=> !p`).
fn validate_contract(
    file: usize,
    decl: &FnDecl,
    contract: &[ParamDeduction],
    inferred: &[ParamDeduction],
    errors: &mut Vec<FileDiagnostic>,
) {
    // The written clause entry for a parameter, if any — used for the error
    // span and to read a `+Q` reapplied set. Absent for a defaulted parameter.
    let entry_for = |name: &str| -> Option<&Deduction> {
        decl.deductions.as_ref().and_then(|list| {
            list.iter().find(|d| {
                !matches!(d.kind, DeductionKind::Preserve(_))
                    && d.param_name().is_some_and(|n| n.name == name)
            })
        })
    };
    for ((w, i), p) in contract.iter().zip(inferred).zip(&decl.params) {
        // [deduce-default] A Copy scalar's fate is nothing to deduce — it is
        // duplicated for free — so a default-keep is never contradicted by the
        // body "moving" one.
        if is_copy_scalar_param(&p.ty) {
            continue;
        }
        let entry = entry_for(&w.param);
        // Where to point the diagnostic: a written entry's span, else the
        // parameter's own.
        let span = entry.map(|e| e.span).unwrap_or(p.name.span);
        if w.kept && !i.kept {
            let remedy = if entry.is_some() {
                format!(
                    "deduction promises `{}` back to the caller, but the body \
                     moves it",
                    w.param
                )
            } else {
                format!(
                    "`{}` is kept by default, but the body moves it (returns it, \
                     stores it, or passes it to a consuming call): write `=> !{}` \
                     to declare the move, or `copy(…)` at the move site to keep \
                     the caller's value",
                    w.param, w.param
                )
            };
            errors.push(FileDiagnostic::error(file, span, remedy));
            continue;
        }
        let declared = declared_quals(&p.ty);
        let promised = w.effect.kept_quals(&declared);
        let survives = i.effect.kept_quals(&declared);
        // [deduce-reapply] A claim written `+Q` is **established by this
        // function**, so the body is not asked whether it survived.
        let reapplied: Vec<&str> = match entry.map(|e| &e.kind) {
            Some(DeductionKind::Exhaustive { reapplied, .. }) => {
                reapplied.iter().map(|q| q.name.name.as_str()).collect()
            }
            _ => Vec::new(),
        };
        for q in &promised {
            if !w.kept || survives.contains(q) || reapplied.contains(&q.as_str()) {
                continue;
            }
            // [deduce-gained] Two different mistakes, and the difference is
            // what the reader needs: a qualifier the parameter *has* that the
            // body may drop, versus one it never had and the body does not
            // establish either.
            let message = if declared.contains(q) {
                format!(
                    "deduction promises qualifier `{q}` on `{}`, but the body \
                     may remove it",
                    w.param
                )
            } else {
                format!(
                    "deduction keeps qualifier `{q}`, which is not declared on \
                     parameter `{}` and nothing in the body establishes it — a \
                     plain entry reports what the body leaves behind, and \
                     `+{q}` (legal only in `{q}`'s own file) is how a function \
                     *claims* to establish one",
                    w.param
                )
            };
            errors.push(FileDiagnostic::error(file, span, message));
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
    /// walked (`if`/`when` branches, loop bodies, lambda
    /// bodies). A refinement's *addition* is honored only at depth 0
    /// [qual-refn-infer]: this walk is a meet over all uses rather than a
    /// flow analysis, so a call that may not run cannot establish a fact
    /// the signature then promises. Removals are unaffected — applying one
    /// unconditionally is the conservative direction.
    cond_depth: usize,
    /// [proj-field] Struct name → its `proj` field names. Storing into
    /// such a field *lends* the value rather than moving it.
    proj_fields: &'a HashMap<String, HashSet<String>>,
    /// Checker expression types by span, for a bare `{…}` literal whose
    /// struct is not written.
    expr_ty: &'a HashMap<Key, Ty>,
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
    /// The qualifiers a parameter carries at this point in the walk: its
    /// declared set as the uses seen so far have changed it [deduce-infer]
    /// [deduce-gained].
    fn current_quals(&self, name: &str) -> Vec<String> {
        let declared = self
            .decl
            .params
            .iter()
            .find(|p| p.name.name == name)
            .map(|p| declared_quals(&p.ty))
            .unwrap_or_default();
        self.params
            .iter()
            .find(|p| p.param == name)
            .map(|p| p.effect.kept_quals(&declared))
            .unwrap_or(declared)
    }

    /// [qual-refn-narrow] `add_quals` for a claim a call **keeps** rather than
    /// establishes: legal inside a branch, because the claim was there before
    /// the call either way.
    fn keep_quals(&mut self, name: &str, added: &[String]) {
        let saved = std::mem::replace(&mut self.cond_depth, 0);
        self.add_quals(name, added);
        self.cond_depth = saved;
    }

    /// A use that puts a qualifier back [qual-refn]. Ignored inside a
    /// conditional: a call that may not have happened cannot have established
    /// anything.
    fn add_quals(&mut self, name: &str, added: &[String]) {
        if self.cond_depth > 0 {
            return;
        }
        // [deduce-gained] A qualifier the parameter does not declare is
        // recorded too: everything reaching here came from a **refinement**
        // written by the claim's owner [qual-refn], so the body really does
        // leave the value with it — `add(list, x)` makes a plain `Mut List<T>`
        // non-empty. Until 2026-09-23 the gain was filtered away, so a function
        // could not report a claim its body established unless its own
        // parameter already carried it (found by the user on `std.heap`'s
        // `push`).
        let added: Vec<String> = added.to_vec();
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
            // calls contribute to the enclosing fn's inferred contract
            // exactly as if written there. Where they run relative to the
            // rest is not modelled here, so a refinement's addition inside
            // one does not reach the contract [qual-refn-infer].
            _ => {}
        }
    }

    /// Whether `e`, returned, is a projection the signature declares: its
    /// provenance root is a `from` source of a wholesale `proj` in the
    /// return type [readonly-return].
    fn returns_projection_of(&self, e: &Expr) -> bool {
        let Some(rt) = &self.decl.return_type else { return false };
        let sources: Vec<String> = proj_sources_of(rt);
        if sources.is_empty() {
            return false;
        }
        fn root(e: &Expr) -> Option<&str> {
            match e {
                Expr::Ident(id) => Some(&id.name),
                Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => {
                    root(base)
                }
                Expr::NonNull { operand, .. } => root(operand),
                // A constructor around the borrow (`emitted(e)`): its argument.
                Expr::Call { args, .. } if args.len() == 1 => root(&args[0]),
                _ => None,
            }
        }
        root(e).is_some_and(|r| sources.iter().any(|s| s == r))
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
            Expr::Assert { cond, message, .. } => {
                self.expr(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::Unreachable { message, .. } | Expr::Todo { message, .. } => {
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            // [elvis] The picked value leaves through the expression, so the
            // subject is used the way a returned value is; the right side is
            // an ordinary expression.
            Expr::Elvis { subject, rhs, .. } => {
                self.expr(subject);
                self.expr(rhs);
            }
            Expr::Placeholder { .. } => {}
            // [safe-nav] The receiver is read (navigated), and any call
            // arguments are evaluated; mirror a dot read/call's effects.
            Expr::SafeNav { base, access, .. } => {
                self.expr(base);
                if let SafeAccess::Call { args, named, .. } = access {
                    for a in args {
                        self.expr(a);
                    }
                    for n in named {
                        self.expr(&n.value);
                    }
                }
            }
            Expr::Call {
                callee, args, span, ..
            } => self.call(callee, args, *span),
            // [fn-overload-at] A scope-selected callee only appears inside a
            // `Call` (or as a fn value, which moves nothing); the receiver of
            // its dot form is an ordinary read.
            Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
                if let Some(base) = base {
                    self.expr(base);
                }
            }
            // Struct/array/tuple construction stores the value.
            Expr::StructLit { ty, fields, span } => {
                // [proj-field] A `proj` field lends: the value is read,
                // not stored, so its parameter stays kept.
                let struct_name: Option<String> = match ty {
                    Some(Type::Named { base, .. }) => Some(base.name.name.clone()),
                    _ => self
                        .expr_ty
                        .get(&(self.file, *span))
                        .and_then(|t| match t.strip_quals() {
                            Ty::Named { name, .. } => Some(name.clone()),
                            _ => None,
                        }),
                };
                let lent = struct_name
                    .as_deref()
                    .and_then(|n| self.proj_fields.get(n));
                for f in fields {
                    match &f.kind {
                        StructLitFieldKind::Named { name, value }
                            if lent.is_some_and(|set| set.contains(&name.name)) =>
                        {
                            self.expr(value)
                        }
                        StructLitFieldKind::Named { value, .. } => self.moving_expr(value),
                        StructLitFieldKind::Spread(v) => self.moving_expr(v),
                        // [comptime-inline] Gone before this runs.
                        StructLitFieldKind::InlineFor { .. } => {}
                    }
                }
            }
            Expr::Tuple { elems, .. } => {
                for el in elems {
                    self.moving_expr(el);
                }
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
            | Expr::IncDec { operand, .. }
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
            // [qual-lift] The check reads its subject.
            Expr::Widen { subject, .. } => self.expr(subject),
            // [try] The delimiter's body is ordinary code: the calls in it
            // contribute to the enclosing fn's inferred contract. A throw
            // can leave it early, so an addition inside one does not reach
            // the contract [qual-refn-infer].
            Expr::Try { body, .. } => self.cond_block(body),
            // [expr-escape] Returning or breaking with a value **moves** it
            // (2026-09-21: the same rule, now reached as an expression).
            Expr::Return { value: Some(v), .. } => {
                // [readonly-return] A fn returning `proj(p, …) T` hands
                // its result out *borrowed*: returning a source (or a
                // projection of one) keeps it. Everything else returned is
                // moved.
                if self.returns_projection_of(v) {
                    self.expr(v);
                } else {
                    self.moving_expr(v);
                }
            }
            Expr::Break { value: Some(v), .. } => self.moving_expr(v),
            Expr::Return { value: None, .. } | Expr::Break { value: None, .. }
            | Expr::Continue { .. } => {}
            // [actor-spawn-expr] A spawn's arguments cross the seam, so a
            // value passed to a child is *consumed* — walked as ordinary
            // reads here (the send-as-move accounting arrives with the
            // checker slice, which is what types these forms at all).
            Expr::Spawn {
                handler,
                with_items,
                pool,
                ..
            } => {
                self.expr(handler);
                for handler in with_items {
                    self.expr(handler);
                }
                if let Some(pool) = pool {
                    self.expr(pool);
                }
            }
            // [actor-self-send] A leaf: nothing to walk into.
            Expr::SelfScoped { .. } | Expr::SelfAddr { .. } => {}
            // [actor-replyto] The captures are reads.
            Expr::ReplyTo { captures, .. } => {
                for capture in captures {
                    self.expr(capture);
                }
            }
            // [actor-waitfor] Its block runs once, like a `try` body.
            Expr::WaitFor { body, .. } => self.cond_block(body),
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
                        // [deduce-syntax] A fn type's unmentioned parameter is
                        // kept (the default); only `!x` / `x: Never` moves.
                        let kept = param_names
                            .get(i)
                            .and_then(|n| n.as_ref())
                            .map(|n| {
                                !list.iter().any(|d| {
                                    d.param_name().is_some_and(|q| q.name == n.name)
                                        && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
                                })
                            })
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
            // [qual-refn-narrow] What the value carries *here*, before this
            // call's own effect is applied: a conditional refinement is about
            // the claim the argument arrives with, and that is the declared set
            // as this walk has changed it so far — `add` earlier in the body
            // may have established something the parameter never declared
            // [deduce-gained].
            let had = self.current_quals(&name);
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
            // [qual-refn-narrow] A refinement that wrote a *narrower* parameter
            // applies only where the claim is already there. Here "already
            // there" means the enclosing fn's parameter declares it: that is
            // what this pass reasons about, and what the promise being checked
            // is made of.
            let groups: Vec<(Vec<String>, Vec<String>, bool)> = self
                .refinements
                .for_param_all(self.file, callee, &params[pidx].name.name)
                .into_iter()
                .filter(|g| g.requires.iter().all(|q| had.contains(q)))
                .map(|g| (g.add.clone(), g.remove.clone(), !g.requires.is_empty()))
                .collect();
            for (add, remove, keeps) in groups {
                if !remove.is_empty() {
                    self.remove_quals(&name, &remove);
                }
                if add.is_empty() {
                    continue;
                }
                if keeps {
                    // The claim is **kept**, not established: whether the call
                    // happened or not, what the caller had it still has — so
                    // this one survives a conditional, where an establishing
                    // refinement must not (a call that may not have run cannot
                    // have made a list non-empty).
                    self.keep_quals(&name, &add);
                } else {
                    self.add_quals(&name, &add);
                }
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

/// [proj-field] Whether a written type carries a `proj` qualifier
/// anywhere.
/// [proj-anywhere] The `from` sources of every wholesale `proj` in a type.
fn proj_sources_of(ty: &Type) -> Vec<String> {
    fn in_ref(r: &salvo_syntax::ast::TypeRef, out: &mut Vec<String>) {
        if crate::types::is_proj_name(&r.name.name) {
            out.extend(r.from.iter().map(|i| i.name.clone()));
        }
        for a in &r.args {
            walk(a, out);
        }
    }
    fn walk(ty: &Type, out: &mut Vec<String>) {
        match ty {
            Type::Literal { .. } => {}
            Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                in_ref(base, out);
            }
            Type::QualifiedGroup { qualifiers, base, .. } => {
                for q in qualifiers {
                    in_ref(q, out);
                }
                walk(base, out);
            }
            Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
                for a in arms {
                    walk(a, out);
                }
            }
            Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => walk(elem, out),
            Type::Fn { .. } => {}
        }
    }
    let mut out = Vec::new();
    walk(ty, &mut out);
    out
}

fn type_has_proj(ty: &Type) -> bool {
    fn in_ref(r: &salvo_syntax::ast::TypeRef) -> bool {
        crate::types::is_proj_name(&r.name.name) || r.args.iter().any(type_has_proj)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_has_proj(base),
        Type::Nullable { inner, .. } | Type::Array { elem: inner, .. } => type_has_proj(inner),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => arms.iter().any(type_has_proj),
        _ => false,
    }
}
