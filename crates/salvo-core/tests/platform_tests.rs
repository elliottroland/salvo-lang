//! [platform-type] The platform tests that are not yet data: the rest are
//! `tests/check/platform/` (TESTS.md T1). What is left is kept as it was,
//! over its prelude, because over the real std a refused `by` clause on a
//! platform type also reports diagnostics of its own (ROADMAP §0ar).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `std.core.prelude`: `std.core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n",
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

fn check_errors(src: &str) -> Vec<FileDiagnostic> {
    check_files(&[("main.sv", src)])
}

/// [platform-check] Several user modules beside the prelude.
fn check_files(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(*name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), false);
    }
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
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    // [ir-analyze] The path-exit/throw linear leak checks moved to the IR.
    errors.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked));
    errors.extend(checked.errors);
    errors
}

/// [platform-type] What the kinds forbid is refused at the declaration: an
/// obligation clause stamped `by`. (The rest of this test — the three kinds
/// in use, the threadsafe value type, `iterable` without `Iter`, the linear
/// leak — is `check/platform/types.sv`.) Kept over the prelude: over the real
/// std, `: Hashed<self> by auto` also reports `group-obligation:no-member`
/// for `hash` and `eq`, and `: Iter<self, ClientIter, Int> by auto` also
/// reports `obligation-by:not-self` suggesting `: Iter<self> by auto`.
#[test]
fn platform_types_are_opaque_handles_in_three_kinds() {
    let errors = |src: &str| -> Vec<&'static str> {
        check_errors(src).iter().filter(|d| d.is_error()).map(|d| d.id.unwrap_or("?")).collect()
    };
    let refused = [
        ("platform type Client : Hashed<self> by auto\n", "platform-type:obligation"),
        // [platform-iterable] `Iter` is the one clause it may take, and not
        // stamped.
        ("platform type Client : Iter<self, ClientIter, Int> by auto\n", "platform-type:obligation"),
    ];
    for (src, want) in refused {
        let errs = errors(src);
        assert!(errs.contains(&want), "{src}: wanted `{want}` in {errs:?}");
    }
}
