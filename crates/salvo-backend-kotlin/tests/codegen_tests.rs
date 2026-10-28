//! Kotlin codegen tests: golden snapshots of generated code, and what emit
//! data cannot say — hand-built companions and host files, `salvo platform
//! generate`'s skeletons, the warnings channel, checks scoped to one function
//! or spanning every emitted file, and the checked-in examples. One
//! construct's emitted text, and the programs the backend refuses, are emit
//! data ([test-emit-data], `tests/emit/`); compiling and running programs is
//! the run data's business ([test-run-data], `salvo-cli/tests/run/`), on both
//! backends at once.

use std::path::Path;

use salvo_core::{Program, SourceSet};


/// The demo program exercising M2 features: structs, defaults, spread/copy,
/// nullability + `is` with binding, string interpolation, effects (Console),
/// `use`, iterator functions with `yield`, `for`/`while` loops, and std
/// intrinsics.
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

// Its own pass, named `Upto` rather than `Range`: `std.core.range` exports a
// `Range` of its own [mod-export], and two same-named structs in two modules
// confuse the pass-driving resolution (an open defect — ROADMAP.md).
struct Upto {
    start: Int,
    end: Int
}

fn upto(start: Int, end: Int) -> Upto {
    return Upto {start: start, end: end}
}

iter fn iter(r: Upto) -> Emitted Int | Finished {
    state UptoIter {
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
    use StdConsole
    let person = Person {name: "Roland", surname: "Elliott", age: 36}
    greet(person)
    let anon = Person {...person, surname: None}
    greet(anon)
    // [col-of-nonempty] The constructor claims `NonEmpty`, so `first` answers
    // an element and needs no `!`.
    let names = List("a", "b")
    println("first: ${names.first()}")
    let mut_names = Mut List("x")
    mut_names.add("y")
    println("size: ${mut_names.size()}")
}
"#;

/// [platform-handler] std's own route: a `platform handler` declared in a
/// *std* module, implemented by a host companion std ships
/// (`lib/platform/std/core/…`) rather than one `salvo platform generate` writes.
/// This is FS-1's `HostRawFs` shape (FILE_SYSTEM.md §5.8) with a clock
/// standing in for the filesystem.
#[test]
fn a_std_platform_handler_is_supplied_by_a_shipped_companion() {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    // A std module — `std.core.*` is implicitly imported, so the program below
    // sees these names without an `import`.
    sources.add(
        "std/core/rawclock.sv",
        SourceSet::classify(Path::new("std/core/rawclock.sv")).unwrap(),
        "export effect RawClock {\n    fn raw_now() [] -> Int\n}\n\n\
         export platform handler HostRawClock of RawClock\n"
            .to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        "fn main() [use] -> None {\n    use StdConsole()\n    \
         use HostRawClock()\n    println(\"now=${raw_now()}\")\n}\n"
            .to_string(),
        false,
    );
    // The companion std ships for this backend, mounted exactly as a
    // customer's `platform/` file is [platform-tree].
    sources.add_companion(
        std::path::PathBuf::from("platform/std/core/rawclock.kt"),
        salvo_core::ModulePath(vec!["std".into(), "core".into(), "rawclock".into()]),
        "package salvo.platform.std.core.rawclock\n\nimport salvo.std.core.rawclock.*\n\n\
         class HostRawClock : RawClockPlatform {\n    override fun raw_now(): Int = 7\n}\n"
            .to_string(),
        true,
    );
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    // The drivers' pre-resolution expansions [actor-route-stub].
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let errors: Vec<_> = expansion.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "expansion errors: {errors:?}");
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: sources.companions,
    };
    let files = emit(&program)
        .unwrap_or_else(|errors| panic!("codegen errors:\n{}", errors.join("\n")));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content
            .contains("salvo.std.core.rawclock.__Platform_HostRawClock()"),
        "expected the shipped host class to be constructed, got:\n{}",
        main.content
    );
    // The companion travels into the output like any other.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "platform/std/core/rawclock.kt"),
        "expected std's host companion to be copied"
    );
    // And `salvo platform generate` writes nothing for it: std's host file
    // is shipped, not generated.
    let skeletons = salvo_backend_kotlin::platform_skeletons(&program)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert!(
        skeletons.is_empty(),
        "std host files are shipped, not generated: {:?}",
        skeletons
            .iter()
            .map(|f| f.rel_path.display().to_string())
            .collect::<Vec<_>>()
    );
}

/// [std-cache] A program of std and `extra`, over the process's std front
/// (`salvo_testkit::front`, TESTS.md C2): std was read, parsed, expanded and
/// checked once, and the program holds its very modules, so only `extra` is
/// parsed, expanded ([actor-route-stub], what `salvo compile` runs before
/// checking) and — through [`emit`] — checked.
fn build_program(extra: &[(&str, &str)]) -> Program {
    let built = salvo_testkit::front::program("kt", extra);
    let errors: Vec<_> = built.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse or expansion errors: {errors:?}");
    built.program
}

/// [std-cache] `emit_program` over the std front [`build_program`] built on.
fn emit(program: &Program) -> Result<Vec<salvo_backend_kotlin::EmittedFile>, Vec<String>> {
    let front = salvo_testkit::front::std_front("kt");
    salvo_backend_kotlin::emit_program_over(program, front)
}

/// Emits one source (plus std) — the Rust backend's `generate` by another
/// name, since this crate's `generate_demo` is fixed to `DEMO`.
fn generate_files(extra: &[(&str, &str)]) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(extra);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

fn generate_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", DEMO)]);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

#[test]
fn golden_demo_kotlin() {
    let files = generate_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

/// The M3 demo: Result-style unions with qualifiers, `when` exhaustiveness,
/// precise `is Err Str` checks, and flow narrowing. `ok`/`err` are
/// constructive-qualifier constructor functions (`-> T as Ok`).
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
    use StdConsole
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

fn generate_unions_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", UNIONS_DEMO)]);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

#[test]
fn golden_unions_kotlin() {
    let files = generate_unions_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

/// The M4 demo: predicate qualifiers (`qualifies` calls at runtime),
/// struct-field overrides with casts, qualifier-based overloading (with
/// erasure mangling), `while x is T`, and qualified union groups.
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
    use StdConsole
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

fn generate_qualifiers_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", QUALIFIERS_DEMO)]);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

#[test]
fn golden_qualifiers_kotlin() {
    let files = generate_qualifiers_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

/// [implicit-group] [iter-protocol] The composition rendering R5's
/// combinators are built on: a generic combinator takes the *pass* as its
/// subject and reaches its `next` through the `?Next<It, T>` spread. The
/// Kotlin half of the pair of bugs this exposed: an implicit member
/// returning a **union** rendered its Salvo type text into the Kotlin source
/// (`(It) -> Emitted T | Finished`), which is invalid Kotlin
/// [backend-never-wrong]. Same source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_combinator_over_a_next_spread`.

/// [iter-fn] The **release** of a minted pass, Kotlin half: the same
/// source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_an_abandoned_mint` — a combinator abandoning the
/// pass after one element still runs the origin's `defer`.

/// The M5 demo: generic effects with multiple instances in scope,
/// checker-driven disambiguation (explicit type args, expected type),
/// handler generic inference at `use` sites, and handler threading through
/// fn call sites — all resolved from the checker's effect tables.
const EFFECTS_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: List<T>, ?copy: (v: T) -> T) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        let value = get(values, i % values.size()) ?: unreachable!()
        i = i + 1
        return copy(value)
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
    use StdConsole
    use CyclicRandom(List(10, 20, 30))
    use CyclicRandom(List("a", "b"))
    draw()
    draw()
    println("lucky: ${next_random<Int>()}")
    println("again: ${lucky_number()}")
}
"#;

fn generate_effects_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", EFFECTS_DEMO)]);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

#[test]
fn golden_effects_kotlin() {
    let files = generate_effects_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// ===== the defect round of 2026-09-25, byte-identical to the Rust side =====

// ===== M6: loops as values =====

/// The M6 demo: loop values from body tails, `break value`, loop `else`
/// in value and statement position, bare `break` (optional value), and a
/// union-typed loop value re-wrapped to the declared type [loop-while-value].
const LOOPS: &str = r#"
// Its own pass, named `Upto` rather than `Range`: `std.core.range` exports a
// `Range` of its own [mod-export], and two same-named structs in two modules
// confuse the pass-driving resolution (an open defect — ROADMAP.md).
struct Upto {
    start: Int,
    end: Int
}

fn upto(start: Int, end: Int) -> Upto {
    return Upto {start: start, end: end}
}

iter fn iter(r: Upto) -> Emitted Int | Finished {
    state UptoIter2 {
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
    use StdConsole

    // Last evaluated body expression is the loop's value.
    let i = 0
    let last = while i++ < 4 {
        i * 10
    } else {
        -1
    }
    println("last: ${last}")

    // `else` provides the value when the loop never runs.
    let j = 9
    let never = while j < 3 {
        j
    } else {
        -1
    }
    println("never: ${never}")

    // `break value` short-circuits; no `else` makes the value optional.
    let found = for x in List(0, 1, 2, 3, 4, 5) {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    // Statement-position loop with `else`, over a producer.
    for x in upto(0, 0) {
        println("unreachable")
    } else {
        println("empty range")
    }

    // A bare `break` keeps the previous iteration's value.
    let k = 0
    let capped = while k < 5 {
        k++
        if k == 3 {
            break
        }
        k
    }
    println("capped: ${capped ?: unreachable!()}")

    // Loop values join into unions and re-wrap to the declared type.
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

fn generate_loops_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", LOOPS)]);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

#[test]
fn golden_loops_kotlin() {
    let files = generate_loops_demo();
    let combined: String = files
        .iter()
        .map(|f| format!("// ===== {} =====\n{}", f.rel_path.display(), f.content))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(combined);
}

// ===== M7: reachability, packages/imports, companions =====

/// A multi-module program: `main` uses `geometry` (imported) but not
/// `unused`; `geometry` has a Kotlin companion file.
fn build_multi_module() -> Program {
    let main = r#"
import geometry.area

fn main() [use] -> None {
    use StdConsole
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
    let mut program = build_program(&[
        ("main.sv", main),
        ("geometry.sv", geometry),
        ("unused.sv", unused),
    ]);
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("geometry_helpers.kt"),
        module: salvo_core::ModulePath(vec!["geometry".into()]),
        content: "package salvo.geometry\n\nfun helper(): Int = 1\n".to_string(),
        platform: false,
    });
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("unused_helpers.kt"),
        module: salvo_core::ModulePath(vec!["unused".into()]),
        content: "package salvo.unused\n".to_string(),
        platform: false,
    });
    program
}

// [mod-used-only] [backend-companion]
#[test]
fn only_used_modules_are_transpiled() {
    let program = build_multi_module();
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().into_owned())
        .collect();
    // `main` and its dependency are emitted; the unused user module and
    // unused std modules are not.
    assert!(paths.contains(&"main.kt".to_string()), "{paths:?}");
    assert!(paths.contains(&"geometry.kt".to_string()), "{paths:?}");
    assert!(!paths.contains(&"unused.kt".to_string()), "{paths:?}");
    // The reachable module's companion is copied; the unreachable one not.
    assert!(
        paths.contains(&"geometry_helpers.kt".to_string()),
        "{paths:?}"
    );
    assert!(
        !paths.contains(&"unused_helpers.kt".to_string()),
        "{paths:?}"
    );
}

// [kt-package] [kt-imports]
#[test]
fn modules_get_packages_and_generated_imports() {
    let program = build_multi_module();
    let files = emit(&program).unwrap();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(main.content.starts_with("package salvo.main\n"));
    let geometry = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "geometry.kt")
        .unwrap();
    assert!(geometry.content.starts_with("package salvo.geometry\n"));
    let imports: Vec<&str> = geometry.content.lines().filter(|l| l.starts_with("import ")).collect();
    assert_eq!(imports, vec!["import salvo.*"], "{}", geometry.content);
}

// [var-let-destructure] Two struct destructures in one block get unique temps.
#[test]
fn struct_destructure_temps_are_unique() {
    let src = r#"
export struct Point {
    x: Int,
    y: Int
}

export fn main() [use] -> None {
    use StdConsole
    let {x} = Point {x: 1, y: 2}
    let {y} = Point {x: 3, y: 4}
    println("${x} ${y}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // [var-let-destructure] Each destructure has its own temporary.
    let temps: Vec<&str> = main.content.lines().filter(|l| l.trim_start().starts_with("val __destructured_")).collect();
    assert_eq!(temps.len(), 2, "{}", main.content);
    assert_ne!(temps[0].split(':').next(), temps[1].split(':').next(), "{}", main.content);
}

// [backend-companion] A companion must not collide with a generated file.
#[test]
fn companion_collision_with_generated_file_is_an_error() {
    let mut program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdConsole\n    println(\"hi\")\n}\n",
    )]);
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: "package salvo.main\n".to_string(),
        platform: false,
    });
    let errors = emit(&program)
        .err()
        .expect("expected collision error");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("companion file `main.kt` collides")),
        "unexpected errors: {errors:?}"
    );
}


// ===== S1: the `copy` intrinsic [platform-intrinsic-fn] [copy-fn] [kt-copy] =====

// ===== bare `return` in value-position blocks of iterator bodies =====
// [iter-protocol] [kt-iter-pass]

// ===== interception [effect-intercept] =====
// A handler may depend on the effect it implements, binding strictly
// outward, and a `use` may shadow an earlier registration [effect-use-no-dup].
// Kotlin needs no new machinery for either — a dependency is a constructor
// field, and the fused class keeps one property per instance — but the
// *lookups* had to become innermost-first: reading the environment as a set
// made a shadowed handler answer calls the inner one owned, which printed
// the outer greeting here and the inner one on Rust (found 2026-09-14).
// The program and its expected stdout are shared with the Rust backend.

// ===== the interceptor chain [effect-handle] =====
// The production shape the 2026-09-20 arc was built for:
// a stateful handler binds as a monitor, a stateless dependent
// handler binds bare with its dependencies injected at construction — one
// of them the effect it implements (interception binds outward), another a
// monitor. The program and its stdout are shared with the Rust backend.

// ===== spawn-inheritance and the `with` clause [actor-spawn-inherit] =====
// The arc's shape, shared verbatim with the Rust backend (whose test also
// pins the hidden handle bundle that side needs; here a JVM reference already
// is a handle, so nothing extra is emitted).

// ===== effect dependencies on handlers [effect-handler-deps] =====
// A handler constructor parameter of effect type is a dependency: the
// member body may use that effect, the `use` site supplies it from scope,
// and callers of the outer effect never mention it.

// [effect-handler-deps] The same programs the Rust backend runs through its
// own emission, asserting the *same* stdout here: that is what backend
// parity means for handler dependencies. Kotlin needs no fusion — objects
// alias — so these also pin that the two strategies agree on handler state,
// dependency chains, `use` in a loop, and nested effect calls.

// ===== E3 step 2: throw and `try` [throw] [throw-try] [kt-throw-signal] =====

/// The whole of non-resumption in one program: propagation through a frame
/// that declares the effect, a linear resource released *before* the call that
/// may throw (which is the only way since `defer` was removed, 2026-09-10 —
/// the checker rejects holding it across the call), two message types meeting
/// at one delimiter
/// (`Thrown (Str | Int)`), a may-throw call inside a loop, and a nested
/// delimiter that must not swallow the outer throw.
const THROW_DEMO: &str = r#"
import std.throw

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
    use StdConsole
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
        total(array_of("ab", "cdefg"))
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

/// [kt-throw-signal] The JVM's unwinding *is* the propagation: `throw`
/// throws a generated stack-trace-less signal, an intermediate frame does
/// nothing at all (no `ControlFlow`, no colouring), and `try` is Kotlin's
/// own `try`/`catch` *expression*, so the outcome falls out of it. `Throw`
/// is never a handler parameter.
#[test]
fn throw_lowers_to_a_signal_and_try_to_a_catch() {
    let program = build_program(&[("main.sv", THROW_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The rest is emit data (`tests/emit/throwing/throw.sv`); a check scoped
    // to one function is not.
    // A propagating frame carries nothing: `parse` is called plainly.
    let measure = main
        .content
        .split("fun measure")
        .nth(1)
        .and_then(|s| s.split("\nfun ").next())
        .unwrap();
    assert!(
        measure.contains("val n: Int = salvo.main.parse(console, line)"),
        "expected plain propagation in:\n{measure}"
    );
}

// ===== the subject-less `when` [when-condition] [kt-when-cond] =====

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
    use StdConsole
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

/// [kt-when-cond] Kotlin has the same construct, so the source shape
/// survives: `cond -> { … }` arms closed by `else -> { … }`. Being total it
/// is an expression without the `else null` filler an `if` chain needs
/// [expr-if-else-none].
#[test]
fn a_subjectless_when_emits_a_subjectless_kotlin_when() {
    let program = build_program(&[("main.sv", WHEN_COND_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    let classify = main
        .content
        .split("fun classify")
        .nth(1)
        .and_then(|s| s.split("\nfun ").next())
        .expect("classify emitted");
    // [ir-branch] A subjectless `when` is a branch chain: an `if` expression
    // whose conditions are the heads, closed by `else` — total, so no
    // optional filler.
    assert!(
        classify.contains("return (if ((n < 0)) {") && classify.contains("} else if ((n == 0)) {"),
        "expected the branch chain:\n{classify}"
    );
    assert!(
        classify.contains("} else {") && !classify.contains("null"),
        "expected a plain `else` arm and no optional filler:\n{classify}"
    );
    // The `is` binding is emit data (`tests/emit/control/when.sv`).
}

// ===== platform handlers [platform-handler] =====

/// [platform-handler] The phase-4 shape in miniature (FILE_SYSTEM.md §5.8):
/// a host handler of an ordinary effect at the bottom, an ordinary Salvo
/// handler depending on it above, and application code that names neither —
/// `HostRawClock` stands in for `HostRawFs`, `DefaultClock` for `DefaultFs`.
/// The dependency also switches the program into the fused emission, so the
/// `use` of a platform handler is exercised on the path phase 4 will take.
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
    use StdConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
"#;

fn platform_handler_program() -> Program {
    build_program(&[("main.sv", PLATFORM_HANDLER_DEMO)])
}

/// [platform-handler] [platform-tree] The skeleton `salvo platform generate`
/// writes for the demo's host handler.
fn platform_handler_skeleton() -> salvo_backend_kotlin::EmittedFile {
    let program = platform_handler_program();
    let mut files = salvo_backend_kotlin::platform_skeletons(&program)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares a platform handler");
    files.remove(0)
}

fn generate_platform_handler_demo_with(
    host: &str,
) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let mut program = platform_handler_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// [platform-handler] [kt-platform-handler] What the compiler emits for a
/// platform handler: the effect's `interface` as for any effect, **no class**
/// — the class is the host's — and a `use` site that constructs the host
/// class by its fully-qualified name, since the host package is nobody's
/// import. `main` stays the program's entry point: unlike a `platform
/// effect`, nothing arrives from outside.
#[test]
fn a_platform_handler_emits_no_class_and_a_host_constructor() {
    let files = generate_platform_handler_demo_with(&platform_handler_skeleton().content);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt should be generated");
    let src = &main.content;
    assert!(
        src.contains("interface RawClock {") && src.contains("fun rawNow(): Int"),
        "expected the generated interface, got:\n{src}"
    );
    assert!(
        !src.contains("class HostRawClock"),
        "a platform handler must not emit a class of its own:\n{src}"
    );
    assert!(
        src.contains("= __Platform_HostRawClock(offset = 35)") && src.contains("salvo.platform.main.HostRawClock(offset)"),
        "expected the `use` site to construct the adapter over the host class, got:\n{src}"
    );
    // The ordinary handler beside it is still emitted, and `main` is still
    // the entry point.
    assert!(
        src.contains("class DefaultClock") && src.contains("fun main()"),
        "expected the Salvo handler and a generated `main`, got:\n{src}"
    );
}

/// [platform-handler] [platform-tree] The skeleton: a class named after the
/// *handler* (the `use` site constructs that name), taking the handler's
/// constructor parameters, implementing the ordinary effect's generated
/// interface with every member stubbed. No `main` is generated — the host
/// owns no entry point here.
#[test]
fn platform_generate_renders_a_host_handler_skeleton() {
    let file = platform_handler_skeleton();
    assert_eq!(file.rel_path.to_string_lossy(), "platform/main.kt");
    let src = &file.content;
    for expected in [
        "package salvo.platform.main",
        "import salvo.main.*",
        "class HostRawClock(private val offset: Int) : RawClockPlatform {",
        "override fun rawNow(): Int {",
        "TODO(\"implement RawClock.raw_now\")",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("fun main()"),
        "a platform handler does not move the entry point:\n{src}"
    );
}

/// [platform-handler] End to end: the generated skeleton with its one stub
/// filled in, compiled and run by kotlinc. The asserted stdout is
/// byte-identical to the Rust backend's run of the same program.
#[test]
fn a_platform_handler() {
    let skeleton = platform_handler_skeleton();
    let host = skeleton.content.replace(
        "TODO(\"implement RawClock.raw_now\")",
        "return offset + 7",
    );
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
}

// ===== [platform-threadsafe] the thread-safety contract =====

/// [platform-threadsafe] [kt-platform-handler] An **undeclared** platform
/// handler binds as a monitor (user decision 2026-09-26): the `use` site wraps
/// the host in the effect's `synchronized` wrapper, so this backend now
/// serializes the instance exactly as Rust does — the divergence closed.
#[test]
fn an_undeclared_platform_handler_is_serialized_behind_the_monitor() {
    let files = generate_platform_handler_demo_with(&platform_handler_skeleton().content);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt should be generated");
    let src = &main.content;
    assert!(
        src.contains("= __Platform_HostRawClock(offset = 35)") && src.contains(": RawClock = __Mon_RawClock(__use_"),
        "expected the monitor wrapper around the undeclared host, got:\n{src}"
    );
}

/// The demo with the contract declared: the host promises concurrent safety,
/// so the compiler binds the raw instance.
const THREADSAFE_PLATFORM_HANDLER_DEMO: &str = r#"
effect RawClock {
    fn raw_now() [] -> Int
}

threadsafe platform handler HostRawClock(offset: Int) of RawClock

effect Clock {
    fn stamp(label: Str) -> Str => label
}

handler DefaultClock [RawClock] of Clock {
    fn stamp(label: Str) -> Str => label {
        return "${label}@${raw_now()}"
    }
}

fn main() [use] {
    use StdConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
"#;

fn threadsafe_platform_handler_program() -> Program {
    build_program(&[("main.sv", THREADSAFE_PLATFORM_HANDLER_DEMO)])
}

fn threadsafe_platform_handler_skeleton() -> salvo_backend_kotlin::EmittedFile {
    let program = threadsafe_platform_handler_program();
    let mut files = salvo_backend_kotlin::platform_skeletons(&program)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares a platform handler");
    files.remove(0)
}

fn generate_threadsafe_platform_handler_demo_with(
    host: &str,
) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let mut program = threadsafe_platform_handler_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// [platform-threadsafe] [kt-handle] A **`threadsafe`** platform handler
/// binds the raw host instance — no `__Mon_E` — because the declared contract
/// is exactly "stateless for the handle's purposes"; an undeclared one binds
/// behind the monitor. The skeleton prints the contract the implementer
/// signs, and the undeclared skeleton says what the compiler does.
#[test]
fn a_threadsafe_platform_handler_binds_raw_and_its_skeleton_prints_the_contract() {
    let skeleton = threadsafe_platform_handler_skeleton();
    let files = generate_threadsafe_platform_handler_demo_with(&skeleton.content);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt should be generated");
    let src = &main.content;
    assert!(
        src.contains("= __Platform_HostRawClock(offset = 35)") && !src.contains("= __Mon_RawClock("),
        "a threadsafe host binds raw, got:\n{src}"
    );
    for expected in [
        "`threadsafe platform handler HostRawClock` — THE CONTRACT YOU ARE SIGNING",
        "NO\n// lock around it",
        "class HostRawClock(private val offset: Int) : RawClockPlatform {",
        "[platform-threadsafe]",
    ] {
        assert!(
            skeleton.content.contains(expected),
            "expected `{expected}` in:\n{}",
            skeleton.content
        );
    }
    let plain = platform_handler_skeleton();
    assert!(
        plain.content.contains("the compiler SERIALIZES this instance"),
        "expected the serialized contract on the undeclared skeleton:\n{}",
        plain.content
    );
}

/// [platform-threadsafe] End to end under kotlinc with the threadsafe
/// skeleton's stub filled in — the same output as the undeclared run and as
/// the Rust backend's.
#[test]
fn a_threadsafe_platform_handler() {
    let skeleton = threadsafe_platform_handler_skeleton();
    let host = skeleton.content.replace(
        "TODO(\"implement RawClock.raw_now\")",
        "return offset + 7",
    );
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
}

// ===== the network sequence, step ①: `net` and `MemTransport` =====

/// The Rust backend's `NET_MEM_TRANSPORT`, verbatim, with the same expected
/// output — the parity assertion for the wire's double.
const NET_MEM_TRANSPORT: &str = r#"import std.net

actor effect Counting {
    send fn total(out: Reply<Str>) => !out
}

// The receiver records rather than prints: a `deliver` answers as soon as
// the frame is handed over, so a print here would race main's own.
handler Tallying(label: Str) of Inbound, Counting {
    mailbox { capacity: 16 }
    state {
        seen: Mut List<Str> = Mut List<Str>()
    }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        seen.add("${label} <- ${to_str(from)}: ${str_of_bytes(frame) ?: "?"}")
    }

    send fn total(out: Reply<Str>) => !out {
        out.send(join(seen, ", "))
    }
}

fn report(label: Str, outcome: Ok None | Err NetError) [Console] -> None => label, outcome {
    when outcome {
        is Ok { println("${label}: sent") }
        is Err {
            let e: NetError = outcome
            println("${label}: ${to_str(e)}")
        }
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let c = NodeEndpoint { host: "c", port: 9 }
    let workers = pool(2)
    let net = spawn MemNetwork() on workers
    use MemTransport(copy(a), copy(net))

    let (in_a, _count_a) = spawn Tallying("a") on workers
    let (in_b, count_b) = spawn Tallying("b") on workers
    report("listen a", listen(copy(a), in_a))
    report("listen b", listen(copy(b), in_b))

    report("hello", deliver(copy(b), to_bytes("hello")))
    net.partition(copy(a), copy(b))
    report("lost", deliver(copy(b), to_bytes("lost")))
    net.heal(copy(a), copy(b))
    report("again", deliver(copy(b), to_bytes("again")))
    report("void", deliver(copy(c), to_bytes("void")))
    net.kill(copy(b))
    report("dead", deliver(copy(b), to_bytes("dead")))

    let got = waitfor out: Reply<Str> { count_b.total(out) }
    let routed = waitfor out: Reply<Int> { net.delivered(out) }
    println("b saw [${got}], routed ${routed}")
}
"#;

/// [platform-threadsafe] A program that never binds `HostTcpTransport`
/// constructs no host class (the shipped companion is copied with `net`, as
/// every companion is, but nothing names it).
#[test]
fn an_unused_threadsafe_platform_handler_is_never_constructed() {
    let files = generate_files(&[("main.sv", NET_MEM_TRANSPORT)]);
    let all: String = files.iter().map(|f| f.content.as_str()).collect();
    assert!(
        !all.contains("salvo.std.net.__Platform_HostTcpTransport("),
        "an unused platform handler was constructed:\n{all}"
    );
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
    use StdConsole()
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    println("after add: ${count(xs)}")
    refill(xs, 2)
    println("after refill: ${count(xs)}")
}
"#;

/// [qual-refn] [qual-erasure] The refined program compiles and runs, and the
/// emitted Kotlin carries no trace of the refinement.
#[test]
fn a_refined_program() {
    let program = build_program(&[("main.sv", REFN_DEMO)]);
    let files = emit(&program)
        .unwrap_or_else(|errors| panic!("codegen errors:\n{}", errors.join("\n")));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted");
    // The qualifier's `qualifies` fn is still emitted (it backs `is`
    // checks), but a refinement is *trusted* like `-> T as Q`
    // [qual-ctor-fn]: no runtime check is emitted where it applies.
    let body = main
        .content
        .split("fun main(")
        .nth(1)
        .expect("main emitted");
    assert!(
        !body.contains("NonEmpty_qualifies"),
        "a refinement must not emit a runtime check:\n{body}"
    );
}

/// [qual-refn] [diag-structured] A suppressed refinement conflict is
/// a *warning*: the program is legal, so it must still emit — *and* the
/// warning must reach the driver, or the diagnostic exists only in `salvo
/// analyze`. Tested per backend because both the gate and the channel are
/// duplicated in each: the same condition has to fire on both sides.
// `with NonEmpty` on both: std's own `NonEmpty` refines `add` too
// [col-nonempty], so without it the disagreement would be three-way and this
// test would be asserting std's presence rather than the rule. Declaring
// compatibility with std's claim is the one-word remedy a user reaches for —
// it leaves exactly the Q1-vs-Q2 conflict this test is about.
const REDUNDANT_SELECTOR_DEMO: &str = r#"
fn main() [use] {
    use StdConsole()
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    add(xs, 2)
    // `@core.list` changes nothing here: there is one `size` that fits and
    // nothing refines it, so the selector is noise [fn-overload-at]. (`add`
    // was the example until `Idx` moved to `std.core.index`: refined in two
    // places, its selector chooses whose refinements apply.)
    println("size: ${size@core.list(xs)}")
}
"#;

#[test]
fn a_redundant_selector_warns_without_stopping_emission() {
    let program = build_program(&[("main.sv", REDUNDANT_SELECTOR_DEMO)]);
    let (files, warnings) = salvo_backend_kotlin::emit_program_reporting(&program)
        .unwrap_or_else(|errors| panic!("a warning must not stop emission: {errors:?}"));
    assert!(!files.is_empty(), "the program should still emit");
    assert_eq!(warnings.len(), 1, "warnings: {warnings:?}");
    assert!(
        warnings[0].starts_with("warning: this `@core.list` is not needed")
            && warnings[0].contains("main.sv:"),
        "a rendered warning with its location: {}",
        warnings[0]
    );
}


// ===== [implicit-group] [rs-seq]-equivalent: the sequence functions =====

/// `map`/`filter`/`reduce` over a `List` (the intrinsic fast path), and over
/// the *passes* an array, a `Str`, an origin and a struct of the program's own
/// hand out — `iter` written at each use site [seq-iterator]. Shared with the Rust
/// backend's `rustc_compiles_and_runs_sequences`: the *checker* picks the
/// overloads, so the two targets must agree element for element.
const SEQ_DEMO: &str = r#"
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListNext<Int> => bag {
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

iter fn iter(n: Naturals) -> Emitted Int | Finished {
    state NaturalsIter2 {
        at = n.from
    }
    if at >= n.from + 4 {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3, 4)
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
    let names = List("ann", "bob", "carol")
    let lens = map(names, n -> size(n))
    let long = filter(names, n -> size(n) > 3)
    println("names: ${size(lens)} ${size(long)}")
    let bag = Bag {items: List(5, 6)}
    println("bag: ${reduce(iter(bag), 0, (a, b) -> a + b)}")
    // [iter-mint] `for` over a *container of one's own*: the loop calls its
    // `iter` once and drives the pass that answers. Until 2026-09-09 the
    // checker recorded that mint and neither emitter made the call, so this
    // shape emitted code the target compiler rejected.
    // (a second bag, because this demo's `iter` *moves* its container — the
    // one above was consumed by the `reduce`.)
    let more = Bag {items: List(5, 6)}
    for n in more {
        println("bag element ${n}")
    }
}
"#;

/// [kt-seq] [seq-iterator] A `fns` group emits nothing, and the `List` fast
/// paths are std functions (`filter` a platform fn over Kotlin's own); the generic body is a
/// plain generic function whose implicit parameter arrives as a trailing
/// argument — an *adapter lambda* when the resolved `iter` is an intrinsic,
/// since an intrinsic has no Kotlin name to reference [implicit-intrinsic].
#[test]
fn sequence_functions_lower_to_collection_operations() {
    let program = build_program(&[("main.sv", SEQ_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    // The lowering itself is emit data (`tests/emit/iteration/sequences.sv`);
    // a check over every emitted file, std's included, is not.
    // A `fns` group is not a value: nothing *declares* `Next`.
    assert!(
        !files.iter().any(|f| f.content.contains("class Next")
            || f.content.contains("interface Next")
            || f.content.contains("object Next")),
        "a `fns` group must emit nothing"
    );
}

// ===== [iter-fn] the generated-pass form =====

// ===== the checked-in examples (examples/README.md) =====

/// The repository's `examples/` directory.
fn examples_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Every example directory, by name, in a stable order.
/// Runs [check] for every example at once, one thread each: emission is the
/// cost and the examples share nothing (ROADMAP 0b). A failing example's
/// panic is printed under its thread and fails the test.
fn for_each_example(check: impl Fn(String) + Sync) {
    std::thread::scope(|scope| {
        for example in example_names() {
            let check = &check;
            scope.spawn(move || check(example));
        }
    });
}

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

/// Emits one example's Kotlin, exactly as `salvo compile` would.
fn emit_example(example: &str) -> Vec<salvo_backend_kotlin::EmittedFile> {
    // [std-cache] std is the process's front (`salvo_testkit::front`); the
    // dependencies and the example's tree come after it.
    let mut sources = SourceSet::default();
    // [manifest-deps] The example's manifest names its dependencies, loaded
    // between std and the example's own tree as `salvo compile` does.
    let project = salvo_core::Project::load(&examples_dir().join(example).join("salvo.toml"))
        .unwrap_or_else(|e| panic!("examples/{example}/salvo.toml: {e}"));
    for dep in project.dependencies().unwrap_or_else(|e| panic!("{e}")) {
        let errors = sources.add_dependency(&dep, "kt");
        assert!(errors.is_empty(), "failed to load dependency `{}`: {errors:?}", dep.name);
    }
    let src = examples_dir().join(example).join("salvo");
    let errors = sources.add_dir(&src, "kt", false, false);
    assert!(errors.is_empty(), "failed to read {example}: {errors:?}");
    // The drivers' pre-resolution expansions [actor-route-stub].
    let built = salvo_testkit::front::std_front("kt").program(sources.files, sources.companions);
    let errors: Vec<_> = built.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse or expansion errors: {errors:?}");
    let program = built.program;
    emit(&program).unwrap_or_else(|errors| {
        panic!("`examples/{example}` no longer compiles:\n{}", errors.join("\n"))
    })
}

/// The generated tree checked in beside each example is what the
/// compiler writes *today* — "stale generated code is worse than none: it is
/// read as what the compiler does" (examples/README.md). A text comparison, so
/// it needs no toolchain, and it also catches an example whose *source* stopped
/// checking: emission then fails outright.
///
/// Added 2026-09-16, after `examples/effects/` was found broken since
/// 2026-09-15 — nothing in the suite read the examples at all.
#[test]
fn every_examples_checked_in_kotlin_is_current() {
    for_each_example(|example| {
        let files = emit_example(&example);
        let root = examples_dir().join(&example).join("kotlin");
        for file in &files {
            let path = root.join(&file.rel_path);
            let found = std::fs::read_to_string(&path).unwrap_or_else(|_| {
                panic!(
                    "examples/{example}/kotlin/{} is missing: regenerate with \
                     `cargo run -- compile --backend kotlin --src \
                     examples/{example}/salvo --target examples/{example}/kotlin`",
                    file.rel_path.display()
                )
            });
            if found != file.content {
                let at = found
                    .lines()
                    .zip(file.content.lines())
                    .position(|(a, b)| a != b)
                    .map(|i| i + 1);
                panic!(
                    "examples/{example}/kotlin/{} is stale (first difference at line \
                     {}): regenerate with `cargo run -- compile --backend kotlin --src \
                     examples/{example}/salvo --target examples/{example}/kotlin`",
                    file.rel_path.display(),
                    at.map(|l| l.to_string()).unwrap_or_else(|| "end of file".into())
                );
            }
        }
    });
}
