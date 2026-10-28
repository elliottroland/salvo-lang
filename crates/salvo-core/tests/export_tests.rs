//! [mod-export] [mod-std-internal] [mod-use] [runtime-layers] What std's own
//! modules may do. Everything a *program* sees of the export rule is test
//! data, `tests/check/exports/` (TESTS.md T1); these tests stay in Rust over
//! a prelude because their point is std files of the test's own (a
//! `std/runtime.sv`, a `std/runtime/timers.sv`), which the real std cannot
//! be swapped for.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// The base types the checker needs, as a std file [platform-intrinsic-std-only] — and
/// exported, since std obeys this rule like anything else.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\n\
                           export intrinsic type Bool\nexport intrinsic type None\n",
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

/// Parses + resolves + checks std files (beside the prelude) over the
/// prelude, returning every diagnostic message.
fn std_diags(std_files: &[(&str, &str)]) -> Vec<String> {
    std_diagnostics(std_files).into_iter().map(|d| d.message).collect()
}

/// [diag-id] The diagnostics themselves, ids and all.
fn std_diagnostics(std_files: &[(&str, &str)]) -> Vec<salvo_core::FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in std_files {
        let module = SourceSet::classify(Path::new(&format!("std/{name}"))).unwrap();
        sources.add(format!("std/{name}"), module, src.to_string(), true);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let mut out = resolution.errors.clone();
    let symbols = Symbols::collect(&program);
    let checked = check_program(&program, &resolution, &symbols);
    out.extend(checked.errors.iter().cloned());
    out
}

/// [mod-std-internal] std may import its own runtime module, by name. (That
/// a program may not is `check/exports/std_internal.sv`.)
#[test]
fn std_imports_its_own_runtime_module() {
    let runtime = "export fn tick_count() [] -> Int {\n    return 0\n}\n";
    let from_std = "import std.runtime.tick_count\n\nexport fn twice() [] -> Int {\n    return tick_count() + tick_count()\n}\n";
    let errs = std_diags(&[("runtime.sv", runtime), ("timing.sv", from_std)]);
    assert!(errs.is_empty(), "std may import its own module: {errs:?}");
}

/// [mod-use] A module-level `use` binds for every function of the module
/// without being declared — in the runtime module only. (That it is refused
/// anywhere else is `check/exports/std_internal.sv`.)
#[test]
fn a_module_level_use_is_the_runtime_module_s_alone() {
    let runtime = "effect Ticks {\n    fn tick() -> Int\n}\n\nhandler Ticking() of Ticks {\n    fn tick() -> Int {\n        return 1\n    }\n}\n\n\
                   use Ticking()\n\nexport fn twice() [] -> Int {\n    return tick() + tick()\n}\n";
    let errs = std_diags(&[("runtime.sv", runtime)]);
    assert!(errs.is_empty(), "the runtime module may bind at module level: {errs:?}");
}

/// [runtime-layers] The runtime's core may not import one of its services,
/// which are built on it; a service imports the core freely.
#[test]
fn the_runtime_core_does_not_import_its_services() {
    let service = "import std.runtime\n\nexport fn later() [] -> Int {\n    return now() + 1\n}\n";
    let core_ok = "export fn now() [] -> Int {\n    return 1\n}\n";
    let errs = std_diags(&[("runtime.sv", core_ok), ("runtime/timers.sv", service)]);
    assert!(errs.is_empty(), "{errs:?}");
    let core_bad = "import std.runtime.timers.later\n\nexport fn now() [] -> Int {\n    return 1\n}\n";
    let errs = std_diagnostics(&[("runtime.sv", core_bad), ("runtime/timers.sv", service)]);
    assert!(errs.iter().any(|d| d.id == Some("runtime-layers:core-imports-service")), "{errs:?}");
}
