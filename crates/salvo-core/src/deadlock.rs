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

use salvo_syntax::ast::{self, Item};
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
    /// [mixed-handler] [actor-deadlock-cycle] An **occupancy** edge (SH-4,
    /// user decision 2026-09-19): a caller of a mixed handler's sync member
    /// parks until the servant answers, and a parked *activation* stalls its
    /// own mailbox while it waits. Inferred from declarations — an actor
    /// handler that declares a plain-effect dependency with mixed handlers
    /// in the program may reach a façade call — never written.
    Occupancy,
    /// A send that can block on a full mailbox.
    BackPressure,
}

impl EdgeKind {
    /// Whether the edge waits unconditionally — the error class. A gate, a
    /// declared block and an inferred occupancy all do; back-pressure needs
    /// full mailboxes. And occupancy admits **no downgrade**: the ungated-
    /// side argument (the other actor keeps serving) is exactly what an
    /// occupied activation's `running` flag removes, so a cycle that mixes
    /// occupancy with ordinary sends is still an error.
    fn unconditional(self) -> bool {
        matches!(self, EdgeKind::Wait | EdgeKind::Block | EdgeKind::Occupancy)
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
    // Then the whole graph. A cycle containing an **occupancy** edge is an
    // error even when back-pressure closes it — the ungated-side downgrade
    // does not apply, because an occupied activation serves *nothing* of its
    // mailbox [mixed-handler]. A cycle that only closes through sends is the
    // load-conditioned class, and a warning.
    let seen_nodes: BTreeSet<String> = reported.iter().flatten().cloned().collect();
    for cycle in cycles(&graph) {
        let occupying = cycle.windows(2).any(|pair| {
            graph
                .get(&pair[0])
                .and_then(|tos| tos.get(&pair[1]))
                .is_some_and(|e| e.kind == EdgeKind::Occupancy)
        });
        if occupying {
            if reported.insert(cycle.clone()) {
                report(&graph, &cycle, out, true);
            }
            continue;
        }
        if cycle.iter().all(|n| seen_nodes.contains(n)) {
            continue;
        }
        report(&graph, &cycle, out, false);
    }
}

/// [actor-deadlock-cycle] For tests: the graph's edges as `from → to (Kind,
/// handler)` lines, sorted, so a test can pin a node that closes no cycle
/// (a route's, [route-stub]), which no diagnostic would show.
pub fn edges(program: &Program, symbols: &Symbols<'_>, out: &Checked) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for (from, tos) in build(program, symbols, out) {
        for (to, e) in tos {
            lines.push(format!("{from} → {to} ({:?}, {})", e.kind, e.handler));
        }
    }
    lines
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
    let is_plain = |name: &str| {
        symbols
            .effects
            .get(name)
            .is_some_and(|decl| !decl.is_actor)
    };
    // [mixed-handler] The mixed handlers of each plain effect: what an
    // occupancy edge can point at. Over types, not instances — a program
    // that binds `SystemRandom` everywhere still gets the edge if a mixed
    // `CyclicRandom` exists, the same trade the whole graph makes.
    //
    // [effect-handle] [monitor-handler] And, since shareable-by-default (user
    // decision 2026-09-20, option (b): waits under a lock are priced, not
    // refused), the **stateful plain handlers worth a node of their own**:
    // a plain-face, no-send-member, *stateful* handler
    // (`crate::handler_is_stateful` — the same predicate the emitters key
    // the handle's lock on) that declares dependencies or waits. Its
    // callers run its members on their own threads under its lock, so what
    // its members can wait on or send to becomes an edge held while the
    // lock is, and a handler depending on its effect may park in it (an
    // occupancy edge in). A stateless handler's members run on the caller's
    // thread with nothing held, so the caller's own edges already price
    // them; a pure monitor (no deps, no waits) adds nothing. Neither gets a
    // node.
    let mut mixed_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut monitor_like: Vec<(usize, &ast::HandlerDecl)> = Vec::new();
    let mut monitor_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            let Item::Handler(h) = item else { continue };
            let plain_faces = !h.of.is_empty()
                && h.of
                    .iter()
                    .all(|of| base_name(of).is_some_and(is_plain));
            if !plain_faces {
                continue;
            }
            if !h.fns.iter().any(|f| f.is_send) {
                let stateful = crate::handler_is_stateful(
                    h,
                    out.parking_handlers.contains(h.name.name.as_str()),
                );
                if !stateful {
                    continue;
                }
                let has_deps = !deps_of(out, symbols, h).is_empty();
                let waits = out
                    .waitfor_sites
                    .iter()
                    .any(|(handler, _)| *handler == h.name.name);
                if has_deps || waits {
                    monitor_like.push((file_idx, h));
                    for of in &h.of {
                        if let Some(n) = base_name(of) {
                            monitor_of
                                .entry(n.to_string())
                                .or_default()
                                .push(h.name.name.clone());
                        }
                    }
                }
                continue;
            }
            for of in &h.of {
                if let Some(n) = base_name(of) {
                    mixed_of
                        .entry(n.to_string())
                        .or_default()
                        .push(h.name.name.clone());
                }
            }
        }
    }
    // [actor-waitfor] Which handlers *wait*: directly (a member contains a
    // `waitfor` expression) or transitively (a member binds — constructs — a
    // handler that waits, running its members inline). A fixpoint over the
    // recorded constructions, anchored at whatever brought the wait in.
    let mut waits_of: BTreeMap<&str, (usize, Span)> = BTreeMap::new();
    for (handler, (file, span)) in &out.waitfor_sites {
        waits_of.entry(handler.as_str()).or_insert((*file, *span));
    }
    loop {
        let mut grew = false;
        for (owner, constructed, (file, span)) in &out.handler_constructs {
            if waits_of.contains_key(constructed.as_str())
                && !waits_of.contains_key(owner.as_str())
            {
                waits_of.insert(owner.as_str(), (*file, *span));
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    // Every handler of an actor effect: the protocol it serves, whether any
    // of its members gates, and what it can send to.
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            let Item::Handler(h) = item else { continue };
            // [effect-handler-multi] A handler may serve several protocols, and
            // each is its own node: two nodes that happen to be one actor is
            // the same conservative approximation the type-level graph already
            // makes, so every edge below is added from every face.
            let served: Vec<&str> = h
                .of
                .iter()
                .filter_map(|of| base_name(of))
                .filter(|name| is_actor(name))
                .collect();
            // [mixed-handler] A mixed handler's servant is a node of its
            // own: it serves no actor effect, but its send members can send
            // through the `Addr` values it holds, and those are the edges
            // that close an occupancy cycle. [defer-deduction] Since parking
            // landed, a servant can gate too (`replyto!` in a send member)
            // and can wait (`waitfor` in one), so its edge kinds mirror an
            // actor's: a gate makes its sends waits, a wait makes them
            // blocks, and otherwise they are back-pressure. Task sends count
            // against it the same way they count against an actor
            // [task-mint]. Its dependencies are the servant's (the façade
            // never sees them), so they leave from this node too: an actor
            // dependency is a target like a held addr, and a plain one with
            // mixed or wait-capable shareable handlers is an occupancy edge,
            // exactly as from an actor handler. Its own face is skipped:
            // interception binds outward, so it closes no cycle.
            let mixed = h.fns.iter().any(|f| f.is_send)
                && !h.of.is_empty()
                && h.of
                    .iter()
                    .all(|of| base_name(of).is_some_and(is_plain));
            // [effect-any] A router `of any E` is a servant too: it is not a
            // member of `E` but a forwarder in front of them, so its sends
            // to `E` leave from a node of its own — otherwise every router
            // would read as `E → E`, a cycle the type-level graph cannot
            // tell from a real one. Callers' edges still land on `E`, which
            // is what a binding of `E` may be.
            let router = h.of_any.iter().any(|any| *any);
            if mixed || router {
                let gate = out
                    .actor_gates
                    .iter()
                    .find(|(handler, _)| *handler == h.name.name)
                    .map(|(_, key)| *key);
                let blocks = waits_of.get(h.name.name.as_str()).copied();
                let own_faces: Vec<&str> = h.of.iter().filter_map(base_name).collect();
                let deps: Vec<(String, usize, Span)> = deps_of(out, symbols, h)
                    .into_iter()
                    .map(|(key, span)| (key, file_idx, span))
                    .collect();
                let mut sends: Vec<(String, usize, Span)> = Vec::new();
                for (handler, target, (file, span)) in &out.actor_sends {
                    if *handler == h.name.name {
                        sends.push((target.clone(), *file, *span));
                    }
                }
                for task in tasks_reached(out, &h.name.name) {
                    for (owner, target, (file, span)) in &out.task_sends {
                        if *owner == task {
                            sends.push((target.clone(), *file, *span));
                        }
                    }
                }
                let servant = Servant {
                    handler: &h.name.name,
                    own_faces: &own_faces,
                    deps,
                    sends,
                    gate,
                    blocks,
                };
                servant.add_to(&mut graph, &is_actor, &mixed_of, &monitor_of);
                continue;
            }
            if served.is_empty() {
                continue;
            }
            let gate = out
                .actor_gates
                .iter()
                .find(|(handler, _)| *handler == h.name.name)
                .map(|(_, key)| *key);
            // [actor-waitfor] The occupancy half, **inferred** since SH-5(d)
            // deleted the `[waitfor]` declaration (user decision 2026-09-19):
            // a handler waits when a member of its own contains a `waitfor`
            // expression, or when it binds a handler that waits — the
            // propagation the deleted capability used to spell, computed
            // here over the recorded sites and constructions instead.
            let blocks = waits_of.get(h.name.name.as_str()).copied();
            // Declared dependencies: any member may call any of them, and a
            // helper fn declaring `[Q]` is reachable only if the handler
            // declares `Q` too — so the declaration covers the whole call
            // graph without walking it.
            let mut targets: Vec<(String, usize, Span)> = Vec::new();
            // [mixed-handler] Occupancy edges, inferred from the same
            // declarations: a dependency on a plain effect that has mixed
            // handlers means a member may call a façade — and park until the
            // servant answers, stalling its own mailbox meanwhile. One edge
            // per mixed handler of the effect, since the graph cannot know
            // which one a binding will choose [actor-deadlock-cycle].
            let mut occupancies: Vec<(String, usize, Span)> = Vec::new();
            for (key, dep_span) in deps_of(out, symbols, h) {
                let name = crate::typekey::plain(&key);
                if !is_actor(&key) {
                    if let Some(mixed_handlers) = mixed_of.get(name) {
                        for mh in mixed_handlers {
                            occupancies.push((servant_node(mh), file_idx, dep_span));
                        }
                    }
                    // [effect-handle] [monitor-handler] A dep on a plain effect
                    // with wait-capable shareable handlers may park inside
                    // one — through its lock when it is stateful — so the
                    // edge points at the handler's own node (option (b),
                    // user decision 2026-09-20).
                    if let Some(monitors) = monitor_of.get(name) {
                        for mh in monitors {
                            occupancies.push((monitor_node(mh), file_idx, dep_span));
                        }
                    }
                    continue;
                }
                // Interception: an own-effect dependency binds outward, so it
                // closes no cycle. "Own" is any face this handler wears.
                if served.contains(&name) {
                    continue;
                }
                targets.push((name.to_string(), file_idx, dep_span));
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
            for (target, file, span) in occupancies {
                let edge = Edge {
                    kind: EdgeKind::Occupancy,
                    handler: h.name.name.clone(),
                    file,
                    span,
                };
                for from in &served {
                    let slot = graph.entry((*from).to_string()).or_default();
                    match slot.get(&target) {
                        Some(existing) if existing.kind <= edge.kind => {}
                        _ => {
                            slot.insert(target.clone(), edge.clone());
                        }
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
                for from in &served {
                    let slot = graph.entry((*from).to_string()).or_default();
                    match slot.get(&target) {
                        // Keep the stronger edge; ties keep the first site.
                        Some(existing) if existing.kind <= edge.kind => {}
                        _ => {
                            slot.insert(target.clone(), edge.clone());
                        }
                    }
                }
            }
        }
    }
    // [route-stub] [effect-any] Every protocol a `use any` routes has a
    // route the IR derives (`ir::build::actors`, `__Route_X`), which is no
    // AST handler — so its node is built here, as the one an `of any X`
    // router gets above: a servant of its own, forwarding to `X`
    // (back-pressure, since a route neither gates nor waits) and depending
    // on the `RouteSelector<X>` the `use any` found, whose mixed servants
    // and priced shareable handlers are occupancy edges. Its sites are the
    // protocol's first `use any`, in source order. Nothing points at the
    // node — callers' sends land on `X`, which is what a binding of `any X`
    // may be — so it closes no cycle; what it keeps is the forward off `X`,
    // where it would read as `X → X`.
    let mut routed: BTreeMap<&str, (usize, Span)> = BTreeMap::new();
    for ((file, span), effect) in &out.route_uses {
        let slot = routed.entry(effect.as_str()).or_insert((*file, *span));
        if (*file, span.start) < (slot.0, slot.1.start) {
            *slot = (*file, *span);
        }
    }
    for (effect, (file, span)) in routed {
        let handler = crate::ir::build::actors::route_name(effect);
        let own_faces = [crate::typekey::plain(effect)];
        let deps: Vec<(String, usize, Span)> = out
            .use_deps
            .get(&(file, span))
            .into_iter()
            .flatten()
            .filter_map(|t| match t.strip_quals() {
                crate::types::Ty::Named { name, .. } => Some((name.clone(), file, span)),
                _ => None,
            })
            .collect();
        let servant = Servant {
            handler: &handler,
            own_faces: &own_faces,
            deps,
            sends: vec![(effect.to_string(), file, span)],
            gate: None,
            blocks: None,
        };
        servant.add_to(&mut graph, &is_actor, &mixed_of, &monitor_of);
    }
    // [effect-handle] [monitor-handler] The shareable plain handlers worth a
    // node (option (b), user decision 2026-09-20): occupancy edges out to
    // whatever their members can park in — the mixed servants and priced
    // monitors of their dependency effects — plus actor-dep sends and task
    // sends, Block-kind when the handler itself waits. The in-edges are the
    // dependents' business (every loop above already points occupancy edges
    // at `monitor_of` nodes). Interception is skipped like an actor's
    // own-face dependency: it binds strictly outward, so it closes no cycle.
    for (file_idx, h) in monitor_like {
        let from = monitor_node(&h.name.name);
        let own_faces: Vec<&str> = h.of.iter().filter_map(base_name).collect();
        let blocks = waits_of.get(h.name.name.as_str()).copied();
        let mut occupancies: Vec<(String, usize, Span)> = Vec::new();
        let mut targets: Vec<(String, usize, Span)> = Vec::new();
        for (key, dep_span) in deps_of(out, symbols, h) {
            let name = crate::typekey::plain(&key);
            if own_faces.contains(&name) {
                continue;
            }
            if !is_actor(&key) {
                if let Some(mixed_handlers) = mixed_of.get(name) {
                    for mh in mixed_handlers {
                        occupancies.push((servant_node(mh), file_idx, dep_span));
                    }
                }
                if let Some(monitors) = monitor_of.get(name) {
                    for mh in monitors {
                        if *mh != h.name.name {
                            occupancies.push((monitor_node(mh), file_idx, dep_span));
                        }
                    }
                }
                continue;
            }
            targets.push((name.to_string(), file_idx, dep_span));
        }
        for task in tasks_reached(out, &h.name.name) {
            for (owner, target, (file, span)) in &out.task_sends {
                if *owner == task && is_actor(target) {
                    targets.push((target.clone(), *file, *span));
                }
            }
        }
        for (target, file, span) in occupancies {
            let edge = Edge {
                kind: EdgeKind::Occupancy,
                handler: h.name.name.clone(),
                file,
                span,
            };
            let slot = graph.entry(from.clone()).or_default();
            match slot.get(&target) {
                Some(existing) if existing.kind <= edge.kind => {}
                _ => {
                    slot.insert(target, edge);
                }
            }
        }
        for (target, file, span) in targets {
            let (kind, file, span) = match blocks {
                Some((bfile, bspan)) => (EdgeKind::Block, bfile, bspan),
                None => (EdgeKind::BackPressure, file, span),
            };
            let edge = Edge {
                kind,
                handler: h.name.name.clone(),
                file,
                span,
            };
            let slot = graph.entry(from.clone()).or_default();
            match slot.get(&target) {
                Some(existing) if existing.kind <= edge.kind => {}
                _ => {
                    slot.insert(target, edge);
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
    // [mixed-handler] An occupancy cycle gets its own report, anchored at the
    // occupancy edge — the seam where a caller parks on a servant — with the
    // three ways out §3.3 of the design named.
    if let Some(occ) = edges.iter().find(|e| e.kind == EdgeKind::Occupancy) {
        out.errors.push(FileDiagnostic::error(
            occ.file,
            occ.span,
            format!(
                "these actors can wait for each other through a shared handler: {path} \
                 ({handlers}). A sync member of a mixed handler parks its caller until \
                 the servant answers, and a parked activation serves nothing of its own \
                 mailbox — so each step of this cycle waits on the next, and the \
                 fulfilment routes back through a stalled queue. Break the cycle: have \
                 the servant answer without reaching the peer, respell the consulting \
                 call as a send plus a continuation, or bind a handler of the effect \
                 that does not wait — a monitor, or a scope-local `use`"
            ),
        ));
        return;
    }
    // [waitfor-effect] Which unconditional cycle this is decides what the
    // remedy is: a gate is respelled, a declared block is a design to move
    // off the waiting path.
    let blocking = edges.iter().any(|e| e.kind == EdgeKind::Block);
    match (unconditional, blocking) {
        (true, true) => out.errors.push(FileDiagnostic::error(
            first.file,
            first.span,
            format!(
                "these actors wait for each other: {path} ({handlers}). A handler whose \
                 member waits serves no message while it waits, so each of them \
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

/// [mixed-handler] [effect-any] [route-stub] A servant node's inputs: a mixed
/// handler's servant, an `of any X` router, or a derived route. One shape,
/// so the three cannot drift apart.
struct Servant<'a> {
    /// The handler, which names the node and the edges.
    handler: &'a str,
    /// The faces it wears, plain: a dependency on one of them binds outward
    /// (interception), so it closes no cycle.
    own_faces: &'a [&'a str],
    /// Its dependencies, by effect key, with their sites.
    deps: Vec<(String, usize, Span)>,
    /// What it sends to (its own sends, then its tasks'), by effect key.
    sends: Vec<(String, usize, Span)>,
    /// A gate in a member: its sends are waits.
    gate: Option<(usize, Span)>,
    /// A wait in a member: its sends are blocks.
    blocks: Option<(usize, Span)>,
}

impl Servant<'_> {
    /// The node's out-edges: an actor dependency is a target like a held
    /// addr; a plain one with mixed or wait-capable shareable handlers is an
    /// occupancy edge to each, exactly as from an actor handler; and every
    /// send to an actor effect is a target, of the kind the gate or wait
    /// makes it.
    fn add_to(
        self,
        graph: &mut BTreeMap<String, BTreeMap<String, Edge>>,
        is_actor: &dyn Fn(&str) -> bool,
        mixed_of: &BTreeMap<String, Vec<String>>,
        monitor_of: &BTreeMap<String, Vec<String>>,
    ) {
        let from = servant_node(self.handler);
        let mut targets: Vec<(String, usize, Span)> = Vec::new();
        let mut occupancies: Vec<(String, usize, Span)> = Vec::new();
        for (key, file, dep_span) in self.deps {
            let name = crate::typekey::plain(&key);
            if self.own_faces.contains(&name) {
                continue;
            }
            if is_actor(&key) {
                targets.push((name.to_string(), file, dep_span));
                continue;
            }
            for mh in mixed_of.get(name).into_iter().flatten() {
                occupancies.push((servant_node(mh), file, dep_span));
            }
            for mh in monitor_of.get(name).into_iter().flatten() {
                occupancies.push((monitor_node(mh), file, dep_span));
            }
        }
        for (target, file, span) in occupancies {
            let edge = Edge { kind: EdgeKind::Occupancy, handler: self.handler.to_string(), file, span };
            let slot = graph.entry(from.clone()).or_default();
            match slot.get(&target) {
                Some(existing) if existing.kind <= edge.kind => {}
                _ => {
                    slot.insert(target, edge);
                }
            }
        }
        targets.extend(self.sends.into_iter().filter(|(target, _, _)| is_actor(target)));
        for (target, file, span) in targets {
            let (kind, file, span) = match (self.gate, self.blocks) {
                (Some((gfile, gspan)), _) => (EdgeKind::Wait, gfile, gspan),
                (None, Some((bfile, bspan))) => (EdgeKind::Block, bfile, bspan),
                (None, None) => (EdgeKind::BackPressure, file, span),
            };
            let edge = Edge { kind, handler: self.handler.to_string(), file, span };
            let slot = graph.entry(from.clone()).or_default();
            match slot.get(target.as_str()) {
                Some(existing) if existing.kind <= edge.kind => {}
                _ => {
                    slot.insert(target, edge);
                }
            }
        }
    }
}

/// [mixed-handler] The graph node of a mixed handler's servant. Handlers are
/// not effects, so the node needs a name of its own — one that reads in a
/// printed path and cannot collide with a protocol's.
fn servant_node(handler: &str) -> String {
    format!("{handler}'s servant")
}

/// [effect-handle] [monitor-handler] The graph node of a shareable plain
/// handler worth pricing (deps or waits — option (b), user decision
/// 2026-09-20): its members run on the callers' threads, under the lock
/// when it is stateful, so a wait inside stalls every caller behind it.
fn monitor_node(handler: &str) -> String {
    format!("{handler}'s lock")
}

/// The base name of a written type, for a handler's `of` clause.
fn base_name(ty: &ast::Type) -> Option<&str> {
    match ty {
        ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
        _ => None,
    }
}

/// [effect-handler-deps] [effect-prereq] A handler's dependencies as the
/// checker expanded them (aliases flattened, its faces' prerequisites added),
/// by effect key, with the span each came from.
fn deps_of(out: &Checked, symbols: &Symbols<'_>, h: &ast::HandlerDecl) -> Vec<(String, Span)> {
    out.handler_deps
        .get(symbols.key_or(h, &h.name.name))
        .into_iter()
        .flatten()
        .filter_map(|(t, span)| match t.strip_quals() {
            crate::types::Ty::Named { name, .. } => Some((name.clone(), *span)),
            _ => None,
        })
        .collect()
}
