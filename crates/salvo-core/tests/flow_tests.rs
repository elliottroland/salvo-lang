//! [flow-use-after-move] The IR's use-after-move pass (`ir::flow`, PIPELINE.md
//! R8), the only use-after-move diagnostic since piece 3a: over a corpus of
//! small programs, the checker reports nothing (a consumed read is never its
//! error), the gate (`ir::analyze`) reports what the pass finds over the IR
//! built from the checker's tables, and each case's reads are pinned.
//!
//! Until piece 3a this was a differential: the checker's own "consumed
//! (moved) by …" diagnostic, switched back on for the test, had to report
//! the same reads (by start offset, its same-call variant included). It
//! agreed on the final corpus — every fn-value, `once`-call and
//! projection-root case included — and was removed with the diagnostic; the
//! pinned expectations are those reads. The checker still owns the
//! consumption marking (`narrowed = Never`) its linear and poison machinery
//! read, so a case whose expectation changes must say whether the pass's rule
//! changed (ROADMAP 0o's unsound join/loop rules, say) or the checker's
//! marking did.

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
                assert!(d.message.contains(&format!("`{}` cannot be used here", d.local)), "{}", d.message);
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
            d.message.contains("cannot be used here: it was consumed (moved) at line "),
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
