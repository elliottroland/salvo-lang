//! Effects on fn types [fn-effects].
//!
//! An effect list on a fn type is a *requirement the caller of the value
//! supplies*, not a capability the value carries: `(s: Str) [Logger] -> Str`
//! means "call me with Logger available". So a lambda body performs only the
//! effects its type declares, a higher-order fn *inherits* its fn-typed
//! parameters' effects (user decision 2026-09-04 — the only reason to take
//! `f` is to call it), and a fn value can be stored or passed freely because
//! it holds no handler: only *calling* it needs one.
//!
//! The rest of these tests are test data, `tests/check/fn_effects/` (TESTS.md T1).

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
const STD_ARRAY: &str = "export struct ArrayNext<T> canbe Mut {\n    at: Int\n}\n\
     export intrinsic fn iter<T>(array: T[]) [] -> Mut ArrayNext<T> => array\n\
     export intrinsic fn next<T>(p: Mut ArrayNext<T>) [] -> Emitted T | Finished => p: Mut\n";

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T>(value: T) [] -> +Emitted T => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     export fns Next<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

/// [diag-id] The id of every error `src` reports.
fn errors(src: &str) -> Vec<&'static str> {
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
    // Errors only: an unused-variable *warning* [var-unused] is a different
    // severity, and these tests are about which programs are rejected.
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.id.unwrap_or("?"))
        .collect()
}

const PRELUDE: &str = r#"

effect Logger {
    fn log(message: Str) -> None => message
}

effect Counter {
    fn bump() -> None
}

handler QuietLogger of Logger {
    fn log(message: Str) -> None => message {}
}
handler ZeroCounter of Counter {
    fn bump() -> None {}
}

fn note(text: Str) [] -> None => text {}
"#;

// ===== `use` is not part of a fn type =====

/// [fn-effects] Registering a handler is local to a body, so `use` in a fn
/// type's effect list is meaningless — a lambda may `use` exactly when the
/// function containing it may.
///
/// Kept over its prelude, not moved to `check/fn_effects/`: checked exactly,
/// one `[use]` is reported three times (`fn-effects:use`).
#[test]
fn use_in_a_fn_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [use] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n"
    ));
    assert!(errs.contains(&"fn-effects:use"), "expected the `use` rejection, got: {errs:?}");
}
