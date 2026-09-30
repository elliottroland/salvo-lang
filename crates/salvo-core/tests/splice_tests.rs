//! [host-splice] Platform templates in the checker: what a marker may hold,
//! what it is checked against, and how a template must match its `.sv` file.

use std::path::{Path, PathBuf};

use salvo_core::template::TemplateFile;
use salvo_core::{check_program, resolve, ModulePath, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Bool\nexport intrinsic type Str\nexport intrinsic type List<T>\n\
export provenance qualifier Ok<T> of T\nexport provenance qualifier Err<T> of T\n\
export fn ok<T>(value: T) [] -> +Ok T {\n    return value\n}\n\
export fn err<T>(value: T) [] -> +Err T {\n    return value\n}\n";

/// Errors of `main.sv` with `platform/main.sv.kt` holding [kotlin].
fn errors(sv: &str, kotlin: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add("std/core/prelude.sv", SourceSet::classify(Path::new("core/prelude.sv")).unwrap(), STD_PRELUDE.to_string(), true);
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), sv.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(parse_errors.is_empty(), "parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let template = TemplateFile {
        rel_path: PathBuf::from("platform/main.sv.kt"),
        module: ModulePath(vec!["main".to_string()]),
        lang: "kotlin".to_string(),
        content: kotlin.to_string(),
        name: "platform/main.sv.kt".to_string(),
    };
    let mut out: Vec<String> = salvo_core::template::apply(&[template], &mut sources.files, &mut modules)
        .into_iter()
        .map(|d| d.message)
        .collect();
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program { files: sources.files, modules, companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    out.extend(checked.errors.iter().filter(|d| d.is_error()).map(|d| d.message.clone()));
    out
}

#[test]
fn markers_are_checked_against_their_types() {
    let sv = "struct P { n: Int }\nfn f(p: P) [] -> Int => p\n";
    assert_eq!(errors(sv, "`fn f(p: P) -> Int` {\n    return `p.n` + `P { n: @{3} }`.n\n}\n"), Vec::<String>::new());
    let errs = errors(sv, "`fn f(p: P) -> Int` {\n    return `p.n : Str`\n}\n");
    assert!(errs.iter().any(|e| e.contains("this hole is `Str`, and `Int` is not one")), "{errs:?}");
    assert!(!errors(sv, "`fn f(p: P) -> Int` {\n    return `nope`\n}\n").is_empty());
}

#[test]
fn a_template_matches_its_declarations() {
    let sv = "fn f(s: Str) [] -> Int => s\n";
    let errs = errors(sv, "`fn f(s: Int) -> Int` {\n    return 1\n}\n");
    assert!(errs.iter().any(|e| e.contains("does not declare without a body")), "{errs:?}");
    let errs = errors(sv, "`fn f(s: Str) -> Str` {\n    return 1\n}\n");
    assert!(errs.iter().any(|e| e.contains("does not match the declaration's")), "{errs:?}");
    let errs = errors(sv, "`fn g() -> Int` {\n    return 1\n}\n`fn f(s: Str) -> Int` { return 1 }\n");
    assert!(errs.iter().any(|e| e.contains("`fn g()`")), "{errs:?}");
    // A bodiless fn nothing implements.
    let errs = errors("fn h() [] -> Int\n", "");
    assert!(errs.iter().any(|e| e.contains("`fn h` has no body")), "{errs:?}");
}

#[test]
fn a_return_marker_takes_the_fn_return_type() {
    let sv = "fn f(n: Int) [] -> Ok Int | Err Str => n\n";
    assert_eq!(errors(sv, "`fn f(n: Int) -> Ok Int | Err Str` {\n    return `ok(n)`\n}\n"), Vec::<String>::new());
}

#[test]
fn typed_host_names_are_declared_redeclared_and_assigned() {
    let sv = "struct A { x: Int }\nstruct B { y: Int }\nfn f(xs: List<A>) [] -> Int => xs\n";
    let body = |inner: &str| format!("`fn f(xs: List<A>) -> Int` {{\n{inner}\n}}\n");
    assert_eq!(errors(sv, &body("return `xs`.map { `a : A` -> `a.x` }.sum()")), Vec::<String>::new());
    assert!(!errors(sv, &body("return `xs`.map { `a : A` -> `a.y` }.sum()")).is_empty(), "no field `y`");
    assert_eq!(errors(sv, &body("return `xs`.map { a -> `(@a : A).x` }.sum()")), Vec::<String>::new());
    assert!(!errors(sv, &body("return `xs`.map { a -> `(@a : A).y` }.sum()")).is_empty());
    assert_eq!(errors(sv, &body("fun one(`v : A`) = `v.x`\nfun two(`v : B`) = `v.y`\nreturn 0")), Vec::<String>::new());
    assert!(!errors(sv, &body("val `xs : A` = null\nreturn 0")).is_empty(), "a parameter is not redeclared");
    assert_eq!(errors(sv, &body("var `n : Int` = 0\n`n = 3`\nreturn n")), Vec::<String>::new());
    assert!(!errors(sv, &body("`m = 3`\nreturn 0")).is_empty(), "an undeclared target");
}

#[test]
fn a_platform_handler_template_implements_its_members() {
    let sv = "effect E {\n    fn go(n: Int) -> Int => n\n}\nplatform handler H(start: Int) of E {\n    at: Int = start\n}\n";
    let ok = "`platform handler H(start: Int) of E` {\n    `fn go(n: Int) -> Int` {\n        `at` += `n`\n        return `at`\n    }\n}\n";
    assert_eq!(errors(sv, ok), Vec::<String>::new());
    let errs = errors(sv, &ok.replace("fn go(n: Int)", "fn stop(n: Int)"));
    assert!(errs.iter().any(|e| e.contains("implements no member of `E`")), "{errs:?}");
    let errs = errors(sv, &ok.replace("H(start: Int)", "H(begin: Int)"));
    assert!(errs.iter().any(|e| e.contains("does not match the declaration's")), "{errs:?}");
    let sv_ts = sv.replace("platform handler", "threadsafe platform handler");
    let errs = errors(&sv_ts, &ok.replace("`platform handler", "`threadsafe platform handler"));
    assert!(errs.iter().any(|e| e.contains("cannot hold Salvo state yet")), "{errs:?}");
}

#[test]
fn strings_and_comments_are_not_scanned_and_double_backticks_escape() {
    let sv = "fn f(s: Str) [] -> Str => s\n";
    let kt = "// a `comment` with backticks\n`fn f(s: Str) -> Str` {\n    val ``in`` = \"a `string`\"\n    return `s` + ``in``\n}\n";
    assert_eq!(errors(sv, kt), Vec::<String>::new());
}

// [host-splice] `e : name` and `e : return`: a value takes the type of a place
// — a declared host name, a parameter, the fn's return type — so a union host
// code hides is written once (user decision 2026-09-30, option (a)). A place
// is lowercase, so `e : T` is still a type; a place nothing declares, or
// `: return` outside a fn, is an error, and the value is checked against the
// place's type.
#[test]
fn a_marker_ascribes_the_type_of_a_place() {
    let sv = "fn f(n: Int) [] -> Ok Int | Err Str => n\n".to_string();
    let body = |inner: &str| format!("`fn f(n: Int) -> Ok Int | Err Str` {{\n{inner}\n}}\n");
    // By a declared name, in both branches of host control flow.
    let branches = "val `answer : Ok Int | Err Str` = if (`n` > 0) `ok(n) : answer` else `err(\"neg\") : answer`\nreturn answer";
    assert_eq!(errors(&sv, &body(branches)), Vec::<String>::new());
    // By the fn's return type, and a declaration by a place.
    assert_eq!(errors(&sv, &body("val x = `ok(n) : return`\nreturn x")), Vec::<String>::new());
    assert_eq!(errors(&sv, &body("val `a : return` = `ok(n) : return`\nval `b : a` = a\nreturn `b`")), Vec::<String>::new());
    // By a parameter: `n` is an Int, and a Str is not one.
    let errs = errors(&sv, &body("val x = `\"s\" : n`\nreturn `ok(n)`"));
    assert!(errs.iter().any(|e| e.contains("this hole is `Int`")), "{errs:?}");
    // Nothing called `nope`.
    let errs = errors(&sv, &body("val x = `ok(n) : nope`\nreturn x"));
    assert!(errs.iter().any(|e| e.contains("no parameter, state field or declared name is called `nope`")), "{errs:?}");
    // `: return` where there is no fn.
    let errs = errors(&sv, &format!("{}\nval top = `ok(1) : return`\n", body("return `ok(n)`")));
    assert!(errs.iter().any(|e| e.contains("`: return` takes the enclosing fn's return type")), "{errs:?}");
}
