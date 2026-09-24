//! Rust codegen tests: golden snapshots of generated code, and (when
//! `rustc` is on PATH) full compile-and-run verifications with exact
//! stdout assertions — mirroring the Kotlin backend's kotlinc tests.

use std::path::Path;
use std::process::Command;

use salvo_core::{Program, SourceSet};

fn build_program(extra: &[(&str, &str)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "rs", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, content) in extra {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, content.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.is_empty(),
            "parse errors in {}: {errors:?}",
            file.name
        );
        modules.push(module);
    }
    Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    }
}

fn generate(extra: &[(&str, &str)]) -> Vec<salvo_backend_rust::EmittedFile> {
    let program = build_program(extra);
    salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// Runs the checker+emitter on a source and returns the errors.
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src)]);
    salvo_backend_rust::emit_program(&program)
        .err()
        .expect("expected errors")
}

/// Compiles the generated files with rustc and runs the binary, asserting
/// the exact stdout. Skipped when rustc is not installed.
fn run_rust_files(files: &[salvo_backend_rust::EmittedFile], tag: &str, expected: &str) {
    // As in the Kotlin backend: the gate and the cache are at the point of
    // use, so neither can be bypassed by a test that forgets them.
    let rustc = salvo_testkit::rustc();
    if !rustc.available {
        return;
    }
    // A pass is a pure function of the generated code, the expected output
    // and the compiler, so it is worth remembering; `SALVO_E2E_FRESH=1`
    // ignores the stamps.
    let mut parts: Vec<Vec<u8>> = vec![
        b"rust-files".to_vec(),
        rustc.version.as_bytes().to_vec(),
        expected.as_bytes().to_vec(),
    ];
    for f in files {
        parts.push(f.rel_path.to_string_lossy().as_bytes().to_vec());
        parts.push(f.content.as_bytes().to_vec());
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    let Some(stamp) = salvo_testkit::cached(
        env!("CARGO_TARGET_TMPDIR"),
        &format!("rust-files {tag}"),
        &refs,
    ) else {
        return;
    };
    let dir = salvo_testkit::scratch(env!("CARGO_TARGET_TMPDIR"), &format!("rs-{tag}"));
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
    stamp.verified();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Whether the compile-and-run tests should exercise `rustc`. Probed once
/// per test binary by `salvo-testkit`, which also owns the `SALVO_SKIP_E2E`
/// gate and the version string that goes into every cache key.
fn rustc_available() -> bool {
    salvo_testkit::rustc().available
}

// ===== demo: structs, defaults, spread, nullability, effects, iterators =====

const DEMO: &str = r#"
struct Person {
    name: Str,
    surname: Str? = None,
    age: Int
}

fn full_name(person: Person) -> Str => person {
    if (person.surname is Str surname) {
        return "${person.name} ${surname}"
    }
    return person.name
}

fn greet(person: Person) [Console] -> None => person {
    println("Hello, ${full_name(person)}!")
    for i in upto(1, 4) {
        println("  ${i}: ${person.age + i}")
    }
}

// Its own pass, named `Upto` rather than `Range`: `core.range` exports a
// `Range` of its own [mod-export], and two same-named structs in two modules
// confuse the pass-driving resolution (an open defect — ROADMAP.md).
struct Upto {
    start: Int,
    end: Int
}

fn upto(start: Int, end: Int) -> Upto {
    return Upto {start: start, end: end}
}

iter fn next(r: Upto) -> Emitted Int | Finished {
    state {
        at: Int = r.start
    }
    if at >= r.end {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] -> None {
    use StdOutConsole
    let person = Person {name: "Roland", surname: "Elliott", age: 36}
    greet(person)
    let anon = Person {...person, surname: None}
    greet(anon)
    // [col-of-nonempty] The constructor claims `NonEmpty`, so `first` answers
    // an element and needs no `!`.
    let names = list_of("a", "b")
    println("first: ${names.first()}")
    let mut_names = mut_list_of("x")
    mut_names.add("y")
    println("size: ${mut_names.size()}")
}
"#;

fn generate_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", DEMO)])
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
export fn read(list: List<Int>) -> Int => list {
    return list.size()
}

export fn fill(list: Mut List<Int>, n: Int) -> None => list: Mut {
    add(list, n)
}

export fn consume(list: List<Int>) -> Int => !list {
    return list.size()
}

export fn main() [use] -> None {
    use StdOutConsole
    let items: Mut List<Int> = mut_list_of(1, 2)
    fill(items, 3)
    println("${read(items)}")
    println("${consume(items)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Kept -> `&`; kept+Mut -> `&mut`; omitted -> owned (move).
    assert!(
        main.content.contains("pub fn read(list: &Vec<i32>) -> i32"),
        " {}",
        main.content
    );
    assert!(
        main.content
            .contains("pub fn fill(list: &mut Vec<i32>, n: i32)"),
        " {}",
        main.content
    );
    assert!(
        main.content
            .contains("pub fn consume(list: Vec<i32>) -> i32"),
        " {}",
        main.content
    );
    // Call sites render the matching argument shapes; the moved arg
    // passes by value.
    assert!(
        main.content.contains("fill(&mut items, 3)"),
        "{}",
        main.content
    );
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

fn main() [use] -> None {
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
    generate(&[("main.sv", UNIONS_DEMO)])
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
    assert!(unions
        .content
        .contains("impl<T1, T2> std::fmt::Display for Union2<T1, T2>"));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    // Wrap at return boundaries, positional arm identity.
    assert!(main
        .content
        .contains("return Union2::<i32, String>::U1(ok(input));"));
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
    generate(&[("main.sv", QUALIFIERS_DEMO)])
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
    assert!(main
        .content
        .contains("pub fn Surname_qualifies(person: &Person) -> bool"));
    assert!(main
        .content
        .contains("pub fn Positive_qualifies(int: i32) -> bool"));
    assert!(main.content.contains("if Surname_qualifies(person)"));
    assert!(main.content.contains("if Positive_qualifies(n)"));
    // The qualified overload is mangled.
    assert!(main
        .content
        .contains("pub fn full_name__Surname(person: &Person) -> String"));
    // Field overrides read out of the declared representation.
    assert!(main
        .content
        .contains("person.surname.as_ref().unwrap().clone()"));
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
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        let value = get(values, i % values.size())!
        i = i + 1
        return copy(value)
    }
}

fn draw() [local Random<Int>, local Random<Str>, Console] {
    let n: Int = next_random()
    let s: Str = next_random()
    println("${s}: ${n}")
}

fn lucky_number() [local Random<Int>] -> Int {
    return next_random()
}

fn main() [use] {
    use StdOutConsole
    use local CyclicRandom(list_of(10, 20, 30))
    use local CyclicRandom(list_of("a", "b"))
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    println("again: ${lucky_number()}")
}
"#;

fn generate_effects_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", EFFECTS_DEMO)])
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
    assert!(main
        .content
        .contains("impl<T: Clone + 'static> Random<T> for CyclicRandom<T>"));
    // Effect deps as leading `&mut dyn` parameters.
    assert!(main.content.contains(
        "pub fn draw(random_i32: &mut dyn Random<i32>, random_string: &mut dyn Random<String>, console: &mut dyn Console)"
    ));
    // `use` instantiates handlers into `let mut` locals.
    assert!(main
        .content
        .contains("let mut console = StdOutConsole::new();"));
    assert!(// [effect-handler-generics] The handler is constructed *at* a type — the
    // turbofish is written even where rustc could have inferred it, since a
    // stateless generic handler gives it nothing to infer from.
    main.content.contains("let mut random_i32 = CyclicRandom::<i32>::new(vec![10, 20, 30], move |__i0| __i0.clone());"));
    // Threading and expected-type disambiguation.
    assert!(main
        .content
        .contains("draw(&mut random_i32, &mut random_string, &mut console);"));
    assert!(main
        .content
        .contains("let mut n: i32 = random_i32.next_random();"));
    assert!(main
        .content
        .contains("let mut s: String = random_string.next_random();"));
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
// Its own pass, named `Upto` rather than `Range`: `core.range` exports a
// `Range` of its own [mod-export], and two same-named structs in two modules
// confuse the pass-driving resolution (an open defect — ROADMAP.md).
struct Upto {
    start: Int,
    end: Int
}

fn upto(start: Int, end: Int) -> Upto {
    return Upto {start: start, end: end}
}

iter fn next(r: Upto) -> Emitted Int | Finished {
    state {
        at: Int = r.start
    }
    if at >= r.end {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] -> None {
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

    let found = for x in [0, 1, 2, 3, 4, 5] {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    for x in upto(0, 0) {
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
    generate(&[("main.sv", LOOPS)])
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
    assert!(main
        .content
        .contains("let mut __loop1: Option<i32> = None;"));
    assert!(main.content.contains("__loop1 = Some(i * 10);"));
    assert!(main.content.contains("__loop1.unwrap()"));
    // `else` runs only when the loop never did.
    assert!(main.content.contains("let mut __loop1_ran = false;"));
    assert!(main.content.contains("if !__loop1_ran {"));
    // `break value` assigns before breaking.
    assert!(main.content.contains("__loop3 = Some(x);"));
    // Optional joins keep the plain Option local (no unwrap). The number
    // shifted by one when a `for` over an `Iter<T>` started naming its pass:
    // driving a producer costs one fresh loop name.
    assert!(main.content.contains("__loop6\n})"));
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

fn main() [use] -> None {
    use StdOutConsole
    println("area: ${area(3, 4)}")
}
"#;
    let geometry = r#"
export fn area(w: Int, h: Int) -> Int {
    return w * h
}
"#;
    let unused = r#"
export fn never_called() -> Int {
    return 42
}
"#;
    build_program(&[
        ("main.sv", main),
        ("geometry.sv", geometry),
        ("unused.sv", unused),
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
    assert!(main
        .content
        .contains("#[path = \"geometry.rs\"]\npub mod geometry;"));
    assert!(main
        .content
        .contains("#[path = \"core/console.rs\"]\npub mod core_console;"));
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

// [rs-effects] [backend-never-wrong]
#[test]
fn generic_effect_members_are_rejected() {
    let src = r#"
export effect Weird {
    fn pick<T>(value: T) -> T => !value
}

export handler PassThrough of Weird {
    fn pick<T>(value: T) -> T {
        return value
    }
}

export fn main() [use] -> None {
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
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
         let big: Long = 5L\n    let ratio: Float = 2.5f\n    let d: Double = 1.5\n    \
         println(\"${big} ${ratio} ${d}\")\n}\n",
    )]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs not emitted");
    assert!(main.content.contains("= 5i64"), "content: {}", main.content);
    assert!(
        main.content.contains("= 2.5f32"),
        "content: {}",
        main.content
    );
    assert!(main.content.contains("= 1.5"), "content: {}", main.content);
}

// [qual-ctor-predicate] Predicate-qualifier constructors emit as plain
// fns after erasure; overloads on the qualified type resolve statically
// (mangled name).
#[test]
fn predicate_qualifier_constructors_emit_plain_fns() {
    let src = r#"
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

export fn make() -> +Positive Int {
    return 1
}

export fn describe(x: Positive Int) -> Str {
    return "positive"
}

export fn describe(x: Int) -> Str {
    return "unknown"
}

export fn main() [use] -> None {
    use StdOutConsole
    println(describe(make()))
}
"#;
    let program = build_program(&[("main.sv", src)]);
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

// ===== S1: the `copy` intrinsic [intrinsic-fn] [copy-fn] [rs-copy] =====

/// The same shape as the Kotlin copy demo: identity/clone lowering per
/// type, plus a fate-linked alias (`let zs = xs`) whose source must stay
/// physically valid ([fate-link]: linked bindings clone, not move).
const COPY_DEMO: &str = r#"
struct Person canbe Mut {
    name: Str,
    age: Int
}

fn main() [use] -> None {
    use StdOutConsole
    let s = "hi"
    let t = copy(s)
    println(t)
    let xs = mut_list_of(1, 2, 3)
    let ys = copy(xs)
    ys.add(4)
    println("${xs.size()} ${ys.size()}")
    let p = Mut Person {name: "a", age: 1}
    let q = copy(p)
    q.name = "b"
    println("${p.name} ${q.name}")
    let arr = array_of(1, 2)
    let brr = copy(arr)
    brr[0] = 9
    println("${arr[0]} ${brr[0]}")
    let zs = xs
    println("${zs.size()}")
}
"#;

// [intrinsic-fn] [rs-copy] `copy` lowers to
// `.clone()` on the argument's place; a fate-linked `let` from a bare
// identifier clones instead of moving [fate-link].
#[test]
fn copy_lowers_to_clone_and_linked_lets_clone() {
    let files = generate(&[("main.sv", COPY_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // `copy` clones the place, whatever the type.
    assert!(
        main.content.contains("let mut t = s.clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut ys = xs.clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut q = p.clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut brr = arr.clone();"),
        "generated:\n{}",
        main.content
    );
    // The fate-linked alias is a real borrow since S3
    // [rs-borrow-locals]: both `zs` and `xs` stay usable, no clone.
    assert!(
        main.content.contains("let mut zs = &xs;"),
        "generated:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_copy() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", COPY_DEMO)]);
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

fn consume(text: Str) -> None => !text {
}

fn main() [use] -> None {
    use StdOutConsole
    let people = list_of(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
    for s in list_of("x", "y") {
        consume(s)
    }
    let xs = mut_list_of(1, 2)
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
    let files = generate(&[("main.sv", S2_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The claimed parameter is owned (moved in).
    assert!(
        main.content
            .contains("pub fn longest_name(persons: Vec<Person>) -> String"),
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
    let files = generate(&[("main.sv", S2_DEMO)]);
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

fn count_long(persons: List<Person>) -> Int => persons {
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
    let xs = mut_list_of(1, 2)
    let ys = xs
    let n = size(ys)
    add(xs, 9)
    return n + size(xs)
}

fn main() [use] -> None {
    use StdOutConsole
    let people = list_of(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println("${count_long(people)}")
    println("${poison_guards_the_borrow()}")
}
"#;

// [rs-borrow-locals] Borrow-mode bindings from pure places emit `&T`
// locals; borrow-mode loops over concrete non-union elements iterate by
// reference; the read-only pipeline is clone-free.
#[test]
fn borrow_mode_bindings_emit_borrows() {
    let files = generate(&[("main.sv", S3_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("pub fn count_long(persons: &Vec<Person>) -> i32"),
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
    let files = generate(&[("main.sv", S3_DEMO)]);
    let expected = "1\n5\n";
    run_rust_files(&files, "s3-borrows", expected);
}

// ===== L6: linear types [linear-obligation] =====

/// The resource pattern linearity exists for: open, use, close — with
/// `discard` as the deliberate drop [linear-discard]. The checker
/// guarantees no path leaks the handle; the demo verifies the lowering
/// (Rust: `discard` lowers to `drop`).
const LINEAR_DEMO: &str = r#"
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


fn open_file(path: Str) [Console] -> FileHandle => !path {
    println("open ${path}")
    return FileHandle {fd: size(path)}
}

fn close_file(h: FileHandle) [Console] -> None => !h {
    println("close fd=${h.fd}")
    close(h)
}

fn main() [use] -> None {
    use StdOutConsole
    let h = open_file("data.txt")
    let n = h.fd
    println("fd=${n}")
    close_file(h)
    let temp = open_file("scratch")
    close(temp)
    let note = "note"
    discard(note)
    println("done")
}
"#;

// [linear-discard] `discard` lowers to `drop(...)` on the moved value.
#[test]
fn discard_lowers_to_drop() {
    let files = generate(&[("main.sv", LINEAR_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // [linear-discard] A linear value's discharge is its own `close`; what
    // `discard` still lowers to is `drop`, for the non-linear values it is
    // now the escape hatch for. The name carries a suffix because std
    // declares a `close` too (the `Lines` discharger [fs-surface]) — one
    // name, one overload set, and the emitters spell every overload after
    // the first apart.
    assert!(
        main.content.contains("close__3(h)"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("close__3(temp)"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("drop(note)"),
        "generated:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_linear() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_DEMO)]);
    let expected = "open data.txt\nfd=8\nclose fd=8\nopen scratch\ndone\n";
    run_rust_files(&files, "l6-linear", expected);
}

// ===== L7a: generic linear opt-in [linear-generics] =====

/// `<T canbe linear>` in action: the opted std surface makes a linear
/// collection workflow legal end to end — construct empty, `add`
/// individually, `size`, and `discard` the (linear) collection.
const LINEAR_GENERICS_DEMO: &str = r#"
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


fn open_file(n: Int) [Console] -> FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn hold<T canbe linear>(value: T) -> T {
    return value
}

fn main() [use] -> None {
    use StdOutConsole
    let h = hold(open_file(9))
    println("held fd=${h.fd}")
    close(h)
    println("done")
}
"#;

#[test]
fn rustc_compiles_and_runs_linear_generics() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_GENERICS_DEMO)]);
    let expected = "open 9\nheld fd=9\ndone\n";
    run_rust_files(&files, "l7a-linear-generics", expected);
}

// ===== L7b: once fn types [once-fn] =====

/// A `once` parameter accepts both a capture-consuming lambda (which is
/// `once`-typed by construction) and a plain lambda (inverted
/// subtyping); the checker guarantees at most one call.
const ONCE_DEMO: &str = r#"
fn run_once(f: once () [Console] -> None) {
    f()
}

fn consume_list(v: List<Int>) [Console] -> None => !v {
    println("consumed ${size(v)} items")
}

fn main() [use] -> None {
    use StdOutConsole
    let xs = list_of(1, 2, 3)
    let g = () -> { consume_list(xs) }
    run_once(g)
    let n = 7
    let plain = () -> { println("plain ${n}") }
    run_once(plain)
    println("done")
}
"#;

// [once-fn] `once` fn parameters emit `impl FnOnce`.
#[test]
fn once_fn_params_emit_fnonce() {
    let files = generate(&[("main.sv", ONCE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains(
            "pub fn run_once(console: &mut dyn Console, f: impl FnOnce(&mut dyn Console))"
        ),
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
    let files = generate(&[("main.sv", ONCE_DEMO)]);
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

fn find_adult(persons: List<Person>) -> proj(persons) Person? => persons {
    for person in persons {
        if person.age >= 18 {
            return person
        }
    }
    return None
}

fn head_of(persons: List<Person>, tag: Str) -> proj(persons) Person? => persons, tag {
    return first(persons)
}

fn main() [use] -> None {
    use StdOutConsole
    let people = list_of(Person {name: "Kid", age: 9}, Person {name: "Grace", age: 45})
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
// std `first` is clone-free.
#[test]
fn derived_returns_emit_borrows() {
    let files = generate(&[("main.sv", DERIVED_DEMO)]);
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
    let files = generate(&[("main.sv", DERIVED_DEMO)]);
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

fn apply_keeping(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] v => data {
    return f(data) + f(data)
}

fn apply_consuming(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] !v {
    return f(data)
}

fn count(people: List<Person>) -> Int => people {
    return size(people)
}

fn main() [use] -> None {
    use StdOutConsole
    let people = list_of(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
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
    let files = generate(&[("main.sv", CONTRACTS_DEMO)]);
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
    let files = generate(&[("main.sv", CONTRACTS_DEMO)]);
    let expected = "twice=4\nstill=2\nnamed=4\neaten=2\ndone\n";
    run_rust_files(&files, "l7d-contracts", expected);
}

// ===== `is` on union-typed struct-field subjects =====
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects, and narrow like them [flow-place];
// both backends must agree with the checker's is_tests lowering.

const FIELD_IS_DEMO: &str = r#"
struct Holder {
    result: Ok Int | Err Str
}

fn main() [use] -> None {
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
    let files = generate(&[("main.sv", FIELD_IS_DEMO)]);
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
    let files = generate(&[("main.sv", FIELD_IS_DEMO)]);
    run_rust_files(&files, "field-is", "ok 1\nplain 1\nerr bad\n");
}

// ===== place-based narrowing of field reads [flow-place] =====
// [flow-place] `is` narrows *places*: after `p.surname is Str` the field
// itself reads as `Str` (no binding needed), field chains included.
// Invalidation is [flow-place-invalidate]; `when` stays variable-only
// [when-union-subject].

const PLACE_NARROW_DEMO: &str = r#"
struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    address: Mut Address
}

struct Holder {
    result: Ok Int | Err Str
}

fn describe(p: Person) -> Str => p {
    if p.surname is Str {
        return "${p.name} ${p.surname}"
    }
    return p.name
}

fn main() [use] -> None {
    use StdOutConsole
    println(describe(Person {name: "Ann", surname: "Lee", address: Mut Address {city: "Rome"}}))
    println(describe(Person {name: "Bo", address: Mut Address {city: None}}))
    let p = Person {name: "Cy", surname: "Ray", address: Mut Address {city: "Oslo"}}
    if p.address.city is Str {
        println("city ${p.address.city}")
    }
    let h = Holder {result: ok(3)}
    if h.result is Ok {
        println("ok ${h.result}")
    }
}
"#;

/// [flow-place] A narrowed nullable field read unwraps the `Option`
/// physically [rs-option]; a narrowed wrapper-union field read takes the
/// arm accessor [rs-union-enums].
#[test]
fn narrowed_field_reads_unwrap() {
    let files = generate(&[("main.sv", PLACE_NARROW_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("p.surname.as_ref().unwrap().clone()"),
        "expected the narrowed field read to unwrap in:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("p.address.city.as_ref().unwrap().clone()"),
        "expected the narrowed field *chain* read to unwrap in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("*h.result.u1()"),
        "expected the narrowed wrapper-union field read to use the arm \
         accessor in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_place_narrowing() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PLACE_NARROW_DEMO)]);
    run_rust_files(&files, "place-narrow", "Ann Lee\nBo\ncity Oslo\nok 3\n");
}

// [flow-place] [op-no-none] A narrowed field is usable as an *operand*,
// which the optional-strictness rules used to reject outright.

const PLACE_OPERAND_DEMO: &str = r#"
struct Reading {
    label: Str,
    value: Int? = None
}

fn main() [use] -> None {
    use StdOutConsole
    let r = Reading {label: "temp", value: 21}
    if r.value is Int {
        println("${r.label}: ${r.value + 1}")
    }
    let empty = Reading {label: "none"}
    if empty.value is Int {
        println("unreachable")
    } else {
        println("${empty.label}: no value")
    }
}
"#;

/// [flow-place] [rs-option] The operand read unwraps the `Option`; Rust has
/// no smart cast to lean on, so this is the same lowering everywhere.
#[test]
fn narrowed_field_operand_unwraps() {
    let files = generate(&[("main.sv", PLACE_OPERAND_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("r.value.unwrap() + 1"),
        "expected the narrowed operand to unwrap in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_place_operand() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PLACE_OPERAND_DEMO)]);
    run_rust_files(&files, "place-operand", "temp: 22\nnone: no value\n");
}

// ===== tuple element access [expr-tuple-index] =====
// `t.0` reads a tuple element; a constant index narrows like a field
// [flow-place], and nesting (`t.1.0`) is two projections.

const TUPLE_INDEX_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole
    let t: (Int, Str, Bool) = (1, "two", true)
    println("${t.0} ${t.1} ${t.2}")
    let nested: (Int, (Str, Int)) = (7, ("in", 9))
    println("${nested.1.0} ${nested.1.1}")
    let maybe: (Str?, Int) = ("here", 5)
    if maybe.0 is Str {
        println("some ${maybe.0} ${maybe.1 + 1}")
    } else {
        println("none")
    }
}
"#;

/// [rs-tuple-index] Rust tuples index natively; a narrowed element unwraps
/// its `Option` [rs-option].
#[test]
fn tuple_elements_emit_native_indexes() {
    let files = generate(&[("main.sv", TUPLE_INDEX_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("t.0"),
        "expected a native tuple index in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("nested.1.0.clone()"),
        "expected a nested index chain in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("maybe.0.as_ref().unwrap().clone()"),
        "expected the narrowed element to unwrap in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_tuple_index() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TUPLE_INDEX_DEMO)]);
    run_rust_files(&files, "tuple-index", "1 two true\nin 9\nsome here 6\n");
}

// ===== effect dependencies on handlers [effect-handler-deps] =====
// A handler constructor parameter of effect type is a dependency: the
// member body may use that effect, the `use` site supplies it from scope,
// and callers of the outer effect never mention it.

const HANDLER_DEPS_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> None => message
}

handler ConsoleLogger [local Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

fn work() [local Logger] -> None {
    log("from work")
}

fn main() [use] -> None {
    use StdOutConsole
    use local ConsoleLogger()
    work()
    log("from main")
}
"#;

/// [effect-handler-deps] [rs-effect-fusion] The dependency is neither a
/// field nor a `new` parameter: the member bodies move into a generated
/// `__Impl_H` trait that takes it as a fused value, and the `use` site
/// builds a fusion owning the handler and forwarding the effect to it.
#[test]
fn handler_dependencies_fuse() {
    let program = build_program(&[("main.sv", HANDLER_DEPS_DEMO)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert!(
        c.contains("pub struct ConsoleLogger {\n}") && c.contains("pub fn new() -> Self"),
        "the dependency must not become a field or a `new` parameter:\n{c}"
    );
    assert!(
        c.contains("pub trait __Impl_ConsoleLogger")
            && c.contains("fn log<__Fx: __Has_Console>(&mut self, __fx: &mut __Fx, message: &String)"),
        "expected the member bodies in a trait taking the fused dependency \
         behind its Has bound:\n{c}"
    );
    assert!(
        c.contains("pub fn work<__Fx: __Has_Logger>(__fx: &mut __Fx)"),
        "callers must not mention the dependency, and a single effect still \
         fuses (uniformity):\n{c}"
    );
    assert!(
        c.contains("pub trait __Has_Logger {\n    fn __get_Logger(&mut self) -> &mut dyn Logger;\n}"),
        "expected the Has-accessor trait beside the effect:\n{c}"
    );
    assert!(
        c.contains("__outer: &'a mut dyn __Has_Console") && c.contains("__h: __H,"),
        "expected a fusion chaining to the provider and owning the handler:\n{c}"
    );
    assert!(
        c.contains("let Self { __outer, __h } = self;")
            && c.contains("let mut __deps = __Deps_ConsoleLogger{ __p: &mut **__outer };")
            && c.contains("__Impl_ConsoleLogger::log(__h, &mut __deps, message)"),
        "the forwarding impl must split `&mut self` into disjoint field \
         borrows before threading the dependency adapter:\n{c}"
    );
    assert!(
        c.contains("fn __get_Logger(&mut self) -> &mut dyn Logger {\n        self\n    }"),
        "a dependent handler's accessor returns the fusion itself, which \
         carries the raw effect impl:\n{c}"
    );
}

/// [rs-effect-fusion] The gate is program-wide but *narrow*: a program
/// where no handler declares a dependency keeps the per-effect `&mut dyn`
/// parameters, so nothing about existing output changes.
#[test]
fn programs_without_handler_dependencies_do_not_fuse() {
    let files = generate(&[("main.sv", NO_DEPS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert!(
        c.contains("pub fn work(logger: &mut dyn Logger, console: &mut dyn Console)"),
        "expected one `&mut dyn` parameter per effect:\n{c}"
    );
    assert!(
        !c.contains("__Fx_") && !c.contains("__Prov_") && !c.contains("__Has_"),
        "no fusion items should be generated:\n{c}"
    );
}

const NO_DEPS_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> None => message
}

handler PlainLogger of Logger {
    fn log(message: Str) -> None => message { }
}

fn work() [Logger, Console] -> None {
    log("hi")
    println("there")
}

fn main() [use] -> None {
    use StdOutConsole
    use PlainLogger
    work()
}
"#;

#[test]
fn rustc_compiles_and_runs_handler_dependencies() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", HANDLER_DEPS_DEMO)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    // The same stdout the Kotlin backend produces for this program.
    run_rust_files(&files, "handler-deps", "LOG: from work\nLOG: from main\n");
}

// ===== interception [effect-intercept] =====
// A handler that depends on the effect it implements wraps the instance
// registered before it (binds strictly outward), and a `use` may shadow an
// earlier registration — innermost wins [use-no-dup]. Both are new in
// 2026-09-14; the emission constraint is that a fusion must carry exactly
// **one** `__Has_E` impl per effect (two is E0119), so the shadowed
// instance loses its inherited accessor while staying reachable through
// `__outer` for the intercepting handler's own dependency.

const INTERCEPTION_DEMO: &str = r#"
effect Greeter {
    fn greet(name: Str) -> Str => name
}

effect Store<T> {
    fn keep(value: T) -> Str => value
}

handler Plain of Greeter {
    fn greet(name: Str) -> Str => name {
        return "hello ${name}"
    }
}

handler Formal of Greeter {
    fn greet(name: Str) -> Str => name {
        return "Good day, ${name}"
    }
}

handler Loud [local Greeter] of Greeter {
    fn greet(name: Str) -> Str => name {
        return "${greet(name)}!"
    }
}

handler Counting [local Greeter] of Greeter {
    count: Int = 0
    fn greet(name: Str) -> Str => name {
        count = count + 1
        return "${greet(name)} (${count})"
    }
}

handler MemStore<T> of Store<T> {
    fn keep(value: T) -> Str => value {
        return "kept"
    }
}

handler Twice [local Store<Int>] of Store<Int> {
    fn keep(value: Int) -> Str => value {
        return "${keep(value)} ${keep(value)}"
    }
}

fn shout(name: Str) [local Greeter, Console] -> None => name {
    println(greet(name))
}

fn main() [use] -> None {
    use StdOutConsole
    use local Plain
    shout("a")
    use local Formal
    shout("b")
    use local Counting
    shout("c")
    if true {
        use local Loud
        shout("d")
    }
    shout("e")
    use local MemStore<Int>()
    use local MemStore<Str>()
    println(keep(1))
    println(keep("x"))
    use local Twice
    println(keep(2))
    println(keep("y"))
}
"#;

/// [effect-intercept] [use-no-dup] [rs-effect-fusion] The shapes
/// interception needs from the fusion.
#[test]
fn interception_shapes() {
    let files = generate(&[("main.sv", INTERCEPTION_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    // One `__Has_E` impl per effect per fusion struct. Two is E0119, and it
    // is exactly what a shadowing `use` produced before the accessor for the
    // shadowed instance was suppressed.
    let mut checked = 0;
    for n in 1..=12 {
        for effect in ["Greeter", "Store<i32>", "Store<String>"] {
            let needle = format!("__Has_{effect} for __Fx_main_{n}<");
            let count = c.matches(needle.as_str()).count();
            checked += count;
            assert!(
                count <= 1,
                "fusion `__Fx_main_{n}` has {count} `{needle}` impls — a \
                 shadowed instance must lose its inherited accessor:\n{c}"
            );
        }
    }
    assert!(
        checked >= 8,
        "the impl scan found only {checked} accessors, so it is not testing \
         what it claims:\n{c}"
    );
    // The intercepting handler still reaches the shadowed instance, through
    // the provider it was handed: that is "binds strictly outward".
    assert!(
        c.contains("let mut __deps = __Deps_Loud{ __p: &mut **__outer };")
            && c.contains("__Impl_Loud::greet(__h, &mut __deps, name)"),
        "an intercepting member must be handed the outer instance through \
         `__outer`:\n{c}"
    );
    assert!(
        c.contains("impl<'a, __P: __Has_Greeter + ?Sized> __Has_Greeter for __Deps_Loud"),
        "the adapter's accessor forwards to the provider, not to the fusion \
         that owns the intercepting handler:\n{c}"
    );
    // A shadowed instance stays in the provider conjunction, or the
    // dependency above would have nothing to bind to.
    assert!(
        c.contains("__Has_Greeter") && c.contains("__outer: &'a mut dyn __Prov_"),
        "expected the shadowed effect to remain reachable through a provider \
         trait:\n{c}"
    );
    // [effect-intercept] Only one instance of a generic effect is
    // intercepted; the sibling keeps the handler it had.
    assert!(
        c.contains("__Deps_Twice") && c.contains("__Has_Store<i32>"),
        "expected the interception of one instance of a generic effect:\n{c}"
    );
}

#[test]
fn rustc_compiles_and_runs_interception() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", INTERCEPTION_DEMO)]);
    // The same stdout the Kotlin backend produces for this program.
    run_rust_files(
        &files,
        "interception",
        "hello a\nGood day, b\nGood day, c (1)\nGood day, d (2)!\n\
         Good day, e (3)\nkept\nkept\nkept kept\nkept\n",
    );
}

// ===== the shareable-by-default interceptor chain [use-local] =====
// The production shape the 2026-09-20 arc was built for, with no `local`
// anywhere: a stateful handler binds as a monitor, a stateless dependent
// handler binds bare and captures its dependencies as owned handles at
// construction — one of them the effect it implements (interception binds
// outward), another a monitor's handle.

const SHAREABLE_CHAIN_DEMO: &str = r#"
effect Counter {
    fn bump() -> None
    fn total() -> Int
}

handler MemCounter of Counter {
    n: Int = 0
    fn bump() { n = n + 1 }
    fn total() -> Int { return n }
}

effect Logger {
    fn log(m: Str) -> None => m
}

handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m {
        println("log: ${m}")
    }
}

handler Shout [Logger, Counter] of Logger {
    fn log(m: Str) -> None => m {
        bump()
        log("${m}!")
    }
}

fn work(step: Str) [Logger] -> None => step {
    log(step)
}

fn audit() [Counter, Console] -> None {
    println("shouted ${total()}")
}

fn main() [use] -> None {
    use StdOutConsole
    use MemCounter()
    use PlainLogger()
    work("plain")
    use Shout()
    work("loud")
    work("louder")
    audit()
}
"#;

const SHAREABLE_CHAIN_OUTPUT: &str = "log: plain\nlog: loud!\nlog: louder!\nshouted 2\n";

/// [use-local] [effect-handler-deps] [rs-monitor] The chain end to end:
/// `MemCounter` behind its lock, `PlainLogger` bare with a captured console
/// handle, `Shout` wrapping the `Logger` registered before it — and the
/// count proving both `work` calls went through the interceptor and the
/// monitor. Byte-identical stdout on Kotlin.
#[test]
fn rustc_compiles_and_runs_a_shareable_interceptor_chain() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SHAREABLE_CHAIN_DEMO)]);
    run_rust_files(&files, "shareable-chain", SHAREABLE_CHAIN_OUTPUT);
}

// ===== spawn-inheritance and the `with` clause [spawn-inherit] =====
// The arc's shape, with nothing written where nothing needs to be: the
// interception wiring lives in a fn that *received* the effects it wires (the
// lexical cut is lifted — Rust threads the handles in one hidden bundle), a
// spawned actor inherits its dependency from the spawning scope with no
// clause at all, and a `with` clause overrides one — a private instance,
// including for the self-dependency.

const SPAWN_INHERIT_DEMO: &str = r#"
effect Clock {
    fn now() -> Int
}

handler TickingClock of Clock {
    t: Int = 0
    fn now() -> Int { t = t + 1 return t }
}

effect Logger {
    fn log(m: Str) -> None => m
}

handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m { println("log: ${m}") }
}

handler Stamped [Logger, Clock] of Logger {
    fn log(m: Str) -> None => m {
        log("[t=${now()}] ${m}")
    }
}

handler FixedClock of Clock {
    fn now() -> Int { return 99 }
}

actor effect Reporter {
    send fn report(what: Str, done: Reply<Int>) => !what, !done
}

handler Reporting() [Logger] of Reporter {
    mailbox { capacity: 4 }

    send fn report(what: Str, done: Reply<Int>) {
        log("reported ${what}")
        done.send(1)
    }
}

fn work(step: Str) [Logger] -> None => step {
    log(step)
}

fn interception() [Logger, Clock, use] -> None {
    work("plain")
    use Stamped
    work("stamped")
}

fn main() [use, spawn] -> None {
    use StdOutConsole
    use TickingClock()
    use PlainLogger()
    interception()
    let r = spawn Reporting() on pool(1)
    let acked = waitfor done: Reply<Int> {
        r.report("inherited", done)
    }
    let sink = acked
    use Stamped with FixedClock()
    work("overridden")
}
"#;

const SPAWN_INHERIT_OUTPUT: &str =
    "log: plain\nlog: [t=1] stamped\nlog: reported inherited\nlog: [t=99] overridden\n";

/// [spawn-inherit] [with-clause] [rs-handle-bundle] The arc end to end:
/// a signature-supplied effect captured into a handler (the hidden `__Hs_N`
/// bundle), a spawn inheriting its dependency from the scope, and a `with`
/// clause overriding the self-dependency with a private instance.
/// Byte-identical stdout on Kotlin.
#[test]
fn rustc_compiles_and_runs_spawn_inheritance() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SPAWN_INHERIT_DEMO)]);
    run_rust_files(&files, "spawn-inherit", SPAWN_INHERIT_OUTPUT);
}

/// [rs-handle-bundle] The shapes the lift needs: one hidden bundle parameter
/// after the fused value, built at the call site from the caller's own eager
/// handles, and read as fields at the capture.
#[test]
fn a_handle_bundle_threads_signature_supplied_effects() {
    let files = generate(&[("main.sv", SPAWN_INHERIT_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert!(
        c.contains("pub fn interception<__Fx: __Has_Logger + __Has_Clock>(__fx: &mut __Fx, __hs: &__Hs_1)"),
        "expected one hidden bundle parameter after the fused value:\n{c}"
    );
    assert!(
        c.contains("pub struct __Hs_1 {") && c.contains("pub logger: crate::__Mon_Logger,"),
        "expected the generated bundle struct with a handle per effect:\n{c}"
    );
    assert!(
        c.contains("Stamped::new(__hs.logger.clone(), __hs.clock.clone())"),
        "the capture must read the bundle's fields:\n{c}"
    );
    assert!(
        c.contains("interception(&mut __fx3, &__Hs_1 {"),
        "the caller must build the bundle from its own handles:\n{c}"
    );
}

/// [use-local] [effect-handler-deps] A **generic** dependent handler works
/// in the owned-handle form: `Relay<T>`'s dependency is a handle field, so
/// no generated trait has to name the handler's generics — the cut that
/// still stands for the fusion form (`generic_dependent_handler_is_a_
/// codegen_error`) does not bind here.
#[test]
fn rustc_compiles_and_runs_a_generic_handler_with_a_handle_dep() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    const SRC: &str = r#"
effect Sink<T> {
    fn accept(value: T) -> Str => value
}

handler Relay<T> [Console] of Sink<T> {
    fn accept(value: T) -> Str => value {
        println("relayed")
        return "ok"
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use Relay<Int>()
    println(accept(1))
}
"#;
    let files = generate(&[("main.sv", SRC)]);
    run_rust_files(&files, "generic-handle-dep", "relayed\nok\n");
}

/// [rs-effect-fusion] Identical fusions are **one** struct (user decision
/// 2026-09-14): two fns registering the same handler over the same inherited
/// set were emitting a struct and a full set of impls each.
#[test]
fn identical_fusions_are_one_struct() {
    let src = r#"
export effect Logger {
    fn log(m: Str) -> None => m
}

export handler PlainLogger [local Console] of Logger {
    fn log(m: Str) -> None => m {
        println("log: ${m}")
    }
}

export fn first() [Console, use] -> None {
    use local PlainLogger()
    log("first")
}

export fn second() [Console, use] -> None {
    use local PlainLogger()
    log("second")
}

export fn main() [use] -> None {
    use StdOutConsole
    first()
    second()
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert_eq!(
        c.matches("pub struct __Fx_").count(),
        2,
        "expected one fusion struct per *shape* — one for each `use` site \
         shape, not one per site:\n{c}"
    );
    assert_eq!(
        c.matches("__Fx_first_1 { __outer:").count(),
        2,
        "both fns should construct the shared struct:\n{c}"
    );
}

// ===== the fusion in anger [rs-effect-fusion] =====
// Handler state behind a dependency, a *two*-dependency handler whose
// second dependency is another handler's effect, a fn needing two effects
// calling one needing a subset, nested `use` scopes with the outer one used
// again afterwards, a `use` inside a fn that already has effects, and two
// effect calls in one expression (which must not borrow the fused value
// twice).

const FUSION_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> T
}

effect Counter {
    fn bump() -> None
    fn total() -> Int
}

effect Logger {
    fn log(message: Str) -> None => message
}

effect Audit {
    fn note(message: Str) -> None => message
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

handler MemCounter of Counter {
    n: Int = 0
    fn bump() -> None { n = n + 1 }
    fn total() -> Int { return n }
}

handler ConsoleLogger [local Console] of Logger {
    seen: Int = 0
    fn log(message: Str) -> None => message {
        seen = seen + 1
        println("LOG ${seen}: ${message}")
    }
}

handler CountingAudit [local Console, local Counter] of Audit {
    fn note(message: Str) -> None => message {
        bump()
        println("[${total()}] ${message}")
    }
}

fn shout(message: Str) [Console] -> None => message {
    println("!! ${message}")
}

fn banner() [Console, local Logger] -> None {
    log("banner")
    shout("done")
}

fn draw() [Console, local Random<Int>, use] -> None {
    use local MemCounter
    bump()
    println("drew ${next_random<Int>()} at ${total()}")
}

fn main() [use] -> None {
    use StdOutConsole
    use local ConsoleLogger()
    banner()
    if true {
        use local MemCounter
        use local CountingAudit()
        note("inner")
        banner()
    }
    log("outer again")
    use local CyclicRandom(array_of(10, 20, 30))
    draw()
    draw()
}
"#;

#[test]
fn fusion_shapes() {
    let files = generate(&[("main.sv", FUSION_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    // A fn needing effects takes one *generic* fused value bounded by the
    // Has-accessor traits — never the effect traits, so member names cannot
    // collide on it — and forwards it to a callee needing a subset.
    assert!(
        c.contains("pub fn banner<__Fx: __Has_Console + __Has_Logger>(__fx: &mut __Fx)")
            && c.contains("shout(&mut *__fx,"),
        "expected a Has-bounded fused parameter forwarded to a smaller callee:\n{c}"
    );
    // Dependencies need a Sized Has-implementing view over the single
    // provider field — uniformly, one dependency or two.
    assert!(
        c.contains("pub struct __Deps_CountingAudit<'a, __P: ?Sized>")
            && c.contains("let mut __deps = __Deps_CountingAudit{ __p: &mut **__outer };")
            && c.contains("fn note<__Fx: __Has_Console + __Has_Counter>(&mut self, __fx: &mut __Fx"),
        "expected the dependency adapter and a Has-bounded generic member:\n{c}"
    );
    // Provider traits exist only as `__outer` field types, and conjoin the
    // Has traits, never the effects.
    assert!(
        c.contains("pub trait __Prov_Console_Logger: __Has_Console + __Has_Logger {}")
            && c.contains(
                "impl<T: __Has_Console + __Has_Logger + ?Sized> __Prov_Console_Logger for T {}"
            ),
        "expected a provider trait over Has supertraits with its blanket impl:\n{c}"
    );
    // Member dispatch is accessor-then-method: the Has trait's turbofish
    // disambiguates generic instances, the member call is on `&mut dyn E`.
    assert!(
        c.contains("__Has_Random::<i32>::__get_Random(&mut __fx"),
        "expected accessor dispatch for a generic effect:\n{c}"
    );
    // Two effect calls in one expression: the inner one is hoisted, or the
    // fused value would be borrowed twice (`E0499`).
    assert!(
        c.contains("{ let __a1 = &(format!(\"drew {} at {}\""),
        "expected nested effect calls to be hoisted into a temporary:\n{c}"
    );
}

#[test]
fn rustc_compiles_and_runs_fusion() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FUSION_DEMO)]);
    run_rust_files(
        &files,
        "fusion",
        "LOG 1: banner\n!! done\n[1] inner\nLOG 2: banner\n!! done\n\
         LOG 3: outer again\ndrew 20 at 1\ndrew 30 at 1\n",
    );
}

/// [backend-never-wrong] A dependent handler whose *generated* trait would
/// have to name the handler's own generic parameters (a dependency or member
/// signature mentioning them) is reported: the fusion cannot supply that
/// type argument, because it never derives the handler's generics.
#[test]
fn generic_dependent_handler_is_a_codegen_error() {
    const SRC: &str = r#"
export effect Sink<T> {
    fn accept(value: T) -> None => value
}

export handler Relay<T> [local Console] of Sink<T> {
    fn accept(value: T) -> None => value {
        println("relayed")
    }
}

export fn main() [use] -> None {
    use StdOutConsole
    use local Relay<Int>()
    accept(1)
}
"#;
    let program = build_program(&[("main.sv", SRC)]);
    let errors = match salvo_backend_rust::emit_program(&program) {
        Ok(_) => panic!("expected a codegen error for a generic dependent handler"),
        Err(errors) => errors,
    };
    let msg = errors
        .iter()
        .find(|e| e.contains("generic parameters"))
        .unwrap_or_else(|| panic!("got {errors:?}"));
    assert!(
        msg.contains("Relay") && msg.contains("fuse"),
        "the cut should name the handler and the mechanism: {msg}"
    );
}

// [rs-effect-fusion] A dependency *chain* (Audit needs Logger needs
// Console), a `use` inside a loop body, effect calls nested in another
// call's arguments, and a predicate qualifier whose `qualifies` declares an
// effect — all through one fused value.

const FUSION_CHAIN_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> None => message
}

effect Audit {
    fn note(message: Str) -> None => message
}

effect Tally {
    fn add_up(n: Int) -> None => n
    fn tally() -> Int
}

handler MemTally of Tally {
    sum: Int = 0
    fn add_up(n: Int) -> None => n { sum = sum + n }
    fn tally() -> Int { return sum }
}

handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

handler LoggingAudit [Logger] of Audit {
    count: Int = 0
    fn note(message: Str) -> None => message {
        count = count + 1
        log("note ${count}: ${message}")
    }
}

qualifier Loud of Str {
    fn qualifies(text: Str) [Console] -> Bool {
        println("checking ${text}")
        return text.size() > 3
    }
}

fn label(n: Int) [Console] -> Str {
    println("labelling ${n}")
    return "n=${n}"
}

fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger()
    use LoggingAudit()
    note("first")
    for i in [1, 2] {
        use MemTally
        add_up(i)
        log("loop ${i} tally ${tally()}")
    }
    log(label(7))
    let text = "hello"
    if text is Loud {
        note("loud")
    }
}
"#;

#[test]
fn rustc_compiles_and_runs_fusion_chain() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FUSION_CHAIN_DEMO)]);
    run_rust_files(
        &files,
        "fusion-chain",
        "LOG: note 1: first\nLOG: loop 1 tally 1\nLOG: loop 2 tally 2\n\
         labelling 7\nLOG: n=7\nchecking hello\nLOG: note 2: loud\n",
    );
}

// [rs-effect-fusion] The `use` site and the dependent handler may live in
// different modules: the generated `__Impl_H` trait travels with the
// handler and arrives through the module's glob import [rs-imports].
const FUSION_LOGGING_MODULE: &str = r#"
export effect Logger {
    fn log(message: Str) -> None => message
}

export handler ConsoleLogger [Console] of Logger {
    tag: Str = "M"
    fn log(message: Str) -> None => message {
        println("${tag}: ${message}")
    }
}

export fn work() [Logger] -> None {
    log("from work")
}
"#;

const FUSION_MAIN_MODULE: &str = r#"
import logging.work
import logging.Logger
import logging.ConsoleLogger

fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger()
    work()
    log("from main")
}
"#;

#[test]
fn rustc_compiles_and_runs_cross_module_fusion() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[
        ("logging.sv", FUSION_LOGGING_MODULE),
        ("main.sv", FUSION_MAIN_MODULE),
    ]);
    run_rust_files(&files, "fusion-cross", "M: from work\nM: from main\n");
}

// [rs-effect-fusion] Constructor parameters mixing a dependency with plain
// data, a dependent member calling a fn that does its own `use`, an
// effect-using lambda passed to an effect-*free* higher-order fn (allowed:
// the call threads nothing, so nothing aliases), three effects in one
// signature, a `use` inside a `while` body, and an effect member taking a
// `Mut` parameter.

const FUSION_MIXED_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> None => message
}

effect Sink {
    fn keep(items: Mut List<Int>) -> None => !items
    fn kept() -> Int
}

effect Counter {
    fn bump() -> None
    fn total() -> Int
}

handler PrefixLogger(prefix: Str, level: Int) [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("${prefix}[${level}] ${message}")
        tallied(message)
    }
}

handler MemSink of Sink {
    held: Mut List<Int> = mut_list_of()
    fn keep(items: Mut List<Int>) -> None => !items { held = items }
    fn kept() -> Int { return size(held) }
}

handler MemCounter of Counter {
    n: Int = 0
    fn bump() -> None { n = n + 1 }
    fn total() -> Int { return n }
}

fn tallied(text: Str) [Console, use] -> None => text {
    use MemSink
    let xs: Mut List<Int> = mut_list_of()
    add(xs, size(text))
    keep(xs)
    println("  tallied ${kept()}")
}

fn twice(f: (s: Str) [Logger] -> Str) -> Str =>[f] s {
    return f("a")
}

fn report(label: Str) [Console, Logger, Counter] -> None => label {
    bump()
    log("${label} #${total()}")
}

fn main() [use] -> None {
    use StdOutConsole
    use PrefixLogger("L", 3)
    use MemCounter
    log(twice(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }))
    report("one")
    let i = 0
    while i < 2 {
        use MemSink
        let ys: Mut List<Int> = mut_list_of()
        add(ys, copy(i))
        keep(ys)
        report("loop ${kept()}")
        i = i + 1
    }
}
"#;

const FUSION_MIXED_STDOUT: &str = "L[3] in lambda a\n  tallied 1\nL[3] done a\n\
     \x20 tallied 1\nL[3] one #1\n  tallied 1\nL[3] loop 1 #2\n  tallied 1\n\
     L[3] loop 1 #3\n  tallied 1\n";

#[test]
fn rustc_compiles_and_runs_fusion_mixed() {
    if Command::new("rustc").arg("--version").output().is_err() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FUSION_MIXED_DEMO)]);
    run_rust_files(&files, "fusion-mixed", FUSION_MIXED_STDOUT);
}

/// [fn-effects] The program the fusion used to lose to Kotlin: a function
/// *value* that uses an effect, passed to a callee that needs one too.
/// Capturing made the closure hold a borrow that overlapped the call's own
/// (`E0499`), and hoisting could not separate them. Threading the effect
/// *into* the value lifts it: the closure captures nothing, so the two
/// borrows are of different things.
#[test]
fn rustc_compiles_and_runs_effect_using_fn_value() {
    const SRC: &str = r#"
export effect Logger {
    fn log(message: Str) -> None => message
}

export handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

export fn run_it(f: (s: Str) [Logger] -> Str) [Console] -> None =>[f] s {
    println(f("x"))
}

export fn demo() [Console, Logger] -> None {
    run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    })
}

export fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger()
    demo()
}
"#;
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SRC)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The effect arrives as a parameter of the closure, not a capture —
    // under the fusion, as the single `dyn` provider the closure combines
    // into a Sized fused value [rs-effect-fusion].
    assert!(
        main.content.contains(": &mut dyn __Has_Logger,")
            && main.content.contains("{ __outer: __prov }"),
        "expected the effect threaded into the closure as a Has provider:\n{}",
        main.content
    );
    run_rust_files(&files, "fn-effects", "LOG: in lambda x\ndone x\n");
}

// ===== operator typing: widening, literal adoption, conversions =====
// [op-arith] [op-promote] [lit-adopt] [op-convert] The same source and
// expected stdout as the Kotlin backend's case [backend-parity]: `Long`
// arithmetic with widened `Int` operands, adopted literals, mixed-width
// ordering, and the explicit conversions. Values are chosen fractional
// where floats print, since whole-double rendering differs between the
// backends today (open defect).

const OPERATOR_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole
    let x: Long = 1
    let n = 5
    let y = x + n * 2
    let big = 4000000000L
    let scaled = big * 2 + n
    let d: Double = 3
    let g = d + to_double(n) + 0.25
    let f: Float = 0.5
    let h = f * 1.5f
    let cmp = n < x
    let back = to_int(big)
    let neg: Long = -7
    println("${y} ${scaled} ${g} ${h} ${cmp} ${back} ${neg}")
}
"#;

const OPERATOR_STDOUT: &str = "11 8000000005 8.25 0.75 false -294967296 -7\n";

/// [op-promote] [lit-adopt] Rust has no mixed-width operators, so widened
/// operands cast (`as i64`), and adopted literals render at their checked
/// type (`1i64`).
#[test]
fn promotions_cast_and_literals_adopt() {
    let files = generate(&[("main.sv", OPERATOR_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    let c = &main.content;
    assert!(
        c.contains("x: i64 = 1i64;"),
        "expected the adopted literal suffixed:\n{c}"
    );
    assert!(
        c.contains("x + ((n * 2) as i64)"),
        "expected the widened operand cast as a whole:\n{c}"
    );
    assert!(
        c.contains("as i64) < x"),
        "expected the ordering operand cast:\n{c}"
    );
    // [op-convert] The operand is parenthesized inside the cast: `as` binds
    // tighter than unary minus, so `-1 as u8` would be `-(1 as u8)` — which
    // rustc refuses for `to_byte(-1)` and would have meant the wrong thing
    // for a saturating target [byte-value].
    assert!(
        c.contains("((big) as i32)"),
        "expected the conversion intrinsic lowered to a cast:\n{c}"
    );
}

#[test]
fn rustc_compiles_and_runs_operators() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", OPERATOR_DEMO)]);
    run_rust_files(&files, "operators", OPERATOR_STDOUT);
}

// ===== [effect-at] [effect-member-overload] shared member names =====
// The same source and expected stdout as the Kotlin backend's case
// [backend-parity]: two effects declaring `close`, resolved by
// availability in `shut` and by the `@Effect` selector in `main`.

const EFFECT_AT_DEMO: &str = r#"
effect Store {
    fn open(path: Str) -> Int => path
    fn close(handle: Int) -> Str
}

effect Net {
    fn close(handle: Int) -> Str
}

handler MemStore of Store {
    fn open(path: Str) -> Int => path {
        return 7
    }
    fn close(handle: Int) -> Str {
        return "fs closed ${handle}"
    }
}

handler MemNet of Net {
    fn close(handle: Int) -> Str {
        return "net closed ${handle}"
    }
}

fn shut(h: Int) [Store] -> Str {
    return close(h)
}

fn main() [use] -> None {
    use StdOutConsole
    use MemStore()
    use MemNet()
    let h = open("a.txt")
    println(shut(h))
    println(close@Store(h))
    println(close@Net(9))
    println(h.close@Net())
}
"#;

const EFFECT_AT_STDOUT: &str = "fs closed 7\nfs closed 7\nnet closed 9\nnet closed 7\n";

#[test]
fn rustc_compiles_and_runs_effect_selectors() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", EFFECT_AT_DEMO)]);
    run_rust_files(&files, "effect-at", EFFECT_AT_STDOUT);
}



// ===== destructuring a loop element [let-destructure] =====

/// [let-destructure] A `for` binds a **pattern**, not just a name: the header
/// binds the element to a temporary and the body opens with the pattern's
/// bindings. Expected output is verbatim the Kotlin backend's.
const LOOP_DESTRUCTURE: &str = r#"
struct Row { name: Str, score: Int }

fn main() [use] -> None {
    use StdOutConsole
    // [let-destructure] A loop element destructures like a `let`: a tuple by
    // position, a struct by field (renamed where the pattern says so).
    let pairs: List<(Str, Int)> = [("a", 1), ("b", 2)]
    for (k, v) in iter(pairs) {
        println("${k}=${v}")
    }
    let rows: List<Row> = [Row { name: "x", score: 7 }, Row { name: "y", score: 8 }]
    for {name: who, score} in iter(rows) {
        println("${who} ${score}")
    }
    // A binding the body assigns to is the local's own, not the element's.
    let nums: List<(Int, Int)> = [(1, 2), (3, 4)]
    for (a, b) in iter(nums) {
        a = a + b
        println("${a}")
    }
    // In value position, and nested — two loops, two element temporaries.
    let sum = for (x, y) in iter(pairs) {
        size(x) + y
    } else {
        0
    }
    println("sum ${sum}")
}
"#;

const LOOP_DESTRUCTURE_OUTPUT: &str = "a=1\nb=2\nx 7\ny 8\n3\n7\nsum 3\n";

#[test]
fn rustc_compiles_and_runs_loop_destructuring() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LOOP_DESTRUCTURE)]);
    run_rust_files(&files, "loop-destructure", LOOP_DESTRUCTURE_OUTPUT);
}

/// [let-destructure] [rs-borrow-locals] What it lowers to: the element in a
/// temporary, then one binding per name — **by reference**, which is what makes
/// one shape serve an owned element and a borrowed one (a pattern with `mut`
/// bindings cannot move out of a pass's projection, `E0507`). A name the body
/// assigns to takes an owned copy instead, since a reference cannot be
/// reassigned.
#[test]
fn loop_destructuring_binds_the_parts_of_a_temporary() {
    let files = generate(&[("main.sv", LOOP_DESTRUCTURE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("let k = &__elem.0;") && text.contains("let v = &__elem.1;"),
        "expected the tuple parts bound by reference:\n{text}"
    );
    assert!(
        text.contains("let who = &__elem2.name;") && text.contains("let score = &__elem2.score;"),
        "expected the struct fields bound by reference, renamed:\n{text}"
    );
    assert!(
        text.contains("let mut a = __elem3.0.clone();"),
        "expected an assigned-to binding to own its copy:\n{text}"
    );
}

// ===== tuples past three [type-tuple] =====

/// [type-tuple] Rust tuples are native at any arity, so the interesting half
/// of this case is the *parity*: the expected output is verbatim the Kotlin
/// backend's, where the same program needs a generated tuple class
/// [kt-tuple-class].
const BIG_TUPLE_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole
    // [type-tuple] Past three elements Kotlin has no tuple type of its own,
    // so the backend generates one; Rust's are native at any arity.
    let q: (Int, Str, Bool, Int, Str) = (1, "two", true, 4, "five")
    println("${q.0} ${q.1} ${q.2} ${q.3} ${q.4}")
    let (a, b, c, d, e) = q
    println("${a} ${b} ${c} ${d} ${e}")
    // Ordered *and* hashed, so the generated class has to compare and hash
    // structurally like a `Pair` does.
    let keys: SortedSet<(Int, Int, Int, Int)> = sorted_set_of((2, 0, 0, 0), (1, 9, 9, 9))
    for k in iter(keys) {
        println("${k.0}${k.1}${k.2}${k.3}")
    }
}
"#;

const BIG_TUPLE_OUTPUT: &str = "1 two true 4 five\n1 two true 4 five\n1999\n2000\n";

#[test]
fn rustc_compiles_and_runs_tuples_past_three() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", BIG_TUPLE_DEMO)]);
    run_rust_files(&files, "big-tuple", BIG_TUPLE_OUTPUT);
}

// ===== union coercion inside arrays/tuples/lambda returns =====
// [type-union] Elements of array/tuple literals and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.
// Also covers: a fn-type `let` annotation is dropped in Rust
// (`impl Trait` is invalid on bindings [fn-contract]).

const NESTED_COERCION_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

type Result = Ok Int | Err Str

fn tag_ok<T>(value: T) -> +Ok T {
    return value
}

fn tag_err<T>(value: T) -> +Err T {
    return value
}

fn describe(r: Result) -> Str {
    if r is Ok {
        return "ok ${r}"
    }
    return "err ${r}"
}

fn main() [use] -> None {
    use StdOutConsole
    let arr: List<Result> = [tag_ok(1), tag_err("a")]
    for x in arr {
        println(describe(copy(x)))
    }
    let tup: (Str, Result) = ("t", tag_ok(2))
    let (label, r) = tup
    println(describe(r))
    let make: (flag: Bool) -> Result = (flag: Bool) -> {
        return if flag { tag_ok(3) } else { tag_err("b") }
    }
    println(describe(make(true)))
    println(describe(make(false)))
}
"#;

#[test]
fn union_coercion_in_array_tuple_lambda() {
    let files = generate(&[("main.sv", NESTED_COERCION_DEMO)]);
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
        "Union2::<i32, String>::U1(tag_ok(1))",
        "Union2::<i32, String>::U2(tag_err(\"a\".to_string()))",
        "Union2::<i32, String>::U1(tag_ok(2))",
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
    let files = generate(&[("main.sv", NESTED_COERCION_DEMO)]);
    run_rust_files(
        &files,
        "nested-coercion",
        "ok 1\nerr a\nok 2\nok 3\nerr b\n",
    );
}

// ===== effect member fns with their own generics =====
// [effect-member-generics] [rs-effects] `dyn` traits cannot have generic
// methods: the rust backend rejects them loudly instead of emitting
// invalid code.
#[test]
fn effect_member_generics_are_rejected_loudly() {
    let src = r#"
export effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

export handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T {
        return a
    }
}

export fn main() [use] -> None {
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
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        let value = get(values, i % values.size())!
        i = i + 1
        return copy(value)
    }
}

fn roll() [local Random<Count>] -> Count {
    return next_random()
}

fn main() [use] -> None {
    use StdOutConsole
    use local CyclicRandom(list_of(7, 8))
    println("${roll()} ${roll()}")
}
"#;

#[test]
fn aliased_effect_types_resolve_to_the_same_handler() {
    let files = generate(&[("main.sv", ALIASED_EFFECT_DEMO)]);
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
    let files = generate(&[("main.sv", ALIASED_EFFECT_DEMO)]);
    run_rust_files(&files, "aliased-effects", "7 8\n");
}

// ===== std array functions =====
// [type-array] Arrays get the `core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02). `T[]` and `List<T>` share the `Vec<T>`
// rendering, so their lowerings mirror each other.

const ARRAY_STD_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use CyclicRandom(array_of(1, 2, 3, 4))
    let nums: Int[] = array_of(3, 4, 5)
    println("size ${nums.size()} get ${nums.get(2)!} first ${nums.first()!}")
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
"#;

#[test]
fn array_std_functions_lower() {
    let files = generate(&[("main.sv", ARRAY_STD_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    for needle in [
        "nums.len() as i32",
        "nums.get((2) as i64 as usize)",
        "nums.first()",
    ] {
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
    let files = generate(&[("main.sv", ARRAY_STD_DEMO)]);
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

fn tag(value: Str) -> +Environment.Tag Str {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", DOT_NAMES)]);
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
    run_rust_files(
        &files,
        "dot_names",
        "prod / Production\ntagged t1\nplain t2\n",
    );
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

fn tag(v: Str) -> +Environment.Tag Str {
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

fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", DOT_NAME_UNIONS)]);
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

fn check(l: Mut List<Int>) -> +Checked Mut List<Int> {
    return l
}

fn trust(l: Mut List<Int>) -> +Trusted Mut List<Int> {
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

fn main() [use] -> None {
    use StdOutConsole()
    let t = trust(mut_list_of(1, 2))
    t.add(3)
    println(describe(t))
    let c = check(mut_list_of(1, 2))
    c.add(3)
    println(describe(c))
    let c2 = check(mut_list_of(4, 5))
    println(describe(c2))
}
"#;

// [qual-subject] [deduce-syntax] [qual-erasure] Provenance survives a
// mutating call where state does not — and both subjects erase, so the
// difference shows up only in which overload the checker picked.
#[test]
fn provenance_survives_mutation_where_state_does_not() {
    let program = build_program(&[("main.sv", QUAL_SUBJECTS)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_rust_files(&files, "qual_subjects", "trusted 3\nplain 3\nchecked 2\n");
}

// ===== E3 step 2: throw and `try` [throw] [try] [rs-throw-controlflow] =====

/// The whole of non-resumption in one program: propagation through a frame
/// that declares the effect, a linear resource released *before* the call that
/// may throw (which is the only way since `defer` was removed, 2026-09-10 —
/// the checker rejects holding it across the call), two message types meeting
/// at one delimiter
/// (`Thrown (Str | Int)`), a may-throw call inside a loop, and a nested
/// delimiter that must not swallow the outer throw.
const THROW_DEMO: &str = r#"
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


fn open_file(n: Int) [Console] -> FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn close_file(h: FileHandle) [Console] -> None => !h {
    println("close fd=${h.fd}")
    close(h)
}

fn parse(line: Str) [Throw<Str>, Console] -> Int => !line {
    println("parse ${line}")
    if size(line) == 0 {
        throw("empty line")
    }
    return size(line)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn measure(line: Str) [Throw<Str>, Console] -> Int => !line {
    let h = open_file(1)
    let fd = copy(h.fd)
    close_file(h)
    let n = parse(line)
    return n + fd
}

fn total(lines: Str[]) [Throw<Str>, Console] -> Int => !lines {
    let sum = 0
    for line in lines {
        let inner = try {
            limit(size(line))
        }
        when inner {
            is Ok {
                println("within limit ${inner}")
            }
            is Thrown {
                println("over limit ${inner}")
            }
        }
        sum = sum + parse(line)
    }
    return sum
}

fn report_text(outcome: Ok Int | Thrown Str) [Console] -> None => !outcome {
    when outcome {
        is Ok {
            println("ok ${outcome}")
        }
        is Thrown {
            println("thrown: ${outcome}")
        }
    }
}

fn main() [use] -> None {
    use StdOutConsole
    report_text(try { measure("hello") })
    report_text(try { measure("") })
    let mixed = try {
        let n = measure("longer line")
        limit(n)
    }
    when mixed {
        is Ok {
            println("mixed ok ${mixed}")
        }
        is Thrown {
            println("mixed thrown")
        }
    }
    let counted = try {
        total(["ab", "cdefg"])
    }
    when counted {
        is Ok {
            println("counted ${counted}")
        }
        is Thrown {
            println("counted thrown: ${counted}")
        }
    }
    println("done")
}
"#;

const THROW_OUTPUT: &str = "open 1\nclose fd=1\nparse hello\nok 6\n\
                            open 1\nclose fd=1\nparse \nthrown: empty line\n\
                            open 1\nclose fd=1\nparse longer line\nmixed thrown\n\
                            within limit 2\nparse ab\nover limit 5\nparse cdefg\n\
                            counted 7\ndone\n";

/// [rs-throw-controlflow] A fn that may throw returns `ControlFlow<M, T>`:
/// the message type *is* the `Break` payload, so `throw` is a plain return
/// and propagation is `?` — no handler, no dispatch, no allocation. `Throw`
/// is never a `&mut dyn` parameter.
#[test]
fn throw_lowers_to_controlflow() {
    let files = generate(&[("main.sv", THROW_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains(
            "pub fn parse(console: &mut dyn Console, line: String) -> ControlFlow<String, i32>"
        ),
        "expected a ControlFlow return shape in:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("return ControlFlow::Break(\"empty line\".to_string());"),
        "expected `throw` to return Break in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("return ControlFlow::Continue("),
        "expected returns to wrap in Continue in:\n{}",
        main.content
    );
    // [throw] The effect emits no trait: there are no handlers to implement.
    let std_throw = files
        .iter()
        .find(|f| f.rel_path.ends_with("core/throw.rs"))
        .map(|f| f.content.clone())
        .unwrap_or_default();
    assert!(
        !std_throw.contains("trait Throw"),
        "expected no trait for the throw effect in:\n{std_throw}"
    );
    // [rs-exit-splice] With nothing pending at the exit, propagation is the
    // plain `?`: the release was written before the call, so the throw path
    // owes nothing. (Until 2026-09-10 a `defer` here forced the long form —
    // a `match` on the `ControlFlow` with the release spliced into the
    // `Break` arm.)
    let measure = main
        .content
        .split("pub fn measure")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert!(
        measure.contains("close_file(console, h);")
            && !measure.contains("ControlFlow::Break(__m) => {"),
        "expected plain `?` propagation with the release ahead of the call in:\n{measure}"
    );
}

/// [rs-try-label] `try` is a *labelled block*, not a closure: nothing is
/// captured (the body reads the fn's effect parameters directly), and an
/// throw inside it breaks the label with the outcome's thrown arm.
#[test]
fn try_lowers_to_a_labelled_block() {
    let files = generate(&[("main.sv", THROW_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("'try_1: {"),
        "expected a labelled block in:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("|| -> ControlFlow"),
        "expected no closure lowering in:\n{}",
        main.content
    );
    // Two message types meeting at one delimiter wrap into the message
    // union's arms [union-arm-identity]; a single type stays bare.
    assert!(
        main.content.contains("Union2::<String, i32>::U2(__m)"),
        "expected the message wrapped into its arm in:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_throw() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", THROW_DEMO)]);
    run_rust_files(&files, "throw", THROW_OUTPUT);
}

// ===== E3 step 3: effects threaded into fn values [fn-effects] =====

/// The same program the Kotlin suite runs: an effect-using fn value passed
/// to a callee that needs one too (what the fusion cut rejected), a named fn
/// whose effect list matches, and a *pure* named fn in the same position.
/// Identical stdout on both backends is what parity means here.
const FN_EFFECTS_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> None => message
}

handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

fn shout(s: Str) [Logger] -> Str => s {
    log("shouting ${s}")
    return "${s}!"
}

fn plain(s: Str) -> Str => s {
    return "${s}."
}

// No effect list of its own: `run_it` *inherits* `[Logger]` from `f`.
fn run_it(f: (s: Str) [Logger] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn demo() [Console, Logger] -> None {
    println(run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }, "x"))
    println(run_it(shout, "one"))
    println(run_it(plain, "two"))
}

fn main() [use] -> None {
    use StdOutConsole
    use ConsoleLogger()
    demo()
}
"#;

const FN_EFFECTS_STDOUT: &str = "LOG: in lambda x\ndone x\nLOG: shouting one\none!\ntwo.\n";

/// [fn-effects] The effect is a leading parameter of the closure type, so
/// nothing is captured — which is what lets the value cross a call that
/// borrows the same effect value (the lifted fusion cut). Under the fusion
/// it is the single `&mut dyn` Has-provider [rs-effect-fusion]; the lambda
/// and the named-fn adapter rebuild a Sized fused value from it.
#[test]
fn fn_type_effects_thread_into_closures() {
    let files = generate(&[("main.sv", FN_EFFECTS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("f: &mut impl FnMut(&mut dyn __Has_Logger, &String) -> String"),
        "expected the Has provider in the closure type:\n{}",
        main.content
    );
    assert!(
        main.content.contains(": &mut dyn __Has_Logger,")
            && main.content.contains("{ __outer: __prov }"),
        "expected the provider as a leading closure parameter, combined in \
         the body:\n{}",
        main.content
    );
    // The adapter for a named fn: takes the expected provider, rebuilds a
    // fused value for a declaration that needs it (`shout`).
    assert!(
        main.content.contains("shout(&mut __fx"),
        "expected the named-fn adapter to forward a fused value:\n{}",
        main.content
    );
    assert!(
        main.content.contains("plain(__a0)"),
        "expected the pure fn to ignore the threaded effect:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_fn_type_effects() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FN_EFFECTS_DEMO)]);
    run_rust_files(&files, "fn-type-effects", FN_EFFECTS_STDOUT);
}

// ===== the widening check `^` [qual-lift] =====

// [qual-lift] `^` tests the arm *and* removes the claim, so a branch can
// `when` the union inside a qualified one — the shape `try` outcomes produce.
const WIDEN_DEMO: &str = r#"
fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {
    if n < 0 {
        throw("negative")
    }
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn describe(n: Int) [Console] -> None {
    let nested = try { wrapped(n) }
    when nested {
        is ^Ok {
            when nested {
                is Ok {
                    println("value ${nested}")
                }
                is Err {
                    println("error ${nested}")
                }
            }
        }
        is Thrown {
            println("thrown ${nested}")
        }
    }
}

fn main() [use] -> None {
    use StdOutConsole
    describe(7)
    describe(0)
    describe(-1)
    let mixed = try {
        let a = wrapped(1)
        limit(9)
    }
    when mixed {
        is Ok {
            println("mixed ok ${mixed}")
        }
        is ^Thrown {
            when mixed {
                is Str {
                    println("message text ${mixed}")
                }
                is Int {
                    println("message number ${mixed}")
                }
            }
        }
    }
}
"#;

const WIDEN_STDOUT: &str = "value 7\nerror zero\nthrown negative\nmessage number 9\n";

/// [qual-lift] The peel is *materialized*: the widened value is bound to a
/// shadowing local, so a nested `when` scrutinizes the inner union rather
/// than the wrapper it came out of. Without it the inner match reads the
/// outer arm and the second branch is dead code — which is what this program
/// caught while `^` was being built.
#[test]
fn widening_materializes_the_peel() {
    let files = generate(&[("main.sv", WIDEN_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("let mut nested = nested.u1().clone();"),
        "expected the widened value bound to a shadowing local:\n{}",
        main.content
    );
    // The nested `when` then matches on the shadowed (inner) value.
    let describe = main
        .content
        .split("pub fn describe")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert_eq!(
        describe.matches("match nested {").count(),
        2,
        "expected an outer and an inner match in:\n{describe}"
    );
}

#[test]
fn rustc_compiles_and_runs_widening() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", WIDEN_DEMO)]);
    run_rust_files(&files, "widen", WIDEN_STDOUT);
}

// ===== the subject-less `when` [when-condition] [rs-when-cond] =====

// [when-condition] A condition chain with a mandatory `else`, in value and
// statement position, with `is` heads that narrow, a chain that returns from
// every branch, and one nested inside a subject `when`'s arm.
const WHEN_COND_DEMO: &str = r#"
fn classify(n: Int) -> Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

fn sign(n: Int) -> Int {
    when {
        n < 0 { return -1 }
        n > 0 { return 1 }
        else { return 0 }
    }
}

fn describe(value: Str | Int) [Console] -> None => !value {
    when {
        value is Str s { println("str ${s}") }
        else { println("int ${value}") }
    }
}

fn label(n: Int) [Console] -> Str {
    let tag = when {
        n < 10 { "small" }
        n < 100 {
            println("  medium branch")
            "medium"
        }
        else { "large" }
    }
    return tag
}

fn nested(o: Ok Int | Err Str) [Console] -> None => !o {
    when o {
        is Ok {
            when {
                o > 0 { println("positive ok ${o}") }
                else { println("nonpositive ok ${o}") }
            }
        }
        is Err {
            println("err ${o}")
        }
    }
}

fn main() [use] -> None {
    use StdOutConsole
    println(classify(-5))
    println(classify(0))
    println(classify(7))
    println("sign ${sign(-3)} ${sign(0)} ${sign(9)}")
    describe("hi")
    describe(42)
    println(label(5))
    println(label(50))
    println(label(500))
    nested(ok(3))
    nested(err("bad"))
}
"#;

const WHEN_COND_STDOUT: &str = "negative\nzero\npositive\nsign -1 0 1\nstr hi\nint 42\n\
                                small\n  medium branch\nmedium\nlarge\n\
                                positive ok 3\nerr bad\n";

/// [rs-when-cond] Rust has no subject-less `match`, so the chain lowers to
/// `if`/`else if`/`else`. The mandatory `else` makes it total, so no
/// `unreachable!()` filler and — in value position — no `else { None }`
/// [if-else-none].
#[test]
fn a_subjectless_when_emits_an_if_chain() {
    let files = generate(&[("main.sv", WHEN_COND_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    let classify = main
        .content
        .split("pub fn classify")
        .nth(1)
        .and_then(|s| s.split("\npub fn ").next())
        .expect("classify emitted");
    assert!(
        classify.contains("if n < 0 {") && classify.contains("} else if n == 0 {"),
        "expected an if/else-if chain:\n{classify}"
    );
    assert!(
        classify.contains("} else {") && !classify.contains("None"),
        "expected a plain `else` and no optional filler:\n{classify}"
    );
    assert!(
        !main.content.contains("unreachable!"),
        "a total chain needs no filler arm:\n{}",
        main.content
    );
    // An `is` head still declares its binding at the top of the branch.
    assert!(
        main.content.contains("let mut s = value.u1().clone();"),
        "expected the `is` binding inside the branch:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_when_cond() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", WHEN_COND_DEMO)]);
    run_rust_files(&files, "when-cond", WHEN_COND_STDOUT);
}

// ===== a `try` body is ordinary code to the name/mutability censuses =====

// [try] The Rust counterpart of the Kotlin `var` miss: `collect_mutated_expr`
// and `collect_declared_expr` had no `Expr::Try` arm, so a variable assigned
// only inside a `try` body — and a local *declared* only inside one, which
// the generated-name census must know about — were invisible. Rust emits
// `let mut` unconditionally, so the mutability half is latent here rather
// than fatal; the declaration half is what would collide.
const TRY_MUTATION_DEMO: &str = r#"
fn risky(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

fn main() [use] -> None {
    use StdOutConsole
    let counter = 0
    let outcome = try {
        counter = counter + 1
        let step = risky(3)
        step
    }
    println("counter ${counter}")
}
"#;

#[test]
fn rustc_compiles_and_runs_try_mutation() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TRY_MUTATION_DEMO)]);
    run_rust_files(&files, "try-mutation", "counter 1\n");
}

// ===== platform effects [platform-effect] =====

/// The same source the Kotlin backend runs, so the asserted stdout is the
/// parity claim.
const PLATFORM_DEMO: &str = r#"
platform effect Telemetry {
    fn record(name: Str, value: Int) [] -> None => name, value
}

fn work(n: Int) [Telemetry] -> Int {
    record("work", n)
    return n + 1
}

fn main() [use, Telemetry] {
    use StdOutConsole()
    println("result=${work(41)}")
}
"#;

fn platform_demo_program() -> salvo_core::Program {
    build_program(&[("main.sv", PLATFORM_DEMO)])
}

/// [platform-tree] The host skeleton `salvo platform generate` writes for
/// the demo, as the compiler renders it.
fn platform_skeleton() -> salvo_backend_rust::EmittedFile {
    let program = platform_demo_program();
    let mut files = salvo_backend_rust::platform_skeletons(&program, None)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares platform effects");
    files.remove(0)
}

/// Emits the demo with `host` mounted as its platform companion — which is
/// what a source tree with a `platform/` directory produces
/// [platform-tree]. The host is *required*, so every emission test goes
/// through here.
fn generate_platform_demo_with(host: &str) -> Vec<salvo_backend_rust::EmittedFile> {
    let mut program = platform_demo_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.rs"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

fn generate_platform_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate_platform_demo_with(&platform_skeleton().content)
}

/// [platform-effect] [rs-platform-entry] A platform effect emits the same
/// `trait` an ordinary effect does and threads as `&mut dyn` exactly the
/// same way — but no handler struct, and `main` becomes `salvo_main` taking
/// the instance, because Rust requires `fn main` in the crate root and the
/// host's is the one that belongs there.
#[test]
fn platform_effect_emits_a_trait_and_a_host_entry() {
    let files = generate_platform_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs should be generated");
    let src = &main.content;
    assert!(
        src.contains("pub trait Telemetry {")
            && src.contains("fn record(&mut self, name: &String, value: i32);"),
        "expected the generated trait, got:\n{src}"
    );
    assert!(
        !src.contains("struct Telemetry"),
        "a platform effect must not emit a handler struct:\n{src}"
    );
    assert!(
        src.contains("pub fn salvo_main(telemetry: &mut dyn Telemetry)"),
        "expected the renamed entry point, got:\n{src}"
    );
    assert!(
        src.contains("pub fn work(telemetry: &mut dyn Telemetry, n: i32) -> i32"),
        "expected the effect threaded into `work`, got:\n{src}"
    );
    // [platform-tree] [rs-platform-host] Rust wants `fn main` in the crate
    // root, so the root mounts the host and delegates to it.
    assert!(
        src.contains("#[path = \"platform/main.rs\"]\npub mod platform_main;"),
        "expected the host mount, got:\n{src}"
    );
    assert!(
        src.contains("fn main() {\n    crate::platform_main::main()\n}"),
        "expected the crate-root delegation, got:\n{src}"
    );
}

/// [platform-tree] [rs-platform-host] The generated skeleton: a unit struct
/// per platform effect implementing the generated trait with every member
/// stubbed, plus the `main` the crate root delegates to. Paths are written
/// out in full, because the host is a mounted module and a qualified path is
/// the spelling that survives wherever the mounting puts it.
#[test]
fn platform_generate_renders_a_host_skeleton() {
    let file = platform_skeleton();
    assert_eq!(
        file.rel_path.to_string_lossy(),
        "platform/main.rs",
        "the host file mirrors its module under `platform/`"
    );
    let src = &file.content;
    for expected in [
        "pub struct TelemetryHost;",
        "impl crate::Telemetry for TelemetryHost {",
        "fn record(&mut self, name: &String, value: i32) {",
        "todo!(\"implement Telemetry.record\")",
        "pub fn main() {\n    crate::salvo_main(&mut TelemetryHost)\n}",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

/// [platform-tree] The host file is not optional: without it the crate has
/// no `main`, and the error has to name the command that writes one —
/// otherwise the only symptom is rustc's `E0601` against generated code.
#[test]
fn a_missing_host_file_names_the_command() {
    let program = platform_demo_program();
    let errors = salvo_backend_rust::emit_program(&program)
        .err()
        .expect("a platform program without a host must not emit");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("platform/main.rs") && e.contains("salvo platform generate")),
        "expected the missing-host error, got:\n{}",
        errors.join("\n")
    );
}

/// [platform-effect] [platform-tree] End to end under rustc with the
/// *generated* skeleton, one stub body filled in — so the test proves the
/// skeleton is complete and correct everywhere else, including the trait
/// path, the member signature and the entry-point call. This is the stdout
/// the Kotlin backend also asserts.
#[test]
fn rustc_compiles_and_runs_a_platform_effect() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let skeleton = platform_skeleton();
    let host = skeleton.content.replace(
        "todo!(\"implement Telemetry.record\")",
        "println!(\"[telemetry] {}={}\", name, value);",
    );
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
    let files = generate_platform_demo_with(&host);
    let expected = "[telemetry] work=41\nresult=42\n";
    run_rust_files(&files, "platform", expected);
}

// ===== platform handlers [platform-handler] =====

/// [platform-handler] The phase-4 shape in miniature (FILE_SYSTEM.md §5.8):
/// a host handler of an ordinary effect at the bottom, an ordinary Salvo
/// handler depending on it above, and application code that names neither.
/// The dependency switches the program into the fused emission, which is the
/// path phase 4 will take. Verbatim the Kotlin backend's demo.
const PLATFORM_HANDLER_DEMO: &str = r#"
effect RawClock {
    fn raw_now() [] -> Int
}

// The host implements this one, in Kotlin or Rust: bodyless here, and
// constructed at its `use` like any handler.
platform handler HostRawClock(offset: Int) of RawClock

effect Clock {
    fn stamp(label: Str) -> Str => label
}

handler DefaultClock [RawClock] of Clock {
    fn stamp(label: Str) -> Str => label {
        return "${label}@${raw_now()}"
    }
}

fn main() [use] {
    use StdOutConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
"#;

fn platform_handler_program() -> salvo_core::Program {
    build_program(&[("main.sv", PLATFORM_HANDLER_DEMO)])
}

fn platform_handler_skeleton() -> salvo_backend_rust::EmittedFile {
    let program = platform_handler_program();
    let mut files = salvo_backend_rust::platform_skeletons(&program, None)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares a platform handler");
    files.remove(0)
}

fn generate_platform_handler_demo_with(host: &str) -> Vec<salvo_backend_rust::EmittedFile> {
    let mut program = platform_handler_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.rs"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// [platform-handler] [rs-platform-handler] What the compiler emits: the
/// effect's `trait` as for any effect, **no struct** — the struct is the
/// host's — and a `use` site constructing it through its mounted path.
/// `main` stays in generated code: unlike a `platform effect`, nothing
/// arrives from outside.
#[test]
fn a_platform_handler_emits_no_struct_and_a_host_constructor() {
    let files = generate_platform_handler_demo_with(&platform_handler_skeleton().content);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs should be generated");
    let src = &main.content;
    assert!(
        src.contains("pub trait RawClock {") && src.contains("fn raw_now(&mut self) -> i32"),
        "expected the generated trait, got:\n{src}"
    );
    assert!(
        !src.contains("pub struct HostRawClock"),
        "a platform handler must not emit a struct of its own:\n{src}"
    );
    assert!(
        src.contains("crate::platform_main::HostRawClock::new(35)"),
        "expected the `use` site to construct the host struct, got:\n{src}"
    );
    // The host companion is mounted, and `main` is still generated.
    assert!(
        src.contains("#[path = \"platform/main.rs\"]\npub mod platform_main;"),
        "expected the host mount, got:\n{src}"
    );
    assert!(
        src.contains("fn main()") && src.contains("pub struct DefaultClock"),
        "expected the generated `main` and the Salvo handler, got:\n{src}"
    );
}

/// [platform-handler] [platform-tree] The skeleton: a struct named after the
/// *handler* (the `use` site constructs that name through `::new`), holding
/// the handler's constructor parameters, implementing the ordinary effect's
/// generated trait with every member stubbed. No `main`: a platform handler
/// does not move the entry point.
#[test]
fn platform_generate_renders_a_host_handler_skeleton() {
    let file = platform_handler_skeleton();
    assert_eq!(file.rel_path.to_string_lossy(), "platform/main.rs");
    let src = &file.content;
    for expected in [
        "pub struct HostRawClock {",
        "offset: i32,",
        "pub fn new(offset: i32) -> Self {",
        "impl crate::RawClock for HostRawClock {",
        "fn raw_now(&mut self) -> i32 {",
        "todo!(\"implement RawClock.raw_now\")",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("pub fn main()"),
        "a platform handler does not move the entry point:\n{src}"
    );
}

/// [platform-handler] [platform-tree] [backend-never-wrong] A `use` of a
/// platform handler with no host file is an error naming the command, not
/// generated code referencing a struct nobody wrote.
#[test]
fn a_missing_host_file_for_a_platform_handler_names_the_command() {
    let program = platform_handler_program();
    let errors = salvo_backend_rust::emit_program(&program)
        .err()
        .expect("a `use` of a platform handler without a host must not emit");
    assert!(
        errors.iter().any(|e| e.contains("use HostRawClock")
            && e.contains("platform/main.rs")
            && e.contains("salvo platform generate")),
        "expected the missing-host error, got:\n{}",
        errors.join("\n")
    );
}

/// [platform-handler] End to end under rustc with the *generated* skeleton,
/// one stub filled in. The asserted stdout is byte-identical to the Kotlin
/// backend's run of the same program.
#[test]
fn rustc_compiles_and_runs_a_platform_handler() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let skeleton = platform_handler_skeleton();
    let host = skeleton
        .content
        .replace("todo!(\"implement RawClock.raw_now\")", "self.offset + 7");
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
    let files = generate_platform_handler_demo_with(&host);
    run_rust_files(&files, "platform-handler", "boot@42\n");
}

// ===== [rs-fn-param-convention] generic fn-typed parameters =====

/// A generic higher-order fn, in every argument form: an un-annotated
/// lambda, an *annotated* one, and a named fn — over a `Copy` element type
/// (`Int`) and a non-`Copy` one (`Str`).
const GENERIC_HOF: &str = r#"
fn apply<T, U>(value: T, f: (T) -> U) -> U {
    return f(value)
}

fn shout(word: Str) -> Str {
    return "${word}!"
}

fn main() [use] {
    use StdOutConsole()
    let ns = list_of(1, 2)
    for v in map(iter(copy(ns)), n -> n * 2) { println("bare=${v}") }
    for v in map(iter(ns), (n: Int) -> n * 3) { println("ann=${v}") }
    let ws = list_of("hi")
    for w in map(iter(copy(ws)), (s: Str) -> shout(s)) { println("str=${w}") }
    for w in map(iter(copy(ws)), shout) { println("named=${w}") }
    for n in map(iter(ws), s -> size(s)) { println("size=${n}") }
    println("applied=${apply(2, (n: Int) -> n + 1)}")
}
"#;

const GENERIC_HOF_OUTPUT: &str =
    "bare=2\nbare=4\nann=3\nann=6\nstr=hi!\nnamed=hi!\nsize=2\napplied=3\n";

/// [rs-fn-param-convention] [fn-contract] The declaration of a generic
/// fn-typed parameter borrows — a type variable is never known to be
/// `Copy` — so a lambda passed into that position has to bind its
/// parameters the same way. Deciding from the lambda's *own* annotation
/// instead disagreed exactly where the two types differ: `(n: Int) -> …`
/// rendered `|n: i32|` against `FnMut(&T)` and rustc rejected the call
/// with `E0631`, while the un-annotated form compiled — so the bug was
/// invisible until a lambda was annotated.
///
/// The other convention — a callback the callee *keeps*, which arrives owned
/// and `'static` because it is called after the call returns — is asserted
/// where it belongs, on the hand-written composed pass
/// (`a_composed_pass_stores_its_source_and_callback` [rs-fn-field]). std has no
/// such function since the lazy pair was removed 2026-09-10.
#[test]
fn a_lambda_binds_a_generic_fn_parameter_by_reference() {
    let files = generate(&[("main.sv", GENERIC_HOF)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The plain generic higher-order fn: borrowed `FnMut`.
        "f: &mut impl FnMut(&T) -> U",
        // The lambda's *inner* convention follows the declaration.
        "|n: &i32|",
        // [yield-proj] The element is a borrow, so the retagged
        // parameter is one reference deeper than the annotation.
        "|s: &&String|",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

/// The same program under rustc: the conventions have to agree for every
/// argument form and both `Copy`-ness cases, and the output is what the
/// Kotlin backend asserts for the same source.
#[test]
fn rustc_compiles_and_runs_a_generic_higher_order_fn() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", GENERIC_HOF)]);
    run_rust_files(&files, "generic-hof", GENERIC_HOF_OUTPUT);
}

// ===== [iter-protocol] laziness, now a property of the pass =====

/// [seq-into] The combinator surface: **eager**, over a pass, with `map_to`
/// for mapping into a collection the caller provides. The lazy pair went with
/// the 2026-09-10 removal, so the generic bodies are reached by passing a pass
/// explicitly — which is what `map(iter(copy(xs)), …)` is for here.
///
/// Same source and stdout as the Kotlin backend's twin. What it exercises that
/// nothing did before: the *generic* `?Yield` body — `map`/`filter` over a
/// list take their `List` fast path, so the polymorphic one had never run —
/// and an implicit with a `Mut` parameter (`?add`).
pub const SEQ_SURFACE_DEMO: &str = r#"
fn double(n: Int) -> Int {
    return n * 2
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdOutConsole()
    let xs = list_of(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    let generic = map(iter(copy(xs)), double)
    println("generic ${generic.size()}")
    let snapshot = copy(xs)
    let kept = filter(iter(snapshot), is_even)
    println("kept ${kept.size()}")
    let out = map_to(mut_list_of<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(mut_list_of<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
"#;

pub const SEQ_SURFACE_OUTPUT: &str = "eager 4\ngeneric 4\nkept 2\nsink 4\nchained 6\n";

#[test]
fn rustc_compiles_and_runs_the_combinator_surface() {
    let files = generate(&[("main.sv", SEQ_SURFACE_DEMO)]);
    let seq = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/seq.rs"))
        .expect("core/seq.rs")
        .content;
    // The generic body drives its subject through the `next` the call site
    // resolved, which arrives as an ordinary borrowed callback [implicit-group].
    assert!(
        seq.contains("next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>"),
        "expected the resolved `next` as a borrowed callback in:\n{seq}"
    );
    // [fn-contract] A kept `Mut` position of an implicit's fn type borrows
    // mutably: `add` cannot append to a destination handed over by value.
    assert!(
        seq.contains("add: &mut dyn FnMut(&mut D, U)"),
        "expected the `?add` implicit to take its destination by `&mut` in:\n{seq}"
    );
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // [rs-implicit-turbofish] The instantiation is spelled out, or the adapter
    // closures have nothing to infer their parameter types from.
    assert!(
        main.contains("map_to::<"),
        "expected the type arguments spelled out in:\n{main}"
    );
    run_rust_files(&files, "seq-surface", SEQ_SURFACE_OUTPUT);
}

/// An **unbounded** producer that terminates only because the consumer stops.
/// This was a lazy-chain test until 2026-09-10; with the lazy pair gone the
/// same property is what a plain `for` with a `break` states, and it is the
/// property that mattered — an `iter fn` computes one element per turn, so an
/// endless source costs nothing until it is driven.
#[test]
fn rustc_compiles_and_runs_a_break_out_of_an_unbounded_producer() {
    let files = generate(&[("main.sv", UNBOUNDED_DEMO)]);
    run_rust_files(&files, "unbounded-break", UNBOUNDED_OUTPUT);
}

pub const UNBOUNDED_DEMO: &str = r#"
struct Naturals {
    from: Int
}

fn naturals() -> Naturals {
    return Naturals {from: 0}
}

iter fn next(n: Naturals) -> Emitted Int | Finished {
    state {
        at: Int = n.from
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn triple(n: Int) -> Int {
    return n * 3
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdOutConsole()
    for v in iter(naturals()) {
        if v > 4 {
            break
        }
        if is_even(v) {
            println("v ${triple(v)}")
        }
    }
}
"#;

pub const UNBOUNDED_OUTPUT: &str = "v 0\nv 6\nv 12\n";

// ===== [implicit-param] [implicit-group] implicit parameters =====

/// Every path through the feature in one program: a `params` group spread
/// with no binder, its members resolved from the visible `Int` overloads, a
/// call overriding *one* member by name, forwarding through a generic fn
/// whose `T` is opaque, and an individually declared `?add` reached by the
/// same name. The Kotlin backend asserts the same stdout for the same
/// source.
/// [implicit-group] [iter-protocol] The **composition rendering** R5's
/// combinators are built on (user decision 2026-09-09): a generic combinator
/// takes the *pass* as its subject and reaches its `next` through the
/// `?Yield<It, T>` spread. Two things had never been exercised together
/// before — an implicit member that takes a `Mut` parameter and returns a
/// **union** — and each was broken on one backend: the group member's
/// deduction list was dropped when its fn type was built (so no mutating
/// `next` could fill the position), and the adapter closure re-borrowed a
/// parameter the position had already handed over as `&mut` (E0596).

/// [iter-fn] The **release** of a minted pass: the mint is the
/// compiler's value, so the compiler closes it after the call — even when the
/// combinator abandons it after one element, which is the case a hand-written
/// driving loop cannot cover. `__close` is idempotent, so a drained pass pays
/// nothing. Without this the `defer` in the origin's body never ran.

/// [linear-group] [iter-protocol] A **raw pass that owns something**: it
/// declares `: Linear<self>` beside `: Yield<self, T>`, so [group-obligation]
/// requires the `close` — and *driving* is what releases it (user decision
/// 2026-09-09). The loop calls `close` on every exit: exhaustion, `break` and
/// `return` alike. Before this the linear obligation counted as discharged by
/// the move into the loop — bookkeeping — while the resource leaked.
pub const RAW_CLOSE_DEMO: &str = r#"
linear struct Ticks : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(l: Mut Ticks) -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    discard(l)
    println("closed")
}

fn drained() [Console] -> None {
    let lines = Mut Ticks { at: 2 }
    for n in lines {
        println("n ${n}")
    }
    close(lines)
    println("after drain")
}

fn abandoned() [Console] -> None {
    let lines = Mut Ticks { at: 5 }
    for n in lines {
        println("m ${n}")
        break
    }
    close(lines)
    println("after break")
}

fn main() [use] {
    use StdOutConsole()
    drained()
    abandoned()
}
"#;

pub const RAW_CLOSE_OUTPUT: &str = "n 2\nn 1\nclosed\nafter drain\nm 5\nclosed\nafter break\n";

#[test]
fn a_raw_pass_is_driven_in_place_and_closed_explicitly() {
    // [iter-drive-in-place] [linear-group] No implicit discharge sites
    // (user decision 2026-09-12): the named pass is driven where it lives —
    // no loop-local, no finally — and the program's own `close(lines)` is
    // the discharge.
    let files = generate(&[("main.sv", RAW_CLOSE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        !main.content.contains("__loop1_pass"),
        "a named pass must not be bound into a loop local:\n{}",
        main.content
    );
    assert!(
        // Suffixed: std declares a `close` too [fs-surface].
        main.content.contains("close__3(console, lines);"),
        "expected the program's own explicit close:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_released_raw_pass() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", RAW_CLOSE_DEMO)]);
    run_rust_files(&files, "raw_close", RAW_CLOSE_OUTPUT);
}

/// Two mints in one call, and two mints of the **same origin value**: each is a

/// [iter-generic-drive] [iter-drive-in-place] The three things the generic drive
/// buys, in one program: a combinator driving its pass with an ordinary `for`,
/// the caller **carrying that pass on** afterwards (the parity case — Rust used
/// to bind the `&mut` parameter into a local, so the caller never saw the
/// position the loop reached), and a `yield fn` performing a **generic** effect.
///
/// Shared with the other backend, byte for byte.
pub const GENERIC_DRIVE_DEMO: &str = r#"
struct Slice<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj(p) T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

// The pass is *kept*, so the loop advances it where it lives and the caller may
// drive it further.
fn take<It>(it: Mut It, count: Int, ?Yield<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    let seen = 0
    for n in it {
        sum = sum + n
        seen = seen + 1
        if seen == count {
            break
        }
    }
    return sum
}

effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % size(values)
        return values[i]
    }
}

struct Rolls {
    count: Int
}

fn rolls(count: Int) -> Rolls {
    return Rolls {count: count}
}

iter fn next(r: Rolls) [Random<Int>] -> Emitted Int | Finished {
    state {
        made: Int = 0
    }
    if made >= r.count {
        return finished()
    }
    made = made + 1
    return emitted(next_random())
}

fn main() [use] {
    use StdOutConsole()
    use CyclicRandom(array_of(7, 8, 9))
    let xs = list_of(1, 2, 3, 4)
    let p = slice(xs)
    println("first ${take(p, 2)}")
    println("rest ${take(p, 9)}")
    for v in rolls(3) {
        println("v ${v}")
    }
}
"#;

pub const GENERIC_DRIVE_OUTPUT: &str = "first 3\nrest 7\nv 8\nv 9\nv 7\n";

/// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
/// linear pass releases it through the `?Linear<It>` spread. The release cannot
/// print — an implicitly resolved fn is effect-free [implicit-resolve] — so the
/// call is asserted in the emitted code and the program is run to prove it
/// builds.
pub const GENERIC_CLOSE_DEMO: &str = r#"
linear struct Handle : Yield<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) -> None => !h { discard(h) }

// Owns the pass and takes its discharge as a consuming callback (user
// decision 2026-09-12: no implicit discharge sites — the loop drives in
// place, and `end(it)` is the explicit terminal on the one path out).
fn drain<It canbe linear>(it: Mut It, stop: Int, end: (x: It) -> None, ?Yield<It, Int>) [] -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
        if sum > stop {
            break
        }
    }
    end(it)
    return sum
}

fn main() [use] {
    use StdOutConsole()
    println("all ${drain(open_handle(4), 100, close)}")
    println("cut ${drain(open_handle(4), 5, close)}")
}
"#;

pub const GENERIC_CLOSE_OUTPUT: &str = "all 10\ncut 7\n";

pub const YIELD_SPREAD_DEMO: &str = r#"
struct Slice<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj(p) T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Yield<It, T>) -> Mut List<U> => it: Mut, mapper {
    let out = mut_list_of<U>()
    let going = true
    while going {
        let step = next(it)
        when step {
            is Emitted {
                add(out, mapper(step))
            }
            is Finished {
                going = false
            }
        }
    }
    return out
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdOutConsole()
    let xs = list_of(1, 2, 3)
    let doubled = map2(slice(xs), double)
    for d in doubled {
        println("d ${d}")
    }
}
"#;

pub const YIELD_SPREAD_OUTPUT: &str = "d 2\nd 4\nd 6\n";

/// The adapter for a `Mut` implicit position passes the parameter straight
/// through: it arrives as `&mut T` already.
#[test]
fn a_mut_implicit_position_is_not_borrowed_twice() {
    let files = generate(&[("main.sv", YIELD_SPREAD_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("&mut |__i0| next"),
        "expected the adapter to pass the `&mut` parameter through, got:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>"),
        "expected the spread position to be a `&mut`-taking, union-returning fn, got:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_combinator_over_a_yield_spread() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", YIELD_SPREAD_DEMO)]);
    run_rust_files(&files, "yield_spread", YIELD_SPREAD_OUTPUT);
}

pub const IMPLICIT_DEMO: &str = r#"
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return a + b }
fn zero() -> Int { return 0 }
fn times(a: Int, b: Int) -> Int { return a * b }
fn one() -> Int { return 1 }

fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

fn total_all<T>(rows: List<List<T>>, ?Field<T>) -> T => rows {
    let acc = zero()
    for row in rows {
        acc = add(acc, total(row))
    }
    return acc
}

fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T {
    return add(a, b)
}

fn main() [use] {
    use StdOutConsole()
    println("total=${total(list_of(1, 2, 3))}")
    println("product=${total(list_of(2, 3, 4), add = times, zero = one)}")
    // [col-of-nonempty] Literals rather than constructor calls: the element
    // constructors claim `NonEmpty`, and a `List<NonEmpty List<Int>>` does not
    // fit a `List<List<Int>>` position (type arguments are invariant). A literal
    // claims nothing, which is what a nested plain list needs.
    println("nested=${total_all([[1, 2], [3]])}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
"#;

pub const IMPLICIT_OUTPUT: &str = "total=6\nproduct=24\nnested=6\npair=42\nlambda=6\n";

/// [implicit-param] An implicit parameter lowers to an ordinary trailing
/// parameter of fn type, and the call site passes what resolution found —
/// so nothing about the feature survives into Rust. A group leaves no trace
/// at all: it was never a value [implicit-group].
/// [implicit-param] A call fills the **callee's** implicit parameters, read out
/// of a side table — so the answer must not depend on where the callee is
/// declared. It did until 2026-09-22: a callee written *below* its caller (or in
/// a file sorted after it) looked like a fn with no implicits, so the call was
/// accepted with none filled and the emitted call was short an argument, on both
/// backends. The checker could not see it — it accepted the call either way —
/// which is why the test lives here, where the symptom was.
#[test]
fn an_implicit_argument_does_not_depend_on_declaration_order() {
    let files = generate(&[(
        "main.sv",
        r#"
fn caller(a: Str, b: Str) -> Int => a, b {
    return bigger(a, b)
}

// Declared *after* its caller, and takes an implicit.
fn bigger<T>(a: T, b: T, ?Ordered<T>) -> Int => a, b {
    if cmp(a, b) > 0 {
        return 1
    }
    return 0
}

fn main() [use] {
    use StdOutConsole()
    let n = caller("bb", "a")
    println("${n}")
}
"#,
    )]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // The callee takes the position…
    assert!(
        src.contains("pub fn bigger<T: Clone>(a: &T, b: &T, cmp: &mut dyn FnMut(&T, &T) -> i32)"),
        "expected the implicit position in:\n{src}"
    );
    // …and the call from *above* it fills the position rather than omitting it.
    assert!(
        src.contains("bigger::<String>(a, b, &mut |__i0, __i1|"),
        "expected the call to pass its implicit argument in:\n{src}"
    );
}

#[test]
fn implicit_parameters_lower_to_trailing_fn_arguments() {
    let files = generate(&[("main.sv", IMPLICIT_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The group's members, expanded in declaration order.
        // `dyn`, uniformly: an effect member's implicits land in an
        // object-safe trait, and forwarding has to compose in every
        // direction, so one convention serves both.
        // [rs-fn-param-convention] A member's parameters are *kept* (no
        // deduction clause keeps everything [fn-contract]), so a non-Copy
        // position borrows — `&T` at a type variable, which is never known to
        // be Copy. By value it was a move, and a generic fn that compared two
        // values and then used one of them did not compile (2026-09-21).
        "pub fn total<T: Clone>(xs: &Vec<T>, add: &mut dyn FnMut(&T, &T) -> T, \
         zero: &mut dyn FnMut() -> T)",
        // A resolved default is passed as an adapter closure over the fn,
        // which bridges the position's convention to the callee's own: `add`
        // takes its `Int`s by value, so the adapter clones out of the
        // borrows (free for a scalar).
        "&mut |__i0, __i1| add((__i0).clone(), (__i1).clone())",
        // The same bridging for an override written at the call site
        // [implicit-override].
        "&mut |__i0, __i1| times__2((__i0).clone(), (__i1).clone())",
        // Forwarding reborrows the enclosing fn's own parameter.
        "&mut *add",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("struct Field"),
        "a `params` group must leave no runtime representation:\n{src}"
    );
}

/// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
#[test]
fn rustc_compiles_and_runs_implicit_parameters() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", IMPLICIT_DEMO)]);
    run_rust_files(&files, "implicits", IMPLICIT_OUTPUT);
}

/// [cmp-groups] Comparison, equality and hashing as **params groups** — the
/// ordering round's foundation (user decisions 2026-09-21). Every canonical
/// implementation for an intrinsic type is exercised directly, then through a
/// group spread at a Copy scalar *and* at a `Str`, which is the case that
/// makes the position's convention load-bearing: `min_of` compares two values
/// and then answers one of them, so a *kept* position must not move what it
/// was handed [rs-fn-param-convention].
///
/// [cmp-hash-values] No hash **value** appears in the expected output: each
/// backend hashes with its host's own algorithm, so the values differ by
/// design and only the agreement with `eq` is asserted — the posture `random`
/// already has.
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_the_comparison_groups`. That equality is the
/// assertion: `Str` ordering is by code point on both backends, which Kotlin
/// has to arrange deliberately [kt-ordered].
pub const CMP_DEMO: &str = r#"
// [cmp-groups] The three capabilities, asked for by name. Nothing about a `T`
// is knowable, so an ordering arrives as an implicit parameter — and the call
// site fills it with the canonical overload for the type it instantiates.
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn same<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return eq(a, b)
}

// [cmp-hash-values] A hash value is never printed: it differs between the
// backends by design. What holds on both is that equal values hash equal —
// which is what this answers.
fn digest_agrees<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn sign(n: Int) [] -> Str {
    if n < 0 { return "<" }
    if n > 0 { return ">" }
    return "="
}

fn main() [use] {
    use StdOutConsole()
    let ab = "ab"
    let b = "b"
    let same_ab = "ab"
    // The canonical `cmp` at each intrinsic type, called directly.
    println("int ${sign(cmp(1, 2))}${sign(cmp(2, 2))}${sign(cmp(3, 2))}")
    println("long ${sign(cmp(to_long(9), to_long(4)))}")
    println("byte ${sign(cmp(to_byte(200), to_byte(3)))}")
    println("char ${sign(cmp('a', 'b'))}")
    println("bool ${sign(cmp(false, true))}")
    // `Str` compares by **code point**, which is what the Kotlin backend has
    // to arrange deliberately: "ab" < "b" because 'a' < 'b'.
    println("str ${sign(cmp(ab, b))}${sign(cmp(b, b))}${sign(cmp(b, ab))}")
    // Equality covers the float widths, where no total order exists.
    println("eq ${eq(1, 1)} ${eq(1.5, 2.5)} ${eq(to_float(1.0), to_float(1.0))} ${eq(ab, b)}")
    println("same ${same(7, 7)} ${same(ab, b)} ${same(ab, same_ab)}")
    println("digest ${digest_agrees(ab, same_ab)} ${digest_agrees(3, 3)}")
    // `min_of` answers one of its arguments, so it **moves** them: the
    // deduction is inferred from the body [deduce-infer], and these two are
    // the last use of their values.
    println("min ${min_of(4, 2)} ${min_of(ab, b)}")
}
"#;

pub const CMP_OUTPUT: &str = "int <=>\nlong >\nbyte >\nchar <\nbool <\nstr <=>\n\
                              eq true false true false\nsame true false true\n\
                              digest true true\nmin 2 ab\n";

/// The lowerings themselves: each canonical is the host's own operation
/// [backend-intrinsic], and `Str` ordering goes through `str`'s byte-wise
/// `Ord` — which *is* code-point order.
#[test]
fn the_comparison_groups_lower_to_host_operations() {
    let files = generate(&[("main.sv", CMP_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // `Ordering` is a fieldless `#[repr(i8)]` enum whose discriminants are
        // the sign convention `cmp` answers, so the cast is the lowering.
        "(Ord::cmp(&(1), &(2)) as i32)",
        "(Ord::cmp(&ab[..], &b[..]) as i32)",
        "(&ab[..] == &b[..])",
        "std::hash::DefaultHasher::new()",
        // [rs-fn-param-convention] A kept position borrows: the generic body
        // compares `a` and `b` and still owns them afterwards.
        "pub fn min_of<T: Clone>(a: T, b: T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> T",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // A group is never a value, so nothing of the three survives as a type.
    assert!(
        !src.contains("struct Ordered") && !src.contains("struct Hashed"),
        "a `params` group must leave no runtime representation:\n{src}"
    );
}

/// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
#[test]
fn rustc_compiles_and_runs_the_comparison_groups() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", CMP_DEMO)]);
    run_rust_files(&files, "compare_groups", CMP_OUTPUT);
}

/// [cmp-canonical] The **canonical** implementation of a capability for a type,
/// `@`-scoped to it: declared in the type's file, imported with the type, the
/// default selection for an implicit of the same shape, and named by the same
/// selector at a call or as a value. Two modules, because "travels with the
/// type" is the property that needs a second one.
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_a_canonical_implementation`.
pub const CANONICAL_PEOPLE: &str = r#"
export struct Person {
    name: Str,
    age: Int
}

// The canonical ordering for `Person`: an ordinary overload that travels with
// the type, exported to match it.
export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

"#;

pub const CANONICAL_MAIN: &str = r#"
// Only the *type* is imported; its canonical rides along [cmp-canonical].
import people.Person

// An ordinary overload of the same shape, in *this* module — the `Own` rung,
// which today's ladder would let win silently. It cannot: an ambiguity around a
// canonical is always an error [cmp-canonical], so every call below selects.
fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn main() [use] {
    use StdOutConsole()
    let ada = Person { name: "ada", age: 36 }
    let bob = Person { name: "bob", age: 5 }
    // Both spellings of the selector: at a call, and as a value filling an
    // implicit parameter [implicit-override].
    println("by age ${cmp@Person(ada, bob)}")
    println("by name ${cmp@main(ada, bob)}")
    let younger = min_of(ada, bob, cmp = cmp@Person)
    println("younger ${younger.name}")
}
"#;

pub const CANONICAL_OUTPUT: &str = "by age 1\nby name -1\nyounger bob\n";

#[test]
fn rustc_compiles_and_runs_a_canonical_implementation() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[
        ("people.sv", CANONICAL_PEOPLE),
        ("main.sv", CANONICAL_MAIN),
    ]);
    run_rust_files(&files, "canonical_impl", CANONICAL_OUTPUT);
}

/// [cmp-auto] `: auto Ordered<self>` and friends: the compiler writes the
/// structural implementations, `@`-scoped to the type, and lowers them to the
/// derive the `default` clause asks for — so the generated
/// member and the type's own `Ord`/`Hash` cannot disagree (user decision
/// 2026-09-21, lowering split by author).
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_auto_members`.
pub const AUTO_DEMO: &str = r#"
// Ordering is lexicographic by field declaration order, which is the language's
// rule on both backends.
struct Point : auto Ordered<self>, auto Hashed<self> {
    x: Int,
    y: Int
}

// [cmp-auto] The **function-level** form, with no obligation clause at all:
// `auto` is a modifier on the declaration, so this is what the clause above is
// sugar for — and it is what a backend's derive hangs on.
struct Tag {
    label: Str
}

auto fn eq@Tag(a: Tag, b: Tag) [] -> Bool => a, b

// A generic struct's generated members are generic too.
struct Box<T> : auto Eq<self> {
    item: T
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn sign(n: Int) [] -> Str {
    if n < 0 { return "<" }
    if n > 0 { return ">" }
    return "="
}

fn main() [use] {
    use StdOutConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 9 }
    let r = Point { x: 1, y: 2 }
    println("cmp ${sign(cmp(p, q))}${sign(cmp(p, r))}${sign(cmp(q, p))}")
    println("eq ${eq(p, q)} ${eq(p, r)}")
    // [cmp-hash-values] The agreement with `eq`, never the value.
    println("hash agrees ${eq(hash(p), hash(r))}")
    let tags = eq(Tag { label: "a" }, Tag { label: "a" })
    let boxes = eq(Box<Int> { item: 1 }, Box<Int> { item: 2 })
    println("others ${tags} ${boxes}")
    // The capability through an implicit, filled with the generated canonical.
    let smaller = min_of(p, q)
    println("smaller ${smaller.y}")
}
"#;

pub const AUTO_OUTPUT: &str =
    "cmp <=>\neq false true\nhash agrees true\nothers true false\nsmaller 2\n";

/// The lowering: the generated member stands on the derive, and the derive is
/// what the `default` clause asks for.
#[test]
fn default_obligations_lower_to_the_hosts_derives() {
    let files = generate(&[("main.sv", AUTO_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // `auto Ordered` + `auto Hashed` ask for exactly the derives
        // the `default` clauses ask for.
        "#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub struct Point",
        // The generated members are ordinary Rust fns over the derive.
        "(Ord::cmp(a, b) as i32)",
        "std::hash::DefaultHasher::new()",
        // A generic struct's member carries the bound its derive carries.
        "pub fn eq__",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

#[test]
fn rustc_compiles_and_runs_auto_members() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", AUTO_DEMO)]);
    run_rust_files(&files, "auto_members", AUTO_OUTPUT);
}

/// [op-order] [op-equality] The **operators** through the groups: `a < b` is
/// `cmp(a, b) < 0` and `a == b` is `eq(a, b)`, at a struct, at a `Str` (which
/// had no ordering at all before, and now orders by code point on both
/// backends) and at a generic `T` whose capability arrives as an implicit.
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_operators_through_the_groups`. The `Str` line is
/// the parity claim: the JVM's own `<` would compare UTF-16 code units.
pub const GROUP_OPERATOR_DEMO: &str = r#"
struct Point : auto Ordered<self>, auto Eq<self> {
    x: Int,
    y: Int
}

// A hand-written canonical, which the operators reach exactly as they reach a
// generated one [cmp-canonical].
struct Age {
    years: Int,
    label: Str
}

fn cmp@Age(a: Age, b: Age) [] -> Int => a, b {
    return cmp(a.years, b.years)
}

fn eq@Age(a: Age, b: Age) [] -> Bool => a, b {
    return eq(a.years, b.years)
}

// [implicit-forward] Generic code publishes the capability in its signature,
// and the operators inside go through the parameter.
fn larger_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if a > b {
        return a
    }
    return b
}

fn same_of<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return a == b
}

fn main() [use] {
    use StdOutConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 9 }
    println("struct ${p < q} ${p == q} ${p != q} ${q >= p}")
    let young = Age { years: 5, label: "bob" }
    let old = Age { years: 36, label: "ada" }
    println("by hand ${young < old} ${young == old}")
    // `Str` ordering: "ab" < "b" by code point, on both backends.
    let ab = "ab"
    let b = "b"
    println("strs ${ab < b} ${ab == b} ${same_of(ab, b)}")
    println("generic ${larger_of(3, 9)} ${larger_of(p, q).y} ${larger_of(young, old).label}")
}
"#;

pub const GROUP_OPERATOR_OUTPUT: &str = "struct true false true true\nby hand true false\n\
                                   strs true false false\ngeneric 9 9 ada\n";

#[test]
fn rustc_compiles_and_runs_operators_through_the_groups() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", GROUP_OPERATOR_DEMO)]);
    run_rust_files(&files, "operators_through_groups", GROUP_OPERATOR_OUTPUT);
}

/// [cmp-carry] [cmp-binder] A structure that **holds** an ordering: the
/// identity is fixed at construction, travels in the type, and is what the
/// body compares with — so the *same* generic function answers differently for
/// two lists built with two orderings (user decisions 2026-09-21, the ordering round
/// step 5).
///
/// The point for a backend is that there is nothing to see: a qualifier erases
/// [qual-erasure], so the carried identity lowers as the **implicit parameter**
/// it is resolved as [implicit-param] — an ordinary trailing argument, the
/// machinery both emitters already had. What the program proves is that the
/// *checker* filled it from the type rather than from what is visible at the
/// call: `least_index` resolves `cmp` to `by_name` for the second list, where
/// the canonical `cmp@Person` is equally in scope.
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_a_carried_ordering`.
pub const CARRY_DEMO: &str = r#"
// The claim a ranked list carries: the ordering it is ranked by, named in the
// qualifier's own **fn slot**.
qualifier Ranked<T>(?cmp: (T, T) -> Int) of List<T>

struct Person { name: Str, age: Int }

// [cmp-canonical] The canonical ordering for a `Person`: by age.
fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

// A second ordering a ranked list may be built with instead.
fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

// [cmp-binder] Here the binder is an implicit parameter the fn declares, so
// resolution fills it and the result type publishes what it chose.
fn empty_ranked<T>(?cmp: (T, T) -> Int) -> +Ranked<T>(?cmp) Mut List<T> {
    return mut_list_of()
}

// [cmp-binder] And here it is captured from the argument's type: nothing is
// written but `?cmp`, and the slot it fills states its type. The claim is
// re-minted on the way out, which is what a constructor fn may do today —
// keeping it across a `Mut` parameter is ROADMAP's D2.
fn rank_add<T>(r: Ranked<T>(?cmp) Mut List<T>, elem: T) -> +Ranked<T>(?cmp) Mut List<T> => !r, !elem {
    add(r, elem)
    return r
}

// One body, two answers: it compares with whatever ordering its argument was
// built with.
fn least_index<T>(r: Ranked<T>(?cmp) List<T>) -> Int => r {
    let best = 0
    let i = 1
    while i < size(r) {
        if cmp(get(r, i)!, get(r, best)!) < 0 {
            best = copy(i)
        }
        i = i + 1
    }
    return best
}

// A body that never needs the identity takes the claim bare.
fn ranked_size<T>(r: Ranked List<T>) -> Int => r {
    return size(r)
}

fn main() [use] {
    use StdOutConsole()

    // Built with the canonical ordering: the youngest ranks first.
    let by_age = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(), Person {name: "Ada", age: 36}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Cyd", age: 31}
    )
    let youngest = get(by_age, least_index(by_age))!
    println("by age: ${youngest.name} of ${ranked_size(by_age)}")

    // Built with another ordering: the same code, a different answer.
    let alpha = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(cmp = by_name), Person {name: "Cyd", age: 31}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Ada", age: 36}
    )
    let first = get(alpha, least_index(alpha))!
    println("by name: ${first.name} of ${ranked_size(alpha)}")
}
"#;

pub const CARRY_OUTPUT: &str = "by age: Bob of 3\nby name: Ada of 3\n";

/// [cmp-carry] [col-membership] A **keyed container** kept by the ordering its
/// type names: two sorted sets of the same people, one canonical and one by age,
/// and one generic function over either — which is emitted with no extra
/// parameter, because the marker stops at the container's edge [rs-collections].
///
/// Membership is `cmp`-distinct, so a second person of an age already present is
/// not a new member under the ordering by age, and is one under the canonical
/// ordering. Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_a_keyed_container_ordering`, where the same ordering
/// becomes a `TreeSet` comparator rather than a marker — two lowerings of one
/// rule, and the equality of the output is the assertion.
pub const KEYED_DEMO: &str = r#"
struct Person { name: Str, age: Int }

auto fn cmp@Person(a: Person, b: Person) -> Int
auto fn eq@Person(a: Person, b: Person) -> Bool

fn by_age(a: Person, b: Person) -> Int => a, b {
    return cmp(a.age, b.age)
}

// One body, either ordering: the identity is the container's, not the caller's.
fn lowest<T>(s: SortedSet<T>) -> T? => s {
    return min(s)
}

fn main() [use] {
    use StdOutConsole()
    let byname = sorted_set_of(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24}
    )
    let byage: SortedSet<Person>(by_age) = sorted_set_of(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24}
    )
    println("byname ${lowest(byname)!.name} of ${size(byname)}")
    println("byage ${lowest(byage)!.name} of ${size(byage)}")

    // `cmp`-distinct membership: a second 24-year-old is the same member under
    // `by_age`, and its own member under the canonical ordering.
    let twin = Person {name: "Eve", age: 24}
    let grown_age: Mut SortedSet<Person>(by_age) = mut_sorted_set_of()
    let grown_name: Mut SortedSet<Person> = mut_sorted_set_of()
    add(grown_age, Person {name: "Bob", age: 24})
    add(grown_name, Person {name: "Bob", age: 24})
    println("twin ${add(grown_age, copy(twin))} ${add(grown_name, twin)}")
}
"#;

pub const KEYED_OUTPUT: &str = "byname Ada of 3\nbyage Bob of 3\ntwin false true\n";

/// [rs-collections] A generic function over a **hash** container that actually
/// hashes. This did not compile before the keyed containers moved behind a store
/// (2026-09-22): the emitter writes `<T: Clone>` while the runtime's hashing
/// operations needed `T: Hash + Eq`, so `contains` inside generic code was a
/// rustc error in emitted output — a program the checker accepted and the backend
/// could not build. Every operation is bound-free now, which removed the
/// requirement rather than adding one.
#[test]
fn rustc_compiles_and_runs_a_generic_over_a_hash_container() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let src = r#"
fn has<T>(s: Set<T>, e: T) -> Bool => s, e {
    return contains(s, e)
}

fn tally<K, V>(m: Map<K, V>) -> Int => m {
    return size(m)
}

fn main() [use] {
    use StdOutConsole()
    let seen = set_of(1, 2, 3)
    let ages: Map<Str, Int> = {"ada": 36, "bob": 24}
    println("has ${has(seen, 2)} ${has(seen, 9)} of ${tally(ages)}")
    println("map ${to_str(ages)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    run_rust_files(
        &files,
        "generic_hash_container",
        "has true false of 2\nmap {ada: 36, bob: 24}\n",
    );
}

/// [cmp-carry] [col-membership] A **hash** container keyed by the `hash` and `eq`
/// its type names: membership is `eq`-distinct, so a second person of an age
/// already present is one member under a pair that keys by age and a new one under
/// the canonical pair. Source and expected stdout are **verbatim** the Kotlin
/// backend's `kotlinc_compiles_and_runs_a_keyed_hash_pair` — there the pair is
/// passed to a runtime container as two function references, here it is two
/// zero-sized markers.
pub const KEYED_HASH_DEMO: &str = r#"
struct Person { name: Str, age: Int }

auto fn hash@Person(value: Person) -> Long
auto fn eq@Person(a: Person, b: Person) -> Bool

// A pair that keys by age alone: two people of an age are one member.
fn age_hash(p: Person) -> Long => p {
    return to_long(p.age)
}

fn same_age(a: Person, b: Person) -> Bool => a, b {
    return a.age == b.age
}

// One body, either keying: the pair is the container's, not the caller's.
fn tally<T>(s: Set<T>) -> Int => s {
    return size(s)
}

fn main() [use] {
    use StdOutConsole()
    let all: Mut Set<Person> = mut_set_of()
    let byage: Mut Set<Person>(age_hash, same_age) = mut_set_of()
    let bob = Person {name: "Bob", age: 24}
    add(all, copy(bob))
    add(byage, bob)
    let eve = Person {name: "Eve", age: 24}
    let fresh = add(all, copy(eve))
    let twin = add(byage, eve)
    println("added ${fresh} ${twin}")
    println("sizes ${tally(all)} ${tally(byage)}")
}
"#;

pub const KEYED_HASH_OUTPUT: &str = "added true false\nsizes 2 1\n";

#[test]
fn rustc_compiles_and_runs_a_keyed_hash_pair() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", KEYED_HASH_DEMO)]);
    run_rust_files(&files, "keyed_hash", KEYED_HASH_OUTPUT);
}

#[test]
fn rustc_compiles_and_runs_a_keyed_container_ordering() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", KEYED_DEMO)]);
    run_rust_files(&files, "keyed_ordering", KEYED_OUTPUT);
}

#[test]
fn rustc_compiles_and_runs_a_carried_ordering() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", CARRY_DEMO)]);
    run_rust_files(&files, "carried_ordering", CARRY_OUTPUT);
}

/// [rs-fn-field] A **composed pass, hand-written**: it stores both its source
/// and its callback. Storing a function in a struct field used to be a Rust
/// codegen error while Kotlin accepted it (a live backend divergence, recorded
/// as R0 finding 5); the field is now an `Rc<dyn Fn…>`, the same
/// representation a generated pass has always used for a callback, so the
/// program builds on both targets — which is what makes user-written
/// combinators possible at all, not just std's generated ones.
pub const FN_FIELD_DEMO: &str = r#"
struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

struct Doubling : Yield<self, Int> canbe Mut {
    src: Mut Countdown,
    f: (Int) -> Int
}

fn next(d: Mut Doubling) -> Emitted Int | Finished => d: Mut {
    let step = next(d.src)
    when step {
        is Emitted {
            let g = d.f
            return emitted(g(step))
        }
        is Finished {
            return finished()
        }
    }
}

fn twice(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdOutConsole()
    let doubled = Mut Doubling { src: Mut Countdown { at: 3 }, f: twice }
    for n in doubled {
        println("n ${n}")
    }
    println("done")
}
"#;

pub const FN_FIELD_OUTPUT: &str = "n 6\nn 4\nn 2\ndone\n";

/// [rs-fn-field] The representation: an `Rc<dyn Fn…>` field, `#[derive(Clone)]`
/// without `Debug` (a `dyn Fn` has none), a hand-written `Debug` printing the
/// callback as `<fn>`, and `Rc::new` at the store.
#[test]
fn a_function_in_a_struct_field_is_an_rc_dyn_fn() {
    let files = generate(&[("main.sv", FN_FIELD_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("pub f: std::rc::Rc<dyn Fn(i32) -> i32>,"),
        "expected an Rc<dyn Fn> field, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains("impl std::fmt::Debug for Doubling"),
        "expected a hand-written Debug, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains(".field(\"f\", &\"<fn>\")"),
        "expected the callback to print as <fn>, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains("f: std::rc::Rc::new(twice)"),
        "expected the store to wrap in Rc::new, got:\n{}",
        main.content
    );
}

/// Under rustc, with the stdout the Kotlin backend asserts byte for byte.
#[test]
fn rustc_compiles_and_runs_a_composed_pass_with_a_stored_callback() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FN_FIELD_DEMO)]);
    run_rust_files(&files, "fn_field", FN_FIELD_OUTPUT);
}

/// [iter-protocol] A hand-written **pass**: `zip`, which `yield` cannot
/// express because it reads two sources at once. A struct, a `next`, and
/// nothing else — no state machine, no compiler support beyond driving it.
pub const PASS_DEMO: &str = r#"
struct Zip : Yield<self, (Str, Int)> canbe Mut {
    left: List<Str>,
    right: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> Emitted (Str, Int) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        return emitted((copy(l), r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> Zip => !left, !right {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Countdown {
    return Countdown { at: from }
}

fn main() [use] {
    use StdOutConsole()
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = list_of("ada", "grace", "alan")
    let ages = list_of(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}
"#;

pub const PASS_OUTPUT: &str = "n 3\nn 2\nn 1\nada is 36\ngrace is 45\ndone\n";

/// The driving loop: the subject is moved into a mutable local and each turn
/// calls the resolved `next`, matching the `Emitted` arm the *checker* chose
/// [union-arm-identity]. A `while let`, so the condition is re-evaluated per
/// turn and `Finished` needs no arm of its own.
#[test]
fn a_pass_lowers_to_a_while_let_driving_loop() {
    let files = generate(&[("main.sv", PASS_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("_pass = countdown(3);"),
        "expected the subject bound to a local in:\n{src}"
    );
    assert!(
        src.contains("while let Union2::U1(mut n) = next"),
        "expected a while-let driving loop in:\n{src}"
    );
    assert!(
        !src.contains("for mut n in countdown"),
        "the native for-loop fallback should be gone from:\n{src}"
    );
}

/// [iter-protocol] [throw] A **fallible** pass: the producer yields a result
/// and the *consumer* throws. This is the answer to "does the protocol need
/// `Throw` support" — it does not, so `Emitted T | Finished` stays exactly two
/// arms and a `yield` fn never has to declare an effect it cannot perform
/// while suspended. Also the regression test for the element being a union
/// under a qualifier [qual-group]: `emitted(err(...))` is a qualifier applied
/// to an already-qualified value, so the arm it lands in is a qualified union
/// *group* and the value needs two wraps — inner union first. That needed an
/// intermediate annotated `let` until 2026-09-10.
pub const FALLIBLE_PASS_DEMO: &str = r#"
struct Reader : Yield<self, Ok Str | Err Str> canbe Mut {
    lines: List<Str>,
    at: Int
}

fn next(r: Mut Reader) -> Emitted (Ok Str | Err Str) | Finished => r: Mut {
    let line = get(r.lines, r.at)
    if line is Str {
        r.at = r.at + 1
        if line == "boom" {
            return emitted(err("bad line at ${r.at}"))
        }
        return emitted(ok(copy(line)))
    }
    return finished()
}

fn reader(lines: List<Str>) -> Reader => !lines {
    return Reader { lines: lines, at: 0 }
}

fn read_all(lines: List<Str>) [Throw<Str>, Console] -> Int {
    let count = 0
    for outcome in reader(lines) {
        when outcome {
            is Ok {
                println("line ${outcome}")
                count = count + 1
            }
            is Err {
                return throw("stopped: ${outcome}")
            }
        }
    }
    return count
}

fn main() [use] {
    use StdOutConsole()
    let good = try { read_all(list_of("alpha", "beta")) }
    when good {
        is Ok { println("read ${good}") }
        is Thrown { println("failed: ${good}") }
    }
    let bad = try { read_all(list_of("alpha", "boom", "gamma")) }
    when bad {
        is Ok { println("read ${bad}") }
        is Thrown { println("failed: ${bad}") }
    }
}
"#;

pub const FALLIBLE_PASS_OUTPUT: &str =
    "line alpha\nline beta\nread 2\nline alpha\nfailed: stopped: bad line at 2\n";

#[test]
fn a_fallible_pass_yields_a_result() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    run_rust_files(&files, "fallible-pass", FALLIBLE_PASS_OUTPUT);
}

/// [qual-group] The nested wrap, read off the generated source rather than
/// only from the program's output: `emitted(err(...))` lands in the group arm
/// `Emitted (Ok Str | Err Str)`, whose inner union is a wrapper of its own, so
/// the value takes *two* wraps — inner arm first, and the arm index comes
/// from the qualifier left over after the group's own is removed.
#[test]
fn a_flattened_qualifier_wraps_the_inner_union_first() {
    let files = generate(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs");
    for (arm, what) in [("U2", "err"), ("U1", "ok")] {
        let needle = format!(
            "Union2::<Union2<String, String>, Finished>::U1(Union2::<String, String>::{arm}("
        );
        assert!(
            main.content.contains(&needle),
            "expected the inner {what} arm to be wrapped before the outer one in:\n{}",
            main.content
        );
    }
}

/// [qual-group] The same wrap where the remainder carries *no* qualifier at
/// all: `Emitted (Str | Int)` is a group over plain arms. Worth its own
/// program because the checker reaches it by a different route — the value
/// subtypes the arm outright, so the nested reading is never consulted — while
/// the *representation* is the same two wraps. Until 2026-09-10 this
/// type-checked and emitted one wrap, which rustc rejected (E0308):
/// [backend-never-wrong] holding, not the code being right.
const PLAIN_GROUP_DEMO: &str = r#"
fn step(n: Int) -> Emitted (Str | Int) | Finished {
    if n == 1 {
        return emitted("one")
    }
    if n == 2 {
        return emitted(2)
    }
    return finished()
}

fn show(n: Int) [Console] {
    let r = step(n)
    when r {
        is ^Emitted {
            when r {
                is Str { println("str ${r}") }
                is Int { println("int ${r}") }
            }
        }
        is Finished { println("finished") }
    }
}

fn main() [use] {
    use StdOutConsole()
    show(1)
    show(2)
    show(3)
}
"#;

const PLAIN_GROUP_OUTPUT: &str = "str one\nint 2\nfinished\n";

/// [rs-narrow-mut] Mutating through a **narrowed** place. Every shape the
/// unwrap can take is here, because the read form of each was silently wrong:
/// `&mut` of `p.as_ref().unwrap().clone()` compiles and mutates the clone, so
/// a pass driven through a narrowed handle re-emitted its first element for
/// ever while Kotlin — whose smart cast *is* the storage — advanced.
///
/// - a narrowed optional passed to a `Mut` parameter (`Option::as_mut`);
/// - a narrowed union arm, same thing through `u1_mut()`;
/// - an assignment whose *base* is narrowed (`r.at = 2`), which reached for a
///   field of the `Option` and was an E0609 rather than a silent wrong answer;
/// - a narrowed `state` slot of an `iter fn`, which is the shape that found it:
///   a lazy flatten holding the inner pass. That one also needed the
///   desugaring to stop giving the synthesized `__p` base the read's span.
const NARROW_MUT_DEMO: &str = r#"
struct Flat {
    rows: List<List<Int>>
}

fn show(step: Emitted Int | Finished) [Console] {
    when step {
        is ^Emitted { println("got ${step}") }
        is Finished { println("end") }
    }
}

iter fn next(f: Flat) -> Emitted Int | Finished {
    state { at: Int = 0, inner: Mut ListYield<Int>? = None }
    while true {
        if inner is Mut ListYield<Int> {
            let step = next(inner)
            when step {
                is ^Emitted { return emitted(step) }
                is Finished { inner = None }
            }
        }
        let row = get(f.rows, at)
        if row is None {
            return finished()
        }
        at = at + 1
        inner = iter(row)
    }
    return finished()
}

fn main() [use] {
    use StdOutConsole()
    let base = list_of(1, 2)
    let p: Mut ListYield<Int>? = iter(base)
    if p is Mut ListYield<Int> {
        show(next(p))
        show(next(p))
        show(next(p))
    }
    let qs = list_of(7, 8)
    let q: Mut ListYield<Int> | Int = iter(qs)
    if q is Mut ListYield<Int> {
        show(next(q))
        show(next(q))
    }
    let rs = list_of(1, 2, 3)
    let r: Mut ListYield<Int>? = iter(rs)
    if r is Mut ListYield<Int> {
        r.at = 2
        show(next(r))
    }
    // Literals, not constructor calls: the element constructors claim
    // `NonEmpty` [col-of-nonempty], which a plain `List<List<Int>>` field
    // refuses (type arguments are invariant).
    let all = Flat { rows: [[1, 2], [3, 4, 5]] }
    for n in iter(all) {
        println("n ${n}")
    }
}
"#;

const NARROW_MUT_OUTPUT: &str = "got 1\ngot 2\nend\ngot 7\ngot 8\ngot 3\nn 1\nn 2\nn 3\nn 4\nn 5\n";

#[test]
fn rustc_compiles_and_runs_mutation_through_narrowed_places() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", NARROW_MUT_DEMO)]);
    run_rust_files(&files, "narrow-mut", NARROW_MUT_OUTPUT);
}

/// [rs-narrow-mut] The same thing read off the generated source, so the
/// regression is caught with no toolchain on PATH: a mutable use borrows into
/// the storage, and the tell-tale `&mut` of a clone appears nowhere.
#[test]
fn a_mutable_use_of_a_narrowed_place_borrows_the_storage() {
    let files = generate(&[("main.sv", NARROW_MUT_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs");
    let src = &main.content;
    for needle in [
        // The optional, and the `state` slot inside the generated pass.
        "next__5(p.as_mut().unwrap())",
        "next__5(__p.inner.as_mut().unwrap())",
        // The union arm.
        "next__5(q.u1_mut())",
        // The assignment base.
        "r.as_mut().unwrap().at = 2",
    ] {
        assert!(src.contains(needle), "expected `{needle}` in:\n{src}");
    }
    assert!(
        !src.contains("&mut (p.as_ref"),
        "a mutable use must not borrow a clone:\n{src}"
    );
}

/// [rs-narrow-mut] The other two halves of the same rule, found 2026-09-20
/// while designing `?.` and silently wrong until then: the **intrinsic**
/// argument path and the `^` **branch shadow** [rs-widen-shadow].
///
/// The 2026-09-10 fix covered declared calls (`ParamMode::RefMut` →
/// `borrowed_mut_arg`) and assignment bases. It missed:
///
/// - an **intrinsic** whose parameter is `Mut` — `intrinsic_arg_code` rendered
///   every place with `emit_place`, so `add(a, 9)` on a narrowed
///   `Mut List<Int>?` emitted `a.as_ref().unwrap().clone().push(9)`: compiles,
///   warns about nothing, drops the element. Every read cloned afresh, so the
///   mutation was gone on the next line;
/// - the `^` branch shadow, which bound an owned clone, so a mutation *held*
///   inside the branch and vanished when it ended.
///
/// Both disagreed with Kotlin, which casts the storage and mutates the real
/// value — the repro and the reasoning are in COMPLETED.md. The shapes here are
/// the ones that were wrong, each with a different storage: a variable and a
/// struct field over the nullable representation, a wrapper arm through `^` and
/// through `when`/`is`, a wrapper behind an `Option`, a **moved** union
/// parameter (whose binder needs `mut`), and a handler **state field**
/// (`self.held`).
const NARROW_MUT_ARM_DEMO: &str = r#"
struct Shelf canbe Mut {
    items: Mut List<Int>? = None
}

effect Bag {
    fn stash(n: Int) -> None
    fn dump() -> Str
}

handler Holder of Bag {
    held: Ok Mut List<Int> | Err Str = ok(mut_list_of(0))

    fn stash(n: Int) -> None {
        if held is ^Ok {
            add(held, n)
        }
    }

    fn dump() -> Str {
        if held is ^Ok {
            return to_str(held)
        }
        return "err"
    }
}

fn eat(o: Ok Mut List<Int> | Err Str) [] -> Str => !o {
    if o is ^Ok {
        add(o, 7)
        return to_str(o)
    }
    return "err"
}

fn main() [use] {
    use StdOutConsole()
    use Holder()

    let a: Mut List<Int>? = mut_list_of(1)
    if a is Mut { add(a, 9) }
    if a is Mut { println("var ${to_str(a)}") }

    let b = Mut Shelf { items: mut_list_of(1) }
    if b.items is Mut { add(b.items, 9) }
    if b.items is Mut { println("field ${to_str(b.items)}") }

    let c: Ok Mut List<Int> | Err Str = ok(mut_list_of(1))
    if c is ^Ok { add(c, 9) }
    if c is ^Ok { println("widen ${to_str(c)}") }

    let d: Ok Mut List<Int> | Err Str = ok(mut_list_of(1))
    when d { is Ok { add(d, 9) } is Err { } }
    if d is ^Ok { println("when ${to_str(d)}") }

    let e: Ok Mut List<Int> | Err Str | None = ok(mut_list_of(1))
    if e is ^Ok { add(e, 9) }
    if e is ^Ok { println("nullable ${to_str(e)}") }

    println("moved ${eat(ok(mut_list_of(1)))}")
    stash(5)
    println("state ${dump()}")
}
"#;

/// Every line reads back what was just written. The Kotlin backend asserts the
/// same string — the point of the fix is that the two agree [backend-parity].
const NARROW_MUT_ARM_OUTPUT: &str = "var [1, 9]\nfield [1, 9]\nwiden [1, 9]\nwhen [1, 9]\nnullable [1, 9]\nmoved [1, 7]\nstate [0, 5]\n";

#[test]
fn rustc_compiles_and_runs_mutation_through_a_narrowed_mut_arm() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", NARROW_MUT_ARM_DEMO)]);
    run_rust_files(&files, "narrow-mut-arm", NARROW_MUT_ARM_OUTPUT);
}

/// [rs-narrow-mut] The same regression caught off the generated source, with no
/// toolchain on PATH. The negative assertions are the sharp ones: the two
/// clone-then-mutate shapes that were the defect must appear nowhere.
#[test]
fn a_mut_payload_is_peeled_by_borrowing_the_storage() {
    let files = generate(&[("main.sv", NARROW_MUT_ARM_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs");
    let src = &main.content;
    for needle in [
        // The intrinsic path: a variable and a field over the nullable repr.
        "a.as_mut().unwrap().push(9)",
        "b.items.as_mut().unwrap().push(9)",
        // The intrinsic path over a wrapper arm, narrowed by `when`/`is`.
        "d.u1_mut().push(9)",
        // The `^` shadow: a borrow into the storage, and no `mut` on a binding
        // that is already a reference.
        "let c = c.u1_mut();",
        "let e = e.as_mut().unwrap().u1_mut();",
        // A moved union parameter binds `mut`, or the peel cannot borrow it.
        "pub fn eat(mut o: ",
        // A handler state field peels through `self`, shadowing the field's
        // name for the branch.
        "let held = self.held.u1_mut();",
    ] {
        assert!(src.contains(needle), "expected `{needle}` in:\n{src}");
    }
    for forbidden in [
        ".as_ref().unwrap().clone().push(",
        ".u1().clone().push(",
        "let mut c = c.u1().clone();",
    ] {
        assert!(
            !src.contains(forbidden),
            "a mutation must not land on a clone (`{forbidden}`):\n{src}"
        );
    }
}

/// [rs-narrow-mut] [backend-never-wrong] The one shape with no `&mut` to give:
/// a union-typed parameter the frame received **borrowed**, whose arm is `Mut`.
/// Kotlin mutates the caller's value; Rust cannot, and the deduction system has
/// no way to ask for the `&mut` (a written `=> o: Mut` is refused, since the
/// `Mut` is the arm's claim and not the parameter's). So it is a reported error
/// naming the remedy, not rustc's E0596 and certainly not the clone.
#[test]
fn peeling_a_mut_arm_out_of_a_borrowed_parameter_is_reported() {
    let errors = expect_errors(
        r#"
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)
    }
}
"#,
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("read-only here") && e.contains("union arm")),
        "expected the borrowed-parameter refusal, got: {errors:?}"
    );
}

/// [expr-escape] The three escapes are expressions of type `Never` (user
/// decision 2026-09-21, step 2 of the `?` family sequence). Two shapes that
/// were *ungrammatical* before it, because `return`/`break`/`continue` were
/// statements:
///
/// - an escape nested in an **argument position** (`twice(return 0)`), which is
///   the same shape a `Never`-returning call like `throw(m)` always had;
/// - a `break` as the value of an `if` feeding a `let`, which is the tail
///   position `?:`'s right-hand side will need in step 5.
///
/// Both backends render the escape natively, so the assertion is the output.
const ESCAPE_EXPR_DEMO: &str = r#"
fn twice(n: Int) -> Int {
    return n * 2
}

fn guarded(n: Int) -> Int {
    if n < 0 {
        return twice(return 0)
    }
    return n
}

fn first_big(limit: Int) -> Int {
    let total: Int = 0
    let i: Int = 0
    while true {
        let step: Int = if i < limit { i } else { break }
        total = total + step
        i = i + 1
    }
    return total
}

fn main() [use] {
    use StdOutConsole()
    println("guarded(-1) ${guarded(-1)}")
    println("guarded(7) ${guarded(7)}")
    println("first_big(4) ${first_big(4)}")
}
"#;

const ESCAPE_EXPR_OUTPUT: &str = "guarded(-1) 0\nguarded(7) 7\nfirst_big(4) 6\n";

#[test]
fn rustc_compiles_and_runs_escapes_in_expression_position() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ESCAPE_EXPR_DEMO)]);
    run_rust_files(&files, "escape-expr", ESCAPE_EXPR_OUTPUT);
}

/// [qual-lift] Step 3's new form: a lift **binding**. Two shapes, because they
/// emit differently — a lift that peels a wrapper arm reads the payload out of
/// the storage, while a lift of a *qualifier only* binds the value itself,
/// since qualifiers are erased. The second was the bug this test was written
/// against: the `is`-binding path assumed a payload read and emitted
/// `list.as_ref().unwrap().clone()` for a plain `&mut Vec` on Rust, and a cast
/// to a non-existent type (`list as Mut`) on Kotlin.
const LIFT_BINDING_DEMO: &str = r#"
fn classify(n: Int) -> Ok Int | Err Str {
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn describe(n: Int) [Console] -> None {
    let outcome = classify(n)
    if outcome is ^Ok value {
        println("ok ${value}")
    }
    if outcome is Err {
        println("err ${outcome}")
    }
}

fn read_only(list: Mut List<Int>) [Console] -> None => list: Mut {
    if list is ^Mut plain {
        println("size ${size(plain)}")
    }
}

fn main() [use] {
    use StdOutConsole()
    describe(7)
    describe(0)
    read_only(mut_list_of(1, 2, 3))
}
"#;

const LIFT_BINDING_OUTPUT: &str = "ok 7\nerr zero\nsize 3\n";

#[test]
fn rustc_compiles_and_runs_a_lift_binding() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LIFT_BINDING_DEMO)]);
    run_rust_files(&files, "lift-binding", LIFT_BINDING_OUTPUT);
}

/// [elvis] [placeholder] Step 4: `?:` picks the non-`None` arms, and `_` on the
/// right is the `None` side. The shapes here are the ones that emit differently:
///
/// - an escaping right side (`?: return None`), which is why step 2 came first;
/// - a value right side, where the two sides must agree on a representation —
///   a bare `Str` **wraps** into the subject's `Str | Int` union, exactly as an
///   `if`'s branches converge;
/// - precedence: `n ?: 0 > 3` is `(n ?: 0) > 3` and `n ?: 1 + 1` is
///   `n ?: (1 + 1)`, Kotlin's tier;
/// - right-associative chaining.
const ELVIS_DEMO: &str = r#"
struct Person {
    name: Str,
    nickname: Str? = None
}

fn shout(p: Person) -> Str? {
    let nick: Str = p.nickname ?: return None
    return "${nick}!"
}

fn greet(p: Person) -> Str {
    let nick: Str = p.nickname ?: "friend"
    return "hello ${nick}"
}

fn pick(n: Int) -> Str | Int | None {
    if n == 0 {
        return None
    }
    if n == 1 {
        return "one"
    }
    return n
}

fn main() [use] {
    use StdOutConsole()
    let named = Person {name: "Ada", nickname: "Addy"}
    let plain = Person {name: "Bob"}
    let s = shout(named)
    if s is Str {
        println("shout ${s}")
    }
    if shout(plain) is None {
        println("shout none")
    }
    println(greet(named))
    println(greet(plain))

    let v: Str | Int = pick(2) ?: "none"
    when v {
        is Str { println("str ${v}") }
        is Int { println("int ${v}") }
    }
    let n: Int? = 5
    if n ?: 0 > 3 {
        println("bigger")
    }
    println("sum ${n ?: 1 + 1}")
    let a: Int? = None
    let b: Int? = 7
    println("chain ${a ?: b ?: 0}")
}
"#;

const ELVIS_OUTPUT: &str = "shout Addy!\nshout none\nhello Addy\nhello friend\nint 2\nbigger\nsum 5\nchain 7\n";

#[test]
fn rustc_compiles_and_runs_the_elvis_operator() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ELVIS_DEMO)]);
    run_rust_files(&files, "elvis", ELVIS_OUTPUT);
}

/// [safe-call] Step 4's other half: `?.` reads a field or calls a dot-notation
/// function on the non-`None` side, and the result carries a `None` arm of its
/// own — so it composes with `?:` and re-tests at each link.
///
/// The four shapes are the ones that emit differently: a member whose type is
/// plain (needs the wrapper on Rust) and one already optional (must **not** be
/// wrapped twice — an E0308 rustc caught, which Kotlin never saw, having no
/// wrapper to double), each with the receiver present and absent.
const SAFE_CALL_DEMO: &str = r#"
struct Address {
    city: Str,
    zip: Str? = None
}

struct Person {
    name: Str,
    address: Address? = None
}

fn show(label: Str, v: Str?) [Console] -> None {
    let s: Str = v ?: "-"
    println("${label} ${s}")
}

fn main() [use] {
    use StdOutConsole()
    let full = Person {name: "Ada", address: Address {city: "Bath", zip: "BA1"}}
    let bare = Person {name: "Bob"}
    show("city", full.address?.city)
    show("nocity", bare.address?.city)
    show("zip", full.address?.zip)
    show("nozip", bare.address?.zip)
    let nick: Str? = "addy"
    show("call", nick?.to_upper())
}
"#;

const SAFE_CALL_OUTPUT: &str = "city Bath\nnocity -\nzip BA1\nnozip -\ncall ADDY\n";

#[test]
fn rustc_compiles_and_runs_the_safe_call() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SAFE_CALL_DEMO)]);
    run_rust_files(&files, "safe-call", SAFE_CALL_OUTPUT);
}

/// [pick] Step 5: the qualifier form of `?:`. `^Ok?:` **lifts** the tag, so the
/// picked value is a plain `Int`; `Ok?:` keeps it, so the value is an `Ok Int`
/// that can be returned as-is. `_` on the right is the **unpicked** arm with its
/// own tag, which is the one place the placeholder earns its keep — `Err Str` has
/// no other spelling there.
///
/// The subject is a **call**, so it is evaluated once into a temporary
/// [is-bind-once]: the test, the picked read and `_` all reach the same value.
const PICK_DEMO: &str = r#"
fn parse(text: Str) -> Ok Int | Err Str {
    if size(text) == 0 {
        return err("empty")
    }
    return ok(size(text))
}

fn doubled(text: Str) -> Ok Int | Err Str {
    let n: Int = parse(text) ^Ok?: return _
    return ok(n * 2)
}

fn kept(text: Str) -> Ok Int | Err Str {
    let n: Ok Int = parse(text) Ok?: return _
    return n
}

fn show(label: Str, r: Ok Int | Err Str) [Console] -> None {
    when r {
        is Ok { println("${label} ok ${r}") }
        is Err { println("${label} err ${r}") }
    }
}

// [elvis-guard] After a guarding pick the subject reads as the matched *arm*:
// `r` is an `Ok Int` below, so it needs no further check.
fn guarded(text: Str) [Console] -> None {
    let r = parse(text)
    let n: Int = r ^Ok?: return
    let same: Ok Int = r
    println("guarded ${n} ${same}")
}

fn main() [use] {
    use StdOutConsole()
    show("doubled", doubled("abc"))
    show("doubled", doubled(""))
    show("kept", kept("abcd"))
    show("kept", kept(""))
    guarded("hello")
    guarded("")
}
"#;

const PICK_OUTPUT: &str =
    "doubled ok 6\ndoubled err empty\nkept ok 4\nkept err empty\nguarded 5 5\n";

#[test]
fn rustc_compiles_and_runs_a_qualifier_pick() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PICK_DEMO)]);
    run_rust_files(&files, "pick", PICK_OUTPUT);
}

/// [rewrap] A value spanning **fewer arms than its storage** is produced by
/// mapping arm to arm, which is what the three shapes deferred out of the `?`
/// family all needed. Two of them are here:
///
/// - a **multi-arm lift binding**: `o is ^Ok value` where two arms carry `Ok`,
///   so `value` is an `Int | Str` out of a three-arm union;
/// - a **pick whose unpicked side spans two arms**: `_` is `Err Str | Thrown Str`
///   out of the same three-arm storage — the shape from the original sketch.
///
/// The mapping itself is the one an annotated `let` has always emitted
/// [let-infer]; what was missing was reaching it from a site with no slot.
const REWRAP_DEMO: &str = r#"
fn classify(n: Int) -> Ok Int | Ok Str | Err Str {
    if n == 0 {
        return err("zero")
    }
    if n > 0 {
        return ok(n)
    }
    return ok("negative")
}

fn attempt(text: Str) -> Ok Int | Err Str | Thrown Str {
    if size(text) == 0 {
        return err("empty")
    }
    if size(text) > 5 {
        return thrown("too long")
    }
    return ok(size(text))
}

fn show(n: Int) [Console] -> None {
    let o = classify(n)
    if o is ^Ok value {
        when value {
            is Int { println("int ${value}") }
            is Str { println("str ${value}") }
        }
    }
    if o is Err {
        println("err ${o}")
    }
}

fn run(text: Str) -> Err Str | Thrown Str {
    let n: Int = attempt(text) ^Ok?: return _
    return err("len ${n}")
}

fn report(text: Str) [Console] -> None {
    let r = run(text)
    when r {
        is Err { println("err ${r}") }
        is Thrown { println("thrown ${r}") }
    }
}

fn main() [use] {
    use StdOutConsole()
    show(3)
    show(-1)
    show(0)
    report("abc")
    report("")
    report("abcdefgh")
}
"#;

const REWRAP_OUTPUT: &str =
    "int 3\nstr negative\nerr zero\nerr len 3\nerr empty\nthrown too long\n";

#[test]
fn rustc_compiles_and_runs_a_sub_union_rewrap() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", REWRAP_DEMO)]);
    run_rust_files(&files, "rewrap", REWRAP_OUTPUT);
}

#[test]
fn rustc_compiles_and_runs_a_group_over_plain_arms() {    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PLAIN_GROUP_DEMO)]);
    run_rust_files(&files, "plain-group", PLAIN_GROUP_OUTPUT);
}

#[test]
fn rustc_compiles_and_runs_a_hand_written_pass() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PASS_DEMO)]);
    run_rust_files(&files, "hand-written-pass", PASS_OUTPUT);
}

/// [implicit-param] An effect member is an ordinary signature, so it may
/// declare implicit parameters (user decision 2026-09-06): the trait method
/// takes them, every handler's implementation takes them, and the call site
/// fills them. They render as `dyn` here, not `impl` — an effect trait is
/// used as `&mut dyn E` [rs-effects], and `impl Trait` in argument position
/// would cost object safety.
const MEMBER_IMPLICIT_DEMO: &str = r#"
effect Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v
}

handler Angle of Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v {
        return "<${fmt(v)}>"
    }
}

fn fmt(n: Int) -> Str => n {
    return "n=${n}"
}

fn loud(n: Int) -> Str => n {
    return "N=${n}!"
}

fn main() [use] {
    use StdOutConsole()
    use Angle()
    println(show(7))
    println(show(7, fmt = loud))
}
"#;

const MEMBER_IMPLICIT_OUTPUT: &str = "<n=7>\n<N=7!>\n";

#[test]
fn an_effect_member_carries_its_implicits_into_the_trait() {
    let files = generate(&[("main.sv", MEMBER_IMPLICIT_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The trait method, object-safe.
        "fn show(&mut self, v: i32, fmt: &mut dyn FnMut(i32) -> String) -> String;",
        // The implementation has to match it exactly.
        "fn show(&mut self, v: i32, fmt: &mut dyn FnMut(i32) -> String) -> String {",
        // And the call site fills it.
        "show.show(7, &mut |__i0| fmt(__i0))",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

#[test]
fn rustc_compiles_and_runs_effect_member_implicits() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MEMBER_IMPLICIT_DEMO)]);
    run_rust_files(&files, "member-implicits", MEMBER_IMPLICIT_OUTPUT);
}

// ===== [effect-handler-generics] a `use` gives its handler type arguments =====

/// A stateless generic handler: `use Plain<Int>()` is the *only* thing that
/// can say what `T` is, since there is no constructor argument to infer it
/// from. The written arguments used to be discarded by the checker and both
/// emitters, so this program type-checked and then failed in rustc
/// (`E0283`, plus `E0392` for the handler struct) — a [backend-never-wrong]
/// violation, fixed 2026-09-06.
const HANDLER_GENERICS_DEMO: &str = r#"
effect Show<T> {
    fn show(v: T) -> Str => v
}

// Stateless *and* generic: nothing but the `use` site can say what `T` is.
handler Plain<T> of Show<T> {
    fn show(v: T) -> Str => v {
        return "plain"
    }
}

effect Tag<T> {
    fn tagged(v: T) -> Str => v
}

// Generic with state: the constructor argument used to be the only thing
// that could bind `T`, and still works.
handler Prefixed<T>(prefix: Str) of Tag<T> {
    fn tagged(v: T) -> Str => v {
        return "${prefix}!"
    }
}

fn main() [use] {
    use StdOutConsole()
    use Plain<Int>()
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}
"#;

const HANDLER_GENERICS_OUTPUT: &str = "plain\np!\n";

#[test]
fn a_generic_handler_is_constructed_at_its_type() {
    let files = generate(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The turbofish, from the written type argument alone...
        "let mut show_i32 = Plain::<i32>::new();",
        // ...and where a constructor argument could also have bound it.
        "let mut tag_i32 = Prefixed::<i32>::new(\"p\".to_string());",
        // A type parameter no field mentions needs `PhantomData`, or the
        // struct itself does not compile (`E0392`).
        "__phantom_T: std::marker::PhantomData<T>,",
        "__phantom_T: std::marker::PhantomData,",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

#[test]
fn rustc_compiles_and_runs_a_generic_handler() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    run_rust_files(&files, "handler-generics", HANDLER_GENERICS_OUTPUT);
}

// ===== qualifier refinements [qual-refn] =====

/// A refinement is *compile-time only*: `NonEmpty` is erased like every
/// qualifier [qual-erasure], and a refinement adds no call, no check and no
/// wrapper — it only decides which overload the checker picks. So the two
/// backends run the same source to the same output with no refinement
/// machinery anywhere in between.
const REFN_DEMO: &str = r#"
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

// Only callable while the compiler still believes the list is non-empty.
fn count<T canbe linear>(list: NonEmpty List<T>) -> Int => list {
    return size(list)
}

fn refill(list: Mut NonEmpty List<Int>, value: Int) -> None => list: Mut NonEmpty {
    add(list, value)
}

fn main() [use] {
    use StdOutConsole()
    let xs: Mut List<Int> = mut_list_of()
    add(xs, 1)
    println("after add: ${count(xs)}")
    refill(xs, 2)
    println("after refill: ${count(xs)}")
}
"#;

const REFN_EXPECTED: &str = "after add: 1\nafter refill: 2\n";

/// [qual-refn] [qual-erasure] The same source and the same expected stdout as
/// the Kotlin backend asserts — which is the parity claim itself. The emitted
/// Rust carries no trace of the refinement.
#[test]
fn rustc_compiles_and_runs_a_refined_program() {
    let files = generate(&[("main.sv", REFN_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The qualifier's `qualifies` fn is still emitted (it backs `is`
    // checks), but a refinement is *trusted* like `-> T as Q`
    // [qual-ctor-fn]: no runtime check is emitted where it applies.
    let body = main
        .content
        .split("pub fn main(")
        .nth(1)
        .expect("main emitted");
    assert!(
        !body.contains("NonEmpty_qualifies"),
        "a refinement must not emit a runtime check:\n{body}"
    );
    run_rust_files(&files, "refn", REFN_EXPECTED);
}

/// [qual-refn-conflict] [diag-structured] A suppressed refinement conflict is
/// a *warning*: the program is legal, so it must still emit — *and* the
/// warning must reach the driver, or the diagnostic exists only in `salvo
/// analyze`. Tested per backend because both the gate and the channel are
/// duplicated in each: the same condition has to fire on both sides.
// `with NonEmpty` on both: std's own `NonEmpty` refines `add` too
// [col-nonempty], so without it this would assert a three-way disagreement
// including std's, rather than the rule. Declaring compatibility with std's
// claim is the one-word remedy, and leaves the Q1-vs-Q2 conflict intact.
const REFN_CONFLICT_DEMO: &str = r#"
qualifier Q1<T> of List<T> with NonEmpty {
    fn qualifies(list: List<T>) -> Bool { return size(list) > 0 }
    refn add(list: Mut List<T>, elem: T) => list: +Q1
}

qualifier Q2<T> of List<T> with NonEmpty {
    fn qualifies(list: List<T>) -> Bool { return size(list) > 0 }
    refn add(list: Mut List<T>, elem: T) => list: +Q2
}

fn main() [use] {
    use StdOutConsole()
    let xs: Mut List<Int> = mut_list_of()
    add(xs, 1)
    if xs is Q1 {
        println("checked by hand: ${size(xs)}")
    }
}
"#;

#[test]
fn a_refinement_conflict_warns_without_stopping_emission() {
    let program = build_program(&[("main.sv", REFN_CONFLICT_DEMO)]);
    let (files, warnings) = salvo_backend_rust::emit_program_reporting(&program, None)
        .unwrap_or_else(|errors| panic!("a warning must not stop emission: {errors:?}"));
    assert!(!files.is_empty(), "the program should still emit");
    assert_eq!(warnings.len(), 1, "warnings: {warnings:?}");
    assert!(
        warnings[0].starts_with("warning: the refinements of `Q1` and `Q2` disagree")
            && warnings[0].contains("main.sv:"),
        "a rendered warning with its location: {}",
        warnings[0]
    );
}

// ===== [fn-overload-rank] O1: concrete beats generic =====

/// The same source and the same expected stdout as the Kotlin backend's
/// `kotlinc_runs_the_most_specific_overload`: the winner is the checker's
/// choice, so the two targets must agree on it.
const OVERLOAD_SPECIFICITY: &str = r#"
fn describe<T>(value: T) -> Str {
    return "generic"
}

fn describe(value: Int) -> Str {
    return "concrete"
}

fn main() [use] {
    use StdOutConsole()
    println(describe(3))
    println(describe("text"))
}
"#;

#[test]
fn rustc_runs_the_most_specific_overload() {
    let files = generate(&[("main.sv", OVERLOAD_SPECIFICITY)]);
    run_rust_files(&files, "overload-specificity", "concrete\ngeneric\n");
}

// ===== [str-drop-mut] [rs-mut-str] `Mut Str` is a `String` =====

/// The same source and the same expected stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_strings`: `Mut Str` is a *different type*
/// there (`StringBuilder`) and the same one here, so what the pair asserts
/// is that dropping `Mut` costs the program nothing on either target.
const STRING_DEMO: &str = r#"
fn shout(text: Str) -> Str {
    return to_upper(text)
}

fn main() [use] {
    use StdOutConsole()
    // A builder, and the drop that lets it reach the `Str` surface.
    let b = mut_str("he", "llo")
    append(b, " world")
    set(b, 0, 'H')
    println(shout(b))
    println("size: ${size(b)}")
    // `copy` of a builder is a new builder, not an alias.
    let dup = copy(b)
    append(dup, "!")
    println("${b} / ${dup}")
    clear(dup)
    println("cleared: [${dup}]")

    let sep = ","
    let dash = "-"
    let parts = split("a,b,,c", sep)
    let joined = join(parts, dash)
    println("${size(parts)} ${joined}")

    let trimmed = trim("  pad  ")
    let pa = "pa"
    let ad = "ad"
    println("[${trimmed}] ${starts_with(trimmed, pa)} ${ends_with(trimmed, ad)} ${contains(trimmed, ad)}")
    println(to_lower(shout(trimmed)))

    let hay = "hello"
    let ll = "ll"
    let lo = "lo"
    let at = index_of(hay, ll)
    let nowhere = index_of(hay, pa)
    println("${at!} ${nowhere is None}")
    let sub = substr(hay, 1, 3)
    let oob = substr(hay, 1, 9)
    println("${sub!} ${oob is None}")
    let n = parse_int("42")
    let bad = parse_int(pa)
    println("${n!} ${bad is None}")
    println("${trim_prefix(hay, hay)}${trim_suffix(hay, lo)}|")

    let count = mut_list_of<Int>()
    for c in iter(hay) {
        add(count, 1)
    }
    println("chars: ${size(count)}")

    // [bytes-type] [byte-value] The text/byte bridge: UTF-8 out, strict UTF-8
    // back in, and an octet that is unsigned on both targets. The buffer is a
    // `Bytes`, not a list of octets, so nothing here boxes on the JVM.
    let bytes = to_bytes("hé")
    let round = str_of_bytes(bytes)
    let broken = mut_bytes()
    add(broken, to_byte(200))
    println("bytes: ${bytes} ${to_hex(bytes)} ${round!} ${str_of_bytes(broken) is None}")

    // Operators drop `Mut` too, so equality is by content on both targets.
    let x = mut_str(pa)
    let y = mut_str(pa)
    println("equal: ${x == y}")
}
"#;

/// The stdout both backends must produce, byte for byte.
const STRING_DEMO_OUTPUT: &str = "HELLO WORLD\nsize: 11\nHello world / Hello world!\n\
                                  cleared: []\n4 a-b--c\n[pad] true true true\npad\n\
                                  2 true\nel true\n42 true\nhel|\nchars: 5\n\
                                  bytes: [104, 195, 169] 68c3a9 hé true\nequal: true\n";

/// [rs-mut-str] `Mut` erases here — `Mut Str` and `Str` are both `String` —
/// so a drop renders *nothing*, and the string surface is a set of `&str`
/// methods with the char-vs-byte corrections Salvo's semantics need.
#[test]
fn mut_str_is_a_plain_string() {
    let files = generate(&[("main.sv", STRING_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [str-drop-mut] No conversion anywhere: the argument is borrowed as it
    // stands, and interpolation prints the `String` itself.
    assert!(main.contains("shout(&b)"), "unexpected:\n{main}");
    assert!(
        main.contains(r#"format!("{} / {}", b, dup)"#),
        "unexpected:\n{main}"
    );
    // The parts are borrowed, not moved: a variadic position is untracked by
    // the flow analysis, so an owned `vec![pa]` would move a variable the
    // checker still considers live.
    assert!(
        main.contains(r#"let mut x = [&pa[..]].concat();"#),
        "unexpected:\n{main}"
    );
    // Characters, not bytes — `find` answers in bytes, so the prefix is
    // re-counted.
    assert!(
        main.contains("__s.find(&ll[..]).map(|__b| __s[..__b].chars().count() as i32)"),
        "unexpected:\n{main}"
    );
    // [rs-mut-str] `set` goes through the generated trait: method syntax
    // auto-refs an owned local and a `&mut String` parameter alike.
    assert!(main.contains("b.salvo_set(0, 'H')"), "unexpected:\n{main}");
    assert!(
        main.contains("use crate::strings::*;"),
        "unexpected:\n{main}"
    );
    let support = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "strings.rs")
        .expect("strings.rs emitted");
    assert!(
        support.content.contains("pub trait SalvoStr")
            && support.content.contains("impl SalvoStr for String"),
        "unexpected:\n{}",
        support.content
    );
    // The support file is only generated when something needs it.
    let plain = generate(&[("main.sv", "export fn main() {\n}\n")]);
    assert!(
        !plain
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "strings.rs"),
        "the string helpers must not be emitted for a program that never mutates a string"
    );
}

#[test]
fn rustc_compiles_and_runs_strings() {
    let files = generate(&[("main.sv", STRING_DEMO)]);
    run_rust_files(&files, "strings", STRING_DEMO_OUTPUT);
}

/// [fn-variadic] A `...spread` into a variadic intrinsic *is* the whole
/// collection: `vec![parts]` would be a vector of one vector. Cloned rather
/// than moved, matching Kotlin's `listOf(*arr)` — and the string parts are
/// borrowed, since `mut_str` reads them.
#[test]
fn a_spread_into_a_variadic_intrinsic_is_the_collection() {
    let src = r#"
export fn main() [use] {
    use StdOutConsole()
    let parts = array_of("a", "b")
    let sb = mut_str(...parts)
    let xs = set_of(...parts)
    println("${sb} ${size(xs)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [col-of-nonempty] `set_of` rather than `list_of`: the list constructors
    // now have a *first* parameter, which a spread may not supply
    // [fn-variadic], so a lone spread reaches the purely variadic intrinsics —
    // which is where this lowering lives anyway.
    assert!(
        main.contains("let mut sb = parts.concat();")
            && main.contains("parts.iter().cloned()"),
        "unexpected:\n{main}"
    );
    run_rust_files(&files, "strings-spread", "ab 2\n");
}

/// The case the generated trait exists for: `set` through a `&mut String`
/// *parameter* and through a **field** projection, next to an owned local.
/// An inline `&mut` re-borrow fails on the parameter (E0596), and method
/// syntax works for all three. Same source and stdout as the Kotlin
/// backend's `kotlinc_compiles_and_runs_mut_str_places`.
const MUT_STR_PLACES: &str = r#"
struct Buf canbe Mut {
    text: Mut Str
}

fn grow(s: Mut Str) -> None => s: Mut {
    append(s, "!")
    set(s, 0, 'G')
    clear(s)
    append(s, "grown")
}

fn main() [use] {
    use StdOutConsole()
    let b = mut_str("seed")
    grow(b)
    println("${b} ${size(b)}")
    let buf = Mut Buf {text: mut_str("in-struct")}
    append(buf.text, "!")
    set(buf.text, 0, 'I')
    println("${buf.text}")
}
"#;

#[test]
fn rustc_compiles_and_runs_mut_str_places() {
    let files = generate(&[("main.sv", MUT_STR_PLACES)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // A `Mut Str` parameter is a `&mut String`, and the helper is reached by
    // method syntax on it.
    assert!(
        main.contains("pub fn grow(s: &mut String)") && main.contains("s.salvo_set(0, 'G')"),
        "unexpected:\n{main}"
    );
    assert!(
        main.contains("buf.text.salvo_set(0, 'I')"),
        "unexpected:\n{main}"
    );
    run_rust_files(&files, "mut-str-places", "grown 5\nIn-struct!\n");
}

// ===== [rs-seq] [seq-pass] [implicit-group] the sequence functions =====

/// The same source and stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_sequences`: `map`/`filter`/`reduce` over a `List`
/// (the intrinsic fast path), and over the *passes* an array, a `Str`, an origin
/// and a struct of the program's own hand out — `iter` written at each use site
/// [seq-pass].
const SEQ_DEMO: &str = r#"
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListYield<Int> => bag {
    return iter(bag.items)
}

fn double(n: Int) -> Int {
    return n * 2
}

struct Naturals {
    from: Int
}

fn naturals(from: Int) -> Naturals {
    return Naturals {from: from}
}

iter fn next(n: Naturals) -> Emitted Int | Finished {
    state {
        at: Int = n.from
    }
    if at >= n.from + 4 {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] {
    use StdOutConsole()
    let xs = list_of(1, 2, 3, 4)
    let doubled = map(xs, n -> n * 2)
    let sum = reduce(xs, 0, (a, b) -> a + b)
    let big = filter(xs, n -> n > 2)
    println("list: ${size(doubled)} ${sum} ${size(big)}")
    let named = map(xs, double)
    println("named: ${size(named)}")
    let arr = array_of(10, 20, 30)
    let arr_sum = reduce(iter(copy(arr)), 0, (a, b) -> a + b)
    let arr_mapped = map(iter(arr), n -> n + 1)
    println("array: ${arr_sum} ${size(arr_mapped)}")
    let hello = "hello"
    let letters = filter(iter(hello), c -> c == 'l')
    println("chars: ${size(letters)}")
    let lazy_sum = reduce(iter(naturals(1)), 0, (a, b) -> a + b)
    let tripled = map(xs, n -> n * 3)
    let chained = filter(iter(tripled), n -> n > 6)
    println("iter: ${lazy_sum} ${size(chained)}")
    let names = list_of("ann", "bob", "carol")
    let lens = map(names, n -> size(n))
    let long = filter(names, n -> size(n) > 3)
    println("names: ${size(lens)} ${size(long)}")
    let bag = Bag {items: list_of(5, 6)}
    println("bag: ${reduce(iter(bag), 0, (a, b) -> a + b)}")
    // [iter-pass] `for` over a *container of one's own*: the loop calls its
    // `iter` once and drives the pass that answers. Until 2026-09-09 the
    // checker recorded that mint and neither emitter made the call, so this
    // shape emitted code the target compiler rejected.
    // (a second bag, because this demo's `iter` *moves* its container — the
    // one above was consumed by the `reduce`.)
    let more = Bag {items: list_of(5, 6)}
    for n in more {
        println("bag element ${n}")
    }
}
"#;

const SEQ_DEMO_OUTPUT: &str = "list: 4 10 2\nnamed: 4\narray: 60 3\nchars: 2\n\
                               iter: 10 2\nnames: 3 1\nbag: 11\n\
                               bag element 5\nbag element 6\n";

/// [rs-seq] The `List` fast paths go through the generated helpers, whose
/// generic parameters are what give a callback its expected type — the whole
/// reason they are functions rather than inline expressions.
/// [implicit-intrinsic] The generic overload's implicit `iter` arrives as an
/// adapter closure whose body is the *intrinsic's own lowering*: emitting
/// `iter(__i0)` would name the generated `iter` **module** (E0423).
#[test]
fn sequence_functions_lower_to_helpers() {
    let files = generate(&[("main.sv", SEQ_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    assert!(
        main.contains("salvo_map(&xs[..], |n| *n * 2)")
            && main.contains("salvo_reduce(&xs[..], 0,")
            && main.contains("salvo_filter(&xs[..],"),
        "unexpected:\n{main}"
    );
    // A named fn as the callback wraps in an adapter: a fn item's own
    // convention is by value, and the helper hands it `&T` [fn-contract].
    assert!(
        main.contains("let mut named = salvo_map(&xs[..], |"),
        "unexpected:\n{main}"
    );
    // [iter-fn] A pass subject reaches the generic overload through its `iter`,
    // and the `next` the checker resolved arrives as the implicit argument.
    assert!(
        main.contains("__Pass_Naturals"),
        "expected the pass to be minted through its generated `iter` in:\n{main}"
    );
    // The support file is there, and only because something needed it.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "seq.rs"),
        "seq.rs should be emitted"
    );
    let plain = generate(&[("main.sv", "export fn main() {\n}\n")]);
    assert!(
        !plain
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "seq.rs"),
        "the sequence helpers must not be emitted for a program that never uses them"
    );
}

#[test]
fn rustc_compiles_and_runs_sequences() {
    let files = generate(&[("main.sv", SEQ_DEMO)]);
    run_rust_files(&files, "sequences", SEQ_DEMO_OUTPUT);
}

// ===== [fn-overload-at] [fn-rename] the caller's two overrides =====

/// The same source and stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_overload_overrides`: a module-level `size` that
/// shadows std's, `@module` to reach past it (and past a *local* of the same
/// name), and a `rename` giving one of two unrankable overloads a name of its
/// own. Both mechanisms are compile-time only, so what the pair asserts is
/// that the two targets agree about which declaration each call meant.
const OVERLOAD_OVERRIDE_DEMO: &str = r#"
qualifier Even of Int with Small {
    fn qualifies(n: Int) -> Bool { return n % 2 == 0 }
}

qualifier Small of Int with Even {
    fn qualifies(n: Int) -> Bool { return n < 10 }
}

fn size<T>(list: List<T>) -> Str => list {
    return "mine"
}

fn label(n: Even Int) -> Str => n {
    return "even"
}

fn label(n: Small Int) -> Str => n {
    return "small"
}

// The two `label`s are unrankable — one qualifier each, and the *kind* of
// qualifier never ranks — so one of them takes a name of its own.
rename fn label_small = label(n: Small Int)

fn main() [use] {
    use StdOutConsole()
    let xs = list_of(1, 2, 3)
    // This module's `size` wins; `@core.list` reaches std's.
    println(size(xs))
    println("core: ${size@core.list(xs)}")
    println("dot: ${xs.size@core.list()}")
    let n = 4
    if n is Even && n is Small {
        println("${label(n)} ${label_small(n)}")
    }
    // A local shadows every function of that name — `@` is the way out.
    let describe = "a local"
    println(describe)
    println(describe@main(7))
}

fn describe(n: Int) -> Str {
    return "fn ${n}"
}
"#;

const OVERLOAD_OVERRIDE_OUTPUT: &str = "mine\ncore: 3\ndot: 3\neven small\na local\nfn 7\n";

/// [fn-overload-at] [rs-shadowed-call] Neither override survives into the
/// output — with one Rust-specific consequence: a **local of the same name**
/// shadows the
/// function in Rust's value namespace (E0618), where Kotlin keeps functions
/// and properties apart, so such a call is spelled as a path.
#[test]
fn scope_selectors_and_renames_are_erased() {
    let files = generate(&[("main.sv", OVERLOAD_OVERRIDE_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    assert!(!main.contains('@'), "unexpected `@` in:\n{main}");
    assert!(
        !main.contains("label_small"),
        "unexpected rename in:\n{main}"
    );
    // `@core.list` is std's `size`, mangled because this program declares its
    // own [rs-fn-mangling].
    assert!(
        main.contains("(xs.len() as i32)"),
        "expected the core lowering:\n{main}"
    );
    // The renamed overload is called by its declaration's mangled name.
    assert!(
        main.contains("label__Small(&n)") || main.contains("label__Small(n)"),
        "unexpected:\n{main}"
    );
    // The shadowed call goes through the crate path.
    assert!(
        main.contains("crate::describe(7)"),
        "expected a path past the local:\n{main}"
    );
}

#[test]
fn rustc_compiles_and_runs_overload_overrides() {
    let files = generate(&[("main.sv", OVERLOAD_OVERRIDE_DEMO)]);
    run_rust_files(&files, "overload-overrides", OVERLOAD_OVERRIDE_OUTPUT);
}

// ===== [iter-generic-drive] driving a generic pass =====

/// [iter-drive-in-place] The lowering a *kept* pass gets: no local, no clone —
/// the loop advances the parameter where it lives, which is what makes the
/// caller's next drive continue from where this one stopped. Binding it into a
/// local cloned the `&mut`, and the caller never saw the position (Kotlin
/// aliased it and did, so the two backends disagreed [backend-parity]).
#[test]
fn a_kept_pass_is_driven_in_place() {
    let files = generate(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("while let Union2::U1(mut n) = next(it)"),
        "expected the in-place drive in:\n{src}"
    );
    // Precisely: no local for *this* loop. (A substring like `_pass = it` also
    // matches a mint call — `__loop2_pass = iter__4(…)` — so the assertion names
    // the loop.)
    assert!(
        !src.contains("let mut __loop1_pass"),
        "a kept pass must not be bound into a local in:\n{src}"
    );
}

#[test]
fn rustc_compiles_and_runs_a_generic_drive() {
    let files = generate(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    run_rust_files(&files, "generic-drive", GENERIC_DRIVE_OUTPUT);
}

/// [linear-generics] The discharge is the program's: the pass is driven in
/// place and the consuming callback `end(it)` is the explicit terminal —
/// no loop-local, no deferred splice (user decision 2026-09-12).
#[test]
fn an_owned_generic_pass_is_discharged_by_the_callback() {
    let files = generate(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        !src.contains("__loop1_pass"),
        "a named pass must not be bound into a loop local:\n{src}"
    );
    assert!(
        src.contains("end(it)"),
        "expected the explicit consuming-callback discharge in:\n{src}"
    );
}

#[test]
fn rustc_compiles_and_runs_a_generic_close() {
    let files = generate(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    run_rust_files(&files, "generic-close", GENERIC_CLOSE_OUTPUT);
}

// ===== [iter-fn] the generated-pass form =====

/// [iter-fn] A hand-written `next` whose **pass struct is generated** (user
/// decision 2026-09-09): the subject stays ordinary data, the `state { … }`
/// block is the pass's own fields, and the compiler writes the struct plus the
/// `iter` that mints it.
///
/// Six things in one program, and the same source and stdout on both backends:
/// a `for` over a subject, the subject **replayed** (driving copied it), a pass
/// **held** and driven by hand then finished by a `for` — the capability the
/// `yield fn` origin lacks, since each loop re-mints — several `state` fields,
/// a subject field read on every turn, and an **effectful** `next`, whose
/// handlers are threaded into every turn of the loop.
const ITER_FN_DEMO: &str = r#"
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

struct Fibs {
    count: Int
}

iter fn next(f: Fibs) -> Emitted Int | Finished {
    state {
        a: Int = 0,
        b: Int = 1,
        made: Int = 0
    }
    if made >= f.count {
        return finished()
    }
    let now = copy(a)
    let sum = a + b
    a = copy(b)
    b = copy(sum)
    made = made + 1
    return emitted(now)
}

struct Noisy {
    limit: Int
}

iter fn next(n: Noisy) [Console] -> Emitted Int | Finished {
    state {
        at: Int = 0
    }
    if at >= n.limit {
        println("  done")
        return finished()
    }
    println("  turn ${at}")
    at = at + 1
    return emitted(copy(at))
}

struct Row {
    label: Str,
    times: Int
}

fn describe(r: Row) -> Str => r {
    return "${r.label}!"
}

iter fn next(r: Row) -> Emitted Str | Finished {
    state {
        left: Int = r.times
    }
    if left <= 0 {
        return finished()
    }
    left = left - 1
    return emitted(describe(r))
}

fn main() [use] {
    use StdOutConsole()

    let c = Countdown { from: 3 }
    for n in c {
        println("n ${n}")
    }
    for n in c {
        println("again ${n}")
    }

    let p = iter(c)
    let first = next(p)
    when first {
        is Emitted { println("first ${first}") }
        is Finished { println("empty") }
    }
    for n in p {
        println("rest ${n}")
    }

    println("fib total ${reduce(iter(Fibs { count: 7 }), 0, (acc: Int, n: Int) -> acc + n)}")

    let noisy = Noisy { limit: 2 }
    for v in noisy {
        println("v ${v}")
    }
    let row = Row { label: "hey", times: 2 }
    for s in row {
        println("s ${s}")
    }
}
"#;

const ITER_FN_OUTPUT: &str = "n 3\nn 2\nn 1\nagain 3\nagain 2\nagain 1\nfirst 3\n\
                              rest 2\nrest 1\nfib total 20\n  turn 0\nv 1\n  turn 1\n\
                              v 2\n  done\ns hey!\ns hey!\n";

#[test]
fn rustc_compiles_and_runs_the_iter_fn_form() {
    let files = generate(&[("main.sv", ITER_FN_DEMO)]);
    run_rust_files(&files, "iter-fn", ITER_FN_OUTPUT);
}

/// [iter-fn] What the expansion emits, and what it does *not*: a plain struct
/// carrying the subject and the `state` fields, a plain `iter` that copies the
/// subject in, and a plain `next` — no state machine, no `__advance`, no state
/// number, and no per-`defer` flag.
#[test]
fn an_iter_fn_emits_a_plain_struct_and_next() {
    let files = generate(&[("main.sv", ITER_FN_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [iter-fn] Tier 1 — the body never reads the subject, so the pass holds
    // *nothing* of it: no field, and no clone at the mint.
    assert!(
        main.contains("pub struct __Pass_Countdown {\n    pub at: i32,\n}")
            && main.contains("__Pass_Countdown { at: c.from }"),
        "expected a subject-free pass:\n{main}"
    );
    // Tier 2 — the body reads `f.count` on every turn, so that one field is
    // snapshotted at the mint. The subject itself is still not held, and the
    // snapshot keeps the field's own name.
    assert!(
        main.contains("pub struct __Pass_Fibs {\n    pub count: i32,")
            && main.contains("__Pass_Fibs { count: f.count, a: 0, b: 1, made: 0 }")
            && !main.contains("pub __subject: Fibs"),
        "expected a per-field snapshot:\n{main}"
    );
    // Tier 3 — the body hands the *whole* subject to `describe`, so no snapshot
    // can stand in for it and the pass holds the subject — **borrowed**, as a
    // `proj` field [proj-field]: the struct carries a lifetime and the mint
    // clones nothing [copy-opt-in].
    assert!(
        main.contains("pub struct __Pass_Row<'s> {\n    pub __subject: &'s Row,")
            && main.contains("__Pass_Row { __subject: r, left: r.times }"),
        "expected the whole subject to be borrowed:\n{main}"
    );
    assert!(
        !main.contains("__advance") && !main.contains("__state"),
        "an `iter fn` needs no state machine:\n{main}"
    );
    // [fn-effects] An effectful `next` takes its handlers as leading arguments,
    // threaded into every turn of the loop. The mangling index counts the
    // visible `next` overloads, so it moved when std's lazy pair (two of them)
    // was removed 2026-09-10, again when `Set` and `Map` brought their own
    // passes (two more) 2026-09-13, again when `Bytes` and the fs chunk
    // pass brought two more 2026-09-15, and again when `core.range` joined std
    // 2026-09-23 (its `next` counts even though the module is private), and
    // again when `reversed`/`enumerate` (then `indices`) brought more (the
    // refinement-types sequence, step 0, 2026-09-23).
    assert!(
        main.contains("next__15(&mut console, &mut __loop"),
        "expected the handler threaded into the drive:\n{main}"
    );
}

/// [implicit-infer] A generic function over **containers** rather than passes:
/// it takes the container and asks for its `iter` as an implicit parameter, so
/// the pass type is decided by *which `iter` fills it* and the `?Yield` spread
/// beside it resolves against what that taught (user decision 2026-09-10).
///
/// One function, three container kinds, no type arguments written — including a
/// `iter fn` subject, whose generated pass **cannot** be named, which is why
/// inferring it had to work rather than being worked around.
const CONTAINER_IMPLICIT_DEMO: &str = r#"
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListYield<Int> => bag {
    return iter(bag.items)
}

fn total<C, It>(c: C, ?iter: (c: C) -> proj(c) in (Mut It), ?Yield<It, Int>) -> Int =>[iter] c => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdOutConsole()
    let c = Countdown { from: 3 }
    println("generated pass: ${total(c)}")
    let b = Bag { items: list_of(4, 5) }
    println("written iter: ${total(b)}")
    // A literal, not `list_of(1, 2, 3)`: a *qualified* argument type
    // (`NonEmpty List<Int>` [col-of-nonempty]) currently stops `It` binding
    // through the `?iter` position, and the sibling `next` at the nearer rung
    // then wins — an open defect, with this line as its repro (ROADMAP.md).
    println("a list: ${total([1, 2, 3])}")
}
"#;

const CONTAINER_IMPLICIT_OUTPUT: &str = "generated pass: 6\nwritten iter: 9\na list: 6\n";

#[test]
fn rustc_compiles_and_runs_a_container_combinator() {
    let files = generate(&[("main.sv", CONTAINER_IMPLICIT_DEMO)]);
    run_rust_files(&files, "container-implicit", CONTAINER_IMPLICIT_OUTPUT);
}

/// [fate-field-disjoint] [rs-borrow-locals] L5's payoff on this backend, and
/// the reason it needs no new emission: a borrow-mode binding from `p.name`
/// already emits a real borrow (`let mut n = &p.name;`), and rustc allows it
/// to live across a `&mut` of a *disjoint field* of the same local. So
/// Salvo's field precision and rustc's coincide — these programs were
/// checker-rejected before L5 (the remedy was `copy`, a real clone), and they
/// compile clone-free now.
///
/// Shared with the Kotlin backend, byte for byte.
const FIELD_DISJOINT_DEMO: &str = r#"
struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Pair {
    left: Mut List<Str>,
    right: Mut List<Str>
}

fn touch(list: Mut List<Str>) -> None => list: Mut {
    add(list, "t")
    return None
}

fn main() [use] {
    use StdOutConsole()
    // read one field, mutate another
    let p = Person { name: "ann", tags: mut_list_of("x") }
    let n = p.name
    add(p.tags, "y")
    println("A ${n} ${size(p.tags)}")

    // two disjoint mutable fields
    let q = Pair { left: mut_list_of("l"), right: mut_list_of("r") }
    let l = q.left
    add(q.right, "r2")
    println("C ${size(l)} ${size(q.right)}")

    // read a field, hand a disjoint field to a mutating fn
    let p2 = Person { name: "dee", tags: mut_list_of("x") }
    let n2 = p2.name
    touch(p2.tags)
    println("D ${n2} ${size(p2.tags)}")

    // assignment to a disjoint field
    let p3 = Mut Person { name: "eve", tags: mut_list_of("x") }
    let n3 = p3.name
    p3.tags = mut_list_of("q", "r")
    println("E ${n3} ${size(p3.tags)}")
}
"#;

const FIELD_DISJOINT_OUTPUT: &str = "A ann 2\nC 1 2\nD dee 2\nE eve 2\n";

#[test]
fn rustc_compiles_and_runs_field_disjoint_access() {
    let files = generate(&[("main.sv", FIELD_DISJOINT_DEMO)]);
    // The borrow is real and lives across the disjoint mutation: that is the
    // shape rustc has to accept for L5 to be clone-free here.
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        main.contains("let mut n = &p.name;") && main.contains("p.tags.push("),
        "expected a disjoint-field borrow held across the mutation in:\n{main}"
    );
    run_rust_files(&files, "field-disjoint", FIELD_DISJOINT_OUTPUT);
}

/// [fate-partial-move] L5's move half end to end: a field handed to a
/// consuming fn is a **real partial move** in the emitted Rust
/// (`eat(p.tags);` then `p.name.clone()`), and reassigning the moved field
/// is rustc's reinitialization — both accepted by borrowck. These programs
/// were checker-rejected before the move half landed.
///
/// Shared with the Kotlin backend, byte for byte.
const PARTIAL_MOVE_DEMO: &str = r#"
struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

fn eat(list: Mut List<Str>) -> None => !list {
    add(list, "eaten")
    return None
}

fn main() [use] {
    use StdOutConsole()
    // hand one field away, keep reading the other
    let p = Person { name: "ann", tags: mut_list_of("x") }
    eat(p.tags)
    println("1 ${p.name}")

    // the same through a move-mode binding
    let q = Person { name: "bob", tags: mut_list_of("y") }
    let t = q.tags
    add(t, "z")
    println("2 ${q.name} ${size(t)}")

    // put the field back, and the whole value works again
    let u = Mut Person { name: "eve", tags: mut_list_of("t") }
    eat(u.tags)
    u.tags = mut_list_of("new", "pair")
    println("3 ${u.name} ${size(u.tags)}")
}
"#;

const PARTIAL_MOVE_OUTPUT: &str = "1 ann\n2 bob 2\n3 eve 2\n";

#[test]
fn rustc_compiles_and_runs_a_partial_move() {
    let files = generate(&[("main.sv", PARTIAL_MOVE_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // The field moves out as a raw place (no clone), and the sibling is
    // still read afterwards: a genuine partial move.
    assert!(
        main.contains("eat(p.tags);") && main.contains("p.name.clone()"),
        "expected a partial move with a live sibling read in:\n{main}"
    );
    run_rust_files(&files, "partial-move", PARTIAL_MOVE_OUTPUT);
}

/// [inc-dec] All four step forms, in both statement and value position: the
/// fixity decides the *value* (postfix the old, prefix the new) and the
/// operator the direction. Rust has neither operator, so a value-position
/// step becomes a block; the stdout is what pins the two backends together.
///
/// Shared with the other backend, byte for byte.
const INC_DEC_DEMO: &str = r#"
fn main() [use] {
    use StdOutConsole()
    let i = 5
    i++
    i--
    ++i
    --i
    println("statements: ${i}")

    let a = 10
    let post = a++
    println("post: ${post} ${a}")
    let b = 10
    let pre = ++b
    println("pre: ${pre} ${b}")
    let c = 10
    let cd = c--
    println("post-dec: ${cd} ${c}")
    let d = 10
    let dd = --d
    println("pre-dec: ${dd} ${d}")
}
"#;

const INC_DEC_OUTPUT: &str =
    "statements: 5\npost: 10 11\npre: 11 11\npost-dec: 10 9\npre-dec: 9 9\n";

#[test]
fn rustc_compiles_and_runs_inc_dec() {
    let files = generate(&[("main.sv", INC_DEC_DEMO)]);
    run_rust_files(&files, "inc-dec", INC_DEC_OUTPUT);
}

/// [rs-opt-borrow] A local bound from a derived-return call with an optional
/// result holds `Option<&T>`. Narrowing it and using the value where an owned
/// `T` is expected used to emit `.as_ref().unwrap().clone()` — a clone of the
/// *reference* (`&&T` → `&T`), so rustc rejected checker-clean code. The
/// unwrap now yields the reference and clones through it.
const OPT_BORROW_DEMO: &str = r#"
struct Person { name: Str }

fn take(p: Person) -> Str => !p { return p.name }

fn main() [use] {
    use StdOutConsole()
    // Annotated to a plain list: the element constructor claims `NonEmpty`
    // [col-of-nonempty], and on a non-empty list `first` answers an element
    // rather than the optional this case is about.
    let people: List<Person> = list_of(Person { name: "ann" }, Person { name: "bob" })
    let head = first(people)
    if head is None {
        return
    }
    println("${take(copy(head))}")
    println("${head.name}")
}
"#;

#[test]
fn rustc_compiles_and_runs_an_optional_borrow_unwrap() {
    let files = generate(&[("main.sv", OPT_BORROW_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        main.contains("head.unwrap().clone()") && !main.contains("head.as_ref().unwrap()"),
        "expected the reference unwrap, not a clone of the reference, in:\n{main}"
    );
    run_rust_files(&files, "opt-borrow", "ann\nann\n");
}

/// [iter-fn] [proj-field] [yield-proj] An `iter fn` over a *generic* subject
/// that emits borrowed elements. The generated pass **borrows** its subject
/// (`__subject: proj Box<T>`, user decision 2026-09-11) instead of copying it,
/// so nothing has to `copy` a value of type `T` — which is what used to refuse
/// generic subjects on the Kotlin backend [kt-copy]. `proj(b)` in the
/// written return names the subject; the desugar redirects it to the pass.
const GENERIC_ITER_FN_DEMO: &str = r#"
struct Box<T> {
    items: List<T>
}

iter fn next<T>(b: Box<T>) -> Emitted (proj(b) T) | Finished {
    state {
        at: Int = 0
    }
    let e = get(b.items, at)
    if e is None {
        return finished()
    }
    at = at + 1
    return emitted(e)
}

fn main() [use] {
    use StdOutConsole()
    let b = Box<Str> { items: list_of("a", "b") }
    for s in iter(b) {
        println(s)
    }
    let n = Box<Int> { items: list_of(1, 2, 3) }
    println("${reduce(iter(n), 0, (acc, x) -> acc + x)}")
}
"#;

const GENERIC_ITER_FN_OUTPUT: &str = "a\nb\n6\n";

#[test]
fn rustc_compiles_and_runs_a_generic_subject_iter_fn() {
    let files = generate(&[("main.sv", GENERIC_ITER_FN_DEMO)]);
    run_rust_files(&files, "generic-iter-fn", GENERIC_ITER_FN_OUTPUT);
}

/// [deduce-syntax] [proj-anywhere] The `=>` clause's projection forms end to
/// end: a wholesale `proj(a, b)` joined across branches, and a
/// re-pointing entry `v.items: proj(other)` that makes a view borrow a
/// different list — Rust ties the struct's lifetime to the new source.
const DEDUCTION_CLAUSE_DEMO: &str = r#"
struct View canbe Mut {
    items: proj List<Int>,
    at: Int
}

fn view(items: List<Int>) -> Mut View {
    return Mut View { items: items, at: 0 }
}

fn repoint(v: Mut View, other: List<Int>) -> None => v: Mut, v.items: proj(other) {
    v.items = other
    v.at = 0
}

fn either(a: List<Int>, b: List<Int>, flag: Bool) -> proj(a, b) List<Int> {
    if flag {
        return a
    }
    return b
}

fn main() [use] {
    use StdOutConsole()
    let a = list_of(1, 2)
    let b = list_of(3, 4, 5)
    println("${size(either(a, b, true))} ${size(either(a, b, false))}")
    let v = view(a)
    repoint(v, b)
    println("${size(v.items)} at ${v.at}")
}
"#;

const DEDUCTION_CLAUSE_OUTPUT: &str = "2 3\n3 at 0\n";

#[test]
fn rustc_compiles_and_runs_the_deduction_clause_projections() {
    let files = generate(&[("main.sv", DEDUCTION_CLAUSE_DEMO)]);
    run_rust_files(&files, "deduction-clause", DEDUCTION_CLAUSE_OUTPUT);
}

// ===== [proj-type] a borrowed union arm into a `proj`-typed parameter =====

/// The phase-2b cut, closed 2026-09-12: a pass's `next` returns
/// `Emitted (proj Str) | Finished` — a union whose payload arm *borrows* —
/// and a fn takes that union whole by writing the projection in its
/// parameter type. The same value is also read as a plain projection
/// (`get(...)!`) and copied out of it ([copy-fn]).
const PROJ_ARM_PARAM_DEMO: &str = r#"
fn show(step: Emitted (proj Str) | Finished) [Console] -> None {
    when step {
        is Emitted { println(step) }
        is Finished { println("done") }
    }
}

fn main() [use] {
    use StdOutConsole()
    let words = list_of("ann", "bo")
    let p = iter(words)
    show(next(p))
    show(next(p))
    show(next(p))
    let e = get(words, 0)!
    println(e)
    let owned: Str = copy(e)
    println(owned)
}
"#;

const PROJ_ARM_PARAM_OUTPUT: &str = "ann\nbo\ndone\nann\nann\n";

#[test]
fn rustc_compiles_and_runs_a_borrowed_union_arm_into_a_proj_parameter() {
    let files = generate(&[("main.sv", PROJ_ARM_PARAM_DEMO)]);
    run_rust_files(&files, "proj-arm-param", PROJ_ARM_PARAM_OUTPUT);
}

// ===== [lambda-view] a capture-rooted projection, clone-free =====

/// A callback picking elements out of a *captured* list — the projection
/// is rooted in the capture, which no fn type can name, so the closure
/// itself carries the link [lambda-view]. The emitted Rust must hold the
/// borrow (`Vec<&String>`, no clone in the lambda) and deref the retagged
/// index in the cast (`(*i) as usize` — E0606 without it; both were open
/// defects, closed 2026-09-12).
const PICK_LIST_DEMO: &str = r#"
fn main() [use] {
    use StdOutConsole()
    let all = list_of("a", "b", "c", "d")
    let indices = list_of(1, 3)
    let picked = map(indices, i -> get(all, i)!)
    for w in picked {
        println(w)
    }
    let p = iter(indices)
    let doubled = map(p, i -> i + i)
    for n in doubled {
        println("${n}")
    }
}
"#;

const PICK_LIST_OUTPUT: &str = "b\nd\n2\n6\n";

#[test]
fn rustc_compiles_and_runs_a_capture_rooted_projection() {
    let files = generate(&[("main.sv", PICK_LIST_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // The lambda returns the borrow itself: no clone, and the Copy index
    // is deref'd for the cast — which goes through `i64`, so a negative
    // literal index stays an *answer* rather than rustc's E0600 [col-bounds].
    assert!(
        main.contains("|i| all.get((*i) as i64 as usize).expect(\"salvo: value is absent"),
        "expected a clone-free, deref'd pick lambda:\n{main}"
    );
    run_rust_files(&files, "pick-list", PICK_LIST_OUTPUT);
}

// ===== [linear-union-arm] the fallible-open shape, end to end =====

/// O-C2 (user decision 2026-09-12): a written linear union arm. The `Ok`
/// path closes the handle, the `Err` path owes nothing — and the emitted
/// code is ordinary union lowering, because linearity is static and
/// backend-identical [linear-static].
const FALLIBLE_OPEN_DEMO: &str = r#"
linear struct InputStream {
    path: Str
}

fn close(s: InputStream) [Console] -> None => !s {
    println("closed ${s.path}")
    discard(s)
}

fn open(path: Str) -> Ok InputStream | Err Str {
    if size(path) > 0 {
        return ok(InputStream {path: path})
    }
    return err("empty path")
}

fn attempt(path: Str) [Console] -> None {
    let r = open(path)
    when r {
        is Ok {
            println("opened")
            close(r)
        }
        is Err {
            println("error: ${r}")
        }
    }
}

fn main() [use] {
    use StdOutConsole()
    attempt("a.txt")
    attempt("")
}
"#;

const FALLIBLE_OPEN_OUTPUT: &str = "opened\nclosed a.txt\nerror: empty path\n";

#[test]
fn rustc_compiles_and_runs_a_linear_union_arm() {
    let files = generate(&[("main.sv", FALLIBLE_OPEN_DEMO)]);
    run_rust_files(&files, "linear-union-arm", FALLIBLE_OPEN_OUTPUT);
}

// ===== [linear-group] [linear-composite] the wrapper pass, end to end =====

/// Acceptance shape 4 (user decision 2026-09-12): a lazy `take` over a
/// linear source — a `linear struct` wrapper holding the source as a
/// concrete linear field, driven in place, discharged through its own
/// same-file `close_take`, which closes the source. The uncomposable-pass
/// cut is closed.
const WRAPPER_PASS_DEMO: &str = r#"
linear struct Ticks : Yield<self, Int> canbe Mut {
    at: Int
}

fn open_lines(from: Int) -> Mut Ticks {
    return Mut Ticks { at: from }
}

fn next(l: Mut Ticks) -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    println("closed lines")
    discard(l)
}

linear struct Take : Yield<self, Int> canbe Mut {
    source: Mut Ticks,
    left: Int
}

fn take(source: Mut Ticks, n: Int) -> Mut Take => !source {
    return Mut Take { source: source, left: n }
}

fn next(t: Mut Take) -> Emitted Int | Finished => t: Mut {
    if t.left <= 0 {
        return finished()
    }
    t.left = t.left - 1
    return next(t.source)
}

fn close_take(t: Take) [Console] -> None => !t {
    close(t.source)
}

fn main() [use] {
    use StdOutConsole()
    let t = take(open_lines(5), 2)
    for n in t {
        println("n ${n}")
    }
    close_take(t)
    println("done")
}
"#;

const WRAPPER_PASS_OUTPUT: &str = "n 5\nn 4\nclosed lines\ndone\n";

#[test]
fn rustc_compiles_and_runs_a_linear_wrapper_pass() {
    let files = generate(&[("main.sv", WRAPPER_PASS_DEMO)]);
    run_rust_files(&files, "wrapper-pass", WRAPPER_PASS_OUTPUT);
}

// ===== [fn-value-select] [linear-discard] `drop` as the uniform callback =====

/// The consuming-callback pattern's two callers (user decision 2026-09-12):
/// a linear pass hands its own discharger (`close`), a plain pass hands
/// std's generic `drop` — instantiated *by name* from the position's fn
/// type, which is the generic-fn-value instantiation closed with phase 3.
const DROP_CALLBACK_DEMO: &str = r#"
linear struct Handle : Yield<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) -> None => !h { discard(h) }

struct Counter : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Counter) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    end(it)
    return sum
}

fn main() [use] {
    use StdOutConsole()
    let h = open_handle(3)
    println("linear ${drain(h, close)}")
    let c = Mut Counter { at: 3 }
    println("plain ${drain(c, drop)}")
}
"#;

const DROP_CALLBACK_OUTPUT: &str = "linear 6\nplain 6\n";

#[test]
fn rustc_compiles_and_runs_drop_as_a_consuming_callback() {
    let files = generate(&[("main.sv", DROP_CALLBACK_DEMO)]);
    run_rust_files(&files, "drop-callback", DROP_CALLBACK_OUTPUT);
}


// ===== collections =====

/// [col-insertion-order] [rs-collections] Set and Map, whose observable
/// behavior is the part that has to be identical on both backends: the
/// iteration order, a position-preserving overwrite, an order-preserving
/// removal that hands the value back, and last-wins duplicate keys. The
/// Kotlin backend runs the *same source* against the *same expected
/// output* (`kotlinc_compiles_and_runs_every_case`), which is what makes
/// this a parity test rather than two independent ones.
const COLLECTIONS_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()

    let s: Mut Set<Str> = mut_set_of("b", "a", "c")
    println("set: ${to_str(s)} size: ${size(s)}")
    let added_new = add(s, "d")
    let added_dup = add(s, "a")
    println("add new: ${added_new} dup: ${added_dup} now: ${to_str(s)}")
    let has_a = contains(s, "a")
    let gone = remove(s, "a")
    let gone_again = remove(s, "a")
    println("contains: ${has_a} remove: ${gone} again: ${gone_again} left: ${to_str(s)}")
    let dedup = set_of(1, 2, 1, 3)
    println("dedup: ${to_str(dedup)} size: ${size(dedup)}")

    let m: Mut Map<Str, Int> = mut_map_of(("one", 1), ("two", 2))
    put(m, "three", 3)
    println("map: ${to_str(m)}")
    put(m, "one", 111)
    println("overwrite keeps position: ${to_str(m)}")
    let two = get(m, "two")
    if two is Int {
        println("get: ${two}")
    }
    let absent = get(m, "nope")
    let is_absent = absent is None
    let taken = remove(m, "two")
    if taken is Int {
        println("absent: ${is_absent} removed: ${taken} left: ${to_str(m)}")
    }
    let has_one = contains_key(m, "one")
    println("contains_key: ${has_one} size: ${size(m)}")
    let dup_keys = map_of(("x", 1), ("y", 2), ("x", 9))
    println("dup keys: ${to_str(dup_keys)}")
    let ints: Mut Map<Int, Str> = mut_map_of()
    put(ints, 7, "seven")
    put(ints, 3, "three")
    println("int keys: ${to_str(ints)}")
}
"#;

/// The expected output is shared with the Kotlin backend's case of the same
/// name — a divergence in either is a [backend-parity] defect.
const COLLECTIONS_OUTPUT: &str = "set: {b, a, c} size: 3\n\
     add new: true dup: false now: {b, a, c, d}\n\
     contains: true remove: true again: false left: {b, c, d}\n\
     dedup: {1, 2, 3} size: 3\n\
     map: {one: 1, two: 2, three: 3}\n\
     overwrite keeps position: {one: 111, two: 2, three: 3}\n\
     get: 2\n\
     absent: true removed: 2 left: {one: 111, three: 3}\n\
     contains_key: true size: 2\n\
     dup keys: {x: 9, y: 2}\n\
     int keys: {7: seven, 3: three}\n";

#[test]
fn rustc_compiles_and_runs_collections() {
    let files = generate(&[("main.sv", COLLECTIONS_DEMO)]);
    run_rust_files(&files, "collections", COLLECTIONS_OUTPUT);
}

/// [rs-collections] The runtime module is emitted, mounted in the crate root
/// and imported by the module that uses it.
///
/// It is *not* asserted that a program without collections omits it, because
/// today it does not: module reachability is name-based and deliberately
/// conservative [mod-used-only], and `core.map` declares a `get` overload
/// that `core.list`'s own `next` body calls — so both collection modules,
/// and with them this runtime, are reachable from any program that touches
/// a list or a string. That is dead code (the crate allows it) rather than
/// wrong code, and the fix is a precision pass on reachability — using the
/// checker's *resolved* call targets for overloaded names instead of the
/// name alone. Recorded in ROADMAP.md.
#[test]
fn the_collections_runtime_is_emitted_mounted_and_imported() {
    let files = generate(&[("main.sv", COLLECTIONS_DEMO)]);
    assert!(
        files.iter().any(|f| f.rel_path.ends_with("collections.rs")),
        "a program using Set/Map should emit the runtime module"
    );
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .expect("main.rs");
    assert!(
        main.content.contains("mod collections;"),
        "the crate root should mount it: {}",
        main.content
    );
    assert!(
        main.content.contains("use crate::collections::*;"),
        "the using module should import it: {}",
        main.content
    );
    // The ordered types are what it exists for, and the emitted signatures
    // name them rather than Rust's unordered `HashMap`/`HashSet`
    // [col-insertion-order].
    let runtime = files
        .iter()
        .find(|f| f.rel_path.ends_with("collections.rs"))
        .expect("collections.rs");
    assert!(
        runtime.content.contains("pub struct SalvoMap<K, V>")
            && runtime.content.contains("pub struct SalvoSet<T>"),
        "the runtime should define the ordered types"
    );
    assert!(
        !main.content.contains("HashMap<") && !main.content.contains("HashSet<"),
        "emitted code should use the ordered types, not Rust's unordered ones: {}",
        main.content
    );
}

/// [iter-pass] Iterating a `Set` (its elements) and a `Map` (its keys, with
/// values reached through `get`), plus a combinator over each pass. Shares
/// its source and expected output with the Kotlin case of the same name —
/// the pair is the [backend-parity] test.
const COLLECTION_ITER_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()
    let s = set_of("b", "a", "c")
    for e in iter(s) {
        println("elem: ${e}")
    }
    let xs = to_list(s)
    println("as list: ${to_str(xs)}")
    let n: Int = reduce(iter(s), 0, (acc, e) -> acc + size(e))
    println("total length: ${n}")

    let m = map_of(("one", 1), ("two", 2), ("three", 3))
    for k in iter(m) {
        let v = get(m, k)
        if v is Int {
            println("${k} -> ${v}")
        }
    }
    println("keys: ${to_str(keys(m))}")
    let total: Int = reduce(iter(m), 0, (acc, k) -> acc + size(k))
    println("key length total: ${total}")

    let empty: Set<Int> = set_of()
    for e in iter(empty) {
        println("unreachable ${e}")
    }
    println("done")
}
"#;

const COLLECTION_ITER_OUTPUT: &str = "elem: b\n\
     elem: a\n\
     elem: c\n\
     as list: [b, a, c]\n\
     total length: 3\n\
     one -> 1\n\
     two -> 2\n\
     three -> 3\n\
     keys: [one, two, three]\n\
     key length total: 11\n\
     done\n";

#[test]
fn rustc_compiles_and_runs_collection_iteration() {
    let files = generate(&[("main.sv", COLLECTION_ITER_DEMO)]);
    run_rust_files(&files, "collection-iter", COLLECTION_ITER_OUTPUT);
}

// ===== collection literals [col-literal] =====

/// [col-literal] Every literal form end to end: a list from brackets, a set
/// and a map from braces, `Mut` adopted from the position, empty literals
/// typed by their position, the bare struct literal still winning on an
/// identifier key, and an array built by `array_of` now that brackets are a
/// list. Shares source and expected output with the Kotlin case of the same
/// name [backend-parity].
const COLLECTION_LIT_DEMO: &str = r#"
struct Point {
    x: Int,
    y: Int
}

fn takes_set(s: Set<Int>) -> Int => s {
    return size(s)
}

fn main() [use] -> None {
    use StdOutConsole()
    let xs = [1, 2, 3]
    println("list: ${to_str(xs)}")
    let mxs: Mut List<Int> = [4, 5]
    add(mxs, 6)
    println("mut list: ${to_str(mxs)}")
    let s: Set<Str> = {"b", "a", "b"}
    println("set: ${to_str(s)}")
    let m: Map<Str, Int> = {"one": 1, "two": 2}
    println("map: ${to_str(m)}")
    let ms: Mut Set<Int> = {}
    add(ms, 9)
    println("empty then filled: ${to_str(ms)}")
    let mm: Map<Str, Int> = {}
    println("empty map: ${to_str(mm)} via param: ${takes_set({})}")
    let p: Point = {x: 1, y: 2}
    println("struct literal still: ${p.x},${p.y}")
    let arr = array_of(7, 8)
    println("array: ${size(arr)} ${arr[0]}")
}
"#;

const COLLECTION_LIT_OUTPUT: &str = "list: [1, 2, 3]\n\
     mut list: [4, 5, 6]\n\
     set: {b, a}\n\
     map: {one: 1, two: 2}\n\
     empty then filled: {9}\n\
     empty map: {} via param: 0\n\
     struct literal still: 1,2\n\
     array: 2 7\n";

#[test]
fn rustc_compiles_and_runs_collection_literals() {
    let files = generate(&[("main.sv", COLLECTION_LIT_DEMO)]);
    run_rust_files(&files, "collection-literals", COLLECTION_LIT_OUTPUT);
}

// ===== equality, ordering and struct keys [col-equality] =====

/// [op-equality] [op-order] [cmp-auto] Equality and ordering as
/// **capabilities**: the structural implementations a `default` obligation
/// generates, a hashed struct as a set element and a map key, lexicographic
/// list/tuple order, and Salvo's own float equality.
///
/// The last line is the one this pair exists for: a Kotlin data class's
/// generated `equals` calls `Double.equals`, for which `NaN` equals itself,
/// while Rust's derive is IEEE. The same program printed `true` on one
/// backend and `false` on the other until the Kotlin emitter began writing
/// its own `equals` [kt-float-eq]. Shares source and expected output with the
/// Kotlin case of the same name [backend-parity].
const EQUALITY_DEMO: &str = r#"
struct Point : auto Ordered<self>, auto Hashed<self> {
    x: Int,
    y: Int
}

struct Version : auto Ordered<self>, auto Hashed<self> {
    parts: List<Int>,
    label: (Str, Int)
}

// Equality only — and a float field, which is fine for `eq` and not for a key.
struct Measure : auto Eq<self> {
    value: Double
}

fn main() [use] -> None {
    use StdOutConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 3 }
    let r = Point { x: 1, y: 2 }
    let eq = p == r
    let ne = p != q
    let lt = p < q
    let ge = q >= p
    println("eq: ${eq} ne: ${ne} lt: ${lt} ge: ${ge}")

    let seen: Mut Set<Point> = {}
    add(seen, Point { x: 1, y: 2 })
    let dup = add(seen, Point { x: 1, y: 2 })
    println("struct key: size ${size(seen)} dup ${dup}")

    let by: Mut Map<Point, Str> = {}
    put(by, Point { x: 9, y: 9 }, "nine")
    let hit = get(by, Point { x: 9, y: 9 })
    if hit is Str {
        println("struct lookup: ${hit}")
    }

    let a = Version { parts: [1, 2, 0], label: ("a", 1) }
    let b = Version { parts: [1, 3], label: ("a", 1) }
    let c = Version { parts: [1, 2], label: ("a", 1) }
    let ab = a < b
    let ca = c < a
    println("list order: ${ab} prefix: ${ca}")

    let zero = 0.0
    let m1 = Measure { value: zero / zero }
    let m2 = Measure { value: zero / zero }
    let nan_eq = m1 == m2
    println("struct nan equality: ${nan_eq}")
}
"#;

const EQUALITY_OUTPUT: &str = "eq: true ne: true lt: true ge: true\n\
     struct key: size 1 dup false\n\
     struct lookup: nine\n\
     list order: true prefix: true\n\
     struct nan equality: false\n";

#[test]
fn rustc_compiles_and_runs_equality_and_ordering() {
    let files = generate(&[("main.sv", EQUALITY_DEMO)]);
    run_rust_files(&files, "equality-ordering", EQUALITY_OUTPUT);
}

// ===== sorted collections [col-sorted] =====

/// [col-sorted] `SortedSet`/`SortedMap` end to end: key order rather than
/// insertion order, `min`/`max`, `first_key`/`last_key`, and in-order
/// iteration. Rust uses `BTreeSet`/`BTreeMap`, Kotlin a `TreeSet`/`TreeMap`
/// built with Salvo's own comparator — shares source and expected output with
/// the Kotlin case of the same name [backend-parity].
const SORTED_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()
    let s: Mut SortedSet<Str> = mut_sorted_set_of("pear", "apple", "fig")
    println("set: ${to_str(s)}")
    let added = add(s, "banana")
    let dup = add(s, "apple")
    println("add ${added} dup ${dup} now ${to_str(s)}")
    // [col-nonempty] `add` established `NonEmpty`, so `min`/`max` resolve to
    // the overloads that answer with an element rather than an optional —
    // no test needed here any more.
    let lo = min(s)
    let hi = max(s)
    println("min ${lo} max ${hi}")
    let had = remove(s, "fig")
    println("removed ${had} left ${to_str(s)} size ${size(s)}")
    let ns = sorted_set_of(10, 2, 33, 4)
    println("ints ${to_str(ns)} list ${to_str(to_list(ns))}")

    let m: Mut SortedMap<Str, Int> = mut_sorted_map_of(("two", 2), ("one", 1))
    put(m, "three", 3)
    println("map ${to_str(m)}")
    let v = get(m, "two")
    if v is Int {
        println("get ${v}")
    }
    // Likewise: `put` established the claim on the map.
    let fk = first_key(m)
    let lk = last_key(m)
    println("first ${fk} last ${lk}")
    let taken = remove(m, "one")
    if taken is Int {
        println("took ${taken} left ${to_str(m)}")
    }
    let has3 = contains_key(m, "three")
    println("keys ${to_str(keys(m))} has ${has3}")
    for e in iter(ns) {
        println("e ${e}")
    }
    for k in iter(m) {
        println("k ${k}")
    }
}
"#;

const SORTED_OUTPUT: &str = "set: {apple, fig, pear}\n\
     add true dup false now {apple, banana, fig, pear}\n\
     min apple max pear\n\
     removed true left {apple, banana, pear} size 3\n\
     ints {2, 4, 10, 33} list [2, 4, 10, 33]\n\
     map {one: 1, three: 3, two: 2}\n\
     get 2\n\
     first one last two\n\
     took 1 left {three: 3, two: 2}\n\
     keys [three, two] has true\n\
     e 2\n\
     e 4\n\
     e 10\n\
     e 33\n\
     k three\n\
     k two\n";

#[test]
fn rustc_compiles_and_runs_sorted_collections() {
    let files = generate(&[("main.sv", SORTED_DEMO)]);
    run_rust_files(&files, "sorted-collections", SORTED_OUTPUT);
}

/// [col-sorted] [kt-ordered] Strings order by **code point**, not by UTF-16
/// code unit — the one case where the JVM's natural `String.compareTo`
/// disagrees with Rust's byte-wise `Ord`.
///
/// U+1F600 is astral (a surrogate pair starting 0xD83D) and U+FF21 is a BMP
/// character at 0xFF21: code-unit order puts the surrogate *first*, code-point
/// order puts U+FF21 first. Rust is code-point order, so Kotlin's sorted
/// collections are built with a comparator that compares code points
/// [backend-parity].
const CODEPOINT_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()
    let bmp = "Ａ"
    let s = sorted_set_of("😀", "Ａ")
    let first = min(s)
    if first is Str {
        let is_bmp = first == bmp
        println("smallest is the BMP char: ${is_bmp}")
    }
}
"#;

#[test]
fn rustc_compiles_and_runs_codepoint_string_order() {
    let files = generate(&[("main.sv", CODEPOINT_DEMO)]);
    run_rust_files(
        &files,
        "codepoint-order",
        "smallest is the BMP char: true\n",
    );
}

// ===== generated constructors and converters [col-by] [col-convert] =====

/// [col-by] [col-convert] The `*_by` constructors (a size and a rule per
/// index), both `to_map` forms (a list of pairs, and a list plus a rule),
/// `to_set`'s dedup, and array element assignment. Shares source and expected
/// output with the Kotlin case of the same name [backend-parity].
const BY_AND_CONVERT_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()
    let a = array_by(3, i -> i * 2)
    println("array_by ${a[0]} ${a[1]} ${a[2]}")
    let xs = list_by(4, i -> i + 10)
    println("list_by ${to_str(xs)}")
    let s = set_by(5, i -> i % 3)
    println("set_by ${to_str(s)}")
    let m = map_by(3, i -> (i, i * i))
    println("map_by ${to_str(m)}")
    let dups = [1, 2, 2, 3]
    let uniq = to_set(dups)
    println("to_set ${to_str(uniq)}")
    let pairs = [("a", 1), ("b", 2)]
    let m2 = to_map(pairs)
    println("to_map pairs ${to_str(m2)}")
    let words = ["alpha", "be"]
    let m3 = to_map(words, w -> (w, size(w)))
    println("to_map rule ${to_str(m3)}")
    let arr = array_of(1, 2, 3)
    arr[0] = 9
    println("assigned ${arr[0]} size ${size(arr)}")
}
"#;

const BY_AND_CONVERT_OUTPUT: &str = "array_by 0 2 4\n\
     list_by [10, 11, 12, 13]\n\
     set_by {0, 1, 2}\n\
     map_by {0: 0, 1: 1, 2: 4}\n\
     to_set {1, 2, 3}\n\
     to_map pairs {a: 1, b: 2}\n\
     to_map rule {alpha: 5, be: 2}\n\
     assigned 9 size 3\n";

#[test]
fn rustc_compiles_and_runs_generated_constructors() {
    let files = generate(&[("main.sv", BY_AND_CONVERT_DEMO)]);
    run_rust_files(&files, "by-and-convert", BY_AND_CONVERT_OUTPUT);
}


// ===== C-6 the claims a list can carry [col-nonempty] [col-sorted-list]
// [col-noteq] =====

/// Shared verbatim with the Kotlin backend's `kotlinc_compiles_and_runs_list_claims`
/// — same source, same expected stdout. The parity is the point: the whole
/// surface is std's own, and `sort`'s ordering and `binary_search`'s answer
/// within an equal run both have to be the language's rather than the
/// target's.
pub const LIST_CLAIMS_DEMO: &str = r#"
// A position that demands distinctness needs no duplicate check of its own.
fn count_unique(xs: Distinct List<Int>) [] -> Int => xs {
    return size(xs)
}

// [cmp-carry] An ordering of the program's own, which is nothing like the
// code-point order `cmp(Str, Str)` gives: what `sort` publishes into the
// `Sorted` claim is *this* one, so the search and the insert below use it too.
fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn main() [use] {
    use StdOutConsole()

    // NonEmpty by construction: the element constructor requires a first, so
    // `first` answers with an element, not `Int?` [col-of-nonempty].
    let built = list_of(10, 20)
    println("built ${first(built)}")

    // NonEmpty by refinement: `add` establishes the claim on the qualifier's
    // behalf, so `first` resolves to the same overload.
    let grown: Mut List<Int> = mut_list_of()
    add(grown, 7)
    println("grown ${first(grown)}")

    // NonEmpty by test.
    let maybe = list_of(1, 2)
    if maybe is NonEmpty {
        println("tested ${first(maybe)}")
    }

    // Sorted, minted by `sort`; equal elements answer the lowest index.
    let ordered = sort(list_of(5, 1, 4, 1, 3))
    println("sorted ${to_str(ordered)}")
    if binary_search(ordered, 1) is Int at {
        println("lowest 1 at ${at}")
    }
    if binary_search(ordered, 9) is None {
        println("9 absent")
    }

    // add_sorted keeps the claim across the mutation.
    let live: Mut Sorted List<Int> = mut_sort(list_of(10, 40))
    add_sorted(live, 20)
    add_sorted(live, 5)
    println("still sorted ${to_str(live)}")

    // Strings order by code point on both backends.
    let words = sort(list_of("pear", "Apple", "fig"))
    println("words ${to_str(words)}")

    // [cmp-carry] …unless the claim names another ordering, which is what the
    // slot is for: `by_len` sorts, and `binary_search` and `add_sorted` are
    // computed with it because the list's *type* carries it. Under the
    // code-point order none of these three lines would come out this way.
    let bylen = sort(list_of("pear", "fig", "Apple"), cmp = by_len)
    println("by length ${to_str(bylen)}")
    if binary_search(bylen, "kiwi") is Int at {
        println("a 4-letter word at ${at}")
    }
    let growing: Mut Sorted List<Str> = mut_sort(list_of("pear", "fig", "Apple"), cmp = by_len)
    add_sorted(growing, "durian")
    println("still by length ${to_str(growing)}")

    // Distinct, minted by a set.
    let unique = to_list(set_of(3, 1, 3, 2))
    println("distinct ${to_str(unique)} of ${count_unique(unique)}")

    // [qual-overload] The same claim over four more subjects: `NonEmpty` is
    // one name whose meaning the subject decides.
    let seen: Mut Set<Str> = mut_set_of()
    add(seen, "a")
    if seen is NonEmpty {
        println("set claim ${size(seen)}")
    }
    let tally: Mut Map<Str, Int> = mut_map_of()
    put(tally, "k", 1)
    if tally is NonEmpty {
        println("map claim ${size(tally)}")
    }
    let ranked = sorted_set_of(30, 10, 20)
    if ranked is NonEmpty {
        let lo = min(ranked)
        let hi = max(ranked)
        println("sorted claim ${lo}..${hi}")
    }
    let bykey = sorted_map_of(("b", 2), ("a", 1))
    if bykey is NonEmpty {
        let fk = first_key(bykey)
        println("sorted map claim ${fk}")
    }
}
"#;

pub const LIST_CLAIMS_OUTPUT: &str = "built 10\n\
     grown 7\n\
     tested 1\n\
     sorted [1, 1, 3, 4, 5]\n\
     lowest 1 at 0\n\
     9 absent\n\
     still sorted [5, 10, 20, 40]\n\
     words [Apple, fig, pear]\n\
     by length [fig, pear, Apple]\n\
     a 4-letter word at 1\n\
     still by length [fig, pear, Apple, durian]\n\
     distinct [3, 1, 2] of 3\n\
     set claim 1\n\
     map claim 1\n\
     sorted claim 10..30\n\
     sorted map claim a\n";

#[test]
fn rustc_compiles_and_runs_list_claims() {
    let files = generate(&[("main.sv", LIST_CLAIMS_DEMO)]);
    run_rust_files(&files, "list-claims", LIST_CLAIMS_OUTPUT);
}

// ===== [col-bounds] every index answers rather than failing =====

/// Shared verbatim with the Kotlin backend's `kotlinc_compiles_and_runs_bounds`
/// (bar `export` on `main`) — same source, same expected stdout. The parity is
/// the assertion twice over: `swap` reports out-of-range as `false` where
/// `Vec::swap` panics and a JVM list throws, and a **literal** negative index
/// answers rather than failing to build, which it did not until 2026-09-22
/// (`(-1) as usize` is rustc's E0600 — a [backend-never-wrong] violation the
/// Kotlin side never had).
pub const BOUNDS_DEMO: &str = r#"
fn main() [use] {
    use StdOutConsole()

    // [col-bounds] A total exchange, and the one positional write a list of
    // obligations can have: nothing enters, nothing leaves, nothing is dropped.
    let xs: Mut List<Str> = mut_list_of("a", "b", "c")
    println("swapped ${swap(xs, 0, 2)} ${to_str(xs)}")
    println("in place ${swap(xs, 1, 1)} ${to_str(xs)}")
    println("past the end ${swap(xs, 0, 3)} ${to_str(xs)}")
    println("negative ${swap(xs, -1, 0)} ${to_str(xs)}")

    // Reads answer an optional, at either end, for a literal as for a variable.
    if get(xs, -1) is None {
        println("read before the start is absent")
    }
    if get(xs, 9) is None {
        println("read past the end is absent")
    }
    if char_at("hi", -1) is None {
        println("a character before the start is absent")
    }

    // A positional *write* answers whether it wrote, which is the same report
    // for the same reason: a write that quietly did nothing has no symptom at
    // the call. Out of range the value is the caller's and stays untouched.
    let text = mut_str("hello")
    println("wrote ${set(text, 0, 'H')} ${text}")
    println("write past the end ${set(text, 9, 'X')} ${text}")
    let buf = mut_bytes(bytes_of(to_byte(1), to_byte(2)))
    println("wrote a byte ${set(buf, 1, to_byte(9))} ${to_hex(buf)}")
    println("byte past the end ${set(buf, 2, to_byte(9))} ${to_hex(buf)}")

    // The heap's own move, in the spelling it uses [fn-dot].
    let heap: Mut List<Int> = mut_list_of(5, 9, 7)
    heap.swap(0, 2)
    println("sifted ${to_str(heap)}")
}
"#;

pub const BOUNDS_OUTPUT: &str = "swapped true [c, b, a]\n\
     in place true [c, b, a]\n\
     past the end false [c, b, a]\n\
     negative false [c, b, a]\n\
     read before the start is absent\n\
     read past the end is absent\n\
     a character before the start is absent\n\
     wrote true Hello\n\
     write past the end false Hello\n\
     wrote a byte true 0109\n\
     byte past the end false 0109\n\
     sifted [7, 9, 5]\n";

#[test]
fn rustc_compiles_and_runs_bounds() {
    let files = generate(&[("main.sv", BOUNDS_DEMO)]);
    run_rust_files(&files, "bounds", BOUNDS_OUTPUT);
}

// ===== [deduce-reapply] [cmp-carry] a binary heap in user space =====

/// Shared verbatim with the Kotlin backend's `kotlinc_compiles_and_runs_a_heap`
/// (bar `export` on `main`). **The exercise the whole qualifier round was for**:
/// a heap written outside std — a claim on an ordinary list, carrying the
/// ordering it is kept by, mutated through `Mut` parameters that *re-establish*
/// the claim. Every piece is a rule this repository decided: `Heap<T>(?cmp)`
/// [cmp-carry], the `?cmp` binder [cmp-binder], `+Heap<T>(?cmp)`
/// [deduce-reapply], `swap` [col-bounds], and the negated guard that routes the
/// bare `heap_pop` to the `NonEmpty` overload [is-narrow-guard].
///
/// Two heaps, one ordered canonically and one by a function of the program's
/// own, drained by the *same* generic function — which is the point of the
/// ordering living in the type.
pub const HEAP_DEMO: &str = r#"
// [cmp-carry] A binary heap in **user space**: a claim on an ordinary list,
// carrying the ordering it is kept by, with mutators that keep the claim.
export qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T> with NonEmpty

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> +Heap<T>(?cmp) Mut List<T> {
    return mut_list_of()
}

// [deduce-reapply] **Establishing** the claim rather than keeping one: `list`
// arrives an ordinary list and leaves a `Heap<T>(?cmp)`, which a function in the
// qualifier's own file may say about a parameter. The ordering it establishes the
// claim *under* is the one the call resolved, so the caller's heap is ordered by
// what it asked for.
export fn heapify<T>(list: Mut List<T>, ?Ordered<T>) [] -> None
=> list: +Heap<T>(?cmp) Mut {
    let i = size(list) / 2
    while i > 0 {
        i -= 1
        let at = copy(i)
        while at < size(list) {
            let child = at * 2 + 1
            if child >= size(list) {
                break
            }
            let smaller = copy(child)
            if child + 1 < size(list) && cmp(list.get(child + 1)!, list.get(child)!) < 0 {
                smaller += 1
            }
            if cmp(list.get(at)!, list.get(smaller)!) <= 0 {
                break
            }
            list.swap(at, smaller)
            at = copy(smaller)
        }
    }
}

// [deduce-reapply] `add` strips the claim — a mutating callee must — and this
// function is the one that knows the sift puts it back.
export fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) [] -> None
=> heap: +Heap<T>(?cmp) Mut, !elem {
    add(heap, elem)
    let i = size(heap) - 1
    while i > 0 {
        let parent = (i - 1) / 2
        if cmp(heap.get(parent)!, heap.get(i)!) <= 0 {
            break
        }
        heap.swap(i, parent)
        i = copy(parent)
    }
}

export fn heap_pop<T>(heap: Heap<T>(?cmp) Mut List<T>) [] -> T?
=> heap: +Heap<T>(?cmp) Mut {
    if heap !is NonEmpty {
        return None
    }
    return heap_pop(heap)
}

export fn heap_pop<T>(heap: NonEmpty Heap<T>(?cmp) Mut List<T>) [] -> T
=> heap: +Heap<T>(?cmp) Mut {
    let last = size(heap) - 1
    heap.swap(0, last)
    let least = heap.remove_at(last)!
    let i = 0
    while i < size(heap) {
        let child = i * 2 + 1
        if child >= size(heap) {
            break
        }
        let smaller = copy(child)
        if child + 1 < size(heap) && cmp(heap.get(child + 1)!, heap.get(child)!) < 0 {
            smaller += 1
        }
        if cmp(heap.get(i)!, heap.get(smaller)!) <= 0 {
            break
        }
        heap.swap(i, smaller)
        i = copy(smaller)
    }
    return least
}

fn by_last_digit(a: Int, b: Int) [] -> Int => a, b {
    return cmp(a % 10, b % 10)
}

// The binder again, on a concrete element type: this drains *either* heap
// below, each in the ordering its own type carries [cmp-binder].
fn drain_heap(heap: Heap<Int>(?cmp) Mut List<Int>) [Console] -> None
=> heap: +Heap<Int>(?cmp) Mut {
    let out = mut_str()
    while heap_pop(heap) is Int n {
        append(out, "${n} ")
    }
    println("${out}")
}

fn main() [use] {
    use StdOutConsole()

    let h = empty_heap<Int>()
    heap_push(h, 5)
    heap_push(h, 3)
    heap_push(h, 9)
    heap_push(h, 1)
    heap_push(h, 7)
    println("root ${first(h)!}")
    drain_heap(h)

    // A list nobody claimed anything about, made a heap by a function that
    // establishes the claim [deduce-reapply] — and then read by the same
    // surface, which only accepts a heap.
    let raw: Mut List<Int> = mut_list_of(8, 2, 6, 4)
    heapify(raw)
    heap_push(raw, 3)
    drain_heap(raw)

    // A second heap under an ordering of the program's own: the claim carries
    // it, so the same functions pop in *that* order.
    let byDigit = empty_heap<Int>(cmp = by_last_digit)
    heap_push(byDigit, 25)
    heap_push(byDigit, 13)
    heap_push(byDigit, 41)
    drain_heap(byDigit)
}
"#;

pub const HEAP_OUTPUT: &str = "root 1\n\
     1 3 5 7 9 \n\
     2 3 4 6 8 \n\
     41 13 25 \n";

#[test]
fn rustc_compiles_and_runs_a_heap() {
    let files = generate(&[("main.sv", HEAP_DEMO)]);
    run_rust_files(&files, "heap", HEAP_OUTPUT);
}


// ===== [fn-variadic] mixing plain arguments with a `...spread` =====

/// The shape: a mixed tail is **assembled** into one vector in written order,
/// and — because that vector is fresh — it is *not* cloned again by the
/// constructor, which a borrowed forward would be (`parts.clone()`, asserted
/// in `a_spread_into_a_variadic_intrinsic_is_the_collection`).
#[test]
fn a_mixed_variadic_tail_is_assembled_once() {
    let src = r#"
export fn main() [use] {
    use StdOutConsole()
    let rest = array_of(2, 3)
    let all = list_of(1, ...rest)
    println("${size(all)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [col-of-nonempty] The list constructors take their leading elements as
    // real parameters now, so the lowering starts from `vec![…]` and extends
    // with the tail rather than pushing each leading element.
    assert!(
        main.contains("let mut __v = vec![1];")
            && main.contains("__v.extend(rest.iter().cloned());"),
        "the tail should be assembled in order:\n{main}"
    );
    assert!(
        !main.contains("__v }.clone()"),
        "an assembled vector is already owned and must not be cloned again:\n{main}"
    );
}

/// Shared verbatim with the Kotlin backend's
/// `kotlinc_compiles_and_runs_mixed_spread`. Kotlin has a native spread
/// operator, so mixed calls always worked there; this asserts the *Rust*
/// side now agrees, across every variadic shape std has — the collection
/// constructors (which **store** the tail), `mut_str` (which only **reads**
/// it, so a lone spread stays a borrow), and an ordinary user-declared
/// variadic, which takes a different code path from the intrinsics.
pub const MIXED_SPREAD_DEMO: &str = r#"
fn join_all(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let out = mut_str(...parts)
    println("${sep} ${out}")
}

fn main() [use] {
    use StdOutConsole()
    let rest = array_of(2, 3)
    let all = list_of(1, ...rest)
    println("list_of ${to_str(all)}")

    // The spread source is still usable: a variadic position is not tracked,
    // so the store clones rather than moves.
    println("reusable ${size(rest)}")

    let s = set_of(9, ...rest)
    println("set_of ${to_str(s)}")
    let sorted = sorted_set_of(9, ...rest)
    println("sorted_set_of ${to_str(sorted)}")

    let pairs = array_of(("b", 2))
    let m = map_of(("a", 1), ...pairs)
    println("map_of ${to_str(m)}")

    let words = array_of("b", "c")
    let joined = mut_str("a", ...words)
    println("mut_str ${joined}")
    join_all("ordinary", "x", ...words)

    // A lone spread still forwards the whole collection — into a purely
    // variadic constructor. `list_of` is not one any more [col-of-nonempty]:
    // its element shape takes a *first*, which a spread may not supply.
    let lone = set_of(...rest)
    println("lone ${to_str(lone)}")
}
"#;

pub const MIXED_SPREAD_OUTPUT: &str = "list_of [1, 2, 3]\n\
     reusable 2\n\
     set_of {9, 2, 3}\n\
     sorted_set_of {2, 3, 9}\n\
     map_of {a: 1, b: 2}\n\
     mut_str abc\n\
     ordinary xbc\n\
     lone {2, 3}\n";

#[test]
fn rustc_compiles_and_runs_mixed_spread() {
    let files = generate(&[("main.sv", MIXED_SPREAD_DEMO)]);
    run_rust_files(&files, "mixed-spread", MIXED_SPREAD_OUTPUT);
}

// ===== [kt-variadic] a user-declared variadic, every element type =====

/// Shared verbatim with the Kotlin backend's
/// `kotlinc_compiles_and_runs_user_variadics`. The Kotlin side is the one this
/// exists for: a `vararg` of a *primitive* element type is an `IntArray`, not
/// an `Array<Int>`, and those are unrelated on the JVM — so the parameter is
/// emitted as a plain array instead. Rust is here to hold the parity.
pub const USER_VARIADIC_DEMO: &str = r#"
fn count_ints(label: Str, ...ns: Int[]) [Console] -> Int => label, ns {
    let seen = mut_list_of(0)
    for n in iter(ns) {
        add(seen, copy(n))
    }
    println("${label} ${size(ns)}")
    return size(seen)
}

fn join_strs(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let joined = mut_str(...parts)
    println("${sep} ${joined}")
}

fn main() [use] {
    use StdOutConsole()

    // A variadic of a *primitive* element type: plain, spread, mixed, empty.
    let a = count_ints("plain", 1, 2, 3)
    let rest = array_of(2, 3)
    let b = count_ints("spread", ...rest)
    let c = count_ints("mixed", 1, ...rest)
    let d = count_ints("empty")
    println("counts ${a} ${b} ${c} ${d}")

    // The spread source survives the calls: a variadic position is not
    // tracked, so the callee's owned parameter is built by cloning.
    println("reusable ${size(rest)}")

    // A reference element type, which always worked.
    join_strs("refs", "a", "b")
    let words = array_of("y", "z")
    join_strs("refs", "x", ...words)
}
"#;

pub const USER_VARIADIC_OUTPUT: &str = "plain 3\n\
     spread 2\n\
     mixed 3\n\
     empty 0\n\
     counts 4 3 4 1\n\
     reusable 2\n\
     refs ab\n\
     refs xyz\n";

#[test]
fn rustc_compiles_and_runs_user_variadics() {
    let files = generate(&[("main.sv", USER_VARIADIC_DEMO)]);
    run_rust_files(&files, "user-variadics", USER_VARIADIC_OUTPUT);
}

// ===== member overloading within one effect [effect-member-overload] =====

/// [effect-member-overload] The phase-4 shape: `Fs` declares `close` once per
/// stream token (FILE_SYSTEM.md §5.10.2 sub-question A). Rust cannot overload
/// a trait method at all, so the names have to be made distinct — by
/// `salvo_core`'s rule, so the Kotlin backend picks the same ones.
const MEMBER_OVERLOADS: &str = r#"
struct InFile { id: Int }
struct OutFile { id: Int }

effect Vault {
    fn close(f: InFile) -> Str => !f
    fn close(f: OutFile) -> Str => !f
    fn describe(f: InFile) -> Str => f
}

handler Files of Vault {
    fn close(f: InFile) -> Str => !f {
        return "closed in ${f.id}"
    }
    fn close(f: OutFile) -> Str => !f {
        return "closed out ${f.id}"
    }
    fn describe(f: InFile) -> Str => f {
        return "file ${f.id}"
    }
}

fn main() [use] {
    use StdOutConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    println(close(InFile { id: 1 }))
    println(close@Vault(OutFile { id: 2 }))
}
"#;

const MEMBER_OVERLOADS_OUTPUT: &str = "file 3\nclosed in 1\nclosed out 2\n";

/// [effect-member-overload] [rs-effects] The trait declares the overloads
/// under distinct names — the first keeps the plain one — the handler's impl
/// uses those same names, and each call site emits the one the *checker*
/// resolved.
#[test]
fn effect_member_overloads_get_distinct_names() {
    let files = generate(&[("main.sv", MEMBER_OVERLOADS)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The trait: `close`, then `close__2`, and the un-overloaded member
        // keeps its own name. [rs-borrows] The **consuming** overloads take
        // their parameter by value (`=> !f`) while the keeping `describe`
        // borrows — a member's written clause is its whole contract.
        "fn close(&mut self, f: InFile) -> String;",
        "fn close__2(&mut self, f: OutFile) -> String;",
        "fn describe(&mut self, f: &InFile) -> String;",
        // The handler implements both under the trait's names, with the
        // trait's modes.
        "impl Vault for Files {",
        "fn close(&mut self, f: InFile) -> String {",
        "fn close__2(&mut self, f: OutFile) -> String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // The call sites: the `InFile` one takes the base name, the `OutFile` one
    // (written with the `@Vault` selector) the suffixed name.
    assert!(
        src.contains(".close(InFile {") && src.contains(".close__2(OutFile {"),
        "expected both call sites to name their own overload and pass the \
         consumed argument owned, got:\n{src}"
    );
}

/// [effect-member-overload] End to end: which overload runs is the checker's
/// answer, and rustc must have no opinion. Byte-identical stdout on Kotlin.
#[test]
fn rustc_compiles_and_runs_member_overloads() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MEMBER_OVERLOADS)]);
    run_rust_files(&files, "member-overloads", MEMBER_OVERLOADS_OUTPUT);
}

// ===== member parameter modes from the declared clause [rs-borrows] =====

/// [rs-borrows] [deduce-syntax] An effect member's parameter modes come from
/// its *written* deduction clause: `=> !t` is taken by value, `=> t: Mut` by
/// `&mut`. Phase 4's `Fs` is built out of exactly these two shapes — a
/// consuming `close(s: InStream)` and a mutating `read_line(s: Mut InStream)`
/// — and the interesting case is a **dependent** handler forwarding both,
/// since the trait method, the handler impls, the generated `__Impl_H` trait
/// and the fusion's forwarding impl must all agree or rustc refuses.
const MEMBER_MODES: &str = r#"
struct Token canbe Mut { id: Int }

effect Sink {
    fn take(t: Token) -> Int => !t
    fn bump(t: Mut Token) -> None => t: Mut
}

handler Direct of Sink {
    fn take(t: Token) -> Int => !t {
        return t.id
    }
    fn bump(t: Mut Token) -> None => t: Mut {
        t.id = t.id + 1
    }
}

handler Doubling [local Sink] of Sink {
    fn take(t: Token) -> Int => !t {
        return take(t) * 2
    }
    fn bump(t: Mut Token) -> None => t: Mut {
        bump(t)
        bump(t)
    }
}

fn main() [use] {
    use StdOutConsole()
    use Direct()
    use local Doubling()
    let t: Mut Token = Mut Token { id: 1 }
    bump(t)
    println("bumped ${t.id}")
    println("took ${take(t)}")
}
"#;

const MEMBER_MODES_OUTPUT: &str = "bumped 3\ntook 6\n";

/// [rs-borrows] The consumed parameter is owned in the trait, in both
/// handlers, and at the call site; the `Mut` one is `&mut` throughout.
#[test]
fn effect_member_modes_follow_the_declared_clause() {
    let files = generate(&[("main.sv", MEMBER_MODES)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The trait.
        "fn take(&mut self, t: Token) -> i32;",
        "fn bump(&mut self, t: &mut Token);",
        // The independent handler.
        "fn take(&mut self, t: Token) -> i32 {",
        "fn bump(&mut self, t: &mut Token) {",
        // The dependent handler's own trait, and the fusion's forwarding
        // impl, which passes the consumed value straight through.
        "fn take<__Fx: __Has_Sink>(&mut self, __fx: &mut __Fx, t: Token) -> i32;",
        "fn bump<__Fx: __Has_Sink>(&mut self, __fx: &mut __Fx, t: &mut Token);",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

/// [rs-borrows] End to end: the modes have to agree across four renderers,
/// and a program that moves a token into a member and mutates another
/// through one is what proves they do. Byte-identical stdout on Kotlin.
#[test]
fn rustc_compiles_and_runs_member_modes() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MEMBER_MODES)]);
    run_rust_files(&files, "member-modes", MEMBER_MODES_OUTPUT);
}

// ===== linear tokens discharged by effect members [linear-group] =====

/// [linear-group] [effect-member-overload] [rs-borrows] Phase 4's token shape
/// in miniature, and the three changes it needs at once: a **linear** token
/// whose only discharger is an *effect member*, that member **overloaded** per
/// token type, and the consuming overloads taking their token **by value**
/// while the mutating members take `&mut`. `discard` inside the handler bodies
/// is the amendment (user decision 2026-09-14): discharger status attaches to
/// the member declaration, so every handler's implementation of it terminates
/// the obligation.
const LINEAR_MEMBER_DISCHARGE: &str = r#"
// The phase-4 token shape in miniature: a linear token whose only discharger
// is an *effect member*, discharged inside the handler that implements it.
linear struct InTape canbe Mut { handle: Int }
linear struct OutTape canbe Mut { handle: Int }

effect Tape {
    fn open_read(path: Str) -> Mut InTape => path
    fn open_write(path: Str) -> Mut OutTape => path
    fn read_line(s: Mut InTape) -> Str => s: Mut
    fn write(s: Mut OutTape, text: Str) -> Int => s: Mut, text
    fn close(s: InTape) -> Str => !s
    fn close(s: OutTape) -> Str => !s
}

handler MemTape of Tape {
    fn open_read(path: Str) -> Mut InTape => path {
        return Mut InTape { handle: size(path) }
    }
    fn open_write(path: Str) -> Mut OutTape => path {
        return Mut OutTape { handle: size(path) }
    }
    fn read_line(s: Mut InTape) -> Str => s: Mut {
        s.handle = s.handle + 1
        return "line ${s.handle}"
    }
    fn write(s: Mut OutTape, text: Str) -> Int => s: Mut, text {
        s.handle = s.handle + size(text)
        return size(text)
    }
    fn close(s: InTape) -> Str => !s {
        discard(s)
        return "closed in"
    }
    fn close(s: OutTape) -> Str => !s {
        discard(s)
        return "closed out"
    }
}

fn main() [use] {
    use StdOutConsole()
    use MemTape()
    let r: Mut InTape = open_read("data.txt")
    println(read_line(r))
    println(close(r))
    let w: Mut OutTape = open_write("out.txt")
    let n = write(w, "hello")
    println("wrote ${n}")
    println(close(w))
}
"#;

const LINEAR_MEMBER_DISCHARGE_OUTPUT: &str = "line 9\nclosed in\nwrote 5\nclosed out\n";

#[test]
fn a_linear_token_is_discharged_by_an_effect_member() {
    let files = generate(&[("main.sv", LINEAR_MEMBER_DISCHARGE)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The consuming overloads own their token; the mutating members
        // borrow it mutably.
        "fn close(&mut self, s: InTape) -> String;",
        "fn close__2(&mut self, s: OutTape) -> String;",
        "fn read_line(&mut self, s: &mut InTape) -> String;",
        // A discharged token is dropped, so `discard` emits nothing that
        // could resurrect it: the value simply ends there.
        "fn close(&mut self, s: InTape) -> String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

#[test]
fn rustc_compiles_and_runs_a_linear_token_closed_by_a_member() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_MEMBER_DISCHARGE)]);
    run_rust_files(
        &files,
        "linear-member-discharge",
        LINEAR_MEMBER_DISCHARGE_OUTPUT,
    );
}

// ===== std's byte buffer [bytes-type] =====

/// [bytes-type] [byte-value] The whole `Bytes` surface in one program: both
/// constructors, the read side, the `Mut` side, structural equality, a copy
/// that does not alias, the text bridge and a `for` over the buffer.
///
/// The two backends could not be further apart underneath — a `Vec<u8>` here,
/// a shipped `SalvoBytes` class on the JVM [kt-bytes] — so the value of this
/// case is that it prints the same bytes, the same hex and the same numbers on
/// both.
const BYTES_PROGRAM: &str = r#"
fn main() [use] -> None {
    use StdOutConsole()
    // The two constructors: a fixed buffer, and one under construction.
    let fixed = bytes_of(to_byte(0), to_byte(255), to_byte(200))
    println("fixed: ${fixed} / ${to_hex(fixed)} / ${size(fixed)}")
    let buf = mut_bytes()
    buf.add(to_byte(104))
    buf.append(to_bytes("é"))
    println("built: ${buf} / ${to_hex(buf)}")
    // A `Mut Bytes` reaches the read surface by dropping its `Mut`.
    let text = str_of_bytes(buf)
    when text {
        is Str { println("text: ${text}") }
        is None { println("text: invalid") }
    }
    // Invalid UTF-8 decodes to nothing, strictly, on both backends.
    let broken = mut_bytes()
    broken.add(to_byte(200))
    println("broken: ${str_of_bytes(broken) is None}")
    // Reading: element, slice, search — each optional where it can miss.
    println("at 1: ${to_int(get(fixed, 1)!)} / oob: ${get(fixed, 9) is None}")
    println("slice: ${slice(fixed, 1, 3)!} / bad: ${slice(fixed, 1, 9) is None}")
    println("index: ${index_of(fixed, to_byte(200))!} / ${index_of(fixed, to_byte(7)) is None}")
    // Writing in place, and the clear that makes a buffer reusable.
    let scratch = mut_bytes(fixed)
    scratch.set(0, to_byte(1))
    scratch.set(9, to_byte(2))
    println("set: ${scratch}")
    clear(scratch)
    println("cleared: ${scratch} / ${size(scratch)}")
    // `==` is structural, and `copy` really copies.
    let a = bytes_of(to_byte(1), to_byte(2))
    let b = bytes_of(to_byte(1), to_byte(2))
    println("equal: ${a == b} / ${a == fixed}")
    let dup = copy(a)
    let grow = mut_bytes(dup)
    grow.add(to_byte(3))
    println("copy independent: ${a} vs ${grow}")
    // A `for` over a buffer, and the combinators over the same pass.
    let total = 0
    for byte in fixed {
        total = total + to_int(byte)
    }
    println("sum: ${total}")
    let doubled = map(iter(a), each -> to_int(each) * 2)
    println("mapped: ${doubled}")
}
"#;

const BYTES_OUTPUT: &str = "fixed: [0, 255, 200] / 00ffc8 / 3\n\
                            built: [104, 195, 169] / 68c3a9\ntext: hé\n\
                            broken: true\nat 1: 255 / oob: true\n\
                            slice: [255, 200] / bad: true\nindex: 2 / true\n\
                            set: [1, 255, 200]\ncleared: [] / 0\n\
                            equal: true / false\n\
                            copy independent: [1, 2] vs [1, 2, 3]\n\
                            sum: 455\nmapped: [2, 4]\n";
#[test]
fn rustc_compiles_and_runs_bytes() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", BYTES_PROGRAM)]);
    run_rust_files(&files, "bytes", BYTES_OUTPUT);
}

/// [bytes-type] `Bytes` and `Mut Bytes` are the *same* Rust type — a
/// `Vec<u8>`, unboxed — so a dropped `Mut` renders nothing and the buffer
/// needs no runtime helper at all, which is the asymmetry with Kotlin's
/// shipped class [kt-bytes].
#[test]
fn bytes_is_a_vec_of_u8() {
    let files = generate(&[("main.sv", BYTES_PROGRAM)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("let mut buf = Vec::<u8>::new();"),
        "expected a plain vector for the builder:\n{}",
        main.content
    );
    // A fixed buffer is a `vec![]` of `u8`, and a `for` over one iterates the
    // bytes natively — no pass, no boxing, nothing shipped.
    assert!(
        main.content.contains("for mut byte in fixed.clone()"),
        "expected the native byte loop:\n{}",
        main.content
    );
    assert!(
        !files
            .iter()
            .any(|f| f.rel_path.to_string_lossy().contains("bytes.rs")
                && f.content.contains("struct SalvoBytes")),
        "the rust backend ships no buffer class"
    );
}

// ===== std's filesystem [platform-handler] [linear-group] =====

/// std's `core.fs`, exercised end to end against real files: the
/// `platform handler HostRawFs` at the bottom, `DefaultFs [RawFs]` above it,
/// linear stream tokens discharged by an effect member, the `Lines` pass,
/// the one-shots, a `position` after a ranged open, and a failure path whose
/// linear `FsError` is acknowledged once. `__DIR__` is replaced with a
/// scratch directory: the program makes it, works in it and removes it, so
/// nothing is left behind and the output mentions no paths.
///
/// The Kotlin backend runs this program verbatim, and both must print these
/// lines byte for byte — the whole point of the layering is that only the
/// host file differs.
const FS_PROGRAM: &str = r#"
fn describe(e: FsError) [] -> Str => e {
    if e.kind is NotFound {
        return "not found"
    }
    if e.kind is InvalidUtf8 {
        return "not utf-8"
    }
    return "other"
}

fn main() [use] -> None {
    use StdOutConsole()
    use HostRawFs()
    use DefaultFs()

    let dir = "__DIR__"
    let made = create_dirs(dir)
    when made {
        is Ok { println("made") }
        is Err { println("made: ${describe(made)}") ignore(made) }
    }

    let path = "__DIR__/notes.txt"
    let written = write_str(path, "alpha\nbeta\ngamma\n")
    when written {
        is Ok { println("wrote ${written}") }
        is Err { println("wrote: ${describe(written)}") ignore(written) }
    }

    let all = read_lines(path)
    when all {
        is Ok { println("lines: ${all}") }
        is Err { println("lines: ${describe(all)}") ignore(all) }
    }

    let opened = open_read(path)
    when opened {
        is Ok {
            let p = lines(opened)
            for line in p {
                println("line: ${line}")
            }
            let closed = close(p)
            when closed {
                is Ok { println("closed") }
                is Err { println("closed: ${describe(closed)}") ignore(closed) }
            }
        }
        is Err { println("open: ${describe(opened)}") ignore(opened) }
    }

    // A ranged open: "alpha\n" is six bytes, so the next line starts there.
    let tail = open_read_at(path, 6)
    when tail {
        is Ok {
            let s: InStream = tail
            let line = read_line(s)
            when line {
                is Str { println("at 6: ${line}") }
                is None { println("at 6: end") }
            }
            println("position: ${position(s)}")
            let shut = close(s)
            when shut {
                is Ok { println("") }
                is Err { println("tail: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("tail: ${describe(tail)}") ignore(tail) }
    }

    let missing = read_to_str("__DIR__/nope.txt")
    when missing {
        is Ok { println("unexpected") }
        is Err { println("missing: ${describe(missing)}") ignore(missing) }
    }

    let listed = list_dir(dir)
    when listed {
        is Ok { println("dir: ${listed}") }
        is Err { println("dir: ${describe(listed)}") ignore(listed) }
    }


    // [fs-bytes] The byte surface: a file that is not text, written and read
    // back exactly. `to_byte` keeps the low 8 bits and a `Byte` is unsigned on
    // both backends, so 255 prints as 255 on both — a signed byte would print
    // -1 on one of them [backend-parity].
    let out = open_write("__DIR__/raw.bin")
    when out {
        is Ok {
            let w: OutStream = out
            let data = bytes_of(to_byte(0), to_byte(255), to_byte(200))
            let n = write_bytes(w, data)
            let m = write(w, "hé")
            println("bytes: ${n} + ${m} at ${position(w)}")
            let shut = close(w)
            when shut {
                is Ok { println("wrote raw") }
                is Err { println("wrote raw: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("raw open: ${describe(out)}") ignore(out) }
    }
    let raw = open_read("__DIR__/raw.bin")
    when raw {
        is Ok {
            let s: InStream = raw
            let head = read_bytes(s, 3)
            when head {
                is Ok { println("read: ${head} at ${position(s)} (${to_hex(head)})") }
                is Err { println("read: ${describe(head)}") ignore(head) }
            }
            // Bytes and text off one stream: the position is bytes either way,
            // so the text read picks up exactly where the byte read stopped.
            let text = read_all(s)
            when text {
                is Ok { println("then: ${text}") }
                is Err { println("then: ${describe(text)}") ignore(text) }
            }
            let shut = close(s)
            when shut {
                is Ok { println("read raw") }
                is Err { println("read raw: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("raw read: ${describe(raw)}") ignore(raw) }
    }
    // Opening between the bytes of a character is a *seek*, not a decode: it
    // succeeds, and the strict decode afterwards is what fails — recorded, so
    // `close` reports it too [fs-errors-at-close].
    let split = open_read_at("__DIR__/raw.bin", 5)
    when split {
        is Ok {
            let s: InStream = split
            let bad = read_all(s)
            when bad {
                is Ok { println("split: ${bad}") }
                is Err { println("split: ${describe(bad)}") ignore(bad) }
            }
            let shut = close(s)
            when shut {
                is Ok { println("split closed") }
                is Err { println("split close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("split open: ${describe(split)}") ignore(split) }
    }

    // [fs-read-to] The fill-a-buffer read: one buffer, cleared and refilled,
    // instead of a payload per step.
    let held = open_read("__DIR__/raw.bin")
    when held {
        is Ok {
            let s: InStream = held
            let buf = mut_bytes()
            let steps = 0
            let moved = 0
            let reading = true
            while reading {
                clear(buf)
                let got = read_to(s, buf, 2)
                if got is Err {
                    println("read_to: ${describe(got)}")
                    ignore(got)
                    reading = false
                } else {
                    let n: Int = got
                    if n == 0 {
                        reading = false
                    } else {
                        steps = steps + 1
                        moved = moved + n
                    }
                }
            }
            println("filled ${moved} bytes in ${steps} reads")
            let shut = close(s)
            when shut {
                is Ok { println("filled closed") }
                is Err { println("filled close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("filled open: ${describe(held)}") ignore(held) }
    }
    // The chunk pass: `for` over a stream's bytes, a fresh buffer per step.
    let ch = open_chunks("__DIR__/raw.bin", 4)
    when ch {
        is Ok {
            let p = ch
            let seen = 0
            for chunk in p {
                seen = seen + size(chunk)
            }
            println("pass saw ${seen} bytes")
            let shut = close(p)
            when shut {
                is Ok { println("pass closed") }
                is Err { println("pass close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("pass open: ${describe(ch)}") ignore(ch) }
    }
    // And the text side of `read_to`: a line per iteration, one builder.
    let lined = open_read(path)
    when lined {
        is Ok {
            let s: InStream = lined
            let line = mut_str()
            let lines_seen = 0
            let reading = true
            while reading {
                clear(line)
                if read_line_to(s, line) {
                    lines_seen = lines_seen + 1
                } else {
                    reading = false
                }
            }
            println("read ${lines_seen} lines into one builder")
            let shut = close(s)
            when shut {
                is Ok { println("lines closed") }
                is Err { println("lines close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("lines open: ${describe(lined)}") ignore(lined) }
    }
    // The one-shots that own the buffer: a whole-file copy and a whole-file
    // byte read, with no stream and no buffer in sight.
    let copied = copy_file(path, "__DIR__/copy.txt")
    when copied {
        is Ok { println("copied ${copied}") }
        is Err { println("copied: ${describe(copied)}") ignore(copied) }
    }
    let all_bytes = read_to_bytes("__DIR__/copy.txt")
    when all_bytes {
        is Ok { println("copy holds ${size(all_bytes)} bytes") }
        is Err { println("copy read: ${describe(all_bytes)}") ignore(all_bytes) }
    }
    let gone_copy = delete("__DIR__/copy.txt")
    when gone_copy {
        is Ok { println("copy deleted") }
        is Err { println("copy deleted: ${describe(gone_copy)}") ignore(gone_copy) }
    }
    let gone_bin = delete("__DIR__/raw.bin")
    when gone_bin {
        is Ok { println("raw deleted") }
        is Err { println("raw deleted: ${describe(gone_bin)}") ignore(gone_bin) }
    }

    let gone = delete(path)
    when gone {
        is Ok { println("deleted") }
        is Err { println("deleted: ${describe(gone)}") ignore(gone) }
    }
    let gone_dir = delete(dir)
    when gone_dir {
        is Ok { println("removed") }
        is Err { println("removed: ${describe(gone_dir)}") ignore(gone_dir) }
    }
}
"#;

const FS_OUTPUT: &str = "made\nwrote 17\nlines: [alpha, beta, gamma]\n\
                         line: alpha\nline: beta\nline: gamma\nclosed\n\
                         at 6: beta\nposition: 11\n\nmissing: not found\n\
                         dir: [notes.txt]\n\
                         bytes: 3 + 3 at 6\nwrote raw\n\
                         read: [0, 255, 200] at 3 (00ffc8)\nthen: hé\nread raw\n\
                         split: not utf-8\nsplit close: not utf-8\n\
                         filled 6 bytes in 3 reads\nfilled closed\n\
                         pass saw 6 bytes\npass closed\n\
                         read 3 lines into one builder\nlines closed\n\
                         copied 17\ncopy holds 17 bytes\ncopy deleted\n\
                         raw deleted\n\
                         deleted\nremoved\n";

/// The program with its scratch directory baked in. Under the crate's own
/// target tmpdir, so the two backends' runs cannot collide.
fn fs_program() -> String {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("fs-e2e-data");
    FS_PROGRAM.replace("__DIR__", &dir.to_string_lossy())
}

/// [rs-platform-handler] What the layering emits: no struct for the platform
/// handler, `DefaultFs` as an ordinary Salvo handler reaching `RawFs` through
/// the fusion, and the consuming member taking its token **by value** so the
/// discharge is a move rather than a clone.
#[test]
fn the_fs_surface_emits_a_host_seam_and_owned_tokens() {
    let files = generate(&[("main.sv", &fs_program())]);
    let surface = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/fs.rs"))
        .expect("core/fs.rs")
        .content;
    for expected in [
        "pub trait Fs {",
        // The token is consumed: by value, not `&InStream`.
        "fn close(&mut self, s: InStream)",
        // …and kept members borrow it.
        "fn read_line(&mut self, s: &InStream) -> Option<String>",
        // [effect-available] One overload set: the pass's discharger is a
        // *fn* named `close`, beside the two members of that name, and the
        // program calls both.
        "pub fn close<__Fx: __Has_Fs>(__fx: &mut __Fx, p: Lines)",
    ] {
        assert!(
            surface.contains(expected),
            "expected `{expected}` in:\n{surface}"
        );
    }
    // The host seam is its own module [mod-used-only]: a program that never
    // opens a file links none of it.
    let host = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/hostfs.rs"))
        .expect("core/hostfs.rs")
        .content;
    for expected in [
        "pub trait RawFs {",
        "fn raw_close_read(&mut self, handle: i64)",
        "pub struct DefaultFs",
    ] {
        assert!(host.contains(expected), "expected `{expected}` in:\n{host}");
    }
    assert!(
        !host.contains("pub struct HostRawFs"),
        "the platform handler's struct is the host's:\n{host}"
    );
    // std ships the host file, and it travels into the output.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path == std::path::Path::new("platform/core/hostfs.rs")),
        "expected std's host companion to be emitted"
    );
}

#[test]
fn rustc_compiles_and_runs_the_fs_surface() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", &fs_program())]);
    run_rust_files(&files, "fs-surface", FS_OUTPUT);
}

// ===== std's in-memory filesystem and the restriction over it =====

/// [fs-double] [effect-intercept] `MemFs` fakes the whole of `Fs` — streams
/// included — so this program has **no host handler at all** and touches no
/// disk, which is the point of putting the stream operations on the effect.
/// Then `RestrictedFs("notes")` intercepts it for the length of a block, so
/// the containment logic is tested over memory too.
///
/// The byte offsets are the hazard the fake exists to get right: `position`
/// answers eleven here exactly as it does against real files (the fs case
/// above asserts the same number), because `MemFs` counts UTF-8 bytes with
/// `byte_size` while slicing its content by characters. A fake that counted
/// characters would let this test pass and production break.
const MEMFS_PROGRAM: &str = r#"
fn describe(e: FsError) [] -> Str => e {
    if e.kind is NotFound {
        return "not found"
    }
    if e.kind is PathEscapes {
        return "escapes"
    }
    if e.kind is NotADirectory {
        return "not a directory"
    }
    if e.kind is InvalidUtf8 {
        return "not utf-8"
    }
    return "other"
}

fn read_and_say(label: Str, path: Str) [Fs, Console] -> None => label, path {
    let text = read_to_str(path)
    when text {
        is Ok { println("${label}: ${size(text)}") }
        is Err { println("${label}: ${describe(text)}") ignore(text) }
    }
}

fn main() [use] -> None {
    use StdOutConsole()
    // No host handler anywhere: `MemFs` fakes the whole of `Fs`, streams
    // included, so this program touches no disk.
    use MemFs()

    let wrote = write_str("notes/a.txt", "alpha\nbeta\ngamma\n")
    when wrote {
        is Ok { println("wrote ${wrote}") }
        is Err { println("wrote: ${describe(wrote)}") ignore(wrote) }
    }

    let lines = read_lines("notes/a.txt")
    when lines {
        is Ok { println("lines: ${lines}") }
        is Err { println("lines: ${describe(lines)}") ignore(lines) }
    }

    // The byte offset the fake has to agree with the host about: "alpha\n" is
    // six bytes, and the position after reading "beta\n" is eleven.
    let tail = open_read_at("notes/a.txt", 6)
    when tail {
        is Ok {
            let s: InStream = tail
            let line = read_line(s)
            when line {
                is Str { println("at 6: ${line}") }
                is None { println("at 6: end") }
            }
            println("position: ${position(s)}")
            let shut = close(s)
            when shut {
                is Ok { println("closed") }
                is Err { println("close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("at 6: ${describe(tail)}") ignore(tail) }
    }

    let listed = list_dir("notes")
    when listed {
        is Ok { println("dir: ${listed}") }
        is Err { println("dir: ${describe(listed)}") ignore(listed) }
    }


    // [fs-bytes] The byte surface: a file that is not text, written and read
    // back exactly. `to_byte` keeps the low 8 bits and a `Byte` is unsigned on
    // both backends, so 255 prints as 255 on both — a signed byte would print
    // -1 on one of them [backend-parity].
    let out = open_write("notes/raw.bin")
    when out {
        is Ok {
            let w: OutStream = out
            let data = bytes_of(to_byte(0), to_byte(255), to_byte(200))
            let n = write_bytes(w, data)
            let m = write(w, "hé")
            println("bytes: ${n} + ${m} at ${position(w)}")
            let shut = close(w)
            when shut {
                is Ok { println("wrote raw") }
                is Err { println("wrote raw: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("raw open: ${describe(out)}") ignore(out) }
    }
    let raw = open_read("notes/raw.bin")
    when raw {
        is Ok {
            let s: InStream = raw
            let head = read_bytes(s, 3)
            when head {
                is Ok { println("read: ${head} at ${position(s)} (${to_hex(head)})") }
                is Err { println("read: ${describe(head)}") ignore(head) }
            }
            // Bytes and text off one stream: the position is bytes either way,
            // so the text read picks up exactly where the byte read stopped.
            let text = read_all(s)
            when text {
                is Ok { println("then: ${text}") }
                is Err { println("then: ${describe(text)}") ignore(text) }
            }
            let shut = close(s)
            when shut {
                is Ok { println("read raw") }
                is Err { println("read raw: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("raw read: ${describe(raw)}") ignore(raw) }
    }
    // Opening between the bytes of a character is a *seek*, not a decode: it
    // succeeds, and the strict decode afterwards is what fails — recorded, so
    // `close` reports it too [fs-errors-at-close].
    let split = open_read_at("notes/raw.bin", 5)
    when split {
        is Ok {
            let s: InStream = split
            let bad = read_all(s)
            when bad {
                is Ok { println("split: ${bad}") }
                is Err { println("split: ${describe(bad)}") ignore(bad) }
            }
            let shut = close(s)
            when shut {
                is Ok { println("split closed") }
                is Err { println("split close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("split open: ${describe(split)}") ignore(split) }
    }

    // [fs-read-to] The fill-a-buffer read: one buffer, cleared and refilled,
    // instead of a payload per step.
    let held = open_read("notes/raw.bin")
    when held {
        is Ok {
            let s: InStream = held
            let buf = mut_bytes()
            let steps = 0
            let moved = 0
            let reading = true
            while reading {
                clear(buf)
                let got = read_to(s, buf, 2)
                if got is Err {
                    println("read_to: ${describe(got)}")
                    ignore(got)
                    reading = false
                } else {
                    let n: Int = got
                    if n == 0 {
                        reading = false
                    } else {
                        steps = steps + 1
                        moved = moved + n
                    }
                }
            }
            println("filled ${moved} bytes in ${steps} reads")
            let shut = close(s)
            when shut {
                is Ok { println("filled closed") }
                is Err { println("filled close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("filled open: ${describe(held)}") ignore(held) }
    }
    // The chunk pass: `for` over a stream's bytes, a fresh buffer per step.
    let ch = open_chunks("notes/raw.bin", 4)
    when ch {
        is Ok {
            let p = ch
            let seen = 0
            for chunk in p {
                seen = seen + size(chunk)
            }
            println("pass saw ${seen} bytes")
            let shut = close(p)
            when shut {
                is Ok { println("pass closed") }
                is Err { println("pass close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("pass open: ${describe(ch)}") ignore(ch) }
    }
    // And the text side of `read_to`: a line per iteration, one builder.
    let lined = open_read("notes/a.txt")
    when lined {
        is Ok {
            let s: InStream = lined
            let line = mut_str()
            let lines_seen = 0
            let reading = true
            while reading {
                clear(line)
                if read_line_to(s, line) {
                    lines_seen = lines_seen + 1
                } else {
                    reading = false
                }
            }
            println("read ${lines_seen} lines into one builder")
            let shut = close(s)
            when shut {
                is Ok { println("lines closed") }
                is Err { println("lines close: ${describe(shut)}") ignore(shut) }
            }
        }
        is Err { println("lines open: ${describe(lined)}") ignore(lined) }
    }
    // The one-shots that own the buffer: a whole-file copy and a whole-file
    // byte read, with no stream and no buffer in sight.
    let copied = copy_file("notes/a.txt", "notes/copy.txt")
    when copied {
        is Ok { println("copied ${copied}") }
        is Err { println("copied: ${describe(copied)}") ignore(copied) }
    }
    let all_bytes = read_to_bytes("notes/copy.txt")
    when all_bytes {
        is Ok { println("copy holds ${size(all_bytes)} bytes") }
        is Err { println("copy read: ${describe(all_bytes)}") ignore(all_bytes) }
    }
    let gone_copy = delete("notes/copy.txt")
    when gone_copy {
        is Ok { println("copy deleted") }
        is Err { println("copy deleted: ${describe(gone_copy)}") ignore(gone_copy) }
    }
    let gone_bin = delete("notes/raw.bin")
    when gone_bin {
        is Ok { println("raw deleted") }
        is Err { println("raw deleted: ${describe(gone_bin)}") ignore(gone_bin) }
    }

    // The restriction, over the same memory, for the length of the block: an
    // interceptor wraps the handler already registered.
    if true {
        use RestrictedFs("notes")
        read_and_say("inside", "a.txt")
        read_and_say("through ..", "sub/../a.txt")
        read_and_say("escape", "../secret.txt")
        read_and_say("absolute", "/etc/hosts")
    }
    // Out of the block the unrestricted filesystem answers again.
    read_and_say("unrestricted", "notes/a.txt")
}
"#;

const MEMFS_OUTPUT: &str = "wrote 17\nlines: [alpha, beta, gamma]\nat 6: beta\nposition: 11\n\
                         closed\ndir: [a.txt]\n\
                         bytes: 3 + 3 at 6\nwrote raw\n\
                         read: [0, 255, 200] at 3 (00ffc8)\nthen: hé\nread raw\n\
                         split: not utf-8\nsplit close: not utf-8\n\
                         filled 6 bytes in 3 reads\nfilled closed\n\
                         pass saw 6 bytes\npass closed\n\
                         read 3 lines into one builder\nlines closed\n\
                         copied 17\ncopy holds 17 bytes\ncopy deleted\n\
                         raw deleted\n\
                         inside: 17\nthrough ..: 17\n\
                         escape: escapes\nabsolute: escapes\nunrestricted: 17\n";

#[test]
fn rustc_compiles_and_runs_the_memory_filesystem() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MEMFS_PROGRAM)]);
    run_rust_files(&files, "memfs", MEMFS_OUTPUT);
}

// ===== [effect-available] a member name that is also a std fn =====

/// [effect-available] A user effect may name a member after a std function, and
/// both must keep working — the member where an instance is in scope, the
/// function everywhere else and wherever `@module` says so. Two bugs lived here
/// until 2026-09-15, both from the checker and the emitters asking *different
/// questions*: the checker's availability rule reads an **import-scoped**
/// table, and inside `std/core/seq.sv` a program's `Tally` is not in it, so
/// `add(out, x)` resolved to std's own `add` and nothing was recorded; the
/// emitters read the **program-wide** `Symbols::effect_of_fn` and emitted a
/// member dispatch anyway.
///
/// The first symptom was loud — "`std/core/seq.sv`: no handler for effect
/// `Tally`" from inside std, for any program declaring such a member, whether
/// or not it ever called the colliding name. The second was **silently wrong
/// output**: `to_upper@core.string("hi")` ran the *member* and printed `hi!`
/// where `HI` was asked for, on both backends. Hence a compile-and-run case
/// rather than a text assertion — only running it catches the second.
///
/// Same program and same expected output on the Kotlin backend.
const MEMBER_NAME_COLLISION: &str = r#"
effect Tally {
    fn add(n: Int) -> None => !n
    fn read() -> Int
}

handler Summing() of Tally {
    sum: Int = 0

    fn add(n: Int) -> None {
        sum = sum + n
    }

    fn read() -> Int {
        return sum
    }
}

effect Shout {
    fn to_upper(s: Str) -> Str => !s
}

handler Excited() of Shout {
    fn to_upper(s: Str) -> Str {
        return "${s}!"
    }
}

fn double(x: Int) [] -> Int {
    return x * 2
}

fn main() [use] {
    use StdOutConsole()
    use Summing()
    use Excited()
    // The member: an instance is in scope, so the bare name is the member.
    add(4)
    add(5)
    println("tally ${read()}")
    // std's own `add`, reached by hand — and std's internals, which call
    // `add(out, x)` inside `map`, reached without the program saying anything.
    let xs: Mut List<Int> = mut_list_of()
    add@core.list(xs, 7)
    let mapped = map(iter([1, 2, 3]), double)
    println("list ${size(xs)} mapped ${size(mapped)}")
    // Both spellings of a name owned by an effect *and* by std.
    let mine = to_upper@Shout("hi")
    let theirs = to_upper@core.string("hi")
    println("member ${mine} std ${theirs}")
}
"#;

const MEMBER_NAME_COLLISION_OUTPUT: &str = "tally 9\nlist 1 mapped 3\nmember hi! std HI\n";

#[test]
fn rustc_compiles_and_runs_a_member_named_like_a_std_fn() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MEMBER_NAME_COLLISION)]);
    run_rust_files(&files, "member-name-collision", MEMBER_NAME_COLLISION_OUTPUT);
}

// ===== [rs-actor] asynchronous effect handlers =====

/// [actor-spawn-expr] [actor-use-addr] [actor-waitfor] The first program that
/// *runs* a process: a counter handler bound asynchronously, two sends, then
/// `main`'s bridge asking for the total. The expected output is identical on
/// the Kotlin backend — the parity assertion for the whole surface, not just
/// the scheduler library it rests on.
const ACTOR: &str = r#"
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 8 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let counter = spawn Counting() on pool(1)
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("sum ${sum}")
}
"#;

fn generate_actor_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", ACTOR)])
}

#[test]
fn rustc_compiles_and_runs_an_actor() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_actor_demo();
    run_rust_files(&files, "actor", "sum 5\n");
}

/// [rs-actor] What the lowering *is*, asserted on the generated text so a
/// regression names itself: a message enum per protocol, an actor struct
/// wrapping the handler, and the scheduler module carried into the output.
#[test]
fn an_actor_lowers_to_a_message_enum_and_a_body() {
    let files = generate_actor_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains("pub enum __Msg_Counter {"),
        "the protocol's message enum is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("impl crate::scheduler::SalvoActor for __Actor_Counting"),
        "the actor body is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("crate::scheduler::salvo_spawn("),
        "the spawn is missing:\n{}",
        main.content
    );
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "scheduler.rs"),
        "the scheduler module is not part of the program"
    );
}

/// [monitor-handler] [rs-monitor] The monitor spawn (SH-3, user decision
/// 2026-09-19): a **plain** effect's handler shared behind a lock. One
/// instance serves `main` (use-bound) and a spawned actor (supplied through
/// the dependency clause), and the output is deterministic because the three
/// draws are sequenced by the `waitfor` — the parity assertion with the
/// Kotlin backend's case of the same name.
const MONITOR: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    cursor: Int = 0

    fn next() -> Int {
        cursor = (cursor * 31 + seed) % 100000
        return cursor
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
"#;

fn generate_monitor_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", MONITOR)])
}

#[test]
fn rustc_compiles_and_runs_a_monitor() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_monitor_demo();
    run_rust_files(
        &files,
        "monitor",
        "main drew 12345\nactor drew 95040\nmain drew 58585\n",
    );
}

/// [rs-monitor] What the monitor lowering *is*, asserted on the generated
/// text: the per-effect lock wrapper implementing the effect's trait, the
/// spawn building `Arc<Mutex<…>>` with no scheduler call, and the handle
/// passing into the actor's dependency clause as itself (cloned, not
/// stub-wrapped).
#[test]
fn a_monitor_lowers_to_a_lock_wrapper() {
    let files = generate_monitor_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains("pub struct __Mon_Random {"),
        "the lock wrapper is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("impl Random for __Mon_Random {"),
        "the wrapper does not implement the effect:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("__Mon_Random::new(Box::new(__Lock_Random::new("),
        "the monitor spawn is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("pub trait __Share_Random: Random + Send {"),
        "the clone-box supertrait is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("self.inner.lock().unwrap().next()"),
        "a member does not lock and delegate:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("__Stub_Random"),
        "a plain effect must not get a send stub:\n{}",
        main.content
    );
}

/// [mixed-handler] [rs-mixed] The mixed handler (SH-1): state confined to
/// `send fn` members (the servant), sync members on the caller's thread (the
/// façade) sending to it and waiting — one shared instance serving `main`
/// and a spawned actor, behind a plain effect nobody declares `[waitfor]`
/// for. Same program, same output as the Kotlin backend's case.
const MIXED: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    cursor: Int = 0

    send fn advance(out: Reply<Int>) => !out {
        cursor = (cursor * 31 + seed) % 100000
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

actor effect Drawer {
    send fn draw(out: Reply<Int>) => !out
}

handler Drawing() [Random] of Drawer {
    mailbox { capacity: 4 }

    send fn draw(out: Reply<Int>) {
        send(out, next())
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
"#;

/// [defer-deduction] [mixed-handler] A **rung-4** mixed handler: the servant
/// declares `=> defer out` and answers by *forwarding* the reply to another
/// actor — the caller's wait ends when Delphi discharges it, one activation
/// later. Also exercises the `use … on POOL` sugar. Same output as Kotlin.
const DEFERRING: &str = r#"
effect Random {
    fn next() -> Int
}

actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    n: Int = 0

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Forwarding(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(out)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    let oracle = spawn Delphi() on pool(1)
    use Forwarding(oracle) on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
"#;

#[test]
fn rustc_compiles_and_runs_a_deferring_mixed_handler() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", DEFERRING)]);
    run_rust_files(&files, "deferring", "first 7\nsecond 14\n");
}

/// [defer-deduction] [actor-replyto] [rs-mixed] Parking inside a mixed
/// handler — the full rung-4 shape: the servant captures the caller's reply
/// in a continuation on its own `settled` member (`__Cont_H`, handler-keyed),
/// consults another actor, and answers when the resume delivers the oracle's
/// number one activation later. Same program and output as Kotlin.
const PARKING: &str = r#"
effect Random {
    fn next() -> Int
}

actor effect Oracle {
    send fn divine(out: Reply<Int>) => !out
}

handler Delphi() of Oracle {
    mailbox { capacity: 4 }
    n: Int = 0

    send fn divine(out: Reply<Int>) {
        n = n + 7
        send(out, n)
    }
}

handler Parking(oracle: Addr<Oracle>) of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        oracle.divine(replyto settled(out))
    }

    send fn settled(out: Reply<Int>, drawn: Int) => !out, !drawn {
        send(out, drawn + tally)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    let oracle = spawn Delphi() on pool(1)
    use Parking(oracle) on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
"#;

#[test]
fn rustc_compiles_and_runs_a_parking_mixed_handler() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", PARKING)]);
    run_rust_files(&files, "parking-mixed", "first 8\nsecond 16\n");
}

/// [mixed-handler] [actor-self-send] [rs-mixed] A servant reaching its
/// siblings (user decision 2026-09-19): a bare sibling call in a send member
/// and `k@self(…)` in both member kinds, all lowering to enqueues on the
/// servant's own mailbox (`__Msg_H`), the reply travelling hop to hop by
/// value. Same program and output as Kotlin.
const SERVANT_CHAIN: &str = r#"
effect Random {
    fn next() -> Int
}

handler Chain() of Random {
    mailbox { capacity: 4 }
    tally: Int = 0

    send fn advance(out: Reply<Int>) => defer out {
        tally = tally + 1
        relay(out)
    }

    send fn relay(out: Reply<Int>) => defer out {
        deliver@self(out)
    }

    send fn deliver(out: Reply<Int>) => !out {
        send(out, tally * 10)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance@self(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    use Chain() on pool(1)
    println("first ${next()}")
    println("second ${next()}")
}
"#;

#[test]
fn rustc_compiles_and_runs_a_servant_sibling_chain() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SERVANT_CHAIN)]);
    run_rust_files(&files, "servant-chain", "first 10\nsecond 20\n");
}

fn generate_mixed_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", MIXED)])
}

#[test]
fn rustc_compiles_and_runs_a_mixed_handler() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_mixed_demo();
    run_rust_files(
        &files,
        "mixed",
        "main drew 12345\nactor drew 95040\nmain drew 58585\n",
    );
}

/// [rs-mixed] What the mixed lowering *is*: the handler-keyed message enum
/// and actor body (the servant), the façade struct carrying the addr and
/// ctor params, the façade send lowering, and the spawn answering the handle
/// over the façade.
#[test]
fn a_mixed_handler_lowers_to_a_servant_and_a_facade() {
    let files = generate_mixed_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains("pub enum __Msg_CyclicRandom {"),
        "the servant's message enum is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("impl crate::scheduler::SalvoActor for __Actor_CyclicRandom"),
        "the servant's actor body is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("pub struct __Fac_CyclicRandom {"),
        "the façade struct is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("impl Random for __Fac_CyclicRandom {"),
        "the façade does not implement the effect:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "crate::scheduler::salvo_send(self.__addr, Box::new(__Msg_CyclicRandom::Advance("
        ),
        "the façade send is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("__Mon_Random::new(Box::new(__Fac_CyclicRandom {"),
        "the mixed spawn does not answer the handle over the façade:\n{}",
        main.content
    );
}

/// [actor-spawn-expr] [effect-handler-deps] **A dependent spawn**: the child
/// declares `[Log, Tally]` and the spawn's `with` clause supplies one of each
/// kind — `Recording()` as a **construction**, built on the child, and `tally`
/// as an **`Addr`**, bound to a forwarding stub. That is the binding swap
/// executed: `Counting` is compiled once and neither of its member bodies can
/// tell which of its two dependencies is a process.
///
/// The ordering is deterministic without any synchronisation, and that is the
/// point of arrival order: `bump`, `bump`, `report` are served in that order by
/// one actor, `report` forwards `main`'s own token to the child's `Log`
/// instance (so the answer comes from *inside* the child), and the two `tick`s
/// the bumps sent are already in `tally`'s queue by the time `main` asks it for
/// a total. The expected output is identical on the Kotlin backend.
const DEP_SPAWN: &str = r#"
actor effect Log {
    send fn note(what: Str) => !what
    send fn dump(out: Reply<Str>) => !out
}

actor effect Tally {
    send fn tick(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn report(out: Reply<Str>) => !out
}

handler Recording() of Log {
    mailbox { capacity: 1 }

    last: Str = "none"

    send fn note(what: Str) {
        last = what
    }

    send fn dump(out: Reply<Str>) {
        out.send(copy(last))
    }
}

handler Summing() of Tally {
    mailbox { capacity: 8 }

    sum: Int = 0

    send fn tick(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

handler Counting() [Log, Tally] of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {
        note("bumped ${n}")
        tick(n)
    }

    send fn report(out: Reply<Str>) {
        dump(out)
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let tally = spawn Summing() on pool(1)
    let counter = spawn Counting() with Recording(), tally on pool(1)
    counter.bump(2)
    counter.bump(3)
    let last = waitfor out: Reply<Str> {
        counter.report(out)
    }
    println("last ${last}")
    let sum = waitfor out: Reply<Int> {
        tally.total(out)
    }
    println("sum ${sum}")
}
"#;

const DEP_SPAWN_OUTPUT: &str = "last bumped 3\nsum 5\n";

fn generate_dep_spawn_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", DEP_SPAWN)])
}

#[test]
fn rustc_compiles_and_runs_a_dependent_spawn() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_dep_spawn_demo();
    run_rust_files(&files, "dep-spawn", DEP_SPAWN_OUTPUT);
}

/// [rs-actor] [rs-effect-fusion] The shape a dependent spawn lowers to: a
/// **flat provider** owning one dependency instance per declared dependency,
/// implementing each one's Has-accessor trait; a process generic in those
/// instances; and a `handle` that builds the same `__Deps_H` view a fusion's
/// forwarding impl builds, over the provider instead of over `__outer`.
#[test]
fn a_dependent_spawn_lowers_to_a_flat_provider() {
    let files = generate_dep_spawn_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("pub struct __Prov_Counting<__D0, __D1> {"),
        "the flat provider is missing:\n{text}"
    );
    assert!(
        text.contains("impl<__D0: Log, __D1: Tally> __Has_Log for __Prov_Counting<__D0, __D1>")
            && text.contains(
                "impl<__D0: Log, __D1: Tally> __Has_Tally for __Prov_Counting<__D0, __D1>"
            ),
        "the provider's Has-accessor impls are missing:\n{text}"
    );
    assert!(
        text.contains("prov: __Prov_Counting<__D0, __D1>,"),
        "the process does not own its provider:\n{text}"
    );
    assert!(
        text.contains("let mut __deps = __Deps_Counting{ __p: &mut self.prov };"),
        "the activation does not build the deps view:\n{text}"
    );
    assert!(
        text.contains("__Impl_Counting::bump(&mut self.handler, &mut __deps, n)"),
        "the dispatch does not go through the dependent-member trait:\n{text}"
    );
    // The clause's two kinds, in the handler's declaration order: a
    // construction for `Log`, a forwarding stub for the `Addr<Tally>`.
    assert!(
        text.contains(
            "__Prov_Counting { __d0: Recording::new(), __d1: __Stub_Tally::new(tally) }"
        ),
        "the spawn does not build the provider from its clause:\n{text}"
    );
}

/// [backend-never-wrong] What this slice still refuses, and does not
/// mis-emit: a **generic** handler as a process. (The dependent-handler
/// refusal that used to live here is gone — the feature landed.)
#[test]
fn the_remaining_process_cuts_are_errors() {
    let generic = expect_errors(
        r#"
actor effect Counter {
    send fn bump(n: Int) => !n
}

handler Counting<T>(seed: T) of Counter {
    mailbox { capacity: 8 }

    send fn bump(n: Int) {}
}

fn main() [use, spawn] {
    let c = spawn Counting(1) on pool(1)
}
"#,
    );
    assert!(
        generic
            .iter()
            .any(|e| e.contains("spawning generic handler `Counting` is not supported yet")),
        "expected the generic-handler refusal, got {generic:?}"
    );
}

// ===== [actor-replyto] [actor-self-send] parked continuations =====

/// [actor-replyto] **The shape the slice exists for: `main` is not in the
/// loop.** A fetcher process asks a database process for a row, parking a
/// continuation for the answer; the database replies *to the fetcher*, whose
/// continuation runs and only then fulfils `main`'s `waitfor` token. Before
/// this, every answer had to come back through `main`, which is why the earlier
/// process tests all chain through it.
///
/// The caller's token travels as a **capture** of the continuation
/// (`replyto arrived(out)`), which is what makes the shape writable without
/// linearity-in-collections: the token is moved into the continuation rather
/// than stored in handler state ([linear-composite] still refuses that, and
/// storing one is item 7). Deterministic without any synchronisation, because
/// `main` blocks until the whole chain has run. Identical output on the Kotlin
/// backend.
const REPLYTO_CHAIN: &str = r#"
actor effect Db {
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
}

actor effect Notices {
    send fn fetch(id: Int, out: Reply<Str>) => !id, !out
    send fn arrived(out: Reply<Str>, text: Str) => !out, !text
}

handler Rows() of Db {
    mailbox { capacity: 4 }

    send fn lookup(id: Int, out: Reply<Str>) {
        out.send("row ${id}")
    }
}

handler Fetching() [Db] of Notices {
    mailbox { capacity: 4 }

    send fn fetch(id: Int, out: Reply<Str>) {
        lookup(id, replyto arrived(out))
    }

    send fn arrived(out: Reply<Str>, text: Str) {
        out.send("got ${text}")
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let p = pool(2)
    let rows = spawn Rows() on p
    let fetcher = spawn Fetching() with rows on p
    let answer = waitfor out: Reply<Str> {
        fetcher.fetch(7, out)
    }
    println(answer)
}
"#;

/// [actor-replyto] `replyto!` — the **gate**: bounded selective receive, at
/// most one outstanding per actor. `start` mints a gated continuation and
/// `main` then sends `note("late")`, which is already queued when the
/// activation ends; while gated the process serves *only* the awaited reply, so
/// the recorded order is `reply R, user late`. Ungated it would be the other
/// way round, which is what makes the output the assertion.
const REPLYTO_GATE: &str = r#"
actor effect Echo {
    send fn ping(out: Reply<Str>) => !out
}

actor effect Trace {
    send fn start()
    send fn arrived(text: Str) => !text
    send fn note(what: Str) => !what
    send fn report(out: Reply<Str>) => !out
}

handler Echoing() of Echo {
    mailbox { capacity: 4 }

    send fn ping(out: Reply<Str>) {
        out.send("R")
    }
}

handler Tracing() [Echo] of Trace {
    mailbox { capacity: 4 }

    steps: Mut List<Str> = mut_list_of()

    send fn start() {
        ping(replyto! arrived())
    }

    send fn arrived(text: Str) {
        add(steps, "reply ${text}")
    }

    send fn note(what: Str) {
        add(steps, "user ${what}")
    }

    send fn report(out: Reply<Str>) {
        out.send(join(steps, ", "))
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let p = pool(3)
    let echo = spawn Echoing() on p
    let tracer = spawn Tracing() with echo on p
    tracer.start()
    tracer.note("late")
    let got = waitfor out: Reply<Str> {
        tracer.report(out)
    }
    println(got)
}
"#;

/// [actor-self-send] `k@self(args)` and **both** its readings, in one program
/// and with the same answer from each: an enqueue on the actor's own mailbox
/// when the handler was spawned, and the ordinary inline member call when it
/// was `use`d. A handler is compiled once, so which one applies is a property
/// of the *instance* — the emitters discriminate on the generated `__addr`
/// field at run time rather than compiling the member twice.
const SELF_SEND: &str = r#"
actor effect Steps {
    send fn begin(n: Int, out: Reply<Str>) => !n, !out
    send fn again(n: Int, out: Reply<Str>) => !n, !out
}

handler Stepping() of Steps {
    mailbox { capacity: 4 }

    steps: Mut List<Str> = mut_list_of()

    send fn begin(n: Int, out: Reply<Str>) {
        add(steps, "begin ${n}")
        again@self(n + 1, out)
    }

    send fn again(n: Int, out: Reply<Str>) {
        add(steps, "again ${n}")
        out.send(join(steps, ", "))
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let s = spawn Stepping() on pool(1)
    let spawned = waitfor out: Reply<Str> {
        s.begin(1, out)
    }
    println("spawned ${spawned}")
    use Stepping()
    let inline = waitfor out: Reply<Str> {
        begin(1, out)
    }
    println("inline ${inline}")
}
"#;

fn generate_replyto_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", REPLYTO_CHAIN)])
}

#[test]
fn rustc_compiles_and_runs_a_parked_continuation() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    run_rust_files(&generate_replyto_demo(), "replyto-chain", "got row 7\n");
}

#[test]
fn rustc_compiles_and_runs_a_gated_continuation() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", REPLYTO_GATE)]);
    run_rust_files(&files, "replyto-gate", "reply R, user late\n");
}

#[test]
fn rustc_compiles_and_runs_both_readings_of_a_self_send() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SELF_SEND)]);
    run_rust_files(
        &files,
        "self-send",
        "spawned begin 1, again 2\ninline begin 1, again 2\n",
    );
}

/// [actor-watch] The monitor surface, end to end: an actor faults, the
/// scheduler answers the watcher's token with an `Exit`, a watch registered
/// *after* the death answers immediately, and a send to the corpse is a silent
/// no-op. `main` gets its token from `waitfor`, so watching needs no handler of
/// its own.
///
/// The reason's *text* is deliberately not printed: it is the host's account of
/// the fault (a panic message here, an exception's on the Kotlin backend), the
/// one thing on this surface that is not identical across backends
/// [actor-watch]. Everything else is, and the expected output below is
/// verbatim the Kotlin backend's.
const WATCH: &str = r#"
actor effect Counter {
    send fn bump(n: Int) => !n
    send fn crash()
}

handler Counting() of Counter {
    mailbox { capacity: 8 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    // A faulted activation is what death *is* — there is no `kill`, and a
    // Salvo-level throw cannot cross a member boundary.
    send fn crash() {
        let xs: List<Int> = [1]
        sum = sum + get(xs, 9)!
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let c = spawn Counting() on pool(1)
    c.bump(2)
    let died = waitfor out: Reply<Exit> {
        watch(c, out)
        c.crash()
    }
    println("died with a reason: ${size(died.reason) > 0}")
    let again = waitfor out: Reply<Exit> {
        watch(c, out)
    }
    println("late watch answered: ${size(again.reason) > 0}")
    // Sends to the dead are silent no-ops, so this changes nothing and the
    // program ends normally.
    c.bump(3)
    println("done")
}
"#;

const WATCH_OUTPUT: &str =
    "died with a reason: true\nlate watch answered: true\ndone\n";

/// [linear-container] [linear-state] **Obligations in a collection**, end to
/// end: an actor parks reply tokens in `Mut List<Reply<Str>>` state, answers
/// them one at a time with `remove_first`, and `drain`s the rest on shutdown —
/// putting a fresh list back, because an activation may not leave its state
/// with a hole. Two waiters ask before any answer arrives, so the queue really
/// holds two obligations at once; the second is answered by the drain.
///
/// Expected output is verbatim the Kotlin backend's.
const LINEAR_QUEUE: &str = r#"
actor effect Desk {
    send fn ticket(out: Reply<Str>) => !out
    send fn serve(name: Str) => !name
    send fn close_up(reason: Str) => !reason
}

handler Desking() of Desk {
    mailbox { capacity: 8 }

    waiting: Mut List<Reply<Str>> = mut_list_of()

    send fn ticket(out: Reply<Str>) {
        add(waiting, out)
    }

    // One obligation leaves the queue, and the `None` arm owes nothing.
    send fn serve(name: Str) {
        let next = remove_first(waiting)
        when next {
            is Reply<Str> { next.send("served ${name}") }
            is None { discard(name) }
        }
    }

    // The terminal: every parked token is answered, and the state is whole
    // again before the activation ends.
    send fn close_up(reason: Str) {
        drain(waiting, r -> send(r, "closed: ${reason}"))
        waiting = mut_list_of()
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let desk = spawn Desking() on pool(1)
    let first = waitfor a: Reply<Str> {
        desk.ticket(a)
        let second = waitfor b: Reply<Str> {
            desk.ticket(b)
            desk.serve("ada")
            desk.close_up("end of day")
        }
        println("second ${second}")
    }
    println("first ${first}")
}
"#;

const LINEAR_QUEUE_OUTPUT: &str = "second closed: end of day\nfirst served ada\n";

#[test]
fn rustc_compiles_and_runs_obligations_in_a_collection() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LINEAR_QUEUE)]);
    run_rust_files(&files, "linear-queue", LINEAR_QUEUE_OUTPUT);
}

/// [rs-state-take] [linear-container] The lowering the case rests on: taking a
/// container out of handler state is `std::mem::take` (a field behind
/// `&mut self` cannot be moved, and cloning it would duplicate every
/// obligation in it), and the drain hands each element to the callback by
/// value.
#[test]
fn draining_state_lowers_to_a_take() {
    let files = generate(&[("main.sv", LINEAR_QUEUE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content
            .contains("std::mem::take(&mut self.waiting).into_iter().for_each("),
        "the drain of a state field is not a take:\n{}",
        main.content
    );
    assert!(
        main.content.contains("(first.unwrap()).send(")
            || main.content.contains("(next.unwrap()).send("),
        "a taken obligation must be moved, not cloned:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_death_watch() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", WATCH)]);
    run_rust_files(&files, "watch", WATCH_OUTPUT);
}

/// [actor-watch] [rs-actor] What a `watch` lowers to: the scheduler call
/// **plus the `Exit` builder** the watch site closes over — the runtime holds a
/// reason string and cannot construct a Salvo struct, so the constructor
/// travels with the registration.
#[test]
fn a_watch_lowers_to_a_scheduler_call_with_an_exit_builder() {
    let files = generate(&[("main.sv", WATCH)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains(
            "crate::scheduler::salvo_watch(c, out, |__reason| Box::new(Exit { reason: __reason }))"
        ),
        "the watch registration or its `Exit` builder is missing:\n{}",
        main.content
    );
}

/// [effect-handler-multi] Handlers of several effects, end to end: T-4(a)'s own
/// shape — one actor with a public `Timer` face and a `TimerCtl` admin face over
/// one mailbox and one piece of state, whose `spawn` answers one addr per face —
/// beside the synchronous version of the same pattern, where one `use` binds
/// both faces.
///
/// Expected output is verbatim the Kotlin backend's.
const MULTI_FACE: &str = r#"
// [effect-handler-multi] One actor, one mailbox, one owner of the state, two
// typed faces: the public protocol and the test-control one. Least authority
// falls out of the types — a holder of `timer` cannot name `advance`.
actor effect Timer {
    send fn after(millis: Int, out: Reply<Str>) => !out
}

actor effect TimerCtl {
    send fn advance(millis: Int) => !millis
    send fn pending(out: Reply<Int>) => !out
}

handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }

    waiting: Mut List<Reply<Str>> = mut_list_of()
    now: Int = 0

    send fn after(millis: Int, out: Reply<Str>) {
        add(waiting, out)
    }

    send fn advance(millis: Int) {
        now = now + millis
        let at = now
        drain(waiting, r -> send(r, "fired at ${at}"))
        waiting = mut_list_of()
    }

    send fn pending(out: Reply<Int>) {
        out.send(size(waiting))
    }
}

// The synchronous half of the same shape: a public face and an admin face over
// one piece of state, bound by one `use`.
effect Tally {
    fn bump(n: Int) -> None => !n
}

effect Stats {
    fn total() -> Int
}

handler Counting() of Tally, Stats {
    sum: Int = 0

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

fn count() [local Tally, local Stats] -> Int {
    bump(2)
    bump(3)
    return total()
}

fn main() [use, spawn] {
    use StdOutConsole()
    let (timer, ctl) = spawn ManualTime() on pool(1)
    let idle = waitfor c: Reply<Int> { ctl.pending(c) }
    println("pending ${idle}")
    let fired = waitfor f: Reply<Str> {
        timer.after(10, f)
        ctl.advance(10)
    }
    println(fired)
    let left = waitfor c: Reply<Int> { ctl.pending(c) }
    println("pending ${left}")
    use local Counting()
    println("total ${count()}")
}
"#;

const MULTI_FACE_OUTPUT: &str = "pending 0\nfired at 10\npending 0\ntotal 5\n";

#[test]
fn rustc_compiles_and_runs_a_handler_of_several_effects() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MULTI_FACE)]);
    run_rust_files(&files, "multi-face", MULTI_FACE_OUTPUT);
}

/// [effect-handler-multi] [rs-actor] What the two faces lower to: one trait impl
/// per face over one struct, one dispatcher per protocol, a `handle` that asks
/// each protocol in turn (one mailbox carries both), and a spawn whose value is
/// a tuple of the same scheduler index — the addrs differ only in their Salvo
/// type.
#[test]
fn several_faces_lower_to_one_actor_with_a_dispatcher_each() {
    let files = generate(&[("main.sv", MULTI_FACE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("impl Timer for ManualTime {") && text.contains("impl TimerCtl for ManualTime {"),
        "one trait impl per face is missing:\n{text}"
    );
    assert!(
        text.contains("fn __dispatch_Timer(&mut self, msg: crate::__Msg_Timer)")
            && text.contains("fn __dispatch_TimerCtl(&mut self, msg: crate::__Msg_TimerCtl)"),
        "one dispatcher per protocol is missing:\n{text}"
    );
    assert!(
        text.contains("match msg.downcast::<crate::__Msg_Timer>()")
            && text.contains("match msg.downcast::<crate::__Msg_TimerCtl>()"),
        "the delivery must ask each protocol in turn:\n{text}"
    );
    assert!(
        text.contains("let __a = crate::scheduler::salvo_spawn(") && text.contains("(__a, __a)"),
        "the spawn must answer one addr per face:\n{text}"
    );
    assert!(
        text.contains("impl Tally for Counting {") && text.contains("impl Stats for Counting {"),
        "the synchronous handler's faces are missing:\n{text}"
    );
}

/// [actor-on-idle] The quiescence hook, end to end: the program hears about
/// the scheduler running dry, twice — once with the pool settled and nothing
/// outstanding, once with an actor gated on an answer another actor has parked
/// and will never send. The counts are what distinguish *done* from *stuck*.
///
/// The hook is also what makes the second report **deterministic**: the message
/// sent just before the registration cannot still be queued when the answer
/// arrives, because a queued entry is deliverable and a deliverable entry is
/// not idle. Expected output is verbatim the Kotlin backend's.
const ON_IDLE: &str = r#"
actor effect Desk {
    send fn ask(out: Reply<Str>) => !out
}

// Parks every token it is given and answers none, which is legal because the
// obligation lives in state [linear-state] — and is exactly the shape a
// quiescence report has to be able to name.
handler Desking() of Desk {
    mailbox { capacity: 4 }

    waiting: Mut List<Reply<Str>> = mut_list_of()

    send fn ask(out: Reply<Str>) {
        add(waiting, out)
    }
}

actor effect Client {
    send fn go(desk: Addr<Desk>) => !desk
    send fn answered(word: Str) => !word
}

// The gate: while the continuation is outstanding this actor serves nothing
// else, so its mailbox is stalled for good.
handler Clienting() of Client {
    mailbox { capacity: 4 }

    send fn go(desk: Addr<Desk>) {
        desk.ask(replyto! answered())
    }

    send fn answered(word: Str) {
        discard(word)
    }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let p = pool(1)
    let desk = spawn Desking() on p
    let client = spawn Clienting() on p
    let settled = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("settled: gates ${settled.parked_gates}, tokens ${settled.parked_tokens}")
    client.go(desk)
    let stuck = waitfor i: Reply<Idle> { on_idle(p, i) }
    println("stuck: gates ${stuck.parked_gates}, tokens ${stuck.parked_tokens}")
}
"#;

const ON_IDLE_OUTPUT: &str =
    "settled: gates 0, tokens 0\nstuck: gates 1, tokens 1\n";

#[test]
fn rustc_compiles_and_runs_a_quiescence_hook() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ON_IDLE)]);
    run_rust_files(&files, "on-idle", ON_IDLE_OUTPUT);
}

/// [actor-on-idle] [rs-actor] What `on_idle` lowers to, on `watch`'s
/// precedent: the scheduler call **plus the `Idle` builder** the registration
/// site closes over, since the runtime holds two counts and cannot construct a
/// Salvo struct.
#[test]
fn a_quiescence_hook_lowers_to_a_scheduler_call_with_an_idle_builder() {
    let files = generate(&[("main.sv", ON_IDLE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains(
            "crate::scheduler::salvo_on_idle(p, i, |__gates, __tokens| \
             Box::new(Idle { parked_gates: __gates, parked_tokens: __tokens }))"
        ),
        "the idle registration or its `Idle` builder is missing:\n{}",
        main.content
    );
}

/// [rs-actor] [actor-replyto] The lowering: a continuation enum beside the
/// message enum; the two generated fields on the handler; a `__dispatch`
/// factored out of `handle` so member invocation lives in one place; and a
/// `resume` that pops the slot, casts the answer to the target member's
/// *trailing* parameter type, and dispatches.
#[test]
fn a_parked_continuation_lowers_to_a_slot_table() {
    let files = generate_replyto_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("pub enum __Cont_Fetching {")
            && text.contains("Arrived(crate::scheduler::SalvoReply),"),
        "the continuation enum is missing, or its captures are wrong:\n{text}"
    );
    assert!(
        text.contains("__addr: Option<usize>,")
            && text.contains("__parked: std::collections::HashMap<u64, __Cont_Fetching>,"),
        "the handler's generated fields are missing:\n{text}"
    );
    assert!(
        text.contains("self.handler.__addr = Some(_ctx.addr);"),
        "the activation does not write its own address:\n{text}"
    );
    assert!(
        text.contains("crate::scheduler::salvo_mint(self.__addr")
            && text.contains("self.__parked.insert(__s,"),
        "the mint does not park a continuation:\n{text}"
    );
    assert!(
        text.contains("let Some(__cont) = self.handler.__parked.remove(&slot)")
            && text.contains("*value.downcast::<String>().expect(\"the awaited answer\")"),
        "`resume` does not dispatch the parked continuation:\n{text}"
    );
}

/// [actor-replyto] The two refusals this slice adds, both checker-side. A
/// handler that parks may only be **spawned** (user decision 2026-09-15): bound
/// with `use` its members run inline, so the answer would have no mailbox to
/// arrive on and no dispatcher to run it — the continuation would silently never
/// run. And a **remote** mint — `k` naming a member of another actor effect in
/// scope — is the generalized form of a later slice, named rather than
/// mis-resolved.
#[test]
fn the_replyto_refusals_are_errors() {
    let parking = expect_errors(REPLYTO_USED);
    assert!(
        parking
            .iter()
            .any(|e| e.contains("mints a continuation with `replyto`, so it can only be `spawn`ed")),
        "expected the parking-handler `use` refusal, got {parking:?}"
    );
    let remote = expect_errors(REMOTE_MINT);
    assert!(
        remote
            .iter()
            .any(|e| e.contains("minting toward another actor's member is not supported yet")),
        "expected the remote-mint refusal, got {remote:?}"
    );
}

const REPLYTO_USED: &str = r#"
actor effect Db {
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
}

actor effect Notices {
    send fn fetch(id: Int) => !id
    send fn arrived(id: Int, text: Str) => !id, !text
}

handler Rows() of Db {
    mailbox { capacity: 8 }

    send fn lookup(id: Int, out: Reply<Str>) { out.send("r") }
}

handler Fetching() [Db] of Notices {
    mailbox { capacity: 8 }

    send fn fetch(id: Int) {
        lookup(copy(id), replyto arrived(id))
    }
    send fn arrived(id: Int, text: Str) {}
}

fn main() [use, spawn] {
    let rows = spawn Rows() on pool(1)
    use rows
    use Fetching()
    fetch(9)
}
"#;

const REMOTE_MINT: &str = r#"
actor effect Db {
    send fn lookup(id: Int, out: Reply<Str>) => !id, !out
    send fn arrived(id: Int, text: Str) => !id, !text
}

actor effect Ask {
    send fn go(id: Int) => !id
}

handler Rows() of Db {
    mailbox { capacity: 8 }

    send fn lookup(id: Int, out: Reply<Str>) { out.send("r") }
    send fn arrived(id: Int, text: Str) {}
}

handler Asking() [Db] of Ask {
    mailbox { capacity: 4 }

    send fn go(id: Int) {
        lookup(copy(id), replyto arrived(id))
    }
}

fn main() [use, spawn] {
    let rows = spawn Rows() on pool(1)
    let a = spawn Asking() with rows on pool(1)
    a.go(1)
}
"#;

// ===== [actor-use-addr] the forwarding stub =====

/// [actor-use-addr] `use addr` binds an effect to a **stub** that sends to a
/// process, so a function declaring `[Log]` never learns that its capability is
/// a process — the point of the form, and the shape a dependent spawn will
/// reuse. Same program and same expected output on the Kotlin backend.
const ADDR_STUB: &str = r#"
actor effect Log {
    send fn note(what: Str) => !what
    send fn count(out: Reply<Int>) => !out
}

handler Counting() of Log {
    mailbox { capacity: 8 }

    seen: Int = 0

    send fn note(what: Str) {
        seen = seen + 1
    }

    send fn count(out: Reply<Int>) {
        out.send(seen)
    }
}

fn work() [Log] {
    note("a")
    note("b")
}

fn main() [use, spawn] {
    use StdOutConsole()
    let logger = spawn Counting() on pool(1)
    use logger
    work()
    let n = waitfor out: Reply<Int> {
        count(out)
    }
    println("noted ${n}")
}
"#;

fn generate_addr_stub_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", ADDR_STUB)])
}


/// [waitfor-effect] [waitfor-dedicated] [main-pool] The `waitfor` package, as a
/// program. Three things run here that could not be written before 2026-09-17:
/// a **handler** that waits (`[waitfor]` in its dependency list, so its members
/// may occupy the thread), the **dedicated placement** its spawn therefore
/// needs (`on thread()`, a `Dedicated Pool` the clause consumes), and an actor
/// on **`main`'s own pool** — a spawn with no `on` clause, whose activations
/// run on main's thread while main waits [waitfor-pump]. The Kotlin backend
/// asserts the same output.
const WAITFOR_PACKAGE: &str = r#"
actor effect TimerApi {
    send fn after(ms: Int, out: Reply<Int>) => !out
}

actor effect ClockApi {
    send fn now(out: Reply<Int>) => !out
}

actor effect Counter {
    send fn bump(n: Int) => !n
    send fn total(out: Reply<Int>) => !out
}

handler Timing(at: Int) of TimerApi {
    mailbox { capacity: 4 }

    send fn after(ms: Int, out: Reply<Int>) {
        out.send(at + ms)
    }
}

// The T-5 shape: a handler whose member *waits* for another actor's answer.
// It declares the capability, so every spawn of it must give it a thread.
handler Clocking() [TimerApi] of ClockApi {
    mailbox { capacity: 4 }

    send fn now(out: Reply<Int>) {
        let at = waitfor fired: Reply<Int> {
            after(0, fired)
        }
        out.send(at)
    }
}

handler Counting() of Counter {
    mailbox { capacity: 4 }

    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdOutConsole()

    let timer = spawn Timing(1000) on pool(1)
    // `thread()` is consumed here: one thread, one occupant.
    let clock = spawn Clocking() with timer on thread()
    let now = waitfor out: Reply<Int> {
        clock.now(out)
    }
    println("the clock says ${now}")

    // No `on`: the pool current here, of which `main` is the single worker.
    // These activations run during the `waitfor` below, on main's own thread.
    let counter = spawn Counting()
    counter.bump(2)
    counter.bump(3)
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
    println("the main pool's actor says ${sum}")
}
"#;

const WAITFOR_PACKAGE_OUTPUT: &str = "the clock says 1000\nthe main pool's actor says 5\n";

#[test]
fn rustc_compiles_and_runs_the_waitfor_package() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", WAITFOR_PACKAGE)]);
    run_rust_files(&files, "waitfor-package", WAITFOR_PACKAGE_OUTPUT);
}


/// [free-send-fn] [task-mint] [task-pool-inherit] The **task kernel**, as a
/// program: a free `send fn` that runs by being scheduled, an ordinary
/// function that mints a continuation for it and returns (no park, no block),
/// and the placement — inherited at one mint, written at the other. The
/// Kotlin backend asserts the same output.
const TASK_KERNEL: &str = r#"
actor effect Db {
    send fn query(id: Int, reply: Reply<Int>) => !reply
}

handler Rows() of Db {
    mailbox { capacity: 4 }

    send fn query(id: Int, reply: Reply<Int>) {
        reply.send(id * 10)
    }
}

// The free send fn: no return type, every parameter consumed, and it runs on a
// pool rather than in anyone's frame. Its trailing parameter is the answer.
send fn finish(label: Str, out: Reply<Str>, row: Int) => !label, !out, !row {
    out.send("${label}=${row}")
}

// An ordinary function — full features, synchronous — that wires future work
// and returns. It never parks and never blocks, and it is called identically
// from `main` and from anywhere else.
fn fetch(id: Int, out: Reply<Str>) [Db] -> None => !out {
    query(id, replyto finish("row", out))
}

fn fetch_on(id: Int, out: Reply<Str>, p: Pool) [Db] -> None => !out, p {
    query(id, replyto finish("placed", out) on p)
}

fn main() [use, spawn] {
    use StdOutConsole()
    let workers = pool(2)
    let db = spawn Rows() on workers
    use db

    // The continuation inherits `main`'s pool, so it runs on main's own thread
    // while main waits [waitfor-pump].
    println(waitfor a: Reply<Str> { fetch(7, a) })
    // And here it runs on the worker pool instead.
    println(waitfor b: Reply<Str> { fetch_on(4, b, workers) })
}
"#;

const TASK_KERNEL_OUTPUT: &str = "row=70
placed=40
";

#[test]
fn rustc_compiles_and_runs_the_task_kernel() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TASK_KERNEL)]);
    run_rust_files(&files, "task-kernel", TASK_KERNEL_OUTPUT);
}

/// [rs-task] The lowering: no continuation enum, no slot, no parked table —
/// the closure *is* the continuation, and the pool is the one the mint chose.
#[test]
fn a_task_mint_lowers_to_a_scheduled_closure() {
    let files = generate(&[("main.sv", TASK_KERNEL)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool()"),
        "an omitted `on` clause must inherit the current pool:\n{text}"
    );
    assert!(
        text.contains("Box::new(move |__v| finish("),
        "the continuation must be a moved one-shot closure:\n{text}"
    );
    assert!(
        !text.contains("__parked.insert"),
        "a task mint parks nothing: the closure is the continuation:\n{text}"
    );
}

#[test]
fn rustc_compiles_and_runs_a_stub_bound_effect() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate_addr_stub_demo();
    run_rust_files(&files, "addr-stub", "noted 2\n");
}

#[test]
fn a_stub_implements_the_effect_by_sending() {
    let files = generate_addr_stub_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains("pub struct __Stub_Log {")
            && main.content.contains("impl Log for __Stub_Log"),
        "the forwarding stub is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Stub_Log::new("),
        "`use addr` does not build the stub:\n{}",
        main.content
    );
}


/// [is-bind-once] The defect this rule closed, as a program: an `is` binding
/// whose subject is a **call** must evaluate it exactly once. Until 2026-09-16
/// both backends emitted the subject twice — once for the test, once for the
/// payload — so this loop called `remove_first` twice per turn and *silently
/// dropped* every other element (and, for a linear element, its obligation).
///
/// Four shapes, because the emitters have four paths: a `while` and an `if` in
/// statement position, and both again in value position. The expected output is
/// identical on the Kotlin backend.
const IS_BIND_ONCE: &str = r#"
fn drain_ints(xs: Mut List<Int>) [Console] -> None => xs: Mut {
    // statement `while`: one call per turn, so every element is seen
    while remove_first(xs) is Int n {
        println("  took ${n}")
    }
}

fn first_or(xs: Mut List<Int>, fallback: Int) [] -> Int => xs: Mut, fallback {
    // value-position `if`: one call, and the binding is that call's value
    return if remove_first(xs) is Int n { n } else { fallback }
}

fn sum_down(xs: Mut List<Int>) [] -> Int => xs: Mut {
    let total = 0
    // value-position `while`: the loop's value is its last body value
    let last = while remove_first(xs) is Int n {
        total = total + n
        n
    } else { 0 }
    discard(last)
    return total
}

fn main() [use] {
    use StdOutConsole()
    let a: Mut List<Int> = mut_list_of(1, 2, 3)
    drain_ints(a)
    println("left ${size(a)}")

    let b: Mut List<Int> = mut_list_of(7, 8)
    println("first ${first_or(b, 0)} then ${size(b)}")

    let c: Mut List<Int> = mut_list_of(1, 2, 3, 4)
    println("sum ${sum_down(c)} left ${size(c)}")

    // statement `if`: the call happens once, so the list loses exactly one
    let d: Mut List<Int> = mut_list_of(5, 6)
    if remove_first(d) is Int n {
        println("one ${n} left ${size(d)}")
    }
}
"#;

const IS_BIND_ONCE_OUTPUT: &str = "  took 1\n  took 2\n  took 3\nleft 0\n\
first 7 then 1\nsum 10 left 0\none 5 left 1\n";

#[test]
fn rustc_compiles_and_runs_is_bindings_over_calls() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", IS_BIND_ONCE)]);
    run_rust_files(&files, "is-bind-once", IS_BIND_ONCE_OUTPUT);
}

/// [is-bind-once] …and the shape it lowers to: one temporary, read by the test
/// and by the binding, with the `while` becoming a `loop` so the evaluation
/// happens once *per iteration*.
#[test]
fn an_is_binding_over_a_call_hoists_its_subject() {
    let files = generate(&[("main.sv", IS_BIND_ONCE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("loop {") && text.contains("if !(__is1.is_some()) {"),
        "the while did not become a test-inside loop:\n{text}"
    );
    assert!(
        !text.contains("while ({ let __l"),
        "a subject is still emitted inside the loop condition:\n{text}"
    );
    // The binding reads the temporary, not a second call.
    assert!(
        text.contains("let mut n = __is1.unwrap()") || text.contains("let mut n = *__is1"),
        "the binding does not read the hoisted temporary:\n{text}"
    );
}

// ===== the checked-in examples (examples/README.md) =====

/// The repository's `examples/` directory.
fn examples_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Every example directory, by name, in a stable order.
fn example_names() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(examples_dir())
        .expect("examples/ is readable")
        .filter_map(|e| {
            let e = e.expect("a readable entry");
            let name = e.file_name().to_string_lossy().to_string();
            e.path().join("salvo").is_dir().then_some(name)
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no examples found");
    out
}

/// Emits one example's Rust, exactly as `salvo compile` would.
fn emit_example(example: &str) -> Vec<salvo_backend_rust::EmittedFile> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "rs", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let src = examples_dir().join(example).join("salvo");
    let errors = sources.add_dir(&src, "rs", false, false);
    assert!(errors.is_empty(), "failed to read {example}: {errors:?}");
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    };
    salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("`examples/{example}` no longer compiles:\n{}", errors.join("\n"))
    })
}

/// The generated tree checked in beside each example is what the
/// compiler writes *today* — the convention every example README rests on
/// ("stale generated code is worse than none: it is read as what the compiler
/// does"). A pure text comparison, so it needs no toolchain — and it catches an
/// example whose *source* stopped checking, since emission then fails outright.
///
/// Added 2026-09-16, after `examples/effects/` was found broken since
/// 2026-09-15: nothing in the suite read the examples at all.
#[test]
fn every_examples_checked_in_rust_is_current() {
    for example in example_names() {
        let files = emit_example(&example);
        let root = examples_dir().join(&example).join("rust");
        for file in &files {
            let path = root.join(&file.rel_path);
            let found = std::fs::read_to_string(&path).unwrap_or_else(|_| {
                panic!(
                    "examples/{example}/rust/{} is missing: regenerate with \
                     `cargo run -- compile --backend rust --src examples/{example}/salvo \
                     --target examples/{example}/rust`",
                    file.rel_path.display()
                )
            });
            if found != file.content {
                // A whole generated file in the failure message is unreadable;
                // the first differing line is what a reader needs.
                let at = found
                    .lines()
                    .zip(file.content.lines())
                    .position(|(a, b)| a != b)
                    .map(|i| i + 1);
                panic!(
                    "examples/{example}/rust/{} is stale (first difference at line {}): \
                     regenerate with `cargo run -- compile --backend rust --src \
                     examples/{example}/salvo --target examples/{example}/rust`",
                    file.rel_path.display(),
                    at.map(|l| l.to_string()).unwrap_or_else(|| "end of file".into())
                );
            }
        }
    }
}

/// …and each example still runs to the stdout checked in beside it,
/// which is also the parity assertion: the Kotlin backend asserts the same
/// bytes from the same file.
#[test]
fn every_example_runs_to_its_expected_output() {
    for example in example_names() {
        let files = emit_example(&example);
        let expected =
            std::fs::read_to_string(examples_dir().join(&example).join("expected.txt"))
                .unwrap_or_else(|_| panic!("examples/{example}/expected.txt is missing"));
        run_rust_files(&files, &format!("example-{example}"), &expected);
    }
}

// ===================== time [time-types] [time-timer] =====================

/// [time-types] [time-clock] [time-ticker] [mod-import-module] The time surface
/// end to end: one `import time` for the module, spans built and read back,
/// both timelines' `between`, the epoch bridge, and the two clock effects.
const TIME_SURFACE: &str = r#"
import time

fn main() [use] -> None {
    use StdOutConsole()
    use DefaultTicker()
    use DefaultClock()
    let d = millis(1500)
    println("span=${d}")
    println("sum=${plus(d, seconds(1))}")
    println("scaled=${times(d, 2)}")
    println("abs=${abs(minus(seconds(1), seconds(3)))}")
    println("millis=${to_millis(d)}")
    println("units=${micros(250)} ${nanos(37)} ${minutes(2)} ${hours(1)}")
    let t0 = tick()
    let t1 = plus(t0, micros(250))
    println("ticks=${between(t0, t1)} ${between(t1, t0)}")
    let i = epoch_milli(1700000000000)
    println("epoch=${to_epoch_milli(i)} ${to_epoch_second(i)}")
    println("shifted=${to_epoch_milli(plus(i, seconds(2)))}")
    println("wall=${to_epoch_nano(now()) > 0L}")
    println("mono to wall=${to_epoch_nano(to_instant(t0)) > 0L}")
    println("round trip=${to_epoch_nano(to_instant(to_tick(i))) == to_epoch_nano(i)}")
    println("ordered=${t1 > t0} ${d == millis(1500)}")
}
"#;

const TIME_SURFACE_OUTPUT: &str = "span=1500ms\nsum=2500ms\nscaled=3s\nabs=2s\nmillis=1500\n\
                                   units=250us 37ns 120s 3600s\nticks=250us -250us\n\
                                   epoch=1700000000000 1700000000\nshifted=1700000002000\n\
                                   wall=true\nmono to wall=true\nround trip=true\n\
                                   ordered=true true\n";

/// [time-timer] A real deadline: `DefaultTimer` spawned like any actor, a
/// `waitfor` bridging the fire into `main`, and the fire's `at` proving the
/// deadline was honoured. Asserted as *inequalities*, since a real clock cannot
/// promise an exact number.
const TIME_TIMER: &str = r#"
import time

fn main() [use, spawn] -> None {
    use StdOutConsole()
    use DefaultTicker()
    let timers = spawn DefaultTimer() on pool(1)
    let started = tick()
    let first = waitfor fired: Reply<Fired> {
        timers.after(millis(50), fired)
    }
    let waited = between(started, first.at)
    println("waited enough: ${to_millis(waited) >= 50}")
    let second = waitfor again: Reply<Fired> {
        timers.after(millis(10), again)
    }
    println("in order: ${second.at > first.at}")
}
"#;

const TIME_TIMER_OUTPUT: &str = "waited enough: true\nin order: true\n";

/// [time-manual] [effect-handler-multi] The pure-Salvo fake: `ManualTime` wears
/// `Timer` and `TimerCtl`, the code under test holds only the `Timer` addr, and
/// the test sequences itself with `on_idle` before advancing virtual time — so
/// a two-second deadline is observed in microseconds, deterministically.
const TIME_MANUAL: &str = r#"
import time

actor effect Sleeper {
    send fn nap(wait: Duration, done: Reply<Str>) => !wait, !done
    send fn woke(done: Reply<Str>, f: Fired) => !done, !f
}

handler Napping() [Timer] of Sleeper {
    mailbox { capacity: 8 }

    send fn nap(wait: Duration, done: Reply<Str>) => !wait, !done {
        after(wait, replyto woke(done))
    }

    send fn woke(done: Reply<Str>, f: Fired) => !done, !f {
        send(done, "woke at ${to_millis(between(Tick {nanos: 0L}, f.at))}ms")
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let sleeper = spawn Napping() with timer on p
    let answer = waitfor result: Reply<Str> {
        sleeper.nap(seconds(2), result)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(seconds(2))
    }
    println(answer)
    // Virtual time *accumulates*: the second nap measures from where the first
    // advance left `now`, so a one-second deadline answers at three seconds.
    let second = waitfor later: Reply<Str> {
        sleeper.nap(seconds(1), later)
        waitfor drained: Reply<Idle> {
            on_idle(p, drained)
        }
        ctl.advance(seconds(1))
    }
    println(second)
}
"#;

const TIME_MANUAL_OUTPUT: &str = "woke at 2000ms\nwoke at 3000ms\n";

/// [time-coupling] The three test postures of the coupling stance, in one
/// program: a decision that takes its times as *data* and so needs no clock at
/// all; a scripted `Ticker` for code that reads one locally; and the unified
/// test clock — a `Ticker` whose reading is a zero deadline on the very timer
/// the test advances, so a measurement taken across virtual time is exact.
///
/// The last one is what steps 1 and 5 of the second sequence were owed for: it
/// needs `[waitfor]` as a propagating capability and `ManualTime` firing an
/// already-due deadline at registration.
const TIME_COUPLING: &str = r#"
import time

// Time as data: no effect, so nothing to fake.
fn verdict(started: Tick, at: Tick, budget: Duration) [] -> Str {
    if between(started, at) > budget {
        return "late"
    }
    return "in time"
}

// A scripted clock: readings from the constructor, half a second apart.
handler SteppingTicker(step: Duration) of Ticker {
    at: Long = 0

    fn tick() -> Tick {
        at = at + step.nanos
        return Tick {nanos: at}
    }
}

// The unified test clock: a reading is a deadline of zero, so the answer is the
// timer's own virtual now. The timer arrives as a value, not a dependency — a
// handler with dependencies of its own cannot be built in a spawn `with` clause.
handler TestTicker(timer: Addr<Timer>) of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> {
            timer.after(nanos(0), answer)
        }
        return fired.at
    }
}

actor effect Sleeper {
    send fn nap(wait: Duration, out: Reply<Str>) => !wait, !out
    send fn woke(started: Tick, out: Reply<Str>, f: Fired) => !started, !out, !f
}

handler Napping() [Timer, Ticker] of Sleeper {
    mailbox { capacity: 8 }

    send fn nap(wait: Duration, out: Reply<Str>) {
        after(wait, replyto woke(tick(), out))
    }

    send fn woke(started: Tick, out: Reply<Str>, f: Fired) {
        send(out, "napped ${elapsed(started)}, fired at ${to_millis(between(Tick {nanos: 0}, f.at))}ms")
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()
    println("data: ${verdict(Tick {nanos: 0}, Tick {nanos: 1000000000}, millis(1500))}")
    use SteppingTicker(millis(500))
    let started = tick()
    println("scripted: ${to_millis(elapsed(started))} ${to_millis(elapsed(started))}")

    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let sleeper = spawn Napping() with timer, TestTicker(timer) on thread()
    let napped = waitfor answer: Reply<Str> {
        sleeper.nap(seconds(2), answer)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(seconds(2))
    }
    println("unified: ${napped}")
}
"#;

const TIME_COUPLING_OUTPUT: &str = "data: in time\nscripted: 500 1000\n\
                                    unified: napped 2s, fired at 2000ms\n";


/// [time-types] [mod-import-module] The surface compiles and runs, and both
/// backends print the same text.
#[test]
fn rustc_compiles_and_runs_the_time_surface() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TIME_SURFACE)]);
    run_rust_files(&files, "time-surface", TIME_SURFACE_OUTPUT);
}

/// [time-timer] A real deadline fires, and the fires are ordered.
#[test]
fn rustc_compiles_and_runs_a_real_timer() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TIME_TIMER)]);
    run_rust_files(&files, "time-timer", TIME_TIMER_OUTPUT);
}

/// [time-manual] Virtual time, in pure Salvo, through a two-face handler.
#[test]
fn rustc_compiles_and_runs_manual_time() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TIME_MANUAL)]);
    run_rust_files(&files, "time-manual", TIME_MANUAL_OUTPUT);
}

/// [time-coupling] The three postures, and the unified test clock in particular:
/// a measurement taken across virtual time is exact, because one clock is behind
/// both the deadline and the reading.
#[test]
fn rustc_compiles_and_runs_the_coupling_postures() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TIME_COUPLING)]);
    run_rust_files(&files, "time-coupling", TIME_COUPLING_OUTPUT);
}

/// [time-timer] [rs-time] What a deadline registration lowers to: the scheduler
/// call plus the `Fired` builder the site closes over — `watch`/`on_idle`'s
/// shape, since the runtime holds a number and cannot construct a Salvo struct.
#[test]
fn a_deadline_lowers_to_a_scheduler_call_with_a_fired_builder() {
    let files = generate(&[("main.sv", TIME_TIMER)]);
    let time = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "time.rs")
        .expect("time.rs");
    assert!(
        time.content.contains("crate::scheduler::salvo_after(")
            && time
                .content
                .contains("|__at| Box::new(Fired { at: Tick { nanos: __at } })"),
        "expected the deadline lowering in:\n{}",
        time.content
    );
    // [time-types] [rs-time] And the clock readings come from the time runtime,
    // which travels with the scheduler.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "hosttime.rs"),
        "expected hosttime.rs to be emitted"
    );
}

// ===== [rs-opt-borrow] an owned optional local, read twice =====

/// [rs-opt-borrow] Two `!`s on one optional **local** are two *reads*: the
/// checker allows them (reading an optional is free), so the lowering must not
/// move the value the first time. The form is the one a narrowed read already
/// takes — borrow the `Option`, unwrap the borrow, clone the payload (found
/// 2026-09-23 writing `std/test.test.sv`, which is the shape below).
const REREAD_OPTIONAL: &str = r#"
fn len_of(s: Str) -> Int => s {
    return size(s)
}

fn shout(s: Str) -> Str => !s {
    return s
}

fn main() [use] {
    use StdOutConsole()
    let maybe: Str? = "hello"
    println("once ${size(maybe!)}")
    println("twice ${size(maybe!)}")
    println("kept ${len_of(maybe!)}")
    println("owned ${shout(maybe!)}")
    let n: Int? = 3
    println("copy ${n! + n!}")
}
"#;

#[test]
fn an_owned_optional_local_is_read_through_a_borrow() {
    let files = generate(&[("main.sv", REREAD_OPTIONAL)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // Non-Copy payload: borrowed, unwrapped, cloned — so the second read still
    // has something to read.
    // Four reads of the same local, every one of them borrowing the `Option`.
    assert_eq!(
        main.matches("maybe.as_ref().expect(\"salvo: value is absent").count(),
        4,
        "expected every read to borrow:\n{main}"
    );
    assert!(!main.contains("maybe.expect("), "a read moved the local:\n{main}");
    // A Copy payload needs none of it: the `Option` is Copy, so `.expect` moves
    // nothing and the shorter form stands.
    assert_eq!(
        main.matches("n.expect(\"salvo: value is absent").count(),
        2,
        "expected the Copy payload to stay direct:\n{main}"
    );
    // [rs-read-mode] A **kept** parameter is a read: the unwrap answers the
    // `&String` straight into the position — no clone, and no second `&`.
    assert!(
        main.contains("len_of(maybe.as_ref().expect(\"salvo: value is absent"),
        "a kept argument cloned:\n{main}"
    );
    // …and a *consuming* one still gets a value of its own.
    assert!(
        main.contains("shout(maybe.as_ref().expect(\"salvo: value is absent at main:16:28\").clone())"),
        "a consumed argument did not get an owned value:\n{main}"
    );
    run_rust_files(
        &files,
        "reread-optional",
        "once 5\ntwice 5\nkept 5\nowned hello\ncopy 6\n",
    );
}

/// [rs-read-mode] An **intrinsic** argument takes its mode from the
/// intrinsic's own declaration, not from the position the call sits in:
/// `contains(str, needle) => str, needle` keeps both, so a `!` read of a local
/// hands the lowering the reference it already has. `add(list: Mut List<T>,
/// elem: T) => !elem` keeps the *list* and consumes the element — so the list
/// is reached mutably and the element arrives owned.
const INTRINSIC_OPTIONAL_ARGS: &str = r#"
fn main() [use] {
    use StdOutConsole()
    let maybe: Str? = "hello"
    if contains(maybe!, "ell") { println("found") }
    println("upper ${to_upper(maybe!)}")
    let xs: Mut List<Int>? = mut_list_of(1)
    add(xs!, 3)
    println("xs ${size(xs!)}")
}
"#;

#[test]
fn an_intrinsic_argument_takes_the_intrinsics_own_mode() {
    let files = generate(&[("main.sv", INTRINSIC_OPTIONAL_ARGS)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // A kept parameter: the unwrap's `&String` goes straight into the template.
    assert!(
        main.contains(
            "if maybe.as_ref().expect(\"salvo: value is absent at main:5:17\").contains("
        ),
        "a kept intrinsic argument cloned:\n{main}"
    );
    assert!(
        !main.contains(".clone().contains("),
        "a kept intrinsic argument cloned:\n{main}"
    );
    assert!(
        !main.contains(".clone().to_uppercase()") && !main.contains(".clone().len()"),
        "a kept intrinsic receiver cloned:\n{main}"
    );
    // [rs-narrow-mut] A `Mut` parameter reaches the payload *mutably*. Rendered
    // owned (as it was until 2026-09-23) this read
    // `xs.as_ref().expect(…).clone().push(3)`, which appends to the clone and
    // prints `xs 1` where Kotlin prints `xs 2`.
    assert!(
        main.contains("xs.as_mut().expect(\"salvo: value is absent at main:8:9\").push(3)"),
        "a `Mut` intrinsic argument did not reach the storage:\n{main}"
    );
    run_rust_files(
        &files,
        "intrinsic-optional-args",
        "found\nupper HELLO\nxs 2\n",
    );
}

/// [rs-read-mode] A **narrowed** read is the same question as `!`, one path
/// over: `narrow_unwrap` renders through a borrow either way, and the
/// `.clone()` is the position's to ask for.
const NARROWED_READS: &str = r#"
fn len_of(s: Str) -> Int => s { return size(s) }
fn shout(s: Str) -> Str => !s { return s }

struct Box { label: Str? }

fn main() [use] {
    use StdOutConsole()
    let name: Str? = "hello"
    if name is Str {
        println("size ${size(name)}")
        println("kept ${len_of(name)}")
        let held: Str = name
        println("held ${held}")
        println("owned ${shout(name)}")
    }
    let b = Box { label: "boxed" }
    if b.label is Str {
        println("field ${len_of(b.label)}")
    }
    let n: Int? = 4
    if n is Int {
        println("copy ${n + n}")
    }
}
"#;

#[test]
fn a_narrowed_read_borrows_unless_the_position_owns() {
    let files = generate(&[("main.sv", NARROWED_READS)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // A kept intrinsic parameter and a kept declared one: the unwrap's `&String`
    // stands as the argument — no clone, and no `&` in front of it either.
    assert!(
        main.contains("(name.as_ref().unwrap().chars().count() as i32)"),
        "a narrowed intrinsic argument cloned:\n{main}"
    );
    assert!(
        main.contains("len_of(name.as_ref().unwrap())"),
        "a narrowed kept argument cloned:\n{main}"
    );
    // A narrowed **field** read reaches a `&T` position the same way.
    assert!(
        main.contains("len_of(b.label.as_ref().unwrap())"),
        "a narrowed field argument cloned:\n{main}"
    );
    // The owning positions still get a value: a `let` and a consuming parameter.
    assert!(
        main.contains("let mut held: String = name.as_ref().unwrap().clone()"),
        "a binding took a reference:\n{main}"
    );
    assert!(
        main.contains("shout(name.as_ref().unwrap().clone())"),
        "a consumed argument took a reference:\n{main}"
    );
    // A Copy payload is copied out of the representation in either mode.
    assert!(
        main.contains("n.unwrap() + n.unwrap()"),
        "a Copy narrowed read grew an `as_ref`:\n{main}"
    );
    run_rust_files(
        &files,
        "narrowed-reads",
        "size 5\nkept 5\nheld hello\nowned hello\nfield 5\ncopy 8\n",
    );
}

// ===== mutable element handles [proj-mut] [rs-elem-mut] =====

/// [proj-mut] Mutation through element handles of a `List<Mut T>`: a
/// statement-scoped one (`bump(get(xs, 0)!)` → a `get_mut` splice), a bound
/// one in a callee (a captured-index virtual binding), and reads afterwards
/// observing both mutations — the aliasing semantics Kotlin gets natively.
const ELEM_MUT_DEMO: &str = r#"
struct Counter canbe Mut {
    n: Int
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

fn poke(xs: List<Mut Counter>) -> None {
    let h = get(xs, 1)!
    h.n = h.n + 10
    bump(h)
}

fn main() [use] {
    use StdOutConsole()
    let xs: List<Mut Counter> = list_of(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    bump(get(xs, 0)!)
    poke(xs)
    let a = get(xs, 0)!
    let b = get(xs, 1)!
    println("${a.n} ${b.n}")
}
"#;

/// [rs-elem-mut] The three renderings: the container parameter arrives
/// `&mut` (element-level `Mut` lends), the statement-scoped handle splices
/// `get_mut`, and the bound handle is a captured index re-materialized at
/// every use — never a bound `&mut`.
#[test]
fn elem_mut_handles_render_as_get_mut_and_captured_indices() {
    let files = generate(&[("main.sv", ELEM_MUT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn poke(xs: &mut Vec<Counter>)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("let __h0 = (1) as usize;"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("xs[__h0].n = xs[__h0].n + 10;"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("bump(&mut xs[__h0]);"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains(".get_mut((0) as usize)"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_elem_mut_handles() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ELEM_MUT_DEMO)]);
    run_rust_files(&files, "elem_mut", "2 13\n");
}

/// [rs-loc] ④a slice 5 — a **search loop** whose element binding is
/// returned: inside the locator variant the `for` over a list becomes an
/// *indexed* loop and the binding a captured-index handle, so the found
/// **position** travels out. The shape a `&mut` return could never take
/// (the NLL/pass-hidden-position case).
const SEARCH_LOOP_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn wounded(es: List<Mut Entity>) -> (proj(es) Mut Entity)? {
    for e in es {
        if e.hp < 10 {
            return e
        }
    }
    return None
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 50 }, Mut Entity { hp: 3 })
    heal(wounded(es)!)
    println("${get(es, 0)!.hp} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn a_search_loop_returns_the_found_position() {
    let files = generate(&[("main.sv", SEARCH_LOOP_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn wounded__loc(es: &Vec<Entity>) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("for __li0 in 0..es.len()"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("return Some(__li0);"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_search_loop_lender() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", SEARCH_LOOP_DEMO)]);
    run_rust_files(&files, "search_loop", "50 13\n");
}

/// [canbe-entry] [rs-loc] ④b — a callee declaring `=> a canbe d` takes
/// two element handles that **may be the same element**: the covered
/// positions render as one shared anchor plus a locator each (two `&mut`
/// into one container cannot coexist), and the same-call rule stands down
/// without any disjointness proof.
const CANBE_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int, energy: Int }

fn attack(a: Mut Entity, d: Mut Entity) -> None
=> a canbe d, a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return None
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(
        Mut Entity { hp: 10, energy: 5 },
        Mut Entity { hp: 20, energy: 8 })
    let i = 0
    let j = 1
    if i is Idx(es) {
        if j is Idx(es) {
            attack(get(es, i), get(es, j))
            // …and the aliasing case the entry exists for: one element,
            // both handles.
            attack(get(es, i), get(es, i))
        }
    }
    println("${get(es, 0)!.hp} ${get(es, 0)!.energy} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn covered_positions_render_as_anchor_and_locators() {
    let files = generate(&[("main.sv", CANBE_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content
            .contains("pub fn attack(__anchor: &mut Vec<Entity>, __c0: usize, __c1: usize)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("__anchor[__c0].energy = __anchor[__c0].energy - 1;"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("attack(&mut es,"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_covered_call() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", CANBE_DEMO)]);
    run_rust_files(&files, "canbe_covered", "8 3 18\n");
}

/// [elem-distinct] [rs-elem-mut] The distinct-pair shapes: a proven pair of
/// statement-scoped handles in one call, a proven pair of *bound* handles in
/// one call, and interleaved mutation through two bound handles — the checker
/// spares the sibling, the emitter splits the container once per pair call.
const DISTINCT_PAIR_DEMO: &str = r#"
struct Entity canbe Mut {
    hp: Int,
    energy: Int
}

fn attack(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return None
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(
        Mut Entity { hp: 10, energy: 5 },
        Mut Entity { hp: 20, energy: 8 })
    let i = 0
    let j = 1
    if j is NotEq(i) {
        attack(get(es, i)!, get(es, j)!)
        let a = get(es, i)!
        let d = get(es, j)!
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        attack(a, d)
    }
    println("${get(es, 0)!.hp} ${get(es, 0)!.energy} ${get(es, 1)!.hp} ${get(es, 1)!.energy}")
}
"#;

/// [rs-elem-mut] A proven pair call renders as one `salvo_pair_mut`
/// preamble (a `split_at_mut`, no aliasing) whose two `&mut` halves are the
/// call's arguments — for the temporary shape and the bound-handle shape
/// alike [rs-runtime-source].
#[test]
fn a_distinct_pair_call_splits_the_container_once() {
    let files = generate(&[("main.sv", DISTINCT_PAIR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    let splices = main.content.matches("salvo_pair_mut(&mut es[..],").count();
    assert_eq!(splices, 2, "one preamble per pair call:\n{}", main.content);
    assert!(
        main.content.contains("attack(__pm0, __pm1);"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_distinct_pair_calls() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", DISTINCT_PAIR_DEMO)]);
    run_rust_files(&files, "distinct_pair", "11 3 17 8\n");
}

/// [rs-loc] Locator-specialized lending (④a slice 1, re-founding ③'s
/// mechanism): a user-written accessor whose result some call site
/// mutates gets a demand-driven `__loc` emission beside the read one,
/// and the C family is ordinary std Salvo riding the same mechanism.
const LEND_MUT_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (proj(es) Mut Entity)? {
    return get(es, 0)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal(front(es)!)
    let hp = front(es)!.hp
    println("${hp} ${get(es, 1)!.hp}")
}
"#;

/// [rs-loc] Both emissions exist — the read one untouched, the locator
/// variant answering position data from a *read-mode* search — and only
/// the mutable-use call site takes the variant, materializing
/// `&mut anchor[loc]` in a block whose read borrow ends before the write
/// borrow begins.
#[test]
fn a_mut_used_lender_gets_a_demand_driven_locator_variant() {
    let files = generate(&[("main.sv", LEND_MUT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn front(es: &mut Vec<Entity>) -> Option<&Entity>"),
        "{}",
        main.content
    );
    assert!(
        main.content
            .contains("pub fn front__loc(es: &Vec<Entity>) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("heal({ let __l0 = front__loc(&es)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("&mut es[__l0] })"),
        "{}",
        main.content
    );
    // The read call site keeps the read emission.
    assert!(
        main.content.contains("front(&mut es)"),
        "{}",
        main.content
    );
}

/// [rs-loc] ④a slice 2 — the bound-mint lift: a handle from a user
/// accessor binds (①'s "direct `get` only" cut widened), re-materializes
/// per use, and tolerates container *reads* between uses — the exact
/// shape a bound `&mut` could never survive (E0502).
const BOUND_ACCESSOR_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (proj(es) Mut Entity)? {
    return get(es, 0)
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    let boss = front(es)!
    boss.hp = boss.hp + 100
    let n = size(es)
    boss.hp = boss.hp + n
    println("${get(es, 0)!.hp} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn a_bound_accessor_handle_is_a_captured_locator() {
    let files = generate(&[("main.sv", BOUND_ACCESSOR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("let __h0 = front__loc(&es)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("es[__h0].hp = es[__h0].hp"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_bound_accessor_handle() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", BOUND_ACCESSOR_DEMO)]);
    run_rust_files(&files, "bound_accessor", "107 7\n");
}

#[test]
fn rustc_compiles_and_runs_a_mut_lending_accessor() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LEND_MUT_DEMO)]);
    run_rust_files(&files, "lend_mut", "15 7\n");
}

/// [col-update] The C family end to end: `update` writes in place through
/// the callback's handle, `update2` takes the proven pair, and the
/// `preserve Idx` promise keeps the trailing reads total.
const UPDATE_FAMILY_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 10 }, Mut Entity { hp: 20 })
    let i = 0
    let j = 1
    if i is Idx(es) {
        if j is Idx(es) {
            update(es, i, (e: Mut Entity) -> { e.hp = e.hp + 1 })
            if j is NotEq(i) {
                update2(es, i, j, (a: Mut Entity, b: Mut Entity) -> {
                    a.hp = a.hp + 100
                    b.hp = b.hp + 200
                })
            }
            println("${get(es, i).hp} ${get(es, j).hp}")
        }
    }
}
"#;

#[test]
fn rustc_compiles_and_runs_the_update_family() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", UPDATE_FAMILY_DEMO)]);
    run_rust_files(&files, "update_family", "111 220\n");
}

/// [rs-loc] ④a slice 3 — a **lending fn value** serves a `Mut`
/// position: the closure answers a locator (position data crosses the
/// boundary where a `&mut` could not), and the use site materializes. The
/// `?at` idiom — a generic accessor supplied by the caller, the house
/// pattern that pierces opacity — is the acceptance test.
const LENDING_FN_VALUE_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn bump_at(es: List<Mut Entity>, i: Int,
           at: (c: List<Mut Entity>, k: Int) -> proj(c) Mut Entity?) -> None {
    heal(at(es, i)!)
    return None
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    bump_at(es, 1, (c: List<Mut Entity>, k: Int) -> get(c, k))
    println("${get(es, 0)!.hp} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn a_lending_fn_value_renders_as_a_locator_closure() {
    let files = generate(&[("main.sv", LENDING_FN_VALUE_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("-> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("&mut es[__l0] }") || main.content.contains("&mut es[__l1] }"),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_a_lending_fn_value() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LENDING_FN_VALUE_DEMO)]);
    run_rust_files(&files, "lending_fn_value", "5 17\n");
}

/// [col-locate] [rs-loc] ④a slice 3′ — std's `Locate` bundle: a generic
/// algorithm hands out mutable handles because the caller supplies `at`,
/// and the implicit renders as a **locator** closure (position data out,
/// read-mode parameters) with the handle materialized at the use site.
const LOCATE_BUNDLE_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn heal_at<L>(c: List<Mut Entity>, l: L, ?Locate<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l)!)
    return None
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal_at(es, 1)
    println("${get(es, 0)!.hp} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn a_locate_implicit_renders_as_a_locator_closure() {
    let files = generate(&[("main.sv", LOCATE_BUNDLE_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("at: &mut dyn FnMut(&Vec<Entity>, &L) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("at__loc("),
        "{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_the_locate_bundle() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LOCATE_BUNDLE_DEMO)]);
    run_rust_files(&files, "locate_bundle", "5 17\n");
}

/// [rs-loc] ④a slice 4 — an **effect member** lends a mutable handle: the
/// trait carries both faces (the read one, explicitly lifetime-tagged so
/// the borrow ties to the container rather than to `&mut self`, and the
/// `__loc` one), every handler and adapter implements both, and a `Mut`
/// position routes to the locator face.
const LENDING_MEMBER_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

effect Lender {
    fn lease(es: List<Mut Entity>) -> (proj(es) Mut Entity)? => es
}

handler FirstLender of Lender {
    fn lease(es: List<Mut Entity>) -> (proj(es) Mut Entity)? => es {
        return get(es, 0)
    }
}

fn run(es: List<Mut Entity>) [Lender] -> None {
    heal(lease(es)!)
    return None
}

fn main() [use] {
    use StdOutConsole()
    use FirstLender()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    run(es)
    println("${get(es, 0)!.hp} ${get(es, 1)!.hp}")
}
"#;

#[test]
fn a_lending_effect_member_carries_both_faces() {
    let files = generate(&[("main.sv", LENDING_MEMBER_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content
            .contains("fn lease<'a>(&mut self, es: &'a Vec<Entity>) -> Option<&'a Entity>;"),
        "{}",
        main.content
    );
    assert!(
        main.content
            .contains("fn lease__loc(&mut self, es: &Vec<Entity>) -> Option<usize>;"),
        "{}",
        main.content
    );
    assert!(main.content.contains("lease__loc(es)"), "{}", main.content);
}

#[test]
fn rustc_compiles_and_runs_a_lending_effect_member() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LENDING_MEMBER_DEMO)]);
    run_rust_files(&files, "lending_member", "15 7\n");
}


/// [rs-elem-mut] The v1 cut, loud: a proven pair call in a *value*
/// position is refused with a codegen error naming the remedy — never
/// silently wrong output [backend-never-wrong].
#[test]
fn a_distinct_pair_call_in_a_value_position_is_refused() {
    let src = r#"
struct Entity canbe Mut { hp: Int }

fn poke(a: Mut Entity, d: Mut Entity) -> Int => a: Mut, d: Mut {
    a.hp = a.hp + 1
    d.hp = d.hp + 1
    return a.hp
}

fn main() [use] {
    use StdOutConsole()
    let es: List<Mut Entity> = list_of(Mut Entity { hp: 1 }, Mut Entity { hp: 2 })
    let i = 0
    let j = 1
    if j is NotEq(i) {
        let x = poke(get(es, i)!, get(es, j)!)
        println("${x}")
    }
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let errors = salvo_backend_rust::emit_program(&program)
        .err()
        .expect("a value-position pair call must be a codegen error");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("give the call its own statement")),
        "got {errors:?}"
    );
}
