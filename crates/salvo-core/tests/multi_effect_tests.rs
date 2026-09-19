//! [effect-handler-multi] Handlers of **several effects** — the checker's half
//! (T-4(a), user decision 2026-09-17): one handler, one piece of state, one
//! face per effect it implements.
//!
//! What is checked here is the accounting the feature needs: every member of
//! every face implemented, one signature where one method implements two faces,
//! the faces all of one kind and each named once, a `spawn` answering one addr
//! per face, and a `use` binding all of them. The emission of all this lives in
//! the backends' tests, where it is compiled and run.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations the sources need, loaded as
/// a std file since only std may write `intrinsic`.
const STD_PRELUDE: &str = "\
export intrinsic type Int
export intrinsic type Str
export intrinsic type Bool
export intrinsic type List<T> canbe Mut
export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value
export intrinsic type Addr<E>
export struct Mailbox { capacity: Int }
export linear intrinsic type Reply<T>
export intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value
export intrinsic type Pool
export intrinsic fn pool(size: Int) [spawn] -> Pool => size
export provenance qualifier Dedicated of Pool
export intrinsic fn thread() [spawn] -> Dedicated Pool
";

/// The two-faced pair T-4 is named for: a public protocol and an admin one,
/// both `actor effect`s, plus a synchronous pair beside them.
const PRELUDE: &str = r#"
actor effect Timer {
    send fn after(millis: Int, out: Reply<Str>) => !out
}

actor effect TimerCtl {
    send fn advance(millis: Int) => !millis
    send fn pending(out: Reply<Int>) => !out
}

effect Tally {
    fn bump(n: Int) -> None => !n
}

effect Stats {
    fn total() -> Int
}
"#;

fn errors(src: &str) -> Vec<String> {
    diagnostics(src)
        .into_iter()
        .filter(|(error, _)| *error)
        .map(|(_, message)| message)
        .collect()
}

/// Every diagnostic the program produces, as `(is_error, message)`.
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
        .map(|d| (d.is_error(), d.message.clone()))
        .collect()
}

/// The shape the step exists for: one actor, one mailbox, two typed faces, and
/// a `spawn` that answers one addr per face — so a holder of the public face
/// cannot name the admin one.
#[test]
fn a_handler_may_implement_several_actor_effects() {
    let errs = errors(
        r#"
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }

    now: Int = 0

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired at ${now}")
    }

    send fn advance(millis: Int) {
        now = now + millis
    }

    send fn pending(out: Reply<Int>) {
        out.send(now)
    }
}

fn main() [spawn] -> None {
    let (timer, ctl) = spawn ManualTime() on pool(1)
    let n = waitfor c: Reply<Int> { ctl.pending(c) }
    let s = waitfor f: Reply<Str> {
        timer.after(1, f)
        ctl.advance(1)
    }
    discard(n)
    discard(s)
}
"#,
    );
    assert!(errs.is_empty(), "the two-faced actor is legal: {errs:?}");
}

/// The addrs are typed by their own face, which is where least authority comes
/// from: the public one cannot reach the admin protocol's members.
#[test]
fn an_addr_of_one_face_cannot_name_the_others_members() {
    let errs = errors(
        r#"
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }

    now: Int = 0

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    send fn advance(millis: Int) {
        now = now + millis
    }

    send fn pending(out: Reply<Int>) {
        out.send(now)
    }
}

fn main() [spawn] -> None {
    let (timer, ctl) = spawn ManualTime() on pool(1)
    timer.advance(1)
}
"#,
    );
    assert!(
        errs.iter().any(|m| m.contains("has no member named `advance`")
            || m.contains("no member named `advance`")),
        "the public face must not reach the admin member: {errs:?}"
    );
}

/// A single face still answers a bare `Addr<E>`: the tuple is what *several*
/// faces produce, so nothing about the one-face spawn changes. Asserted by
/// *using* the value as an addr, since a tuple has no members to send to.
#[test]
fn one_face_still_answers_a_bare_addr() {
    let errs = errors(
        r#"
handler Ticking() of Timer {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }
}

fn main() [spawn] -> None {
    let timer = spawn Ticking() on pool(1)
    let s = waitfor f: Reply<Str> { timer.after(1, f) }
    discard(s)
}
"#,
    );
    assert!(
        errs.is_empty(),
        "a one-face spawn answers an addr, not a tuple: {errs:?}"
    );
}

/// And with several faces the value *is* a tuple, so reading it as a bare addr
/// is refused — the two shapes are distinguished by the number of faces alone.
#[test]
fn several_faces_answer_a_tuple_rather_than_an_addr() {
    let errs = errors(
        r#"
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 4 }

    now: Int = 0

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    send fn advance(millis: Int) {
        now = now + millis
    }

    send fn pending(out: Reply<Int>) {
        out.send(now)
    }
}

fn main() [spawn] -> None {
    let timer = spawn ManualTime() on pool(1)
    timer.advance(1)
}
"#,
    );
    assert!(
        !errs.is_empty(),
        "a two-face spawn is a tuple, so a bare send through it must fail: {errs:?}"
    );
}

/// Conformance: a face whose member nobody implements is an error *here*,
/// naming the member — it used to be the target compiler's missing-trait-method
/// complaint, which put a Salvo mistake in a foreign diagnostic.
#[test]
fn every_member_of_every_face_must_be_implemented() {
    let errs = errors(
        r#"
handler Half() of Timer, TimerCtl {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    send fn advance(millis: Int) {}
}
"#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("does not implement `TimerCtl.pending(Reply<Int>)`")),
        "the unimplemented member must be named: {errs:?}"
    );
}

/// One method may implement a same-named member of two faces — when the
/// signatures are the same, which is the only case in which "one method
/// implements both" means anything.
#[test]
fn one_method_may_implement_a_same_named_member_of_two_faces() {
    let errs = errors(
        r#"
effect Health {
    fn status() -> Str
}

effect Admin {
    fn status() -> Str
}

handler Serving() of Health, Admin {
    fn status() -> Str {
        return "ok"
    }
}
"#,
    );
    assert!(errs.is_empty(), "identical signatures are one member: {errs:?}");
}

/// …and is refused where overloading cannot tell them apart: same parameters,
/// different return type, so no method can answer for both (T-4(a)'s
/// refinement, and the one case the decision names as rejected).
#[test]
fn same_named_members_that_overloading_cannot_distinguish_are_refused() {
    let errs = errors(
        r#"
effect Health {
    fn status() -> Str
}

effect Admin {
    fn status() -> Int
}

handler Serving() of Health, Admin {
    fn status() -> Str {
        return "ok"
    }
}
"#,
    );
    assert!(
        errs.iter().any(|m| m.contains("whose signatures differ in what overloading cannot see")),
        "expected the indistinguishable-signature refusal: {errs:?}"
    );
}

/// Different *parameters* are what overloading distinguishes, so a handler may
/// implement both with two members.
#[test]
fn same_named_members_with_different_parameters_are_two_members() {
    let errs = errors(
        r#"
effect Ping {
    fn ping(n: Int) -> Str => n
}

effect Pong {
    fn ping(m: Str) -> Str => !m
}

handler Both() of Ping, Pong {
    fn ping(n: Int) -> Str {
        return "int"
    }

    fn ping(m: Str) -> Str {
        return m
    }
}
"#,
    );
    assert!(
        errs.is_empty(),
        "overloading distinguishes them, so both are implemented: {errs:?}"
    );
}

/// The faces must be of one **kind**: a handler is bound one way or the other,
/// and a handler mixing an `actor effect` with a plain one is refused — the
/// mixed handler [mixed-handler] is a different construct, classified by
/// shape, with every face plain.
#[test]
fn the_faces_must_all_be_of_one_kind() {
    let errs = errors(
        r#"
handler Mixed() of Timer, Stats {
    mailbox { capacity: 4 }

    send fn after(millis: Int, out: Reply<Str>) {
        out.send("fired")
    }

    fn total() -> Int {
        return 0
    }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("implements effects of two kinds")),
        "expected the mixed-kind refusal: {errs:?}"
    );
}

/// Naming a face twice says nothing new and would make a `spawn` answer the
/// same addr twice.
#[test]
fn a_face_may_not_be_named_twice() {
    let errs = errors(
        r#"
handler Twice() of Stats, Stats {
    fn total() -> Int {
        return 0
    }
}
"#,
    );
    assert!(
        errs.iter().any(|m| m.contains("already implements `Stats`")),
        "expected the duplicate-face refusal: {errs:?}"
    );
}

/// A `use` binds **every** face, which is what makes the synchronous
/// public-face/admin-face pair writable: one instance, two effect lists that
/// reach it.
#[test]
fn a_use_binds_every_face() {
    let errs = errors(
        r#"
handler Counting() of Tally, Stats {
    sum: Int = 0

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

fn work() [Tally, Stats] -> Int {
    bump(2)
    return total()
}

fn main() [use] -> None {
    use Counting()
    discard(work())
}
"#,
    );
    assert!(errs.is_empty(), "both faces are bound by one `use`: {errs:?}");
}

/// A multi-face handler **constructed in a spawn's `use` clause** is refused:
/// the child owns what a clause builds, and one instance cannot be two of its
/// dependencies. Two addrs of the same actor are the shape that works.
#[test]
fn a_multi_face_construction_in_a_spawn_clause_is_refused() {
    let errs = errors(
        r#"
handler Counting() of Tally, Stats {
    sum: Int = 0

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

actor effect Worker {
    send fn go(n: Int) => n
}

handler Working() [Tally] of Worker {
    mailbox { capacity: 4 }

    send fn go(n: Int) {
        bump(n)
    }
}

fn main() [spawn] -> None {
    let w = spawn Working() use Counting() on pool(1)
    discard(w)
}
"#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("implements several effects") && m.contains("spawn's `use` clause")),
        "expected the clause-construction refusal: {errs:?}"
    );
}
