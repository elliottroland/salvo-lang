//! [flow-use-after-move] The IR's use-after-move pass (03-generate-02-flow,
//! PIPELINE.md R8 piece 1): per path, the set of wholly moved locals; a later
//! read of one is the error. Run by tests only for now — the checker's own
//! consumed-read diagnostic is still the one a build reports — and checked
//! against it by `tests/flow_tests.rs`.
//!
//! What moves a local: a `Read { consume: true }` of the whole local (no
//! steps) whose declared type does not move for free
//! ([`crate::types::moves_for_free`]: a Copy scalar is copied), and a lambda
//! `Capture { consumed: true }`, at the lambda. What uses one: any read of a
//! place rooted at it, consumed or not, at any depth of steps. What revives
//! one: an assignment of the whole local, and a fresh `Let` of it.
//!
//! The joins and the loops **mirror the checker's rules exactly**, including
//! where those rules are unsound (PIPELINE.md §8, R8 piece 1): the point of
//! this piece is that the two agree, so the checker's version can be switched
//! off; tightening the rules is a later, separate change.
//!
//! * The walk is sequential over every statement of a block, including the
//!   dead ones after a `return`/`break`/`continue` (the checker walks them
//!   too). A block *leaves* when any of its statements does.
//! * A `Branch`/`Switch` forks the set per arm; the arms that fall through
//!   are joined by union (moved on any path is moved); when every arm leaves
//!   the set is the one before the arms. Then every local *assigned* inside
//!   an arm is revived — for a `Branch` (an `if`), only in the arms that fall
//!   through; for a `Switch` (a `when`), in every arm (the checker's
//!   `reset_assigned`).
//! * A loop body is walked once from the entry set; when its end set differs,
//!   once more from that end set (the back edge). The set after the loop is
//!   the last walk's end set joined with every `break`'s, and then every local
//!   assigned anywhere in the body is revived. A `continue` contributes
//!   nothing to the next iteration.
//! * A narrowing binding of a whole local (`Narrow`, `Alias`) is the same
//!   storage: moving it moves the local. A narrowing justified by a test *of
//!   that local* where it is made (a `Test`, an arm whose condition tests
//!   it, a switch on it) starts alive even when the local was moved — the
//!   checker re-narrows the variable at the test, which is where it reports
//!   the use; any other (`After` a branch, an enclosing arm that tested
//!   something else, a loop value, a claim) inherits the local's state.
//! * A lambda body is walked from the enclosing set (a read of a moved
//!   capture is a use), as its own loop context; what it moves stays inside.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::nodes::*;

/// One use of a moved local: where it was read, its name as written, and
/// where it was moved.
pub struct FlowDiagnostic {
    pub span: Span,
    pub message: String,
    /// The local as written (`x`, not the builder's `x~2`).
    pub local: String,
    /// The move this read comes after.
    pub moved_at: Span,
}

/// Check every fn body of `program` (handler members and `init`s included).
pub fn check(program: &Program) -> Vec<FlowDiagnostic> {
    let mut out = Vec::new();
    for module in &program.modules {
        for decl in &module.decls {
            out.extend(check_one(decl));
        }
    }
    out
}

/// Check one declaration: a fn, or a handler's members.
pub fn check_one(decl: &Decl) -> Vec<FlowDiagnostic> {
    let mut out = Vec::new();
    match decl {
        Decl::Fn(f) => check_fn(f, &mut out),
        Decl::Impl(h) => {
            for f in h.init.iter().chain(h.members.iter()) {
                check_fn(f, &mut out);
            }
        }
        _ => {}
    }
    out
}

fn check_fn(f: &FnDecl, out: &mut Vec<FlowDiagnostic>) {
    let Some(body) = &f.body else { return };
    let mut w = Walker::default();
    for p in &f.params {
        w.decl_ty.insert(p.local.0.clone(), p.ty.clone());
    }
    let mut moved = Moved::new();
    w.walk_block(body, &mut moved);
    out.append(&mut w.out);
}

/// The wholly moved locals on one path, each with the span of its move.
type Moved = BTreeMap<String, Span>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    FallsThrough,
    Leaves,
}

#[derive(Default)]
struct Walker {
    out: Vec<FlowDiagnostic>,
    /// Reported use spans: a read reached by both loop walks reports once.
    reported: HashSet<(u32, u32)>,
    /// Each local's declared type: a narrowing binding has its local's, as
    /// the checker judges a move by the variable's declared type.
    decl_ty: HashMap<String, Ty>,
    /// A narrowing binding or alias of a whole local → that local.
    alias_of: HashMap<String, String>,
    /// A narrowing binding or alias of a *projection* → the projection's
    /// root: reading it reads the root.
    part_of: HashMap<String, String>,
    /// Per enclosing loop (innermost last): the sets at its `break`s.
    breaks: Vec<Vec<Moved>>,
    /// What each test node, each branch arm's condition and each switch
    /// tests: the storage of the whole locals it reads as its subject — so a
    /// narrowing can tell a test of *its* local from an enclosing arm's.
    tests: HashMap<u32, String>,
    conds: HashMap<(u32, usize), Vec<String>>,
    switches: HashMap<u32, String>,
}

impl Walker {
    /// The storage a local names: itself, or the local a chain of whole
    /// narrowings and aliases leads to.
    fn storage<'a>(&'a self, local: &'a str) -> &'a str {
        let mut at = local;
        let mut steps = 0;
        while let Some(next) = self.alias_of.get(at) {
            at = next;
            steps += 1;
            if steps > 64 {
                break;
            }
        }
        at
    }

    /// The moved local that a read of `local` uses, if any, and its move.
    fn moved_use(&self, local: &str, moved: &Moved) -> Option<(String, Span)> {
        let mut at = local;
        for _ in 0..64 {
            if let Some(span) = moved.get(at) {
                return Some((at.to_string(), *span));
            }
            match self.part_of.get(at) {
                Some(root) => at = root,
                None => return None,
            }
        }
        None
    }

    fn report_use(&mut self, local: &str, span: Span, moved: &Moved) {
        let Some((name, moved_at)) = self.moved_use(local, moved) else { return };
        let written = written_name(self.storage(&name)).to_string();
        if written.starts_with("__") {
            return;
        }
        if !self.reported.insert((span.start, span.end)) {
            return;
        }
        self.out.push(FlowDiagnostic {
            span,
            message: format!("`{written}` cannot be used here: it was consumed (moved) earlier; reassign it before use"),
            local: written,
            moved_at,
        });
    }

    /// Every local naming the same storage as `local`.
    fn group(&self, local: &str) -> Vec<String> {
        let root = self.storage(local).to_string();
        let mut out: Vec<String> = self.alias_of.keys().filter(|l| self.storage(l) == root).cloned().collect();
        out.push(root);
        if !out.iter().any(|l| l == local) {
            out.push(local.to_string());
        }
        out
    }

    /// `local` is moved at `span`, unless its declared type moves for free.
    fn mark_moved(&mut self, local: &str, read_ty: &Ty, span: Span, moved: &mut Moved) {
        let ty = self.decl_ty.get(self.storage(local)).unwrap_or(read_ty);
        if crate::types::moves_for_free(ty) {
            return;
        }
        for l in self.group(local) {
            moved.entry(l).or_insert(span);
        }
    }

    fn mark_moved_capture(&mut self, local: &str, span: Span, moved: &mut Moved) {
        for l in self.group(local) {
            moved.entry(l).or_insert(span);
        }
    }

    fn revive(&self, local: &str, moved: &mut Moved) {
        for l in self.group(local) {
            moved.remove(&l);
        }
    }

    fn walk_block(&mut self, block: &Block, moved: &mut Moved) -> Flow {
        let mut flow = Flow::FallsThrough;
        for s in &block.stmts {
            if self.walk_stmt(s, moved) == Flow::Leaves {
                flow = Flow::Leaves;
            }
        }
        if let Some(v) = &block.value {
            if self.walk_expr(v, moved) == Flow::Leaves {
                flow = Flow::Leaves;
            }
        }
        flow
    }

    fn walk_stmt(&mut self, s: &Stmt, moved: &mut Moved) -> Flow {
        match s {
            Stmt::Loop { body, .. } => {
                self.walk_loop(body, None, moved);
                Flow::FallsThrough
            }
            Stmt::ForEach { local, ty, iterable, body, .. } => {
                let flow = self.walk_expr(iterable, moved);
                self.decl_ty.insert(local.0.clone(), ty.clone());
                self.walk_loop(body, Some(&local.0), moved);
                flow
            }
            Stmt::Let { local, ty, value, .. } => {
                let flow = self.walk_expr(value, moved);
                self.decl_ty.insert(local.0.clone(), ty.clone());
                self.alias_of.remove(&local.0);
                self.part_of.remove(&local.0);
                moved.remove(&local.0);
                flow
            }
            Stmt::Alias { local, ty, place, .. } => {
                self.walk_steps(place, moved);
                self.bind_view(&local.0, ty, place, false, moved);
                Flow::FallsThrough
            }
            Stmt::Narrow { local, ty, from, because, .. } => {
                self.walk_steps(from, moved);
                let fresh = from.steps.is_empty() && self.tested_here(because, &from.root.0);
                self.bind_view(&local.0, ty, from, fresh, moved);
                Flow::FallsThrough
            }
            Stmt::Unpack { locals, from, .. } => {
                self.walk_steps(from, moved);
                for (l, t) in locals {
                    self.decl_ty.insert(l.0.clone(), t.clone());
                    moved.remove(&l.0);
                }
                Flow::FallsThrough
            }
            Stmt::Assign { place, value } => {
                self.walk_steps(place, moved);
                let flow = self.walk_expr(value, moved);
                if place.steps.is_empty() {
                    self.revive(&place.root.0, moved);
                }
                flow
            }
            Stmt::Expr(e) => self.walk_expr(e, moved),
            Stmt::Return(value) => {
                if let Some(e) = value {
                    self.walk_expr(e, moved);
                }
                Flow::Leaves
            }
            Stmt::Break => {
                if let Some(frame) = self.breaks.last_mut() {
                    frame.push(moved.clone());
                }
                Flow::Leaves
            }
            Stmt::Continue => Flow::Leaves,
            Stmt::Obligation(_) => Flow::FallsThrough,
        }
    }

    /// Whether `because` is a test of `local` itself, made where the
    /// narrowing is: the checker re-narrows the variable there, so the
    /// narrowing starts alive. A narrowing that persists from an earlier
    /// fact — `After` a branch, or an enclosing arm whose test was of
    /// something else — is the variable as it stands.
    fn tested_here(&self, because: &Justification, local: &str) -> bool {
        let storage = self.storage(local);
        match because {
            Justification::Test { test } => self.tests.get(&test.0).is_some_and(|t| t == storage),
            Justification::Cond { branch, arm } => self.conds.get(&(branch.0, *arm)).is_some_and(|ts| ts.iter().any(|t| t == storage)),
            Justification::Arm { switch, .. } | Justification::SafeNav { switch } => self.switches.get(&switch.0).is_some_and(|t| t == storage),
            Justification::After { .. } | Justification::LoopValue { .. } | Justification::Claim => false,
        }
    }

    /// The storage of the whole local `e` reads, if it is such a read.
    fn subject_of(&self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Read { place, .. } if place.steps.is_empty() => Some(self.storage(&place.root.0).to_string()),
            _ => None,
        }
    }

    /// The locals a condition tests (`x is …`, at any depth of `&&`, `!`).
    fn tested_in(&self, cond: &Expr, out: &mut Vec<String>) {
        if let ExprKind::Test { subject, .. } = &cond.kind {
            out.extend(self.subject_of(subject));
        }
        each_operand(cond, &mut |sub| self.tested_in(sub, out));
    }

    /// A narrowing binding or alias `local` of `place`.
    fn bind_view(&mut self, local: &str, ty: &Ty, place: &Place, fresh: bool, moved: &mut Moved) {
        moved.remove(local);
        if place.steps.is_empty() {
            let root = place.root.0.clone();
            let declared = self.decl_ty.get(self.storage(&root)).cloned().unwrap_or_else(|| ty.clone());
            self.decl_ty.insert(local.to_string(), declared);
            self.part_of.remove(local);
            if !fresh {
                if let Some(span) = moved.get(self.storage(&root)).copied() {
                    moved.insert(local.to_string(), span);
                }
            }
            self.alias_of.insert(local.to_string(), root);
        } else {
            self.decl_ty.insert(local.to_string(), ty.clone());
            self.alias_of.remove(local);
            self.part_of.insert(local.to_string(), place.root.0.clone());
        }
    }

    /// The index expressions of a place, in order.
    fn walk_steps(&mut self, place: &Place, moved: &mut Moved) {
        for s in &place.steps {
            if let Step::Index(e) = s {
                self.walk_expr(e, moved);
            }
        }
    }

    /// A loop body, the checker's way: once from the entry set, and once
    /// more from the end set when the body changed it; after the loop, the
    /// end set joined with every `break`'s, then everything the body assigns
    /// revived. `binding` is a `for` binding, fresh on every iteration.
    fn walk_loop(&mut self, body: &Block, binding: Option<&str>, moved: &mut Moved) {
        self.breaks.push(Vec::new());
        let entry = moved.clone();
        let mut end = entry.clone();
        if let Some(b) = binding {
            end.remove(b);
        }
        self.walk_block(body, &mut end);
        if !end.keys().eq(entry.keys()) {
            let mut again = end.clone();
            if let Some(b) = binding {
                again.remove(b);
            }
            self.walk_block(body, &mut again);
            end = again;
        }
        let breaks = self.breaks.pop().unwrap_or_default();
        for b in breaks {
            join_into(&mut end, b);
        }
        let mut assigned = Vec::new();
        assigned_in_block(body, &mut assigned);
        for name in assigned {
            self.revive(&name, &mut end);
        }
        if let Some(b) = binding {
            end.remove(b);
        }
        *moved = end;
    }

    fn walk_expr(&mut self, e: &Expr, moved: &mut Moved) -> Flow {
        match &e.kind {
            ExprKind::Read { place, consume } => {
                self.walk_steps(place, moved);
                self.report_use(&place.root.0, e.span, moved);
                if *consume && place.steps.is_empty() {
                    self.mark_moved(&place.root.0, &e.ty, e.span, moved);
                }
                Flow::FallsThrough
            }
            ExprKind::Branch { id, arms, otherwise } => {
                for (i, (cond, _)) in arms.iter().enumerate() {
                    let mut tested = Vec::new();
                    self.tested_in(cond, &mut tested);
                    self.conds.insert((id.0, i), tested);
                }
                // A block expression (one always-true arm): the statements
                // run in sequence, nothing forks.
                if arms.len() == 1 && otherwise.is_none() && matches!(arms[0].0.kind, ExprKind::Bool(true)) {
                    return self.walk_block(&arms[0].1, moved);
                }
                let mut falls: Vec<Moved> = Vec::new();
                let mut revived: Vec<&Block> = Vec::new();
                for (cond, body) in arms {
                    self.walk_expr(cond, moved);
                    let mut path = moved.clone();
                    if self.walk_block(body, &mut path) == Flow::FallsThrough {
                        falls.push(path);
                        revived.push(body);
                    }
                }
                match otherwise {
                    Some(body) => {
                        let mut path = moved.clone();
                        if self.walk_block(body, &mut path) == Flow::FallsThrough {
                            falls.push(path);
                            revived.push(body);
                        }
                    }
                    None => {
                        if !arms.iter().any(|(c, _)| matches!(c.kind, ExprKind::Bool(true))) {
                            falls.push(moved.clone());
                        }
                    }
                }
                let flow = if falls.is_empty() { Flow::Leaves } else { Flow::FallsThrough };
                self.join(falls, moved);
                self.revive_assigned(&revived, moved);
                flow
            }
            ExprKind::Switch { id, subject, arms } => {
                if let Some(t) = self.subject_of(subject) {
                    self.switches.insert(id.0, t);
                }
                self.walk_expr(subject, moved);
                let mut falls: Vec<Moved> = Vec::new();
                for arm in arms {
                    let mut path = moved.clone();
                    if self.walk_block(&arm.body, &mut path) == Flow::FallsThrough {
                        falls.push(path);
                    }
                }
                let flow = if falls.is_empty() { Flow::Leaves } else { Flow::FallsThrough };
                self.join(falls, moved);
                // A `when` resets what *any* arm assigns, leaving or not.
                let all: Vec<&Block> = arms.iter().map(|a| &a.body).collect();
                self.revive_assigned(&all, moved);
                flow
            }
            ExprKind::Lambda { params, body, captures, .. } => {
                let mut inner = moved.clone();
                for p in params {
                    self.decl_ty.insert(p.local.0.clone(), p.ty.clone());
                    inner.remove(&p.local.0);
                }
                let loops = std::mem::take(&mut self.breaks);
                self.walk_block(body, &mut inner);
                self.breaks = loops;
                for c in captures {
                    if c.consumed {
                        self.mark_moved_capture(&c.local.0, e.span, moved);
                    }
                }
                Flow::FallsThrough
            }
            ExprKind::Try { body } => self.walk_block(body, moved),
            ExprKind::WaitFor { local, token_ty, body } => {
                self.decl_ty.insert(local.0.clone(), token_ty.clone());
                moved.remove(&local.0);
                self.walk_block(body, moved);
                Flow::FallsThrough
            }
            _ => {
                if let ExprKind::Test { id, subject, .. } = &e.kind {
                    if let Some(t) = self.subject_of(subject) {
                        self.tests.insert(id.0, t);
                    }
                }
                let mut flow = Flow::FallsThrough;
                each_operand(e, &mut |sub| {
                    if self.walk_expr(sub, moved) == Flow::Leaves {
                        flow = Flow::Leaves;
                    }
                });
                if matches!(e.kind, ExprKind::Throw { .. } | ExprKind::Unreachable { .. }) || e.ty == Ty::Never {
                    flow = Flow::Leaves;
                }
                flow
            }
        }
    }

    /// The set after a fork: the union of the arms that fall through, or the
    /// set before the arms when every arm leaves.
    fn join(&self, falls: Vec<Moved>, moved: &mut Moved) {
        let mut it = falls.into_iter();
        let Some(mut acc) = it.next() else { return };
        for other in it {
            join_into(&mut acc, other);
        }
        *moved = acc;
    }

    fn revive_assigned(&self, blocks: &[&Block], moved: &mut Moved) {
        let mut assigned = Vec::new();
        for b in blocks {
            assigned_in_block(b, &mut assigned);
        }
        for name in assigned {
            self.revive(&name, moved);
        }
    }
}

/// Moved on either path is moved.
fn join_into(acc: &mut Moved, other: Moved) {
    for (k, v) in other {
        acc.entry(k).or_insert(v);
    }
}

/// A local's name as written: the builder suffixes a rebinding (`x~2`).
fn written_name(local: &str) -> &str {
    match local.find('~') {
        Some(i) => &local[..i],
        None => local,
    }
}

/// Every whole local assigned anywhere inside `block`, lambdas included —
/// what the checker's `collect_assigned` resets after a branch or a loop.
fn assigned_in_block(block: &Block, out: &mut Vec<String>) {
    for s in &block.stmts {
        match s {
            Stmt::Assign { place, value } => {
                if place.steps.is_empty() {
                    out.push(place.root.0.clone());
                }
                assigned_in_expr(value, out);
            }
            Stmt::Loop { body, .. } => assigned_in_block(body, out),
            Stmt::ForEach { iterable, body, .. } => {
                assigned_in_expr(iterable, out);
                assigned_in_block(body, out);
            }
            Stmt::Let { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value)) => assigned_in_expr(value, out),
            _ => {}
        }
    }
    if let Some(v) = &block.value {
        assigned_in_expr(v, out);
    }
}

fn assigned_in_expr(e: &Expr, out: &mut Vec<String>) {
    match &e.kind {
        ExprKind::Branch { arms, otherwise, .. } => {
            for (c, b) in arms {
                assigned_in_expr(c, out);
                assigned_in_block(b, out);
            }
            if let Some(b) = otherwise {
                assigned_in_block(b, out);
            }
        }
        ExprKind::Switch { subject, arms, .. } => {
            assigned_in_expr(subject, out);
            for a in arms {
                assigned_in_block(&a.body, out);
            }
        }
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => assigned_in_block(body, out),
        _ => each_operand(e, &mut |sub| assigned_in_expr(sub, out)),
    }
}

/// The operand expressions of `e` in evaluation order — everything but the
/// forms `walk_expr` handles itself (branches, switches, lambdas, `try`,
/// `wait for`). A read's index steps are its operands.
fn each_operand(e: &Expr, f: &mut impl FnMut(&Expr)) {
    match &e.kind {
        ExprKind::Read { place, .. } => {
            for s in &place.steps {
                if let Step::Index(i) = s {
                    f(i);
                }
            }
        }
        ExprKind::Call { args, .. }
        | ExprKind::Op { args, .. }
        | ExprKind::Tuple(args)
        | ExprKind::List(args)
        | ExprKind::Array(args)
        | ExprKind::Concat(args)
        | ExprKind::SelfSend { args, .. } => {
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
        ExprKind::ReplyTo { target, captures, pool, .. } => {
            if let ReplyTarget::Task { effects, .. } = target {
                for x in effects {
                    f(x);
                }
            }
            for c in captures {
                f(c);
            }
            if let Some(p) = pool {
                f(p);
            }
        }
        ExprKind::Spawn { handler, deps, pool, join, .. } => {
            f(handler);
            for d in deps {
                f(d);
            }
            if let Some(p) = pool {
                f(p);
            }
            if let Some(j) = join {
                f(j);
            }
        }
        ExprKind::Test { subject, .. } => f(subject),
        ExprKind::Assert { cond, message, .. } => {
            f(cond);
            if let Some(m) = message {
                f(m);
            }
        }
        ExprKind::Unreachable { message, .. } => {
            if let Some(m) = message {
                f(m);
            }
        }
        ExprKind::UnionToStr { value, arms } => {
            f(value);
            for a in arms {
                f(a);
            }
        }
        ExprKind::MakeUnion { value, .. }
        | ExprKind::Rewrap { value, .. }
        | ExprKind::DropMut { value }
        | ExprKind::Present { value }
        | ExprKind::BorrowAs { value, .. }
        | ExprKind::Widen { value }
        | ExprKind::Spread { value }
        | ExprKind::AddrInstance { addr: value }
        | ExprKind::Handle { instance: value }
        | ExprKind::Throw { message: value } => f(value),
        ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Unit
        | ExprKind::MakeNone
        | ExprKind::FnValue(_)
        | ExprKind::SelfAddr
        | ExprKind::Unsupported(_)
        | ExprKind::Branch { .. }
        | ExprKind::Switch { .. }
        | ExprKind::Lambda { .. }
        | ExprKind::Try { .. }
        | ExprKind::WaitFor { .. } => {}
    }
}
