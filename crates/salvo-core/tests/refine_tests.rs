//! Qualifier refinements [qual-refn]: a qualifier states what functions it
//! does not own do to *its* claim, which is what recovers the precision
//! [deduce-syntax] gives up when it forbids a mutating function from
//! promising a qualifier it has never heard of.
//!
//! The observable in most of these tests is *overload resolution*: a
//! `needs_nonempty` overload matches only while the checker still believes
//! the value is `NonEmpty`, so "clean" means the refinement applied and
//! "no matching overload" means it did not.
//!
//! The diagnostics are test data, `tests/check/refinements/` (TESTS.md T1).
//! What stays here could not move to the real std without a different
//! result; it says why.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Str\nexport intrinsic type Int\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

/// Parses + resolves + checks the given files (the first is `main.sv`) and
/// returns every diagnostic message, warnings included — an unnecessary
/// selector is a *warning* [qual-refn], so the tests have to see both
/// severities.
fn diagnostics(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, content) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            content.to_string(),
            false,
        );
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, parse) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = parse.iter().filter(|d| d.is_error()).collect();
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
        .map(|d| d.message.clone())
        .collect()
}

/// A list-like type with a mutator whose exhaustive deduction list
/// necessarily drops every claim it does not name [deduce-syntax] — the
/// situation refinements exist for.
const PRELUDE: &str = r#"
export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

export fn needs_nonempty<T>(s: NonEmpty Store<T>) -> Int => s {
    return 1
}

"#;

/// `NonEmpty` with the refinement that makes `push` establish it.
const NONEMPTY: &str = r#"
export qualifier NonEmpty<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    // Pushing a value leaves the store non-empty.
    refn push(s: Mut Store<T>, value: T) => s: +NonEmpty
}
"#;

// Kept in Rust, on this prelude: the importing half is test data
// (`refinements/places.sv`), but in the file without the import the unknown
// qualifier is reported twice — `unknown qualifier` and then an annotation
// mismatch against the type it could not resolve ("expected `NonEmpty
// Store<Int>`, found `Mut Store<Int>`") — so the data file would pin a second
// diagnostic for one mistake [type-unknown-lenient].
/// [qual-refn-scope] A qualifier's refinements travel with it: a file that
/// does not import the qualifier does not get its refinements, which is what
/// "the user opts into the refinements when they opt into the qualifier"
/// means.
#[test]
fn a_refinement_is_only_in_scope_with_its_qualifier() {
    let quals = format!(
        "{PRELUDE}{NONEMPTY}\nfn qualified<T>(s: Store<T>) -> +NonEmpty Store<T>\n=> !s {{\n    \
         return s\n}}\n"
    );
    // Importing the qualifier: the refinement applies.
    let with_import = "import quals.NonEmpty\nimport quals.Store\n\
                       import quals.push\nimport quals.needs_nonempty\n\
                       export fn g(s: Mut Store<Int>) -> None {\n    \
                       push(s, 1)\n    \
                       let _n = needs_nonempty(s)\n}\n";
    assert!(
        diagnostics(&[("quals.sv", &quals), ("user.sv", with_import)]).is_empty(),
        "{:?}",
        diagnostics(&[("quals.sv", &quals), ("user.sv", with_import)])
    );
    // Not importing it: `needs_nonempty` cannot be named either, so the
    // observable is the *other* overload — the value stays unqualified, and
    // a call that needs the claim is unavailable by name. What this test
    // pins is that the refinement is not applied program-wide: the file
    // without the import reports the unresolved qualifier rather than
    // silently benefiting from it.
    let without = "import quals.Store\nimport quals.push\n\
                   export fn g(s: Mut Store<Int>) -> None {\n    \
                   push(s, 1)\n    \
                   let t: NonEmpty Store<Int> = s\n}\n";
    let diags = diagnostics(&[("quals.sv", &quals), ("user.sv", without)]);
    assert!(
        diags.iter().any(|d| d.contains("NonEmpty")),
        "expected the unimported qualifier to be unknown here: {diags:?}"
    );
}
