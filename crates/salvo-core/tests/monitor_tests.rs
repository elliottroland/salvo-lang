//! [monitor-handler] The monitor spawn (SH-3, user decision 2026-09-19):
//! `spawn H(args)` where every face of `H` is a **plain** effect shares one
//! instance behind a lock, answers the same `Addr<E>` type an actor spawn
//! answers, and is bound with `use addr` like any handle. The restriction is
//! the design: a monitor handler declares **no dependencies**, so its members
//! are pure state transformation — no effects, no waits — and the lock is
//! innermost by construction.

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

/// The shared shapes: a plain effect with mutable state behind it (the
/// monitor), and an actor that depends on it.
const PRELUDE: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    cursor: Int = 0

    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}
"#;

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

/// The whole surface in one program — the acceptance test: a monitor spawned
/// with no `on` clause, its handle `use`-bound in `main` (calls resolve
/// through the effect), and the same handle supplied to a spawned actor's
/// dependency clause. An addr is freely reusable, so one handle serves both.
#[test]
fn a_monitor_spawn_shares_a_plain_effect_handler() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    let rng = spawn CyclicRandom(12345)
    use rng
    let first = next()
    let drawer = spawn Drawing() use rng
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    let again = next()
}
",
    );
    assert!(errs.is_empty(), "got {errs:?}");
}

/// [monitor-handler] Naming `Addr<E>` of a plain effect is legal since SH-3:
/// it is the shared monitor's handle — in a parameter, a return, a `let`.
/// (Until 2026-09-19 this was [actor-effect-kind]'s refusal.)
#[test]
fn a_plain_effect_addr_is_a_legal_type() {
    let errs = errors(
        "\
fn hold(handle: Addr<Random>) -> Int => handle {
    return 1
}

fn main() [use, spawn] {
    let rng = spawn CyclicRandom(1)
    let n = hold(rng)
}
",
    );
    assert!(errs.is_empty(), "got {errs:?}");
}

/// A monitor runs on its callers' threads, so an `on` clause has nothing to
/// place and is refused by name.
#[test]
fn a_monitor_spawn_refuses_a_pool() {
    let errs = errors(
        "\
fn main() [use, spawn] {
    let rng = spawn CyclicRandom(1) on pool(1)
}
",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("monitor") && m.contains("Remove the `on` clause")),
        "expected the placement refusal: {errs:?}"
    );
}

/// [use-local] The dependency restriction was lifted 2026-09-20 (user
/// decision): a monitor may declare shareable deps, captured as owned
/// handles from the **enclosing scope** at the spawn. With no binding in
/// scope the resolution fails by name; with one, the spawn is legal.
#[test]
fn a_monitor_spawn_resolves_dependencies_from_the_enclosing_scope() {
    let errs = errors(
        "\
handler Noisy(seed: Int) [Console] of Random {
    cursor: Int = 0

    fn next() -> Int {
        println(\"drawing\")
        cursor = cursor + seed
        return cursor
    }
}

fn main() [use, spawn] {
    let rng = spawn Noisy(1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("depends on effect `Console`")
            && m.contains("register one before this spawn")),
        "expected the unresolved-dependency error: {errs:?}"
    );

    let errs = errors(
        "\
handler Quiet() of Console {
    fn println(line: Str) -> None => !line {
        let _ = line
    }
}

handler Noisy(seed: Int) [Console] of Random {
    cursor: Int = 0

    fn next() -> Int {
        println(\"drawing\")
        cursor = cursor + seed
        return cursor
    }
}

fn main() [use, spawn] {
    use Quiet()
    let rng = spawn Noisy(1)
}
",
    );
    assert!(
        errs.is_empty(),
        "a dep-bearing monitor spawn resolves from scope: {errs:?}"
    );
}

/// A handler of several plain effects has no representation yet: one shared
/// instance would sit behind several faces. Refused rather than guessed.
#[test]
fn a_multi_face_monitor_spawn_is_refused() {
    let errs = errors(
        "\
effect Peek {
    fn peek() -> Int
}

handler Wide(seed: Int) of Random, Peek {
    cursor: Int = 0

    fn next() -> Int {
        cursor = cursor + seed
        return cursor
    }

    fn peek() -> Int {
        return cursor
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

/// [actor-sendable] The instance crosses to every thread that binds the
/// handle, so a fn-typed constructor parameter — shared, not owned — is
/// refused by name at the spawn. `use local`-binding the same handler stays
/// legal ([use-local]: since 2026-09-20 the bare `use` is itself a sharing
/// site, so the lock-free binding is the spelled opt-out).
#[test]
fn a_monitor_spawn_refuses_unsendable_state() {
    let errs = errors(
        "\
handler Derived(step: (n: Int) -> Int) of Random {
    cursor: Int = 0

    fn next() -> Int {
        cursor = step(cursor)
        return cursor
    }
}

fn main() [use, spawn] {
    let rng = spawn Derived(n -> n + 1)
    use local Derived(n -> n + 2)
    let ok = next()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot be shared")
            && m.contains("step")
            && m.contains("must be sendable")),
        "expected the sendability refusal: {errs:?}"
    );
    assert!(
        !errs.iter().any(|m| m.contains("use local Derived")),
        "the `use local` binding of the same handler must stay legal: {errs:?}"
    );
}

// ===== shareable by default (user decisions 2026-09-20) [use-local] =====

/// [use-local] A bare `use` binds shareable with no annotation anywhere: a
/// stateful handler as a monitor, satisfying a callee's bare `[E]`
/// requirement.
#[test]
fn a_bare_use_of_a_stateful_handler_is_shareable() {
    let errs = errors(
        "\
fn draw() [Random] -> Int {
    return next()
}

fn main() [use] {
    use CyclicRandom(7)
    let n = draw()
}
",
    );
    assert!(errs.is_empty(), "expected a clean program: {errs:?}");
}

/// [effect-local] The call-site rule, both halves: a `use local` binding
/// satisfies only `[local E]` requirements — a bare `[E]` means shareable
/// and is refused, naming both remedies.
#[test]
fn a_local_binding_satisfies_only_local_requirements() {
    let errs = errors(
        "\
fn draw() [Random] -> Int {
    return next()
}

fn peek() [local Random] -> Int {
    return next()
}

fn main() [use] {
    use local CyclicRandom(7)
    let n = draw()
    let m = peek()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("requires a shareable `Random`")
            && m.contains("local Random")),
        "expected the bare requirement refused with both remedies: {errs:?}"
    );
    assert_eq!(errs.len(), 1, "the `[local Random]` call is legal: {errs:?}");
}

/// [effect-local] A shareable binding satisfies both forms: `[local E]` is
/// the weaker requirement, so a monitor binding passes through it.
#[test]
fn a_shareable_binding_satisfies_a_local_requirement() {
    let errs = errors(
        "\
fn peek() [local Random] -> Int {
    return next()
}

fn main() [use] {
    use CyclicRandom(7)
    let n = peek()
}
",
    );
    assert!(errs.is_empty(), "expected a clean program: {errs:?}");
}

/// [use-local] A handler that cannot be shared is refused under the bare
/// default, and the error names the opt-out.
#[test]
fn an_unshareable_handler_names_the_opt_out() {
    let errs = errors(
        "\
handler Derived(step: (n: Int) -> Int) of Random {
    cursor: Int = 0

    fn next() -> Int {
        cursor = step(cursor)
        return cursor
    }
}

fn main() [use] {
    use Derived(n -> n + 1)
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot be bound shareable")
            && m.contains("step")
            && m.contains("use local Derived")),
        "expected the refusal to name the opt-out: {errs:?}"
    );
}

/// [use-local] [effect-local] A `local E` dependency accepts a scope-local
/// binding, which no shared instance can capture — so it pins the handler
/// itself to `use local`.
#[test]
fn a_local_dependency_pins_the_handler_to_local_bindings() {
    let errs = errors(
        "\
effect Beacon {
    fn shine() -> Int
}

handler Relaying [local Random] of Beacon {
    fn shine() -> Int {
        return next()
    }
}

fn main() [use] {
    use CyclicRandom(7)
    use Relaying()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("cannot be bound shareable")
            && m.contains("local Random")
            && m.contains("use local Relaying")),
        "expected the local dep named as the blocker: {errs:?}"
    );
}

/// [use-local] [effect-handler-deps] The two dependency shapes the
/// owned-handle capture excludes, each named as a blocker: an actor-effect
/// dependency (an addr constructor parameter is the shape for that), and a
/// generic effect instance.
#[test]
fn fusion_pinning_dependencies_block_a_shareable_binding() {
    let errs = errors(
        "\
effect Beacon {
    fn shine() -> Int
}

handler ViaDrawer [Drawer] of Beacon {
    fn shine() -> Int {
        return waitfor got: Reply<Int> {
            draw(got)
        }
    }
}

effect Store<T> {
    fn keep(value: T) -> Int => value
}

handler MemStore of Store<Int> {
    fn keep(value: Int) -> Int => value {
        return value
    }
}

handler ViaStore [Store<Int>] of Beacon {
    fn shine() -> Int {
        return keep(3)
    }
}

fn main() [use, spawn] {
    let d = spawn Drawing() on pool(1)
    use d
    use ViaDrawer()
    use MemStore()
    use ViaStore()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("`ViaDrawer` cannot be bound shareable")
            && m.contains("actor effect")),
        "expected the actor-effect dep blocker: {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("`ViaStore` cannot be bound shareable")
            && m.contains("generic effect instance")),
        "expected the generic-instance dep blocker: {errs:?}"
    );
}

/// [use-local] [monitor-handler] The generic monitor (user decision
/// 2026-09-20): a stateful handler of a generic effect *instance* shares by
/// default like any other — `handler Keeping of Store<Int>` needs no
/// `local` anywhere.
#[test]
fn a_stateful_handler_of_a_generic_instance_shares() {
    let errs = errors(
        "\
effect Store<T> {
    fn keep(value: T) -> Int => value
}

handler Keeping of Store<Int> {
    total: Int = 0

    fn keep(value: Int) -> Int => value {
        total = total + value
        return total
    }
}

fn stash() [Store<Int>] -> Int {
    return keep(2)
}

fn main() [use] {
    use Keeping()
    let n = stash()
}
",
    );
    assert!(errs.is_empty(), "expected a clean program: {errs:?}");
}

/// [use-local] [effect-handler-deps] The v1 lexical cut: a shareable
/// construction captures a dependency handle off a binding *in this
/// function* — an effect that arrives through the enclosing signature has
/// no binding value to mint one from, and the error points at
/// spawn-inheritance as the lift.
#[test]
fn a_signature_supplied_dependency_cannot_be_captured() {
    let errs = errors(
        "\
effect Beacon {
    fn shine() -> Int
}

handler Relaying [Random] of Beacon {
    fn shine() -> Int {
        return next()
    }
}

fn wire() [Random, use] {
    use Relaying()
}

fn main() [use] {
    use CyclicRandom(7)
    wire()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("captures a handle for `Random`")
            && m.contains("enclosing signature")),
        "expected the lexical-capture refusal: {errs:?}"
    );
}

/// [use-local] [effect-local] A shareable-default dependency is captured as
/// an owned handle, which a `use local` binding cannot yield.
#[test]
fn a_captured_dependency_needs_a_shareable_binding() {
    let errs = errors(
        "\
effect Beacon {
    fn shine() -> Int
}

handler Relaying [Random] of Beacon {
    fn shine() -> Int {
        return next()
    }
}

fn main() [use] {
    use local CyclicRandom(7)
    use Relaying()
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("depends on a shareable `Random`")
            && m.contains("use local")),
        "expected the local dep binding refused: {errs:?}"
    );
}

/// [use-local] [actor-deadlock-cycle] Waits under a monitor's lock are
/// priced, not refused — option (b), user decision 2026-09-20: dep-bearing
/// shareable handlers get graph nodes, and a cycle through held locks is
/// reported before it runs, naming the locks.
#[test]
fn a_cycle_through_monitor_locks_is_reported() {
    let errs = errors(
        "\
effect Ping {
    fn ping() -> Int
}

effect Pong {
    fn pong() -> Int
}

handler BasePing of Ping {
    fn ping() -> Int { return 1 }
}

handler BasePong of Pong {
    fn pong() -> Int { return 2 }
}

handler Pinger [Pong] of Ping {
    a: Int = 0
    fn ping() -> Int {
        a = pong()
        return a
    }
}

handler Ponger [Ping] of Pong {
    b: Int = 0
    fn pong() -> Int {
        b = ping()
        return b
    }
}

fn main() [use] {
    use BasePing()
    use BasePong()
    use Pinger()
    use Ponger()
    let got = pong()
    let sink = got
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("Pinger's lock")
            && m.contains("Ponger's lock")
            && m.contains("wait for each other through a shared handler")),
        "expected the lock cycle reported by name: {errs:?}"
    );
}
