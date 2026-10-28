//! [str-drop-mut] [type-canbe-mut] `Str canbe Mut`, and the coercion that
//! makes a `Mut Str` usable as a `Str`.
//!
//! `Mut` is the one qualifier a backend may render as a *different type*
//! (Kotlin's `Mut Str` is a `StringBuilder`, which is not a `String`), so
//! dropping it is a real conversion rather than the free widening every
//! other qualifier gets [qual-erasure]. The checker therefore *records* the
//! drop instead of leaving the emitters to guess — the standing
//! checker/emitter agreement invariant — and it has to record it at every
//! site where a value flows into a plain-`Str` position.
//!
//! The diagnostics are test data, `tests/check/strings/` (TESTS.md T1); the
//! drop tests here read the checker's table, over the real std. One
//! diagnostic test stays on its old prelude, and says why.

use std::path::Path;

use salvo_core::{check_program, resolve, Coercion, Program, SourceSet, Symbols};

/// `src` checked over the real std, asserting it reports no error, with
/// only the rows of its own file kept (std's are seeded into every check).
fn checked(src: &str) -> salvo_core::Checked {
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let mut c = out.checked;
    c.coerce.retain(|k, _| k.file() >= user);
    c
}

/// How many `Mut` drops the checker recorded in the user file, and what they
/// were dropped *from*.
fn drops(src: &str) -> Vec<String> {
    let out = checked(src);
    let mut found: Vec<String> = out
        .coerce
        .values()
        .filter_map(|c| match c {
            Coercion::DropMut { from, .. } => Some(from.to_string()),
            _ => None,
        })
        .collect();
    found.sort();
    found
}

/// The coercion a `DropMut` carries with it, if any — one expression has one
/// coercion slot, and both changes have to happen.
fn drop_continuations(src: &str) -> Vec<String> {
    let out = checked(src);
    out.coerce
        .values()
        .filter_map(|c| match c {
            Coercion::DropMut { then, .. } => {
                Some(then.as_ref().map(|t| format!("{t:?}")).unwrap_or_default())
            }
            _ => None,
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn body(stmts: &str) -> String {
    format!("fn takes(text: Str) -> Int => text {{\n    return size(text)\n}}\n\nfn probe() -> None {{\n{stmts}\n}}\n")
}

// ===== every drop site =====

/// A call argument — including one that reaches an intrinsic lowering, where
/// a missed conversion would hand `StringBuilder` to a `String` method.
#[test]
fn a_call_argument_drops_mut() {
    assert_eq!(
        drops(&body("    let b = Mut Str()\n    let n = takes(b)")),
        vec!["Mut Str"]
    );
    assert_eq!(
        drops(&body("    let b = Mut Str()\n    let n = size(b)")),
        vec!["Mut Str"]
    );
}

/// A `let` annotation, and a `return` — the two places a value leaves its
/// expression for a declared type.
#[test]
fn annotations_and_returns_drop_mut() {
    assert_eq!(
        drops(&body("    let plain: Str = Mut Str()")),
        vec!["Mut Str"]
    );
    let src = "fn made() -> Str {\n    return Mut Str()\n}\n";
    assert_eq!(drops(src), vec!["Mut Str"]);
}

/// A struct field.
#[test]
fn a_struct_field_drops_mut() {
    let src = "struct Label {\n    text: Str\n}\n\nfn probe() -> None {\n    \
               let l = Label {text: Mut Str()}\n}\n";
    assert_eq!(drops(src), vec!["Mut Str"]);
}

/// A union arm: the value is wrapped as a *plain* `Str`, so the drop has to
/// happen first — and the wrap has to happen too, which is why a `DropMut`
/// carries a continuation.
#[test]
fn a_union_arm_drops_mut_and_keeps_the_wrap() {
    let src = "fn probe() -> None {\n    let u: Str | Int = Mut Str()\n}\n";
    assert_eq!(drops(src), vec!["Mut Str"]);
    let carried = drop_continuations(src);
    assert_eq!(carried.len(), 1, "{carried:?}");
    assert!(
        carried[0].contains("WrapUnion"),
        "the union wrap must survive the drop: {}",
        carried[0]
    );
}

/// Interpolation reads a value's *text*.
#[test]
fn interpolation_drops_mut() {
    assert_eq!(
        drops(&body("    let b = Mut Str()\n    let s = \"${b}\"")),
        vec!["Mut Str"]
    );
}

/// Operators: equality especially, since a builder compares by identity
/// where a string compares by content — a live divergence between the two
/// targets if the drop were skipped.
#[test]
fn operators_drop_mut() {
    let src = body(
        "    let x = Mut Str()\n    let y = Mut Str()\n    let same = x == y",
    );
    assert_eq!(drops(&src), vec!["Mut Str", "Mut Str"]);
}

// ===== where nothing is dropped =====

/// A `Mut Str` parameter keeps it: the position asked for a builder.
#[test]
fn a_mut_position_keeps_mut() {
    let src = body("    let b = Mut Str()\n    append(b, \"x\")");
    assert!(drops(&src).is_empty(), "{:?}", drops(&src));
}

/// An *optional* `Mut Str` keeps it too — a `Mut` arm anywhere in the
/// expected type means the value may stay a builder.
#[test]
fn an_optional_mut_position_keeps_mut() {
    let src = "fn probe() -> None {\n    let maybe: Mut Str? = Mut Str()\n}\n";
    assert!(drops(src).is_empty(), "{:?}", drops(src));
}

/// A generic parameter keeps it: `T` binds to the argument's own type, so
/// nothing was widened. (`copy(b)` is the case that matters — a copy of a
/// builder is a builder.)
#[test]
fn a_generic_position_keeps_mut() {
    let src = format!(
        "fn keep<T>(value: T) [] -> None => value {{}}\n\n{}",
        body("    let b = Mut Str()\n    keep(b)")
    );
    assert!(drops(&src).is_empty(), "{:?}", drops(&src));
}

/// A plain `Str` never records a drop — the coercion exists for the one
/// qualifier that is not erased.
#[test]
fn a_plain_str_records_nothing() {
    assert!(drops(&body("    let n = takes(\"plain\")")).is_empty());
}

// ===== the type =====

/// [type-canbe-mut] `Str canbe Mut`, so `Mut Str` is a type — where `Mut
/// Int` still is not.
//
// Kept on the old prelude: over the real std, `let n: Mut Int = 1` reports
// `struct-mut:not-canbe-mut` and then `var-let-infer:annotation-mismatch`
// ("expected `Mut Int`, found `Int`"), a second diagnostic for one mistake
// [type-unknown-lenient], which a data file would pin. The `Mut Str` half is
// `strings/strings.sv`'s `opts_into_mut`.
#[test]
fn str_opts_into_mut() {
    assert!(prelude_errors(&body("    let b: Mut Str = mut_str()")).is_empty());
    let ids = prelude_errors(&body("    let n: Mut Int = 1"));
    assert!(ids.contains(&"struct-mut:not-canbe-mut"), "{ids:?}");
}

// ===== the old prelude, for `str_opts_into_mut` =====

/// [platform-intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`). Module `std.core.prelude`:
/// `std.core.*` is implicitly imported, so the test source sees these names.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Bool\nexport intrinsic type Char\nexport intrinsic type Str canbe Mut\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn mut_str(...parts: Str[]) [] -> Mut Str => parts\nexport intrinsic fn size(str: Str) [] -> Int => str\nexport intrinsic fn append(str: Mut Str, text: Str) [] -> None => str: Mut, text\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
);

fn prelude_checked(src: &str) -> (Program, salvo_core::Checked) {
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
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let out = check_program(&program, &resolution, &symbols);
    (program, out)
}

/// [diag-id] The ids of the errors of `src` over the old prelude, for the
/// one test kept on it.
fn prelude_errors(src: &str) -> Vec<&'static str> {
    prelude_checked(src).1.errors.iter().filter(|d| d.is_error()).map(|d| d.id.unwrap_or("?")).collect()
}
