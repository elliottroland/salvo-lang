//! [flow-use-after-move] The IR's use-after-move pass (`ir::flow`, PIPELINE.md
//! R8), the only use-after-move diagnostic since piece 3a and the only
//! partial-move one since piece 3b [fate-partial-move]: over a corpus of
//! small programs, the checker reports nothing (a consumed read, or a read of
//! a place moved out, is never its error), the gate (`ir::analyze`) reports
//! what the pass finds over the IR built from the checker's tables, and each
//! case's reads are pinned.
//!
//! Until piece 3a this was a differential: the checker's own "consumed
//! (moved) by …" diagnostic, switched back on for the test, had to report
//! the same reads (by start offset, its same-call variant included). It
//! agreed on the final corpus — every fn-value, `once`-call and
//! projection-root case included — and was removed with the diagnostic; the
//! pinned expectations are those reads. The partial-move cases (`part_…`,
//! piece 3b) were compared the same way with the checker's two
//! `check_moved_place` messages before they were removed: same reads, same
//! messages, one difference (`part_same_call`, noted there). The checker
//! still owns the consumption marking (`narrowed = Never`) and the
//! `moved_places` record its linear and poison machinery read, so a case
//! whose expectation changes must say whether the pass's rule changed
//! (ROADMAP 0o's unsound join/loop rules, say) or the checker's marking did.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use salvo_core::{Program, SourceSet};

/// Shared by every case (each case is its own module).
const HELPERS: &str = "struct Box {
    s: Str
}

fn eat(s: Str) [] -> None => !s {
}

fn eat2(a: Str, b: Str) [] -> None => !a, !b {
}

fn look(s: Str) [] -> Int {
    return 0
}

fn mk() [] -> Str {
    return \"a\"
}

fn take<T>(v: T) [] -> None => !v {
}

fn peek<T>(v: T) [] -> Int {
    return 0
}

fn keep(g: () -> Int) [] -> None => !g {
}

fn call(g: () -> Int) [] -> Int {
    return g()
}

fn run_once(g: once () -> Int) [] -> Int {
    return g()
}

fn eatl(xs: Mut List<Str>) [] -> None => !xs {
}

struct Bag {
    items: Mut List<Str>
}

struct FnBox {
    run: () -> Int
}

struct Rec canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Nest {
    a: Rec,
    other: Mut List<Str>
}

struct Holder {
    r: Rec
}

struct Cnt {
    n: Int,
    tags: Mut List<Str>
}

struct OptRec {
    inner: Rec?
}

fn newrec() [] -> Rec {
    return Rec { name: \"a\", tags: Mut List<Str>() }
}

fn takep(p: Rec) [] -> None => !p {
}

fn relist(xs: Mut List<Str>) [] -> Mut List<Str> => !xs {
    return xs
}

fn eatl2(a: Mut List<Str>, b: Mut List<Str>) [] -> None => !a, !b {
}

";

/// `(name, source)`: each a use-after-move shape, or a near miss that must
/// stay clean on both sides.
const CASES: &[(&str, &str)] = &[
    // A consuming call, then a read.
    ("straight", "fn f() [] -> Int {\n    let s = mk()\n    eat(s)\n    return look(s)\n}\n"),
    // Moved on one path of an `if`: unusable after it.
    ("one_arm", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    if c {\n        eat(s)\n    }\n    return look(s)\n}\n"),
    ("both_arms", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    if c {\n        eat(s)\n    } else {\n        eat(s)\n    }\n    return look(s)\n}\n"),
    // An arm that leaves takes its move with it.
    ("arm_exits", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    if c {\n        eat(s)\n        return 0\n    }\n    return look(s)\n}\n"),
    ("when_arm", "fn f(v: Int | Bool) [] -> Int {\n    let s = mk()\n    when v {\n        is Int {\n            eat(s)\n        }\n        is Bool {\n        }\n    }\n    return look(s)\n}\n"),
    // A move in a loop: the next iteration's move is a use, and so is the
    // read after the loop.
    ("loop_then_after", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    while c {\n        eat(s)\n    }\n    return look(s)\n}\n"),
    ("loop_next_iteration", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    let n = 0\n    while c {\n        n = n + look(s)\n        eat(s)\n    }\n    return n\n}\n"),
    ("reassign_revives", "fn f() [] -> Int {\n    let s = mk()\n    eat(s)\n    s = mk()\n    return look(s)\n}\n"),
    ("loop_reassign_revives", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    let n = 0\n    while c {\n        n = n + look(s)\n        eat(s)\n        s = mk()\n    }\n    return n + look(s)\n}\n"),
    ("return_in_arm", "fn f(c: Bool) [] -> Str {\n    let s = mk()\n    if c {\n        return s\n    }\n    let _n = look(s)\n    return s\n}\n"),
    // The moves that are not calls.
    ("struct_store", "fn f() [] -> Int {\n    let s = mk()\n    let b = Box { s: s }\n    return look(s) + look(b.s)\n}\n"),
    ("spread", "fn f() [] -> Int {\n    let b = Box { s: mk() }\n    let b2 = Box { ...b }\n    return look(b.s) + look(b2.s)\n}\n"),
    ("use_arg", "effect Named {\n    fn name() -> Str\n}\n\nhandler Naming(who: Str) of Named {\n    fn name() -> Str {\n        return copy(who)\n    }\n}\n\nfn f() [use] -> Int {\n    let s = mk()\n    use Naming(s)\n    return look(s)\n}\n"),
    ("break_value", "fn f(c: Bool, d: Bool) [] -> Int {\n    let s = mk()\n    let _r = while c {\n        if d {\n            break s\n        }\n    }\n    return look(s)\n}\n"),
    ("lambda_mutates", "fn f() [] -> Int {\n    let xs = Mut List<Str>()\n    let g = () -> { add(xs, \"a\") }\n    g()\n    return size(xs)\n}\n"),
    ("lambda_mutates_int", "fn f() [] -> Int {\n    let n = 0\n    let g = () -> { n = n + 1 }\n    g()\n    return n\n}\n"),
    // Copy scalars never move, a generic `T` bound to one included.
    ("copy_scalar", "fn f() [] -> Int {\n    let n = 1\n    take(n)\n    take(n)\n    return n\n}\n"),
    ("generic_param", "fn g<T>(v: T) [] -> None => !v {\n    take(v)\n    take(v)\n}\n"),
    // A narrowed union's move is judged by its declared type.
    ("narrowed_scalar", "fn f(v: Int | Bool) [] -> None => !v {\n    if v is Int {\n        take(v)\n    }\n    take(v)\n}\n"),
    ("narrowed_consume", "fn f(v: Str?) [] -> Int => !v {\n    if v is Str {\n        eat(v)\n    }\n    if v is Str {\n        return look(v)\n    }\n    return 0\n}\n"),
    ("narrowed_after_guard", "fn f(v: Str?) [] -> Int => !v {\n    if v is None {\n        return 0\n    }\n    eat(v)\n    return look(v)\n}\n"),
    // A narrowing that persists from a guard is the variable's: a move
    // through it reaches every later narrowing of the variable.
    ("narrowed_kept_in_arm", "fn f(v: Str?, c: Bool) [] -> Int => !v {\n    if v is None {\n        return 0\n    }\n    eat(v)\n    if c {\n        return look(v)\n    }\n    return 0\n}\n"),
    ("narrowed_moved_in_arm", "fn f(v: Str?, c: Bool) [] -> Int => !v {\n    if v is None {\n        return 0\n    }\n    if c {\n        eat(v)\n    }\n    return look(v)\n}\n"),
    // The same-call rule: the later argument is the use.
    ("same_call", "fn f() [] -> None {\n    let s = mk()\n    eat2(s, s)\n}\n"),
    ("nested_lambda", "fn f() [] -> Int {\n    let g = () -> {\n        let t = mk()\n        eat(t)\n        return look(t)\n    }\n    return g()\n}\n"),
    ("lambda_reads_moved", "fn f() [] -> Int {\n    let s = mk()\n    eat(s)\n    let g = () -> look(s)\n    return g()\n}\n"),
    ("for_each", "fn f() [] -> Int {\n    let s = mk()\n    let xs = List(\"a\", \"b\")\n    for x in xs {\n        eat(copy(x))\n        eat(s)\n    }\n    return look(s)\n}\n"),
    ("interpolate", "fn f() [] -> Str {\n    let s = mk()\n    eat(s)\n    return \"${s}!\"\n}\n"),
    // The checker's rules where they are not what they seem (findings,
    // PIPELINE.md §8): mirrored, not fixed.
    // `continue` carries nothing to the next iteration: accepted, though
    // the second iteration reads a moved `s`.
    ("continue_skips", "fn f(c: Bool, d: Bool) [] -> Int {\n    let s = mk()\n    let n = 0\n    while c {\n        n = n + look(s)\n        if d {\n            eat(s)\n            continue\n        }\n    }\n    return n\n}\n"),
    // The back edge is taken from the body's end even when the body always
    // breaks: `eat(s)` is reported as a second-iteration use.
    ("break_moves", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    while c {\n        eat(s)\n        break\n    }\n    return look(s)\n}\n"),
    // An arm that assigns `s` revives it after the `if`, even though the arm
    // then moved it: accepted.
    ("assign_then_consume_in_arm", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    if c {\n        s = mk()\n        eat(s)\n    }\n    return look(s)\n}\n"),
    // The same in a loop body.
    ("assign_then_consume_in_loop", "fn f(c: Bool) [] -> Int {\n    let s = mk()\n    while c {\n        s = mk()\n        eat(s)\n    }\n    return look(s)\n}\n"),
    // A `when` revives what *any* arm assigns, an arm that leaves included:
    // accepted, though the `Bool` path reaches the read with `s` moved.
    ("when_assign_in_leaving_arm", "fn f(v: Int | Bool) [] -> Int {\n    let s = mk()\n    eat(s)\n    when v {\n        is Int {\n            s = mk()\n            return look(s)\n        }\n        is Bool {\n        }\n    }\n    return look(s)\n}\n"),
    ("else_if_chain", "fn f(a: Bool, b: Bool) [] -> Int {\n    let s = mk()\n    if a {\n        return 0\n    } elif b {\n        eat(s)\n    } else {\n        return 1\n    }\n    return look(s)\n}\n"),
    // An inner loop's move is the outer loop's next-iteration use.
    ("nested_loops", "fn f(c: Bool, d: Bool) [] -> Int {\n    let s = mk()\n    let n = 0\n    while c {\n        n = n + look(s)\n        while d {\n            n = n + 1\n        }\n        if d {\n            eat(s)\n        }\n    }\n    return n\n}\n"),
    ("moved_in_condition", "fn eats(s: Str) [] -> Bool => !s {\n    return true\n}\n\nfn f() [] -> Int {\n    let s = mk()\n    if eats(s) {\n        return look(s)\n    }\n    return 0\n}\n"),
    // A literal moves what it stores after reading every component: the IR
    // evaluates the reading components first [flow-use-after-move].
    ("tuple_reads_before_store", "fn f() [] -> Int {\n    let s = mk()\n    let t = (s, look(s))\n    return 0\n}\n"),
    // A written `if true` that breaks is a branch that leaves, not the
    // builder's block expression: the body is not changed, so not re-walked.
    ("break_in_always_arm", "fn f() [] -> Int {\n    let s = mk()\n    let _r = while true {\n        if true {\n            break s\n        }\n    }\n    return look(s)\n}\n"),
    ("spread_reads_base", "fn f() [] -> Int {\n    let b = Box { s: mk() }\n    let b2 = Box { ...b, s: \"${b.s}!\" }\n    return look(b2.s)\n}\n"),
    // [R8 piece 3a] Fn values: a consuming parameter takes one though
    // `param_mode` lends it (the IR's `moved` flag carries the move).
    ("fn_moved_then_called", "fn f(g: () -> Int) [] -> Int => !g {\n    keep(g)\n    return g()\n}\n"),
    ("fn_moved_then_passed", "fn f(g: () -> Int) [] -> None => !g {\n    keep(g)\n    keep(g)\n}\n"),
    ("fn_moved_then_lent", "fn f(g: () -> Int) [] -> Int => !g {\n    keep(g)\n    return call(g)\n}\n"),
    ("fn_lambda_moved", "fn f() [] -> Int {\n    let g = () -> 1\n    keep(g)\n    return g()\n}\n"),
    ("fn_stored", "fn f(g: () -> Int) [] -> Int => !g {\n    let b = FnBox { run: g }\n    return g()\n}\n"),
    ("fn_returned_in_arm", "fn f(g: () -> Int, c: Bool) [] -> () -> Int => !g {\n    if c {\n        return g\n    }\n    keep(g)\n    return () -> 0\n}\n"),
    ("fn_moved_in_loop", "fn f(g: () -> Int, c: Bool) [] -> None => !g {\n    while c {\n        keep(g)\n    }\n}\n"),
    ("fn_same_call", "fn both(a: () -> Int, b: () -> Int) [] -> None => !a, !b {\n}\n\nfn f(g: () -> Int) [] -> None => !g {\n    both(g, g)\n}\n"),
    // Near misses: a lent fn value, called or lent twice, stays usable.
    ("fn_lent_twice", "fn f(g: () -> Int) [] -> Int {\n    let n = call(g)\n    return n + call(g) + g() + g()\n}\n"),
    ("fn_moved_after_reassign", "fn f() [] -> Int {\n    let g = () -> 1\n    keep(g)\n    g = () -> 2\n    return g()\n}\n"),
    // `once` fns: a call is the move.
    ("once_called_twice", "fn f(g: once () -> Int) [] -> Int {\n    let n = g()\n    return n + g()\n}\n"),
    ("once_in_loop", "fn f(g: once () -> Int) [] -> None {\n    for i in List(1, 2) {\n        g()\n    }\n}\n"),
    ("once_passed_then_called", "fn f(g: once () -> Int) [] -> Int {\n    let n = run_once(g)\n    return n + g()\n}\n"),
    ("once_called_then_passed", "fn f(g: once () -> Int) [] -> Int {\n    let n = g()\n    return n + run_once(g)\n}\n"),
    // Near misses: at most once.
    ("once_maybe", "fn f(g: once () -> Int, c: Bool) [] -> Int {\n    if c {\n        return g()\n    }\n    return 0\n}\n"),
    ("once_each_arm", "fn f(g: once () -> Int, c: Bool) [] -> Int {\n    if c {\n        return g()\n    } else {\n        return g() + 1\n    }\n}\n"),
    ("plain_called_twice", "fn f(g: () -> Int) [] -> Int {\n    return g() + g()\n}\n"),
    // Projection roots: a move of mutable data the analysis could not place
    // (an unwrap) takes the root wholly.
    ("projection_root_unwrap", "fn f(o: Mut List<Str>?) [] -> Int => !o {\n    eatl(o ?: unreachable!())\n    return peek(o)\n}\n"),
    ("projection_root_field", "fn f(o: Bag?) [] -> Int => !o {\n    eatl((o ?: unreachable!()).items)\n    return peek(o)\n}\n"),
    ("projection_root_same_call", "fn eat_both(a: Mut List<Str>, b: Mut List<Str>?) [] -> None => !a, !b {\n}\n\nfn f(o: Mut List<Str>?) [] -> None => !o {\n    eat_both(o ?: unreachable!(), o)\n}\n"),
    ("projection_root_in_arm", "fn f(o: Mut List<Str>?, c: Bool) [] -> Int => !o {\n    if c {\n        eatl(o ?: unreachable!())\n    }\n    return peek(o)\n}\n"),
    // Near misses: reassigned, or moved only on a path that leaves.
    ("projection_root_reassigned", "fn f() [] -> Int {\n    let o: Mut List<Str>? = Mut List<Str>()\n    eatl(o ?: unreachable!())\n    o = None\n    return peek(o)\n}\n"),
    ("projection_root_arm_exits", "fn f(o: Mut List<Str>?, c: Bool) [] -> Int => !o {\n    if c {\n        eatl(o ?: unreachable!())\n        return 1\n    }\n    return peek(o)\n}\n"),
    // [R8 piece 3b] [fate-partial-move] Partial moves: a projection of
    // mutable data moved out of its root. The whole value is then a use, and
    // so is an overlapping place; a disjoint one is not.
    ("part_whole_call", "fn f() [] -> None {\n    let p = newrec()\n    eatl(p.tags)\n    takep(p)\n}\n"),
    ("part_same_field", "fn f() [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    return peek(p.tags)\n}\n"),
    ("part_twice", "fn f() [] -> None {\n    let p = newrec()\n    eatl(p.tags)\n    eatl(p.tags)\n}\n"),
    ("part_disjoint", "fn f() [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    return look(p.name)\n}\n"),
    ("part_param", "fn f(p: Rec) [] -> None => !p {\n    eatl(p.tags)\n    takep(p)\n}\n"),
    // Nested places: a prefix either way overlaps.
    ("part_nested_prefix", "fn f(n: Nest) [] -> Int => !n {\n    eatl(n.a.tags)\n    return peek(n.a)\n}\n"),
    ("part_nested_sibling", "fn f(n: Nest) [] -> Int => !n {\n    eatl(n.a.tags)\n    return look(n.a.name)\n}\n"),
    ("part_nested_disjoint", "fn f(n: Nest) [] -> Int => !n {\n    eatl(n.other)\n    return peek(n.a.tags)\n}\n"),
    ("part_nested_under", "fn f(n: Nest) [] -> Int => !n {\n    takep(n.a)\n    return peek(n.a.tags)\n}\n"),
    // An index may alias any element; a tuple position only itself.
    ("part_index", "fn f() [] -> Int {\n    let xs = array_of(Mut List<Str>(), Mut List<Str>())\n    eatl(xs[0])\n    return peek(xs[1])\n}\n"),
    ("part_index_whole", "fn f() [] -> Int {\n    let xs = array_of(Mut List<Str>(), Mut List<Str>())\n    eatl(xs[0])\n    return peek(xs)\n}\n"),
    ("part_tuple", "fn f(t: (Mut List<Str>, Mut List<Str>)) [] -> Int => !t {\n    eatl(t.0)\n    return peek(t.1) + peek(t)\n}\n"),
    // Reassigning the place, or the root, puts the data back.
    ("part_reassign_field", "fn f() [] -> Int {\n    let p = Mut Rec { name: mk(), tags: Mut List<Str>() }\n    eatl(p.tags)\n    p.tags = Mut List<Str>()\n    return peek(p.tags) + peek(p)\n}\n"),
    ("part_reassign_root", "fn f() [] -> None {\n    let p = newrec()\n    eatl(p.tags)\n    p = newrec()\n    takep(p)\n}\n"),
    ("part_reassign_from_itself", "fn f() [] -> None {\n    let p = Mut Rec { name: mk(), tags: Mut List<Str>() }\n    eatl(p.tags)\n    p.tags = relist(p.tags)\n    takep(p)\n}\n"),
    // Branches and loops: moved on any path is moved.
    ("part_in_arm", "fn f(c: Bool) [] -> Int {\n    let p = newrec()\n    if c {\n        eatl(p.tags)\n    }\n    return peek(p.tags)\n}\n"),
    ("part_arm_exits", "fn f(c: Bool) [] -> Int {\n    let p = newrec()\n    if c {\n        eatl(p.tags)\n        return 0\n    }\n    return peek(p)\n}\n"),
    ("part_when_arm", "fn f(v: Int | Bool) [] -> Int {\n    let p = newrec()\n    when v {\n        is Int {\n            eatl(p.tags)\n        }\n        is Bool {\n        }\n    }\n    return peek(p)\n}\n"),
    ("part_arm_reassigns", "fn f(c: Bool) [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    if c {\n        p = newrec()\n    }\n    return peek(p)\n}\n"),
    ("part_in_loop", "fn f(c: Bool) [] -> Int {\n    let p = newrec()\n    while c {\n        eatl(p.tags)\n    }\n    return peek(p.tags)\n}\n"),
    ("part_loop_reassign", "fn f(c: Bool) [] -> Int {\n    let p = Mut Rec { name: mk(), tags: Mut List<Str>() }\n    while c {\n        eatl(p.tags)\n        p.tags = Mut List<Str>()\n    }\n    return peek(p)\n}\n"),
    ("part_loop_reassign_root", "fn f(c: Bool) [] -> Int {\n    let p = newrec()\n    while c {\n        eatl(p.tags)\n        p = newrec()\n    }\n    return peek(p)\n}\n"),
    ("part_for_binding", "fn f(rs: List<Rec>) [] -> Int => !rs {\n    for r in rs {\n        eatl(r.tags)\n    }\n    return 0\n}\n"),
    // A partially moved value handed on whole: returned, stored, captured.
    ("part_returned", "fn f() [] -> Rec {\n    let p = newrec()\n    eatl(p.tags)\n    return p\n}\n"),
    ("part_stored", "fn f() [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    let h = Holder { r: p }\n    return 0\n}\n"),
    ("part_lambda_reads", "fn f() [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    let g = () -> peek(p)\n    return g()\n}\n"),
    ("part_same_call", "fn f() [] -> None {\n    let p = newrec()\n    eatl2(p.tags, p.tags)\n}\n"),
    ("part_then_whole_moved", "fn f() [] -> Int {\n    let p = newrec()\n    eatl(p.tags)\n    takep(p)\n    return look(p.name)\n}\n"),
    // Through a move-mode binding of the field, and through narrowings.
    ("part_move_mode_binding", "fn f() [] -> Int {\n    let p = newrec()\n    let t = p.tags\n    eatl(t)\n    return look(p.name) + peek(p.tags)\n}\n"),
    ("part_narrowed_root", "fn f(p: Rec?) [] -> None => !p {\n    if p is Rec {\n        eatl(p.tags)\n        takep(p)\n    }\n}\n"),
    ("part_narrowed_field", "fn f(o: OptRec) [] -> Int => !o {\n    if o.inner is Rec {\n        eatl(o.inner.tags)\n    }\n    return peek(o.inner)\n}\n"),
    // Near misses: a Copy scalar field, and immutable data, never move.
    ("part_copy_scalar", "fn f(c: Cnt) [] -> Int => !c {\n    take(c.n)\n    take(c.n)\n    return c.n + peek(c)\n}\n"),
    ("part_immutable_field", "fn f() [] -> Int {\n    let p = newrec()\n    eat(p.name)\n    takep(p)\n    return 0\n}\n"),
];

fn program(cases: &[(&str, &str)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, body) in cases {
        let file = format!("{name}.sv");
        let module = SourceSet::classify(Path::new(&file)).unwrap();
        sources.add(&file, module, format!("{HELPERS}{body}"), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    Program { files: sources.files, modules, companions: sources.companions }
}

/// A position within a case's own source, 1-based.
fn case_line_col(content: &str, offset: u32) -> (u32, u32) {
    let (line, col) = salvo_syntax::span::line_col(content, offset);
    (line - HELPERS.lines().count() as u32, col)
}

#[test]
fn the_gate_reports_what_the_flow_pass_finds() {
    let program = program(CASES);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let user: HashMap<&str, usize> =
        program.files.iter().enumerate().filter(|(_, f)| !f.is_std).map(|(i, f)| (f.name.as_str(), i)).collect();

    // (a) The checker as the gates run it: no error in the corpus at all (a
    // consumed read is never the checker's), then the IR of the corpus
    // modules and the flow pass over it.
    let normal = salvo_core::check_program(&program, &resolution, &symbols);
    let errors: Vec<String> = normal
        .errors
        .iter()
        .filter(|d| d.is_error() && !program.files[d.file].is_std)
        .map(|d| d.render(&program.files))
        .collect();
    assert!(errors.is_empty(), "the checker reports errors in the corpus:\n{}", errors.join("\n"));
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
        program.units().filter(|u| !u.file.is_std).map(|u| &u.file.module).collect();
    let (ir, build_errors) = salvo_core::ir::build_program(&program, &symbols, &resolution, &normal, Some(&wanted));
    assert!(build_errors.is_empty(), "IR build errors: {build_errors:?}");
    let mut flow: BTreeMap<usize, BTreeSet<u32>> = BTreeMap::new();
    let mut total = 0;
    for module in &ir.modules {
        let file = user[module.file_name.as_str()];
        for decl in &module.decls {
            for d in salvo_core::ir::flow::check_one(decl) {
                assert!(
                    d.message.contains(&format!("`{}` cannot be used", d.local)) || d.message.contains(&format!("moved out of `{}`", d.local)),
                    "{}",
                    d.message
                );
                flow.entry(file).or_default().insert(d.span.start);
                total += 1;
            }
        }
    }
    assert_eq!(salvo_core::ir::flow::check(&ir).len(), total, "`check` is `check_one` over every decl");

    // (b) The gate (`ir::analyze`) reports exactly those reads, naming the
    // move.
    let mut gate: BTreeMap<usize, BTreeSet<u32>> = BTreeMap::new();
    for d in salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &normal) {
        assert!(d.is_error(), "{}", d.message);
        assert!(
            d.message.contains("cannot be used here: it was consumed (moved) at line ")
                || d.message.contains("cannot be used as a whole here: `")
                    && d.message.contains("` was moved out of it at line ")
                || d.message.contains("cannot be used here: `") && d.message.contains("` at line "),
            "the gate names the move: {}",
            d.message
        );
        gate.entry(d.file).or_default().insert(d.span.start);
    }
    assert_eq!(gate, flow, "`ir::analyze` reports what the flow pass finds");
    assert!(CASES.iter().all(|(name, _)| user.contains_key(format!("{name}.sv").as_str())));
}

/// What the flow pass reports on each case (line, column within the case):
/// the reads the checker reported with its own diagnostic switched back on,
/// for as long as it had one (the differential, retired with it in R8 piece
/// 3a), so a change to the pass's rules shows up as the case that moved.
#[test]
fn the_flow_pass_reports_these_reads() {
    let expected: &[(&str, &[(u32, u32)])] = &[
        ("straight", &[(4, 17)]),
        ("one_arm", &[(6, 17)]),
        ("both_arms", &[(8, 17)]),
        ("arm_exits", &[]),
        ("when_arm", &[(10, 17)]),
        ("loop_then_after", &[(4, 13), (6, 17)]),
        ("loop_next_iteration", &[(5, 22), (6, 13)]),
        ("reassign_revives", &[]),
        ("loop_reassign_revives", &[]),
        ("return_in_arm", &[]),
        ("struct_store", &[(4, 17)]),
        ("spread", &[(4, 17)]),
        ("use_arg", &[(14, 17)]),
        ("break_value", &[(8, 17)]),
        ("lambda_mutates", &[(5, 17)]),
        ("lambda_mutates_int", &[]),
        ("copy_scalar", &[]),
        ("generic_param", &[(3, 10)]),
        ("narrowed_scalar", &[(5, 10)]),
        ("narrowed_consume", &[(5, 8)]),
        ("narrowed_after_guard", &[(6, 17)]),
        ("narrowed_kept_in_arm", &[(7, 21)]),
        ("narrowed_moved_in_arm", &[(8, 17)]),
        ("same_call", &[(3, 13)]),
        ("nested_lambda", &[(5, 21)]),
        ("lambda_reads_moved", &[(4, 24)]),
        ("for_each", &[(6, 13), (8, 17)]),
        ("interpolate", &[(4, 15)]),
        ("continue_skips", &[]),
        ("break_moves", &[(4, 13), (7, 17)]),
        ("assign_then_consume_in_arm", &[]),
        ("assign_then_consume_in_loop", &[]),
        ("when_assign_in_leaving_arm", &[]),
        ("else_if_chain", &[(10, 17)]),
        ("nested_loops", &[(5, 22), (10, 17)]),
        ("moved_in_condition", &[(8, 21)]),
        ("tuple_reads_before_store", &[]),
        ("spread_reads_base", &[]),
        ("break_in_always_arm", &[(8, 17)]),
        ("fn_moved_then_called", &[(3, 12)]),
        ("fn_moved_then_passed", &[(3, 10)]),
        ("fn_moved_then_lent", &[(3, 17)]),
        ("fn_lambda_moved", &[(4, 12)]),
        ("fn_stored", &[(3, 12)]),
        ("fn_returned_in_arm", &[]),
        ("fn_moved_in_loop", &[(3, 14)]),
        ("fn_same_call", &[(5, 13)]),
        ("fn_lent_twice", &[]),
        ("fn_moved_after_reassign", &[]),
        ("once_called_twice", &[(3, 16)]),
        ("once_in_loop", &[(3, 9)]),
        ("once_passed_then_called", &[(3, 16)]),
        ("once_called_then_passed", &[(3, 25)]),
        ("once_maybe", &[]),
        ("once_each_arm", &[]),
        ("plain_called_twice", &[]),
        ("projection_root_unwrap", &[(3, 17)]),
        ("projection_root_field", &[(3, 17)]),
        ("projection_root_same_call", &[(5, 35)]),
        ("projection_root_in_arm", &[(5, 17)]),
        ("projection_root_reassigned", &[]),
        ("projection_root_arm_exits", &[]),
        ("part_whole_call", &[(4, 11)]),
        ("part_same_field", &[(4, 17)]),
        ("part_twice", &[(4, 10)]),
        ("part_disjoint", &[]),
        ("part_param", &[(3, 11)]),
        ("part_nested_prefix", &[(3, 17)]),
        // The checker checks a field read's base too, so `n.a` of
        // `n.a.name` overlaps the moved `n.a.tags` (over-strict, mirrored).
        ("part_nested_sibling", &[(3, 17)]),
        ("part_nested_disjoint", &[]),
        ("part_nested_under", &[(3, 17)]),
        ("part_index", &[(4, 17)]),
        ("part_index_whole", &[(4, 17)]),
        ("part_tuple", &[(3, 29)]),
        ("part_reassign_field", &[]),
        ("part_reassign_root", &[]),
        // The target is put back before the value is read, so the value's
        // read of the moved field is not a use and its move stays (mirrored:
        // rustc reports the read, E0382, not `takep(p)`).
        ("part_reassign_from_itself", &[(5, 11)]),
        ("part_in_arm", &[(6, 17)]),
        ("part_arm_exits", &[]),
        ("part_when_arm", &[(10, 17)]),
        // Reassigning in one arm does not put the parts back after the join:
        // the other path still has `p.tags` moved.
        ("part_arm_reassigns", &[(7, 17)]),
        ("part_in_loop", &[(4, 14), (6, 17)]),
        ("part_loop_reassign", &[]),
        ("part_loop_reassign_root", &[]),
        ("part_for_binding", &[]),
        ("part_returned", &[(4, 12)]),
        ("part_stored", &[(4, 25)]),
        ("part_lambda_reads", &[(4, 24)]),
        // The one difference from the checker, which accepted this and
        // failed rustc (E0382): it moved a call's projection arguments after
        // reading them all [deduce-same-call].
        ("part_same_call", &[(3, 19)]),
        // A whole use of a partially moved value, which then moves it.
        ("part_then_whole_moved", &[(4, 11), (5, 17)]),
        ("part_move_mode_binding", &[(5, 32)]),
        ("part_narrowed_root", &[(4, 15)]),
        ("part_narrowed_field", &[(5, 17)]),
        ("part_copy_scalar", &[]),
        ("part_immutable_field", &[]),
    ];
    assert_eq!(expected.len(), CASES.len(), "one expectation per case");
    let program = program(CASES);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_core::check_program(&program, &resolution, &symbols);
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
        program.units().filter(|u| !u.file.is_std).map(|u| &u.file.module).collect();
    let (ir, _) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, Some(&wanted));
    let mut found: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    for module in &ir.modules {
        let content = &program.files.iter().find(|f| f.name == module.file_name).unwrap().content;
        let name = module.file_name.trim_end_matches(".sv").to_string();
        let reads = found.entry(name).or_default();
        for d in module.decls.iter().flat_map(salvo_core::ir::flow::check_one) {
            reads.push(case_line_col(content, d.span.start));
        }
        reads.sort();
    }
    let mut wrong = Vec::new();
    for (name, want) in expected {
        let got = found.get(*name).cloned().unwrap_or_default();
        if got != *want {
            wrong.push(format!("{name}: expected {want:?}, got {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}


fn example_program(dir: &Path) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let project = salvo_core::Project::load(&dir.join(salvo_core::MANIFEST_FILE)).expect("manifest");
    let src = project.src();
    let errors = sources.add_dir(&src, "kt", false, false);
    assert!(errors.is_empty(), "failed to read {}: {errors:?}", src.display());
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(!diagnostics.iter().any(|d| d.is_error()), "parse errors in {}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    assert!(!expansion.diagnostics.iter().any(|d| d.is_error()));
    Program { files: sources.files, modules, companions: sources.companions }
}

/// No false positives: over the IR of every example and of std (built from
/// the checker's tables, as the gates do), the flow pass reports
/// nothing — the checker accepts all of it.
#[test]
fn the_flow_pass_accepts_every_example_and_std() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.join("salvo.toml").exists()
                && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]")
        })
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty());
    let mut found = Vec::new();
    for (i, dir) in dirs.iter().enumerate() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let program = example_program(dir);
        let symbols = salvo_core::Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let checked = salvo_core::check_program(&program, &resolution, &symbols);
        // std once (with the first example), then each example's own modules.
        let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
            program.units().filter(|u| i == 0 || !u.file.is_std).map(|u| &u.file.module).collect();
        let (ir, _) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, Some(&wanted));
        for module in &ir.modules {
            let content = &program.files.iter().find(|f| f.name == module.file_name).unwrap().content;
            for d in module.decls.iter().flat_map(salvo_core::ir::flow::check_one) {
                let (line, col) = salvo_syntax::span::line_col(content, d.span.start);
                found.push(format!("{name}: {}:{line}:{col}: {}", module.file_name, d.message));
            }
        }
    }
    assert!(found.is_empty(), "{} false positive(s):\n{}", found.len(), found.join("\n"));
}

