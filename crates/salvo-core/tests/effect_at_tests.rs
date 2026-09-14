//! [effect-member-overload] [effect-at] Two effects may declare the same
//! member name (user decision 2026-09-14, lifting the 2026-09-05 ban): a
//! bare call disambiguates by which effect has a handler in scope, and the
//! `member@Effect(…)` selector picks explicitly — including a generic
//! instance pinned by the call's type arguments
//! (`next_random@Random<Int>()`).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n";

fn errors(src: &str) -> Vec<String> {
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
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

fn assert_ok(src: &str) {
    let errs = errors(src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

/// Two effects sharing `close`, plus handlers and a `use`-heavy `main`.
const SHARED: &str = r#"
effect Fs {
    fn close(handle: Int) -> Str
}

effect Net {
    fn close(handle: Int) -> Str
}

handler MemFs of Fs {
    fn close(handle: Int) -> Str {
        return "fs"
    }
}

handler MemNet of Net {
    fn close(handle: Int) -> Str {
        return "net"
    }
}
"#;

/// A bare call resolves through the one effect that is *available*: `shut`
/// declares only `[Fs]`, so `close` is unambiguous there.
#[test]
fn a_bare_call_resolves_by_availability() {
    assert_ok(&format!(
        "{SHARED}\nfn shut(h: Int) [Fs] -> Str {{\n    return close(h)\n}}\n"
    ));
}

/// With both effects in scope the bare call is ambiguous, and the error
/// names the selector form.
#[test]
fn an_ambiguous_bare_call_names_the_selector() {
    let errs = errors(&format!(
        "{SHARED}\nfn both(h: Int) [Fs, Net] -> Str {{\n    return close(h)\n}}\n"
    ));
    assert!(
        errs.iter().any(|m| m.contains("`close` is a member of `Fs`, `Net`")
            && m.contains("close@Fs")),
        "got {errs:?}"
    );
}

/// [effect-at] The selector picks, both directions, in plain and dot form.
#[test]
fn the_selector_disambiguates() {
    assert_ok(&format!(
        "{SHARED}\nfn both(h: Int) [Fs, Net] -> Str {{\n    let a = close@Fs(h)\n    \
         let b = close@Net(h)\n    let c = h.close@Fs()\n    return \"${{a}}${{b}}${{c}}\"\n}}\n"
    ));
}

/// [effect-at] The named effect must exist, be in scope as an effect, and
/// declare the member; a selected member is not a value.
#[test]
fn selector_errors() {
    let errs = errors(&format!(
        "{SHARED}\nfn probe(h: Int) [Fs] -> None {{\n    close@Files(h)\n    \
         open@Fs(h)\n    let v = close@Fs\n}}\n"
    ));
    assert!(
        errs.iter().any(|m| m.contains("no effect named `Files`")),
        "got {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("effect `Fs` has no member named `open`")),
        "got {errs:?}"
    );
    assert!(
        errs.iter().any(|m| m.contains("not a value")),
        "got {errs:?}"
    );
}

/// [effect-at] The selected effect still needs a handler in scope — the
/// selector picks the effect, not a handler out of thin air.
#[test]
fn the_selector_still_needs_availability() {
    let errs = errors(&format!(
        "{SHARED}\nfn probe(h: Int) [Fs] -> None {{\n    close@Net(h)\n}}\n"
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("no handler for effect `Net`")),
        "got {errs:?}"
    );
}

/// [effect-at] [effect-disambiguation] The call's type arguments keep
/// their meaning under the selector: they pin the generic effect's
/// instance.
#[test]
fn the_selector_composes_with_instance_type_args() {
    let src = r#"
effect Random<T> {
    fn next_random() -> T
}

effect Dice {
    fn next_random() -> Int
}

fn draw() [Random<Int>, Random<Str>, Dice] -> Str {
    let n = next_random@Random<Int>()
    let s = next_random@Random<Str>()
    let d = next_random@Dice()
    return "${n}${s}${d}"
}
"#;
    assert_ok(src);
}

/// [effect-member-unique] Within one effect the duplicate stays an error.
#[test]
fn same_effect_duplicates_stay_errors() {
    let errs = errors(
        "effect Fs {\n    fn close(handle: Int) -> Str\n    fn close(handle: Str) -> Str\n}\n",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("effect `Fs` already declares a member named `close`")),
        "got {errs:?}"
    );
}
