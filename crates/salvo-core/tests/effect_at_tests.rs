//! [effect-member-overload] [effect-at] Two effects may declare the same
//! member name (user decision 2026-09-14, lifting the 2026-09-05 ban): a
//! bare call disambiguates by which effect has a handler in scope, and the
//! `member@Effect(…)` selector picks explicitly — including a generic
//! instance pinned by the call's type arguments
//! (`next_random@Random<Int>()`).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The declarations these sources rely on.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n";

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

/// [effect-member-overload] Within one effect a repeated name is an
/// **overload** (user decision 2026-09-14, §5.10.2 sub-question A): phase 4's
/// `Fs` declares `close` for each stream type, so the parameter types are
/// what tell them apart.
#[test]
fn same_effect_overloads_are_legal() {
    let errs = errors(
        "effect Fs {\n    fn close(handle: Int) -> Str\n    \
             fn close(handle: Str) -> Str => handle\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [effect-member-unique] What stays an error is a *duplicate*: the same name
/// taking the same types, which no call could ever tell apart.
#[test]
fn same_effect_duplicate_signatures_stay_errors() {
    let errs = errors(
        "effect Fs {\n    fn close(handle: Int) -> Str\n    fn close(handle: Int) -> Int\n}\n",
    );
    assert!(
        errs.iter().any(|m| m
            .contains("effect `Fs` already declares a member named `close` with these parameter types")),
        "got {errs:?}"
    );
}

// ===== overloading *within* one effect [effect-member-overload] =====

/// [effect-member-overload] The phase-4 shape: one effect, one member name,
/// two parameter types — `Fs` declares `close` for each stream token
/// (FILE_SYSTEM.md §5.10.2 sub-question A). The call picks by argument type,
/// exactly as a function overload does.
#[test]
fn a_member_overload_resolves_by_argument_type() {
    assert_ok(
        "struct InFile { id: Int }\n\
         struct OutFile { id: Int }\n\n\
         effect Fs {\n    \
             fn close(f: InFile) -> Str => !f\n    \
             fn close(f: OutFile) -> Int => !f\n}\n\n\
         fn shut(a: InFile, b: OutFile) [Fs] -> Str => !a, !b {\n    \
             let text = close(a)\n    \
             let code = close(b)\n    \
             return \"${text}${code}\"\n}\n",
    );
}

/// [effect-member-overload] [effect-at] The selector picks the *effect*; the
/// argument types still pick the overload inside it.
#[test]
fn the_effect_selector_composes_with_member_overloads() {
    assert_ok(
        "struct InFile { id: Int }\n\
         struct OutFile { id: Int }\n\n\
         effect Fs {\n    \
             fn close(f: InFile) -> Str => !f\n    \
             fn close(f: OutFile) -> Int => !f\n}\n\n\
         effect Net {\n    fn close(socket: Int) -> Str\n}\n\n\
         fn shut(a: OutFile) [Fs, Net] -> Str => !a {\n    \
             let code = close@Fs(a)\n    \
             let msg = close@Net(7)\n    \
             return \"${code}${msg}\"\n}\n",
    );
}

/// [effect-member-overload] An argument no overload takes is an error naming
/// the member and what was passed — not a silent pick of the first one
/// [backend-never-wrong].
#[test]
fn a_call_matching_no_member_overload_is_an_error() {
    let errs = errors(
        "struct InFile { id: Int }\n\
         struct OutFile { id: Int }\n\n\
         effect Fs {\n    \
             fn close(f: InFile) -> Str => !f\n    \
             fn close(f: OutFile) -> Int => !f\n}\n\n\
         fn shut() [Fs] -> Str {\n    return \"${close(7)}\"\n}\n",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("no overload of `Fs.close` takes (Int)")),
        "got {errs:?}"
    );
}

/// [effect-member-overload] Overloading by *arity* alone works too, and needs
/// no argument typing at all — the shape `position(s)` / `position(s, base)`
/// would take.
#[test]
fn member_overloads_may_differ_only_in_arity() {
    assert_ok(
        "effect Clock {\n    \
             fn now() -> Int\n    \
             fn now(offset: Int) -> Int\n}\n\n\
         fn stamp() [Clock] -> Int {\n    return now() + now(5)\n}\n",
    );
}

// ===== [effect-available] one overload set: members and fns compete =====
//
// User decision 2026-09-14 (option (a) of three): a name that is both an
// available effect's member and an ordinary fn resolves as *one* set, ranked
// by signature specificity. std needs it — `close` is an `Fs` member per
// stream token *and* the `Lines` pass's discharger — and so does anyone who
// writes a linear pass while a filesystem is in scope.

/// A `Store` with a `close` per handle type, and a module-level `close` of
/// its own. The argument types tell all three apart.
const ONE_SET: &str = r#"
struct Note { text: Str }

effect Store {
    fn close(handle: Int) -> Str
}

handler MemStore of Store {
    fn close(handle: Int) -> Str {
        return "store"
    }
}

fn close(n: Note) -> Str => !n {
    return n.text
}
"#;

/// The member and the fn coexist: `close(1)` is the member, `close(note)` is
/// the fn, in one scope with the handler registered.
#[test]
fn a_member_and_a_fn_of_one_name_resolve_by_argument_type() {
    assert_ok(&format!(
        "{ONE_SET}\nfn work(n: Note) [Store] -> Str => !n {{\n    \
         let a = close(1)\n    return \"${{a}}${{close(n)}}\"\n}}\n"
    ));
}

/// The fn is reachable where it used to be shadowed outright: before the
/// rule, an available `Store` claimed the name and the call was an error
/// naming `Store.close`'s overloads.
#[test]
fn the_fn_side_is_reachable_while_the_effect_is_available() {
    let errs = errors(&format!(
        "{ONE_SET}\nfn work(n: Note) [Store] -> Str => !n {{\n    return close(n)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

/// A **more specific fn** wins over a generic member: specificity decides,
/// exactly as it does between two fn overloads [fn-overload-rank].
#[test]
fn the_more_specific_signature_wins_across_the_two_kinds() {
    assert_ok(
        "effect Sink {\n    fn keep<T>(x: T) -> Str => x\n}\n\n\
         handler Bin of Sink {\n    fn keep<T>(x: T) -> Str => x {\n        \
         return \"any\"\n    }\n}\n\n\
         fn keep(x: Int) -> Str => x {\n    return \"int\"\n}\n\n\
         fn use_it() [Sink] -> Str {\n    return keep(1)\n}\n",
    );
}

/// A genuine tie — same parameter type on both sides — is an error naming
/// both remedies rather than a silent preference.
#[test]
fn an_equally_specific_member_and_fn_are_ambiguous() {
    let errs = errors(
        "effect Store {\n    fn close(handle: Int) -> Str\n}\n\n\
         handler MemStore of Store {\n    fn close(handle: Int) -> Str {\n        \
         return \"store\"\n    }\n}\n\n\
         fn close(handle: Int) -> Str => handle {\n    return \"fn\"\n}\n\n\
         fn work() [Store] -> Str {\n    return close(1)\n}\n",
    );
    assert!(
        errs.iter().any(|m| {
            m.contains("ambiguous call to `close`")
                && m.contains("close@Store")
                && m.contains("close@main")
        }),
        "got {errs:?}"
    );
}

/// The selectors settle it by hand, each picking its own side: `@Effect` the
/// member, `@module` the fn — and `@module` skips the member set whole, which
/// is what makes a shadowed fn callable at all.
#[test]
fn the_selectors_pick_a_side() {
    assert_ok(
        "effect Store {\n    fn close(handle: Int) -> Str\n}\n\n\
         handler MemStore of Store {\n    fn close(handle: Int) -> Str {\n        \
         return \"store\"\n    }\n}\n\n\
         fn close(handle: Int) -> Str => handle {\n    return \"fn\"\n}\n\n\
         fn work() [Store] -> Str {\n    \
         let a = close@Store(1)\n    let b = close@main(1)\n    return \"${a}${b}\"\n}\n",
    );
}

/// Neither side accepting the arguments is **one** diagnostic listing both:
/// to the caller they are one name.
#[test]
fn a_call_that_fits_neither_side_names_both() {
    let errs = errors(&format!(
        "{ONE_SET}\nfn work() [Store] -> Str {{\n    return close(\"nope\")\n}}\n"
    ));
    assert!(
        errs.iter().any(|m| {
            m.contains("no `close` accepts (Str)")
                && m.contains("`Store.close(Int)`")
                && m.contains("`close(Note)`")
        }),
        "got {errs:?}"
    );
}

/// [effect-available] Availability still decides first: with no handler in
/// scope the member set is not a candidate at all, so the fn answers.
#[test]
fn without_a_handler_the_fn_answers() {
    assert_ok(&format!(
        "{ONE_SET}\nfn work(n: Note) -> Str => !n {{\n    return close(n)\n}}\n"
    ));
}
