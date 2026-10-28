//! Member-resolution rules [call-resolve] [struct-field-resolve] [col-index-resolve]
//! [iter-resolve], effects and handlers as non-data [effect-not-data]: what is
//! left in Rust after the move to test data (TESTS.md T1). The diagnostics
//! are `tests/check/members/`, over the real std; what stays here is
//!
//! - [diag-import-suggest] an unresolved call's import suggestions, which an
//!   annotation cannot say, over the real std (`salvo_testkit::front`);
//! - tests whose old diagnostic set holds more than one diagnostic for one
//!   mistake (an effect as a return type or a `let` annotation also reports
//!   the mismatch against it; a fn type declaring `spawn` is reported four
//!   times), kept as they were, over this file's prelude, until that is
//!   settled.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

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
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Str\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Opaque\n",
    // [col-no-literal] Arrays lost their literal syntax, so a test that wants
    // one builds it with the constructor.
    "export intrinsic fn array_of<T>(...elems: T[]) [] -> T[]\n",
    // [actor-spawn-expr] Std's process handle, whose type argument is an
    // *effect* — the one sanctioned exception to [effect-not-data], and the
    // checker keys on this declaration to grant it.
    "export intrinsic type Addr<E>\n",
    "export struct Mailbox { capacity: Int }\n",
    "export intrinsic type List<T> canbe Mut\n",
),
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
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    // Errors only: an unused-variable *warning* [var-unused] is a different
    // severity, and this file is about which references are rejected.
    errors.retain(|d| d.is_error());
    errors
}

fn messages(src: &str) -> Vec<String> {
    check_errors(src)
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

// ===== effects and handlers are not data [effect-not-data]
// ===== [handler-not-value] =====

const EFFECT_PRELUDE: &str = r#"

effect Counter {
    fn bump() -> Int
}

handler MemCounter of Counter {
    state {
        n: Int = 0
    }

    fn bump() -> Int {
        n = n + 1
        return n
    }
}

// [actor-effect-kind] An `Addr` needs a *process* protocol behind it, so the
// addr cases below have one to name; `Counter` stays plain, which is what the
// narrowness case needs.
actor effect Chatter {
    send fn say(what: Str) => !what
}
"#;

fn effect_messages(src: &str) -> Vec<String> {
    messages(&format!("{EFFECT_PRELUDE}\n{src}"))
}

/// [effect-not-data] An effect names a capability, not a type of values.
/// Every data position rejects it; the diagnostic names the two positions
/// that *do* accept an effect, and `use`.
#[test]
fn effect_types_are_rejected_in_data_positions() {
    let cases = [
        ("struct S {\n    c: Counter\n}\n", "struct field"),
        ("fn f(c: Counter) -> Int => c {\n    return 1\n}\n", "parameter"),
        ("fn f() -> Counter {\n    return 1\n}\n", "return type"),
        (
            "fn f() -> Int {\n    let c: Counter = 1\n    return 1\n}\n",
            "`let` annotation",
        ),
        ("type Alias = Counter\n", "type alias"),
    ];
    for (src, what) in cases {
        let errs = effect_messages(src);
        let diag = errs
            .iter()
            .find(|m| m.starts_with("`Counter` is an effect, not a data type"))
            .unwrap_or_else(|| panic!("no rejection for a {what}: {errs:?}"));
        assert!(
            diag.contains("`use`"),
            "the diagnostic should point at `use`: {diag}"
        );
    }
}

#[test]
fn a_fn_type_cannot_declare_the_spawn_capability() {
    let errors = messages(
        "\
fn run(f: (Int) [spawn] -> Int) -> Int {
    return f(1)
}
",
    );
    assert!(
        errors
            .iter()
            .any(|m| m.contains("fn type cannot declare `spawn`")),
        "expected the fn-type spawn refusal, got: {errors:?}"
    );
}

/// [call-resolve] [diag-import-suggest] Unresolved calls carry import
/// suggestions, like every other unresolved name.
#[test]
fn unresolved_call_suggests_imports() {
    let lib = "export fn helper() -> Int {\n    return 1\n}\n";
    let main = "fn f() -> Int {\n    return helper()\n}\n";
    let out = salvo_testkit::front::check(&[("lib.sv", lib), ("main.sv", main)]);
    let diag = out
        .checked
        .errors
        .iter()
        .find(|d| d.message.contains("no function named `helper`"))
        .unwrap_or_else(|| panic!("got {:?}", out.checked.errors));
    assert_eq!(diag.suggested_imports, vec!["lib.helper".to_string()]);
}
