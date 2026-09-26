//! [mod-import-module] The whole-module import: `import time` brings every
//! name in a module into scope, at a rung below a named import and above
//! implicit `core.*` visibility (user decision 2026-09-18).
//!
//! The rule's point is that a bulk import never *fights* anything: it loses
//! silently to an own declaration and to a named import, functions resolve
//! through the ordinary ladder with `@module` as the tie-break
//! [fn-overload-scope], and only two bulk imports carrying one *type* name
//! produce a diagnostic — a warning, since there is no `@` form for a type.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// Parses + resolves a multi-file program (no std) and returns the
/// resolver's diagnostics as rendered messages, errors and warnings alike.
fn resolve_diags(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    resolve(&program)
        .errors
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// The base types the checker needs, as a std file [intrinsic-std-only].
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n";

/// Parses + resolves + checks, returning every diagnostic (both severities).
fn check_diags(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    let mut out: Vec<FileDiagnostic> = resolution.errors.clone();
    out.extend(check_program(&program, &resolution, &symbols).errors);
    out
}

fn errors(diags: &[FileDiagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

// [mod-import-module] One line brings every name in the module: a struct, an
// effect and a free fn, none of them named in the import.
#[test]
fn a_module_import_brings_every_name() {
    let lib = "export struct Span {\n    nanos: Long\n}\n\n\
               export effect Ticking {\n    fn tick() -> Long\n}\n\n\
               export fn millis(n: Long) [] -> Span {\n    return Span {nanos: n}\n}\n";
    let main = "import time\n\n\
                fn use_it() [Ticking] -> Span {\n    \
                let now = tick()\n    return millis(now)\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert!(
        errors(&diags).is_empty(),
        "expected a clean program, got: {:?}",
        errors(&diags)
    );
}

// [mod-import-module] The sweep follows the *prefix*, like the name form
// (`import core.Str` finds `core.string`): `import time` covers `time.clock`
// and `time.timer` too.
#[test]
fn a_module_import_covers_submodules() {
    // Each file names only its own module's declarations [mod-visibility] —
    // the point under test is that *main* sees both modules from one import.
    let types = "export struct Span {\n    nanos: Int\n}\n";
    let clock = "export fn seconds(n: Int) [] -> Int {\n    return n * 1000\n}\n";
    let main = "import time\n\nfn f() [] -> Span {\n    return Span {nanos: seconds(2)}\n}\n";
    let diags = check_diags(&[
        ("time/types.sv", types),
        ("time/clock.sv", clock),
        ("main.sv", main),
    ]);
    assert!(
        errors(&diags).is_empty(),
        "expected submodules to be swept in, got: {:?}",
        errors(&diags)
    );
}

// [mod-import-module] An own declaration wins over a bulk import, silently:
// a convenience import must not break a file that declares its own `Span`.
#[test]
fn an_own_declaration_beats_a_module_import_silently() {
    let lib = "export struct Span {\n    nanos: Long\n}\n";
    let main = "import time\n\nstruct Span {\n    label: Str\n}\n\n\
                fn f() [] -> Span {\n    return Span {label: \"mine\"}\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert!(
        diags.is_empty(),
        "expected no diagnostics at all, got: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

// [mod-import-module] A *named* import outranks a bulk one, also silently —
// which is how a program picks a specific module's type when two carry the
// name.
#[test]
fn a_named_import_beats_a_module_import_silently() {
    let a = "export struct Span {\n    nanos: Long\n}\n";
    let b = "export struct Span {\n    label: Str\n}\n";
    let main = "import time\nimport other.Span\n\n\
                fn f() [] -> Span {\n    return Span {label: \"b\"}\n}\n";
    let diags = check_diags(&[("time.sv", a), ("other.sv", b), ("main.sv", main)]);
    assert!(
        errors(&diags).is_empty(),
        "expected the named import to win, got: {:?}",
        errors(&diags)
    );
}

// [mod-import-module] Two bulk imports carrying one type name: the first
// wins and the second warns, because a type has no `@module` form to
// disambiguate with — the remedy the message names is a named import.
#[test]
fn two_module_imports_of_one_type_name_warn() {
    let a = "export struct Span {\n    nanos: Long\n}\n";
    let b = "export struct Span {\n    label: Str\n}\n";
    let main = "import time\nimport other\n\nfn f() [] -> Long {\n    return 1\n}\n";
    let diags = resolve_diags(&[("time.sv", a), ("other.sv", b), ("main.sv", main)]);
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic: {diags:?}");
    assert!(
        diags[0].contains("Span")
            && diags[0].contains("two whole-module imports")
            && diags[0].contains("`time`")
            && diags[0].contains("`other`"),
        "expected the ambiguity warning, got: {}",
        diags[0]
    );
}

// [mod-import-module] [fn-overload-ambiguous] A bulk-imported overload does
// **not** rank below this module's: both fit, nothing separates them by
// signature, so the call names the place it means (user decision 2026-09-26).
#[test]
fn a_module_import_competes_with_the_own_module() {
    let lib = "export fn label(n: Int) [] -> Str {\n    return \"lib\"\n}\n";
    let main = "import time\n\n\
                fn label(n: Int) [] -> Str {\n    return \"mine\"\n}\n\n\
                fn pick() [] -> Str {\n    return label(1)\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert!(
        errors(&diags)
            .iter()
            .any(|e| e.contains("ambiguous call to `label(Int)`")
                && e.contains("label@time(...)")
                && e.contains("label@main(...)")),
        "got: {:?}",
        errors(&diags)
    );
    // Either selector settles it.
    let main = "import time\n\n\
                fn label(n: Int) [] -> Str {\n    return \"mine\"\n}\n\n\
                fn pick() [] -> Str {\n    return label@main(1)\n}\n\n\
                fn pick_theirs() [] -> Str {\n    return label@time(1)\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert!(
        errors(&diags).is_empty(),
        "expected both calls to resolve, got: {:?}",
        errors(&diags)
    );
}

// [mod-import-module] A module import has no alias: there is nothing to
// qualify a renamed module with.
#[test]
fn a_module_import_rejects_an_alias() {
    let lib = "export struct Span {\n    nanos: Long\n}\n";
    let main = "import time as t\n\nfn f() [] -> Long {\n    return 1\n}\n";
    let diags = resolve_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert_eq!(diags.len(), 1, "expected one diagnostic: {diags:?}");
    assert!(
        diags[0].contains("whole-module import has no alias"),
        "expected the alias refusal, got: {}",
        diags[0]
    );
}

// [mod-import-module] [mod-visibility] Importing `core` is redundant, and
// saying so is better than adding every core name at a second rung (which
// would double every core overload).
#[test]
fn importing_core_is_reported_as_redundant() {
    let core = "export struct Span {\n    nanos: Long\n}\n";
    let main = "import core\n\nfn f() [] -> Long {\n    return 1\n}\n";
    let diags = resolve_diags(&[("core/span.sv", core), ("main.sv", main)]);
    assert_eq!(diags.len(), 1, "expected one diagnostic: {diags:?}");
    assert!(
        diags[0].contains("redundant import"),
        "expected the redundancy warning, got: {}",
        diags[0]
    );
}

// [mod-import-module] An unknown module says so, and lists what there is —
// rather than reporting the old "import path must be `module.item`" shape
// error, which described a form the writer did not intend.
#[test]
fn an_unknown_module_import_names_the_candidates() {
    let lib = "export struct Span {\n    nanos: Long\n}\n";
    let main = "import tyme\n\nfn f() [] -> Long {\n    return 1\n}\n";
    let diags = resolve_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert_eq!(diags.len(), 1, "expected one diagnostic: {diags:?}");
    assert!(
        diags[0].contains("unknown module `tyme`") && diags[0].contains("time"),
        "expected the unknown-module error with candidates, got: {}",
        diags[0]
    );
}

// [mod-import-module] The name form still works, and still takes precedence
// as a *reading*: `import time.Span` is one name, not the module.
#[test]
fn the_name_form_is_unaffected() {
    let lib = "export struct Span {\n    nanos: Long\n}\n\n\
               export struct Other {\n    v: Long\n}\n";
    let main = "import time.Span\n\nfn f() [] -> Span {\n    return Span {nanos: 1}\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main)]);
    assert!(
        errors(&diags).is_empty(),
        "expected the named import to work, got: {:?}",
        errors(&diags)
    );
    // ...and it brings *only* that name: `Other` stays invisible.
    let main_other = "import time.Span\n\nfn f() [] -> Other {\n    return Other {v: 1}\n}\n";
    let diags = check_diags(&[("time.sv", lib), ("main.sv", main_other)]);
    assert!(
        errors(&diags).iter().any(|m| m.contains("Other")),
        "expected `Other` to be unresolved, got: {:?}",
        errors(&diags)
    );
}
