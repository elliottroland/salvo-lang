//! [actor-spawn-expr] [actor-replyto] [actor-waitfor] [actor-use-addr] The
//! asynchronous surface's *checker* rules: what a `spawn` needs to be legal,
//! what a `replyto` may target, where a `waitfor` may stand, and how an `Addr`
//! is called. The forms' syntax is tested in `salvo-syntax`; the types they
//! produce and consume are `std.core.actor`'s ([actor-types]).
//!
//! The shape every test here rests on: a process is a handler bound
//! asynchronously, so almost every rule is one `use` already had, moved to
//! the spawn site.
//!
//! The diagnostics are test data, `tests/check/actors/` (TESTS.md T1). What
//! stays here is what the data cannot hold yet: cases whose program reports
//! a diagnostic that looks wrong, kept as they were until it is fixed.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// A stand-in for `std.core.actor` and the little of std these tests need.
/// `intrinsic` is std-only [platform-intrinsic-std-only], so it is loaded as a std
/// file rather than pasted into the source under test.
const STD_PRELUDE: &str = concat!(
    "\
export intrinsic type Int
export intrinsic type Str
export intrinsic type Bool
export intrinsic type List<T> canbe Mut
export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value
export intrinsic type Addr<E>
export struct Mailbox { capacity: Int }
export linear intrinsic type Reply<T>
export intrinsic fn send<T>(reply: Reply<T>, value: T) [] -> None => !reply, !value
export intrinsic fn array_of<T>(...elems: T[]) [] -> T[]
export intrinsic type Pool
export intrinsic fn pool(size: Int) [spawn] -> Pool => size
export provenance qualifier Dedicated of Pool
export intrinsic fn thread() [spawn] -> Dedicated Pool
export struct Fault { reason: Str }
export actor effect Faults {
    send fn faulted(fault: Fault) => !fault
}
export intrinsic fn pool(size: Int, sink: eff[Faults]) [spawn] -> Pool => size, sink
export struct Exit { reason: Str }
export intrinsic fn watch<E>(target: eff[E], on_exit: Reply<Exit>) [spawn] -> None => target, !on_exit
",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
);

/// The effects and handlers the cases share: a `Counter` protocol with a
/// request/response pair, a `Log` dependency, and handlers for both.
const PRELUDE: &str = r#"
actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
    mailbox { capacity: 16 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
        note("bumped")
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
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
    diagnostics_with_ids(src).into_iter().map(|(error, _, message)| (error, message)).collect()
}

/// [diag-id] Every diagnostic with its id: `(is an error, id, message)`.
fn diagnostics_with_ids(src: &str) -> Vec<(bool, Option<&'static str>, String)> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut msgs: Vec<(bool, Option<&'static str>, String)> = resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| (d.is_error(), d.id, d.message.clone()))
        .collect();
    msgs.extend(
        salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked)
            .into_iter()
            .map(|d| (d.is_error(), d.id, d.message)),
    );
    msgs
}

// ===== Kept: the data would have to annotate a diagnostic that looks wrong =====

/// The target must exist, and must be a `send fn`: an answer arrives as a
/// message.
// Kept: over the real std the `ready()` mint also reports "wants 0 capture(s), found 0".
#[test]
fn replyto_needs_a_send_member_of_its_own_handler() {
    let unknown = errors(
        "\
handler Asking() [Counter] of Log {
    mailbox { capacity: 1 }

    send fn note(what: Str) {
        total(replyto arrived())
    }
}
",
    );
    assert!(
        unknown
            .iter()
            .any(|m| m.contains("has no member `arrived`")),
        "expected the unknown-member error: {unknown:?}"
    );

    let not_send = errors(
        "\
actor effect Ask {
    send fn go(out: Reply<Int>) => !out
    fn ready() -> Bool
}

handler Asker() [Counter] of Ask {
    mailbox { capacity: 1 }

    send fn go(out: Reply<Int>) {
        out.send(1)
    }

    fn ready() -> Bool {
        total(replyto ready())
        return true
    }
}
",
    );
    assert!(
        not_send
            .iter()
            .any(|m| m.contains("is not a `send fn`") && m.contains("answer arrives as a message")),
        "expected the non-send target refusal: {not_send:?}"
    );
}

/// And the *old* spelling is a plain parse error naming the new one — no
/// transitional accept (the `canbe` precedent: nothing outside this repository
/// writes Salvo).
// Kept: checked, the recovered `self.note(what)` cascades into two checker errors.
#[test]
fn the_receiver_spelling_of_a_self_send_is_a_parse_error() {
    let source = "\
handler Working() of Log {
    send fn note(what: Str) {
        self.note(what)
    }
}
";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics.iter().any(|d| d.is_error()
            && d.message.contains("is not the self-send form")
            && d.message.contains("`k@self(…)`")),
        "expected the old form to be refused: {diagnostics:?}"
    );
}

/// [effect-any] The shared shape: a router `of any Counter` in front of two
/// members. Forwarding to a member of the effect it implements is not
/// self-dispatch (the receiver is an addr), and the deadlock graph gives the
/// router a node of its own, so `Counter → Counter` is not reported.
const ROUTER: &str = r#"
handler Spraying(a: eff[Counter], b: eff[Counter]) of any Counter {
    mailbox { capacity: 16 }
    state {
        flip: Bool = false
    }
    send fn bump(n: Int) {
        flip = !flip
        if flip { a.bump(n) } else { b.bump(n) }
    }
    send fn total(out: Reply<Int>) {
        a.total(out)
    }
}

fn tally() [Counter] -> Int {
    bump(1)
    bump(2)
    return waitfor out: Reply<Int> { total(out) }
}

fn poke() [any Counter] -> None {
    bump(3)
}
"#;

/// [effect-any] A fn type carries no strength — `any` on one is refused
/// with the reason.
// Kept: the fn-type refusal is reported twice for one `any` (three times with an empty body).
#[test]
fn any_is_refused_on_fn_types() {
    let errs = errors(&format!(
        "{ROUTER}
fn each(f: () [any Counter] -> None) [any Counter] -> None {{
    f()
}}
"
    ));
    assert!(
        errs.iter().any(|m| m.contains("carry no strength") && m.contains("declare `[any Counter]`")),
        "{errs:?}"
    );
}
