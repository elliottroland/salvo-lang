//! [iter-generator] Planning a `yield` function as a resumable pass.
//!
//! The acceptance test is the I3 prototype: the plan built for
//! `tests/fixtures/gnarly.sv` must be the machine that was hand-written for
//! that body and verified against a push-style oracle — the same eight states,
//! in the same order, with the same resume points. Everything
//! else here is one shape at a time (a bare loop, a `defer` in a loop body, a
//! nested pass, a `break`, a `return`) plus the constructs the planner
//! refuses rather than guesses at.
//!
//! [yield-fn-origin] Every source here is a producer as the language spells
//! one: an **origin** struct declaring `: Yield<self, T>` plus a
//! `yield fn next(origin) -> T`. So the origin is always field 0 of the plan,
//! and what used to be a parameter of the producer is a field read off it
//! (`u.limit`) — the planner sees an ordinary expression either way.

use std::path::{Path, PathBuf};

use salvo_core::generator::{plan_generator, GenError};
use salvo_syntax::ast::{FnDecl, Item};

/// Parse `src` and plan its `yield fn`. Selected by the `is_yield` flag rather
/// than by name: every producer's sugar is called `next`, and a source with a
/// nested hand-written pass declares a second function of that name.
fn plan_of(src: &str) -> Result<String, Vec<GenError>> {
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let decl = yield_fn(&module.items).expect("no `yield fn` in the source");
    plan_generator(decl).map(|plan| plan.render(src))
}

fn yield_fn(items: &[Item]) -> Option<&FnDecl> {
    items.iter().find_map(|item| match item {
        Item::Fn(f) if f.is_yield => Some(f),
        _ => None,
    })
}

fn plan(src: &str) -> String {
    match plan_of(src) {
        Ok(rendered) => rendered,
        Err(errors) => panic!("unexpected plan errors: {errors:?}"),
    }
}

fn errors(src: &str) -> Vec<String> {
    match plan_of(src) {
        Ok(rendered) => panic!("expected errors, got a plan:\n{rendered}"),
        Err(errors) => errors.into_iter().map(|e| e.message).collect(),
    }
}

/// A checked-in `.sv` fixture beside this test file.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

// ============================ the I3 acceptance test ============================

/// [iter-generator] `tests/fixtures/gnarly.sv` — the I3 prototype's body,
/// restated in the current language, whose hand-written machine was checked
/// against a push-style oracle on both backends. Everything resumable at once:
/// a fn-level `defer`, a `defer` inside a loop body reading a per-iteration
/// local, a nested pass alive across the outer body's suspensions, a
/// `continue`, two yields per outer iteration and a third after the loop.
///
/// The eight states below are the prototype machine's, in its order.
#[test]
fn the_gnarly_prototype_plans_to_its_eight_states() {
    let src = std::fs::read_to_string(fixture("gnarly.sv"))
        .expect("the planner's acceptance fixture is checked in");
    let rendered = plan(&src);
    let expected = "\
fields:
  0 g (param)
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
    if !`row < g.limit`:
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
    let src = r#"struct Naturals : Yield<self, Int> {
    from: Int
}

yield fn next(n: Naturals) -> Int {
    let i = copy(n.from)
    while true {
        yield copy(i)
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 n (param)
  1 i (local)
defers:
states:
  0:
    plain `let i = copy(n.from)`
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
    let src = r#"struct Pairs : Yield<self, Int> {
    limit: Int
}

yield fn next(p: Pairs) [Console] -> Int {
    let i = 0
    while i < p.limit {
        println("before")
        yield copy(i)
        println("after")
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 p (param)
  1 i (local)
defers:
states:
  0:
    plain `let i = 0`
    goto 1
  1:
    if !`i < p.limit`:
      goto 3
    plain `println(\"before\")`
    emit `copy(i)` resume 2
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
    let src = r#"struct Upto : Yield<self, Int> {
    limit: Int
}

yield fn next(u: Upto) [Console] -> Int {
    let i = 0
    while true {
        defer { println("turn") }
        if i >= u.limit {
            break
        }
        yield copy(i)
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 u (param)
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
    if `i >= u.limit`:
      discharge d0
      goto 3
    emit `copy(i)` resume 2
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
    let src = r#"struct Head : Yield<self, Int> {
    stop: Int
}

yield fn next(h: Head) [Console] -> Int {
    defer { println("outer") }
    let i = 0
    while true {
        defer { println("inner") }
        if i == h.stop {
            return
        }
        yield copy(i)
        i = i + 1
    }
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 h (param)
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
    if `i == h.stop`:
      discharge d1
      discharge d0
      finish
    emit `copy(i)` resume 2
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
    let src = r#"struct Doubling : Yield<self, Int> {
    items: List<Int>
}

yield fn next(d: Doubling) -> Int {
    for x in d.items {
        yield x * 2
    }
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 d (param)
  1 x (element)
  2 x__pass (pass)
defers:
states:
  0:
    open pass 2 = `d.items`
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
    let src = r#"struct Maybe : Yield<self, Int> {
    flag: Bool
}

yield fn next(m: Maybe) [Console] -> Int {
    if m.flag {
        yield 1
        yield 2
    }
    println("done")
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 m (param)
defers:
states:
  0:
    if `m.flag`:
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
    let src = r#"struct Capped : Yield<self, Int> {
    limit: Int
}

yield fn next(c: Capped) [Console] -> Int {
    let i = 0
    while i < c.limit {
        if i == 3 {
            break
        }
        i = i + 1
    }
    yield i
}
"#;
    assert_eq!(
        plan(&src),
        "\
fields:
  0 c (param)
  1 i (local)
defers:
states:
  0:
    plain `let i = 0`
    plain `while i < c.limit { if i == 3 { break } i = i + 1 }`
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
    let src = "struct Tagged : Yield<self, Int> {
    value: Int | Str
}

yield fn next(t: Tagged) -> Int {
    let v = t.value
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
    let errors = errors(src);
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
    let src = "struct Choice : Yield<self, Int> {
    flag: Bool
}

yield fn next(c: Choice) -> Int {
    let x = if c.flag {
        yield 1
    } else {
        yield 2
    }
}
";
    let errors = errors(src);
    assert!(
        errors.iter().any(|e| e.contains("value position")),
        "{errors:?}"
    );
}

/// Two locals of one name would be two fields of one name: rejected, with the
/// remedy named, rather than renamed behind the author's back.
#[test]
fn a_shadowing_local_is_refused() {
    let src = "struct Bumping : Yield<self, Int> {
    seed: Int
}

yield fn next(b: Bumping) -> Int {
    let v = copy(b.seed)
    while true {
        let v = v + 1
        yield v
    }
}
";
    let errors = errors(src);
    assert!(
        errors.iter().any(|e| e.contains("declared twice")),
        "{errors:?}"
    );
}

/// A local shadowing a *parameter* is the same collision.
#[test]
fn a_local_shadowing_a_parameter_is_refused() {
    let src = "struct Bumping : Yield<self, Int> {
    seed: Int
}

yield fn next(b: Bumping) -> Int {
    let b = 1
    yield b
}
";
    let errors = errors(src);
    assert!(
        errors.iter().any(|e| e.contains("declared twice")),
        "{errors:?}"
    );
}

/// A loop `else` runs only if the loop never ran; a suspending loop has
/// nowhere to record that yet, so it is reported rather than dropped.
#[test]
fn a_suspending_loop_with_an_else_is_refused() {
    let src = "struct Countdown : Yield<self, Int> {
    at: Int
}

yield fn next(c: Countdown) -> Int {
    while c.at > 0 {
        yield copy(c.at)
    } else {
        yield 0
    }
}
";
    let errors = errors(src);
    assert!(
        errors.iter().any(|e| e.contains("`else`")),
        "{errors:?}"
    );
}

/// A destructuring `let` in a suspending block would need one field per part,
/// with the checker's types to name them: reported for now.
#[test]
fn a_destructuring_let_is_refused() {
    let src = "struct Pairs : Yield<self, Int> {
    parts: (Int, Int)
}

yield fn next(p: Pairs) -> Int {
    let (a, b) = p.parts
    yield a
    yield b
}
";
    let errors = errors(src);
    assert!(
        errors.iter().any(|e| e.contains("destructuring `let`")),
        "{errors:?}"
    );
}
