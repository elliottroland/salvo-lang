//! [mixed-handler] The mixed handler (SH-1, user decision 2026-09-19): a
//! plain-effect handler whose `send fn` members own the state (the servant)
//! and whose sync members run on the caller's thread (the façade), sending to
//! the servant and waiting on its answers. This file tests the **checker
//! surface** — classification, confinement, the façade-send resolution, the
//! spawn-only rule, the mailbox requirement, and the local-send-member
//! obligations. The emission is the following slice, and both backends refuse
//! a mixed spawn loudly until it lands.

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

fn main() [use, spawn, waitfor] {
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
