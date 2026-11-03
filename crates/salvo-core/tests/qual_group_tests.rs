//! Qualified unions on a state-qualifier prelude [qual-group].
//!
//! The re-typing that opens one (`o := -Ok` [qual-retype]) is test data,
//! `tests/check/qualifiers/retype.sv`. What stays here could not move to the
//! real std without a different result; each says why.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `std.core.prelude`: `std.core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn first<T>(list: List<T>) [] -> proj(list) T? => list\n",
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

/// [diag-id] Every error as `(id, message)`.
fn errors(src: &str) -> Vec<(&'static str, String)> {
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
        interfaces: None,
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut all: Vec<_> = resolution.errors.iter().chain(checked.errors.iter()).filter(|d| d.is_error()).cloned().collect();
    // [fate-poison] As the gates do, the IR analyses run on every
    // declaration the checker's errors leave clean [ir-analyze]: a poisoned
    // read is the IR flow pass's since R8 piece
    // 3c [flow-use-after-move].
    all.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked).into_iter().filter(|d| d.is_error()));
    all.into_iter().map(|d| (d.id.unwrap_or("?"), d.message)).collect()
}

const PRELUDE: &str = r#"

struct Person canbe Mut {
    name: Str
}

qualifier Ok<T> of T
qualifier Err<T> of T
qualifier Surname of Person

fn note(text: Str) [] -> None => text {}
fn read(p: Person) [] -> None => p {}
fn touch(p: Mut Person) [] -> None => p: Mut {}

fn ok(value: Int) [] -> +Ok Int {
    return value
}

fn err(value: Str) [] -> +Err Str => !value {
    return value
}
"#;

// Kept in Rust, on this prelude, whose `Ok` is a state qualifier. Over the
// real std `Ok` is a provenance qualifier, and `return ok(ok(1))` into `Ok
// (Ok Int | Err Str) | Err Str` is the arm inferred and wrapped twice: the
// run data's `qualifiers/provenance_groups.sv`.
/// [qual-group] The limit of the flat representation, and the reason the
/// annotated-`let` leftover survives: the *same* qualifier twice deduplicates
/// to one, so `ok(ok(x))` is indistinguishable from `ok(x)` and no rule can
/// recover the nesting. The diagnostic says so, since the type it prints
/// looks like it should fit.
#[test]
fn the_same_qualifier_twice_deduplicates_and_says_so() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn step() [] -> Ok (Ok Int | Err Str) | Err Str {{\n\
         return ok(ok(1))\n\
         }}\n"
    ));
    // Kept on the text past the id: the point is the hint the message adds
    // when the qualifier deduplicated, which the id does not tell apart.
    assert!(
        errs.iter().any(|(id, m)| *id == "type-union:no-arm" && m.contains("deduplicates")),
        "expected the deduplication hint, got: {errs:?}"
    );
}
