//! [flow-use-after-move] The IR's use-after-move pass (`ir::flow`, PIPELINE.md
//! R8 piece 1) against the checker's consumed-read diagnostic
//! [deduce-consume]: over a corpus of small programs, the reads the checker
//! reports as "cannot be used here: it was consumed (moved) by …" (the normal
//! walk) must be the reads the flow pass reports over the IR built from the
//! **ownership-blind** walk's tables [ownership-audit] — which is what the
//! pass will see once typing runs ownership-blind (piece 2).
//!
//! Positions are compared by the read's start offset: the checker reports a
//! moved base at the identifier (`b` in `b.s`), the IR at the whole read,
//! which starts there. The checker's same-call variant ("consumed (moved) by
//! an earlier argument of this call") counts: it fires on the later argument,
//! which for a bare name is exactly the read the flow pass reports.
//!
//! `KNOWN` lists the differences that are understood and left for piece 2,
//! each with its reason; anything else is a failure, in either direction.

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
];

/// Understood differences, left for piece 2: `(case, side, line, column)`
/// where `side` is the one that reports the read (`"checker"`: the IR does
/// not mark that move consumed), and line/column are within the case's
/// source.
///
/// The IR marks a move `consume: true` only where the emitters need it: a
/// call's moved argument, a `return`, a move-mode binding, a linear value's
/// move, a lambda's consumed capture. A literal store, a spread, a `use`
/// constructor argument and a `break` value are moves to the checker but
/// plain reads in the IR — and marking them would change the Rust output
/// (an unconsumed read of an owned value is `.clone()`d, a consumed one is
/// moved), so piece 2 must carry those moves some other way.
const KNOWN: &[(&str, &str, u32, u32)] = &[
    ("struct_store", "checker", 4, 17),
    ("spread", "checker", 4, 17),
    ("use_arg", "checker", 14, 17),
    ("break_value", "checker", 8, 17),
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

const CONSUMED_READ: &str = "it was consumed (moved) by";
const SAME_CALL: &str = "consumed (moved) by an earlier argument of this call";

#[test]
fn the_flow_pass_reports_the_reads_the_checker_reports() {
    let program = program(CASES);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let user: HashMap<&str, usize> =
        program.files.iter().enumerate().filter(|(_, f)| !f.is_std).map(|(i, f)| (f.name.as_str(), i)).collect();

    // (a) The checker, normally: its consumed-read errors; nothing else may
    // be an error.
    let normal = salvo_core::check_program(&program, &resolution, &symbols);
    let mut checker: BTreeMap<usize, BTreeSet<u32>> = BTreeMap::new();
    let mut unexpected = Vec::new();
    for d in normal.errors.iter().filter(|d| d.is_error() && !program.files[d.file].is_std) {
        if d.message.contains(CONSUMED_READ) {
            checker.entry(d.file).or_default().insert(d.span.start);
        } else {
            unexpected.push(d.render(&program.files));
        }
    }
    assert!(unexpected.is_empty(), "the corpus must only have consumed-read errors:\n{}", unexpected.join("\n"));

    // (b) Ownership-blind: no typing error in the corpus (the same-call
    // rule is not consumption and still fires), then the IR of the corpus
    // modules and the flow pass over it.
    let blind = salvo_core::check_program_ownership_blind(&program, &resolution, &symbols);
    let typing: Vec<String> = blind
        .errors
        .iter()
        .filter(|d| d.is_error() && !program.files[d.file].is_std && !d.message.contains(SAME_CALL))
        .map(|d| d.render(&program.files))
        .collect();
    assert!(typing.is_empty(), "the ownership-blind walk reports errors in the corpus:\n{}", typing.join("\n"));
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
        program.units().filter(|u| !u.file.is_std).map(|u| &u.file.module).collect();
    let (ir, build_errors) = salvo_core::ir::build_program(&program, &symbols, &resolution, &blind, Some(&wanted));
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

    // Compare per case, modulo the known differences.
    let mut mismatches = Vec::new();
    let mut known_seen = BTreeSet::new();
    for (name, _) in CASES {
        let file = user[format!("{name}.sv").as_str()];
        let content = &program.files[file].content;
        let empty = BTreeSet::new();
        let c = checker.get(&file).unwrap_or(&empty);
        let f = flow.get(&file).unwrap_or(&empty);
        for (side, only) in [("checker", c.difference(f)), ("flow", f.difference(c))] {
            for &at in only {
                let (line, col) = case_line_col(content, at);
                if KNOWN.contains(&(name, side, line, col)) {
                    known_seen.insert((*name, side, line, col));
                } else {
                    mismatches.push(format!("{name}: only the {side} reports {line}:{col}"));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "the checker and the flow pass disagree:\n{}", mismatches.join("\n"));
    let stale: Vec<_> = KNOWN.iter().filter(|k| !known_seen.contains(*k)).collect();
    assert!(stale.is_empty(), "known differences that no longer occur (remove them): {stale:?}");
}

/// What the corpus expects each side to agree on, spelled out per case, so a
/// change to either side's rules shows up as the case that moved rather than
/// as a quietly re-agreeing pair: the reads (line, column within the case)
/// the flow pass reports.
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
        ("struct_store", &[]),
        ("spread", &[]),
        ("use_arg", &[]),
        ("break_value", &[]),
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
    ];
    assert_eq!(expected.len(), CASES.len(), "one expectation per case");
    let program = program(CASES);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let blind = salvo_core::check_program_ownership_blind(&program, &resolution, &symbols);
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
        program.units().filter(|u| !u.file.is_std).map(|u| &u.file.module).collect();
    let (ir, _) = salvo_core::ir::build_program(&program, &symbols, &resolution, &blind, Some(&wanted));
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
/// the ownership-blind tables, as piece 2 will), the flow pass reports
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
        let blind = salvo_core::check_program_ownership_blind(&program, &resolution, &symbols);
        // std once (with the first example), then each example's own modules.
        let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
            program.units().filter(|u| i == 0 || !u.file.is_std).map(|u| &u.file.module).collect();
        let (ir, _) = salvo_core::ir::build_program(&program, &symbols, &resolution, &blind, Some(&wanted));
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
