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

/// The restriction, stated once at the declaration: a handler with
/// dependencies cannot be shared as a monitor — its members could perform
/// effects and wait under the lock, which is every lock pathology at once.
/// The diagnostic names the two ways out.
#[test]
fn a_monitor_spawn_refuses_a_handler_with_dependencies() {
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
        errs.iter().any(|m| m.contains("declares dependencies")
            && m.contains("no effects, no waits")
            && m.contains("Keep it `use`-bound")),
        "expected the dependency refusal: {errs:?}"
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
/// refused by name at the spawn. `use`-binding the same handler stays legal:
/// the classification governs sharing, not existence.
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
    use Derived(n -> n + 2)
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
        !errs.iter().any(|m| m.contains("use")),
        "the `use` binding of the same handler must stay legal: {errs:?}"
    );
}
