//! [effect-handler-multi] Handlers of **several effects** — the checker's half
//! (T-4(a), user decision 2026-09-17): one handler, one piece of state, one
//! face per effect it implements.
//!
//! What is checked here is the accounting the feature needs: every member of
//! every face implemented, one signature where one method implements two faces,
//! the faces all of one kind and each named once, a `spawn` answering one addr
//! per face, and a `use` binding all of them. The emission of all this lives in
//! the backends' tests, where it is compiled and run.
//!
//! The rest of these tests are test data, `tests/check/multi_effect/` (TESTS.md T1).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations the sources need, loaded as
/// a std file since only std may write `intrinsic`.
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
export intrinsic type Pool
export intrinsic fn pool(size: Int) [spawn] -> Pool => size
export provenance qualifier Dedicated of Pool
export intrinsic fn thread() [spawn] -> Dedicated Pool
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
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| (d.is_error(), d.message.clone()))
        .collect()
}

/// And with several faces the value *is* a tuple, so reading it as a bare addr
/// is refused — the two shapes are distinguished by the number of faces alone.
///
/// Kept over its prelude, not moved to `check/multi_effect/`: checked exactly,
/// the one mistake reports three errors (no handler, arg count, arg type).
#[test]
fn several_faces_answer_a_tuple_rather_than_an_addr() {
    let errs = errors(
        r#"
handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 4 }

    state {
        now: Int = 0
    }

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
