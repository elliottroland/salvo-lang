//! Rust codegen tests: golden snapshots of generated code, and (when
//! `rustc` is on PATH) full compile-and-run verifications with exact
//! stdout assertions — mirroring the Kotlin backend's kotlinc tests.

use std::path::Path;
use std::process::Command;

use salvo_core::{Program, SourceSet};

fn build_program(extra: &[(&str, &str, bool)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "rust", "rs", true);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, content, is_define) in extra {
        let rel = Path::new(name);
        let (module, kind) = SourceSet::classify(rel, "rust").unwrap();
        assert_eq!(
            *is_define,
            kind == salvo_core::SourceKind::BackendDefine,
            "unexpected classification for {name}"
        );
        sources.add(*name, module, kind, content.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    }
}

fn generate(extra: &[(&str, &str, bool)]) -> Vec<salvo_backend_rust::EmittedFile> {
    let program = build_program(extra);
    salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// Runs the checker+emitter on a source and returns the errors.
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src, false)]);
    salvo_backend_rust::emit_program(&program)
        .err()
        .expect("expected errors")
}

/// Compiles the generated files with rustc and runs the binary, asserting
/// the exact stdout. Skipped when rustc is not installed.
fn run_rust_files(files: &[salvo_backend_rust::EmittedFile], tag: &str, expected: &str) {
    let dir = std::env::temp_dir().join(format!("salvo-rs-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let src_dir = dir.join("src");
    for f in files {
        let path = src_dir.join(&f.rel_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &f.content).unwrap();
    }
    let bin = dir.join("program");
    let compile = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg(src_dir.join("main.rs"))
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("failed to run rustc");
    assert!(
        compile.status.success(),
        "rustc failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin).output().expect("failed to run binary");
    assert!(
        run.status.success(),
        "generated program crashed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(stdout, expected, "unexpected program output");
    let _ = std::fs::remove_dir_all(&dir);
}

fn rustc_available() -> bool {
    Command::new("rustc").arg("--version").output().is_ok()
}

// ===== demo: structs, defaults, spread, nullability, effects, iterators =====

const DEMO: &str = r#"
struct Person {
    name: Str,
    surname: Str? = None,
    age: Int
}

fn full_name(person: Person) -> [person] Str {
    if (person.surname is Str surname) {
        return "${person.name} ${surname}"
    }
    return person.name
}

fn greet(person: Person) [Console] -> [person] None {
    println("Hello, ${full_name(person)}!")
    for i in range(1, 4) {
        println("  ${i}: ${person.age + i}")
    }
}

fn range(start: Int, end: Int) -> Iter<Int> {
    let i = start
    while i++ < end {
        yield i - 1
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    let person = Person {name: "Roland", surname: "Elliott", age: 36}
    greet(person)
    let anon = Person {...person, surname: None}
    greet(anon)
    let names = list("a", "b")
    println("first: ${names.first()!}")
    let mut_names = mutable_list("x")
    mut_names.add("y")
    println("size: ${mut_names.size()}")
}
"#;

fn generate_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", DEMO, false)])
}

#[test]
fn golden_demo_rust() {
    let files = generate_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// [rs-borrows] Deductions decide parameter modes: kept params borrow,
// `Mut` params borrow mutably, omitted params move.
#[test]
fn deductions_drive_parameter_modes() {
    let src = r#"
fn read(list: List<Int>) -> [list] Int {
    return list.size()
}

fn fill(list: Mut List<Int>, n: Int) -> [list: Mut] None {
    add(list, n)
}

fn consume(list: List<Int>) -> [] Int {
    return list.size()
}

fn main() [use] -> [] None {
    use StdOutConsole
    let items: Mut List<Int> = mutable_list(1, 2)
    fill(items, 3)
    println("${read(items)}")
    println("${consume(items)}")
}
"#;
    let files = generate(&[("main.sv", src, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Kept -> `&`; kept+Mut -> `&mut`; omitted -> owned (move).
    assert!(main.content.contains("pub fn read(list: &Vec<i32>) -> i32"), "{}", main.content);
    assert!(main.content.contains("pub fn fill(list: &mut Vec<i32>, n: i32)"), "{}", main.content);
    assert!(main.content.contains("pub fn consume(list: Vec<i32>) -> i32"), "{}", main.content);
    // Call sites render the matching argument shapes; the moved arg
    // passes by value.
    assert!(main.content.contains("fill(&mut items, 3)"), "{}", main.content);
    assert!(main.content.contains("read(&items)"), "{}", main.content);
    assert!(main.content.contains("consume(items)"), "{}", main.content);
}

#[test]
fn rustc_compiles_and_runs_demo() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_demo();
    let expected = "Hello, Roland Elliott!\n  1: 37\n  2: 38\n  3: 39\n\
                    Hello, Roland!\n  1: 37\n  2: 38\n  3: 39\n\
                    first: a\nsize: 2\n";
    run_rust_files(&files, "demo", expected);
}

// ===== unions =====

const UNIONS_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn parse_age(input: Int) -> Ok Int | Err Str {
    if input >= 0 {
        return ok(input)
    }
    return err("negative age")
}

fn describe(result: Ok Int | Err Str) -> Str {
    let msg = when result {
        is Ok {
            "age ${result}"
        }
        is Err {
            "error: ${result}"
        }
    }
    return msg
}

fn main() [use] -> [] None {
    use StdOutConsole
    let good = parse_age(36)
    println(describe(good))
    let bad = parse_age(-1)
    println(describe(bad))

    let precise: Ok Str | Err Str | Err Bool = ok("yes")
    if precise is Err Str {
        println("err str: ${precise}")
    } elif precise is Ok {
        println("ok: ${precise}")
    } else {
        println("err bool: ${precise}")
    }

    let value = when good {
        is Ok {
            good
        }
        is Err {
            return
        }
    }
    println("value plus one is ${value + 1}")
}
"#;

fn generate_unions_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", UNIONS_DEMO, false)])
}

#[test]
fn golden_unions_rust() {
    let files = generate_unions_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// [rs-union-enums] [union-arm-identity]
#[test]
fn unions_emit_enums() {
    let files = generate_unions_demo();
    let unions = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "unions.rs")
        .expect("unions.rs should be generated");
    assert!(unions.content.contains("pub enum Union2<T1, T2>"));
    assert!(unions.content.contains("pub enum Union3<T1, T2, T3>"));
    assert!(unions.content.contains("impl<T1, T2> std::fmt::Display for Union2<T1, T2>"));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Wrap at return boundaries, positional arm identity.
    assert!(main.content.contains("return Union2::<i32, String>::U1(ok(input));"));
    assert!(main
        .content
        .contains("return Union2::<i32, String>::U2(err(\"negative age\".to_string()));"));
    // Precise `is Err Str` tests a single arm.
    assert!(main.content.contains("matches!(precise, Union3::U2(_))"));
    assert!(main.content.contains("matches!(precise, Union3::U1(_))"));
    // `when` lowers to a match with narrowed reads.
    assert!(main.content.contains("match result {"));
    assert!(main.content.contains("Union2::U1(_) =>"));
    assert!(main.content.contains("result.u1()"));
}

#[test]
fn rustc_compiles_and_runs_unions() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_unions_demo();
    let expected = "age 36\nerror: negative age\nok: yes\nvalue plus one is 37\n";
    run_rust_files(&files, "unions", expected);
}

// ===== qualifiers =====

const QUALIFIERS_DEMO: &str = r#"
struct Person {
    name: Str,
    surname: Str? = None,
    age: Int
}

qualifier Surname of Person {
    surname: Str

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn full_name(person: Person) -> Str {
    return person.name
}

fn full_name(person: Surname Person) -> Str {
    return "${person.name} ${person.surname}"
}

fn describe(person: Person) [Console] {
    if person is Surname {
        println(full_name(person))
    } else {
        println(full_name(person))
    }
}

fn check(n: Int) [Console] {
    if n is Positive {
        println("${n} is positive")
    } else {
        println("${n} is not positive")
    }
}

fn main() [use] {
    use StdOutConsole
    describe(Person {name: "Roland", surname: "Elliott", age: 36})
    describe(Person {name: "Anon", age: 3})
    check(5)
    check(-2)

    let current: Int? = 3
    while current is Int c {
        println("tick ${c}")
        current = if c > 1 { c - 1 } else { None }
    }

    let inner: Ok Str | Err Int = ok("yes")
    let nested: Ok (Ok Str | Err Int) | Err Bool = ok(inner)
    if nested is Ok {
        let back: Ok Str | Err Int = nested
        if back is Ok {
            println("inner ok: ${back}")
        }
    }
}
"#;

fn generate_qualifiers_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", QUALIFIERS_DEMO, false)])
}

#[test]
fn golden_qualifiers_rust() {
    let files = generate_qualifiers_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// [is-qualifies] [rs-fn-mangling] [qual-field-override]
#[test]
fn qualifiers_lower_to_predicates_and_mangled_fns() {
    let files = generate_qualifiers_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Predicate qualifiers become top-level fns taking borrows.
    assert!(main.content.contains("pub fn Surname_qualifies(person: &Person) -> bool"));
    assert!(main.content.contains("pub fn Positive_qualifies(int: i32) -> bool"));
    assert!(main.content.contains("if Surname_qualifies(person)"));
    assert!(main.content.contains("if Positive_qualifies(n)"));
    // The qualified overload is mangled.
    assert!(main.content.contains("pub fn full_name__Surname(person: &Person) -> String"));
    // Field overrides read out of the declared representation.
    assert!(main.content.contains("person.surname.as_ref().unwrap().clone()"));
    // `while x is T` re-binds per iteration; `T?` narrows physically.
    assert!(main.content.contains("while current.is_some()"));
    assert!(main.content.contains("let mut c = current.unwrap();"));
}

#[test]
fn rustc_compiles_and_runs_qualifiers() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_qualifiers_demo();
    let expected = "Roland Elliott\nAnon\n5 is positive\n-2 is not positive\n\
                    tick 3\ntick 2\ntick 1\ninner ok: yes\n";
    run_rust_files(&files, "qualifiers", expected);
}

// ===== effects =====

const EFFECTS_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> [] T
}

handler CyclicRandom<T>(values: List<T>) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        let value = get(values, i % values.size())!
        i = i + 1
        return value
    }
}

fn draw() [Random<Int>, Random<Str>, Console] {
    let n: Int = next_random()
    let s: Str = next_random()
    println("${s}: ${n}")
}

fn lucky_number() [Random<Int>] -> Int {
    return next_random()
}

fn main() [use] {
    use StdOutConsole
    use CyclicRandom(list(10, 20, 30))
    use CyclicRandom(list("a", "b"))
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    println("again: ${lucky_number()}")
}
"#;

fn generate_effects_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", EFFECTS_DEMO, false)])
}

#[test]
fn golden_effects_rust() {
    let files = generate_effects_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// [rs-effects] Effects are traits, deps are `&mut dyn` params, `use`
// locals thread as `&mut local`.
#[test]
fn effects_lower_to_traits_and_mut_dyn_params() {
    let files = generate_effects_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    assert!(main.content.contains("pub trait Random<T> {"));
    assert!(main.content.contains("fn next_random(&mut self) -> T;"));
    assert!(main.content.contains("impl<T: Clone> Random<T> for CyclicRandom<T>"));
    // Effect deps as leading `&mut dyn` parameters.
    assert!(main.content.contains(
        "pub fn draw(random_i32: &mut dyn Random<i32>, random_string: &mut dyn Random<String>, console: &mut dyn Console)"
    ));
    // `use` instantiates handlers into `let mut` locals.
    assert!(main.content.contains("let mut console = StdOutConsole::new();"));
    assert!(main.content.contains("let mut random_i32 = CyclicRandom::new(vec![10, 20, 30]);"));
    // Threading and expected-type disambiguation.
    assert!(main.content.contains("draw(&mut random_i32, &mut random_string, &mut console);"));
    assert!(main.content.contains("let mut n: i32 = random_i32.next_random();"));
    assert!(main.content.contains("let mut s: String = random_string.next_random();"));
}

#[test]
fn rustc_compiles_and_runs_effects() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_effects_demo();
    let expected = "a: 10\nb: 20\nlucky: 30\nagain: 10\n";
    run_rust_files(&files, "effects", expected);
}

// ===== loops as values =====

const LOOPS: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn range(start: Int, end: Int) -> Iter<Int> {
    let i = start
    while i++ < end {
        yield i - 1
    }
}

fn main() [use] -> [] None {
    use StdOutConsole

    let i = 0
    let last = while i++ < 4 {
        i * 10
    } else {
        -1
    }
    println("last: ${last}")

    let j = 9
    let never = while j < 3 {
        j
    } else {
        -1
    }
    println("never: ${never}")

    let found = for x in range(0, 10) {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    for x in range(0, 0) {
        println("unreachable")
    } else {
        println("empty range")
    }

    let k = 0
    let capped = while k < 5 {
        k++
        if k == 3 {
            break
        }
        k
    }
    println("capped: ${capped!}")

    let n = 0
    let verdict: Ok Int | Err Str = while n < 3 {
        if n == 2 {
            break ok(n)
        }
        n++
        err("not yet")
    } else {
        err("empty")
    }
    when verdict {
        is Ok {
            println("ok: ${verdict}")
        }
        is Err {
            println("err: ${verdict}")
        }
    }
}
"#;

fn generate_loops_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", LOOPS, false)])
}

#[test]
fn golden_loops_rust() {
    let files = generate_loops_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// [while-value] [rs-loop-value]
#[test]
fn loops_lower_to_block_expressions() {
    let files = generate_loops_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Value loops become block expressions with an Option result local,
    // unwrapped when the join type has no None arm.
    assert!(main.content.contains("let mut __loop1: Option<i32> = None;"));
    assert!(main.content.contains("__loop1 = Some(i * 10);"));
    assert!(main.content.contains("__loop1.unwrap()"));
    // `else` runs only when the loop never did.
    assert!(main.content.contains("let mut __loop1_ran = false;"));
    assert!(main.content.contains("if !__loop1_ran {"));
    // `break value` assigns before breaking.
    assert!(main.content.contains("__loop3 = Some(x);"));
    // Optional joins keep the plain Option local (no unwrap).
    assert!(main.content.contains("__loop5\n})"));
    // A union-typed loop value re-wraps to the declared arm order.
    assert!(main.content.contains("(match "));
}

#[test]
fn rustc_compiles_and_runs_loops() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_loops_demo();
    let expected = "last: 40\nnever: -1\nfound: 4\nempty range\ncapped: 2\nok: 2\n";
    run_rust_files(&files, "loops", expected);
}

// ===== multi-module: reachability, crate layout, imports =====

fn build_multi_module() -> Program {
    let main = r#"
import geometry.area

fn main() [use] -> [] None {
    use StdOutConsole
    println("area: ${area(3, 4)}")
}
"#;
    let geometry = r#"
fn area(w: Int, h: Int) -> Int {
    return w * h
}
"#;
    let unused = r#"
fn never_called() -> Int {
    return 42
}
"#;
    build_program(&[
        ("main.sv", main, false),
        ("geometry.sv", geometry, false),
        ("unused.sv", unused, false),
    ])
}

// [mod-used-only] [rs-crate] [rs-imports]
#[test]
fn crate_layout_mounts_only_used_modules() {
    let program = build_multi_module();
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().into_owned())
        .collect();
    assert!(paths.contains(&"main.rs".to_string()), "{paths:?}");
    assert!(paths.contains(&"geometry.rs".to_string()), "{paths:?}");
    assert!(!paths.contains(&"unused.rs".to_string()), "{paths:?}");
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // The main module is the crate root: attributes + mounted mods.
    assert!(main.content.starts_with("#![allow("), "{}", main.content);
    assert!(main.content.contains("#[path = \"geometry.rs\"]\npub mod geometry;"));
    assert!(main.content.contains("#[path = \"core/console.rs\"]\npub mod core_console;"));
    assert!(!main.content.contains("pub mod unused;"));
    // Generated imports.
    assert!(main.content.contains("use crate::geometry::*;"));
    assert!(main.content.contains("use crate::core_console::*;"));
}

#[test]
fn rustc_compiles_and_runs_multi_module() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let program = build_multi_module();
    let files = salvo_backend_rust::emit_program(&program).unwrap();
    run_rust_files(&files, "multimod", "area: 12\n");
}

// ===== negative checks =====

// [backend-external]
#[test]
fn missing_define_for_external_fn_is_an_error() {
    let src = r#"
external fn mystery(x: Int) [] -> [x] Int

fn main() [use] -> [] None {
    use StdOutConsole
    println("${mystery(1)}")
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("external fn `mystery` has no rust `define fn`")),
        "unexpected errors: {errors:?}"
    );
}

// [backend-external]
#[test]
fn missing_define_for_external_type_is_an_error() {
    let src = r#"
external type Mystery

fn main() [use] -> [] None {
    use StdOutConsole
    let x: Mystery? = None
    println("${x is None}")
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("external type `Mystery` has no rust `define type`")),
        "unexpected errors: {errors:?}"
    );
}

// [backend-external]
#[test]
fn core_externals_must_be_fully_covered() {
    let fake_core = "external fn uncovered_core_fn(x: Int) [] -> [x] Int\n";
    let program = build_program(&[
        ("core/fake.sv", fake_core, false),
        (
            "main.sv",
            "fn main() [use] -> [] None {\n    use StdOutConsole\n    println(\"hi\")\n}\n",
            false,
        ),
    ]);
    let errors = salvo_backend_rust::emit_program(&program)
        .err()
        .expect("expected coverage errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("external fn `uncovered_core_fn` in core has no rust `define fn`")),
        "unexpected errors: {errors:?}"
    );
}

// [rs-effects] [backend-never-wrong]
#[test]
fn generic_effect_members_are_rejected() {
    let src = r#"
effect Weird {
    fn pick<T>(value: T) -> [] T
}

handler PassThrough of Weird {
    fn pick<T>(value: T) -> T {
        return value
    }
}

fn main() [use] -> [] None {
    use PassThrough
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("generic parameters, which the rust backend")),
        "unexpected errors: {errors:?}"
    );
}

// [lit-numeric] [type-basic] Literal suffixes emit explicit Rust types:
// `5L` -> `5i64`, `2.5f` -> `2.5f32`; unsuffixed literals stay bare.
#[test]
fn numeric_literal_suffixes_emit_rust_types() {
    let program = build_program(&[(
        "main.sv",
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    \
         let big: Long = 5L\n    let ratio: Float = 2.5f\n    let d: Double = 1.5\n    \
         println(\"${big} ${ratio} ${d}\")\n}\n",
        false,
    )]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs not emitted");
    assert!(main.content.contains("= 5i64"), "content: {}", main.content);
    assert!(main.content.contains("= 2.5f32"), "content: {}", main.content);
    assert!(main.content.contains("= 1.5"), "content: {}", main.content);
}

// [qual-ctor-predicate] Predicate-qualifier constructors emit as plain
// fns after erasure; overloads on the qualified type resolve statically
// (mangled name).
#[test]
fn predicate_qualifier_constructors_emit_plain_fns() {
    let src = r#"
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

fn make() -> Int as Positive {
    return 1
}

fn describe(x: Positive Int) -> Str {
    return "positive"
}

fn describe(x: Int) -> Str {
    return "unknown"
}

fn main() [use] -> [] None {
    use StdOutConsole
    println(describe(make()))
}
"#;
    let program = build_program(&[("main.sv", src, false)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs not emitted");
    assert!(
        main.content.contains("fn make() -> i32"),
        "content: {}",
        main.content
    );
    // The Positive overload wins at the call site (reads borrow
    // [rs-borrows], hence the mangled fn taking `&i32`).
    assert!(
        main.content.contains("describe__Positive(&"),
        "content: {}",
        main.content
    );
}

// ===== S1: the `copy` intrinsic [internal-fn] [copy-fn] [rs-copy] =====

/// The same shape as the Kotlin copy demo: identity/clone lowering per
/// type, plus a fate-linked alias (`let zs = xs`) whose source must stay
/// physically valid ([fate-link]: linked bindings clone, not move).
const COPY_DEMO: &str = r#"
struct Person canbe Mut {
    name: Str,
    age: Int
}

fn main() [use] -> [] None {
    use StdOutConsole
    let s = "hi"
    let t = copy(s)
    println(t)
    let xs = mutable_list(1, 2, 3)
    let ys = copy(xs)
    ys.add(4)
    println("${xs.size()} ${ys.size()}")
    let p = Mut Person {name: "a", age: 1}
    let q = copy(p)
    q.name = "b"
    println("${p.name} ${q.name}")
    let arr = [1, 2]
    let brr = copy(arr)
    brr[0] = 9
    println("${arr[0]} ${brr[0]}")
    let zs = xs
    println("${zs.size()}")
}
"#;

// [internal-fn] [rs-copy] `copy` bypasses define templates and lowers to
// `.clone()` on the argument's place; a fate-linked `let` from a bare
// identifier clones instead of moving [fate-link].
#[test]
fn copy_lowers_to_clone_and_linked_lets_clone() {
    let files = generate(&[("main.sv", COPY_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // `copy` clones the place, whatever the type.
    assert!(main.content.contains("let mut t = s.clone();"), "generated:\n{}", main.content);
    assert!(main.content.contains("let mut ys = xs.clone();"), "generated:\n{}", main.content);
    assert!(main.content.contains("let mut q = p.clone();"), "generated:\n{}", main.content);
    assert!(main.content.contains("let mut brr = arr.clone();"), "generated:\n{}", main.content);
    // The fate-linked alias is a real borrow since S3
    // [rs-borrow-locals]: both `zs` and `xs` stay usable, no clone.
    assert!(main.content.contains("let mut zs = &xs;"), "generated:\n{}", main.content);
}

#[test]
fn rustc_compiles_and_runs_copy() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", COPY_DEMO, false)]);
    let expected = "hi\n3 4\na b\n1 9\n3\n";
    run_rust_files(&files, "copy", expected);
}

// ===== S2: move-mode bindings [fate-move-mode] =====

/// The flagship zero-clone consuming pipeline: `persons` is inferred
/// moved (the `name` binding takes ownership through the loop binding),
/// the loop iterates by value, the field moves out, and the result moves
/// up — no clones anywhere. Plus a mutation-driven move-mode binding
/// (`ys` from `xs`) and a per-iteration consumed loop binding.
const S2_DEMO: &str = r#"
struct Person {
    name: Str,
    age: Int
}

fn longest_name(persons: List<Person>) -> Str {
    let longest = ""
    for person in persons {
        let name = person.name
        if size(name) > size(longest) {
            longest = name
        }
    }
    return longest
}

fn consume(text: Str) -> [] None {
}

fn main() [use] -> [] None {
    use StdOutConsole
    let people = list(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
    for s in list("x", "y") {
        consume(s)
    }
    let xs = mutable_list(1, 2)
    let ys = xs
    ys.add(3)
    println("${ys.size()}")
}
"#;

// [fate-move-mode] Move-mode bindings and loops emit real moves: the
// claimed parameter is taken by value, the loop iterates by value, the
// field projection partial-moves, and the binding chain never clones.
#[test]
fn move_mode_bindings_emit_real_moves() {
    let files = generate(&[("main.sv", S2_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The claimed parameter is owned (moved in).
    assert!(
        main.content.contains("pub fn longest_name(persons: Vec<Person>) -> String"),
        "generated:\n{}",
        main.content
    );
    // Move-mode loop: by value, no clone.
    assert!(
        main.content.contains("for mut person in persons {"),
        "generated:\n{}",
        main.content
    );
    // Move-mode bindings: a real partial move and a real move.
    assert!(
        main.content.contains("let mut name = person.name;"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut ys = xs;"),
        "generated:\n{}",
        main.content
    );
    // The whole pipeline is clone-free.
    let pipeline = main
        .content
        .split("pub fn longest_name")
        .nth(1)
        .and_then(|rest| rest.split("pub fn").next())
        .expect("longest_name body");
    assert!(
        !pipeline.contains(".clone()"),
        "pipeline should be clone-free:\n{pipeline}"
    );
}

#[test]
fn rustc_compiles_and_runs_move_modes() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", S2_DEMO, false)]);
    let expected = "Grace\n3\n";
    run_rust_files(&files, "s2-moves", expected);
}

// ===== S3: borrow emission [rs-borrow-locals] =====

/// Borrow-mode bindings and loops emit real borrows: reading through a
/// kept parameter never clones — the collection iterates by reference,
/// the field binding holds `&T`, and reads thread through the
/// reference-binding rendering.
const S3_DEMO: &str = r#"
struct Person {
    name: Str,
    age: Int
}

fn count_long(persons: List<Person>) -> [persons] Int {
    let total = 0
    for person in persons {
        let n = person.name
        if size(n) > 3 {
            total = total + 1
        }
    }
    return total
}

fn poison_guards_the_borrow() -> Int {
    let xs = mutable_list(1, 2)
    let ys = xs
    let n = size(ys)
    add(xs, 9)
    return n + size(xs)
}

fn main() [use] -> [] None {
    use StdOutConsole
    let people = list(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println("${count_long(people)}")
    println("${poison_guards_the_borrow()}")
}
"#;

// [rs-borrow-locals] Borrow-mode bindings from pure places emit `&T`
// locals; borrow-mode loops over concrete non-union elements iterate by
// reference; the read-only pipeline is clone-free.
#[test]
fn borrow_mode_bindings_emit_borrows() {
    let files = generate(&[("main.sv", S3_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("pub fn count_long(persons: &Vec<Person>) -> i32"),
        "generated:\n{}",
        main.content
    );
    // By-reference iteration: the borrowed parameter is iterated bare.
    assert!(
        main.content.contains("for person in persons {"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut n = &person.name;"),
        "generated:\n{}",
        main.content
    );
    // Borrow-mode alias of an owned local.
    assert!(
        main.content.contains("let mut ys = &xs;"),
        "generated:\n{}",
        main.content
    );
    let pipeline = main
        .content
        .split("pub fn count_long")
        .nth(1)
        .and_then(|rest| rest.split("pub fn").next())
        .expect("count_long body");
    assert!(
        !pipeline.contains(".clone()"),
        "read-only pipeline should be clone-free:\n{pipeline}"
    );
}

#[test]
fn rustc_compiles_and_runs_borrows() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", S3_DEMO, false)]);
    let expected = "1\n5\n";
    run_rust_files(&files, "s3-borrows", expected);
}

// ===== L6: linear types [linear-obligation] =====

/// The resource pattern linearity exists for: open, use, close — with
/// `discard` as the deliberate drop [linear-discard]. The checker
/// guarantees no path leaks the handle; the demo verifies the lowering
/// (Rust: `discard` lowers to `drop`).
const LINEAR_DEMO: &str = r#"
struct FileHandle canbe Linear {
    fd: Int
}

fn open_file(path: Str) [Console] -> [] FileHandle {
    println("open ${path}")
    return FileHandle {fd: size(path)}
}

fn close_file(h: FileHandle) [Console] -> [] None {
    println("close fd=${h.fd}")
    discard(h)
}

fn main() [use] -> [] None {
    use StdOutConsole
    let h = open_file("data.txt")
    let n = h.fd
    println("fd=${n}")
    close_file(h)
    let temp = open_file("scratch")
    discard(temp)
    println("done")
}
"#;

// [linear-discard] `discard` lowers to `drop(...)` on the moved value.
#[test]
fn discard_lowers_to_drop() {
    let files = generate(&[("main.sv", LINEAR_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(main.content.contains("drop(h)"), "generated:\n{}", main.content);
    assert!(main.content.contains("drop(temp)"), "generated:\n{}", main.content);
}

#[test]
fn rustc_compiles_and_runs_linear() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_DEMO, false)]);
    let expected = "open data.txt\nfd=8\nclose fd=8\nopen scratch\ndone\n";
    run_rust_files(&files, "l6-linear", expected);
}

// ===== L7a: generic linear opt-in [linear-generics] =====

/// `<T canbe Linear>` in action: the opted std surface makes a linear
/// collection workflow legal end to end — construct empty, `add`
/// individually, `size`, and `discard` the (linear) collection.
const LINEAR_GENERICS_DEMO: &str = r#"
struct FileHandle canbe Linear {
    fd: Int
}

fn open_file(n: Int) [Console] -> [] FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn hold<T canbe Linear>(value: T) -> T {
    return value
}

fn main() [use] -> [] None {
    use StdOutConsole
    let h = hold(open_file(9))
    println("held fd=${h.fd}")
    discard(h)
    let handles: Mut List<FileHandle> = mutable_list()
    add(handles, open_file(1))
    add(handles, open_file(2))
    println("count=${size(handles)}")
    discard(handles)
    println("done")
}
"#;

#[test]
fn rustc_compiles_and_runs_linear_generics() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_GENERICS_DEMO, false)]);
    let expected = "open 9\nheld fd=9\nopen 1\nopen 2\ncount=2\ndone\n";
    run_rust_files(&files, "l7a-linear-generics", expected);
}

// ===== L7b: Once fn types [once-fn] =====

/// A `Once` parameter accepts both a capture-consuming lambda (which is
/// `Once`-typed by construction) and a plain lambda (inverted
/// subtyping); the checker guarantees at most one call.
const ONCE_DEMO: &str = r#"
fn run_once(f: Once () -> None) {
    f()
}

fn consume_list(v: List<Int>) [Console] -> [] None {
    println("consumed ${size(v)} items")
}

fn main() [use] -> [] None {
    use StdOutConsole
    let xs = list(1, 2, 3)
    let g = () -> { consume_list(xs) }
    run_once(g)
    let n = 7
    let plain = () -> { println("plain ${n}") }
    run_once(plain)
    println("done")
}
"#;

// [once-fn] `Once` fn parameters emit `impl FnOnce`.
#[test]
fn once_fn_params_emit_fnonce() {
    let files = generate(&[("main.sv", ONCE_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("pub fn run_once(f: impl FnOnce())"),
        "generated:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_once_fns() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ONCE_DEMO, false)]);
    let expected = "consumed 3 items\nplain 7\ndone\n";
    run_rust_files(&files, "l7b-once", expected);
}

// ===== L7c: derived returns [readonly-return] =====

/// Zero-copy accessors across fn boundaries: a derived return borrows
/// the kept parameter (elided lifetime with one reference parameter, a
/// generated `'a` with more), std's `first` is clone-free, and the
/// caller's narrowing works on the borrowed result.
const DERIVED_DEMO: &str = r#"
struct Person {
    name: Str,
    age: Int
}

fn find_adult(persons: List<Person>) -> [persons] ReadOnly[from: persons] Person? {
    for person in persons {
        if person.age >= 18 {
            return person
        }
    }
    return None
}

fn head_of(persons: List<Person>, tag: Str) -> [persons, tag] ReadOnly[from: persons] Person? {
    return first(persons)
}

fn main() [use] -> [] None {
    use StdOutConsole
    let people = list(Person {name: "Kid", age: 9}, Person {name: "Grace", age: 45})
    let adult = find_adult(people)
    if adult is Person a {
        println("adult: ${a.name}")
    }
    let head = head_of(people, "x")
    if head is Person h {
        println("head: ${h.name}")
    }
    println("done")
}
"#;

// [readonly-return] Derived returns emit borrows: elided lifetime for a
// single reference parameter, a generated `'a` when there are more; the
// std `first` define is clone-free.
#[test]
fn derived_returns_emit_borrows() {
    let files = generate(&[("main.sv", DERIVED_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("pub fn find_adult(persons: &Vec<Person>) -> Option<&Person>"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "pub fn head_of<'a>(persons: &'a Vec<Person>, tag: &String) -> Option<&'a Person>"
        ),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("return Some(person);"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("return persons.first();"),
        "generated:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_derived_returns() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", DERIVED_DEMO, false)]);
    let expected = "adult: Grace\nhead: Kid\ndone\n";
    run_rust_files(&files, "l7c-derived", expected);
}

// ===== L7d: fn-type contracts [fn-contract] =====

/// Contracts on fn types: a keeping contract lets the callee call the
/// value repeatedly and the caller keep the argument (Rust: borrowed
/// argument types, `&mut impl FnMut`); a consuming contract moves the
/// argument; named fns pass through adapter closures (Rust) and
/// function references (Kotlin).
const CONTRACTS_DEMO: &str = r#"
struct Person {
    name: Str,
    age: Int
}

fn apply_keeping(f: (v: List<Person>) -> [v] Int, data: List<Person>) -> [data] Int {
    return f(data) + f(data)
}

fn apply_consuming(f: (v: List<Person>) -> [] Int, data: List<Person>) -> Int {
    return f(data)
}

fn count(people: List<Person>) -> [people] Int {
    return size(people)
}

fn main() [use] -> [] None {
    use StdOutConsole
    let people = list(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    let twice = apply_keeping((v: List<Person>) -> { return size(v) }, people)
    println("twice=${twice}")
    println("still=${size(people)}")
    let named = apply_keeping(count, people)
    println("named=${named}")
    let eaten = apply_consuming((v: List<Person>) -> { return size(v) }, people)
    println("eaten=${eaten}")
    println("done")
}
"#;

// [fn-contract] Keeping contracts borrow, consuming contracts own; fn
// params are `&mut impl FnMut`; named fns wrap in adapters.
#[test]
fn fn_type_contracts_emit_modes() {
    let files = generate(&[("main.sv", CONTRACTS_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains(
            "pub fn apply_keeping(f: &mut impl FnMut(&Vec<Person>) -> i32, data: &Vec<Person>) -> i32"
        ),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "pub fn apply_consuming(f: &mut impl FnMut(Vec<Person>) -> i32, data: Vec<Person>) -> i32"
        ),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("count(&__a0)") || main.content.contains("count(__a0)"),
        "adapter expected:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_fn_contracts() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", CONTRACTS_DEMO, false)]);
    let expected = "twice=4\nstill=2\nnamed=4\neaten=2\ndone\n";
    run_rust_files(&files, "l7d-contracts", expected);
}

// ===== `is` on union-typed struct-field subjects =====
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects; both backends must agree with the
// checker's is_tests lowering.

const FIELD_IS_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

struct Holder {
    result: Ok Int | Err Str
}

fn main() [use] -> [] None {
    use StdOutConsole
    let h = Holder {result: ok(1)}
    if h.result is Ok Int r {
        println("ok ${r}")
    }
    if h.result is Ok {
        println("plain ${h.result}")
    }
    let h2 = Holder {result: err("bad")}
    if h2.result is Err Str e {
        println("err ${e}")
    }
}
"#;

#[test]
fn field_subject_is_lowers_to_union_test() {
    let files = generate(&[("main.sv", FIELD_IS_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("matches!(h.result, Union2::U1(_))"),
        "expected union test on the field in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_field_is() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FIELD_IS_DEMO, false)]);
    run_rust_files(&files, "field-is", "ok 1\nplain 1\nerr bad\n");
}

// ===== union coercion inside arrays/tuples/lambda returns =====
// [union-wrap] Elements of array/tuple literals and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.
// Also covers: a fn-type `let` annotation is dropped in Rust
// (`impl Trait` is invalid on bindings [fn-contract]).

const NESTED_COERCION_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

type Result = Ok Int | Err Str

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn describe(r: Result) -> Str {
    if r is Ok {
        return "ok ${r}"
    }
    return "err ${r}"
}

fn main() [use] -> [] None {
    use StdOutConsole
    let arr: Result[] = [ok(1), err("a")]
    for x in arr {
        println(describe(x))
    }
    let tup: (Str, Result) = ("t", ok(2))
    let (label, r) = tup
    println(describe(r))
    let make: (flag: Bool) -> Result = (flag: Bool) -> {
        return if flag { ok(3) } else { err("b") }
    }
    println(describe(make(true)))
    println(describe(make(false)))
}
"#;

#[test]
fn union_coercion_in_array_tuple_lambda() {
    let files = generate(&[("main.sv", NESTED_COERCION_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("let mut make = "),
        "fn-type let annotation must be dropped in:\n{}",
        main.content
    );
    for needle in [
        "Union2::<i32, String>::U1(ok(1))",
        "Union2::<i32, String>::U2(err(\"a\".to_string()))",
        "Union2::<i32, String>::U1(ok(2))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

#[test]
fn rustc_compiles_and_runs_nested_coercion() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", NESTED_COERCION_DEMO, false)]);
    run_rust_files(&files, "nested-coercion", "ok 1\nerr a\nok 2\nok 3\nerr b\n");
}

// ===== same-name defines and the define/external pairing =====
// [decl-explicit] Every `define fn` implements exactly one `external fn`:
// the external carries the contract, the define the native template.

const PAIRED_EXTERNALS: &str = r#"
external fn twice(s: Str) [] -> [s] Str
external fn twice(i: Int) [] -> [i] Int
"#;

const PAIRED_DEFINES_RS: &str = r#"
define fn twice(s: Str) -> Str {
    inline: ``
    format!("{}{}", ${s}, ${s})
    ``
}

define fn twice(i: Int) -> Int {
    inline: ``
    (${i} * 2)
    ``
}
"#;

#[test]
fn same_name_defines_dispatch_by_param_types() {
    let main = r#"
fn main() [use] -> [] None {
    use StdOutConsole
    println(twice("hi"))
    println("${twice(3)}")
}
"#;
    let files = generate(&[
        ("main.sv", &format!("{PAIRED_EXTERNALS}{main}"), false),
        ("main.rust.sv", PAIRED_DEFINES_RS, true),
    ]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("format!(\"{}{}\""),
        "Str define not chosen in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("(3 * 2)"),
        "Int define not chosen in:\n{}",
        main.content
    );
}

// [decl-explicit] A define with no external has no contract at all.
#[test]
fn define_without_an_external_is_an_error() {
    let program = build_program(&[
        ("main.sv", "fn main() [use] -> [] None {\n    use StdOutConsole\n}\n", false),
        ("main.rust.sv", PAIRED_DEFINES_RS, true),
    ]);
    let errors = salvo_backend_rust::emit_program(&program)
        .err()
        .expect("expected codegen errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("implements no `external fn` declaration")),
        "unexpected errors: {errors:?}"
    );
}

#[test]
fn rustc_compiles_and_runs_paired_defines() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let main = r#"
fn main() [use] -> [] None {
    use StdOutConsole
    println(twice("hi"))
    println("${twice(3)}")
}
"#;
    let files = generate(&[
        ("main.sv", &format!("{PAIRED_EXTERNALS}{main}"), false),
        ("main.rust.sv", PAIRED_DEFINES_RS, true),
    ]);
    run_rust_files(&files, "paired-defines", "hihi\n6\n");
}

// ===== effect member fns with their own generics =====
// [effect-member-generics] [rs-effects] `dyn` traits cannot have generic
// methods: the rust backend rejects them loudly instead of emitting
// invalid code.
#[test]
fn effect_member_generics_are_rejected_loudly() {
    let src = r#"
effect Stash {
    fn pick<T>(a: T, b: T) -> [] T
}

handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T {
        return a
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    use FirstStash
    println("${pick(7, 2)}")
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("cannot dispatch dynamically")),
        "unexpected errors: {errors:?}"
    );
}

// ===== effect environment keyed by checker types =====
// [effect-disambiguation] [rs-effects] The emitter's effect environment is
// keyed by the checker's lowered effect types (`Checked::fn_effects` /
// `use_effects` / `call_effects`), not by type renderings: an effect
// written through a type alias resolves to the same instance a `use`
// registered under the canonical type.

const ALIASED_EFFECT_DEMO: &str = r#"
type Count = Int

effect Random<T> {
    fn next_random() -> [] T
}

handler CyclicRandom<T>(values: List<T>) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        let value = get(values, i % values.size())!
        i = i + 1
        return value
    }
}

fn roll() [Random<Count>] -> Count {
    return next_random()
}

fn main() [use] -> [] None {
    use StdOutConsole
    use CyclicRandom(list(7, 8))
    println("${roll()} ${roll()}")
}
"#;

#[test]
fn aliased_effect_types_resolve_to_the_same_handler() {
    let files = generate(&[("main.sv", ALIASED_EFFECT_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("roll(&mut random_i32)"),
        "handler not threaded through the aliased effect in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_aliased_effects() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ALIASED_EFFECT_DEMO, false)]);
    run_rust_files(&files, "aliased-effects", "7 8\n");
}

// ===== std array functions =====
// [type-array] Arrays get the `core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02). `T[]` and `List<T>` share the `Vec<T>`
// rendering, so the defines mirror each other.

const ARRAY_STD_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> [] T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    use CyclicRandom([1, 2, 3, 4])
    let nums: Int[] = [3, 4, 5]
    println("size ${nums.size()} get ${nums.get(2)!} first ${nums.first()!}")
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
"#;

#[test]
fn array_std_functions_lower() {
    let files = generate(&[("main.sv", ARRAY_STD_DEMO, false)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    for needle in ["nums.len() as i32", "nums.get((2) as usize)", "nums.first()"] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

#[test]
fn rustc_compiles_and_runs_array_std() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ARRAY_STD_DEMO, false)]);
    run_rust_files(
        &files,
        "array-std",
        "size 3 get 5 first 3\niter 3\niter 4\niter 5\nrandom 2 3\n",
    );
}

// ===== N1: dot-names [name-dot] =====

/// The same namespace-struct program the Kotlin backend nests: Rust
/// flattens the names instead.
const DOT_NAMES: &str = r#"
struct Environment {
    id: Environment.Id,
    name: Environment.Name
}

struct Environment.Id {
    value: Str
}

struct Environment.Name {
    value: Str
}

qualifier Environment.Tag of Str

fn tag(value: Str) -> Str as Environment.Tag {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> [] None {
    use StdOutConsole()
    let env = Environment {
        id: Environment.Id {value: "prod"},
        name: Environment.Name {value: "Production"}
    }
    println("${env.id.value} / ${env.name.value}")
    println(label(tag("t1")))
    println(label("t2"))
}
"#;

// [name-dot] [rs-fn-mangling] Rust concatenates a dot-name
// (`Environment.Id` → `EnvironmentId`): modules and structs share one
// type namespace, so a nested `mod Environment` beside
// `struct Environment` would be E0428. Mangled overload names use the
// same flat spelling.
#[test]
fn dot_names_flatten() {
    let program = build_program(&[("main.sv", DOT_NAMES, false)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("pub struct EnvironmentId {")
            && main.content.contains("pub struct EnvironmentName {"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("pub id: EnvironmentId,"),
        "generated:\n{}",
        main.content
    );
    // No dots survive in emitted Rust identifiers.
    assert!(
        !main.content.contains("Environment.Id"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("pub fn label__EnvironmentTag("),
        "generated:\n{}",
        main.content
    );
    run_rust_files(&files, "dot_names", "prod / Production\ntagged t1\nplain t2\n");
}

// [name-dot] [type-union] [is-narrowing] Dot-named types as union arms:
// arm identity, `when` narrowing, a dot-named predicate-free qualifier in
// a nullable union, and `canbe Mut` on a dot-named struct.
const DOT_NAME_UNIONS: &str = r#"
struct Environment.Id canbe Mut {
    value: Str
}

struct Environment.Name {
    value: Str
}

struct Environment {
    id: Environment.Id
}

qualifier Environment.Tag of Str

fn tag(v: Str) -> Str as Environment.Tag {
    return v
}

fn pick(flag: Bool) -> Environment.Id | Environment.Name {
    if flag {
        return Environment.Id {value: "id"}
    }
    return Environment.Name {value: "name"}
}

fn show(x: Environment.Id | Environment.Name) -> Str {
    when x {
        is Environment.Id {
            return "id: ${x.value}"
        }
        is Environment.Name {
            return "name: ${x.value}"
        }
    }
}

fn maybe(t: Environment.Tag Str | None) -> Str {
    if t is Environment.Tag {
        return "tagged ${t}"
    }
    return "none"
}

fn main() [use] -> [] None {
    use StdOutConsole()
    println(show(pick(true)))
    println(show(pick(false)))
    let m = Mut Environment.Id {value: "before"}
    m.value = "after"
    println(m.value)
    println(maybe(tag("x")))
    println(maybe(None))
}
"#;

#[test]
fn dot_names_in_unions_and_narrowing() {
    let program = build_program(&[("main.sv", DOT_NAME_UNIONS, false)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_rust_files(
        &files,
        "dot_name_unions",
        "id: id\nname: name\nafter\ntagged x\nnone\n",
    );
}

// ===== D5: qualifier subjects [qual-subject] =====

/// The state/provenance contrast, observable in the output: `add` is a
/// mutating call, so it strips the *state* claim (`Checked`) and the plain
/// overload is chosen, while the *provenance* claim (`Trusted`) survives
/// and keeps selecting its own overload.
const QUAL_SUBJECTS: &str = r#"
qualifier Checked of List<Int>
provenance qualifier Trusted of List<Int>

fn check(l: Mut List<Int>) -> Mut List<Int> as Checked {
    return l
}

fn trust(l: Mut List<Int>) -> Mut List<Int> as Trusted {
    return l
}

fn describe(l: List<Int>) -> Str {
    return "plain ${l.size()}"
}

fn describe(l: Checked List<Int>) -> Str {
    return "checked ${l.size()}"
}

fn describe(l: Trusted List<Int>) -> Str {
    return "trusted ${l.size()}"
}

fn main() [use] -> [] None {
    use StdOutConsole()
    let t = trust(mutable_list(1, 2))
    t.add(3)
    println(describe(t))
    let c = check(mutable_list(1, 2))
    c.add(3)
    println(describe(c))
    let c2 = check(mutable_list(4, 5))
    println(describe(c2))
}
"#;

// [qual-subject] [deduce-syntax] [qual-erasure] Provenance survives a
// mutating call where state does not — and both subjects erase, so the
// difference shows up only in which overload the checker picked.
#[test]
fn provenance_survives_mutation_where_state_does_not() {
    let program = build_program(&[("main.sv", QUAL_SUBJECTS, false)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_rust_files(
        &files,
        "qual_subjects",
        "trusted 3\nplain 3\nchecked 2\n",
    );
}
