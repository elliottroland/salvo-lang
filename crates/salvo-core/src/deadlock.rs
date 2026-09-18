//! [actor-deadlock-cycle] The static deadlock baseline: a cycle check over
//! the **effect graph** of actors that wait for one another.
//!
//! The design's committed baseline, decided with the phase and recorded in
//! COMPLETED.md's log. Two actors that each park a *gated*
//! continuation on the other's answer wait forever, and the failure is
//! interleaving-dependent — it passes every test and deadlocks in production
//! on the rare crossing — so what is caught here is the **possibility**,
//! whole-program, rather than the occurrence at run time.
//!
//! Nodes are `actor effect`s, because that is what an `Addr` is typed by, and
//! there are two kinds of edge:
//!
//! * a **wait-for** edge, from a gate. While a `replyto!` continuation is
//!   outstanding the actor serves nothing but its answer, so a cycle of
//!   gates is a deadlock that no amount of load changes. Reported as an
//!   **error**.
//! * a **block** edge [waitfor-effect], from a signature: a handler that
//!   declares `[waitfor]` may occupy its thread until an answer arrives, and
//!   its mailbox stays stalled while it does — so it waits exactly as a gate
//!   does, and the cycle it can close is the one a dedicated thread does *not*
//!   remove: a wait whose fulfilment routes back through the waiter's own
//!   mailbox. Reported as an **error** too, and the third edge kind arrives
//!   *from declarations* rather than from sites, which is the modular form the
//!   retired survey predicted for awaits.
//! * a **back-pressure** edge, from an ordinary send. A send into a full
//!   bounded mailbox blocks its activation, so a cycle of sends deadlocks
//!   *only* when the mailboxes involved are simultaneously full. Reported as
//!   a **warning** (user decision 2026-09-16): the program is legal and
//!   usually fine, and refusing every pair of actors that send to each
//!   other would refuse most useful topologies.
//!
//! The imprecision is stated rather than discovered, and it is the same one
//! the design predicted: the graph is over actor **types**, not instances,
//! so a chain of same-protocol workers each sending to the next is a
//! self-loop here and acyclic in the running program. What that costs is a
//! warning on a legal program; stratification (a tier qualifier on an addr)
//! and the fallbacks are the recorded answers if the false positives become a
//! nuisance, deliberately not built until they are observed.
//!
//! **Task bodies are traced whole-program** [task-mint] (user decision
//! 2026-09-17, FC-6): a free `send fn` belongs to no actor, so its sends are
//! attributed to every actor whose mints reach it — following task-to-task
//! mints transitively. The alternative, tasks contributing nothing,
//! under-reports a real cycle class: an actor that hands its work to a task
//! which sends back is the same cycle written in two pieces. A *mint* itself
//! contributes no edge, and needs none: a task has no mailbox, so nothing can
//! fill and nothing can gate.
//!
//! The honest gap, recorded rather than papered over: an obligation parked in
//! a task is a wait-for edge pointing at no effect node, because the token's
//! holder is a closure rather than an actor. What is under it is linearity —
//! the token cannot be dropped [linear-obligation] — and the runtime's idle
//! report, which names the gated waiter when a task's chain dies.
//!
//! **Interception is exempt**, which the design documents did not anticipate:
//! a handler of `E` that declares `[E]` — a policy wrapper around the actor
//! already serving `E`, the phase's showcase pattern — is an `E → E` edge by
//! construction. It can never deadlock, because the dependency binds strictly
//! *outward*: the wrapped instance is a different actor, and the chain ends
//! at the innermost one. So an edge a handler gets from its **own-effect
//! dependency** is dropped, while an addr of its own protocol (a genuine peer
//! mesh) still counts.

use std::collections::{BTreeMap, BTreeSet};

use salvo_syntax::ast::{self, EffectRef, Item};
use salvo_syntax::span::Span;

use crate::check::Checked;
use crate::diag::FileDiagnostic;
use crate::program::{Program, Symbols};

/// Where an edge came from, for the diagnostic and for the severity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum EdgeKind {
    /// A gated mint: the actor waits, whatever the load.
    Wait,
    /// [waitfor-effect] A declared `[waitfor]`: the actor may occupy its
    /// thread until an answer arrives, and serves no message meanwhile.
    Block,
    /// A send that can block on a full mailbox.
    BackPressure,
}

impl EdgeKind {
    /// Whether the edge waits unconditionally — the error class. Both a gate
    /// and a declared block do; back-pressure needs full mailboxes.
    fn unconditional(self) -> bool {
        matches!(self, EdgeKind::Wait | EdgeKind::Block)
    }
}

/// One edge of the graph: who waits, on what, and where it is written.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Edge {
    kind: EdgeKind,
    /// The handler whose member the site is in, for the diagnostic.
    handler: String,
    file: usize,
    span: Span,
}

/// [actor-deadlock-cycle] Reports every cycle in the wait-for graph: an error
/// per cycle of gates, a warning per cycle that needs a back-pressure edge to
/// close.
pub(crate) fn check(program: &Program, symbols: &Symbols<'_>, out: &mut Checked) {
    let graph = build(program, symbols, out);
    if graph.is_empty() {
        return;
    }
    // Errors first, over the wait-for subgraph alone, so a genuine gate cycle
    // is never reported as the weaker thing.
    let waits: BTreeMap<String, BTreeMap<String, Edge>> = graph
        .iter()
        .map(|(from, tos)| {
            (
                from.clone(),
                tos.iter()
                    .filter(|(_, e)| e.kind.unconditional())
                    .map(|(to, e)| (to.clone(), e.clone()))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .filter(|(_, tos)| !tos.is_empty())
        .collect();
    let mut reported: BTreeSet<Vec<String>> = BTreeSet::new();
    for cycle in cycles(&waits) {
        if !reported.insert(cycle.clone()) {
            continue;
        }
        report(&waits, &cycle, out, true);
    }
    // Then the whole graph: a cycle that only closes through a send is the
    // load-conditioned class, and a warning.
    let seen_nodes: BTreeSet<String> = reported.iter().flatten().cloned().collect();
    for cycle in cycles(&graph) {
        if cycle.iter().all(|n| seen_nodes.contains(n)) {
            continue;
        }
        report(&graph, &cycle, out, false);
    }
}

/// The graph, keyed effect → effect. At most one edge per pair is kept — the
/// strongest one, at its first site — because the diagnostic names a cycle,
/// not every way of writing it.
fn build(
    program: &Program,
    symbols: &Symbols<'_>,
    out: &Checked,
) -> BTreeMap<String, BTreeMap<String, Edge>> {
    let mut graph: BTreeMap<String, BTreeMap<String, Edge>> = BTreeMap::new();
    let is_actor = |name: &str| {
        symbols
            .effects
            .get(name)
            .is_some_and(|decl| decl.is_actor)
    };
    // Every handler of an actor effect: the protocol it serves, whether any
    // of its members gates, and what it can send to.
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            let Item::Handler(h) = item else { continue };
            let Some(served) = base_name(&h.of) else {
                continue;
            };
            if !is_actor(served) {
                continue;
            }
            let gate = out
                .actor_gates
                .iter()
                .find(|(handler, _)| *handler == h.name.name)
                .map(|(_, key)| *key);
            // [waitfor-effect] The signature half: a handler that declares
            // `[waitfor]` waits like a gate, and the declaration is the site
            // to blame — that is what "from signatures" means.
            let blocks = h.effects.iter().flatten().find_map(|e| match e {
                EffectRef::WaitFor(span) => Some((file_idx, *span)),
                _ => None,
            });
            // Declared dependencies: any member may call any of them, and a
            // helper fn declaring `[Q]` is reachable only if the handler
            // declares `Q` too — so the declaration covers the whole call
            // graph without walking it.
            let mut targets: Vec<(String, usize, Span)> = Vec::new();
            for dep in h.effects.iter().flatten() {
                let EffectRef::Effect(r) = dep else { continue };
                let name = r.name.name.as_str();
                // Interception: an own-effect dependency binds outward, so it
                // closes no cycle.
                if !is_actor(name) || name == served {
                    continue;
                }
                targets.push((name.to_string(), file_idx, r.span));
            }
            // Sends through an addr the handler holds: the peer half of the
            // graph, and the only place an own-protocol edge survives.
            for (handler, target, (file, span)) in &out.actor_sends {
                if *handler == h.name.name && is_actor(target) {
                    targets.push((target.clone(), *file, *span));
                }
            }
            // [task-mint] And the sends of every task this handler's mints
            // reach, transitively — FC-6's conservative tracing. The site
            // blamed is the send inside the task body, which is the line that
            // closes the cycle.
            for task in tasks_reached(out, &h.name.name) {
                for (owner, target, (file, span)) in &out.task_sends {
                    if *owner == task && is_actor(target) {
                        targets.push((target.clone(), *file, *span));
                    }
                }
            }
            for (target, file, span) in targets {
                let (kind, file, span) = match (gate, blocks) {
                    // A gate anywhere in the handler makes its sends waits:
                    // the actor is serving nothing else meanwhile.
                    (Some((gfile, gspan)), _) => (EdgeKind::Wait, gfile, gspan),
                    // [waitfor-effect] So does a declared block, for the same
                    // reason and with the declaration as its site.
                    (None, Some((bfile, bspan))) => (EdgeKind::Block, bfile, bspan),
                    (None, None) => (EdgeKind::BackPressure, file, span),
                };
                let edge = Edge {
                    kind,
                    handler: h.name.name.clone(),
                    file,
                    span,
                };
                let slot = graph.entry(served.to_string()).or_default();
                match slot.get(&target) {
                    // Keep the stronger edge; ties keep the first site.
                    Some(existing) if existing.kind <= edge.kind => {}
                    _ => {
                        slot.insert(target, edge);
                    }
                }
            }
        }
    }
    graph
}

/// [task-mint] The free `send fn`s a minter's work reaches: what it mints
/// directly, plus what those tasks mint in turn. A fixpoint over
/// `task_mints`, so a chain of continuations is one traced unit — and
/// terminating, because the set only grows and is bounded by the program's
/// send fns.
fn tasks_reached(out: &Checked, minter: &str) -> BTreeSet<String> {
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = out
        .task_mints
        .iter()
        .filter(|(who, _)| who == minter)
        .map(|(_, task)| task.clone())
        .collect();
    while let Some(task) = queue.pop() {
        if !reached.insert(task.clone()) {
            continue;
        }
        for (who, next) in &out.task_mints {
            if *who == task {
                queue.push(next.clone());
            }
        }
    }
    reached
}

/// Every elementary cycle worth reporting: one per strongly connected
/// component, as the shortest cycle through its lowest-named node, so the
/// output is deterministic and a component does not produce a diagnostic per
/// permutation.
fn cycles(graph: &BTreeMap<String, BTreeMap<String, Edge>>) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut done: BTreeSet<String> = BTreeSet::new();
    for start in graph.keys() {
        if done.contains(start) {
            continue;
        }
        // Breadth-first from `start` back to `start`: the shortest cycle it
        // sits on, or none.
        let mut queue: std::collections::VecDeque<Vec<String>> =
            std::collections::VecDeque::new();
        queue.push_back(vec![start.clone()]);
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut found: Option<Vec<String>> = None;
        while let Some(path) = queue.pop_front() {
            let last = path.last().expect("a path has a last node");
            for next in graph.get(last).into_iter().flatten().map(|(to, _)| to) {
                if next == start {
                    let mut cycle = path.clone();
                    cycle.push(start.clone());
                    found = Some(cycle);
                    break;
                }
                if visited.insert(next.clone()) {
                    let mut longer = path.clone();
                    longer.push(next.clone());
                    queue.push_back(longer);
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some(cycle) = found {
            // Everything on the cycle is answered by this one report.
            for node in &cycle {
                done.insert(node.clone());
            }
            out.push(cycle);
        }
    }
    out
}

/// The diagnostic: the cycle as a path, the handlers on it, and what to do.
/// Reported at the site of the *first* edge, which is the seam that closes it
/// — a `replyto!` for a wait cycle, a send for a back-pressure one.
fn report(
    graph: &BTreeMap<String, BTreeMap<String, Edge>>,
    cycle: &[String],
    out: &mut Checked,
    unconditional: bool,
) {
    let mut edges: Vec<&Edge> = Vec::new();
    for pair in cycle.windows(2) {
        if let Some(edge) = graph.get(&pair[0]).and_then(|tos| tos.get(&pair[1])) {
            edges.push(edge);
        }
    }
    let Some(first) = edges.first().copied() else {
        return;
    };
    let path = cycle.join(" → ");
    let handlers: BTreeSet<&str> = edges.iter().map(|e| e.handler.as_str()).collect();
    let handlers: Vec<String> = handlers.iter().map(|h| format!("`{h}`")).collect();
    let handlers = handlers.join(", ");
    // [waitfor-effect] Which unconditional cycle this is decides what the
    // remedy is: a gate is respelled, a declared block is a design to move
    // off the waiting path.
    let blocking = edges.iter().any(|e| e.kind == EdgeKind::Block);
    match (unconditional, blocking) {
        (true, true) => out.errors.push(FileDiagnostic::error(
            first.file,
            first.span,
            format!(
                "these actors wait for each other: {path} ({handlers}). A handler that \
                 declares `[waitfor]` serves no message while it waits, so each of them \
                 is waiting for an answer the next can only produce after its own \
                 arrives — and a thread of its own does not help, since the fulfilment \
                 routes back through a stalled mailbox. Break the cycle: park a \
                 continuation with `replyto` instead of waiting, or have the answer \
                 come from a third actor neither of them waits on"
            ),
        )),
        (true, false) => out.errors.push(FileDiagnostic::error(
            first.file,
            first.span,
            format!(
                "these actors wait for each other: {path} ({handlers}). A `replyto!` \
                 gate serves nothing but its own answer, so each of them is waiting for \
                 a reply the next can only produce after its own arrives — a deadlock \
                 whenever the requests cross. Break the cycle: make one side's \
                 continuation ungated (`replyto`, whose mailbox stays open), or have \
                 the answer come from a third actor neither of them waits on"
            ),
        )),
        (false, _) => out.errors.push(FileDiagnostic::warning(
            first.file,
            first.span,
            format!(
                "these actors send to each other in a cycle: {path} ({handlers}). A \
                 send blocks while the target's mailbox is full, so this deadlocks if \
                 the mailboxes fill at the same time — a failure that appears under \
                 load and not in a test. Give the queues room (a larger `capacity`), or \
                 break the cycle by routing one direction through a reply token, which \
                 has capacity reserved and never blocks"
            ),
        )),
    }
}

/// The base name of a written type, for a handler's `of` clause.
fn base_name(ty: &ast::Type) -> Option<&str> {
    match ty {
        ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
        _ => None,
    }
}
