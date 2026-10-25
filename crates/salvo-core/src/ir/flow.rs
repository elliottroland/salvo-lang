//! [flow-use-after-move] The IR's use-after-move pass (03-generate-02-flow,
//! PIPELINE.md R8): per path, the set of wholly moved locals, and the places
//! moved *out of* each local [fate-partial-move]; a later read of either is
//! the error. It is the only use-after-move diagnostic (since piece 3a the
//! checker reports no consumed read, and since piece 3b no partial-move
//! use): `ir::analyze` runs it per declaration, and `tests/flow_tests.rs`
//! pins it.
//!
//! [fate-partial-move] A `Read { consume: true }` of a projection (not
//! `moved`) moves that place out of its root's storage — composed through
//! narrowings of projections, and for the move through any binding of one —
//! and leaves the root usable. A read of the whole local is a use of it; a
//! read of a projection is a use when it, or a shorter projection of it,
//! overlaps a moved place ([`crate::place::Place::overlaps`]). Assigning a
//! projection puts back what it covers, before its value is read; assigning
//! the whole local puts back everything; joins union the places, and the
//! revivals after a branch or a loop leave them alone — the checker's
//! `moved_places` rules (`revive_moved_places`, `merge_fallthrough`).
//!
//! What moves a local: a `Read { consume: true }` of the whole local (no
//! steps) whose declared type does not move for free
//! ([`crate::types::moves_for_free`]: a Copy scalar is copied); a `Read {
//! moved: true }` — a move the checker made that emission does not (a fn
//! value given to a consuming parameter, a projection taking its root) —
//! of the place's root, whatever its type; a `Call { target:
//! FnRef::Local(l), moved: true }` of `l` (a `once` fn called), after its
//! arguments; and a lambda `Capture { consumed: true }`, at the lambda. What
//! uses one: any read of a place rooted at it, consumed or not, at any depth
//! of steps, and a call of it. What revives one: an assignment of the whole
//! local, and a fresh `Let` of it.
//!
//! The joins and the loops **mirror the checker's rules exactly**, including
//! where those rules are unsound (PIPELINE.md §8, R8 piece 1): they were
//! built to agree with the checker's, so its version could be switched off;
//! tightening them (ROADMAP 0o) is a later, separate change.
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
    /// [fate-partial-move] A read of a value something was moved *out of*,
    /// rather than of a value moved wholly.
    pub part: Option<PartUse>,
}

/// [fate-partial-move] What a use of a partially moved value read, and
/// what had left: `place` is the place read (`p` for the whole value,
/// `p.tags` for an overlapping one), `moved` the place moved out (`p.tags`),
/// both as written.
pub struct PartUse {
    pub place: String,
    pub moved: String,
    /// Whether the read is of the whole value (the place is the root).
    pub whole: bool,
}

impl FlowDiagnostic {
    /// The message a gate shows, naming where the move was: `content` is
    /// the source of the file the diagnostic is in (a move is always in the
    /// same declaration as its use).
    pub fn render(&self, content: &str) -> String {
        let (line, _) = salvo_syntax::span::line_col(content, self.moved_at.start);
        match &self.part {
            None => format!(
                "`{}` cannot be used here: it was consumed (moved) at line {line}; reassign it before use, or `copy` it where it is moved",
                self.local
            ),
            Some(p) => part_message(&self.local, p, &format!(" at line {line}")),
        }
    }
}

/// [fate-partial-move] The two shapes of a partial-move use: the whole
/// value (move the rest individually, or `copy` at the move) and a place
/// overlapping the moved one. `at` names the move (" at line 3"), or is
/// empty.
fn part_message(root: &str, p: &PartUse, at: &str) -> String {
    let moved = &p.moved;
    if p.whole {
        format!(
            "`{root}` cannot be used as a whole here: `{moved}` was moved out of it{at}, so part of the value is gone; \
             move the remaining parts individually, or `copy` at the site that moved `{moved}`"
        )
    } else {
        format!(
            "`{}` cannot be used here: `{moved}` was moved out of `{root}`{at}, and this reads the same data; \
             `copy` at the site that moved it to keep this readable",
            p.place
        )
    }
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
    let mut moved = Moved::default();
    w.walk_block(body, &mut moved);
    out.append(&mut w.out);
}

/// A projection path out of a local, in the checker's terms
/// ([`crate::place::Step`]: a field, a tuple position, an element at an
/// unknown index), so overlap is [`crate::place::Place::overlaps`].
type Path = Vec<crate::place::Step>;

/// The flow state on one path: the wholly moved locals, each with the span
/// of its move; and [fate-partial-move] the places moved *out of* each
/// local (keyed by its storage), in the order they left, each with the span
/// of its move.
#[derive(Clone, Default, PartialEq)]
struct Moved {
    whole: BTreeMap<String, Span>,
    parts: BTreeMap<String, Vec<(Path, Span)>>,
}

impl Moved {
    /// [fate-partial-move] `path` moved out of `root` at `span`; a place
    /// already moved keeps its first move.
    fn add_part(&mut self, root: &str, path: Path, span: Span) {
        let list = self.parts.entry(root.to_string()).or_default();
        if !list.iter().any(|(p, _)| *p == path) {
            list.push((path, span));
        }
    }

    /// A fresh binding of `local`: nothing moved, wholly or in part.
    fn forget(&mut self, local: &str) {
        self.whole.remove(local);
        self.parts.remove(local);
    }
}

/// An IR projection step in the checker's place terms.
fn place_step(s: &Step) -> crate::place::Step {
    match s {
        Step::Field(f) => crate::place::Step::Field(f.clone()),
        Step::Tuple(i) => crate::place::Step::Index(*i),
        Step::Index(_) => crate::place::Step::Element,
    }
}

/// Whether `a` is a (may-alias) prefix of `b` — [`crate::place::Place::is_prefix_of`].
fn is_prefix(a: &[crate::place::Step], b: &[crate::place::Step]) -> bool {
    let place = |p: &[crate::place::Step]| crate::place::Place { root: String::new(), path: p.to_vec() };
    place(a).is_prefix_of(&place(b))
}

/// Whether two paths out of one root may name the same storage.
fn overlaps(a: &[crate::place::Step], b: &[crate::place::Step]) -> bool {
    is_prefix(a, b) || is_prefix(b, a)
}

/// A place as written: `p`, `p.tags`, `xs[?]`.
fn show_place(root: &str, path: &[crate::place::Step]) -> String {
    let mut out = root.to_string();
    for s in path {
        out.push_str(&s.to_string());
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    FallsThrough,
    Leaves,
}

/// A binding of a projection: the local it is part of, and the path.
#[derive(Clone)]
struct PartOf {
    root: String,
    path: Path,
    /// A narrowing (`Narrow`): to the checker, a read through it reads the
    /// place itself. An `Alias` is a variable of its own there.
    view: bool,
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
    /// root: reading it reads the root. With the projection's path, and
    /// whether it is a narrowing (`Narrow`, the checker's same place) rather
    /// than a binding the checker keeps as a variable of its own (`Alias`).
    part_of: HashMap<String, PartOf>,
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
            if let Some(span) = moved.whole.get(at) {
                return Some((at.to_string(), *span));
            }
            match self.part_of.get(at) {
                Some(p) => at = &p.root,
                None => return None,
            }
        }
        None
    }

    /// Reports a use of a wholly moved local; whether it did.
    fn report_use(&mut self, local: &str, span: Span, moved: &Moved) -> bool {
        let Some((name, moved_at)) = self.moved_use(local, moved) else { return false };
        let written = written_name(self.storage(&name)).to_string();
        if written.starts_with("__") {
            return false;
        }
        if !self.reported.insert((span.start, span.end)) {
            return true;
        }
        self.out.push(FlowDiagnostic {
            span,
            message: format!("`{written}` cannot be used here: it was consumed (moved) earlier; reassign it before use"),
            local: written,
            moved_at,
            part: None,
        });
        true
    }

    /// [fate-partial-move] The storage a place rooted at `local` is part of,
    /// and the whole path out of it: through whole narrowings and aliases,
    /// and through narrowings of projections — and, when `moves`, through
    /// every binding of a projection (the checker's move follows the fate
    /// link to its root; a read of such a binding reads a variable of its
    /// own, which nothing moves out of).
    fn place_of(&self, local: &str, steps: &[Step], moves: bool) -> Option<(String, Path)> {
        let mut at = self.storage(local).to_string();
        let mut path: Path = steps.iter().map(place_step).collect();
        for _ in 0..64 {
            match self.part_of.get(&at) {
                Some(p) if p.view || moves => {
                    let mut full = p.path.clone();
                    full.extend(path);
                    path = full;
                    at = self.storage(&p.root).to_string();
                }
                Some(_) => return None,
                None => break,
            }
        }
        Some((at, path))
    }

    /// [fate-partial-move] A read of `steps` out of `local` where something
    /// was moved out of its storage: the checker's `check_moved_place`, at
    /// each projection of the read from the innermost (`n.a` of `n.a.b`
    /// first, as the checker checks a field read's base), the whole value
    /// only when the read is of it. A wholly moved local is the other
    /// diagnostic's (`report_use`).
    fn report_part_use(&mut self, local: &str, steps: &[Step], span: Span, moved: &Moved) {
        let Some((root, path)) = self.place_of(local, steps, false) else { return };
        let Some(parts) = moved.parts.get(&root) else { return };
        let written = written_name(&root).to_string();
        if written.starts_with("__") {
            return;
        }
        let lens: Vec<usize> = if path.is_empty() { vec![0] } else { (1..=path.len()).collect() };
        for len in lens {
            let read = &path[..len];
            let Some((gone, moved_at)) = parts.iter().find(|(p, _)| overlaps(p, read)).cloned() else { continue };
            if !self.reported.insert((span.start, span.end)) {
                return;
            }
            let part = PartUse { place: show_place(&written, read), moved: show_place(&written, &gone), whole: read.is_empty() };
            self.out.push(FlowDiagnostic { span, message: part_message(&written, &part, ""), local: written, moved_at, part: Some(part) });
            return;
        }
    }

    /// [fate-partial-move] A consuming read of a projection moves that
    /// place out of its root's storage; the root stays usable.
    fn mark_part_moved(&mut self, local: &str, steps: &[Step], span: Span, moved: &mut Moved) {
        if let Some((root, path)) = self.place_of(local, steps, true) {
            if path.is_empty() {
                return;
            }
            moved.add_part(&root, path, span);
        }
    }

    /// [fate-partial-move] Assigning a place puts back every moved place
    /// it covers (the checker's `revive_moved_places`): itself and anything
    /// under it; the whole local covers everything.
    fn revive_parts(&self, local: &str, steps: &[Step], moved: &mut Moved) {
        let Some((root, path)) = self.place_of(local, steps, true) else { return };
        if let Some(list) = moved.parts.get_mut(&root) {
            list.retain(|(p, _)| !is_prefix(&path, p));
            if list.is_empty() {
                moved.parts.remove(&root);
            }
        }
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

    /// `local` is moved at `span` by a consuming read, unless its declared
    /// type moves for free.
    fn mark_moved(&mut self, local: &str, read_ty: &Ty, span: Span, moved: &mut Moved) {
        let ty = self.decl_ty.get(self.storage(local)).unwrap_or(read_ty);
        if crate::types::moves_for_free(ty) {
            return;
        }
        self.mark_moved_whole(local, span, moved);
    }

    /// `local` is moved at `span`, whatever its type: a consumed capture, or
    /// a move the checker made (`moved` on a read or a call).
    fn mark_moved_whole(&mut self, local: &str, span: Span, moved: &mut Moved) {
        for l in self.group(local) {
            moved.whole.entry(l).or_insert(span);
        }
    }

    /// The local a place rooted at `local` is part of: through narrowings,
    /// aliases and bindings of projections, to the whole local underneath.
    fn root_of(&self, local: &str) -> String {
        let mut at = self.storage(local);
        for _ in 0..64 {
            match self.part_of.get(at) {
                Some(p) => at = self.storage(&p.root),
                None => break,
            }
        }
        at.to_string()
    }

    /// Revives a wholly moved local (its partial moves are not touched:
    /// the checker's `reset_assigned` leaves `moved_places` alone).
    fn revive(&self, local: &str, moved: &mut Moved) {
        for l in self.group(local) {
            moved.whole.remove(&l);
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
                moved.forget(&local.0);
                flow
            }
            Stmt::Alias { local, ty, place, .. } => {
                self.walk_steps(place, moved);
                self.bind_view(&local.0, ty, place, false, false, moved);
                Flow::FallsThrough
            }
            Stmt::Narrow { local, ty, from, because, .. } => {
                self.walk_steps(from, moved);
                let fresh = from.steps.is_empty() && self.tested_here(because, &from.root.0);
                self.bind_view(&local.0, ty, from, fresh, true, moved);
                Flow::FallsThrough
            }
            Stmt::Unpack { locals, from, .. } => {
                self.walk_steps(from, moved);
                for (l, t) in locals {
                    self.decl_ty.insert(l.0.clone(), t.clone());
                    moved.forget(&l.0);
                }
                Flow::FallsThrough
            }
            Stmt::Assign { place, value } => {
                self.walk_steps(place, moved);
                // [fate-partial-move] The checker puts a projection back
                // when it checks the target, before the value: what the value
                // moves out of the place stays moved (`p.tags = f(p.tags)`).
                if !place.steps.is_empty() {
                    self.revive_parts(&place.root.0, &place.steps, moved);
                }
                let flow = self.walk_expr(value, moved);
                if place.steps.is_empty() {
                    self.revive(&place.root.0, moved);
                    self.revive_parts(&place.root.0, &[], moved);
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
    fn bind_view(&mut self, local: &str, ty: &Ty, place: &Place, fresh: bool, view: bool, moved: &mut Moved) {
        moved.forget(local);
        if place.steps.is_empty() {
            let root = place.root.0.clone();
            let declared = self.decl_ty.get(self.storage(&root)).cloned().unwrap_or_else(|| ty.clone());
            self.decl_ty.insert(local.to_string(), declared);
            self.part_of.remove(local);
            if !fresh {
                if let Some(span) = moved.whole.get(self.storage(&root)).copied() {
                    moved.whole.insert(local.to_string(), span);
                }
            }
            self.alias_of.insert(local.to_string(), root);
        } else {
            self.decl_ty.insert(local.to_string(), ty.clone());
            self.alias_of.remove(local);
            let path = place.steps.iter().map(place_step).collect();
            self.part_of.insert(local.to_string(), PartOf { root: place.root.0.clone(), path, view });
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
            end.forget(b);
        }
        self.walk_block(body, &mut end);
        // [fate-partial-move] A changed set of moved places changes the
        // checker's snapshot too.
        if !end.whole.keys().eq(entry.whole.keys()) || end.parts != entry.parts {
            let mut again = end.clone();
            if let Some(b) = binding {
                again.forget(b);
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
            end.forget(b);
        }
        *moved = end;
    }

    fn walk_expr(&mut self, e: &Expr, moved: &mut Moved) -> Flow {
        match &e.kind {
            ExprKind::Read { place, consume, moved: checker_moved } => {
                self.walk_steps(place, moved);
                if !self.report_use(&place.root.0, e.span, moved) {
                    self.report_part_use(&place.root.0, &place.steps, e.span, moved);
                }
                if *checker_moved {
                    // The checker consumed the root here: a fn value, or a
                    // projection whose move took its root wholly.
                    let root = self.root_of(&place.root.0);
                    self.mark_moved_whole(&root, e.span, moved);
                } else if *consume && place.steps.is_empty() {
                    self.mark_moved(&place.root.0, &e.ty, e.span, moved);
                } else if *consume {
                    // [fate-partial-move] A projection moved out of its root:
                    // the checker records the place, not the variable.
                    self.mark_part_moved(&place.root.0, &place.steps, e.span, moved);
                }
                Flow::FallsThrough
            }
            // [once-fn] A call of a local is a use of it; a call the checker
            // recorded as consuming it (a `once` fn) is its move, after the
            // arguments.
            ExprKind::Call { target: FnRef::Local(l), args, moved: consumes, .. } => {
                self.report_use(&l.0, e.span, moved);
                let mut flow = Flow::FallsThrough;
                for a in args {
                    if self.walk_expr(a, moved) == Flow::Leaves {
                        flow = Flow::Leaves;
                    }
                }
                if *consumes {
                    self.mark_moved_whole(&l.0, e.span, moved);
                }
                if e.ty == Ty::Never {
                    flow = Flow::Leaves;
                }
                flow
            }
            ExprKind::FnValue(FnRef::Local(l)) => {
                self.report_use(&l.0, e.span, moved);
                Flow::FallsThrough
            }
            ExprKind::Branch { id, arms, otherwise } => {
                for (i, (cond, _)) in arms.iter().enumerate() {
                    let mut tested = Vec::new();
                    self.tested_in(cond, &mut tested);
                    self.conds.insert((id.0, i), tested);
                }
                // A block expression (one always-true arm, the builder's,
                // spanning the whole branch): the statements run in
                // sequence, nothing forks. A written `if true { … }` is a
                // branch like any other (its condition has its own span).
                if arms.len() == 1 && otherwise.is_none() && matches!(arms[0].0.kind, ExprKind::Bool(true)) && arms[0].0.span == e.span {
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
                    inner.forget(&p.local.0);
                }
                let loops = std::mem::take(&mut self.breaks);
                self.walk_block(body, &mut inner);
                self.breaks = loops;
                for c in captures {
                    if c.consumed {
                        self.mark_moved_whole(&c.local.0, e.span, moved);
                    }
                }
                Flow::FallsThrough
            }
            ExprKind::Try { body } => self.walk_block(body, moved),
            ExprKind::WaitFor { local, token_ty, body } => {
                self.decl_ty.insert(local.0.clone(), token_ty.clone());
                moved.forget(&local.0);
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

/// Moved on either path is moved; so is a place moved out on either
/// [fate-partial-move] (union, the earlier path's move first, as the
/// checker's `merge_fallthrough` joins `moved_places`).
fn join_into(acc: &mut Moved, other: Moved) {
    for (k, v) in other.whole {
        acc.whole.entry(k).or_insert(v);
    }
    for (root, list) in other.parts {
        for (path, span) in list {
            acc.add_part(&root, path, span);
        }
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
