//! Rust codegen tests: golden snapshots of generated code, and (when
//! `rustc` is on PATH) full compile-and-run verifications with exact
//! stdout assertions — mirroring the Kotlin backend's kotlinc tests.

use std::path::Path;
use std::process::Command;

use salvo_core::{Program, SourceSet};

fn build_program(extra: &[(&str, &str)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "rs", true);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, content) in extra {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, content.to_string(), false);
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

struct Range : Yield<self, Int> {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> [] Range {
    return Range {start: start, end: end}
}

yield fn next(r: Range) -> Int {
    let i = copy(r.start)
    while i++ < r.end {
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
    let files = generate(&[("main.sv", src)]);
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
    assert!(main.content.contains("impl<T: Clone + 'static> Random<T> for CyclicRandom<T>"));
    // Effect deps as leading `&mut dyn` parameters.
    assert!(main.content.contains(
        "pub fn draw(random_i32: &mut dyn Random<i32>, random_string: &mut dyn Random<String>, console: &mut dyn Console)"
    ));
    // `use` instantiates handlers into `let mut` locals.
    assert!(main.content.contains("let mut console = StdOutConsole::new();"));
    assert!(// [effect-handler-generics] The handler is constructed *at* a type — the
    // turbofish is written even where rustc could have inferred it, since a
    // stateless generic handler gives it nothing to infer from.
    main.content.contains("let mut random_i32 = CyclicRandom::<i32>::new(vec![10, 20, 30]);"));
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
struct Range : Yield<self, Int> {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> [] Range {
    return Range {start: start, end: end}
}

yield fn next(r: Range) -> Int {
    let i = copy(r.start)
    while i++ < r.end {
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

    let found = for x in [0, 1, 2, 3, 4, 5] {
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
    assert!(main.content.contains("let mut __loop1: Option<i32> = None;"));
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
    let files = generate(&[("main.sv", S2_DEMO)]);
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
    let files = generate(&[("main.sv", S3_DEMO)]);
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
struct FileHandle : Linear<self> {
    fd: Int
}

fn close(x: FileHandle) -> [] None {}


fn open_file(path: Str) [Console] -> [] FileHandle {
    println("open ${path}")
    return FileHandle {fd: size(path)}
}

fn close_file(h: FileHandle) [Console] -> [] None {
    println("close fd=${h.fd}")
    close(h)
}

fn main() [use] -> [] None {
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
    // now the escape hatch for.
    assert!(main.content.contains("close(h)"), "generated:\n{}", main.content);
    assert!(main.content.contains("close(temp)"), "generated:\n{}", main.content);
    assert!(main.content.contains("drop(note)"), "generated:\n{}", main.content);
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

/// `<T canbe Linear>` in action: the opted std surface makes a linear
/// collection workflow legal end to end — construct empty, `add`
/// individually, `size`, and `discard` the (linear) collection.
const LINEAR_GENERICS_DEMO: &str = r#"
struct FileHandle : Linear<self> {
    fd: Int
}

fn close(x: FileHandle) -> [] None {}


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

// ===== L7b: Once fn types [once-fn] =====

/// A `Once` parameter accepts both a capture-consuming lambda (which is
/// `Once`-typed by construction) and a plain lambda (inverted
/// subtyping); the checker guarantees at most one call.
const ONCE_DEMO: &str = r#"
fn run_once(f: Once () [Console] -> None) {
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
    let files = generate(&[("main.sv", ONCE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("pub fn run_once(console: &mut dyn Console, f: impl FnOnce(&mut dyn Console))"),
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

fn describe(p: Person) -> [p] Str {
    if p.surname is Str {
        return "${p.name} ${p.surname}"
    }
    return p.name
}

fn main() [use] -> [] None {
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
        main.content.contains("p.address.city.as_ref().unwrap().clone()"),
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

fn main() [use] -> [] None {
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
fn main() [use] -> [] None {
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
    fn log(message: Str) -> [message] None
}

handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> [message] None {
        println("LOG: ${message}")
    }
}

fn work() [Logger] -> None {
    log("from work")
}

fn main() [use] -> [] None {
    use StdOutConsole
    use ConsoleLogger()
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
            && c.contains("fn log(&mut self, __fx: &mut dyn Console, message: &String)"),
        "expected the member bodies in a trait taking the fused dependency:\n{c}"
    );
    assert!(
        c.contains("pub fn work(__fx: &mut dyn Logger)"),
        "callers must not mention the dependency:\n{c}"
    );
    assert!(
        c.contains("__outer: &'a mut dyn Console") && c.contains("__h: __H,"),
        "expected a fusion chaining to the provider and owning the handler:\n{c}"
    );
    assert!(
        c.contains("let Self { __outer, __h } = self;")
            && c.contains("__Impl_ConsoleLogger::log(__h, &mut **__outer, message)"),
        "the forwarding impl must split `&mut self` into disjoint field \
         borrows before threading the dependency:\n{c}"
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
        !c.contains("__Fx_") && !c.contains("__Conj_"),
        "no fusion items should be generated:\n{c}"
    );
}

const NO_DEPS_DEMO: &str = r#"
effect Logger {
    fn log(message: Str) -> [message] None
}

handler PlainLogger of Logger {
    fn log(message: Str) -> [message] None { }
}

fn work() [Logger, Console] -> None {
    log("hi")
    println("there")
}

fn main() [use] -> [] None {
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

// ===== the fusion in anger [rs-effect-fusion] =====
// Handler state behind a dependency, a *two*-dependency handler whose
// second dependency is another handler's effect, a fn needing two effects
// calling one needing a subset, nested `use` scopes with the outer one used
// again afterwards, a `use` inside a fn that already has effects, and two
// effect calls in one expression (which must not borrow the fused value
// twice).

const FUSION_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> [] T
}

effect Counter {
    fn bump() -> [] None
    fn total() -> [] Int
}

effect Logger {
    fn log(message: Str) -> [message] None
}

effect Audit {
    fn note(message: Str) -> [message] None
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
    fn bump() -> [] None { n = n + 1 }
    fn total() -> [] Int { return n }
}

handler ConsoleLogger(console: Console) of Logger {
    seen: Int = 0
    fn log(message: Str) -> [message] None {
        seen = seen + 1
        println("LOG ${seen}: ${message}")
    }
}

handler CountingAudit(console: Console, counter: Counter) of Audit {
    fn note(message: Str) -> [message] None {
        bump()
        println("[${total()}] ${message}")
    }
}

fn shout(message: Str) [Console] -> [message] None {
    println("!! ${message}")
}

fn banner() [Console, Logger] -> [] None {
    log("banner")
    shout("done")
}

fn draw() [Console, Random<Int>, use] -> [] None {
    use MemCounter
    bump()
    println("drew ${next_random<Int>()} at ${total()}")
}

fn main() [use] -> [] None {
    use StdOutConsole
    use ConsoleLogger()
    banner()
    if true {
        use MemCounter
        use CountingAudit()
        note("inner")
        banner()
    }
    log("outer again")
    use CyclicRandom([10, 20, 30])
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
    // A fn needing two effects takes one *generic* fused value, so it can
    // forward to a callee needing a subset (`dyn` could not: upcasting
    // only reaches supertraits).
    assert!(
        c.contains("pub fn banner<__Fx: Console + Logger>(__fx: &mut __Fx)")
            && c.contains("shout(&mut *__fx,"),
        "expected a generic fused parameter forwarded to a smaller callee:\n{c}"
    );
    // Two dependencies need a Sized view over the single provider field.
    assert!(
        c.contains("pub struct __Deps_CountingAudit<'a, __P: ?Sized>")
            && c.contains("let mut __deps = __Deps_CountingAudit{ __p: &mut **__outer };")
            && c.contains("fn note<__Fx: Console + Counter>(&mut self, __fx: &mut __Fx"),
        "expected the two-dependency adapter and a generic member:\n{c}"
    );
    // Conjunction traits exist only as `__outer` field types.
    assert!(
        c.contains("pub trait __Conj_Console_Logger: Console + Logger {}")
            && c.contains("impl<T: Console + Logger + ?Sized> __Conj_Console_Logger for T {}"),
        "expected a conjunction trait with its blanket impl:\n{c}"
    );
    // Member dispatch is UFCS: one value implements every effect in scope.
    assert!(
        c.contains("Random::<i32>::next_random(&mut __fx"),
        "expected UFCS dispatch for a generic effect:\n{c}"
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
effect Sink<T> {
    fn accept(value: T) -> [value] None
}

handler Relay<T>(console: Console) of Sink<T> {
    fn accept(value: T) -> [value] None {
        println("relayed")
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    use Relay<Int>()
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
    fn log(message: Str) -> [message] None
}

effect Audit {
    fn note(message: Str) -> [message] None
}

effect Tally {
    fn add_up(n: Int) -> [n] None
    fn tally() -> [] Int
}

handler MemTally of Tally {
    sum: Int = 0
    fn add_up(n: Int) -> [n] None { sum = sum + n }
    fn tally() -> [] Int { return sum }
}

handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> [message] None {
        println("LOG: ${message}")
    }
}

handler LoggingAudit(logger: Logger) of Audit {
    count: Int = 0
    fn note(message: Str) -> [message] None {
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

fn label(n: Int) [Console] -> [] Str {
    println("labelling ${n}")
    return "n=${n}"
}

fn main() [use] -> [] None {
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
effect Logger {
    fn log(message: Str) -> [message] None
}

handler ConsoleLogger(console: Console) of Logger {
    tag: Str = "M"
    fn log(message: Str) -> [message] None {
        println("${tag}: ${message}")
    }
}

fn work() [Logger] -> None {
    log("from work")
}
"#;

const FUSION_MAIN_MODULE: &str = r#"
import logging.work
import logging.Logger
import logging.ConsoleLogger

fn main() [use] -> [] None {
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
    fn log(message: Str) -> [message] None
}

effect Sink {
    fn keep(items: Mut List<Int>) -> [] None
    fn kept() -> [] Int
}

effect Counter {
    fn bump() -> [] None
    fn total() -> [] Int
}

handler PrefixLogger(prefix: Str, console: Console, level: Int) of Logger {
    fn log(message: Str) -> [message] None {
        println("${prefix}[${level}] ${message}")
        tallied(message)
    }
}

handler MemSink of Sink {
    held: Mut List<Int> = mutable_list()
    fn keep(items: Mut List<Int>) -> [] None { held = items }
    fn kept() -> [] Int { return size(held) }
}

handler MemCounter of Counter {
    n: Int = 0
    fn bump() -> [] None { n = n + 1 }
    fn total() -> [] Int { return n }
}

fn tallied(text: Str) [Console, use] -> [text] None {
    use MemSink
    let xs: Mut List<Int> = mutable_list()
    add(xs, size(text))
    keep(xs)
    println("  tallied ${kept()}")
}

fn twice(f: (s: Str) [Logger] -> [s] Str) -> [] Str {
    return f("a")
}

fn report(label: Str) [Console, Logger, Counter] -> [label] None {
    bump()
    log("${label} #${total()}")
}

fn main() [use] -> [] None {
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
        let ys: Mut List<Int> = mutable_list()
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
effect Logger {
    fn log(message: Str) -> [message] None
}

handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> [message] None {
        println("LOG: ${message}")
    }
}

fn run_it(f: (s: Str) [Logger] -> [s] Str) [Console] -> [] None {
    println(f("x"))
}

fn demo() [Console, Logger] -> [] None {
    run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    })
}

fn main() [use] -> [] None {
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
    // The effect arrives as a parameter of the closure, not a capture.
    assert!(
        main.content.contains("|logger: &mut dyn Logger,"),
        "expected the effect threaded into the closure:\n{}",
        main.content
    );
    run_rust_files(&files, "fn-effects", "LOG: in lambda x\ndone x\n");
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

fn tag_ok<T>(value: T) -> T as Ok {
    return value
}

fn tag_err<T>(value: T) -> T as Err {
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
    let arr: Result[] = [tag_ok(1), tag_err("a")]
    for x in arr {
        println(describe(x))
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
    run_rust_files(&files, "nested-coercion", "ok 1\nerr a\nok 2\nok 3\nerr b\n");
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
    let files = generate(&[("main.sv", ARRAY_STD_DEMO)]);
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
    let program = build_program(&[("main.sv", QUAL_SUBJECTS)]);
    let files = salvo_backend_rust::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_rust_files(
        &files,
        "qual_subjects",
        "trusted 3\nplain 3\nchecked 2\n",
    );
}

// ===== E3: deferred blocks [defer] [rs-defer-splice] =====

/// `defer { ... }` at work: LIFO order at the end of a block, on an early
/// `return`, on `continue`/`break` out of a loop body, and discharging a
/// linear obligation on every path of a fn with two exits.
const DEFER_DEMO: &str = r#"
struct FileHandle : Linear<self> {
    fd: Int
}

fn close(x: FileHandle) -> [] None {}


fn open_file(n: Int) [Console] -> [] FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn close_file(h: FileHandle) [Console] -> [] None {
    println("close fd=${h.fd}")
    close(h)
}

fn scoped() [Console] -> [] None {
    defer { println("outer defer") }
    defer { println("inner defer") }
    println("body")
}

fn early(flag: Bool) [Console] -> [] Int {
    defer { println("early defer") }
    if flag {
        return 1
    }
    println("after if")
    return 2
}

fn looping() [Console] -> [] None {
    for i in [1, 2, 3] {
        defer { println("iteration ${i} done") }
        if i == 2 {
            continue
        }
        if i == 3 {
            break
        }
        println("body ${i}")
    }
}

fn with_resource(flag: Bool) [Console] -> [] Int {
    let h = open_file(7)
    defer { close_file(h) }
    if flag {
        return 1
    }
    return h.fd
}

fn main() [use] -> [] None {
    use StdOutConsole
    scoped()
    let a = early(true)
    let b = early(false)
    println("results ${a} ${b}")
    looping()
    let r1 = with_resource(true)
    println("resource ${r1}")
    let r2 = with_resource(false)
    println("resource ${r2}")
}
"#;

const DEFER_OUTPUT: &str = "body\ninner defer\nouter defer\n\
                            early defer\nafter if\nearly defer\nresults 1 2\n\
                            body 1\niteration 1 done\niteration 2 done\n\
                            iteration 3 done\nopen 7\nclose fd=7\nresource 1\n\
                            open 7\nclose fd=7\nresource 7\n";

/// [rs-defer-splice] Rust has no `finally`: the body is spliced at every
/// exit of its block, so `defer` leaves no runtime construct behind. The
/// `return` value is hoisted into a temporary, because it is computed
/// before the deferred code runs.
#[test]
fn defer_splices_at_every_exit() {
    let files = generate(&[("main.sv", DEFER_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    // LIFO at the end of a block, and no scaffolding.
    let scoped = main
        .content
        .split("pub fn scoped")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    let order: Vec<&str> = scoped
        .lines()
        .filter(|l| l.contains("println"))
        .map(|l| l.trim())
        .collect();
    assert_eq!(order.len(), 3, "expected three prints in:\n{scoped}");
    assert!(
        order[0].contains("\"body\"")
            && order[1].contains("inner defer")
            && order[2].contains("outer defer"),
        "expected LIFO order in:\n{scoped}"
    );
    // The early `return` runs the deferred code first, so the value is
    // hoisted.
    let early = main
        .content
        .split("pub fn early")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert!(
        early.contains("let __deferred_value1 = 1;")
            && early.contains("return __deferred_value1;"),
        "expected the return value hoisted in:\n{early}"
    );
    // `continue`/`break` splice the loop body's deferred code too.
    let looping = main
        .content
        .split("pub fn looping")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert_eq!(
        looping.matches("iteration {} done").count(),
        3,
        "expected the body spliced at the two exits and the block end in:\n{looping}"
    );
}

#[test]
fn rustc_compiles_and_runs_defer() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", DEFER_DEMO)]);
    run_rust_files(&files, "defer", DEFER_OUTPUT);
}

// ===== E3 step 2: throw and `try` [throw] [try] [rs-throw-controlflow] =====

/// The whole of non-resumption in one program: propagation through a frame
/// that declares the effect, a linear resource released by a deferred block
/// *on the throw path*, two message types meeting at one delimiter
/// (`Thrown (Str | Int)`), a may-throw call inside a loop, and a nested
/// delimiter that must not swallow the outer throw.
const THROW_DEMO: &str = r#"
struct FileHandle : Linear<self> {
    fd: Int
}

fn close(x: FileHandle) -> [] None {}


fn open_file(n: Int) [Console] -> [] FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn close_file(h: FileHandle) [Console] -> [] None {
    println("close fd=${h.fd}")
    close(h)
}

fn parse(line: Str) [Throw<Str>, Console] -> [] Int {
    println("parse ${line}")
    if size(line) == 0 {
        throw("empty line")
    }
    return size(line)
}

fn limit(n: Int) [Throw<Int>] -> [] Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn measure(line: Str) [Throw<Str>, Console] -> [] Int {
    let h = open_file(1)
    defer { close_file(h) }
    let n = parse(line)
    return n + h.fd
}

fn total(lines: Str[]) [Throw<Str>, Console] -> [] Int {
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

fn report_text(outcome: Ok Int | Thrown Str) [Console] -> [] None {
    when outcome {
        is Ok {
            println("ok ${outcome}")
        }
        is Thrown {
            println("thrown: ${outcome}")
        }
    }
}

fn main() [use] -> [] None {
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

const THROW_OUTPUT: &str = "open 1\nparse hello\nclose fd=1\nok 6\n\
                            open 1\nparse \nclose fd=1\nthrown: empty line\n\
                            open 1\nparse longer line\nclose fd=1\nmixed thrown\n\
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
        main.content
            .contains("pub fn parse(console: &mut dyn Console, line: String) -> ControlFlow<String, i32>"),
        "expected a ControlFlow return shape in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("return ControlFlow::Break(\"empty line\".to_string());"),
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
    // Propagation with a pending deferred block cannot use `?`: the
    // deferred release has to run before the frame is left [defer].
    let measure = main
        .content
        .split("pub fn measure")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert!(
        measure.contains("ControlFlow::Break(__m) => {")
            && measure.contains("close_file(console, h);"),
        "expected the deferred release on the throw path in:\n{measure}"
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
        main.content
            .contains("Union2::<String, i32>::U2(__m)"),
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
    fn log(message: Str) -> [message] None
}

handler ConsoleLogger(console: Console) of Logger {
    fn log(message: Str) -> [message] None {
        println("LOG: ${message}")
    }
}

fn shout(s: Str) [Logger] -> [s] Str {
    log("shouting ${s}")
    return "${s}!"
}

fn plain(s: Str) -> [s] Str {
    return "${s}."
}

// No effect list of its own: `run_it` *inherits* `[Logger]` from `f`.
fn run_it(f: (s: Str) [Logger] -> [s] Str, value: Str) -> [value] Str {
    return f(value)
}

fn demo() [Console, Logger] -> [] None {
    println(run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    }, "x"))
    println(run_it(shout, "one"))
    println(run_it(plain, "two"))
}

fn main() [use] -> [] None {
    use StdOutConsole
    use ConsoleLogger()
    demo()
}
"#;

const FN_EFFECTS_STDOUT: &str = "LOG: in lambda x\ndone x\nLOG: shouting one\none!\ntwo.\n";

/// [fn-effects] The effect is a leading `&mut dyn` parameter of the closure
/// type, so nothing is captured — which is what lets the value cross a call
/// that borrows the same effect value (the lifted fusion cut). A named fn is
/// wrapped in an adapter that takes the expected effects and forwards the
/// ones it declares.
#[test]
fn fn_type_effects_thread_into_closures() {
    let files = generate(&[("main.sv", FN_EFFECTS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("f: &mut impl FnMut(&mut dyn Logger, &String) -> String"),
        "expected the effect in the closure type:\n{}",
        main.content
    );
    assert!(
        main.content.contains("|logger: &mut dyn Logger,"),
        "expected the effect as a leading closure parameter:\n{}",
        main.content
    );
    // The adapter for a named fn: takes the expected effect, forwards what
    // the declaration needs (`shout`), or ignores it (`plain`).
    assert!(
        main.content.contains("__fx0: &mut dyn Logger") && main.content.contains("shout(&mut *__fx0"),
        "expected the named-fn adapter to forward the effect:\n{}",
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

// ===== the widening check `^` [qual-widen] =====

// [qual-widen] `^` tests the arm *and* removes the claim, so a branch can
// `when` the union inside a qualified one — the shape `try` outcomes produce.
const WIDEN_DEMO: &str = r#"
fn wrapped(n: Int) [Throw<Str>] -> [] Ok Int | Err Str {
    if n < 0 {
        throw("negative")
    }
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn limit(n: Int) [Throw<Int>] -> [] Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn describe(n: Int) [Console] -> [] None {
    let nested = try { wrapped(n) }
    when nested {
        ^ Ok {
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

fn main() [use] -> [] None {
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
        ^ Thrown {
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

/// [qual-widen] The peel is *materialized*: the widened value is bound to a
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
        main.content.contains("let mut nested = nested.u1().clone();"),
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
fn classify(n: Int) -> [] Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

fn sign(n: Int) -> [] Int {
    when {
        n < 0 { return -1 }
        n > 0 { return 1 }
        else { return 0 }
    }
}

fn describe(value: Str | Int) [Console] -> [] None {
    when {
        value is Str s { println("str ${s}") }
        else { println("int ${value}") }
    }
}

fn label(n: Int) [Console] -> [] Str {
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

fn nested(o: Ok Int | Err Str) [Console] -> [] None {
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

fn main() [use] -> [] None {
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
fn risky(n: Int) [Throw<Str>] -> [] Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

fn main() [use] -> [] None {
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
    fn record(name: Str, value: Int) [] -> [name, value] None
}

fn work(n: Int) [Telemetry] -> [] Int {
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
    let ns = list(1, 2)
    for v in map(iter(copy(ns)), n -> n * 2) { println("bare=${v}") }
    for v in map(iter(ns), (n: Int) -> n * 3) { println("ann=${v}") }
    let ws = list("hi")
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
/// [rs-fn-field] A callback the callee *keeps* is the exception on the outer
/// level only: it arrives owned and `'static`, since it is called after the
/// call returns — from the composed pass that stored it. The inner convention is
/// the same.
#[test]
fn a_lambda_binds_a_generic_fn_parameter_by_reference() {
    let files = generate(&[("main.sv", GENERIC_HOF)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    let seq = &files
        .iter()
        .find(|f| f.rel_path.ends_with("seq.rs"))
        .expect("core/seq.rs")
        .content;
    // The stored-callback convention, on std's lazy combinator.
    assert!(
        seq.contains("f: impl Fn(&T) -> U + 'static"),
        "expected the owned callback of a composed pass in:\n{seq}"
    );
    for expected in [
        // The plain generic higher-order fn: borrowed `FnMut`.
        "f: &mut impl FnMut(&T) -> U",
        // The lambda's *inner* convention follows the declaration.
        "|n: &i32|",
        "|s: &String|",
    ] {
        assert!(
            src.contains(expected),
            "expected `{expected}` in:\n{src}"
        );
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

/// An unbounded **origin**, filtered lazily by std's `filter_lazy`, and driven
/// twice: the program only terminates if elements are produced on demand, and
/// the second pair of loops only replays because a `for` mints a fresh machine
/// from the origin [yield-fn-origin]. The Kotlin backend asserts the same stdout
/// for the same source.
pub const LAZY_ITER_DEMO: &str = r#"
struct Naturals : Yield<self, Int> {
    from: Int
}

fn naturals(from: Int) -> [] Naturals {
    return Naturals {from: from}
}

yield fn next(n: Naturals) -> Int {
    let i = copy(n.from)
    while true {
        yield copy(i)
        i = i + 1
    }
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] {
    use StdOutConsole()
    let unconsumed = naturals(100)
    println("created")
    for v in filter_lazy(naturals(0), is_even) {
        if v > 6 {
            break
        }
        println("even ${v}")
    }
    let twice = naturals(0)
    for v in twice {
        if v > 2 {
            break
        }
        println("first ${v}")
    }
    for v in twice {
        if v > 2 {
            break
        }
        println("second ${v}")
    }
}
"#;

pub const LAZY_ITER_OUTPUT: &str = "created\neven 0\neven 2\neven 4\neven 6\n\
                                    first 0\nfirst 1\nfirst 2\nsecond 0\nsecond 1\nsecond 2\n";

/// [iter-generator] A `yield fn` lowers to a **struct the compiler wrote**: the
/// shared plan (`salvo_core::generator`) rendered as fields plus a flat `match`
/// on a state number. The origin is held and read, so each `for` builds a fresh
/// machine and replays; what is gone is the `async` block rustc used to
/// transform for us — and with it `SalvoGen`, `SalvoYield`, `Pin`, `Future` and
/// `Waker` — and, since R5, the factory the machine used to be minted from.
#[test]
fn an_origin_lowers_to_a_state_machine() {
    let files = generate(&[("main.sv", LAZY_ITER_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        "struct __Pass_Naturals {",
        "    i: i32,",
        "    __state: u32,",
        "fn __advance(&mut self) -> Option<i32> {",
        "match self.__state {",
        "self.__state = 1;",
        "return Some(__v);",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    for gone in ["SalvoGen", "SalvoYield", "async move", ".await", "SalvoIter"] {
        assert!(!src.contains(gone), "`{gone}` should be gone from:\n{src}");
    }
}

/// A **composed** pass built from an origin keeps the machine: the mint is
/// hoisted into a `let` and *not* closed after the call, because the callee
/// stores it and drives it later [yield-fn-origin]. Closing it there handed the
/// pass a finished machine and the loop saw nothing.
#[test]
fn a_stored_mint_is_not_closed_at_the_call() {
    let files = generate(&[("main.sv", LAZY_ITER_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("let mut __mint1 = __Pass_Naturals::new("),
        "expected the hoisted mint in:\n{src}"
    );
    assert!(
        !src.contains("__mint1.__close()"),
        "a moved mint must not be closed at the call in:\n{src}"
    );
}

/// [seq-lazy] [seq-into] The combinator surface (user decision 2026-09-08):
/// **eager by default**, with `map_lazy`/`filter_lazy` for the lazy pair and
/// `map_to` for mapping into a collection the caller provides.
///
/// Same source and stdout as the Kotlin backend's twin. What it exercises that
/// nothing did before: the *generic* `?Iterable` body — `map`/`filter` over a
/// list take their `List` fast path, so the polymorphic one had never run — a
/// producer holding **implicit** parameters as pass fields, and an implicit
/// with a `Mut` parameter (`?add`).
pub const SEQ_SURFACE_DEMO: &str = r#"
fn double(n: Int) -> Int {
    return n * 2
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdOutConsole()
    let xs = list(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    for v in map_lazy(iter(copy(xs)), double) {
        println("lazy ${v}")
    }
    for v in filter_lazy(iter(copy(xs)), is_even) {
        println("kept ${v}")
    }
    let out = map_to(mutable_list<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(mutable_list<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
"#;

pub const SEQ_SURFACE_OUTPUT: &str =
    "eager 4\nlazy 2\nlazy 4\nlazy 6\nlazy 8\nkept 2\nkept 4\nsink 4\nchained 6\n";

#[test]
fn rustc_compiles_and_runs_the_combinator_surface() {
    let files = generate(&[("main.sv", SEQ_SURFACE_DEMO)]);
    let seq = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/seq.rs"))
        .expect("core/seq.rs")
        .content;
    // A lazy combinator returns a **composed pass**: the source's `next` is a
    // field of it, `Rc`-held like any stored callback [rs-fn-field], and called
    // through the local the body binds it to.
    assert!(
        seq.contains("step: std::rc::Rc<dyn Fn(&mut It) -> Union2<T, Finished>>"),
        "expected the stored `next` as an Rc-held field in:\n{seq}"
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

/// [seq-lazy] Laziness is the observable difference, so this is the test that
/// shows it: an **unbounded** producer, mapped and filtered lazily, that
/// terminates only because the consumer stops. Eager `map` would not return.
#[test]
fn rustc_compiles_and_runs_a_lazy_chain_over_an_unbounded_producer() {
    let files = generate(&[("main.sv", LAZY_CHAIN_DEMO)]);
    run_rust_files(&files, "lazy-chain", LAZY_CHAIN_OUTPUT);
}

pub const LAZY_CHAIN_DEMO: &str = r#"
struct Naturals : Yield<self, Int> {
    from: Int
}

fn naturals() -> [] Naturals {
    return Naturals {from: 0}
}

yield fn next(n: Naturals) -> Int {
    let i = copy(n.from)
    while true {
        yield copy(i)
        i = i + 1
    }
}

fn triple(n: Int) -> Int {
    return n * 3
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdOutConsole()
    for v in map_lazy(filter_lazy(naturals(), is_even), triple) {
        if v > 12 {
            break
        }
        println("v ${v}")
    }
}
"#;

pub const LAZY_CHAIN_OUTPUT: &str = "v 0\nv 6\nv 12\n";

/// [rs-fn-field] A **stored** callback is `impl Fn(…) + 'static`, so a lambda
/// handed to one must be a **`move`** closure: it outlives the call that built
/// the pass. Emitting a borrowing closure was E0373 ("closure may outlive the
/// current function") for *every* capturing callback, immutable ones included —
/// the case the parity rules call free.
///
/// Same source and stdout as the Kotlin backend's twin. The capture is
/// immutable on purpose: a *mutable* one is a checker error [iter-mut-param].
pub const PRODUCER_CALLBACK_DEMO: &str = r#"
struct Upto : Yield<self, Int> {
    limit: Int
}

fn upto(limit: Int) -> [] Upto {
    return Upto {limit: limit}
}

yield fn next(u: Upto) -> Int {
    let i = 0
    while i < u.limit {
        yield copy(i)
        i = i + 1
    }
}

fn scale_by(factors: List<Int>, v: Int) -> [factors] Int {
    return v * factors.first()!
}

fn main() [use] -> None {
    use StdOutConsole()
    let factors = list(10)
    for s in map_lazy(upto(3), (v: Int) -> scale_by(factors, v)) {
        println("a ${s}")
    }
    for s in map_lazy(upto(3), (v: Int) -> scale_by(factors, v)) {
        println("b ${s}")
    }
}
"#;

pub const PRODUCER_CALLBACK_OUTPUT: &str = "a 0\na 10\na 20\nb 0\nb 10\nb 20\n";

#[test]
fn rustc_compiles_and_runs_a_producer_with_a_capturing_callback() {
    let files = generate(&[("main.sv", PRODUCER_CALLBACK_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("move |v"),
        "expected a `move` closure for the stored callback in:\n{src}"
    );
    run_rust_files(&files, "producer-callback", PRODUCER_CALLBACK_OUTPUT);
}

/// [iter-generator] [defer] The parts of a producer only the state machine
/// can carry: a `defer` inside a suspending loop body — registered anew each
/// iteration, discharged at the end of it, and *writing a local the body
/// reads after the loop* — a bare `return` out of the middle of the nest, a
/// nested producer consuming another one, and a second pass that starts over.
///
/// Both backends run this source and assert this stdout byte for byte, which
/// is what makes the flag-and-discharge lowering a parity claim rather than
/// two implementations that happen to agree.
pub const GENERATOR_DEFER_DEMO: &str = r#"
struct Upto : Yield<self, Int> {
    limit: Int
}

fn upto(limit: Int) -> [] Upto {
    return Upto {limit: limit}
}

yield fn next(u: Upto) -> Int {
    let last = 0
    let i = 0
    while i < u.limit {
        defer { last = i }
        if i == 4 {
            return
        }
        yield copy(i)
        i = i + 1
    }
    yield last * 100
}

struct Tagged : Yield<self, Str> {
    items: List<Int>
}

fn tagged(items: List<Int>) -> [] Tagged {
    return Tagged {items: items}
}

yield fn next(t: Tagged) -> Str {
    for x in t.items {
        yield "<${x}>"
    }
}

fn main() [use] -> None {
    use StdOutConsole()
    for s in tagged(map(upto(3), (v: Int) -> copy(v))) {
        println(s)
    }
    for v in upto(2) {
        println("again ${v}")
    }
}
"#;

pub const GENERATOR_DEFER_OUTPUT: &str =
    "<0>\n<1>\n<2>\n<300>\nagain 0\nagain 1\nagain 200\n";

#[test]
fn rustc_compiles_and_runs_a_generator_with_defers() {
    let files = generate(&[("main.sv", GENERATOR_DEFER_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // One flag per `defer` site, and the discharge is guarded by it — which
    // is what makes the release path idempotent.
    assert!(src.contains("__d0: bool,"), "expected a defer flag in:\n{src}");
    assert!(
        src.contains("self.__d0 = true;") && src.contains("self.__run_d0();"),
        "expected the register/discharge pair in:\n{src}"
    );
    run_rust_files(&files, "generator-defers", GENERATOR_DEFER_OUTPUT);
}

/// [yield-fn-origin] The origin-struct sugar, end to end. One program and one
/// expected stdout shared with the Kotlin backend — the parity claim.
///
/// It is the R0 prototype's awkward shape: the producer performs `Console`
/// *while* the consumer drives it (so the interleaving is observable), holds a
/// `defer` that itself prints (so the release path is observable), and the
/// first loop **abandons** it after two elements — then a second `for` over
/// the *same origin* replays from the beginning, which is the point of the
/// origin model: driving mints a fresh machine and consumes nothing.
pub const YIELD_ORIGIN_DEMO: &str = r#"
struct Counter : Yield<self, Int> {
    start: Int
}

yield fn next(c: Counter) [Console] -> Int {
    println("open")
    defer { println("close") }
    let num = copy(c.start)
    while num >= 0 {
        println("make ${num}")
        yield copy(num)
        num = num - 1
    }
}

fn counter(start: Int) -> Counter {
    return Counter { start: start }
}

fn main() [use] {
    use StdOutConsole()
    let c = counter(2)
    let seen = 0
    for n in c {
        println("got ${n}")
        seen = seen + 1
        if seen == 2 {
            break
        }
    }
    for n in c {
        println("again ${n}")
    }
    println("done")
}
"#;

pub const YIELD_ORIGIN_OUTPUT: &str = "open\nmake 2\ngot 2\nmake 1\ngot 1\nclose\n\
                                       open\nmake 2\nagain 2\nmake 1\nagain 1\nmake 0\n\
                                       again 0\nclose\ndone\n";

#[test]
fn rustc_compiles_and_runs_a_yield_origin() {
    let files = generate(&[("main.sv", YIELD_ORIGIN_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // The machine is named after the **origin**, because every `yield fn` is
    // called `next` — and it is `pub`, since the drive site names it directly.
    assert!(
        src.contains("pub struct __Pass_Counter {"),
        "expected the origin machine in:\n{src}"
    );
    // What the origin form does *not* emit: no factory, no `SalvoIter`, no
    // boxing, and no callable `next`.
    assert!(
        !src.contains("SalvoIter") && !src.contains("from_factory"),
        "the factory should be gone from:\n{src}"
    );
    assert!(
        !src.contains("pub fn next("),
        "a `yield fn` must not be emitted as a function in:\n{src}"
    );
    // The drive site constructs the machine and closes it after the loop; the
    // origin is *cloned*, which is what lets the second loop replay.
    assert!(
        src.contains("__Pass_Counter::new(c.clone());")
            && src.contains(".__advance(&mut console)")
            && src.contains(".__close(&mut console);"),
        "expected construct/advance/close in:\n{src}"
    );
    run_rust_files(&files, "yield-origin", YIELD_ORIGIN_OUTPUT);
}

/// Under rustc: an unbounded producer that terminates, and a second pass
/// that starts over. Before the change the first `for` never finished
/// building its `Vec`.
#[test]
fn rustc_compiles_and_runs_a_lazy_iterator() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", LAZY_ITER_DEMO)]);
    run_rust_files(&files, "lazy-iter", LAZY_ITER_OUTPUT);
}

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

/// [yield-fn-origin] The **mint**: anything whose declaration says
/// `: Yield<self, T>` can be passed as is (user decision 2026-09-09). A raw
/// pass is taken as it stands; an **origin** — whose machine is hidden, so the
/// author has nothing `Mut` to hand over — is rewritten at the argument into a
/// fresh instance of its pass. One combinator, both forms, one program: the
/// generic signature always names the pass (`it: Mut It`), because desugaring
/// happens at the call site, before the body runs.
pub const ORIGIN_MINT_DEMO: &str = r#"
struct Counter : Yield<self, Int> {
    start: Int
}

yield fn next(c: Counter) -> Int {
    let num = copy(c.start)
    while num >= 0 {
        yield copy(num)
        num = num - 1
    }
}

fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Yield<It, T>) -> [it: Mut, mapper] Mut List<U> {
    let out = mutable_list<U>()
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

fn counter(start: Int) -> [] Counter {
    return Counter { start: start }
}

// A raw pass in the same position: taken as it stands, no mint.
struct Zip : Yield<self, Int> canbe Mut {
    left: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> [z: Mut] Emitted Int | Finished {
    let e = get(z.left, z.at)
    if e is None {
        return finished()
    }
    z.at = z.at + 1
    return emitted(e)
}

fn main() [use] {
    use StdOutConsole()
    let doubled = map2(counter(2), double)
    for d in doubled {
        println("d ${d}")
    }
    let zipped = map2(Mut Zip { left: list(5, 6), at: 0 }, double)
    for z in zipped {
        println("z ${z}")
    }
}
"#;

pub const ORIGIN_MINT_OUTPUT: &str = "d 4\nd 2\nd 0\nz 10\nz 12\n";

/// The mint's shape: the machine constructed at the argument, the adapter
/// wrapping its advance into the protocol's arms, and `Clone` on the machine
/// (it is a type argument now, and every generic parameter is `Clone`).
#[test]
fn an_origin_argument_mints_a_pass() {
    let files = generate(&[("main.sv", ORIGIN_MINT_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The mint is hoisted into a `let` so the *compiler* can release it after
    // the call, whatever the callee did with it.
    assert!(
        main.content.contains("let mut __mint1 = __Pass_Counter::new(counter(2));")
            && main.content.contains("__mint1.__close();"),
        "expected the mint hoisted and released, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__p.__advance()")
            && main.content.contains("Union2::<i32, Finished>::U1(__v)"),
        "expected the advance adapter, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains("#[derive(Clone)]\npub struct __Pass_Counter"),
        "expected the machine to derive Clone, got:\n{}",
        main.content
    );
    // The raw pass in the same position is untouched.
    assert!(
        !main.content.contains("__Pass_Zip"),
        "a raw pass must not be minted, got:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_an_origin_mint() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", ORIGIN_MINT_DEMO)]);
    run_rust_files(&files, "origin_mint", ORIGIN_MINT_OUTPUT);
}


/// [yield-fn-origin] The **release** of a minted pass: the mint is the
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
struct Lines : Yield<self, Int>, Linear<self> canbe Mut {
    at: Int
}

fn next(l: Mut Lines) -> [l: Mut] Emitted Int | Finished {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Lines) [Console] -> [] None {
    println("closed")
}

fn drained() [Console] -> [] None {
    let lines = Mut Lines { at: 2 }
    for n in lines {
        println("n ${n}")
    }
    println("after drain")
}

fn abandoned() [Console] -> [] None {
    let lines = Mut Lines { at: 5 }
    for n in lines {
        println("m ${n}")
        break
    }
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
fn a_raw_pass_with_a_close_is_released_by_the_loop() {
    let files = generate(&[("main.sv", RAW_CLOSE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("close(console, __loop1_pass);"),
        "expected the release after the loop, got:\n{}",
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

pub const MINT_CLOSE_DEMO: &str = r#"
// An origin whose body defers, so the machine has a release path — and a
// combinator that ABANDONS it after one element.
struct Chatty : Yield<self, Int> {
    limit: Int
}

yield fn next(c: Chatty) [Console] -> Int {
    println("open")
    defer { println("close") }
    let num = copy(c.limit)
    while num > 0 {
        yield copy(num)
        num = num - 1
    }
}

fn first_of<It, T>(it: Mut It, ?Yield<It, T>) -> [it: Mut] Int {
    let step = next(it)
    when step {
        is Emitted {
            return 1
        }
        is Finished {
            return 0
        }
    }
}

fn chatty(limit: Int) -> [] Chatty {
    return Chatty { limit: limit }
}

fn main() [use] {
    use StdOutConsole()
    let got = first_of(chatty(3))
    println("got ${got}")
}
"#;

pub const MINT_CLOSE_OUTPUT: &str = "open\nclose\ngot 1\n";

/// Two mints in one call, and two mints of the **same origin value**: each is a

/// [iter-generic-drive] [iter-drive-in-place] The three things the generic drive
/// buys, in one program: a combinator driving its pass with an ordinary `for`,
/// the caller **carrying that pass on** afterwards (the parity case — Rust used
/// to bind the `&mut` parameter into a local, so the caller never saw the
/// position the loop reached), and a `yield fn` performing a **generic** effect.
///
/// Shared with the other backend, byte for byte.
pub const GENERIC_DRIVE_DEMO: &str = r#"
struct Slice<T> : Yield<self, T> canbe Mut {
    items: List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> [] Mut Slice<T> {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> [p: Mut] Emitted T | Finished {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

// The pass is *kept*, so the loop advances it where it lives and the caller may
// drive it further.
fn take<It>(it: Mut It, count: Int, ?Yield<It, Int>) [] -> [it: Mut] Int {
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
    fn next_random() -> [] T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> [] T {
        i = (i + 1) % size(values)
        return values[i]
    }
}

struct Rolls : Yield<self, Int> {
    count: Int
}

fn rolls(count: Int) -> [] Rolls {
    return Rolls {count: count}
}

yield fn next(r: Rolls) [Random<Int>] -> Int {
    let n = 0
    while n < r.count {
        yield next_random()
        n = n + 1
    }
}

fn main() [use] {
    use StdOutConsole()
    use CyclicRandom([7, 8, 9])
    let p = slice(list(1, 2, 3, 4))
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
struct Handle : Linear<self>, Yield<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> [] Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) -> [h: Mut] Emitted Int | Finished {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) -> [] None {}

// Owns the pass: the loop releases it on every exit, `break` included.
fn drain<It canbe Linear>(it: Mut It, stop: Int, ?Yield<It, Int>, ?Linear<It>) [] -> [] Int {
    let sum = 0
    for n in it {
        sum = sum + n
        if sum > stop {
            break
        }
    }
    return sum
}

fn main() [use] {
    use StdOutConsole()
    println("all ${drain(open_handle(4), 100)}")
    println("cut ${drain(open_handle(4), 5)}")
}
"#;

pub const GENERIC_CLOSE_OUTPUT: &str = "all 10\ncut 7\n";

/// fresh machine, so both replay from the beginning and both are released.
pub const TWO_MINTS_DEMO: &str = r#"
struct Counter : Yield<self, Int> {
    start: Int
}

yield fn next(c: Counter) -> Int {
    let num = copy(c.start)
    while num >= 0 {
        yield copy(num)
        num = num - 1
    }
}

// Two pass parameters of the same type, so one `?Yield` spread serves both.
fn sum_two<It, T>(a: Mut It, b: Mut It, ?Yield<It, T>) -> [a: Mut, b: Mut] Int {
    let n = 0
    let going = true
    while going {
        let step = next(a)
        when step {
            is Emitted {
                n = n + 1
            }
            is Finished {
                going = false
            }
        }
    }
    let going2 = true
    while going2 {
        let step2 = next(b)
        when step2 {
            is Emitted {
                n = n + 10
            }
            is Finished {
                going2 = false
            }
        }
    }
    return n
}

fn counter(start: Int) -> [] Counter {
    return Counter { start: start }
}

fn main() [use] {
    use StdOutConsole()
    // Two mints from two separate origins.
    println("two ${sum_two(counter(1), counter(2))}")
    // Two mints from the SAME origin value: each is a fresh machine, so both
    // replay from the beginning.
    let c = counter(1)
    println("same ${sum_two(c, c)}")
}
"#;

pub const TWO_MINTS_OUTPUT: &str = "two 32\nsame 22\n";

#[test]
fn two_mints_in_one_call_are_independent_and_both_released() {
    let files = generate(&[("main.sv", TWO_MINTS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains("let mut __mint1 = __Pass_Counter::new")
            && main.content.contains("let mut __mint2 = __Pass_Counter::new"),
        "expected two independent machines, got:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__mint1.__close();") && main.content.contains("__mint2.__close();"),
        "expected both to be released, got:\n{}",
        main.content
    );
}

#[test]
fn rustc_compiles_and_runs_an_abandoned_mint() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", MINT_CLOSE_DEMO)]);
    run_rust_files(&files, "mint_close", MINT_CLOSE_OUTPUT);
}

#[test]
fn rustc_compiles_and_runs_two_mints_in_one_call() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", TWO_MINTS_DEMO)]);
    run_rust_files(&files, "two_mints", TWO_MINTS_OUTPUT);
}

pub const YIELD_SPREAD_DEMO: &str = r#"
struct Slice<T> : Yield<self, T> canbe Mut {
    items: List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> [] Mut Slice<T> {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> [p: Mut] Emitted T | Finished {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Yield<It, T>) -> [it: Mut, mapper] Mut List<U> {
    let out = mutable_list<U>()
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
    let xs = list(1, 2, 3)
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

fn total<T>(xs: List<T>, ?Field<T>) -> [xs] T {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

fn total_all<T>(rows: List<List<T>>, ?Field<T>) -> [rows] T {
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
    println("total=${total(list(1, 2, 3))}")
    println("product=${total(list(2, 3, 4), add = times, zero = one)}")
    println("nested=${total_all(list(list(1, 2), list(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
"#;

pub const IMPLICIT_OUTPUT: &str = "total=6\nproduct=24\nnested=6\npair=42\nlambda=6\n";

/// [implicit-param] An implicit parameter lowers to an ordinary trailing
/// parameter of fn type, and the call site passes what resolution found —
/// so nothing about the feature survives into Rust. A group leaves no trace
/// at all: it was never a value [implicit-group].
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
        "pub fn total<T: Clone + 'static>(xs: &Vec<T>, add: &mut dyn FnMut(T, T) -> T, \
         zero: &mut dyn FnMut() -> T)",
        // A resolved default is passed as an adapter closure over the fn.
        "&mut |__i0, __i1| add(__i0, __i1)",
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

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
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

fn next(d: Mut Doubling) -> [d: Mut] Emitted Int | Finished {
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
        main.content.contains("pub f: std::rc::Rc<dyn Fn(i32) -> i32>,"),
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

fn next(z: Mut Zip) -> [z: Mut] Emitted (Str, Int) | Finished {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        return emitted((l, r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> [] Zip {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
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
    let names = list("ada", "grace", "alan")
    let ages = list(36, 45)
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
/// (see the open defect about wrapping an inner arm under a qualifier: the
/// intermediate `let` here is that workaround, not decoration).
pub const FALLIBLE_PASS_DEMO: &str = r#"
struct Reader : Yield<self, Ok Str | Err Str> canbe Mut {
    lines: List<Str>,
    at: Int
}

fn next(r: Mut Reader) -> [r: Mut] Emitted (Ok Str | Err Str) | Finished {
    let line = get(r.lines, r.at)
    if line is Str {
        r.at = r.at + 1
        if line == "boom" {
            let bad: Ok Str | Err Str = err("bad line at ${r.at}")
            return emitted(bad)
        }
        let good: Ok Str | Err Str = ok(line)
        return emitted(good)
    }
    return finished()
}

fn reader(lines: List<Str>) -> [] Reader {
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
    let good = try { read_all(list("alpha", "beta")) }
    when good {
        is Ok { println("read ${good}") }
        is Thrown { println("failed: ${good}") }
    }
    let bad = try { read_all(list("alpha", "boom", "gamma")) }
    when bad {
        is Ok { println("read ${bad}") }
        is Thrown { println("failed: ${bad}") }
    }
}
"#;

pub const FALLIBLE_PASS_OUTPUT: &str = "line alpha\nline beta\nread 2\nline alpha\nfailed: stopped: bad line at 2\n";

#[test]
fn a_fallible_pass_yields_a_result() {
    if !rustc_available() {
        eprintln!("skipping: rustc not found on PATH");
        return;
    }
    let files = generate(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    run_rust_files(&files, "fallible-pass", FALLIBLE_PASS_OUTPUT);
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
    fn show(v: Int, ?fmt: (Int) -> Str) -> [v] Str
}

handler Angle of Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> [v] Str {
        return "<${fmt(v)}>"
    }
}

fn fmt(n: Int) -> [n] Str {
    return "n=${n}"
}

fn loud(n: Int) -> [n] Str {
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
    fn show(v: T) -> [v] Str
}

// Stateless *and* generic: nothing but the `use` site can say what `T` is.
handler Plain<T> of Show<T> {
    fn show(v: T) -> [v] Str {
        return "plain"
    }
}

effect Tag<T> {
    fn tagged(v: T) -> [v] Str
}

// Generic with state: the constructor argument used to be the only thing
// that could bind `T`, and still works.
handler Prefixed<T>(prefix: Str) of Tag<T> {
    fn tagged(v: T) -> [v] Str {
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
    refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
}

// Only callable while the compiler still believes the list is non-empty.
fn count<T canbe Linear>(list: NonEmpty List<T>) -> [list] Int {
    return size(list)
}

fn refill(list: Mut NonEmpty List<Int>, value: Int) -> [list: Mut NonEmpty] None {
    add(list, value)
}

fn main() [use] {
    use StdOutConsole()
    let xs: Mut List<Int> = mutable_list()
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
const REFN_CONFLICT_DEMO: &str = r#"
qualifier Q1<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool { return size(list) > 0 }
    refn add(list: Mut List<T>, elem: T) -> [list: +Q1]
}

qualifier Q2<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool { return size(list) > 0 }
    refn add(list: Mut List<T>, elem: T) -> [list: +Q2]
}

fn main() [use] {
    use StdOutConsole()
    let xs: Mut List<Int> = mutable_list()
    add(xs, 1)
    if xs is Q1 {
        println("checked by hand: ${size(xs)}")
    }
}
"#;

#[test]
fn a_refinement_conflict_warns_without_stopping_emission() {
    let program = build_program(&[("main.sv", REFN_CONFLICT_DEMO)]);
    let (files, warnings) =
        salvo_backend_rust::emit_program_reporting(&program, None).unwrap_or_else(|errors| panic!("a warning must not stop emission: {errors:?}"));
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
    let b = mutable_str("he", "llo")
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

    let count = mutable_list<Int>()
    for c in iter(hay) {
        add(count, 1)
    }
    println("chars: ${size(count)}")

    // Operators drop `Mut` too, so equality is by content on both targets.
    let x = mutable_str(pa)
    let y = mutable_str(pa)
    println("equal: ${x == y}")
}
"#;

/// The stdout both backends must produce, byte for byte.
const STRING_DEMO_OUTPUT: &str = "HELLO WORLD\nsize: 11\nHello world / Hello world!\n\
                                  cleared: []\n4 a-b--c\n[pad] true true true\npad\n\
                                  2 true\nel true\n42 true\nhel|\nchars: 5\nequal: true\n";

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
    assert!(main.contains(r#"format!("{} / {}", b, dup)"#), "unexpected:\n{main}");
    // The parts are borrowed, not moved: a variadic position is untracked by
    // the flow analysis, so an owned `vec![pa]` would move a variable the
    // checker still considers live.
    assert!(main.contains(r#"let mut x = [&pa[..]].concat();"#), "unexpected:\n{main}");
    // Characters, not bytes — `find` answers in bytes, so the prefix is
    // re-counted.
    assert!(
        main.contains("__s.find(&ll[..]).map(|__b| __s[..__b].chars().count() as i32)"),
        "unexpected:\n{main}"
    );
    // [rs-mut-str] `set` goes through the generated trait: method syntax
    // auto-refs an owned local and a `&mut String` parameter alike.
    assert!(main.contains("b.salvo_set(0, 'H')"), "unexpected:\n{main}");
    assert!(main.contains("use crate::strings::*;"), "unexpected:\n{main}");
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
    let plain = generate(&[("main.sv", "fn main() {\n}\n")]);
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
/// borrowed, since `mutable_str` reads them.
#[test]
fn a_spread_into_a_variadic_intrinsic_is_the_collection() {
    let src = r#"
fn main() [use] {
    use StdOutConsole()
    let parts = ["a", "b"]
    let sb = mutable_str(...parts)
    let xs = list(...parts)
    println("${sb} ${size(xs)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    assert!(
        main.contains("let mut sb = parts.concat();")
            && main.contains("let mut xs = parts.clone();"),
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

fn grow(s: Mut Str) -> [s: Mut] None {
    append(s, "!")
    set(s, 0, 'G')
    clear(s)
    append(s, "grown")
}

fn main() [use] {
    use StdOutConsole()
    let b = mutable_str("seed")
    grow(b)
    println("${b} ${size(b)}")
    let buf = Mut Buf {text: mutable_str("in-struct")}
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
    assert!(main.contains("buf.text.salvo_set(0, 'I')"), "unexpected:\n{main}");
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

fn iter(bag: Bag) -> [] Mut ListYield<Int> {
    return iter(bag.items)
}

fn double(n: Int) -> Int {
    return n * 2
}

struct Naturals : Yield<self, Int> {
    from: Int
}

fn naturals(from: Int) -> [] Naturals {
    return Naturals {from: from}
}

yield fn next(n: Naturals) -> Int {
    let i = copy(n.from)
    while i < n.from + 4 {
        yield copy(i)
        i = i + 1
    }
}

fn main() [use] {
    use StdOutConsole()
    let xs = list(1, 2, 3, 4)
    let doubled = map(xs, n -> n * 2)
    let sum = reduce(xs, 0, (a, b) -> a + b)
    let big = filter(xs, n -> n > 2)
    println("list: ${size(doubled)} ${sum} ${size(big)}")
    let named = map(xs, double)
    println("named: ${size(named)}")
    let arr = [10, 20, 30]
    let arr_sum = reduce(iter(copy(arr)), 0, (a, b) -> a + b)
    let arr_mapped = map(iter(arr), n -> n + 1)
    println("array: ${arr_sum} ${size(arr_mapped)}")
    let letters = filter(iter("hello"), c -> c == 'l')
    println("chars: ${size(letters)}")
    let lazy_sum = reduce(naturals(1), 0, (a, b) -> a + b)
    let chained = filter(iter(map(xs, n -> n * 3)), n -> n > 6)
    println("iter: ${lazy_sum} ${size(chained)}")
    let names = list("ann", "bob", "carol")
    let lens = map(names, n -> size(n))
    let long = filter(names, n -> size(n) > 3)
    println("names: ${size(lens)} ${size(long)}")
    let bag = Bag {items: list(5, 6)}
    println("bag: ${reduce(iter(bag), 0, (a, b) -> a + b)}")
}
"#;

const SEQ_DEMO_OUTPUT: &str = "list: 4 10 2\nnamed: 4\narray: 60 3\nchars: 2\n\
                               iter: 10 2\nnames: 3 1\nbag: 11\n";

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
    assert!(main.contains("let mut named = salvo_map(&xs[..], |"), "unexpected:\n{main}");
    // A pass subject's `next` arrives as the implicit argument, and an *origin*
    // is minted into its machine at the call with an adapter over its advance
    // [yield-fn-origin].
    assert!(
        main.contains("__Pass_Naturals::new(naturals(1))"),
        "expected the origin mint in:\n{main}"
    );
    assert!(
        main.contains("__p.__advance()"),
        "expected the advance adapter in:\n{main}"
    );
    // The support file is there, and only because something needed it.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "seq.rs"),
        "seq.rs should be emitted"
    );
    let plain = generate(&[("main.sv", "fn main() {\n}\n")]);
    assert!(
        !plain.iter().any(|f| f.rel_path.to_string_lossy() == "seq.rs"),
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

fn size<T>(list: List<T>) -> [list] Str {
    return "mine"
}

fn label(n: Even Int) -> [n] Str {
    return "even"
}

fn label(n: Small Int) -> [n] Str {
    return "small"
}

// The two `label`s are unrankable — one qualifier each, and the *kind* of
// qualifier never ranks — so one of them takes a name of its own.
rename fn label_small = label(n: Small Int)

fn main() [use] {
    use StdOutConsole()
    let xs = list(1, 2, 3)
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
    assert!(!main.contains("label_small"), "unexpected rename in:\n{main}");
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
    assert!(
        !src.contains("_pass = it"),
        "a kept pass must not be bound into a local in:\n{src}"
    );
}

#[test]
fn rustc_compiles_and_runs_a_generic_drive() {
    let files = generate(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    run_rust_files(&files, "generic-drive", GENERIC_DRIVE_OUTPUT);
}

/// [linear-generics] The release: the implicit `close` is called after the loop
/// *and* registered as a deferred entry, so a `return` out of the body reaches
/// it too. The pass is **moved** into the loop's local, since the fn owns it —
/// cloning it would have left the original unreleased.
#[test]
fn an_owned_generic_pass_is_closed_by_the_loop() {
    let files = generate(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("let mut __loop1_pass = it;"),
        "expected the pass to be moved into the loop in:\n{src}"
    );
    assert!(
        src.contains("close(__loop1_pass);"),
        "expected the implicit release after the loop in:\n{src}"
    );
}

#[test]
fn rustc_compiles_and_runs_a_generic_close() {
    let files = generate(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    run_rust_files(&files, "generic-close", GENERIC_CLOSE_OUTPUT);
}

/// [fn-effects] A `yield fn` may declare a **generic** effect: the machine takes
/// its handlers as ordinary parameters, rendered from the effect *type*, so
/// `Random<Int>` needs nothing beyond what any other effect needs. (The refusal
/// this replaces was about the per-effect-set trait naming, which R5 deleted.)
#[test]
fn a_machine_takes_a_generic_effect_handler() {
    let files = generate(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("fn __advance(&mut self, random_i32: &mut dyn Random<i32>)"),
        "expected the generic handler as a machine parameter in:\n{src}"
    );
    // The *builder* declares no effects, so nothing is threaded into it: the
    // drive site is what supplies the handler.
    assert!(
        src.contains("__Pass_Rolls::new(rolls(3))"),
        "expected the origin builder called without handlers in:\n{src}"
    );
}
