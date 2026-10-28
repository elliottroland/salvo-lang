//! [iter-fn] What of the `iter fn` checks cannot be test data
//! (`tests/check/iter_fn/`, TESTS.md T1): a driver that skips the expansion,
//! and a case whose diagnostics are kept as they were until a defect is
//! fixed. Both keep the hand-written prelude they were written over.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic type List<T> canbe Mut\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export intrinsic fn Mut List<T>(...elems: T[]) [] -> Mut List<T>\n\
     export intrinsic fn List<T>(...elems: T[]) [] -> List<T>\n\
     export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n\
     export intrinsic fn get<T>(list: List<T>, index: Int) [] -> T? => list, index\n\
     export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n\
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T>(value: T) [] -> +Emitted T => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     export fns Next<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n\
     export fns Iter<C, It, T> => iter with next {\n    fn iter(collection: C) -> Mut It\n    fn next(iterator: Mut It) -> Emitted T | Finished => iterator: Mut\n}\n\
     export effect Console {\n    fn print(message: Str) -> None => !message\n}\n\
     export handler StdConsole of Console {\n    fn print(message: Str) -> None => !message {}\n}\n\
     export fn println(message: Str) [Console] -> None => !message {}\n",
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

/// [diag-id] The id of every diagnostic — parse, resolve and check —
/// because an `iter fn`'s own rules are reported by the desugaring, which
/// runs inside the parser.
fn errors(src: &str) -> Vec<&'static str> {
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
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut parse_errors = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        parse_errors.extend(
            diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.id.unwrap_or("?")),
        );
        modules.push(ast);
    }
    // The program-level expansion, as every driver runs it.
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    parse_errors.extend(
        expansion
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.id.unwrap_or("?")),
    );
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    parse_errors
        .into_iter()
        .chain(resolution.errors.iter().map(|d| d.id.unwrap_or("?")))
        // Errors only: an unused-variable *warning* [var-unused] is a
        // different severity, and these tests are about legality.
        .chain(
            checked
                .errors
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.id.unwrap_or("?")),
        )
        .collect()
}

/// [iter-fn] The integration guard: a driver that parses with
/// `parse_module_deferred` owes the module the program-level expansion
/// (`salvo_core::expand`) before resolution. A surviving `iter fn` must
/// never be checked as an ordinary fn (silently different behavior), so
/// resolve reports it loudly instead.
// Kept with the prelude: the case is a driver skipping `salvo_core::expand`, which every front runs.
#[test]
fn an_unexpanded_iter_fn_is_a_loud_resolution_error() {
    let src = "struct Countdown {\n    from: Int\n}\n\
               iter fn iter(c: Countdown) -> Emitted Int | Finished {\n\
               \x20   state CountdownIter {\n        at: Int = 0\n    }\n\
               \x20   if at >= c.from {\n        return finished()\n    }\n\
               \x20   at = at + 1\n\
               \x20   return emitted(at)\n\
               }\n";
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
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        // Deliberately deferred, and never expanded: the mistake under test.
        let (ast, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(diagnostics.iter().all(|d| !d.is_error()), "{diagnostics:?}");
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    assert!(
        resolution
            .errors
            .iter()
            .any(|d| d.id == Some("iter-fn:unexpanded")),
        "expected the loud guard, got: {:?}",
        resolution.errors
    );
}

/// The name is declared beside the fn, so it collides with a type of the same
/// name there — and with another `iter fn`'s state.
// Kept: two `iter fn`s sharing a state name also report the generated `next` as a duplicate fn.
#[test]
fn a_state_name_collides_with_a_type() {
    let errs = errors(
        "struct Range { a: Int }\n\
         iter fn upto(n: Int) -> Emitted Int | Finished {\n    state Range {\n        at: Int = 0\n    }\n    return finished()\n}\n",
    );
    assert!(
        errs.contains(&"mod-collision:duplicate"),
        "{errs:?}"
    );
    let errs = errors(
        "iter fn upto(n: Int) -> Emitted Int | Finished {\n    state Upto {}\n    return finished()\n}\n\
         iter fn downto(n: Int) -> Emitted Int | Finished {\n    state Upto {}\n    return finished()\n}\n",
    );
    assert!(
        errs.contains(&"mod-collision:duplicate"),
        "{errs:?}"
    );
}
