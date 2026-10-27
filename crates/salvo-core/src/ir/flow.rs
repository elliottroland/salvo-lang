//! [flow-use-after-move] The IR's use-after-move pass (03-generate-02-flow,
//! PIPELINE.md R8): per path, the set of wholly moved locals, and the places
//! moved *out of* each local [fate-partial-move]; a later read of either is
//! the error. It is the only use-after-move diagnostic (since piece 3a the
//! checker reports no consumed read, since piece 3b no partial-move use,
//! and since piece 3c no poisoned read [fate-poison]): `ir::analyze` runs
//! it per declaration, and `tests/flow_tests.rs` pins it.
//!
//! [fate-partial-move] A `Read { consume: true }` of a projection (not
//! `moved`) moves that place out of its root's storage — composed through
//! narrowings of projections, and for the move through any binding of one —
//! and leaves the root usable. A read of the whole local is a use of it; a
//! read of a projection is a use when its place overlaps a moved one (is
//! under it or contains it — a sibling is not, at any depth;
//! [`crate::place::Place::overlaps`]). Assigning a projection puts back
//! what it covers once its value is read (a read of the moved place in the
//! value is a use, `p.tags = f(p.tags)`); assigning the whole local puts
//! back everything; joins union the places.
//!
//! What moves a local: a `Read { consume: true }` of the whole local (no
//! steps) whose value does not move for free
//! ([`crate::types::moves_for_free`] of the read's own type, or of the
//! declared type: a Copy scalar, a union narrowed to one included, is
//! copied [copy-scalar-free]); a `Read { moved: true }` — a move the
//! checker made that emission does not (a fn value given to a consuming
//! parameter, a projection taking its root) —
//! of the place's root, whatever its type (but a whole read whose own type
//! is a Copy scalar: the checker marks a narrowed union by its declared
//! type); a `Call { target:
//! FnRef::Local(l), moved: true }` of `l` (a `once` fn called), after its
//! arguments; and a lambda `Capture { consumed: true }`, at the lambda. What
//! uses one: any read of a place rooted at it, consumed or not, at any depth
//! of steps, and a call of it. What revives one: an assignment of the whole
//! local, and a fresh `Let` of it.
//!
//! The joins and the loops are **dataflow over paths** (R8 piece 5, ROADMAP
//! 0o): until then they mirrored the checker's rules, unsound corners
//! included, so its diagnostics could be switched off; the pass's rules
//! are now its own, and the checker's joins only keep its narrowing and its
//! fate links.
//!
//! [fate-poison] [fate-move-mode] **Poisoned reads** (R8 piece 3c). Per path
//! the pass also keeps the fate links of every borrow-mode binding — the
//! checker's (`FnDecl::fate`, from `Checked::bind_facts` as the decision has
//! the binding [fate-move-mode]: by the binding's statement, or an
//! assignment's value), flattened to the roots, each at
//! the projection of its root the value came from. An event on a root
//! poisons every other local linked to it at an overlapping path
//! [fate-field-disjoint]: a move (whole, at `[]`; a projection moved out,
//! at its path, on the place's storage and on every root the locals on the
//! way are linked to — the checker's `projection_move`), a reassignment of
//! the whole local (after its value), and a mutation, which the IR cannot
//! see and the checker records (`Checked::fate_mutations`: a call's `Mut`
//! argument, applied after the call; an assignment through a projection,
//! before its value), with what it spares — the acting handle, a handle to
//! the mutated thing itself under a contents mutation [deduce-field], the
//! handles a live `NotSame` claim proves apart [elem-distinct]. A poisoned
//! local is gone like a moved one (it replaces a move as the reason; a
//! join keeps the first path's poison over a plain move); a read of it
//! names the root and the event. A move-mode binding (`FateFacts::bound`:
//! the links it took ownership through — the IR does not always read its
//! root as a move, `let m = o ?: unreachable!()`) takes ownership at the
//! binding: what was derived from each root it took is poisoned, and a
//! later read of a root it took whole names the binding. Links are by
//! variable: a fresh binding of a name turns the links to its earlier
//! binding into links to no local (`GONE`, the checker's links are by id),
//! which poison nothing but still name the binding that made them. A fresh
//! binding or a reassignment revives, and sets the local's new links; a
//! loop re-walks when the links changed, as when the moves did.
//!
//! * The walk is sequential over every statement of a block, including the
//!   dead ones after a `return`/`break`/`continue` (the checker walks them
//!   too). A block *leaves* when any of its statements does.
//! * A `Branch`/`Switch` forks the state per arm; the arms that fall through
//!   are joined by union (moved on any path is moved); when every arm leaves
//!   the set is the one before the arms. An assignment revives its local on
//!   the path it runs on, so an arm that assigns and then moves leaves it
//!   moved, and an arm that assigns and leaves revives nothing after it.
//! * A loop's top-of-iteration state is the entry state joined with every
//!   back edge — the body's end state when the body falls through (a body
//!   that always leaves has none), and every `continue`'s — and the body is
//!   walked again until that state stops changing. The state after a
//!   `Loop` is its `break`s' joined (none: it never falls through, only
//!   `return`s leave it); after a `ForEach`, the top state's joined with
//!   them, since it also leaves at the top.
//! * A narrowing binding of a whole local (`Narrow`, `Alias`) is the same
//!   storage: moving it moves the local. A narrowing justified by a test *of
//!   that local* where it is made (a `Test`, an arm whose condition tests
//!   it, a switch on it) starts alive even when the local was moved — the
//!   checker re-narrows the variable at the test, which is where it reports
//!   the use; any other (`After` a branch, an enclosing arm that tested
//!   something else, a loop value, a claim) inherits the local's state.
//! * A lambda body is walked from the enclosing set (a read of a moved
//!   capture is a use), as its own loop context; what it moves stays inside.
//!
//! [fate-move-mode] **The bindings' modes** (R8 piece 8; the checker's walk
//! decided them until then). `decide` walks a declaration built with every
//! binding borrow-mode: at each move or mutation of a derived local
//! (`FateFacts::uses`, the checker's record of where one is), every binding
//! behind it — the local's own, then its roots' back along the chain as the
//! links stand on that path — that may take ownership (`FateFacts::eligible`,
//! the checker's judgment at the binding) does. The builder then builds the
//! module again from that `Decision`: a move-mode binding's value is read as
//! a move, the loops whose binding it takes iterate by value, and its links
//! are what it took ownership through (`FateFacts::bound`). `decide` also
//! keeps the virtual places [deduce-field]: a candidate at a mutation event
//! whose binding has not taken ownership on that path, and, for a
//! fields-declaring call's survivor, whose local is still live there. The
//! check walk reports a move or mutation of a derived local none of whose
//! own bindings may take ownership as the S1 error [fate-derived-readonly],
//! and does not make that move.
//!
//! [flow-use-after-move] An assignment through a projection writes into its
//! root, so a root moved or poisoned there is a use, as is a place strictly
//! inside one moved out (R8 piece 8).
//!
//! [linear-state] In a handler member (or `init`) the pass also checks that
//! an activation leaves its state whole (R8 piece 7, the checker's
//! `check_state_whole` until then): at a `return` (after its value) and at
//! the end of a body that falls off it, a state field holding obligations
//! (`FateFacts::linear_state`) moved out of on the path, wholly or in part,
//! is the error. An assignment over such a field while it is not moved out
//! drops the obligations it holds, the ledger's "assigning drops" error for a
//! field the ledger does not track.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::nodes::*;
use crate::check::{FlowLink, FlowMutation};

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
    /// [fate-poison] A read of a value poisoned by an event on a root it
    /// shares fate with, or of a root a move-mode binding took
    /// [fate-move-mode]; `moved_at` is then the event, or the binding.
    pub poison: Option<PoisonUse>,
    /// [linear-state] Not a use: an exit from a handler member (`what` it
    /// does, "return" or "end this member") that leaves the state field
    /// `local` with a hole — moved out of, wholly or in part, on this path.
    pub hole: Option<&'static str>,
    /// [fate-derived-readonly] Not a use of something gone: a move or
    /// mutation of a derived value whose binding stays borrow-mode (no
    /// binding behind it may take ownership) — the S1 error, as the
    /// checker words it.
    pub readonly: bool,
}

/// [fate-poison] Why a read of a value that is not moved itself is a use of
/// a value that is gone.
#[derive(Clone, Debug, PartialEq)]
pub enum PoisonUse {
    /// The value was bound from `root` (borrow-mode) and `root` was then
    /// `event` (`moved`, `mutated`, `reassigned`).
    Shared { root: String, event: &'static str },
    /// [fate-move-mode] The value is the root `binding` was bound from, and
    /// the binding took ownership.
    BoundAway { binding: String },
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
    /// [flow-use-after-move] Not a read: an assignment into `place`, which
    /// lies inside the moved place.
    pub write: bool,
}

impl FlowDiagnostic {
    /// The message a gate shows, naming where the move was: `content` is
    /// the source of the file the diagnostic is in (a move is always in the
    /// same declaration as its use).
    pub fn render(&self, content: &str) -> String {
        if self.hole.is_some() || self.readonly {
            return self.message.clone();
        }
        let (line, _) = salvo_syntax::span::span_line_col(content, self.moved_at);
        if let Some(p) = &self.poison {
            return poison_message(&self.local, p, &format!(" at line {line}"));
        }
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
    if p.write {
        format!(
            "`{}` cannot be assigned here: `{moved}` was moved out of `{root}`{at}, and this writes into it; \
             assign `{moved}` as a whole to put it back, or `copy` at the site that moved it",
            p.place
        )
    } else if p.whole {
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

/// [fate-poison] [fate-move-mode] The two shapes of a poisoned read: a
/// derived value whose root an event ended, and a root a move-mode binding
/// took. `at` names the event or the binding (" at line 3"), or is empty.
fn poison_message(x: &str, p: &PoisonUse, at: &str) -> String {
    match p {
        PoisonUse::BoundAway { binding } => format!(
            "`{x}` cannot be used here: `{binding}` was bound from it{at} and later moves the value, so the binding \
             took ownership; bind with `copy(...)` (e.g. `let {binding} = copy(...)`) to keep `{x}` usable, or \
             reassign it before use"
        ),
        PoisonUse::Shared { root, event } => format!(
            "`{x}` cannot be used here: it was bound from `{root}` and shares its fate, and `{root}` was {event}{at} \
             after the binding; bind it with `copy(...)` to keep an independent value, or reassign it before use"
        ),
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
    let mut w = Walker { fate: Some(&f.fate), ..Walker::default() };
    for p in &f.params {
        w.decl_ty.insert(p.local.0.clone(), p.ty.clone());
    }
    let mut moved = Moved::default();
    if w.walk_block(body, &mut moved) == Flow::FallsThrough {
        // [linear-state] Falling off the end of a member ends the activation.
        w.check_state_whole(f.fate.name_span, "end this member", &moved);
    }
    out.append(&mut w.out);
}

/// [fate-move-mode] Decides, over a declaration built with every binding
/// borrow-mode, which bindings take ownership: at every move or mutation of
/// a derived local (`FateFacts::uses`), each binding behind it — its own,
/// and its roots' back along the chain, as the links stand on that path —
/// that may take ownership does. Also [deduce-field] which bindings stay
/// virtual places: a candidate at a mutation event whose binding has not
/// taken ownership on that path (a binding decided at a later use is still
/// borrow-mode there), and, for a fields-declaring call's survivor, whose
/// local is still live. Adds to `out` (keyed by `file_idx`); answers whether
/// it decided anything, so the builder knows to build the module again.
pub fn decide(decl: &Decl, file_idx: usize, decls: &crate::decl_id::DeclIndex, out: &mut Decision) -> bool {
    let mut any = false;
    let hint = std::cell::Cell::new(0);
    let key = |s: Span| decls.key(file_idx, s, &hint);
    let mut one = |f: &FnDecl| {
        let Some(body) = &f.body else { return };
        let mut w = Walker { fate: Some(&f.fate), deciding: true, ..Walker::default() };
        for p in &f.params {
            w.decl_ty.insert(p.local.0.clone(), p.ty.clone());
        }
        let mut moved = Moved::default();
        w.walk_block(body, &mut moved);
        for stamp in &w.decided {
            any = true;
            out.decided.insert(key(*stamp));
            out.modes.insert(key(*stamp));
            for o in f.fate.origins.get(&stamp).into_iter().flatten() {
                out.modes.insert(key(*o));
            }
        }
        for (stamp, place) in w.virtual_binds {
            any = true;
            out.virtual_binds.insert(key(stamp), place);
        }
    };
    match decl {
        Decl::Fn(f) => one(f),
        Decl::Impl(h) => {
            for f in h.init.iter().chain(h.members.iter()) {
                one(f);
            }
        }
        _ => {}
    }
    any
}

/// A projection path out of a local, in the checker's terms
/// ([`crate::place::Step`]: a field, a tuple position, an element at an
/// unknown index), so overlap is [`crate::place::Place::overlaps`].
type Path = Vec<crate::place::Step>;

/// The flow state on one path: the wholly moved locals, each with the span
/// of its move (or [fate-poison] the event that poisoned it); [fate-partial-move]
/// the places moved *out of* each local (keyed by its storage), in the
/// order they left, each with the span of its move; and [fate-link] the
/// fate links of each borrow-mode binding (by storage), as the checker
/// holds them.
#[derive(Clone, Default, PartialEq)]
struct Moved {
    whole: BTreeMap<String, Gone>,
    parts: BTreeMap<String, Vec<(Path, Span)>>,
    links: BTreeMap<String, Vec<FlowLink>>,
    /// [fate-move-mode] Deciding: the bindings a use on this path has made
    /// take ownership (by stamp) — from there on they hold no links, so no
    /// mutation can leave them a virtual place [deduce-field].
    decided_here: std::collections::BTreeSet<Span>,
}

/// Why a local is unusable, and where it happened: moved, poisoned
/// [fate-poison], or taken by a move-mode binding [fate-move-mode].
#[derive(Clone, PartialEq)]
struct Gone {
    span: Span,
    why: Why,
}

#[derive(Clone, PartialEq)]
enum Why {
    /// Consumed (moved) itself.
    Moved,
    /// [fate-poison] Bound from `root`, which was then `event`.
    Poisoned { root: String, event: &'static str },
    /// [fate-move-mode] A move-mode binding, `binding`, took it.
    BoundAway { binding: String },
}

impl Gone {
    fn moved(span: Span) -> Gone {
        Gone { span, why: Why::Moved }
    }
}

/// [fate-field-disjoint] Whether an event on `event` reaches a link at
/// `link`: either unknown overlaps everything.
fn overlaps_opt(link: Option<&[crate::place::Step]>, event: Option<&[crate::place::Step]>) -> bool {
    match (link, event) {
        (Some(a), Some(b)) => overlaps(a, b),
        _ => true,
    }
}

/// A link's path extended by `more` (`None` either way is unknown).
fn compose(base: Option<&[crate::place::Step]>, more: Option<&[crate::place::Step]>) -> Option<Path> {
    let (base, more) = (base?, more?);
    let mut out = base.to_vec();
    out.extend(more.iter().cloned());
    Some(out)
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

    /// A fresh binding of `local`: nothing moved, wholly or in part, and
    /// no fate links.
    fn forget(&mut self, local: &str) {
        self.whole.remove(local);
        self.parts.remove(local);
        self.links.remove(local);
        // A link names the variable it was made to: the checker's links are
        // by variable, so one to an earlier binding of this name (an earlier
        // iteration's `let`) is not to this one. It stays as a link to no
        // local (`GONE`), poisoning nothing, for the binding that made it
        // [fate-move-mode]: a use of the descendant decides it too.
        for ls in self.links.values_mut() {
            for l in ls.iter_mut() {
                if l.root == local {
                    l.root = GONE.to_string();
                }
            }
            dedupe_links(ls);
        }
    }

    /// [fate-link] `local`'s links from now on: the checker's, or none.
    fn set_links(&mut self, local: &str, links: Option<&Vec<FlowLink>>) {
        match links {
            Some(ls) if !ls.is_empty() => {
                self.links.insert(local.to_string(), ls.clone());
            }
            _ => {
                self.links.remove(local);
            }
        }
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

/// The states a loop body leaves by: each `break`'s (the loop's exits) and
/// each `continue`'s (its back edge, with the body's end).
#[derive(Default)]
struct LoopExits {
    breaks: Vec<Moved>,
    continues: Vec<Moved>,
}

/// A cap on a loop's fixpoint: the state only grows (a join is a union),
/// so it settles long before; the cap only bounds a bug.
const MAX_LOOP_WALKS: usize = 32;

#[derive(Default)]
struct Walker<'f> {
    /// [fate-poison] The fn's flow-only facts.
    fate: Option<&'f FateFacts>,
    out: Vec<FlowDiagnostic>,
    /// Reported use spans: a read reached by several loop walks reports once.
    reported: HashSet<Span>,
    /// Each local's declared type: a narrowing binding has its local's.
    decl_ty: HashMap<String, Ty>,
    /// A narrowing binding or alias of a whole local → that local.
    alias_of: HashMap<String, String>,
    /// A narrowing binding or alias of a *projection* → the projection's
    /// root: reading it reads the root. With the projection's path, and
    /// whether it is a narrowing (`Narrow`, the checker's same place) rather
    /// than a binding the checker keeps as a variable of its own (`Alias`).
    part_of: HashMap<String, PartOf>,
    /// Per enclosing loop (innermost last): the sets at its `break`s and
    /// its `continue`s, on the walk in progress.
    loops: Vec<LoopExits>,
    /// What each test node, each branch arm's condition and each switch
    /// tests: the storage of the whole locals it reads as its subject — so a
    /// narrowing can tell a test of *its* local from an enclosing arm's.
    tests: HashMap<u32, String>,
    conds: HashMap<(u32, usize), Vec<String>>,
    switches: HashMap<u32, String>,
    /// Inside a lambda body: its `return` ends the lambda, not the member.
    lambdas: usize,
    /// [linear-state] The holes reported, by exit and field: a loop's walks
    /// reach an exit more than once.
    holes: HashSet<(Span, String)>,
    /// [fate-move-mode] Deciding (`decide`) rather than checking: the walk
    /// reports nothing, and collects the bindings that take ownership and
    /// the virtual places.
    deciding: bool,
    decided: HashSet<Span>,
    virtual_binds: HashMap<Span, String>,
    /// [fate-derived-readonly] Checking: the spans of the uses refused as
    /// S1 errors, whose move is then not made (one mistake, one
    /// diagnostic).
    refused: HashSet<Span>,
}

impl<'f> Walker<'f> {
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
    fn moved_use(&self, local: &str, moved: &Moved) -> Option<(String, Gone)> {
        let mut at = local;
        for _ in 0..64 {
            if let Some(gone) = moved.whole.get(at) {
                return Some((at.to_string(), gone.clone()));
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
        let Some((name, gone)) = self.moved_use(local, moved) else { return false };
        let written = written_name(self.storage(&name)).to_string();
        if written.starts_with("__") {
            return false;
        }
        if !self.reported.insert(span) {
            return true;
        }
        // [fate-poison] [fate-move-mode] A poisoned value, or a root a
        // move-mode binding took: the checker's two-site messages.
        let poison = match gone.why {
            Why::Moved => None,
            Why::Poisoned { root, event } => Some(PoisonUse::Shared { root, event }),
            Why::BoundAway { binding } => Some(PoisonUse::BoundAway { binding }),
        };
        let message = match &poison {
            Some(p) => poison_message(&written, p, ""),
            None => format!("`{written}` cannot be used here: it was consumed (moved) earlier; reassign it before use"),
        };
        self.out.push(FlowDiagnostic { span, message, local: written, moved_at: gone.span, part: None, poison, hole: None, readonly: false });
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
    /// was moved out of its storage: a use when the place read overlaps a
    /// moved one — it is under it, or contains it (the whole value
    /// included). A sibling of a moved place is not a use, at any depth
    /// (`n.a.name` after `n.a.tags`). A wholly moved local is the other
    /// diagnostic's (`report_use`).
    fn report_part_use(&mut self, local: &str, steps: &[Step], span: Span, moved: &Moved) {
        let Some((root, path)) = self.place_of(local, steps, false) else { return };
        let Some(parts) = moved.parts.get(&root) else { return };
        let written = written_name(&root).to_string();
        if written.starts_with("__") {
            return;
        }
        let Some((gone, moved_at)) = parts.iter().find(|(p, _)| overlaps(p, &path)).cloned() else { return };
        if !self.reported.insert(span) {
            return;
        }
        let part = PartUse { place: show_place(&written, &path), moved: show_place(&written, &gone), whole: path.is_empty(), write: false };
        self.out.push(FlowDiagnostic { span, message: part_message(&written, &part, ""), local: written, moved_at, part: Some(part), poison: None, hole: None, readonly: false });
    }

    /// [flow-use-after-move] An assignment into `steps` of `local` (a
    /// projection) writes into its root, which must be there: a wholly
    /// moved or poisoned root is a use (`eat(p); p.hp = 5`, rustc's E0382),
    /// and so is a place strictly inside one moved out (`p.tags.n = 1`
    /// after moving `p.tags`). Assigning the moved place itself, or one
    /// covering it, puts it back (`revive_parts`), and a sibling is not
    /// affected. Reported at the target's root as written.
    fn report_write_into(&mut self, local: &str, steps: &[Step], span: Span, moved: &Moved) {
        if self.report_use(local, span, moved) {
            return;
        }
        let Some((root, path)) = self.place_of(local, steps, false) else { return };
        let Some(parts) = moved.parts.get(&root) else { return };
        let written = written_name(&root).to_string();
        if written.starts_with("__") {
            return;
        }
        let Some((gone, moved_at)) = parts.iter().find(|(p, _)| p.len() < path.len() && is_prefix(p, &path)).cloned() else { return };
        if !self.reported.insert(span) {
            return;
        }
        let part = PartUse { place: show_place(&written, &path), moved: show_place(&written, &gone), whole: false, write: true };
        self.out.push(FlowDiagnostic { span, message: part_message(&written, &part, ""), local: written, moved_at, part: Some(part), poison: None, hole: None, readonly: false });
    }

    /// [fate-partial-move] A consuming read of a projection moves that
    /// place out of its root's storage; the root stays usable.
    fn mark_part_moved(&mut self, local: &str, steps: &[Step], span: Span, moved: &mut Moved) {
        if let Some((root, path)) = self.place_of(local, steps, true) {
            if path.is_empty() {
                return;
            }
            // [fate-poison] Moving data out poisons what was derived from
            // it — on every root the place is linked to, at the place's
            // path out of each (the checker's `projection_move`).
            for (r, p) in self.event_roots(local, steps, moved) {
                self.poison(&r, p.as_deref(), "moved", span, None, moved);
            }
            moved.add_part(&root, path, span);
        }
    }

    /// [fate-poison] The roots an event on a place rooted at `local`
    /// lands on, each with the place's path out of it: the storage the
    /// place is part of (through narrowings and bindings of projections),
    /// and every root the locals on the way are fate-linked to, their link
    /// paths extended by the rest of the place.
    fn event_roots(&self, local: &str, steps: &[Step], moved: &Moved) -> Vec<(String, Option<Path>)> {
        let mut out: Vec<(String, Option<Path>)> = Vec::new();
        let mut at = self.storage(local).to_string();
        let mut path: Option<Path> = Some(steps.iter().map(place_step).collect());
        for _ in 0..64 {
            out.push((at.clone(), path.clone()));
            for l in moved.links.get(&at).into_iter().flatten() {
                if !out.iter().any(|(r, _)| *r == l.root) {
                    out.push((l.root.clone(), compose(l.path.as_deref(), path.as_deref())));
                }
            }
            match self.part_of.get(&at) {
                Some(p) => {
                    path = compose(Some(&p.path), path.as_deref());
                    at = self.storage(&p.root).to_string();
                }
                None => break,
            }
        }
        out
    }

    /// [fate-poison] An event on `root` at `path`: every other local linked
    /// to it at an overlapping path [fate-field-disjoint] is poisoned. A
    /// mutation (the checker's `record_mutation`) names what it spares: the
    /// acting handle, a handle to the mutated thing itself under a contents
    /// mutation [deduce-field], and the handles proven apart
    /// [elem-distinct].
    fn poison(&self, root: &str, path: Option<&[crate::place::Step]>, event: &'static str, span: Span, m: Option<&FlowMutation>, moved: &mut Moved) {
        let contents_only = m.is_some_and(|m| m.contents_only);
        let victims: Vec<String> = moved
            .links
            .iter()
            .filter(|(v, _)| v.as_str() != root)
            .filter(|(v, _)| !m.is_some_and(|m| m.exempt.as_deref() == Some(v.as_str()) || m.spared.iter().any(|s| s == *v)))
            .filter(|(_, ls)| {
                ls.iter().any(|l| {
                    if l.root != root || !overlaps_opt(l.path.as_deref(), path) {
                        return false;
                    }
                    // A contents mutation spares an inline handle at
                    // exactly the mutated place.
                    if contents_only && !l.crosses {
                        if let (Some(link), Some(ev)) = (l.path.as_deref(), path) {
                            if !link.is_empty() && link == ev {
                                return false;
                            }
                        }
                    }
                    true
                })
            })
            .map(|(v, _)| v.clone())
            .collect();
        for v in victims {
            for g in self.group(&v) {
                moved.whole.insert(g, Gone { span, why: Why::Poisoned { root: root.to_string(), event } });
            }
        }
    }

    /// [fate-poison] The mutation events the checker placed at `span`.
    fn mutations_at(&self, span: Span, assign: bool) -> Vec<FlowMutation> {
        let Some(fate) = self.fate else { return Vec::new() };
        let table = if assign { &fate.assign_mutations } else { &fate.mutations };
        table.get(&span).cloned().unwrap_or_default()
    }

    fn apply_mutations(&self, events: &[FlowMutation], span: Span, moved: &mut Moved) {
        for ev in events {
            self.poison(&ev.root, ev.path.as_deref(), "mutated", span, Some(ev), moved);
        }
    }

    /// [fate-move-mode] The bindings behind a use of `local`, by stamp: the
    /// ones its links were made by, and its roots' back along the chain,
    /// as the links stand on this path (the checker's
    /// `record_move_candidates` until R8 piece 8).
    fn chain(&self, local: &str, moved: &Moved) -> Vec<Span> {
        let mut out: Vec<Span> = Vec::new();
        let mut stack = vec![self.storage(local).to_string()];
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(v) = stack.pop() {
            if !seen.insert(v.clone()) {
                continue;
            }
            for l in moved.links.get(&v).into_iter().flatten() {
                if !out.contains(&l.stamp) {
                    out.push(l.stamp);
                }
                stack.push(self.storage(&l.root).to_string());
            }
        }
        out
    }

    /// [fate-move-mode] [fate-derived-readonly] Moves and mutations of
    /// derived locals at `span`. Deciding: every binding behind each — the
    /// local's own (the checker's links at the use), then back along the
    /// chain as the links stand on this path — that may take ownership does.
    /// Checking: a local none of whose own bindings may take ownership stays
    /// borrow-mode, and the use is the S1 error; its move is then not made.
    /// Answers the refused locals.
    fn derived_uses(&mut self, uses: &[DerivedUseAt], span: Span, moved: &mut Moved) -> Vec<String> {
        let mut refused = Vec::new();
        for u in uses {
            let Some(fate) = self.fate else { continue };
            let eligible = |s: &Span| fate.eligible.contains(&s);
            if self.deciding {
                let mut chain = u.own.clone();
                for s in self.chain(&u.local, moved) {
                    if !chain.contains(&s) {
                        chain.push(s);
                    }
                }
                for stamp in chain {
                    if eligible(&stamp) {
                        self.decided.insert(stamp);
                        moved.decided_here.insert(stamp);
                    }
                }
                continue;
            }
            // Its own binding took ownership when one of them may: every
            // binding behind a use that may, does.
            if u.own.iter().any(eligible) {
                continue;
            }
            refused.push(u.local.clone());
            self.refused.insert(span);
            if self.reported.insert(u.at) {
                let local = written_name(&u.local).to_string();
                self.out.push(FlowDiagnostic { span: u.at, message: u.message.clone(), local, moved_at: u.at, part: None, poison: None, hole: None, readonly: true });
            }
        }
        refused
    }

    /// [deduce-field] Deciding: the virtual-place candidates of a mutation
    /// event at `span` (after it), kept unless their binding took ownership
    /// on this path, or — a fields-declaring call's survivor — their local
    /// is gone.
    fn virtual_here(&mut self, span: Span, moved: &Moved) {
        let Some(fate) = self.fate else { return };
        let Some(cands) = fate.virtual_at.get(&span) else { return };
        for c in cands {
            if moved.decided_here.contains(&c.stamp) {
                continue;
            }
            if let Some(l) = &c.local {
                if self.moved_use(l, moved).is_some() {
                    continue;
                }
            }
            self.virtual_binds.insert(c.stamp, c.place.clone());
        }
    }

    fn uses_at(&self, span: Span, assign: bool) -> Vec<DerivedUseAt> {
        let Some(fate) = self.fate else { return Vec::new() };
        let table = if assign { &fate.assign_uses } else { &fate.uses };
        table.get(&span).cloned().unwrap_or_default()
    }

    /// [fate-link] The links the checker gave a binding statement.
    fn bind_links(&self, id: NodeId) -> Option<&'f Vec<FlowLink>> {
        self.fate?.binds.get(&id.0)
    }

    /// [fate-partial-move] Assigning a place puts back every moved place
    /// it covers (as the checker's former `revive_moved_places` did):
    /// itself and anything under it; the whole local covers everything.
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

    /// `local` is moved at `span` by a consuming read, unless the value
    /// moves for free [copy-scalar-free] — judged by the read's own type,
    /// so a union narrowed to a Copy scalar is copied (and by the local's
    /// declared type, which a narrowing shares).
    fn mark_moved(&mut self, local: &str, read_ty: &Ty, span: Span, moved: &mut Moved) {
        let declared = self.decl_ty.get(self.storage(local));
        if crate::types::moves_for_free(read_ty) || declared.is_some_and(crate::types::moves_for_free) {
            return;
        }
        self.mark_moved_whole(local, span, moved);
    }

    /// `local` is moved at `span`, whatever its type: a consumed capture, or
    /// a move the checker made (`moved` on a read or a call).
    fn mark_moved_whole(&mut self, local: &str, span: Span, moved: &mut Moved) {
        // [fate-poison] Moving a root poisons what was derived from it.
        let root = self.storage(local).to_string();
        self.poison(&root, Some(&[]), "moved", span, None, moved);
        for l in self.group(local) {
            moved.whole.entry(l).or_insert(Gone::moved(span));
        }
    }

    /// [fate-move-mode] A move-mode binding, `binding`, takes ownership
    /// through `links` (`FateFacts::bound`): what was
    /// derived from each root at the link's projection is poisoned, and a
    /// root taken whole is gone, named by the binding (which replaces
    /// whatever it held); one taken at a projection keeps the rest, the
    /// part being the consuming read's [fate-partial-move].
    fn bind_away(&mut self, bound: Option<&(String, Vec<FlowLink>)>, span: Span, moved: &mut Moved) {
        let Some((binding, links)) = bound else { return };
        for l in links {
            self.poison(&l.root, l.path.as_deref(), "moved", span, None, moved);
            if l.path.as_deref().is_some_and(|p| !p.is_empty()) {
                continue;
            }
            for g in self.group(&l.root) {
                moved.whole.insert(g, Gone { span, why: Why::BoundAway { binding: binding.clone() } });
            }
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

    /// Revives a wholly moved local (its partial moves are the caller's:
    /// `revive_parts`).
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
            Stmt::Loop { body, .. } => self.walk_loop(body, None, false, moved),
            Stmt::ForEach { id, local, ty, iterable, body } => {
                let flow = self.walk_expr(iterable, moved);
                self.decl_ty.insert(local.0.clone(), ty.clone());
                let links = self.bind_links(*id);
                self.walk_loop(body, Some((&local.0, links)), true, moved);
                flow
            }
            Stmt::Let { id, local, ty, value } => {
                let flow = self.walk_expr(value, moved);
                let bound = self.fate.and_then(|f| f.bound.get(&id.0));
                self.bind_away(bound, value.span, moved);
                self.decl_ty.insert(local.0.clone(), ty.clone());
                self.alias_of.remove(&local.0);
                self.part_of.remove(&local.0);
                moved.forget(&local.0);
                // [fate-link] A borrow-mode binding shares fate with its roots.
                moved.set_links(&local.0, self.bind_links(*id));
                flow
            }
            Stmt::Alias { id, local, ty, place } => {
                self.walk_steps(place, moved);
                self.bind_view(&local.0, ty, place, false, false, moved);
                // [fate-link] A binding the checker keeps as its place is a
                // variable of its own there, linked to the place's root.
                if !place.steps.is_empty() {
                    moved.set_links(&local.0, self.bind_links(*id));
                }
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
                // [flow-use-after-move] Writing through a projection needs
                // the root there (the checker read the target's base before
                // R8 piece 2, so this was its consumed read).
                if !place.steps.is_empty() {
                    if let Some(&target) = self.fate.and_then(|f| f.assign_targets.get(&value.span)) {
                        self.report_write_into(&place.root.0, &place.steps, target, moved);
                    }
                }
                // [fate-poison] An assignment through a projection mutates
                // its roots, before the value is read (the checker checks
                // the target first) — unless it is a derived local's
                // mutation refused as an S1 error [fate-derived-readonly].
                let uses = self.uses_at(value.span, true);
                let refused = if uses.is_empty() { Vec::new() } else { self.derived_uses(&uses, value.span, moved) };
                let mut events = self.mutations_at(value.span, true);
                events.retain(|ev| !refused.iter().any(|l| written_name(l) == ev.root));
                self.apply_mutations(&events, value.span, moved);
                let flow = self.walk_expr(value, moved);
                // [fate-partial-move] A projection is put back once its new
                // value is in: the value's read of the moved place is a use
                // (`p.tags = f(p.tags)`), and whatever the value moved out of
                // the place is replaced.
                if !place.steps.is_empty() {
                    self.revive_parts(&place.root.0, &place.steps, moved);
                }
                if place.steps.is_empty() {
                    // [linear-state] [linear-obligation] Assigning over a
                    // state field that still holds its obligations drops
                    // them (the ledger owns every other local's case).
                    self.check_state_drop(&place.root.0, value.span, moved);
                    // [fate-move-mode] A move-mode assignment takes its
                    // value's roots first.
                    let bound = self.fate.and_then(|f| f.assign_bound.get(&value.span));
                    self.bind_away(bound, value.span, moved);
                    // [fate-poison] The old value is gone: what was derived
                    // from it is poisoned, and the local takes the new
                    // value's links [fate-link].
                    let root = self.storage(&place.root.0).to_string();
                    self.poison(&root, Some(&[]), "reassigned", value.span, None, moved);
                    self.revive(&place.root.0, moved);
                    self.revive_parts(&place.root.0, &[], moved);
                    let links = self.fate.and_then(|f| f.assigns.get(&value.span));
                    moved.set_links(&root, links);
                }
                flow
            }
            Stmt::Expr(e) => self.walk_expr(e, moved),
            Stmt::Return(value, span) => {
                if let Some(e) = value {
                    self.walk_expr(e, moved);
                }
                // [linear-state] A `return` ends the activation (the value
                // is moved first: `return waiting` leaves the hole too).
                if self.lambdas == 0 {
                    self.check_state_whole(*span, "return", moved);
                }
                Flow::Leaves
            }
            Stmt::Break => {
                if let Some(frame) = self.loops.last_mut() {
                    frame.breaks.push(moved.clone());
                }
                Flow::Leaves
            }
            // A `continue` takes its state to the top of the next
            // iteration: the loop's back edge.
            Stmt::Continue => {
                if let Some(frame) = self.loops.last_mut() {
                    frame.continues.push(moved.clone());
                }
                Flow::Leaves
            }
            Stmt::Obligation(_) => Flow::FallsThrough,
        }
    }

    /// [linear-state] An exit from a handler member at `span`: every state
    /// field holding obligations must be whole on this path — not moved
    /// out of, wholly or in part (the checker's former
    /// `check_state_whole`). Reported once per exit and field.
    fn check_state_whole(&mut self, span: Span, what: &'static str, moved: &Moved) {
        let Some(fate) = self.fate else { return };
        for name in &fate.linear_state {
            if !moved.whole.contains_key(name) && !moved.parts.contains_key(name) {
                continue;
            }
            if !self.holes.insert((span, name.clone())) {
                continue;
            }
            let message = format!(
                "cannot {what}: the state field `{name}` was moved out of and nothing was put back, so this \
                 activation would leave the handler with a hole — assign it a value first (`{name} = …`), since \
                 the obligations an actor holds must survive between activations"
            );
            self.out.push(FlowDiagnostic { span, message, local: name.clone(), moved_at: span, part: None, poison: None, hole: Some(what), readonly: false });
        }
    }

    /// [linear-state] A written assignment of the state field `local` (by
    /// its value's span) while the field is not moved out on this path.
    fn check_state_drop(&mut self, local: &str, value: Span, moved: &Moved) {
        let Some(fate) = self.fate else { return };
        if !fate.linear_state.iter().any(|n| n == local) || moved.whole.contains_key(local) {
            return;
        }
        let Some(&span) = fate.assign_spans.get(&value) else { return };
        if !self.holes.insert((span, local.to_string())) {
            return;
        }
        let message = format!(
            "assigning to `{local}` drops the linear value it still owns; move it onward or `discard({local})` first"
        );
        self.out.push(FlowDiagnostic { span, message, local: local.to_string(), moved_at: span, part: None, poison: None, hole: Some("assign"), readonly: false });
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
                if let Some(gone) = moved.whole.get(self.storage(&root)).cloned() {
                    moved.whole.insert(local.to_string(), gone);
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

    /// A loop body, as dataflow: the state at the top of an iteration is
    /// the entry state joined with every back edge — the body's end when it
    /// falls through, and every `continue`'s — walked again until it stops
    /// changing. A body that always leaves has no back edge from its end.
    /// After the loop: the `break`s joined, and for a `for` (which also
    /// leaves at the top, when the iterable runs out) the top state first;
    /// an unconditional `Loop` with no `break` never falls through.
    /// `binding` is a `for` binding, fresh on every iteration.
    fn walk_loop(&mut self, body: &Block, binding: Option<(&str, Option<&Vec<FlowLink>>)>, exits_at_top: bool, moved: &mut Moved) -> Flow {
        let mut head = moved.clone();
        let mut breaks = Vec::new();
        for _ in 0..MAX_LOOP_WALKS {
            let mut path = head.clone();
            if let Some((b, links)) = binding {
                path.forget(b);
                path.set_links(b, links);
            }
            self.loops.push(LoopExits::default());
            let flow = self.walk_block(body, &mut path);
            let exits = self.loops.pop().unwrap_or_default();
            breaks = exits.breaks;
            let mut next = head.clone();
            if flow == Flow::FallsThrough {
                join_into(&mut next, path);
            }
            for c in exits.continues {
                join_into(&mut next, c);
            }
            if next == head {
                break;
            }
            head = next;
        }
        let mut out: Vec<Moved> = Vec::new();
        if exits_at_top {
            out.push(head.clone());
        }
        out.extend(breaks);
        if out.is_empty() {
            // Only a `return` (or a throw) leaves: nothing follows the loop.
            *moved = head;
            return Flow::Leaves;
        }
        self.join(out, moved);
        if let Some((b, _)) = binding {
            moved.forget(b);
        }
        Flow::FallsThrough
    }

    fn walk_expr(&mut self, e: &Expr, moved: &mut Moved) -> Flow {
        // [fate-move-mode] A move or mutation of a derived local here.
        let uses = self.uses_at(e.span, false);
        let refused = if uses.is_empty() { Vec::new() } else { self.derived_uses(&uses, e.span, moved) };
        let flow = self.walk_expr_kind(e, moved);
        // [fate-poison] A mutation the checker placed here (a call's `Mut`
        // argument, an iterated subject) lands after the expression — but
        // not one refused as an S1 error [fate-derived-readonly].
        let mut events = self.mutations_at(e.span, false);
        events.retain(|ev| !refused.iter().any(|l| written_name(l) == ev.root));
        self.apply_mutations(&events, e.span, moved);
        if self.deciding {
            self.virtual_here(e.span, moved);
        }
        flow
    }

    fn walk_expr_kind(&mut self, e: &Expr, moved: &mut Moved) -> Flow {
        match &e.kind {
            ExprKind::Read { place, consume, moved: checker_moved } => {
                self.walk_steps(place, moved);
                if !self.report_use(&place.root.0, e.span, moved) {
                    self.report_part_use(&place.root.0, &place.steps, e.span, moved);
                }
                if self.refused.contains(&e.span) {
                    // [fate-derived-readonly] Refused: the move is not made.
                } else if *checker_moved && !(place.steps.is_empty() && crate::types::moves_for_free(&e.ty)) {
                    // The checker consumed the root here: a fn value, or a
                    // projection whose move took its root wholly. (It also
                    // marks a narrowed Copy scalar, judging by the declared
                    // union; the value's own type says it is a copy
                    // [copy-scalar-free].)
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
                if *consumes && !self.refused.contains(&e.span) {
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
                for (cond, body) in arms {
                    self.walk_expr(cond, moved);
                    let mut path = moved.clone();
                    if self.walk_block(body, &mut path) == Flow::FallsThrough {
                        falls.push(path);
                    }
                }
                match otherwise {
                    Some(body) => {
                        let mut path = moved.clone();
                        if self.walk_block(body, &mut path) == Flow::FallsThrough {
                            falls.push(path);
                        }
                    }
                    None => {
                        if !arms.iter().any(|(c, _)| matches!(c.kind, ExprKind::Bool(true))) {
                            falls.push(moved.clone());
                        }
                    }
                }
                let flow = if falls.is_empty() { Flow::Leaves } else { Flow::FallsThrough };
                // An assignment in an arm revived its local on that path
                // only: the join says the rest.
                self.join(falls, moved);
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
                flow
            }
            ExprKind::Lambda { params, body, captures, .. } => {
                let mut inner = moved.clone();
                for p in params {
                    self.decl_ty.insert(p.local.0.clone(), p.ty.clone());
                    inner.forget(&p.local.0);
                }
                let loops = std::mem::take(&mut self.loops);
                self.lambdas += 1;
                self.walk_block(body, &mut inner);
                self.lambdas -= 1;
                self.loops = loops;
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
}

/// Moved on either path is moved; so is a place moved out on either
/// [fate-partial-move] (union, the earlier path's move first, as the
/// checker's `merge_fallthrough` joined its `moved_places`).
fn join_into(acc: &mut Moved, other: Moved) {
    for (k, v) in other.whole {
        match acc.whole.get_mut(&k) {
            // [fate-poison] The reason kept is the first path's poison, if
            // any path has one (the checker's `merge_fallthrough`).
            Some(have) => {
                if have.why == Why::Moved && v.why != Why::Moved {
                    *have = v;
                }
            }
            None => {
                acc.whole.insert(k, v);
            }
        }
    }
    // [fate-link] Links union: may-be-linked is linked.
    for (k, ls) in other.links {
        let have = acc.links.entry(k).or_default();
        for l in ls {
            if !have.iter().any(|e| same_link_root(e, &l)) {
                have.push(l);
            }
        }
    }
    for (root, list) in other.parts {
        for (path, span) in list {
            acc.add_part(&root, path, span);
        }
    }
    // [fate-move-mode] Taken ownership on any path is taken (the checker's
    // `take_ownership_on_paths` did it at the join until R8 piece 8).
    acc.decided_here.extend(other.decided_here);
}

/// [fate-move-mode] The root of a link to a binding no longer in scope (an
/// earlier binding of the name): no local is named so, so it poisons
/// nothing, but its stamp still names the binding that made it.
const GONE: &str = "~gone";

/// Two links to the same root, as the checker's links are by variable: a
/// link to a binding gone from scope is its own, by the binding that made it.
fn same_link_root(a: &FlowLink, b: &FlowLink) -> bool {
    a.root == b.root && (a.root != GONE || a.stamp == b.stamp)
}

fn dedupe_links(ls: &mut Vec<FlowLink>) {
    let mut out: Vec<FlowLink> = Vec::new();
    for l in ls.drain(..) {
        if !out.iter().any(|e| same_link_root(e, &l)) {
            out.push(l);
        }
    }
    *ls = out;
}

/// A local's name as written: the builder suffixes a rebinding (`x~2`).
fn written_name(local: &str) -> &str {
    match local.find('~') {
        Some(i) => &local[..i],
        None => local,
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
