//! [actor-deadlock-cycle] [actor-route-stub] [effect-any] A routed protocol in the
//! deadlock graph. The route `use any g` binds is derived in the IR
//! (`__Route_X`), so it is no AST handler; the graph builds its node from the
//! checker's `route_uses`, as a written `of any X` router gets one: a servant
//! of its own, depending on its `RouteSelector<X>`. Since the router
//! in-edges (2026-10-26), every dependency on `any X` points at it — an
//! occupancy edge, since the route locks — next to its edge into `X`, and
//! its forward to `X` is no edge: it would re-spell the caller's.


use salvo_core::{resolve, Symbols};

/// The two protocols every case shares: `Ping`, the routed group's, and
/// `Pong`, served by an actor that depends on the route.
const PROTOCOLS: &str = "\
import std.net

actor effect Ping {
    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out
}

actor effect Pong {
    send fn pong(out: Reply<Int>) => !out
    send fn poke()
}
";

/// `Ponging` depends on `any Ping`, which `main` binds to the route; its
/// `pong` asks a member and parks [actor-replyto]. `gate` picks `replyto!`
/// (gated) or `replyto`.
fn ponging(gate: &str) -> String {
    format!(
        "
handler Ponging() [any Ping] of Pong {{
    mailbox {{ capacity: 1 }}

    send fn pong(out: Reply<Int>) {{
        ping(\"a\", {gate} pinged(out))
    }}
    send fn pinged(out: Reply<Int>, s: Str) => !out {{
        out.send(1)
    }}
    send fn poke() {{}}
}}
"
    )
}

/// A diagnostic as these tests read it: [diag-id] its id, its severity, the
/// line and column it starts at in `main.sv` with that line's text, and its
/// message, whose argument is the cycle — the graph's node names, which an
/// annotation cannot say.
#[derive(Debug)]
struct Diag {
    id: &'static str,
    error: bool,
    line: u32,
    col: u32,
    source_line: String,
    message: String,
}

impl Diag {
    /// Whether the message names this cycle: its path and its handlers.
    fn names(&self, cycle: &str) -> bool {
        self.message.contains(cycle)
    }
}

/// Checks `src` with std loaded, returning the diagnostics (errors and
/// warnings) and the deadlock graph's edges.
fn check(src: &str) -> (Vec<Diag>, Vec<String>) {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let program = salvo_testkit::front::program("kt", &[("main.sv", src)]).program;
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    let diags = resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| {
            let content = &program.files[d.file].content;
            let (line, col) = salvo_syntax::span::line_col(content, d.span.start);
            Diag {
                id: d.id.unwrap_or("?"),
                error: d.is_error(),
                line,
                col,
                source_line: content.lines().nth(line as usize - 1).unwrap_or_default().to_string(),
                message: d.message.clone(),
            }
        })
        .collect();
    let edges = salvo_core::deadlock::edges(&program, &symbols, &checked);
    (diags, edges)
}

/// The route's own edges, and every edge into its node.
fn route_edges(edges: &[String]) -> (Vec<&str>, Vec<&str>) {
    node_edges(edges, "__Route_Ping's servant")
}

/// A node's own edges, and every edge into it.
fn node_edges<'e>(edges: &'e [String], node: &str) -> (Vec<&'e str>, Vec<&'e str>) {
    let out = edges.iter().map(String::as_str).filter(|e| e.starts_with(node)).collect();
    let into = edges
        .iter()
        .map(String::as_str)
        .filter(|e| e.contains(&format!("→ {node}")))
        .collect();
    (out, into)
}

/// The cycle diagnostics among `diags`: the errors and warnings this file is
/// about, and nothing else the program might say.
fn cycles(diags: &[Diag]) -> Vec<&Diag> {
    diags
        .iter()
        .filter(|d| d.id.starts_with("actor-deadlock-cycle:") || d.id == "actor-mixed-handler:occupancy-cycle")
        .collect()
}

/// [actor-deadlock-cycle] A gate cycle through a routed protocol: `Ponging`
/// waits on whichever `Ping` member the route picks, and every member waits
/// on `Pong`. The callers' edge lands on `Ping` — what a binding of `any
/// Ping` may be — so the cycle is `Ping → Pong → Ping`, reported at the
/// member's gate, word for word as with the AST stub. [effect-any] It lands
/// on the route too, as occupancy, but the route's forward is no edge, so
/// the cycle is not reported a second time through the route.
#[test]
fn a_gate_cycle_through_a_route_is_refused() {
    let src = format!(
        "{PROTOCOLS}
handler Pinging(back: eff[Pong]) of Ping {{
    mailbox {{ capacity: 1 }}

    send fn ping(k: Key Str, out: Reply<Str>) {{
        back.pong(replyto! ponged(out))
    }}
    send fn ponged(out: Reply<Str>, n: Int) => !out {{
        out.send(\"x\")
    }}
}}
{}
fn main(g: eff[ActorGroup<Ping>]) [use, spawn]
=> !g {{
    use Sharded<Ping>()
    use any g
    let _p = spawn Ponging() on pool(1)
}}
",
        ponging("replyto!")
    );
    let (diags, edges) = check(&src);
    let cycles: Vec<&Diag> = cycles(&diags);
    assert_eq!(cycles.len(), 1, "one cycle: {diags:?}");
    assert!(
        (cycles[0].error, cycles[0].id, cycles[0].line, cycles[0].col) == (true, "actor-deadlock-cycle:gate-cycle", 16, 19)
            && cycles[0].names("Ping → Pong → Ping (`Pinging`, `Ponging`)"),
        "the gate cycle, at `Pinging`'s gate: {:?}",
        cycles[0]
    );
    let (out, into) = route_edges(&edges);
    assert_eq!(
        out,
        ["__Route_Ping's servant → Elected's lock (Occupancy, __Route_Ping)"],
        "{edges:#?}"
    );
    assert_eq!(into, ["Pong → __Route_Ping's servant (Occupancy, Ponging)"], "{edges:#?}");
}

/// [actor-route-stub] [effect-any] The near miss: the members answer directly, so
/// nothing waits back on `Pong`. The route's forward to `Ping` is no edge —
/// read as `Ping`'s it would be a `Ping → Ping` cycle, and as the route's
/// it would re-spell `Pong → Ping` — and the program is silent, though
/// `Ponging`'s dependency points at the route.
#[test]
fn a_routes_forward_is_not_a_cycle_of_its_protocol() {
    let src = format!(
        "{PROTOCOLS}
handler Pinging() of Ping {{
    mailbox {{ capacity: 1 }}

    send fn ping(k: Key Str, out: Reply<Str>) {{
        out.send(\"x\")
    }}
}}
{}
fn main(g: eff[ActorGroup<Ping>]) [use, spawn]
=> !g {{
    use Sharded<Ping>()
    use any g
    let _p = spawn Ponging() on pool(1)
}}
",
        ponging("replyto!")
    );
    let (diags, edges) = check(&src);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(edges.iter().all(|e| !e.starts_with("Ping → Ping")), "{edges:#?}");
    assert!(edges.contains(&"Pong → Ping (Wait, Ponging)".to_string()), "{edges:#?}");
    let (out, into) = route_edges(&edges);
    assert!(out.iter().all(|e| !e.contains("→ Ping ")), "{edges:#?}");
    assert_eq!(into, ["Pong → __Route_Ping's servant (Occupancy, Ponging)"], "{edges:#?}");
}


/// A selector of the program's own that is stateful and depends on `Pong`,
/// whose `changed` pokes it — a priced shareable handler [actor-monitor-handler] —
/// and the `main` that routes `Ping` through it.
const WATCHING: &str = "
handler Watching<E>(config: RouteConfig = default_route_config()) [Pong] of RouteSelector<E> {
    state {
        seen: Int = 0
    }

    fn config() -> RouteConfig {
        return copy(config)
    }

    fn changed(view: RouteView<E>) -> None => view {
        seen = seen + 1
        poke()
    }

    fn select(view: RouteView<E>, key: Long?) -> eff[E]? => view, key {
        return None
    }
}

fn main(g: eff[ActorGroup<Ping>], b: eff[Pong]) [use, spawn]
=> !g, !b {
    use b
    use Watching<Ping>()
    use any g
    let _p = spawn Ponging() on pool(1)
}
";

/// A `Ping` handler that answers at once and sends nothing.
const ANSWERING: &str = "
handler Pinging() of Ping {
    mailbox { capacity: 1 }

    send fn ping(k: Key Str, out: Reply<Str>) {
        out.send(\"x\")
    }
}
";

/// [actor-route-stub] [actor-monitor-handler] The selector's dependency edges: a policy
/// of the program's own that is stateful and depends on `Pong` is a priced
/// shareable handler, so the route's node gets an occupancy edge to its
/// lock, as a servant's plain dependency does — next to std's `Elected`,
/// the graph being over types. Both protocols' handlers send to each other
/// ungated, which is the back-pressure warning; and [effect-any]
/// `Ponging`'s dependency on `any Ping` points at the route, which closes
/// the second cycle through the selector's lock — an error, since it holds
/// an occupancy edge.
#[test]
fn a_routes_selector_dependency_is_an_occupancy_edge() {
    let src = format!(
        "{PROTOCOLS}
handler Pinging(back: eff[Pong]) of Ping {{
    mailbox {{ capacity: 1 }}

    send fn ping(k: Key Str, out: Reply<Str>) {{
        back.pong(replyto ponged(out))
    }}
    send fn ponged(out: Reply<Str>, n: Int) => !out {{
        out.send(\"x\")
    }}
}}
{}{WATCHING}",
        ponging("replyto")
    );
    let (diags, edges) = check(&src);
    assert_eq!(diags.len(), 2, "{diags:?}");
    assert!(
        (diags[0].error, diags[0].id) == (false, "actor-deadlock-cycle:send-cycle")
            && diags[0].names("Ping → Pong → Ping (`Pinging`, `Ponging`)"),
        "{:?}",
        diags[0]
    );
    assert!(
        (diags[1].error, diags[1].id) == (true, "actor-mixed-handler:occupancy-cycle")
            && diags[1].names(
                "Watching's lock → Pong → __Route_Ping's servant → Watching's lock \
                 (`Ponging`, `Watching`, `__Route_Ping`)"
            ),
        "{:?}",
        diags[1]
    );
    let (out, into) = route_edges(&edges);
    assert_eq!(
        out,
        [
            "__Route_Ping's servant → Elected's lock (Occupancy, __Route_Ping)",
            "__Route_Ping's servant → Watching's lock (Occupancy, __Route_Ping)",
        ],
        "{edges:#?}"
    );
    assert_eq!(into, ["Pong → __Route_Ping's servant (Occupancy, Ponging)"], "{edges:#?}");
    assert!(edges.contains(&"Watching's lock → Pong (BackPressure, Watching)".to_string()), "{edges:#?}");
}

/// [actor-deadlock-cycle] [effect-any] [actor-route-stub] The ROADMAP repro: the
/// members send nothing, so the only cycle is the one through the route.
/// `Ponging` calls into the route on its own thread, under the route's lock;
/// the route consults `Watching` under *its* lock, whose `changed` pokes
/// `Pong` — whose activation is the one stuck inside the route. `Pong →
/// route → Watching's lock → Pong`, an error because it holds occupancy
/// edges, anchored at the dependency on `any Ping` (the seam where the
/// binding is chosen). The near miss: a selector that pokes nothing has no
/// node, and the same program is silent.
#[test]
fn a_cycle_through_a_routes_selector_is_refused() {
    let src = format!("{PROTOCOLS}{ANSWERING}{}{WATCHING}", ponging("replyto"));
    let (diags, edges) = check(&src);
    let found = cycles(&diags);
    assert_eq!(found.len(), 1, "{diags:?}");
    assert!(
        (found[0].error, found[0].id) == (true, "actor-mixed-handler:occupancy-cycle")
            && found[0].names(
                "Pong → __Route_Ping's servant → Watching's lock → Pong \
                 (`Ponging`, `Watching`, `__Route_Ping`)"
            )
            && found[0].source_line.contains("handler Ponging() [any Ping] of Pong"),
        "{:?}",
        found[0]
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    let (_, into) = route_edges(&edges);
    assert_eq!(into, ["Pong → __Route_Ping's servant (Occupancy, Ponging)"], "{edges:#?}");

    let quiet = WATCHING
        .replace(") [Pong] of RouteSelector<E>", ") of RouteSelector<E>")
        .replace("        poke()\n", "");
    let src = format!("{PROTOCOLS}{ANSWERING}{}{quiet}", ponging("replyto"));
    let (diags, edges) = check(&src);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(edges.iter().all(|e| !e.starts_with("Watching's lock")), "{edges:#?}");
}

/// A written router `of any Ping` that pokes `Pong` before it forwards to
/// the one member it holds; `state` is spliced in (empty for a bare one).
fn fanning(state: &str, bump: &str) -> String {
    format!(
        "
handler Fanning(to: eff[Ping]) [Pong] of any Ping {{
    mailbox {{ capacity: 1 }}
{state}
    send fn ping(k: Key Str, out: Reply<Str>) {{
{bump}        poke()
        to.ping(k, out)
    }}
}}

fn main(p: eff[Ping], b: eff[Pong]) [use, spawn]
=> !p, !b {{
    use b
    use Fanning(p)
    let _p = spawn Ponging() on pool(1)
}}
"
    )
}

/// [actor-deadlock-cycle] [effect-any] A written router's node gets the
/// same in-edges a route's does: `Ponging`'s dependency on `any Ping` lands
/// on `Ping` and on `Fanning`'s servant. `Fanning` is stateless and waits
/// for nothing, so it binds bare and its member is `Ponging`'s own code: the
/// edge in is `Ponging`'s kind (back-pressure, its `replyto` being
/// ungated), and the cycle through the router's poke is the warning. Its
/// forward to `Ping` through the addr it holds is no edge.
#[test]
fn a_bare_written_routers_node_gets_the_callers_edge() {
    let src = format!("{PROTOCOLS}{ANSWERING}{}{}", ponging("replyto"), fanning("", ""));
    let (diags, edges) = check(&src);
    let found = cycles(&diags);
    assert_eq!(found.len(), 1, "{diags:?}");
    assert!(
        (found[0].error, found[0].id) == (false, "actor-deadlock-cycle:send-cycle")
            && found[0].names("Fanning's servant → Pong → Fanning's servant (`Fanning`, `Ponging`)"),
        "{:?}",
        found[0]
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    let (out, into) = node_edges(&edges, "Fanning's servant");
    assert_eq!(out, ["Fanning's servant → Pong (BackPressure, Fanning)"], "{edges:#?}");
    assert_eq!(into, ["Pong → Fanning's servant (BackPressure, Ponging)"], "{edges:#?}");
    assert!(edges.contains(&"Pong → Ping (BackPressure, Ponging)".to_string()), "{edges:#?}");
}

/// [actor-deadlock-cycle] [effect-any] [actor-monitor-handler] A stateful written
/// router binds under a lock, so a caller inside it occupies, as in a priced
/// shareable handler: the edge in is an occupancy edge, and the same cycle
/// is an error.
#[test]
fn a_stateful_written_router_is_an_occupancy_edge() {
    let src = format!(
        "{PROTOCOLS}{ANSWERING}{}{}",
        ponging("replyto"),
        fanning("    state {\n        n: Int = 0\n    }\n", "        n = n + 1\n")
    );
    let (diags, edges) = check(&src);
    let found = cycles(&diags);
    assert_eq!(found.len(), 1, "{diags:?}");
    assert!(
        (found[0].error, found[0].id) == (true, "actor-mixed-handler:occupancy-cycle")
            && found[0].names("Fanning's servant → Pong → Fanning's servant (`Fanning`, `Ponging`)")
            && found[0].source_line.contains("handler Ponging() [any Ping] of Pong"),
        "{:?}",
        found[0]
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    let (_, into) = node_edges(&edges, "Fanning's servant");
    assert_eq!(into, ["Pong → Fanning's servant (Occupancy, Ponging)"], "{edges:#?}");
}
