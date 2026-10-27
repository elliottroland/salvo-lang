//! [rs-repoint] A re-pointed view local starts a fresh Rust variable.
//!
//! Salvo lets a local that holds a view be assigned a view of another source
//! (its links follow the assignment, [fate-link]). Rust gives a variable one
//! type, and so one lifetime: every borrow ever stored in it is held for as
//! long as the variable is live, so a first binding of a local that dies
//! early and a later one of a parameter that is returned cannot share it
//! (E0515), nor can a write to the first source happen while the second
//! binding is in use (E0502). Where the old value is dead, the assignment
//! starts a new variable of the same name instead, with a lifetime of its
//! own:
//!
//! * a plain `v = e` becomes the shadowing `let mut v = e;` when the value it
//!   stores does not flow out of the block it sits in (always, in the block
//!   that declares `v`);
//! * any other statement that assigns `v` somewhere inside it (an `if` whose
//!   arms all re-point it, a `loop` that does before its `break`) is preceded
//!   by `let mut v;` when `v`'s old value is dead where the statement starts,
//!   and its values do not flow out of the enclosing block.
//!
//! "Dead" is a backward liveness over the statement tree: an assignment
//! kills, a `return`/`break`/`continue` ends a path, any other mention reads.
//! Conservative where it is unsure (a mention inside an expression is a read,
//! a `when` may fall through, a loop may stop before it writes), so a missed
//! split costs only the old single variable, never a wrong one. Locals a
//! lambda or a `replyto` captures keep their one variable: a capture would
//! see the old binding after a shadow.

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use salvo_core::ir::{Block, Expr, ExprKind, FnRef, Stmt};
use salvo_core::types::Ty;

use super::body::{walk_block, walk_expr, walk_stmt};

/// What happens at a statement (by address).
#[derive(Clone, Debug)]
pub(crate) enum Split {
    /// A plain assignment of the local, emitted as a shadowing `let`.
    Shadow(String, Ty),
    /// Locals declared fresh (`let mut v;`) just before the statement.
    Declare(Vec<(String, Ty)>),
}

impl Split {
    /// Whether the split starts a variable for `local`.
    pub(crate) fn names(&self, local: &str) -> bool {
        match self {
            Split::Shadow(n, _) => n == local,
            Split::Declare(vs) => vs.iter().any(|(n, _)| n == local),
        }
    }
}

pub(crate) fn key(s: &Stmt) -> usize {
    s as *const Stmt as usize
}

/// The splits of a body: for every `let` of a view local (`is_view`) no
/// lambda captures, the statements after it that start a fresh variable.
pub(crate) fn plan(body: &Block, is_view: &dyn Fn(&Ty) -> bool, captured: &HashSet<String>) -> HashMap<usize, Split> {
    let mut out: HashMap<usize, Split> = HashMap::new();
    let mut blocks: Vec<&Block> = vec![body];
    collect_blocks(body, &mut blocks);
    for b in blocks {
        for (k, s) in b.stmts.iter().enumerate() {
            let Stmt::Let { local, ty, .. } = s else { continue };
            if !is_view(ty) || captured.contains(&local.0) {
                continue;
            }
            let v = Var { name: &local.0, ty };
            // The declaring block's exit ends the local's scope (and, in a
            // loop body, the next turn binds it afresh): dead there.
            v.decide(&b.stmts[k + 1..], b.value.as_deref(), false, Ctx { brk: false, cont: false }, &mut out);
        }
    }
    out
}

/// Every block of a body, nested ones included (lambda bodies too: their
/// own `let`s are planned like the fn's).
fn collect_blocks<'b>(b: &'b Block, out: &mut Vec<&'b Block>) {
    let found: RefCell<&mut Vec<&'b Block>> = RefCell::new(out);
    walk_block(b, &mut |s| match s {
        Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => found.borrow_mut().push(body),
        _ => {}
    }, &mut |e| match &e.kind {
        ExprKind::Branch { arms, otherwise, .. } => {
            found.borrow_mut().extend(arms.iter().map(|(_, b)| b));
            if let Some(o) = otherwise {
                found.borrow_mut().push(o);
            }
        }
        ExprKind::Switch { arms, .. } => found.borrow_mut().extend(arms.iter().map(|a| &a.body)),
        ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => found.borrow_mut().push(body),
        _ => {}
    });
}

/// The liveness a `break` and a `continue` continue with.
#[derive(Clone, Copy)]
struct Ctx {
    brk: bool,
    cont: bool,
}

struct Var<'v> {
    name: &'v str,
    ty: &'v Ty,
}

/// How the backward walk treats the local: a read makes it live (`reads`),
/// a plain assignment ends its value (`kills`). Plain liveness has both; the
/// escape walks (does a value written here leave the block?) ignore reads
/// inside, since the fresh variable serves them, and the declaring check
/// ignores kills too, since every write inside lands in the fresh variable.
#[derive(Clone, Copy)]
struct Mode {
    reads: bool,
    kills: bool,
}

const LIVE: Mode = Mode { reads: true, kills: true };
const ESCAPE: Mode = Mode { reads: false, kills: true };
const REACH: Mode = Mode { reads: false, kills: false };

impl Var<'_> {
    /// Decides the splits among `stmts` (then `value`, the block's tail),
    /// whose block exits with the local live (`exit`) or dead, a `break` or
    /// `continue` out of it as `ctx` says.
    fn decide(&self, stmts: &[Stmt], value: Option<&Expr>, exit: bool, ctx: Ctx, out: &mut HashMap<usize, Split>) {
        let live = self.suffixes(stmts, value, exit, ctx, LIVE);
        let escape = self.suffixes(stmts, value, exit, ctx, ESCAPE);
        let reach = self.suffixes(stmts, value, exit, ctx, REACH);
        for (i, s) in stmts.iter().enumerate() {
            if self.plain_assign(s).is_some() {
                // The value stored here is read only inside this block.
                if !escape[i + 1] {
                    out.insert(key(s), Split::Shadow(self.name.to_string(), self.ty.clone()));
                }
                continue;
            }
            self.descend(s, live[i + 1], ctx, out);
            // The old value is dead here, nothing written from here on
            // leaves the block, and some write inside is not a fresh
            // variable of its own already.
            if !live[i] && !reach[i] && self.assigns_in_place(s, out) {
                if let Split::Declare(vs) = out.entry(key(s)).or_insert_with(|| Split::Declare(Vec::new())) {
                    vs.push((self.name.to_string(), self.ty.clone()));
                }
            }
        }
    }

    /// The walk's answer where each `stmts[i]` starts, and after the last.
    fn suffixes(&self, stmts: &[Stmt], value: Option<&Expr>, exit: bool, ctx: Ctx, m: Mode) -> Vec<bool> {
        let mut at = vec![false; stmts.len() + 1];
        at[stmts.len()] = (m.reads && value.is_some_and(|e| self.reads(e))) || exit;
        for i in (0..stmts.len()).rev() {
            at[i] = self.before(&stmts[i], at[i + 1], ctx, m);
        }
        at
    }

    /// The nested statement lists of `s`, decided with the liveness each
    /// exits into.
    fn descend(&self, s: &Stmt, after: bool, ctx: Ctx, out: &mut HashMap<usize, Split>) {
        match s {
            Stmt::Expr(e) => match &e.kind {
                ExprKind::Branch { arms, otherwise, .. } => {
                    for (_, b) in arms {
                        self.decide(&b.stmts, b.value.as_deref(), after, ctx, out);
                    }
                    if let Some(o) = otherwise {
                        self.decide(&o.stmts, o.value.as_deref(), after, ctx, out);
                    }
                }
                ExprKind::Switch { arms, .. } => {
                    for a in arms {
                        self.decide(&a.body.stmts, a.body.value.as_deref(), after, ctx, out);
                    }
                }
                _ => {}
            },
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => {
                let head = self.loop_head(body, after, matches!(s, Stmt::ForEach { .. }), LIVE);
                self.decide(&body.stmts, body.value.as_deref(), head, Ctx { brk: after, cont: head }, out);
            }
            _ => {}
        }
    }

    /// The walk's answer at a loop's head: a fixpoint over the back edge
    /// (`may_exit`: the head may leave the loop, as a `for`'s does).
    fn loop_head(&self, body: &Block, after: bool, may_exit: bool, m: Mode) -> bool {
        let mut head = false;
        loop {
            let inner = self.block_live(body, head, Ctx { brk: after, cont: head }, m);
            let next = inner || (may_exit && after);
            if next == head {
                return head;
            }
            head = next;
        }
    }

    fn block_live(&self, b: &Block, exit: bool, ctx: Ctx, m: Mode) -> bool {
        let mut live = (m.reads && b.value.as_deref().is_some_and(|e| self.reads(e))) || exit;
        for s in b.stmts.iter().rev() {
            live = self.before(s, live, ctx, m);
        }
        live
    }

    /// The walk's answer where `s` starts, given the one after it.
    fn before(&self, s: &Stmt, after: bool, ctx: Ctx, m: Mode) -> bool {
        let reads = |e: &Expr| m.reads && self.reads(e);
        if let Some(value) = self.plain_assign(s) {
            return reads(value) || (!m.kills && after);
        }
        match s {
            Stmt::Expr(e) => match &e.kind {
                ExprKind::Branch { arms, otherwise, .. } => {
                    let mut live = match otherwise {
                        Some(o) => self.block_live(o, after, ctx, m),
                        None => after,
                    };
                    for (c, b) in arms.iter().rev() {
                        live = reads(c) || self.block_live(b, after, ctx, m) || live;
                    }
                    live
                }
                // A `when` may fall through (no arm taken): `after` too.
                ExprKind::Switch { subject, arms, .. } => reads(subject) || arms.iter().any(|a| self.block_live(&a.body, after, ctx, m)) || after,
                _ => reads(e) || after,
            },
            Stmt::Loop { body, .. } => self.loop_head(body, after, false, m),
            Stmt::ForEach { iterable, body, .. } => reads(iterable) || self.loop_head(body, after, true, m),
            Stmt::Return(e, _) => e.as_ref().is_some_and(|e| reads(e)),
            Stmt::Break => ctx.brk,
            Stmt::Continue => ctx.cont,
            Stmt::Obligation(_) => after,
            _ => (m.reads && self.mentioned(s)) || after,
        }
    }

    /// `v = value`, the whole local written.
    fn plain_assign<'s>(&self, s: &'s Stmt) -> Option<&'s Expr> {
        match s {
            Stmt::Assign { place, value } if place.root.0 == self.name && place.steps.is_empty() => Some(value),
            _ => None,
        }
    }

    /// Whether `s` assigns the whole local somewhere inside it, other than
    /// as a shadowing `let` already decided.
    fn assigns_in_place(&self, s: &Stmt, out: &HashMap<usize, Split>) -> bool {
        let found = Cell::new(false);
        walk_stmt(s, &mut |x| {
            if self.plain_assign(x).is_some() && !matches!(out.get(&key(x)), Some(Split::Shadow(..))) {
                found.set(true);
            }
        }, &mut |_| {});
        found.get()
    }

    /// Any mention of the local in `s`, a write inside it included.
    fn mentioned(&self, s: &Stmt) -> bool {
        let found = Cell::new(false);
        walk_stmt(s, &mut |x| {
            if self.stmt_names(x) {
                found.set(true);
            }
        }, &mut |e| {
            if self.expr_names(e) {
                found.set(true);
            }
        });
        found.get()
    }

    /// Any mention of the local in `e` (a write in a block inside it counts).
    fn reads(&self, e: &Expr) -> bool {
        let found = Cell::new(false);
        walk_expr(e, &mut |x| {
            if self.stmt_names(x) {
                found.set(true);
            }
        }, &mut |x| {
            if self.expr_names(x) {
                found.set(true);
            }
        });
        found.get()
    }

    fn stmt_names(&self, s: &Stmt) -> bool {
        match s {
            Stmt::Assign { place, .. } => place.root.0 == self.name,
            Stmt::Narrow { from, .. } | Stmt::Alias { place: from, .. } | Stmt::Unpack { from, .. } => from.root.0 == self.name,
            Stmt::ForEach { local, .. } | Stmt::Let { local, .. } => local.0 == self.name,
            _ => false,
        }
    }

    fn expr_names(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Read { place, .. } => place.root.0 == self.name,
            ExprKind::Call { target: FnRef::Local(l), .. } => l.0 == self.name,
            ExprKind::Lambda { captures, .. } => captures.iter().any(|c| c.local.0 == self.name),
            ExprKind::ReplyTo { captures, .. } => captures.iter().any(|c| self.reads(c)),
            ExprKind::WaitFor { local, .. } => local.0 == self.name,
            _ => false,
        }
    }
}
