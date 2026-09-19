//! [mixed-handler] The mixed handler (SH-1, user decision 2026-09-19): a
//! plain-effect handler whose `send fn` members own the state (the servant)
//! and whose sync members run on the caller's thread (the façade), sending to
//! the servant and waiting on its answers. This file tests the **checker
//! surface** — classification, confinement, the façade-send resolution, the
//! spawn-only rule, the mailbox requirement, the local-send-member
//! obligations, the `defer` contract point and servant parking. The emission
//! lives in the backends, verified end to end by their codegen tests.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "\
export intrinsic type Int
export intrinsic type Str
export intrinsic type Bool
export intrinsic type Addr<E>
export struct Mailbox { capacity: Int }
export linear intrinsic type Reply<T>
export intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value
export intrinsic type Pool
export intrinsic fn pool(size: Int) [spawn] -> Pool => size
export effect Console {
    fn println(line: Str) -> None => !line
}
";

/// The design's own motivating handler: `Random` backed by state that lives
/// behind a thread boundary, reached through a plain effect.
const PRELUDE: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    cursor: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
"#;

fn diagnostics(src: &str) -> Vec<(bool, String)> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        format!("{PRELUDE}\n{src}"),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diags) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| (d.is_error(), d.message.clone()))
        .collect()
}

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        format!("{PRELUDE}\n{src}"),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// The whole checker surface in one program: the declaration checks clean
/// (local send member with its clause, the façade's send + `waitfor` with
/// **no** `[waitfor]` anywhere — SH-5(d)'s down payment), the spawn answers a
/// handle, `use` binds it, and the plain effect's member is callable from
/// ordinary code that declares nothing.
#[test]
fn a_mixed_handler_checks_clean_end_to_end() {
    let errs = errors(
        "\
fn draw_twice() [Random] -> Int {
    return next() + next()
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)
    use rng
    let n = draw_twice()
}
",
    );
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [mixed-handler] Confinement: a sync member touching a state field gets the
/// rule by name, not an "unknown variable" — reads and writes both.
#[test]
fn a_facade_member_cannot_touch_state() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Peeking() of Peek {
    mailbox { capacity: 2 }
    hidden: Int = 0

    send fn poke() {
        hidden = hidden + 1
    }

    fn peek() -> Int {
        return hidden
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`hidden` is the servant's state")
            && m.contains("confines")
            && m.contains("send to a member that answers")),
        "expected the confinement diagnostic: {errs:?}"
    );
}

/// [mixed-handler] Spawn-only: `use` of a mixed handler is refused where the
/// binding is chosen, naming the reason and the spelling that works.
#[test]
fn a_mixed_handler_cannot_be_use_bound() {
    let errs = errors(
        "\
fn main() [use] {
    use CyclicRandom(1)
    let n = next()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("is mixed")
            && m.contains("can only be `spawn`ed")
            && m.contains("no mailbox for the sends")),
        "expected the spawn-only refusal: {errs:?}"
    );
}

/// [mixed-handler] [actor-mailbox] Send members are delivered through a
/// mailbox, so a mixed handler without the slot is told to add one.
#[test]
fn a_mixed_handler_needs_a_mailbox() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler NoBox() of Peek {
    tally: Int = 0

    send fn poke() {
        tally = tally + 1
    }

    fn peek() -> Int {
        return waitfor got: Reply<Int> {
            ask(got)
        }
    }

    send fn ask(out: Reply<Int>) => !out {
        send(out, tally)
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("has a mailbox")
            && m.contains("`send fn` members are delivered through one")),
        "expected the mailbox requirement: {errs:?}"
    );
}

/// [mixed-handler] [free-send-fn] A handler-local send member carries the
/// free send fn's obligations: the clause must be written out (a scheduled
/// body consumes what it is given), and a kept parameter is refused.
#[test]
fn a_local_send_member_writes_its_clause() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Sloppy() of Peek {
    mailbox { capacity: 2 }
    tally: Int = 0

    send fn poke(n: Int) {
        tally = tally + n
    }

    fn peek() -> Int {
        return 0
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("deduction clause must be written out")
            && m.contains("`=> !n`")),
        "expected the clause requirement: {errs:?}"
    );
}

/// [mixed-handler] The servant's dispatch is by member name, so overloading a
/// local send member is refused for now.
#[test]
fn overloaded_local_send_members_are_refused() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Twice() of Peek {
    mailbox { capacity: 2 }
    tally: Int = 0

    send fn poke(n: Int) => !n {
        tally = tally + n
    }

    send fn poke(s: Str) => !s {
        tally = tally + 1
    }

    fn peek() -> Int {
        return 0
    }
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("overloaded") && m.contains("rename one")),
        "expected the overload refusal: {errs:?}"
    );
}

/// [mixed-handler] The façade sees no handler dependencies (they would live
/// in the servant), and — the first-slice cut — a mixed handler cannot
/// declare any yet; the spawn names the remedy.
#[test]
fn a_mixed_handler_with_dependencies_is_refused_for_now() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Noisy() [Console] of Peek {
    mailbox { capacity: 2 }
    tally: Int = 0

    send fn poke() {
        tally = tally + 1
        println(\"poked\")
    }

    fn peek() -> Int {
        return 0
    }
}

fn main() [use, spawn] {
    let p = spawn Noisy()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("declares dependencies")
            && m.contains("does not support yet")
            && m.contains("constructor parameter")),
        "expected the dependency cut: {errs:?}"
    );
}

/// [mixed-handler] One face for now: the façade value behind several effect
/// types has no representation yet.
#[test]
fn a_multi_face_mixed_spawn_is_refused() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Wide(seed: Int) of Random, Peek {
    mailbox { capacity: 2 }
    cursor: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }

    fn peek() -> Int {
        return 0
    }
}

fn main() [use, spawn] {
    let both = spawn Wide(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("several plain effects") && m.contains("not supported yet")),
        "expected the multi-face refusal: {errs:?}"
    );
}


/// [actor-deadlock-cycle] [mixed-handler] SH-4: the occupancy edge, inferred
/// from declarations. An actor handler depending on a plain effect with a
/// mixed handler in the program may park on that servant, and the servant
/// here sends back to the actor's own protocol -- the design's upcall shape,
/// as close as the first slice can write it. The cycle mixes an occupancy
/// edge with a back-pressure edge and is an **error**: the ungated-side
/// downgrade does not apply, because an occupied activation serves nothing.
#[test]
fn an_occupancy_cycle_is_an_error_even_through_back_pressure() {
    let diags = diagnostics(
        "\
actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Feedback(drawer: Addr<Drawer>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        drawer.draw(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}
",
    );
    let errs: Vec<&String> = diags.iter().filter(|(e, _)| *e).map(|(_, m)| m).collect();
    assert!(
        errs.iter().any(|m| m
            .contains("wait for each other through a shared handler")
            && m.contains("Feedback's servant")
            && m.contains("Drawer")
            && m.contains("send plus a continuation")),
        "expected the occupancy-cycle error: {diags:?}"
    );
}

/// The same dependency with no return path is an edge, not a cycle: nothing
/// is reported.
#[test]
fn an_occupancy_edge_without_a_cycle_is_silent() {
    let diags = diagnostics(
        "\
actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(1)
    let drawer = spawn Drawing() use rng
    let _drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
}
",
    );
    assert!(
        diags.is_empty(),
        "an acyclic occupancy edge reports nothing: {diags:?}"
    );
}

/// [defer-deduction] The rung-4 contract point: a reply that leaves the
/// activation any way but the discharge must be declared `defer` — here by
/// forwarding to another actor, and by storing into state. The diagnostic
/// names the declaration.
#[test]
fn an_escaping_reply_requires_defer() {
    let forwarded = errors(
        "\
actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Forwarding(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(
        forwarded.iter().any(|m| m.contains("outlive the activation")
            && m.contains("`=> defer out`")),
        "expected the forwarding escape: {forwarded:?}"
    );

    let passed_on = errors(
        "\
fn stash(keep: Reply<Int>) [] -> None => !keep {
    send(keep, 0)
}

handler Latch() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        stash(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(
        passed_on.iter().any(|m| m.contains("outlive the activation")
            && m.contains("`=> defer out`")),
        "expected the passing-on escape: {passed_on:?}"
    );
}

/// And with the declaration, both are legal — plus the upper bound: a
/// declared `defer` whose body answers in-frame anyway is fine (SH-10(b)).
#[test]
fn a_declared_defer_permits_the_escape_and_need_not_be_exercised() {
    let forwarded = errors(
        "\
actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Forwarding(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(forwarded.is_empty(), "a declared deferral forwards: {forwarded:?}");

    let unexercised = errors(
        "\
handler Prompt() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        send(out, tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(
        unexercised.is_empty(),
        "an upper bound: a `defer` the body never exercises is legal: {unexercised:?}"
    );
}

/// [mixed-handler] [actor-self-send] A servant member reaches a sibling
/// `send fn` by bare call (user decision 2026-09-19) — a self-enqueue — and
/// `@self` is the explicit spelling, in send members and in sync members
/// alike. The whole chain checks clean.
#[test]
fn a_servant_member_reaches_a_sibling_by_bare_call() {
    let diags = diagnostics(
        "\
handler Chain() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn step(out: Reply<Int>) => defer out {
        tally = tally + 1
        relay(out)
    }

    send fn relay(out: Reply<Int>) => defer out {
        deliver@self(out)
    }

    send fn deliver(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step@self(got)
        }
    }
}
",
    );
    assert!(
        diags.is_empty(),
        "the sibling-call chain checks clean: {diags:?}"
    );
}

/// [defer-deduction] A reply riding a bare sibling call's payload leaves the
/// activation, so the escape hook fires exactly as it does for an addr send:
/// the member must declare `defer`.
#[test]
fn a_reply_sent_onward_by_a_bare_sibling_call_requires_defer() {
    let errs = errors(
        "\
handler Chain() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn step(out: Reply<Int>) => !out {
        tally = tally + 1
        relay(out)
    }

    send fn relay(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step(got)
        }
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("outlive the activation")
            && m.contains("`=> defer out`")),
        "expected the payload escape naming the declaration: {errs:?}"
    );
}

/// The bare-call resolution reaches `send fn` siblings only: a sync member
/// named bare from a send member stays unresolved, as sibling members always
/// were outside this rule.
#[test]
fn a_sync_sibling_stays_unresolved_from_a_send_member() {
    let errs = errors(
        "\
handler Chain() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn step(out: Reply<Int>) => !out {
        tally = tally + peek()
        send(out, tally)
    }

    fn peek() -> Int {
        return 1
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            step(got)
        }
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("peek")),
        "expected `peek` to stay unresolved from the servant: {errs:?}"
    );
}

/// [defer-deduction] [actor-replyto] Parking inside a mixed handler — the
/// lift of the staged refusal: a servant member may mint a continuation on
/// another of its own send members, provided the captured reply is declared
/// `defer`. The whole rung-4 shape checks clean.
#[test]
fn parking_in_a_mixed_handler_checks_clean() {
    let diags = diagnostics(
        "\
actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    n: Int = 0

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Parking(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out, !drawn {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(
        diags.is_empty(),
        "parking in a mixed handler checks clean: {diags:?}"
    );
}

/// [defer-deduction] The capture hook still holds: parking a received reply
/// in a continuation lets its answer outlive the activation, so it must be
/// declared `defer` — the diagnostic names the declaration.
#[test]
fn parking_a_received_reply_requires_defer() {
    let errs = errors(
        "\
actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    n: Int = 0

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Parking(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        tally = tally + 1
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out, !drawn {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("outlive the activation")
            && m.contains("`=> defer out`")),
        "expected the capture escape naming the declaration: {errs:?}"
    );
}

/// [actor-deadlock-cycle] [defer-deduction] A gated park in a mixed servant
/// makes its sends Wait-kind, mirroring the actor logic: the servant serves
/// nothing but its answer while the gate holds, so a peer whose handler
/// occupies the servant's effect closes a reported cycle.
#[test]
fn a_gated_park_in_a_mixed_servant_closes_a_cycle() {
    let diags = diagnostics(
        "\
actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Feedback(drawer: Addr<Drawer>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        drawer.draw(replyto! settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out, !drawn {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}
",
    );
    let errs: Vec<&String> = diags.iter().filter(|(e, _)| *e).map(|(_, m)| m).collect();
    assert!(
        errs.iter().any(|m| m
            .contains("wait for each other through a shared handler")
            && m.contains("Feedback's servant")
            && m.contains("Drawer")),
        "expected the gated-servant cycle error: {diags:?}"
    );
}
