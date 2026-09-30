//! [host-splice] Host code in Salvo files, in the checker: what a hole may
//! hold, what it is checked against, and what a platform handler written in
//! place must have.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Bool\nexport intrinsic type Str\nexport intrinsic type List<T>\n";

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add("std/core/prelude.sv", SourceSet::classify(Path::new("core/prelude.sv")).unwrap(), STD_PRELUDE.to_string(), true);
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
    out.errors.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
}

#[test]
fn holes_are_checked_against_their_types() {
    let ok = "struct P { n: Int }\nfn f(p: P) [] -> Int => p\n```kotlin\nreturn @{p.n} + @{ P { n: `3` } }.n\n```\n";
    assert_eq!(errors(ok), Vec::<String>::new());
    let wrong = "struct P { n: Int }\nfn f(p: P) [] -> Int => p\n```kotlin\nreturn @{ p.n : Str }\n```\n";
    let errs = errors(wrong);
    assert!(errs.iter().any(|e| e.contains("this hole is `Str`, and `Int` is not one")), "{errs:?}");
    let unknown = "fn f() [] -> Int\n```kotlin\nreturn @{nope}\n```\n";
    assert!(!errors(unknown).is_empty());
}

#[test]
fn host_code_stands_only_in_a_hole() {
    let errs = errors("fn f() [] -> Int {\n    return `1`\n}\n");
    assert!(errs.iter().any(|e| e.contains("only a `@{…}` hole in a host block may hold")), "{errs:?}");
}

#[test]
fn a_host_body_names_a_backend_once_and_writes_its_clause() {
    let errs = errors("fn f() [] -> Int\n```swift\nreturn 1\n```\n```kotlin\nreturn 1\n```\n```kotlin\nreturn 2\n```\n");
    assert!(errs.iter().any(|e| e.contains("no backend is called `swift`")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("has two ```kotlin blocks")), "{errs:?}");
    // The body is opaque, so the clause must say what happens to `s`.
    let errs = errors("fn g(s: Str) [] -> Int\n```kotlin\nreturn 1\n```\n");
    assert!(!errs.is_empty(), "a host body needs its deduction clause");
}

#[test]
fn a_platform_handler_written_in_place_has_host_members_and_no_threadsafe_state() {
    let src = "effect E {\n    fn go(n: Int) -> Int => n\n}\nplatform handler H of E {\n    count: Int = 0\n    fn go(n: Int) -> Int => n {\n        return n\n    }\n}\n";
    let errs = errors(src);
    assert!(errs.iter().any(|e| e.contains("needs a body in host code")), "{errs:?}");
    assert!(!errs.iter().any(|e| e.contains("state")), "state is allowed: {errs:?}");
    let src = "effect E {\n    fn go(n: Int) -> Int => n\n}\nthreadsafe platform handler H of E {\n    count: Int = 0\n    fn go(n: Int) -> Int => n\n    ```kotlin\n    return @{count}\n    ```\n}\n";
    let errs = errors(src);
    assert!(errs.iter().any(|e| e.contains("cannot hold Salvo state yet")), "{errs:?}");
}

#[test]
fn typed_host_names_are_checked() {
    let base = "struct A { x: Int }\nfn f(xs: List<A>) [] -> Int => xs\n```kotlin\nreturn xs.map { HOLE -> USE }.sum()\n```\n";
    let ok = base.replace("HOLE", "@{a : A}").replace("USE", "@{a.x}");
    assert_eq!(errors(&ok), Vec::<String>::new());
    let wrong = base.replace("HOLE", "@{a : A}").replace("USE", "@{a.y}");
    assert!(!errors(&wrong).is_empty(), "no field `y`");
    let leaf = base.replace("HOLE", "a").replace("USE", "@{ (`a` : A).x }");
    assert_eq!(errors(&leaf), Vec::<String>::new());
    let leaf_wrong = base.replace("HOLE", "a").replace("USE", "@{ (`a` : A).y }");
    assert!(!errors(&leaf_wrong).is_empty(), "no field `y` on an ascribed leaf");
}

#[test]
fn a_handler_level_block_sees_the_constructor_parameters() {
    let base = "effect E {\n    fn go(n: Int) -> Int => n\n}\nplatform handler H(start: Int) of E {\n    ```kotlin\n    private var at = @{HOLE}\n    ```\n    fn go(n: Int) -> Int => n\n    ```kotlin\n    return @{n} + at\n    ```\n}\n";
    assert_eq!(errors(&base.replace("HOLE", "start")), Vec::<String>::new());
    assert!(!errors(&base.replace("HOLE", "nope")).is_empty(), "an unknown name in a handler-level hole");
}
