//! [actor-deadlock-cycle] [route-stub] [effect-any] A routed protocol in the
//! deadlock graph. The route `use any g` binds is derived in the IR
//! (`__Route_X`), so it is no AST handler; the graph builds its node from the
//! checker's `route_uses`, as the AST stub's `of any X` used to get it: a
//! servant of its own, forwarding to `X` and depending on its
//! `RouteSelector<X>`. Each expectation here is what the compiler reported
//! (and, for the graph, the edges it built) before the route moved to the IR,
//! when the stub was a handler `__Route_X of any X` in the AST.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// The two protocols every case shares: `Ping`, the routed group's, and
/// `Pong`, served by an actor that depends on the route.
const PROTOCOLS: &str = "\
import net

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

/// Checks `src` with std loaded, returning the rendered diagnostics (errors
/// and warnings) and the deadlock graph's edges.
fn check(src: &str) -> (Vec<String>, Vec<String>) {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let io_errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(io_errors.is_empty(), "failed to read std: {io_errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: sources.companions,
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let diags = resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.render(&program.files))
        .collect();
    let edges = salvo_core::deadlock::edges(&program, &symbols, &checked);
    (diags, edges)
}

/// The route's own edges, and every edge into its node.
fn route_edges(edges: &[String]) -> (Vec<&str>, Vec<&str>) {
    let node = "__Route_Ping's servant";
    let out = edges.iter().map(String::as_str).filter(|e| e.starts_with(node)).collect();
    let into = edges
        .iter()
        .map(String::as_str)
        .filter(|e| e.contains(&format!("→ {node}")))
        .collect();
    (out, into)
}

/// [actor-deadlock-cycle] A gate cycle through a routed protocol: `Ponging`
/// waits on whichever `Ping` member the route picks, and every member waits
/// on `Pong`. The callers' edge lands on `Ping` — what a binding of `any
/// Ping` may be — so the cycle is `Ping → Pong → Ping`, reported at the
/// member's gate, word for word as with the AST stub.
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
    let cycles: Vec<&String> = diags.iter().filter(|d| d.contains("wait for each other")).collect();
    assert_eq!(cycles.len(), 1, "one cycle: {diags:?}");
    assert!(
        cycles[0].starts_with(
            "error: these actors wait for each other: Ping → Pong → Ping (`Pinging`, `Ponging`). \
             A `replyto!` gate serves nothing but its own answer"
        ) && cycles[0].contains("main.sv:16:19"),
        "the gate cycle, at `Pinging`'s gate: {}",
        cycles[0]
    );
    let (out, into) = route_edges(&edges);
    assert_eq!(
        out,
        [
            "__Route_Ping's servant → Elected's lock (Occupancy, __Route_Ping)",
            "__Route_Ping's servant → Ping (BackPressure, __Route_Ping)",
        ],
        "{edges:#?}"
    );
    assert!(into.is_empty(), "{edges:#?}");
}

/// [route-stub] [effect-any] The near miss: the members answer directly, so
/// nothing waits back on `Pong`. The route's forward to `Ping` leaves from
/// its own node — read as `Ping`'s it would be a `Ping → Ping` cycle — and
/// the program is silent.
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
    let (out, _) = route_edges(&edges);
    assert!(out.contains(&"__Route_Ping's servant → Ping (BackPressure, __Route_Ping)"), "{edges:#?}");
}

/// [route-stub] [monitor-handler] The selector's dependency edges: a policy
/// of the program's own that is stateful and depends on `Pong` is a priced
/// shareable handler, so the route's node gets an occupancy edge to its
/// lock, as a servant's plain dependency does — next to std's `Elected`,
/// the graph being over types. Both protocols' handlers send to each other
/// ungated, so what is reported is the back-pressure warning, and the
/// route's node is on no cycle: nothing points at it.
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
{}
handler Watching<E>(config: RouteConfig = default_route_config()) [Pong] of RouteSelector<E> {{
    state {{
        seen: Int = 0
    }}

    fn config() -> RouteConfig {{
        return copy(config)
    }}

    fn changed(view: RouteView<E>) -> None => view {{
        seen = seen + 1
        poke()
    }}

    fn select(view: RouteView<E>, key: Long?) -> eff[E]? => view, key {{
        return None
    }}
}}

fn main(g: eff[ActorGroup<Ping>], b: eff[Pong]) [use, spawn]
=> !g, !b {{
    use b
    use Watching<Ping>()
    use any g
    let _p = spawn Ponging() on pool(1)
}}
",
        ponging("replyto")
    );
    let (diags, edges) = check(&src);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(
        diags[0].starts_with(
            "warning: these actors send to each other in a cycle: Ping → Pong → Ping \
             (`Pinging`, `Ponging`)"
        ),
        "{}",
        diags[0]
    );
    let (out, into) = route_edges(&edges);
    assert_eq!(
        out,
        [
            "__Route_Ping's servant → Elected's lock (Occupancy, __Route_Ping)",
            "__Route_Ping's servant → Ping (BackPressure, __Route_Ping)",
            "__Route_Ping's servant → Watching's lock (Occupancy, __Route_Ping)",
        ],
        "{edges:#?}"
    );
    assert!(into.is_empty(), "{edges:#?}");
    assert!(edges.contains(&"Watching's lock → Pong (BackPressure, Watching)".to_string()), "{edges:#?}");
}
