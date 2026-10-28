//! Place-based flow narrowing [flow-place] [flow-place-invalidate].
//!
//! `is` narrows *places* — a variable or a field chain out of one — not
//! just variables, so a checked field reads at its narrowed type. The
//! observable used here is `[interp-no-none]`: interpolating a
//! possibly-`None` value is an error, so an accepted interpolation of
//! `p.surname` is proof the field narrowed, and a rejected one is proof
//! the fact was invalidated.
//!
//! The rest of these cases are test data, `tests/check/places/` (TESTS.md
//! T1). This one stays, over its prelude, until a defect is fixed (ROADMAP
//! §0ar): assigning `None` to a narrowed `Str?` field also reports
//! `var-no-widen:assign-mismatch` ("cannot assign `None` to `Str`"), which
//! [var-no-widen] says it must not; the test below filters it out.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `std.core.prelude`: `std.core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
/// [iter-for] [iter-for-native] What a `for` over an array calls, in the
/// arrays' std home [op-unique-shape]: `for x in xs` is `next(iter(xs))`, which
/// the backends walk natively when these are the calls.
const STD_ARRAY: &str = "export qualifier Emitted<T> of T\nexport struct Finished {}\n\
     export struct ArrayNext<T> canbe Mut {\n    at: Int\n}\n\
     export intrinsic fn iter<T>(array: T[]) [] -> Mut ArrayNext<T> => array\n\
     export intrinsic fn next<T>(p: Mut ArrayNext<T>) [] -> Emitted T | Finished => p: Mut\n";

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/array.sv",
        SourceSet::classify(Path::new("std/core/array.sv")).unwrap(),
        STD_ARRAY.to_string(),
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
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    nick: Str? = None,
    tags: Str?[],
    pair: (Str?, Str?),
    address: Mut Address
}

// Opaque observers. Deduction lists are *written*, which is what these
// tests turn on [decl-explicit]: the declared contract is authoritative, so
// the body only has to exist.
fn touch(p: Mut Person) [] -> None => p: Mut {
    p.name = p.name
}

fn read(p: Person) [] -> None => p {}

fn touch_address(a: Mut Address) [] -> None => a: Mut {
    a.city = a.city
}
"#;

fn check(body: &str) -> Vec<String> {
    errors(&format!("{PRELUDE}\nfn probe(p: Mut Person) -> Str => p: Mut {{\n{body}\n}}\n"))
}

fn interp_errors(body: &str) -> Vec<String> {
    check(body)
        .into_iter()
        .filter(|e| e.contains("may be `None`"))
        .collect()
}

/// [flow-place-invalidate] Assigning to the checked field drops the fact.
#[test]
fn assignment_to_the_place_invalidates() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        p.surname = None
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after assignment to fail: {errs:?}");
}
