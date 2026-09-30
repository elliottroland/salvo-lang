//! [type-literal] [union-arm-identity] Unions of literals in the checker: which
//! values they accept, how `is` narrows them, what is refused, and the runtime
//! tables a backend reads (user decisions 2026-09-30).

use std::path::Path;

use salvo_core::{check_program, resolve, Coercion, FileDiagnostic, Program, SourceSet, Symbols};
use salvo_syntax::ast::TypeLit;

/// The declarations these sources rely on, loaded as a *std* file: the base
/// types, and `core.other`'s qualifier and constructor.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Long\nexport intrinsic type Bool\nexport intrinsic type Str\n\
export provenance qualifier Other<T> of T\nexport fn other<T>(value: T) [] -> +Other T {\n    return value\n}\n";

fn checked(src: &str) -> (Program, salvo_core::Checked) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), src.to_string(), false);
    let mut modules = Vec::new();
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
    let out = check_program(&program, &resolution, &symbols);
    (program, out)
}

fn diagnostics(src: &str, errors: bool) -> Vec<String> {
    let (_, out) = checked(src);
    out.errors
        .iter()
        .filter(|d: &&FileDiagnostic| d.is_error() == errors)
        .map(|d| d.message.clone())
        .collect()
}

fn errors(src: &str) -> Vec<String> {
    diagnostics(src, true)
}

const CLASS: &str = "type Class = \"STANDARD\" | \"GLACIER\" | Other Str\n";

#[test]
fn literals_take_their_type_from_the_expected_type_and_narrow() {
    let src = format!(
        "{CLASS}
fn describe(c: Class) [] -> Int => c {{
    when c {{
        is \"STANDARD\" {{ return 1 }}
        is \"GLACIER\" {{ return 2 }}
        is Other {{ return 3 }}
    }}
}}

fn widen(s: Str) [] -> Str => !s {{ return s }}

fn uses() [] -> Int {{
    let a: Class = \"STANDARD\"
    let later: Class? = None
    later = \"GLACIER\"
    let untyped = \"STANDARD\"
    let w = widen(a)
    if later is None {{
        return 0
    }}
    return describe(later) + describe(\"GLACIER\") + describe(other(\"DEEP\"))
}}
"
    );
    assert_eq!(errors(&src), Vec::<String>::new());
}

#[test]
fn a_literal_the_union_does_not_list_is_refused_by_name() {
    let src = format!("{CLASS}fn f() [] -> None {{\n    let c: Class = \"STANDRD\"\n}}\n");
    let errs = errors(&src);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("`\"STANDRD\"` is not one of the values"), "{errs:?}");
    assert!(errs[0].contains("other(\"STANDRD\")"), "{errs:?}");
}

#[test]
fn a_plain_value_must_be_narrowed_or_wrapped() {
    // Open or closed, a `Str` is not one of the literals until narrowed —
    // only `other(…)` reaches the open arm (the user's rule).
    let src = format!(
        "{CLASS}type Closed = \"A\" | \"B\"
fn f(s: Str) [] -> None => s {{
    let open: Class = s
    let closed: Closed = s
}}
"
    );
    let errs = errors(&src);
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(errs[0].contains("narrow it to one of") && errs[0].contains("other(…)"), "{errs:?}");
    assert!(errs[1].contains("narrow it to one of") && !errs[1].contains("other("), "{errs:?}");
}

#[test]
fn a_when_over_literals_is_exhaustive() {
    let src = "type Closed = \"A\" | \"B\"\nfn f(c: Closed) [] -> Int => c {\n    when c {\n        is \"A\" { return 1 }\n    }\n}\n";
    let errs = errors(src);
    assert!(errs.iter().any(|e| e.contains("unhandled union arm \"B\"")), "{errs:?}");
}

#[test]
fn a_literal_beside_its_bare_base_is_a_warning() {
    let warnings = diagnostics("type Sub = \"a\" | Str\n", false);
    assert!(warnings.iter().any(|w| w.contains("is subsumed by the `Str` arm")), "{warnings:?}");
}

#[test]
fn overloads_on_literals_of_one_base_are_refused() {
    let src = "fn pick(x: \"A\") [] -> Int => x { return 1 }\nfn pick(x: \"B\") [] -> Int => x { return 2 }\n";
    let errs = errors(src);
    assert!(errs.iter().any(|e| e.contains("overloaded on literal types of one base")), "{errs:?}");
}

#[test]
fn the_backend_tables_are_the_runtime_arms() {
    // `is "A"` on an open union is a value test on a plain string (no
    // wrapper); in a mixed union the two `Str` literals share runtime arm 0.
    let src = format!(
        "{CLASS}struct P {{ n: Int }}
type Mixed = \"name\" | 3 | \"other\" | P
fn f(c: Class) [] -> Bool => c {{
    return c is \"GLACIER\"
}}
fn g() [] -> Mixed {{
    return \"other\"
}}
"
    );
    let (_, out) = checked(&src);
    assert!(out.errors.iter().all(|e| !e.is_error()), "{:?}", out.errors);
    let test = out
        .is_tests
        .values()
        .find(|t| !t.values.is_empty())
        .expect("a literal test");
    assert_eq!(test.size, 1);
    assert_eq!(test.values, vec![(0, false, vec![TypeLit::Str("GLACIER".into())])]);
    let wrap = out
        .coerce
        .values()
        .find_map(|c| match c {
            Coercion::WrapUnion { target, arm, .. } => Some((target.clone(), *arm)),
            _ => None,
        })
        .expect("a wrap into the mixed union");
    assert_eq!(wrap.1, 0, "\"other\" shares the first Str arm");
    assert_eq!(wrap.0.to_string(), "Str | Int | P");
}
