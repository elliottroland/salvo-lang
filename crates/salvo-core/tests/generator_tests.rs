//! [iter-generator] Planning a `yield` function as a resumable pass.
//!
//! The acceptance test is the I3 prototype: the plan built for
//! `experiments/pull-iterators/gnarly.sv` must be the machine that was
//! hand-written in `gnarly.rs` and verified against an oracle — the same
//! eight states, in the same order, with the same resume points. Everything
//! else here is one shape at a time (a bare loop, a `defer` in a loop body, a
//! nested pass, a `break`, a `return`) plus the constructs the planner
//! refuses rather than guesses at.

use std::path::{Path, PathBuf};

use salvo_core::generator::{plan_generator, GenError};
use salvo_syntax::ast::{FnDecl, Item};

/// Parse `src` and plan the function named `name`.
fn plan_of(src: &str, name: &str) -> Result<String, Vec<GenError>> {
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let decl = find_fn(&module.items, name)
        .unwrap_or_else(|| panic!("no `fn {name}` in the source"));
    plan_generator(decl).map(|plan| plan.render(src))
}

fn find_fn<'a>(items: &'a [Item], name: &str) -> Option<&'a FnDecl> {
    items.iter().find_map(|item| match item {
        Item::Fn(f) if f.name.name == name => Some(f),
        _ => None,
    })
}

fn plan(src: &str, name: &str) -> String {
    match plan_of(src, name) {
        Ok(rendered) => rendered,
        Err(errors) => panic!("unexpected plan errors: {errors:?}"),
    }
}

fn errors(src: &str, name: &str) -> Vec<String> {
    match plan_of(src, name) {
        Ok(rendered) => panic!("expected errors, got a plan:\n{rendered}"),
        Err(errors) => errors.into_iter().map(|e| e.message).collect(),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/salvo-core has a workspace root")
        .to_path_buf()
}

// ============================ the I3 acceptance test ============================

/// [iter-generator] `experiments/pull-iterators/gnarly.sv`, whose hand-written
/// machine (`gnarly.rs`) was checked against a push-style oracle on both
/// backends. Everything resumable at once: a fn-level `defer`, a `defer`
/// inside a loop body reading a per-iteration local, a nested pass alive
/// across the outer body's suspensions, a `continue`, two yields per outer
/// iteration and a third after the loop.
///
/// The eight states below are `gnarly.rs`'s, in its order — the numbering in
/// its comment block is this plan's.
#[test]
fn the_gnarly_prototype_plans_to_its_eight_states() {
    let src = std::fs::read_to_string(repo_root().join("experiments/pull-iterators/gnarly.sv"))
        .expect("the I3 prototype source is checked in");
    let rendered = plan(&src, "walk");
    let expected = "\
fields:
  0 limit (param)
  1 row (local)
  2 r (local)
  3 col (element)
  4 col__pass (pass)
defers:
  0 defer { println(\"close\") }
  1 defer { println(\"row ${r} end\") }
states:
  0:
    plain `println(\"open\")`
    register d0
    plain `let row = 0`
    goto 1
  1:
    if !`row < limit`:
      goto 5
    plain `let r = copy(row)`
    register d1
    open pass 4 = `countdown(2)`
    goto 2
  2:
    drive pass 4 -> field 3
    finished:
      close pass 4
      goto 3
    if `col == 1`:
      goto 2
    emit `r * 10 + col` resume 2
  3:
    emit `r * 100` resume 4
  4:
    plain `row = row + 1`
    discharge d1
    goto 1
  5:
    emit `999` resume 6
  6:
    discharge d0
    finish
  7:
    finish
close:
    close pass 4
    discharge d1
    discharge d0
";
    assert_eq!(rendered, expected, "\n{rendered}");
}

// ================================ one shape at a time ================================

/// The simplest producer there is: three states, and the resume point after
/// the `yield` is the loop head, not a state of its own — the peephole that
/// keeps the machine readable.
#[test]
fn a_bare_loop_resumes_at_its_own_head() {
    let src = r#"fn naturals() -> Once Iter<Int> {
    let i = 0
    while true {
        yield copy(i)
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src, "naturals"),
        "\
fields:
  0 i (local)
defers:
states:
  0:
    plain `let i = 0`
    goto 1
  1:
    if !`true`:
      goto 3
    emit `copy(i)` resume 2
  2:
    plain `i = i + 1`
    goto 1
  3:
    finish
close:
"
    );
}

/// A `yield` in the middle of a loop body: the state *after* it is "the
/// statements after it", and the back edge is a transition.
#[test]
fn a_yield_in_the_middle_of_a_body_splits_it() {
    let src = r#"fn pairs(n: Int) [Console] -> Once Iter<Int> {
    let i = 0
    while i < n {
        println("before")
        yield i
        println("after")
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src, "pairs"),
        "\
fields:
  0 n (param)
  1 i (local)
defers:
states:
  0:
    plain `let i = 0`
    goto 1
  1:
    if !`i < n`:
      goto 3
    plain `println(\"before\")`
    emit `i` resume 2
  2:
    plain `println(\"after\")`
    plain `i = i + 1`
    goto 1
  3:
    finish
close:
"
    );
}

/// A `break` out of a suspending loop runs the loop body's `defer`s on the way
/// out — the same steps the end of the iteration would have run, which is what
/// [defer] means.
#[test]
fn a_break_discharges_the_defers_it_leaves() {
    let src = r#"fn upto(n: Int) [Console] -> Once Iter<Int> {
    let i = 0
    while true {
        defer { println("turn") }
        if i >= n {
            break
        }
        yield i
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src, "upto"),
        "\
fields:
  0 n (param)
  1 i (local)
defers:
  0 defer { println(\"turn\") }
states:
  0:
    plain `let i = 0`
    goto 1
  1:
    if !`true`:
      goto 3
    register d0
    if `i >= n`:
      discharge d0
      goto 3
    emit `i` resume 2
  2:
    plain `i = i + 1`
    discharge d0
    goto 1
  3:
    finish
close:
    discharge d0
"
    );
}

/// A bare `return` ends the body early: every pending deferred block runs,
/// latest first, and the machine reports `Finished` from then on.
#[test]
fn a_return_releases_everything_and_finishes() {
    let src = r#"fn head(n: Int) [Console] -> Once Iter<Int> {
    defer { println("outer") }
    let i = 0
    while true {
        defer { println("inner") }
        if i == n {
            return
        }
        yield i
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src, "head"),
        "\
fields:
  0 n (param)
  1 i (local)
defers:
  0 defer { println(\"outer\") }
  1 defer { println(\"inner\") }
states:
  0:
    register d0
    plain `let i = 0`
    goto 1
  1:
    if !`true`:
      goto 3
    register d1
    if `i == n`:
      discharge d1
      discharge d0
      finish
    emit `i` resume 2
  2:
    plain `i = i + 1`
    discharge d1
    goto 1
  3:
    discharge d0
    finish
  4:
    finish
close:
    discharge d1
    discharge d0
"
    );
}

/// A `for` inside a producer: the pass it drives becomes a field, because it
/// has to survive the body's suspensions.
#[test]
fn a_nested_for_becomes_a_pass_field() {
    let src = r#"fn doubled(xs: Once Iter<Int>) -> Once Iter<Int> {
    for x in xs {
        yield x * 2
    }
}
"#;
    assert_eq!(
        plan(&src, "doubled"),
        "\
fields:
  0 xs (param)
  1 x (element)
  2 x__pass (pass)
defers:
states:
  0:
    open pass 2 = `xs`
    goto 1
  1:
    drive pass 2 -> field 1
    finished:
      close pass 2
      goto 2
    emit `x * 2` resume 1
  2:
    finish
close:
    close pass 2
"
    );
}

/// An `if` whose arm suspends cannot be inlined into the state it is written
/// in: the arm gets states, and the fall-through joins them afterwards.
#[test]
fn a_suspending_if_arm_gets_its_own_states() {
    let src = r#"fn maybe(flag: Bool) [Console] -> Once Iter<Int> {
    if flag {
        yield 1
        yield 2
    }
    println("done")
}
"#;
    assert_eq!(
        plan(&src, "maybe"),
        "\
fields:
  0 flag (param)
defers:
states:
  0:
    if `flag`:
      goto 1
    goto 3
  1:
    emit `1` resume 2
  2:
    emit `2` resume 3
  3:
    plain `println(\"done\")`
    finish
  4:
    finish
close:
"
    );
}

/// A `while` with no `yield` in it is one statement to the emitter, brace to
/// brace, and keeps its own `break`: only control flow that has to cross a
/// suspension is flattened.
#[test]
fn a_yield_free_loop_stays_one_plain_step() {
    let src = r#"fn once_only(n: Int) [Console] -> Once Iter<Int> {
    let i = 0
    while i < n {
        if i == 3 {
            break
        }
        i = i + 1
    }
    yield i
}
"#;
    assert_eq!(
        plan(&src, "once_only"),
        "\
fields:
  0 n (param)
  1 i (local)
defers:
states:
  0:
    plain `let i = 0`
    plain `while i < n { if i == 3 { break } i = i + 1 }`
    emit `i` resume 1
  1:
    finish
close:
"
    );
}

// ================================ refusals ================================

/// [backend-never-wrong] A `when` containing a `yield`: the arm/binding
/// interaction is not something to guess at, so it is reported. `if` works.
#[test]
fn a_when_containing_a_yield_is_refused() {
    let src = "fn f(v: Int | Str) -> Once Iter<Int> {
    when v {
        is Int {
            yield 1
        }
        is Str {
            yield 2
        }
    }
}
";
    let errors = errors(src, "f");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("a `when` containing a `yield`"),
        "{errors:?}"
    );
}

/// A `yield` cannot be an expression's value: it is a statement, and a value
/// position would need the machine to resume *into* an expression.
#[test]
fn a_yield_in_a_value_position_is_refused() {
    let src = "fn f(flag: Bool) -> Once Iter<Int> {
    let x = if flag {
        yield 1
    } else {
        yield 2
    }
}
";
    let errors = errors(src, "f");
    assert!(
        errors.iter().any(|e| e.contains("value position")),
        "{errors:?}"
    );
}

/// Two locals of one name would be two fields of one name: rejected, with the
/// remedy named, rather than renamed behind the author's back.
#[test]
fn a_shadowing_local_is_refused() {
    let src = "fn f(n: Int) -> Once Iter<Int> {
    let v = n
    while true {
        let v = v + 1
        yield v
    }
}
";
    let errors = errors(src, "f");
    assert!(
        errors.iter().any(|e| e.contains("declared twice")),
        "{errors:?}"
    );
}

/// A local shadowing a *parameter* is the same collision.
#[test]
fn a_local_shadowing_a_parameter_is_refused() {
    let src = "fn f(n: Int) -> Once Iter<Int> {
    let n = 1
    yield n
}
";
    let errors = errors(src, "f");
    assert!(
        errors.iter().any(|e| e.contains("declared twice")),
        "{errors:?}"
    );
}

/// A loop `else` runs only if the loop never ran; a suspending loop has
/// nowhere to record that yet, so it is reported rather than dropped.
#[test]
fn a_suspending_loop_with_an_else_is_refused() {
    let src = "fn f(n: Int) -> Once Iter<Int> {
    while n > 0 {
        yield n
    } else {
        yield 0
    }
}
";
    let errors = errors(src, "f");
    assert!(
        errors.iter().any(|e| e.contains("`else`")),
        "{errors:?}"
    );
}

/// A destructuring `let` in a suspending block would need one field per part,
/// with the checker's types to name them: reported for now.
#[test]
fn a_destructuring_let_is_refused() {
    let src = "fn f(p: (Int, Int)) -> Once Iter<Int> {
    let (a, b) = p
    yield a
    yield b
}
";
    let errors = errors(src, "f");
    assert!(
        errors.iter().any(|e| e.contains("destructuring `let`")),
        "{errors:?}"
    );
}
