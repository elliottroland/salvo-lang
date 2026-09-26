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

use std::path::Path;

use salvo_core::{check_program, resolve, Coercion, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`). Module `core.prelude`:
/// `core.*` is implicitly imported, so the test source sees these names.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Bool\nexport intrinsic type Char\nexport intrinsic type Str canbe Mut\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn mut_str(...parts: Str[]) [] -> Mut Str => parts\nexport intrinsic fn size(str: Str) [] -> Int => str\nexport intrinsic fn append(str: Mut Str, text: Str) [] -> None => str: Mut, text\n";

fn checked(src: &str) -> (Program, salvo_core::Checked) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
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
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let out = check_program(&program, &resolution, &symbols);
    (program, out)
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about legality.
    let mut diags = checked(src).1.errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

/// How many `Mut` drops the checker recorded in the user file, and what they
/// were dropped *from*.
fn drops(src: &str) -> Vec<String> {
    let (_, out) = checked(src);
    assert!(out.errors.iter().all(|d| !d.is_error()), "unexpected errors: {:?}", out.errors);
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
    let (_, out) = checked(src);
    assert!(out.errors.iter().all(|d| !d.is_error()), "unexpected errors: {:?}", out.errors);
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

// ===== the type =====

/// [type-canbe-mut] `Str canbe Mut`, so `Mut Str` is a type — where `Mut
/// Int` still is not.
#[test]
fn str_opts_into_mut() {
    assert!(messages(&body("    let b: Mut Str = mut_str()")).is_empty());
    let msgs = messages(&body("    let n: Mut Int = 1"));
    assert!(
        msgs.iter().any(|m| m.contains("`Mut` does not apply to `Int`")),
        "{msgs:?}"
    );
}

/// A `Str` literal is not a `Mut Str`: mutability is asked for, which is
/// what makes `mut_str` mirror `mut_list_of`.
#[test]
fn a_literal_is_not_a_builder() {
    let msgs = messages(&body("    let b: Mut Str = \"plain\""));
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(msgs[0].contains("Mut Str"), "{}", msgs[0]);
}

// ===== every drop site =====

/// A call argument — including one that reaches an intrinsic lowering, where
/// a missed conversion would hand `StringBuilder` to a `String` method.
#[test]
fn a_call_argument_drops_mut() {
    assert_eq!(
        drops(&body("    let b = mut_str()\n    let n = takes(b)")),
        vec!["Mut Str"]
    );
    assert_eq!(
        drops(&body("    let b = mut_str()\n    let n = size(b)")),
        vec!["Mut Str"]
    );
}

/// A `let` annotation, and a `return` — the two places a value leaves its
/// expression for a declared type.
#[test]
fn annotations_and_returns_drop_mut() {
    assert_eq!(
        drops(&body("    let plain: Str = mut_str()")),
        vec!["Mut Str"]
    );
    let src = "fn made() -> Str {\n    return mut_str()\n}\n";
    assert_eq!(drops(src), vec!["Mut Str"]);
}

/// A struct field.
#[test]
fn a_struct_field_drops_mut() {
    let src = "struct Label {\n    text: Str\n}\n\nfn probe() -> None {\n    \
               let l = Label {text: mut_str()}\n}\n";
    assert_eq!(drops(src), vec!["Mut Str"]);
}

/// A union arm: the value is wrapped as a *plain* `Str`, so the drop has to
/// happen first — and the wrap has to happen too, which is why a `DropMut`
/// carries a continuation.
#[test]
fn a_union_arm_drops_mut_and_keeps_the_wrap() {
    let src = "fn probe() -> None {\n    let u: Str | Int = mut_str()\n}\n";
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
        drops(&body("    let b = mut_str()\n    let s = \"${b}\"")),
        vec!["Mut Str"]
    );
}

/// Operators: equality especially, since a builder compares by identity
/// where a string compares by content — a live divergence between the two
/// targets if the drop were skipped.
#[test]
fn operators_drop_mut() {
    let src = body(
        "    let x = mut_str()\n    let y = mut_str()\n    let same = x == y",
    );
    assert_eq!(drops(&src), vec!["Mut Str", "Mut Str"]);
    // [op-arith] `+` on strings is refused outright (user decision
    // 2026-09-14): interpolation is the concatenation story, and the error
    // says so.
    let src = body("    let x = mut_str()\n    let joined = x + x");
    let msgs = messages(&src);
    assert!(
        msgs.iter()
            .any(|m| m.contains("does not concatenate strings") && m.contains("${")),
        "{msgs:?}"
    );
}

// ===== where nothing is dropped =====

/// A `Mut Str` parameter keeps it: the position asked for a builder.
#[test]
fn a_mut_position_keeps_mut() {
    let src = body("    let b = mut_str()\n    append(b, \"x\")");
    assert!(drops(&src).is_empty(), "{:?}", drops(&src));
}

/// An *optional* `Mut Str` keeps it too — a `Mut` arm anywhere in the
/// expected type means the value may stay a builder.
#[test]
fn an_optional_mut_position_keeps_mut() {
    let src = "fn probe() -> None {\n    let maybe: Mut Str? = mut_str()\n}\n";
    assert!(drops(src).is_empty(), "{:?}", drops(src));
}

/// A generic parameter keeps it: `T` binds to the argument's own type, so
/// nothing was widened. (`copy(b)` is the case that matters — a copy of a
/// builder is a builder.)
#[test]
fn a_generic_position_keeps_mut() {
    let src = format!(
        "fn keep<T>(value: T) [] -> None => value {{}}\n\n{}",
        body("    let b = mut_str()\n    keep(b)")
    );
    assert!(drops(&src).is_empty(), "{:?}", drops(&src));
}

/// A plain `Str` never records a drop — the coercion exists for the one
/// qualifier that is not erased.
#[test]
fn a_plain_str_records_nothing() {
    assert!(drops(&body("    let n = takes(\"plain\")")).is_empty());
}

// ===== interpolation renderability [interp-to-str] [interp-struct] =====

/// [interp-to-str] [backend-never-wrong] A value with no text form used to
/// pass the checker and become rustc's "doesn't implement Display". It is a
/// Salvo error now, naming the `to_str` that would fix it.
#[test]
fn interpolating_a_type_with_no_text_form_is_an_error() {
    let src = "struct Inner { n: Int }\n\
               struct Opaque { inner: Inner }\n\
               fn probe(o: Opaque) -> None => o {\n    let _s = \"${o}\"\n}\n";
    let msgs = messages(src);
    assert!(
        msgs.iter().any(|m| m.contains("has no text form")),
        "got {msgs:?}"
    );
}

/// [interp-to-str] A `to_str` in scope is resolved *at the interpolation
/// site* (user decision 2026-09-11), so declaring one is all it takes.
#[test]
fn a_to_str_in_scope_makes_a_type_interpolable() {
    let src = "struct Inner { n: Int }\n\
               struct Opaque { inner: Inner }\n\
               fn to_str(o: Opaque) [] -> Str => o { return \"op\" }\n\
               fn probe(o: Opaque) -> None => o {\n    let _s = \"${o}\"\n}\n";
    let msgs = messages(src);
    assert!(msgs.is_empty(), "expected a clean check, got {msgs:?}");
}

/// [interp-struct] A struct whose every field renders natively interpolates
/// with no declaration at all (user decision 2026-09-11).
#[test]
fn a_struct_of_native_fields_interpolates_by_default() {
    let src = "struct Person { name: Str, age: Int }\n\
               fn probe(p: Person) -> None => p {\n    let _s = \"${p}\"\n}\n";
    let msgs = messages(src);
    assert!(msgs.is_empty(), "expected a clean check, got {msgs:?}");
}

/// [interp-struct] The derivation is for the simple cases only: a field that
/// itself needs a `to_str` is not followed, and the diagnostic asks for one.
#[test]
fn a_struct_with_a_non_native_field_needs_its_own_to_str() {
    let src = "struct Inner { a: Int }\n\
               struct Outer { inner: Inner }\n\
               fn probe(o: Outer) -> None => o {\n    let _s = \"${o}\"\n}\n";
    let msgs = messages(src);
    assert!(
        msgs.iter().any(|m| m.contains("has no text form")),
        "got {msgs:?}"
    );
}
