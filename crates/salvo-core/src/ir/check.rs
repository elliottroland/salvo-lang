//! [ir-linear-check] The IR-level linear-obligation check: a per-path balance
//! over the obligation ledger (`ir::obligations`) and the control-flow
//! structure. It is the "real" check moved onto the IR (IR_CHECKER.md): where
//! the AST checker re-derived each sugar's meaning to decide linearity, this
//! reads the directives the IR builder placed and reports a linear obligation
//! that is still outstanding when a path exits.
//!
//! Model (user decisions 2026-10-12): the outstanding-obligation set is tracked
//! **per path**. `GainParams`/`Gain` add; `Release` (a move) and `Discharge`
//! (a narrow that proves a linear arm absent) remove. A `Branch`/`Switch` forks
//! the set per arm; the fall-through states join.
//!
//! Control *leaves* a path wherever evaluation reaches a `Never`-typed
//! expression — `throw`, an `unreachable` trap, a call to a `-> Never` fn — and
//! at the `return`/`break`/`continue` statements, which the IR keeps as
//! statements. A call to a `throws` fn is a *conditional* leave: on the throw
//! edge control leaves the fn. At every such leave the outstanding set must be
//! empty for the destination's scope (the whole set at a fn leave; only what
//! was gained inside the loop at a `break`/`continue`) — a leftover is the leak.
//!
//! [linear-obligation] The flow rules (R8 piece 7; the checker's until then):
//! where paths come back together — the arms of a branch or a switch that fall
//! through, a loop's exits (a `for`'s top, every `break`) — a value owed before
//! the fork that some paths settled and others still owe (itself, or through a
//! narrowing of it) is "consumed on some paths but not others", reported at the
//! written construct and owed no longer; and a written assignment over a local
//! that still owes drops its value. The set after a loop is its exits' join.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use super::nodes::*;

/// A diagnostic from the IR-level check: a span and a message, matching the
/// shape the driver renders.
pub struct IrDiagnostic {
    pub span: Span,
    pub message: String,
    /// The leaked linear type's name, if known — so the gate (`ir::analyze`)
    /// can name its dischargers in the remedy ("discharge it with `drain`").
    pub discharge_type: Option<String>,
    /// A path-exit leak, as opposed to a flow rule (consumed on some paths
    /// only, an assignment that drops a value).
    pub leak: bool,
}

/// Check every fn body in `program`, returning one diagnostic per leaked
/// obligation.
pub fn check(program: &Program) -> Vec<IrDiagnostic> {
    // Which top-level fns may throw: a call to one is a conditional leave.
    let throws = throws_map(program);
    let mut out = Vec::new();
    for module in &program.modules {
        for decl in &module.decls {
            out.extend(check_one_with(decl, &throws, true));
        }
    }
    // A leak reachable by more than one walk path reports once.
    out.sort_by(|a, b| (a.span.start, &a.message).cmp(&(b.span.start, &b.message)));
    out.dedup_by(|a, b| a.span.start == b.span.start && a.message == b.message);
    out
}

/// Check one declaration (a fn, or a handler's members), returning the leaks.
/// `throws` says which top-level fns may throw (a call to one is a conditional
/// leave); build it once with [`throws_map`] and pass it for every decl.
pub fn check_one(decl: &Decl, throws: &HashMap<DeclId, bool>) -> Vec<IrDiagnostic> {
    check_one_with(decl, throws, true)
}

/// [linear-obligation] Only the flow rules of a declaration in a test file:
/// a value consumed on some paths but not others, and an assignment that
/// drops one — not its path-exit leaks, which are the harness's business.
pub fn check_one_flow_only(decl: &Decl, throws: &HashMap<DeclId, bool>) -> Vec<IrDiagnostic> {
    check_one_with(decl, throws, false)
}

/// The map of which top-level fns may throw, keyed by `DeclId`.
pub fn throws_map(program: &Program) -> HashMap<DeclId, bool> {
    program
        .modules
        .iter()
        .flat_map(|m| m.decls.iter())
        .filter_map(|d| match d {
            Decl::Fn(f) => Some((f.id.clone(), f.throws.is_some())),
            _ => None,
        })
        .collect()
}

fn check_one_with(decl: &Decl, throws: &HashMap<DeclId, bool>, leaks: bool) -> Vec<IrDiagnostic> {
    let mut out = Vec::new();
    match decl {
        Decl::Fn(f) => check_fn(f, throws, leaks, &mut out),
        Decl::Impl(h) => {
            if let Some(init) = &h.init {
                check_fn(init, throws, leaks, &mut out);
            }
            for m in &h.members {
                check_fn(m, throws, leaks, &mut out);
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
    /// The leaked type's name, for naming its dischargers.
    ty_name: Option<String>,
    /// [linear-group] Each linear arm's runtime index paired with its arm
    /// type's display name, so a leak can name the arm instead of a number.
    arm_names: Vec<(usize, String)>,
}

/// The obligation set on one path: place-key → what is outstanding.
type Owed = BTreeMap<Key, Outstanding>;

/// Read-only context for a walk: which fns throw, the loop-entry snapshot (for
/// `break`/`continue` scope), and whether a `try` currently catches throws.
struct Ctx<'a> {
    throws: &'a HashMap<DeclId, bool>,
    loop_base: Option<&'a Owed>,
    in_try: bool,
    /// What the walk learns as it goes, shared by every nested context.
    shared: &'a RefCell<Shared>,
    /// The fn's flow-only facts: which assignments are written, and where.
    fate: &'a FateFacts,
}

impl<'a> Ctx<'a> {
    fn with(&self, loop_base: Option<&'a Owed>, in_try: bool) -> Ctx<'a> {
        Ctx { throws: self.throws, loop_base, in_try, shared: self.shared, fate: self.fate }
    }
}

/// [linear-obligation] The walk's mutable side: per enclosing loop
/// (innermost last), the sets at its `break`s; and each narrowing binding's
/// source, so a path that still owes the binding owes the source.
#[derive(Default)]
struct Shared {
    breaks: Vec<Vec<Owed>>,
    narrow_of: HashMap<Key, Key>,
}

fn check_fn(f: &FnDecl, throws: &HashMap<DeclId, bool>, leaks: bool, out: &mut Vec<IrDiagnostic>) {
    let Some(body) = &f.body else { return };
    let mut owed = Owed::new();
    let shared = RefCell::new(Shared::default());
    let ctx = Ctx { throws, loop_base: None, in_try: false, shared: &shared, fate: &f.fate };
    let mut found = Vec::new();
    let flow = walk_block(body, &mut owed, &ctx, &mut found);
    // Fall-through off the end of the body: anything still owed leaks.
    if flow == Flow::FallsThrough {
        report_all(&owed, &mut found, body.span);
    }
    // A test file reports the flow rules only.
    out.extend(found.into_iter().filter(|d| leaks || !d.leak));
}

/// Whether a block/stmt sequence falls through to its continuation or leaves
/// (a `Never` expression, or `return`/`break`/`continue`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    FallsThrough,
    Leaves,
}

/// Walk a block's statements, updating `owed` for the fall-through path and
/// returning whether the block falls through or leaves.
fn walk_block(block: &Block, owed: &mut Owed, ctx: &Ctx, out: &mut Vec<IrDiagnostic>) -> Flow {
    for s in &block.stmts {
        match s {
            Stmt::Obligation(o) => apply(o, owed),
            Stmt::Return(value, _) => {
                // The returned expression is evaluated first (its own
                // branches/switches may gain, release, and leak); then the
                // return leaves the fn with nothing allowed outstanding.
                if let Some(e) = value {
                    if walk_expr(e, owed, ctx, out) == Flow::Leaves {
                        return Flow::Leaves;
                    }
                }
                report_all(owed, out, stmt_span(s));
                return Flow::Leaves;
            }
            Stmt::Break | Stmt::Continue => {
                // Leaves the loop: only obligations gained inside the loop leak.
                report_since(owed, ctx.loop_base, out, stmt_span(s));
                if matches!(s, Stmt::Break) {
                    if let Some(frame) = ctx.shared.borrow_mut().breaks.last_mut() {
                        frame.push(owed.clone());
                    }
                }
                return Flow::Leaves;
            }
            Stmt::Loop { id, body } | Stmt::ForEach { id, body, .. } => {
                // The loop body is its own path, entered with the current set
                // as its loop base; a `break value` carries no obligation.
                let base = owed.clone();
                let mut inner = owed.clone();
                let inner_ctx = ctx.with(Some(&base), ctx.in_try);
                ctx.shared.borrow_mut().breaks.push(Vec::new());
                let _ = walk_block(body, &mut inner, &inner_ctx, out);
                let breaks = ctx.shared.borrow_mut().breaks.pop().unwrap_or_default();
                // [linear-obligation] After the loop: the paths that leave it —
                // a `for`'s top (where its iterable runs out, the entry set)
                // and every `break` — joined, for what was owed before it (what
                // a `break` still owed of its own was its leak). A value
                // settled on some of them but not the others is the
                // some-paths error, at the written loop.
                let mut exits: Vec<Owed> = Vec::new();
                if matches!(s, Stmt::ForEach { .. }) {
                    exits.push(base.clone());
                }
                exits.extend(breaks.into_iter().map(|mut b| {
                    b.retain(|k, _| base.contains_key(k));
                    b
                }));
                if !exits.is_empty() {
                    let span = ctx.fate.loop_spans.get(&id.0).copied().unwrap_or(body.span);
                    *owed = join_paths(&base, exits, span, ctx, out);
                }
            }
            Stmt::Let { value, .. } | Stmt::Expr(value) => {
                if walk_expr(value, owed, ctx, out) == Flow::Leaves {
                    return Flow::Leaves;
                }
            }
            Stmt::Assign { place, value } => {
                if walk_expr(value, owed, ctx, out) == Flow::Leaves {
                    return Flow::Leaves;
                }
                // [linear-obligation] Assigning over a local that still owes
                // drops its value (a written assignment; the builder's own —
                // a loop's result, a pick — never assign over an obligation).
                // Reported once: the old obligation goes with the report, and
                // the new value's `Gain` follows.
                if place.steps.is_empty() {
                    if let Some(at) = ctx.fate.assign_spans.get(&(value.span.start, value.span.end)) {
                        let key = place_key(place);
                        if owed.remove(&key).is_some() && !key.starts_with("__") {
                            let x = written_name(&key);
                            out.push(IrDiagnostic {
                                span: *at,
                                message: format!(
                                    "assigning to `{x}` drops the linear value it still owns; move it onward or `discard({x})` first"
                                ),
                                discharge_type: None,
                                leak: false,
                            });
                        }
                    }
                }
            }
            Stmt::Narrow { local, from, .. } => {
                ctx.shared.borrow_mut().narrow_of.insert(local.0.clone(), place_key(from));
            }
            Stmt::Alias { .. } | Stmt::Unpack { .. } => {}
        }
    }
    if let Some(v) = &block.value {
        if walk_expr(v, owed, ctx, out) == Flow::Leaves {
            return Flow::Leaves;
        }
    }
    Flow::FallsThrough
}

/// Walk an expression in evaluation order, updating `owed` and reporting leaks
/// at every point control may leave: a `Never`-typed expression (`throw`, an
/// `unreachable` trap, a `-> Never` call), and the throw edge of a call to a
/// `throws` fn. Returns whether evaluation falls through to the expression's
/// value or leaves.
fn walk_expr(e: &Expr, owed: &mut Owed, ctx: &Ctx, out: &mut Vec<IrDiagnostic>) -> Flow {
    match &e.kind {
        ExprKind::Branch { arms, otherwise, .. } => {
            let mut falls: Vec<Owed> = Vec::new();
            for (i, (cond, body)) in arms.iter().enumerate() {
                // The condition is evaluated on the current path, before the
                // fork — a `Never` or throwing call in it leaves here.
                if walk_expr(cond, owed, ctx, out) == Flow::Leaves {
                    return Flow::Leaves;
                }
                let mut path = owed.clone();
                // An earlier arm's positive arm-test failed to get here: its
                // arms are proven absent on this path.
                for (earlier, _) in &arms[..i] {
                    if let Some((subject, tested)) = positive_arm_test(earlier) {
                        discharge_arms(&mut path, &subject, &tested);
                    }
                }
                // [ir-obligation] A negated arm-test `!(x is arm k)` guarding
                // this arm proves `x` is not in arm k *inside* the arm.
                if let Some((subject, tested)) = negative_arm_test(cond) {
                    discharge_arms(&mut path, &subject, &tested);
                }
                if walk_block(body, &mut path, ctx, out) == Flow::FallsThrough {
                    falls.push(path);
                }
            }
            match otherwise {
                Some(body) => {
                    // Every arm's test failed: what a positive one tested is
                    // proven absent in the `else`.
                    let mut path = owed.clone();
                    for (cond, _) in arms {
                        if let Some((subject, tested)) = positive_arm_test(cond) {
                            discharge_arms(&mut path, &subject, &tested);
                        }
                    }
                    if walk_block(body, &mut path, ctx, out) == Flow::FallsThrough {
                        falls.push(path);
                    }
                }
                None => {
                    // No `otherwise`: the subject may skip every arm — *unless*
                    // an arm is unconditional (`cond == true`, which a
                    // `block_expr` wrapper and a desugared `else` produce),
                    // in which case that arm is always taken and there is no
                    // skip-past path. Adding one would merge back obligations
                    // the always-taken arm had already released (a false leak,
                    // found 2026-10-12 on `x ?: unreachable!()` whose wrapper
                    // released a consumed parameter before the pick).
                    let unconditional = arms.iter().any(|(c, _)| matches!(c.kind, ExprKind::Bool(true)));
                    if !unconditional {
                        // Falling past a positive arm-test (`x is arm k`)
                        // proves `x` is not in that arm, discharging its
                        // obligation on the skip path.
                        let mut path = owed.clone();
                        for (cond, _) in arms {
                            if let Some((subject, tested)) = positive_arm_test(cond) {
                                discharge_arms(&mut path, &subject, &tested);
                            }
                        }
                        falls.push(path);
                    }
                }
            }
            if falls.is_empty() {
                *owed = Owed::default();
                return Flow::Leaves;
            }
            let base = owed.clone();
            *owed = join_paths(&base, falls, e.span, ctx, out);
            Flow::FallsThrough
        }
        ExprKind::Switch { subject, arms, .. } => {
            if walk_expr(subject, owed, ctx, out) == Flow::Leaves {
                return Flow::Leaves;
            }
            // [ir-obligation] A switch dispatches on its subject: the subject's
            // obligation is distributed to the per-arm narrows, so it leaves
            // the set on entry. A narrowed binding an arm drops is still caught.
            let before = owed.clone();
            let mut base = owed.clone();
            if let ExprKind::Read { place, .. } = &subject.kind {
                base.remove(&place_key(place));
            }
            let mut falls: Vec<Owed> = Vec::new();
            for arm in arms {
                let mut path = base.clone();
                if walk_block(&arm.body, &mut path, ctx, out) == Flow::FallsThrough {
                    falls.push(path);
                }
            }
            if falls.is_empty() {
                *owed = Owed::default();
                return Flow::Leaves;
            }
            // The subject is owed on an arm's path while that arm's narrowing
            // of it is.
            *owed = join_paths(&before, falls, e.span, ctx, out);
            Flow::FallsThrough
        }
        ExprKind::Try { body } => {
            // A `try` catches throws: inside it, a throwing call is not a fn
            // leave. The body's own value flows out.
            let inner = ctx.with(ctx.loop_base, true);
            walk_block(body, owed, &inner, out)
        }
        // [ir-obligation] `assert!(subject is arm k)` proves, past this point,
        // that `subject` is in arm `k` — so its *other* arms are proven absent
        // and their obligations are discharged. (A failed assert traps, so the
        // continuation only runs when the test held.) This is why a value
        // narrowed by an `assert!` need not be re-handled: the obligation on
        // the ruled-out linear arm is gone, exactly as it is inside a `when`
        // arm or past an else-less branch's positive test.
        ExprKind::Assert { cond, .. } => {
            if let Some((subject, tested)) = positive_arm_test(cond) {
                discharge_complement(owed, &subject, &tested);
            }
            Flow::FallsThrough
        }
        ExprKind::WaitFor { body, .. } => {
            // [actor-waitfor] The reply token minted here must be sent to
            // within the body: anything gained inside the body (the token)
            // and still outstanding at its end leaks. Outer obligations pass
            // through unchanged.
            let base = owed.clone();
            let flow = walk_block(body, owed, ctx, out);
            if flow == Flow::FallsThrough {
                report_since(owed, Some(&base), out, body.span);
                // The body-gained obligations are settled-or-reported; keep
                // only what pre-existed for the continuation.
                owed.retain(|k, _| base.contains_key(k));
            }
            Flow::FallsThrough
        }
        _ => {
            // Evaluate operands left-to-right; any that leaves stops the rest.
            let mut flow = Flow::FallsThrough;
            each_operand(e, &mut |sub| {
                if flow == Flow::FallsThrough && walk_expr(sub, owed, ctx, out) == Flow::Leaves {
                    flow = Flow::Leaves;
                }
            });
            if flow == Flow::Leaves {
                return Flow::Leaves;
            }
            // The throw edge of a call to a `throws` fn: control may leave the
            // fn here (unless a `try` catches it), so the set must be empty.
            if !ctx.in_try {
                if let ExprKind::Call { target: FnRef::Decl(id), .. } = &e.kind {
                    if ctx.throws.get(id).copied().unwrap_or(false) {
                        report_all_msg(owed, out, e.span, "across a call that may throw: the code after it does not run on the throw path, so release it before the call, or move it onward so the obligation travels with it");
                    }
                }
            }
            // A `throw` leaves the fn with the error propagating, so nothing
            // may be outstanding — the same rule as `return`. Other
            // `Never`-typed expressions (an `unreachable` trap, a diverging
            // call like `exit_process`) *terminate* the path rather than
            // propagate out of it: control never continues, there is no "after"
            // to leak into, so they end the path without a discharge
            // requirement.
            if matches!(e.kind, ExprKind::Throw { .. }) {
                report_all(owed, out, e.span);
                return Flow::Leaves;
            }
            if e.ty == Ty::Never {
                return Flow::Leaves;
            }
            Flow::FallsThrough
        }
    }
}

/// Visit the immediate operand expressions of `e` in evaluation order (not
/// descending into branch/switch/try, which `walk_expr` handles, nor into
/// lambda bodies, which are their own fns).
fn each_operand(e: &Expr, f: &mut impl FnMut(&Expr)) {
    match &e.kind {
        ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::Tuple(args) | ExprKind::List(args) | ExprKind::Array(args) | ExprKind::Concat(args) => {
            for a in args {
                f(a);
            }
        }
        ExprKind::MemberCall { instance, args, .. } | ExprKind::HandlerCall { instance, args, .. } => {
            f(instance);
            for a in args {
                f(a);
            }
        }
        ExprKind::Construct { fields } => {
            for (_, v) in fields {
                f(v);
            }
        }
        ExprKind::Send { addr, args, .. } => {
            f(addr);
            for a in args {
                f(a);
            }
        }
        ExprKind::SelfSend { args, .. } => {
            for a in args {
                f(a);
            }
        }
        ExprKind::ReplyTo { captures, .. } => {
            for c in captures {
                f(c);
            }
        }
        ExprKind::Spawn { handler, deps, .. } => {
            f(handler);
            for d in deps {
                f(d);
            }
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::Rewrap { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::BorrowAs { value, .. }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value }
        | ExprKind::UnionToStr { value, .. }
        | ExprKind::AddrInstance { addr: value }
        | ExprKind::Handle { instance: value }
        | ExprKind::Throw { message: value } => f(value),
        _ => {}
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
                        ty_name: r.ty_name.clone(),
                        arm_names: r.arm_names.clone(),
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

/// [linear-obligation] The set after a fork whose `falls` paths come back
/// together at `span`, from `base` before it: their union, except that a
/// value `base` owed which some paths settled (moved, discharged) and others
/// still owe — itself or through a narrowing of it — is the dual of the
/// maybe-moved rule, reported once here and owed no longer (the checker's
/// former `merge_fallthrough` rule, in its words).
fn join_paths(base: &Owed, falls: Vec<Owed>, span: Span, ctx: &Ctx, out: &mut Vec<IrDiagnostic>) -> Owed {
    let shared = ctx.shared.borrow();
    // Whether `key` is (a narrowing, at any depth, of) `of`.
    let descends = |key: &str, of: &str| {
        let mut at = key;
        for _ in 0..64 {
            if at == of {
                return true;
            }
            match shared.narrow_of.get(at) {
                Some(next) => at = next,
                None => return false,
            }
        }
        false
    };
    let mut conflicts: Vec<Key> = Vec::new();
    if falls.len() > 1 {
        for k in base.keys() {
            if k.starts_with("__") {
                continue;
            }
            let owing = falls.iter().filter(|p| p.keys().any(|q| descends(q, k))).count();
            if owing > 0 && owing < falls.len() {
                conflicts.push(k.clone());
            }
        }
    }
    let mut joined: Option<Owed> = None;
    for p in falls {
        joined = Some(join(joined, p));
    }
    let mut joined = joined.unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    for k in &conflicts {
        joined.retain(|q, _| !descends(q, k));
        names.push(written_name(k).to_string());
    }
    names.sort();
    names.dedup();
    for name in names {
        out.push(IrDiagnostic {
            span,
            message: format!(
                "`{name}` owns a linear value that is consumed on some paths but not others; discharge or move it on every path"
            ),
            discharge_type: None,
            leak: false,
        });
    }
    joined
}

/// A local's name as written: the builder suffixes a rebinding (`x~2`).
fn written_name(key: &str) -> &str {
    match key.find('~') {
        Some(i) => &key[..i],
        None => key,
    }
}

fn report_all(owed: &Owed, out: &mut Vec<IrDiagnostic>, _at: Span) {
    for (place, o) in owed {
        report_one(place, o, out, "when the path leaves; move it onward (pass, return, or store it) or discharge it");
    }
}

/// Report every outstanding obligation at a conditional leave (a throwing
/// call), with a context phrase. The obligation stays owed on the normal
/// (non-throw) path, so the caller does not clear the set.
fn report_all_msg(owed: &Owed, out: &mut Vec<IrDiagnostic>, _at: Span, tail: &str) {
    for (place, o) in owed {
        report_one(place, o, out, tail);
    }
}

/// Report obligations outstanding at a `break`/`continue`: only those gained
/// *inside* the loop (not present at the loop entry `base`). An obligation
/// inherited from outside the loop is settled after the loop, not here.
fn report_since(owed: &Owed, base: Option<&Owed>, out: &mut Vec<IrDiagnostic>, _at: Span) {
    for (place, o) in owed {
        let inherited = base.is_some_and(|b| b.contains_key(place));
        if !inherited {
            report_one(place, o, out, "when the path leaves; move it onward (pass, return, or store it) or discharge it");
        }
    }
}

fn report_one(place: &str, o: &Outstanding, out: &mut Vec<IrDiagnostic>, tail: &str) {
    // A synthetic local (a desugaring temporary like `__pick~1`) has no
    // user-facing name; describe it as the value rather than exposing the
    // internal name.
    let subject = if place.starts_with("__") { "this value".to_string() } else { format!("`{place}`") };
    let value = match &o.arms {
        Some(arms) if !arms.is_empty() => {
            // Name each live linear arm by its type (`the `Checked<FsError>`
            // arm`) rather than a bare index, falling back to the index only
            // when the arm's name was not recorded.
            let a: Vec<String> = arms
                .iter()
                .map(|i| match o.arm_names.iter().find(|(idx, _)| idx == i) {
                    Some((_, name)) => format!("the `{name}` arm"),
                    None => format!("arm {i}"),
                })
                .collect();
            format!("{subject} still owns a linear value ({})", a.join(", "))
        }
        _ => format!("{subject} still owns a linear value"),
    };
    out.push(IrDiagnostic { span: o.span, message: format!("{value} {tail}"), discharge_type: o.ty_name.clone(), leak: true });
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

/// Discharge the arms of `subject` that are *not* in `held`: a positive proof
/// `subject is held` (an `assert!`) rules the others out. If no linear arm
/// remains, the obligation is settled.
fn discharge_complement(owed: &mut Owed, subject: &Key, held: &[usize]) {
    let Some(o) = owed.get_mut(subject) else { return };
    if let Some(live) = &mut o.arms {
        live.retain(|a| held.contains(a));
        if live.is_empty() {
            owed.remove(subject);
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
        Stmt::Return(Some(e), _) | Stmt::Expr(e) | Stmt::Let { value: e, .. } | Stmt::Assign { value: e, .. } => e.span,
        _ => Span::default(),
    }
}
