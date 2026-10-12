//! [ir-linear-check] The IR-level linear-obligation check: a per-path balance
//! over the obligation ledger (`ir::obligations`) and the control-flow
//! structure. It is the "real" check moved onto the IR (IR_CHECKER.md): where
//! the AST checker re-derived each sugar's meaning to decide linearity, this
//! reads the directives the IR builder placed and reports a linear obligation
//! that is still outstanding when a path exits.
//!
//! Model (user decision 2026-10-12): the outstanding-obligation set is tracked
//! **per path**. `GainParams`/`Gain` add; `Release` (a move) and `Discharge`
//! (a narrow that proves a linear arm absent) remove. A `Branch`/`Switch` forks
//! the set per arm; an arm that leaves (`return`/`break`/`continue`) must leave
//! nothing outstanding, and the fall-through states join. At the fn body's end
//! and at every `return`, the set must be empty — a leftover is the leak.

use std::collections::BTreeMap;

use super::nodes::*;

/// A diagnostic from the IR-level check: a span and a message, matching the
/// shape the driver renders.
pub struct IrDiagnostic {
    pub span: Span,
    pub message: String,
}

/// Check every fn body in `program`, returning one diagnostic per leaked
/// obligation.
pub fn check(program: &Program) -> Vec<IrDiagnostic> {
    let mut out = Vec::new();
    for module in &program.modules {
        for decl in &module.decls {
            out.extend(check_one(decl));
        }
    }
    // A leak reachable by more than one walk path reports once.
    out.sort_by(|a, b| (a.span.start, &a.message).cmp(&(b.span.start, &b.message)));
    out.dedup_by(|a, b| a.span.start == b.span.start && a.message == b.message);
    out
}

/// Check one declaration (a fn, or a handler's members), returning the leaks.
pub fn check_one(decl: &Decl) -> Vec<IrDiagnostic> {
    let mut out = Vec::new();
    match decl {
        Decl::Fn(f) => check_fn(f, &mut out),
        Decl::Impl(h) => {
            if let Some(init) = &h.init {
                check_fn(init, &mut out);
            }
            for m in &h.members {
                check_fn(m, &mut out);
            }
        }
        _ => {}
    }
    out
}

/// One outstanding obligation: the place that holds the linear value, and the
/// specific linear arms (for a union) still live. `None` arms means the whole
/// value is linear.
type Key = String;

#[derive(Clone)]
struct Outstanding {
    span: Span,
    arms: Option<Vec<usize>>,
    /// Linear fields of a container not yet moved out; when this becomes
    /// empty (and the container had fields), the obligation is settled by
    /// decomposition.
    pending_fields: std::collections::BTreeSet<String>,
    had_fields: bool,
}

/// The obligation set on one path: place-key → what is outstanding.
type Owed = BTreeMap<Key, Outstanding>;

fn check_fn(f: &FnDecl, out: &mut Vec<IrDiagnostic>) {
    let Some(body) = &f.body else { return };
    let mut owed = Owed::new();
    let flow = walk_block(body, &mut owed, None, out);
    // Fall-through off the end of the body: anything still owed leaks.
    if flow == Flow::FallsThrough {
        report_all(&owed, out, body.span);
    }
}

/// Whether a block/stmt sequence falls through to its continuation or leaves
/// (returns, breaks, continues — the obligations must be settled before).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    FallsThrough,
    Leaves,
}

/// Walk a block's statements, updating `owed` for the fall-through path and
/// returning whether the block falls through or leaves. `loop_base` is the
/// obligation set at the nearest enclosing loop entry: a `break`/`continue`
/// leaves the *loop*, not the fn, so only obligations gained *within* the loop
/// (not in `loop_base`) leak there; `return` and fn-body fall-through leak
/// everything. Branch/switch arms are checked independently and joined.
fn walk_block(block: &Block, owed: &mut Owed, loop_base: Option<&Owed>, out: &mut Vec<IrDiagnostic>) -> Flow {
    for s in &block.stmts {
        match s {
            Stmt::Obligation(o) => apply(o, owed),
            Stmt::Return(value) => {
                // The returned expression is evaluated first (its own
                // branches/switches may gain, release, and leak); then the
                // return leaves the fn with nothing allowed outstanding.
                if let Some(e) = value {
                    if walk_expr(e, owed, loop_base, out) == Flow::Leaves {
                        return Flow::Leaves;
                    }
                }
                report_all(owed, out, stmt_span(s));
                return Flow::Leaves;
            }
            Stmt::Break | Stmt::Continue => {
                // Leaves the loop: only obligations gained inside the loop leak.
                report_since(owed, loop_base, out, stmt_span(s));
                return Flow::Leaves;
            }
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => {
                // The loop body is its own path, entered with the current set
                // as its loop base; its fall-through (loop end) does not change
                // the enclosing set, and a `break value` carries no obligation.
                let mut inner = owed.clone();
                let base = owed.clone();
                let _ = walk_block(body, &mut inner, Some(&base), out);
            }
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Expr(value) => {
                if walk_expr(value, owed, loop_base, out) == Flow::Leaves {
                    return Flow::Leaves;
                }
            }
            Stmt::Alias { .. } | Stmt::Narrow { .. } | Stmt::Unpack { .. } => {}
        }
    }
    if let Some(v) = &block.value {
        if walk_expr(v, owed, loop_base, out) == Flow::Leaves {
            return Flow::Leaves;
        }
    }
    Flow::FallsThrough
}

/// Walk an expression for the control flow it contains (branch/switch arms,
/// which fork the obligation set) and the escape it may perform.
fn walk_expr(e: &Expr, owed: &mut Owed, loop_base: Option<&Owed>, out: &mut Vec<IrDiagnostic>) -> Flow {
    match &e.kind {
        ExprKind::Branch { arms, otherwise, .. } => {
            let mut joined: Option<Owed> = None;
            let mut any_fallthrough = false;
            for (cond, body) in arms {
                let mut path = owed.clone();
                // [ir-obligation] A negated arm-test `!(x is arm k)` guarding
                // this arm proves `x` is not in arm k *inside* the arm, so its
                // obligation on that arm is discharged here.
                if let Some((subject, tested)) = negative_arm_test(cond) {
                    discharge_arms(&mut path, &subject, &tested);
                }
                if walk_block(body, &mut path, loop_base, out) == Flow::FallsThrough {
                    any_fallthrough = true;
                    joined = Some(join(joined, path));
                }
            }
            match otherwise {
                Some(body) => {
                    let mut path = owed.clone();
                    if walk_block(body, &mut path, loop_base, out) == Flow::FallsThrough {
                        any_fallthrough = true;
                        joined = Some(join(joined, path));
                    }
                }
                None => {
                    // No `otherwise`: the subject may skip every arm, so the
                    // pre-branch set survives on that implicit path — except
                    // that falling past a positive arm-test (`x is arm k`)
                    // proves `x` is not in that arm, discharging `x`'s
                    // obligation on it [ir-obligation].
                    let mut path = owed.clone();
                    for (cond, _) in arms {
                        if let Some((subject, tested)) = positive_arm_test(cond) {
                            discharge_arms(&mut path, &subject, &tested);
                        }
                    }
                    any_fallthrough = true;
                    joined = Some(join(joined, path));
                }
            }
            if any_fallthrough {
                *owed = joined.unwrap_or_default();
                Flow::FallsThrough
            } else {
                Flow::Leaves
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            // [ir-obligation] A switch dispatches on its subject: the subject's
            // obligation is distributed to the per-arm narrows (each arm either
            // narrows-and-handles its arm or proves the subject absent there),
            // so it leaves the outstanding set on entry. A narrowed binding
            // that an arm then drops is still caught inside that arm.
            let mut base = owed.clone();
            if let ExprKind::Read { place, .. } = &subject.kind {
                base.remove(&place_key(place));
            }
            let mut joined: Option<Owed> = None;
            let mut any_fallthrough = false;
            for arm in arms {
                let mut path = base.clone();
                if walk_block(&arm.body, &mut path, loop_base, out) == Flow::FallsThrough {
                    any_fallthrough = true;
                    joined = Some(join(joined, path));
                }
            }
            if any_fallthrough {
                *owed = joined.unwrap_or_default();
                Flow::FallsThrough
            } else {
                Flow::Leaves
            }
        }
        // Other expression forms do not themselves fork or leave in a way the
        // ledger needs: their consuming reads were turned into Release
        // directives at the enclosing statement.
        _ => Flow::FallsThrough,
    }
}

/// Apply an obligation directive to the path's outstanding set.
fn apply(o: &Obligation, owed: &mut Owed) {
    match o.kind {
        ObligationKind::GainParams | ObligationKind::Gain => {
            for r in &o.refs {
                owed.insert(
                    place_key(&r.place),
                    Outstanding {
                        span: o.span,
                        arms: r.arms.clone(),
                        pending_fields: r.container_fields.iter().cloned().collect(),
                        had_fields: !r.container_fields.is_empty(),
                    },
                );
            }
        }
        ObligationKind::Release | ObligationKind::Discharge => {
            for r in &o.refs {
                let key = place_key(&r.place);
                if owed.remove(&key).is_some() {
                    continue;
                }
                // [linear-container] A release of a projection `x.f` settles
                // the field `f` of container `x`; when `x`'s last linear field
                // is moved out, `x` itself is settled (decomposition).
                if let Some((root, field)) = split_last_field(&r.place) {
                    if let Some(o) = owed.get_mut(&root) {
                        o.pending_fields.remove(&field);
                        if o.had_fields && o.pending_fields.is_empty() {
                            owed.remove(&root);
                        }
                    }
                }
            }
        }
    }
}

/// If `place` is a container field `root.field` (a single `Field` step), the
/// container key and the field name.
fn split_last_field(place: &Place) -> Option<(Key, String)> {
    match place.steps.last() {
        Some(Step::Field(f)) if place.steps.len() == 1 => Some((place.root.0.clone(), f.clone())),
        _ => None,
    }
}

/// Join two fall-through obligation sets: an obligation outstanding on either
/// path is outstanding at the merge (conservative — a value settled on only
/// some paths is caught as a leak on the path that leaves without settling).
fn join(a: Option<Owed>, b: Owed) -> Owed {
    match a {
        None => b,
        Some(mut acc) => {
            for (k, v) in b {
                acc.entry(k).or_insert(v);
            }
            acc
        }
    }
}

fn report_all(owed: &Owed, out: &mut Vec<IrDiagnostic>, _at: Span) {
    for (place, o) in owed {
        report_one(place, o, out);
    }
}

/// Report obligations outstanding at a `break`/`continue`: only those gained
/// *inside* the loop (not present at the loop entry `base`). An obligation
/// inherited from outside the loop is settled after the loop, not here.
fn report_since(owed: &Owed, base: Option<&Owed>, out: &mut Vec<IrDiagnostic>, _at: Span) {
    for (place, o) in owed {
        let inherited = base.is_some_and(|b| b.contains_key(place));
        if !inherited {
            report_one(place, o, out);
        }
    }
}

fn report_one(place: &str, o: &Outstanding, out: &mut Vec<IrDiagnostic>) {
    let what = match &o.arms {
        Some(arms) if !arms.is_empty() => {
            let a: Vec<String> = arms.iter().map(|i| format!("arm {i}")).collect();
            format!("`{place}` still owns a linear value ({}) when the path leaves", a.join(", "))
        }
        _ => format!("`{place}` still owns a linear value when the path leaves"),
    };
    out.push(IrDiagnostic {
        span: o.span,
        message: format!("{what}; move it onward (pass, return, or store it) or discharge it"),
    });
}

/// If `cond` is a negated arm-test `!(subject is arm k)`, the subject's place
/// key and the runtime arms it tests (proven absent inside the guarded arm).
fn negative_arm_test(cond: &Expr) -> Option<(Key, Vec<usize>)> {
    let ExprKind::Op { op: Op::Not, args } = &cond.kind else { return None };
    positive_arm_test(args.first()?)
}

/// If `cond` is a positive arm-test `subject is arm k` (or several arms), the
/// subject's place key and the runtime arms it tests.
fn positive_arm_test(cond: &Expr) -> Option<(Key, Vec<usize>)> {
    let ExprKind::Test { subject, test, .. } = &cond.kind else { return None };
    let ExprKind::Read { place, .. } = &subject.kind else { return None };
    let arms = match test {
        ArmTest::Arm(k) => vec![*k],
        ArmTest::Arms(ks) => ks.clone(),
        _ => return None,
    };
    Some((place_key(place), arms))
}

/// Discharge `tested` arms of `subject`'s obligation on this path: the arms are
/// proven absent. If no linear arms remain, the obligation is settled.
fn discharge_arms(owed: &mut Owed, subject: &Key, tested: &[usize]) {
    let Some(o) = owed.get_mut(subject) else { return };
    match &mut o.arms {
        Some(live) => {
            live.retain(|a| !tested.contains(a));
            if live.is_empty() {
                owed.remove(subject);
            }
        }
        None => {
            // A whole-value linear leaf is not arm-structured; a positive
            // arm-test on it does not settle it.
        }
    }
}

fn place_key(p: &Place) -> String {
    let mut s = p.root.0.clone();
    for step in &p.steps {
        match step {
            Step::Field(f) => {
                s.push('.');
                s.push_str(f);
            }
            Step::Tuple(i) => {
                s.push_str(&format!(".{i}"));
            }
            Step::Index(_) => s.push_str("[_]"),
        }
    }
    s
}

fn stmt_span(s: &Stmt) -> Span {
    match s {
        Stmt::Return(Some(e)) | Stmt::Expr(e) | Stmt::Let { value: e, .. } | Stmt::Assign { value: e, .. } => e.span,
        _ => Span::default(),
    }
}
