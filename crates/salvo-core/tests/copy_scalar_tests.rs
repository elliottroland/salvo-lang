//! [copy-scalar-free] Moving a Copy scalar is a copy (user decision
//! 2026-10-20, ROADMAP §2d item 12): every consuming position — a call's
//! moved parameter, a literal store, a spread — leaves an `Int` usable,
//! judged by the argument's own type, so a generic `T` bound to an `Int`
//! moves for free too. A written `!v` on a concrete scalar parameter warns:
//! it says nothing.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Long\nexport intrinsic type Str\n\
     export intrinsic type Bool\nexport intrinsic type Char\nexport intrinsic type Double\nexport intrinsic type Never\n";

/// The checker's diagnostics over one file (and a scalar-only prelude):
/// `(errors, warnings)`.
fn diagnostics(src: &str) -> (Vec<String>, Vec<String>) {
    let mut sources = SourceSet::default();
    sources.add("std/core/prelude.sv", SourceSet::classify(Path::new("core/prelude.sv")).unwrap(), STD_PRELUDE.to_string(), true);
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), src.to_string(), false);
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(parse_errors.is_empty(), "parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program { files: sources.files, modules, companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut all: Vec<_> = resolution.errors.iter().chain(checked.errors.iter()).cloned().collect();
    // [flow-use-after-move] As the gates do: the IR analyses (a use after a
    // move among them) on every declaration the checker's errors leave clean
    // [ir-analyze].
    all.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked));
    let (errs, warns): (Vec<_>, Vec<_>) = all.into_iter().partition(|d| d.is_error());
    (errs.into_iter().map(|d| d.message).collect(), warns.into_iter().map(|d| d.message).collect())
}

/// A generic moved parameter called with each Copy scalar, twice: the
/// argument stays usable, because its own type is a scalar.
#[test]
fn a_scalar_handed_to_a_generic_moved_parameter_stays_usable() {
    let src = "fn take<T>(v: T) [] -> None => !v {\n}\n\n\
               fn f(i: Int, l: Long, b: Bool, c: Char, d: Double) [] -> Int => i, l, b, c, d {\n    take(i)\n    take(i)\n    take(l)\n    take(l)\n    take(b)\n    take(b)\n    take(c)\n    take(c)\n    take(d)\n    take(d)\n    let x: Int = 1\n    take(x)\n    return x + i\n}\n";
    let (errs, warns) = diagnostics(src);
    assert!(errs.is_empty(), "{errs:?}");
    assert!(warns.is_empty(), "a generic `!v` says something: {warns:?}");
}

/// A literal store and a written `!v` on an `Int` parameter leave the value
/// usable; the `!v` warns that it changes nothing.
#[test]
fn a_stored_or_moved_int_stays_usable_and_a_written_move_warns() {
    let src = "struct P {\n    x: Int\n}\n\n\
               fn eat(v: Int) [] -> None => !v {\n}\n\n\
               fn f(n: Int) [] -> Int => n {\n    let p = P { x: n }\n    let q = P { x: n }\n    eat(n)\n    eat(n)\n    return n + p.x + q.x\n}\n";
    let (errs, warns) = diagnostics(src);
    assert!(errs.is_empty(), "{errs:?}");
    assert!(
        warns.iter().any(|w| w.contains("moving an `Int` is a copy, so `!v` changes nothing for the caller: drop it [copy-scalar-free]")),
        "{warns:?}"
    );
}

/// Anything that is not a Copy scalar is still consumed: a `Str` handed to a
/// moved parameter cannot be read afterwards.
#[test]
fn a_str_handed_to_a_moved_parameter_is_consumed() {
    let src = "fn eat<T>(v: T) [] -> None => !v {\n}\n\n\
               fn f(s: Str) [] -> None => !s {\n    eat(s)\n    eat(s)\n}\n";
    let (errs, _) = diagnostics(src);
    assert!(errs.iter().any(|e| e.contains("`s` cannot be used here: it was consumed (moved) at line 5")), "{errs:?}");
}
