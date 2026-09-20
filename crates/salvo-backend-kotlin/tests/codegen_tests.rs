//! Kotlin codegen tests: golden snapshots of generated code, and (when
//! `kotlinc` is on PATH) a full compile-and-run verification.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

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
    for i in range(1, 4) {
        println("  ${i}: ${person.age + i}")
    }
}

struct Range {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> Range {
    return Range {start: start, end: end}
}

iter fn next(r: Range) -> Emitted Int | Finished {
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
    let names = list_of("a", "b")
    println("first: ${names.first()!}")
    let mut_names = mut_list_of("x")
    mut_names.add("y")
    println("size: ${mut_names.size()}")
}
"#;

/// [platform-handler] std's own route: a `platform handler` declared in a
/// *std* module, implemented by a host companion std ships
/// (`std/platform/core/…`) rather than one `salvo platform generate` writes.
/// This is FS-1's `HostRawFs` shape (FILE_SYSTEM.md §5.8) with a clock
/// standing in for the filesystem.
#[test]
fn a_std_platform_handler_is_supplied_by_a_shipped_companion() {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    // A std module — `core.*` is implicitly imported, so the program below
    // sees these names without an `import`.
    sources.add(
        "std/core/rawclock.sv",
        SourceSet::classify(Path::new("core/rawclock.sv")).unwrap(),
        "export effect RawClock {\n    fn raw_now() [] -> Int\n}\n\n\
         export platform handler HostRawClock of RawClock\n"
            .to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        "fn main() [use] -> None {\n    use StdOutConsole()\n    \
         use HostRawClock()\n    println(\"now=${raw_now()}\")\n}\n"
            .to_string(),
        false,
    );
    // The companion std ships for this backend, mounted exactly as a
    // customer's `platform/` file is [platform-tree].
    sources.add_companion(
        std::path::PathBuf::from("platform/core/rawclock.kt"),
        salvo_core::ModulePath(vec!["core".into(), "rawclock".into()]),
        "package salvo.platform.core.rawclock\n\nimport salvo.core.rawclock.*\n\n\
         class HostRawClock : RawClock {\n    override fun raw_now(): Int = 7\n}\n"
            .to_string(),
        true,
    );
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
    let files = salvo_backend_kotlin::emit_program(&program)
        .unwrap_or_else(|errors| panic!("codegen errors:\n{}", errors.join("\n")));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content
            .contains("salvo.platform.core.rawclock.HostRawClock()"),
        "expected the shipped host class to be constructed, got:\n{}",
        main.content
    );
    // The companion travels into the output like any other.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "platform/core/rawclock.kt"),
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

fn build_program(extra: &[(&str, &str)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true);
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

/// Emits one source (plus std) — the Rust backend's `generate` by another
/// name, since this crate's `generate_demo` is fixed to `DEMO`.
fn generate_files(extra: &[(&str, &str)]) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(extra);
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

fn generate_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", DEMO)]);
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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

// [effect-available] [effect-fn-deps]
#[test]
fn missing_effect_handler_is_an_error() {
    let program = build_program(&[(
        "bad.sv",
        "fn main() [use] -> None {\n    println(\"no console handler used\")\n}\n",
    )]);
    let result = salvo_backend_kotlin::emit_program(&program);
    let errors = result.err().expect("expected codegen errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("no handler for effect `Console`")),
        "unexpected errors: {errors:?}"
    );
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

fn generate_unions_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", UNIONS_DEMO)]);
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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

// [kt-union-wrappers] [union-arm-identity]
#[test]
fn unions_emit_sealed_wrappers() {
    let files = generate_unions_demo();
    let unions = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "unions.kt")
        .expect("unions.kt should be generated");
    assert!(unions.content.contains("sealed interface Union2"));
    assert!(unions.content.contains("sealed interface Union3"));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // Wrap at return boundaries, positional arm identity (the constructor
    // call is wrapped into the union arm).
    assert!(main.content.contains("return U2_1<Int, String>(ok(input))"));
    assert!(main
        .content
        .contains("return U2_2<Int, String>(err(\"negative age\"))"));
    // Qualifier-tagged wrap picks the right arm of the 3-union.
    assert!(main
        .content
        .contains("U3_1<String, String, Boolean>(ok(\"yes\"))"));
    // Precise `is Err Str` tests a single arm; `is Ok` another.
    assert!(main.content.contains("precise is U3_2<*, *, *>"));
    assert!(main.content.contains("precise is U3_1<*, *, *>"));
    // `when` lowers to a sealed when with unwrapped uses.
    assert!(main.content.contains("when (result) {"));
    assert!(main.content.contains("is U2_1<*, *> ->"));
    assert!(main.content.contains("(result.value as Int)"));
}

// [when-exhaustive]
#[test]
fn when_must_be_exhaustive() {
    let src = r#"
export qualifier Ok<T> of T
export qualifier Err<T> of T

export fn f(x: Ok Int | Err Str) -> Int {
    let v = when x {
        is Ok {
            1
        }
    }
    return v
}
"#;
    let program = build_program(&[("bad.sv", src)]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("non-exhaustive `when`")),
        "unexpected errors: {errors:?}"
    );
}

// [when-union-subject]
#[test]
fn when_requires_union_subject() {
    let src = "export fn f(x: Int) -> None {\n    when x {\n        is Int {\n            x\n        }\n    }\n}\n";
    let program = build_program(&[("bad.sv", src)]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("union-typed subject")),
        "unexpected errors: {errors:?}"
    );
}

// [union-arm-identity]
#[test]
fn union_wrap_requires_matching_arm() {
    let src = r#"
export qualifier Ok<T> of T
export qualifier Err<T> of T

export fn err<T>(value: T) -> T as Err {
    return value
}

export fn f() -> Ok Int | Err Str {
    return err(true)
}
"#;
    let program = build_program(&[("bad.sv", src)]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("no arm of")),
        "unexpected errors: {errors:?}"
    );
}

/// Full verification of the unions demo under kotlinc (skipped when kotlinc
/// is not installed).
fn kotlinc_compiles_and_runs_unions() -> KotlinCase {
    let files = generate_unions_demo();
    let expected = "age 36\nerror: negative age\nok: yes\nvalue plus one is 37\n";
    kotlin_case(files, "unions", expected)
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

fn generate_qualifiers_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", QUALIFIERS_DEMO)]);
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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

// [is-qualifies] [kt-qual-mangling] [qual-field-override]
#[test]
fn qualifiers_lower_to_predicates_and_mangled_overloads() {
    let files = generate_qualifiers_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // Predicate qualifiers become top-level `qualifies` functions.
    assert!(main
        .content
        .contains("fun Surname_qualifies(person: Person): Boolean"));
    assert!(main
        .content
        .contains("fun Positive_qualifies(int: Int): Boolean"));
    // `is` predicate checks call them.
    assert!(main.content.contains("if (Surname_qualifies(person))"));
    assert!(main.content.contains("if (Positive_qualifies(n))"));
    // The qualified overload is mangled; the checker routes the narrowed
    // call to it and the unqualified call to the base name.
    assert!(main
        .content
        .contains("fun full_name__Surname(person: Person): String"));
    assert!(main
        .content
        .contains("println(console, full_name__Surname(person))"));
    assert!(main.content.contains("println(console, full_name(person))"));
    // Field overrides cast + assert at the access site.
    assert!(main.content.contains("(person.surname as String)"));
    // `while x is T` lowers with a per-iteration binding.
    assert!(main.content.contains("while (current != null) {"));
    assert!(main.content.contains("val c = current as Int"));
    // A qualified union group is physically the inner union.
    assert!(main
        .content
        .contains("val back: Union2<String, Int> = (nested.value as Union2<String, Int>)"));
}

/// Full verification of the qualifiers demo under kotlinc (skipped when
/// kotlinc is not installed).
fn kotlinc_compiles_and_runs_qualifiers() -> KotlinCase {
    let files = generate_qualifiers_demo();
    let expected = "Roland Elliott\nAnon\n5 is positive\n-2 is not positive\n\
                    tick 3\ntick 2\ntick 1\ninner ok: yes\n";
    kotlin_case(files, "qualifiers", expected)
}

/// Runs the checker on a source and returns the errors (panics if none).
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src)]);
    salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors")
}

/// [iter-protocol] A hand-written **pass** driven by `for`: the same source
/// and the same expected stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_hand_written_pass`. `zip` is the point — it
/// reads two sources at once, which `yield` cannot express.
const PASS_DEMO: &str = r#"
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
        // `get` hands back borrows [copy-opt-in]: building a tuple *stores*
        // the element, so the copy is written where it happens.
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

const PASS_OUTPUT: &str = "n 3\nn 2\nn 1\nada is 36\ngrace is 45\ndone\n";

/// Kotlin has no pattern-matching loop condition, so the driving loop is
/// `while (true)` plus a guard. The arm is spelled with its *real* type
/// arguments where it can be (a non-generic `next`), which is what keeps the
/// element read free of an unchecked cast — a warning in code the user cannot
/// edit.
#[test]
fn a_pass_lowers_to_a_guarded_while_loop() {
    let program = build_program(&[("main.sv", PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains("_pass = countdown(3)") && src.contains("_step = next"),
        "expected the driving loop in:\n{src}"
    );
    assert!(
        src.contains("!is U2_1<Int, Finished>"),
        "expected the arm spelled with real type arguments in:\n{src}"
    );
    assert!(
        !src.contains(" as Int"),
        "a concrete arm needs no cast; got:\n{src}"
    );
}

/// [kt-struct-empty] A fieldless struct is a plain class, not a data class:
/// Kotlin requires a data class to have at least one primary-constructor
/// parameter. `Finished` is the first such struct (std's iterator protocol).
#[test]
fn a_fieldless_struct_is_a_plain_class() {
    let program = build_program(&[("main.sv", PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/iterator.kt"))
        .expect("core/iterator.kt")
        .content;
    assert!(
        src.contains("class Finished") && !src.contains("data class Finished"),
        "expected a plain class for the fieldless struct in:\n{src}"
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
const FALLIBLE_PASS_DEMO: &str = r#"
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
        // The element is a borrow [copy-opt-in]; wrapping it in a result
        // stores it, so the copy is explicit.
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

const FALLIBLE_PASS_OUTPUT: &str =
    "line alpha\nline beta\nread 2\nline alpha\nfailed: stopped: bad line at 2\n";

fn a_fallible_pass_yields_a_result() -> KotlinCase {
    let program = build_program(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "fallible-pass", FALLIBLE_PASS_OUTPUT)
}

/// [qual-group] The nested wrap, read off the generated source rather than
/// only from the program's output: `emitted(err(...))` lands in the group arm
/// `Emitted (Ok Str | Err Str)`, whose inner union is a wrapper of its own, so
/// the value takes *two* wraps — inner arm first, and the arm index comes
/// from the qualifier left over after the group's own is removed.
#[test]
fn a_flattened_qualifier_wraps_the_inner_union_first() {
    let program = build_program(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt");
    for (arm, what) in [("U2_2", "err"), ("U2_1", "ok")] {
        let needle = format!("U2_1<Union2<String, String>, Finished>({arm}<String, String>(");
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
/// type-checked and emitted one wrap, which both target compilers rejected:
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
        ^ Emitted {
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

/// The Kotlin half of a parity check on mutation through a **narrowed** place
/// [flow-place] (the Rust backend's copy of this program carries the reasoning,
/// and its spec the rule). Kotlin never had the bug — a smart cast *is* the
/// storage and objects are references — so this is here to pin the output the
/// two backends must agree on, which is what made the divergence visible in the
/// first place.
const NARROW_MUT_DEMO: &str = r#"
struct Flat {
    rows: List<List<Int>>
}

fn show(step: Emitted Int | Finished) [Console] {
    when step {
        ^ Emitted { println("got ${step}") }
        is Finished { println("end") }
    }
}

iter fn next(f: Flat) -> Emitted Int | Finished {
    state { at: Int = 0, inner: Mut ListYield<Int>? = None }
    while true {
        if inner is Mut ListYield<Int> {
            let step = next(inner)
            when step {
                ^ Emitted { return emitted(step) }
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
    let all = Flat { rows: list_of(list_of(1, 2), list_of(3, 4, 5)) }
    for n in iter(all) {
        println("n ${n}")
    }
}
"#;

const NARROW_MUT_OUTPUT: &str = "got 1\ngot 2\nend\ngot 7\ngot 8\ngot 3\nn 1\nn 2\nn 3\nn 4\nn 5\n";

fn kotlinc_compiles_and_runs_mutation_through_narrowed_places() -> KotlinCase {
    let program = build_program(&[("main.sv", NARROW_MUT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "narrow-mut", NARROW_MUT_OUTPUT)
}

fn kotlinc_compiles_and_runs_a_group_over_plain_arms() -> KotlinCase {
    let program = build_program(&[("main.sv", PLAIN_GROUP_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "plain-group", PLAIN_GROUP_OUTPUT)
}

fn kotlinc_compiles_and_runs_a_hand_written_pass() -> KotlinCase {
    let program = build_program(&[("main.sv", PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "hand-written-pass", PASS_OUTPUT)
}

/// A **composed pass, hand-written**: it stores its source *and* its callback.
/// Kotlin has always accepted a fn-typed field; the Rust backend refused one
/// until R5 lifted it ([rs-fn-field]: an `Rc<dyn Fn…>` field). Same source and
/// same stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_composed_pass_with_a_stored_callback` — the
/// parity claim, and the reason a user-written combinator is now possible on
/// both targets rather than only in generated std code.
const FN_FIELD_DEMO: &str = r#"
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

const FN_FIELD_OUTPUT: &str = "n 6\nn 4\nn 2\ndone\n";

/// [implicit-group] [iter-protocol] The composition rendering R5's
/// combinators are built on: a generic combinator takes the *pass* as its
/// subject and reaches its `next` through the `?Yield<It, T>` spread. The
/// Kotlin half of the pair of bugs this exposed: an implicit member
/// returning a **union** rendered its Salvo type text into the Kotlin source
/// (`(It) -> Emitted T | Finished`), which is invalid Kotlin
/// [backend-never-wrong]. Same source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_combinator_over_a_yield_spread`.

/// [iter-fn] The **release** of a minted pass, Kotlin half: the same
/// source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_an_abandoned_mint` — a combinator abandoning the
/// pass after one element still runs the origin's `defer`.

/// [linear-group] The Kotlin half: a raw pass declaring `: Linear<self>` is
/// released by the loop, in the `finally` — same source and stdout as the Rust
/// [iter-generic-drive] [iter-drive-in-place] The three things the generic drive
/// buys, in one program: a combinator driving its pass with an ordinary `for`,
/// the caller **carrying that pass on** afterwards (the parity case — Rust used
/// to bind the `&mut` parameter into a local, so the caller never saw the
/// position the loop reached), and a `yield fn` performing a **generic** effect.
///
/// Shared with the Rust backend, byte for byte.
const GENERIC_DRIVE_DEMO: &str = r#"
struct Slice<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj[from: p] T) | Finished => p: Mut {
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

const GENERIC_DRIVE_OUTPUT: &str = "first 3\nrest 7\nv 8\nv 9\nv 7\n";

/// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
/// linear pass releases it through the `?Linear<It>` spread. The release cannot
/// print — an implicitly resolved fn is effect-free [implicit-resolve] — so the
/// call is asserted in the emitted code and the program is run to prove it
/// builds.
const GENERIC_CLOSE_DEMO: &str = r#"
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

const GENERIC_CLOSE_OUTPUT: &str = "all 10\ncut 7\n";

/// backend's `rustc_compiles_and_runs_a_released_raw_pass`.
const RAW_CLOSE_DEMO: &str = r#"
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

const RAW_CLOSE_OUTPUT: &str = "n 2\nn 1\nclosed\nafter drain\nm 5\nclosed\nafter break\n";

#[test]
fn a_raw_pass_is_driven_in_place_with_no_finally() {
    // [iter-drive-in-place] [linear-group] No implicit discharge sites
    // (user decision 2026-09-12): no finally splice — the program's own
    // `close(lines)` is the discharge, an ordinary call.
    let program = build_program(&[("main.sv", RAW_CLOSE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        !src.contains("} finally {") && !src.contains("__loop1_pass"),
        "expected no finally splice and no loop local, got:\n{src}"
    );
    assert!(
        // Suffixed: std declares a `close` too [fs-surface].
        src.contains("close__3(console, lines)"),
        "expected the program's own explicit close:\n{src}"
    );
}

fn kotlinc_compiles_and_runs_a_released_raw_pass() -> KotlinCase {
    let program = build_program(&[("main.sv", RAW_CLOSE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "raw-close", RAW_CLOSE_OUTPUT)
}

const YIELD_SPREAD_DEMO: &str = r#"
struct Slice<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj[from: p] T) | Finished => p: Mut {
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

const YIELD_SPREAD_OUTPUT: &str = "d 2\nd 4\nd 6\n";

/// The spread position renders as a Kotlin function type whose result is the
/// union *wrapper*, not the Salvo text.
#[test]
fn a_union_returning_implicit_renders_the_wrapper_type() {
    let program = build_program(&[("main.sv", YIELD_SPREAD_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains("next: (It) -> Union2<T, Finished>"),
        "expected the union wrapper in the spread position, got:\n{src}"
    );
    assert!(
        !src.contains("Emitted T | Finished"),
        "the Salvo type text must not reach the Kotlin source:\n{src}"
    );
}

fn kotlinc_compiles_and_runs_a_combinator_over_a_yield_spread() -> KotlinCase {
    let program = build_program(&[("main.sv", YIELD_SPREAD_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "yield-spread", YIELD_SPREAD_OUTPUT)
}

fn kotlinc_compiles_and_runs_a_composed_pass_with_a_stored_callback() -> KotlinCase {
    let program = build_program(&[("main.sv", FN_FIELD_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "fn-field-pass", FN_FIELD_OUTPUT)
}

// [qual-no-dup]
#[test]
fn duplicate_qualifier_is_rejected() {
    let errors = expect_errors("qualifier Ok<T> of T\n\nfn f(x: Ok Ok Int) -> None {\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("applied more than once")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-with]
#[test]
fn incompatible_qualifiers_are_rejected() {
    let src = r#"
export struct Person {
    name: Str
}

export qualifier Old of Person
export qualifier Surname of Person

export fn f(p: Old Surname Person) -> None {
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("are not compatible")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-of]
#[test]
fn qualifier_of_type_is_enforced() {
    let src = r#"
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

export fn f(x: Positive Str) -> None {
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("does not apply to `Str`")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-ctor-same-file]
#[test]
fn constructor_must_live_with_its_qualifier() {
    let program = build_program(&[
        ("quals.sv", "export qualifier Fancy of Int\n"),
        (
            "other.sv",
            "import quals.Fancy\n\nfn make() -> Int as Fancy {\n    return 1\n}\n",
        ),
    ]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("must be declared in the same file")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-ctor-predicate] Predicate qualifiers may have constructor
// functions (same-file rule still applies): the constructor asserts the
// predicate by construction, qualifiers erase, and overloads on the
// qualified type resolve statically.
#[test]
fn predicate_qualifiers_may_have_constructors() {
    let src = r#"
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

export fn make() -> Int as Positive {
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    // The constructor is a plain fn after erasure; the call site resolves
    // to the Positive overload (mangled name).
    assert!(
        main.content.contains("fun make(): Int"),
        "content: {}",
        main.content
    );
    assert!(
        main.content.contains("describe__Positive(make())"),
        "content: {}",
        main.content
    );
}

// [qual-ctor-simple]
#[test]
fn constructor_return_type_must_be_simple() {
    let src = "export qualifier Fancy of Int\n\nexport fn make() -> (Int | Str) as Fancy {\n    return 1\n}\n";
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("must return a simple type")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-constructive]
#[test]
fn constructive_qualifier_cannot_be_is_tested() {
    let src = "export qualifier Fancy of Int\n\nexport fn f(x: Int) -> None {\n    if x is Fancy {\n    }\n}\n";
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("constructive qualifier")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-predicate]
#[test]
fn qualifies_signature_is_validated() {
    let src = r#"
export qualifier Weird of Int {
    fn qualifies(int: Int) -> Str {
        return "nope"
    }
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`qualifies` must return `Bool`")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-constructive] [qual-ctor-fn]
#[test]
fn constructive_values_only_come_from_constructors() {
    // Plain values never satisfy a constructive qualifier: the assignment
    // is a type error, so the only way in is the constructor fn.
    let src = "export qualifier Fancy of Int\n\nexport fn f() -> None {\n    let x: Fancy Int = 1\n}\n";
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("expected `Fancy Int`, found `Int`")),
        "unexpected errors: {errors:?}"
    );
}

// [type-canbe-mut] `Mut List<T>` maps onto `MutableList<T>`;
// `Mut` on a type without `canbe Mut` is an error.
#[test]
fn mut_types_map_onto_mutable_list() {
    let src = r#"
export fn fill(target: Mut List<Int>, n: Int) -> None => target: Mut {
    add(target, n)
}

export fn main() [use] -> None {
    use StdOutConsole
    let items: Mut List<Int> = mut_list_of(1)
    fill(items, 2)
    println("size: ${items.size()}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(
        main.content
            .contains("fun fill(target: MutableList<Int>, n: Int)"),
        "unexpected: {}",
        main.content
    );
    assert!(main
        .content
        .contains("val items: MutableList<Int> = mutableListOf<Int>(1)"));
}

// [type-canbe-mut] `Mut` only applies to declarations that say `canbe Mut`.
// (`Str` does say so [kt-mut-str]; `Int` is the primitive that never can.)
#[test]
fn mut_requires_a_with_mut_declaration() {
    let errors = expect_errors("fn f(x: Mut Int) -> None {\n}\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`Mut` does not apply to `Int`")),
        "unexpected errors: {errors:?}"
    );
}

/// Full verification: compile the generated Kotlin with kotlinc and run it,
/// checking the program output. Skipped when kotlinc is not installed.
fn kotlinc_compiles_and_runs_demo() -> KotlinCase {
    let files = generate_demo();
    let expected = "Hello, Roland Elliott!\n  1: 37\n  2: 38\n  3: 39\n\
                    Hello, Roland!\n  1: 37\n  2: 38\n  3: 39\n\
                    first: a\nsize: 2\n";
    kotlin_case(files, "demo", expected)
}

/// The M5 demo: generic effects with multiple instances in scope,
/// checker-driven disambiguation (explicit type args, expected type),
/// handler generic inference at `use` sites, and handler threading through
/// fn call sites — all resolved from the checker's effect tables.
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

fn generate_effects_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", EFFECTS_DEMO)]);
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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

// [effect-use] [effect-disambiguation] [kt-effect-params]
#[test]
fn effects_resolve_through_checker_tables() {
    let files = generate_effects_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // `use` registers the *concrete* effect instance: the handler's generics
    // come from the written type arguments or from the constructor arguments,
    // and are written out at the constructor either way
    // [effect-handler-generics] — the emitter does not reason about what
    // Kotlin could have inferred, and a stateless generic handler leaves it
    // nothing to infer from.
    assert!(main
        .content
        .contains("val random_int: Random<Int> = CyclicRandom<Int>(listOf<Int>(10, 20, 30), { __i0 -> __i0 })"));
    assert!(main
        .content
        .contains("val random_string: Random<String> = CyclicRandom<String>(listOf<String>(\"a\", \"b\"), { __i0 -> __i0 })"));
    // Callee effect dependencies are threaded in declaration order.
    assert!(main
        .content
        .contains("draw(random_int, random_string, console)"));
    assert!(main.content.contains("lucky_number(random_int)"));
    // Expected-type disambiguation picks the right handler per call.
    assert!(main
        .content
        .contains("val n: Int = random_int.next_random()"));
    assert!(main
        .content
        .contains("val s: String = random_string.next_random()"));
}

/// Full verification of the effects demo under kotlinc (skipped when
/// kotlinc is not installed).
fn kotlinc_compiles_and_runs_effects() -> KotlinCase {
    let files = generate_effects_demo();
    let expected = "a: 10\nb: 20\nlucky: 30\nagain: 10\n";
    kotlin_case(files, "effects", expected)
}

// [use-requires-use]
#[test]
fn use_requires_the_use_effect() {
    let errors = expect_errors("fn setup() -> None {\n    use StdOutConsole\n}\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("requires the `use` effect")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-no-dup]
#[test]
fn duplicate_effect_in_list_is_rejected() {
    let errors = expect_errors("fn f() [Console, Console] -> None {\n}\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("duplicate effect `Console`")),
        "unexpected errors: {errors:?}"
    );
}

// [use-no-dup] [effect-intercept]
#[test]
fn a_use_may_shadow_an_earlier_registration() {
    // Until 2026-09-14 this was "a handler for `Console` is already
    // registered in this scope"; shadowing is now how interception is
    // written, so a second registration is legal and the innermost wins.
    let program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdOutConsole\n    use StdOutConsole\n}\n",
    )]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert_eq!(
        main.content.matches("StdOutConsole()").count(),
        2,
        "both registrations should be emitted:\n{}",
        main.content
    );
}

// [effect-fn-deps]
#[test]
fn unknown_effect_is_rejected() {
    let errors = expect_errors("fn f() [Consle] -> None {\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("unknown effect `Consle`")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-disambiguation]
#[test]
fn ambiguous_generic_effect_call_is_rejected() {
    let src = r#"
export effect Random<T> {
    fn next_random() -> T
}

export fn f() [Random<Int>, Random<Double>] -> None {
    let x = next_random()
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("ambiguous effect call")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-disambiguation] [use-no-dup] Shadowing collapses repeats of *one*
// instance, and must not collapse two *different* instances of a generic
// effect: those are still ambiguous, registered by `use` as much as by an
// effect list.
#[test]
fn two_registered_instances_are_still_ambiguous() {
    let src = r#"
export effect Random<T> {
    fn next_random() -> T
}

export handler Fixed<T>(value: T) of Random<T> {
    fn next_random() -> T {
        return value
    }
}

export fn main() [use] -> None {
    use Fixed(1)
    use Fixed("a")
    let x = next_random()
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("ambiguous effect call")),
        "unexpected errors: {errors:?}"
    );
}

// [kt-effect-fusion] A generic effect instance cannot be a fused class's
// property: the class has no type parameters, so this is reported rather
// than emitted as an undeclared name (Rust states the same cut). Until
// 2026-09-14 it leaked out as a kotlinc "unresolved reference 'T'".
#[test]
fn a_generic_dependency_is_refused() {
    let errors = expect_errors(
        "effect Store<T> {\n    fn keep(value: T) -> Str => value\n}\n\n\
         handler MemStore<T> of Store<T> {\n    \
         fn keep(value: T) -> Str => value {\n        return \"kept\"\n    }\n}\n\n\
         handler Twice<T> [Store<T>] of Store<T> {\n    \
         fn keep(value: T) -> Str => value {\n        return \"twice\"\n    }\n}\n\n\
         fn main() [use] -> None {\n    use local MemStore<Int>()\n    \
         use local Twice<Int>()\n}\n",
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("an effect set fused here is still generic")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-available]
#[test]
fn effect_member_call_requires_handler_in_scope() {
    let errors = expect_errors("fn main() [use] -> None {\n    print(\"no handler\")\n}\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("no handler for effect `Console`")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-member-no-effects]
#[test]
fn handler_member_effects_are_rejected() {
    let src = r#"
export effect Ping {
    fn ping()
}

export handler LoudPing of Ping {
    fn ping() [Console] {
        println("ping")
    }
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("handler member functions cannot declare effect dependencies")),
        "unexpected errors: {errors:?}"
    );
}

/// Compiles the given files with kotlinc, runs `salvo.MainKt`, and asserts
/// the exact stdout.
/// One compile-and-run verification: a generated program, the JVM entry
/// class to launch, and the exact stdout it must print.
///
/// Cases are built by plain functions listed in [`KOTLIN_CASES`], and one
/// driver test — [`kotlinc_compiles_and_runs_every_case`] — does all the
/// toolchain work: it compiles every outstanding case in a handful of
/// *batched* `kotlinc` invocations (each invocation costs ~2.5s of JVM and
/// compiler startup regardless of input size, so one invocation per test
/// made the fresh suite minutes long) and then runs the programs in
/// parallel. To share one compilation, each case's generated code is
/// rewritten into its own package namespace (`k_<tag>.salvo…`), since every
/// program declares the same `salvo.main.MainKt`.
struct KotlinCase {
    /// Owned rather than `&'static str` so a case can be built from data —
    /// the examples, one case each.
    tag: String,
    /// The JVM class to launch, before package prefixing.
    entry: &'static str,
    files: Vec<salvo_backend_kotlin::EmittedFile>,
    expected: String,
}

/// A case launching the ordinary entry point, `salvo.main.MainKt`.
fn kotlin_case(
    files: Vec<salvo_backend_kotlin::EmittedFile>,
    tag: &str,
    expected: &str,
) -> KotlinCase {
    kotlin_case_with_entry(files, tag, "salvo.main.MainKt", expected)
}

/// A case launching a named entry class — the host's, when a platform
/// effect has moved `main` out of the generated code.
fn kotlin_case_with_entry(
    files: Vec<salvo_backend_kotlin::EmittedFile>,
    tag: &str,
    entry: &'static str,
    expected: &str,
) -> KotlinCase {
    KotlinCase {
        tag: tag.to_string(),
        entry,
        files,
        expected: expected.to_string(),
    }
}

/// Every compile-and-run case. A new case is a function returning
/// [`KotlinCase`] plus an entry here; the driver test below asserts nothing
/// is forgotten by being the only place kotlinc runs.
// ===== C-6 the claims a list can carry [col-nonempty] [col-sorted-list]
// [col-distinct] =====

/// Source and expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_list_claims`. That equality is the assertion: the
/// whole surface is std's own, and both `sort`'s ordering (Kotlin's natural
/// `String` order is UTF-16 code units, Rust's is code points) and
/// `binary_search`'s answer inside an equal run have to come out the
/// language's way rather than the target's.
fn kotlinc_compiles_and_runs_list_claims() -> KotlinCase {
    let src = r#"
// A position that demands distinctness needs no duplicate check of its own.
export fn count_unique(xs: Distinct List<Int>) [] -> Int => xs {
    return size(xs)
}

export fn main() [use] {
    use StdOutConsole()

    // NonEmpty by construction: `first` answers with an element, not `Int?`.
    let built = non_empty_list(10, 20)
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "list-claims", "built 10\n\
     grown 7\n\
     tested 1\n\
     sorted [1, 1, 3, 4, 5]\n\
     lowest 1 at 0\n\
     9 absent\n\
     still sorted [5, 10, 20, 40]\n\
     words [Apple, fig, pear]\n\
     distinct [3, 1, 2] of 3\n\
     set claim 1\n\
     map claim 1\n\
     sorted claim 10..30\n\
     sorted map claim a\n")
}

// ===== [fn-variadic] mixing plain arguments with a `...spread` =====

/// Source and expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_mixed_spread`. Kotlin has a native spread
/// operator (`listOf(first, *rest)`), so mixed calls always worked here —
/// which is exactly why this case is worth keeping on both sides: it is the
/// backend that already agreed, and the assertion is that the Rust side now
/// produces the same answers rather than dropping the leading elements.
fn kotlinc_compiles_and_runs_mixed_spread() -> KotlinCase {
    let src = r#"
export fn join_all(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let out = mut_str(...parts)
    println("${sep} ${out}")
}

export fn main() [use] {
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

    // A lone spread still forwards the whole collection.
    let lone = list_of(...rest)
    println("lone ${to_str(lone)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "mixed-spread", "list_of [1, 2, 3]\n\
     reusable 2\n\
     set_of {9, 2, 3}\n\
     sorted_set_of {2, 3, 9}\n\
     map_of {a: 1, b: 2}\n\
     mut_str abc\n\
     ordinary xbc\n\
     lone [2, 3]\n")
}

// ===== [kt-variadic] a user-declared variadic, every element type =====

/// Source and expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_user_variadics`. This is the case the
/// representation change exists for: with `vararg ns: Int` the parameter was an
/// `IntArray`, which nothing generic accepts, so `iter(ns)` did not compile and
/// an `Array<Int>` could not be spread in. A variadic parameter is now an
/// ordinary `Array<T>` on this backend, and the call site builds it.
fn kotlinc_compiles_and_runs_user_variadics() -> KotlinCase {
    let src = r#"
export fn count_ints(label: Str, ...ns: Int[]) [Console] -> Int => label, ns {
    let seen = mut_list_of(0)
    for n in iter(ns) {
        add(seen, copy(n))
    }
    println("${label} ${size(ns)}")
    return size(seen)
}

export fn join_strs(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let joined = mut_str(...parts)
    println("${sep} ${joined}")
}

export fn main() [use] {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "user-variadics", "plain 3\n\
     spread 2\n\
     mixed 3\n\
     empty 0\n\
     counts 4 3 4 1\n\
     reusable 2\n\
     refs ab\n\
     refs xyz\n")
}

const KOTLIN_CASES: &[fn() -> KotlinCase] = &[
    kotlinc_compiles_and_runs_an_actor,
    kotlinc_compiles_and_runs_a_monitor,
    kotlinc_compiles_and_runs_a_mixed_handler,
    kotlinc_compiles_and_runs_a_deferring_mixed_handler,
    kotlinc_compiles_and_runs_a_parking_mixed_handler,
    kotlinc_compiles_and_runs_a_servant_sibling_chain,
    kotlinc_compiles_and_runs_a_stub_bound_effect,
    kotlinc_compiles_and_runs_a_dependent_spawn,
    kotlinc_compiles_and_runs_a_member_named_like_a_std_fn,
    kotlinc_compiles_and_runs_a_parked_continuation,
    kotlinc_compiles_and_runs_a_gated_continuation,
    kotlinc_compiles_and_runs_both_readings_of_a_self_send,
    kotlinc_compiles_and_runs_a_death_watch,
    kotlinc_compiles_and_runs_a_quiescence_hook,
    kotlinc_compiles_and_runs_a_handler_of_several_effects,
    kotlinc_compiles_and_runs_the_waitfor_package,
    kotlinc_compiles_and_runs_the_task_kernel,
    kotlinc_compiles_and_runs_obligations_in_a_collection,
    kotlinc_compiles_and_runs_the_time_surface,
    kotlinc_compiles_and_runs_a_real_timer,
    kotlinc_compiles_and_runs_manual_time,
    kotlinc_compiles_and_runs_the_coupling_postures,
    kotlinc_compiles_and_runs_is_bindings_over_calls,
    // the checked-in examples, one case each
    kotlin_example_actors,
    kotlin_example_collections,
    kotlin_example_effects,
    kotlin_example_files,
    kotlin_example_iteration,
    kotlin_example_linearity,
    kotlin_example_qualifiers,
    kotlin_example_throw_and_release,
    kotlin_example_time,
    kotlinc_compiles_and_runs_unions,
    kotlinc_compiles_and_runs_qualifiers,
    a_fallible_pass_yields_a_result,
    kotlinc_compiles_and_runs_mutation_through_narrowed_places,
    kotlinc_compiles_and_runs_a_group_over_plain_arms,
    kotlinc_compiles_and_runs_a_hand_written_pass,
    kotlinc_compiles_and_runs_a_released_raw_pass,
    kotlinc_compiles_and_runs_a_combinator_over_a_yield_spread,
    kotlinc_compiles_and_runs_a_composed_pass_with_a_stored_callback,
    kotlinc_compiles_and_runs_demo,
    kotlinc_compiles_and_runs_effects,
    kotlinc_compiles_and_runs_loops,
    kotlinc_compiles_and_runs_mangled_alias,
    kotlinc_compiles_and_runs_multi_module,
    kotlinc_compiles_and_runs_copy,
    kotlinc_compiles_and_runs_move_modes,
    kotlinc_compiles_and_runs_borrows,
    kotlinc_compiles_and_runs_linear,
    kotlinc_compiles_and_runs_linear_generics,
    kotlinc_compiles_and_runs_once_fns,
    kotlinc_compiles_and_runs_derived_returns,
    kotlinc_compiles_and_runs_fn_contracts,
    kotlinc_compiles_and_runs_precedence,
    kotlinc_compiles_and_runs_field_is,
    kotlinc_compiles_and_runs_place_narrowing,
    kotlinc_compiles_and_runs_place_operand,
    kotlinc_compiles_and_runs_tuple_index,
    kotlinc_compiles_and_runs_tuples_past_three,
    kotlinc_compiles_and_runs_loop_destructuring,
    kotlinc_compiles_and_runs_list_element_types,
    kotlinc_compiles_and_runs_collections,
    kotlinc_compiles_and_runs_collection_iteration,
    kotlinc_compiles_and_runs_collection_literals,
    kotlinc_compiles_and_runs_equality_and_ordering,
    kotlinc_compiles_and_runs_operators,
    kotlinc_compiles_and_runs_effect_selectors,
    kotlinc_compiles_and_runs_sorted_collections,
    kotlinc_compiles_and_runs_codepoint_string_order,
    kotlinc_compiles_and_runs_generated_constructors,
    kotlinc_compiles_and_runs_list_claims,
    kotlinc_compiles_and_runs_mixed_spread,
    kotlinc_compiles_and_runs_user_variadics,
    kotlinc_compiles_and_runs_handler_dependencies,
    kotlinc_compiles_and_runs_interception,
    kotlinc_compiles_and_runs_a_shareable_interceptor_chain,
    kotlinc_compiles_and_runs_spawn_inheritance,
    kotlinc_compiles_and_runs_a_generic_handler_with_a_handle_dep,
    kotlinc_compiles_and_runs_handler_deps_in_anger,
    kotlinc_compiles_and_runs_handler_deps_chain,
    kotlinc_compiles_and_runs_handler_deps_mixed,
    kotlinc_compiles_and_runs_nested_coercion,
    kotlinc_compiles_and_runs_member_generics,
    kotlinc_compiles_and_runs_aliased_effects,
    kotlinc_compiles_and_runs_array_std,
    dot_names_emit_nested_classes,
    dot_names_in_unions_and_narrowing,
    provenance_survives_mutation_where_state_does_not,
    kotlin_compiles_and_runs_throw,
    kotlinc_compiles_and_runs_fn_type_effects,
    kotlinc_compiles_and_runs_widening,
    kotlinc_compiles_and_runs_when_cond,
    kotlinc_compiles_and_runs_try_mutation,
    kotlinc_compiles_and_runs_a_platform_effect,
    kotlinc_compiles_and_runs_a_platform_handler,
    kotlinc_compiles_and_runs_member_overloads,
    kotlinc_compiles_and_runs_member_modes,
    kotlinc_compiles_and_runs_a_linear_token_closed_by_a_member,
    kotlinc_compiles_and_runs_overload_delegation,
    kotlinc_compiles_and_runs_the_combinator_surface,
    kotlinc_compiles_and_runs_a_break_out_of_an_unbounded_producer,
    kotlinc_compiles_and_runs_implicit_parameters,
    kotlinc_compiles_and_runs_effect_member_implicits,
    kotlinc_compiles_and_runs_a_generic_handler,
    kotlinc_compiles_and_runs_a_refined_program,
    kotlinc_runs_the_most_specific_overload,
    kotlinc_compiles_and_runs_strings,
    kotlinc_compiles_and_runs_bytes,
    a_spread_into_a_variadic_intrinsic_spreads,
    kotlinc_compiles_and_runs_mut_str_places,
    kotlinc_compiles_and_runs_sequences,
    kotlinc_compiles_and_runs_overload_overrides,
    kotlinc_compiles_and_runs_a_generic_drive,
    kotlinc_compiles_and_runs_a_generic_close,
    kotlinc_compiles_and_runs_the_iter_fn_form,
    kotlinc_compiles_and_runs_a_container_combinator,
    kotlinc_compiles_and_runs_field_disjoint_access,
    kotlinc_compiles_and_runs_a_partial_move,
    kotlinc_compiles_and_runs_inc_dec,
    kotlinc_compiles_and_runs_a_generic_subject_iter_fn,
    kotlinc_compiles_and_runs_the_deduction_clause_projections,
    kotlinc_compiles_and_runs_a_borrowed_union_arm_into_a_proj_parameter,
    kotlinc_compiles_and_runs_a_capture_rooted_projection,
    kotlinc_compiles_and_runs_a_linear_union_arm,
    kotlinc_compiles_and_runs_a_linear_wrapper_pass,
    kotlinc_compiles_and_runs_drop_as_a_consuming_callback,
    kotlinc_compiles_and_runs_the_fs_surface,
    kotlinc_compiles_and_runs_the_memory_filesystem,
];

/// The package prefix isolating one case inside the shared compilation.
fn pkg_prefix(tag: &str) -> String {
    format!("k_{}", tag.replace('-', "_"))
}

/// Rewrites one generated file into the case's own package namespace.
/// Blind textual rewrite: `package salvo…` declarations and every `salvo.`
/// reference (imports and qualified names alike). A string literal
/// containing `salvo.` would be corrupted — which is why the driver
/// refuses expected output mentioning it, so corruption cannot pass.
fn prefix_content(content: &str, pfx: &str) -> String {
    content
        .split('\n')
        .map(|line| {
            if let Some(rest) = line.strip_prefix("package salvo") {
                format!("package {pfx}.salvo{rest}")
            } else {
                line.replace("salvo.", &format!("{pfx}.salvo."))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The one test that exercises kotlinc. Batching per fresh run:
/// stamp-missing cases are split over a few parallel `kotlinc` processes,
/// then every compiled program runs in parallel with its stdout asserted.
/// A stamp is written per case, only after its assertions pass — the cache
/// behaves exactly as it did when every case was its own test.
#[test]
fn kotlinc_compiles_and_runs_every_case() {
    // Build every case first: the content assertions inside case builders
    // run even when kotlinc is missing, as they did when each was a #[test].
    let cases: Vec<KotlinCase> = KOTLIN_CASES.iter().map(|make| make()).collect();
    let mut seen = std::collections::HashSet::new();
    for case in &cases {
        // Tags name packages, scratch paths and stamps.
        assert!(seen.insert(case.tag.clone()), "duplicate case tag {}", case.tag);
        assert!(
            !case.expected.contains("salvo."),
            "case {}: expected output mentions `salvo.`, which the package \
             prefix rewrite would corrupt — pick different program output",
            case.tag
        );
    }
    let kotlinc = salvo_testkit::kotlinc(env!("CARGO_TARGET_TMPDIR"));
    if !kotlinc.available {
        return;
    }
    // Keep only the cases whose stamps miss; identical keys to the old
    // per-test stamps, so existing caches stay valid.
    let mut pending: Vec<(KotlinCase, salvo_testkit::Stamp)> = Vec::new();
    for case in cases {
        let (kind, label) = if case.entry == "salvo.main.MainKt" {
            ("kotlin-files", case.tag.as_str())
        } else {
            ("kotlin-entry", case.entry)
        };
        if let Some(stamp) = cache_stamp(&kotlinc.version, kind, label, &case.files, &case.expected)
        {
            pending.push((case, stamp));
        }
    }
    if pending.is_empty() {
        return;
    }

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // Chunk the pending cases over a few kotlinc processes: each process
    // pays the same ~2.5s startup, and each is internally multi-threaded,
    // so more processes than half the cores just contend.
    let chunk_count = pending.len().min((workers / 2).max(1));
    let per_chunk = pending.len().div_ceil(chunk_count);
    let chunks: Vec<&[(KotlinCase, salvo_testkit::Stamp)]> = pending.chunks(per_chunk).collect();

    // Write each case's sources, prefixed, under its chunk's directory.
    let mut chunk_dirs = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let dir = salvo_testkit::scratch(env!("CARGO_TARGET_TMPDIR"), &format!("kt-batch-{i}"));
        let mut kt_paths = Vec::new();
        for (case, _) in chunk.iter() {
            let pfx = pkg_prefix(&case.tag);
            for f in &case.files {
                let path = dir.join("src").join(&case.tag).join(&f.rel_path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, prefix_content(&f.content, &pfx)).unwrap();
                kt_paths.push(path);
            }
        }
        chunk_dirs.push((dir, kt_paths));
    }

    // Compile the chunks in parallel, one kotlinc each.
    let compile_errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (dir, kt_paths) in &chunk_dirs {
            scope.spawn(|| {
                let out = Command::new("kotlinc")
                    .args(kt_paths.iter().map(|p| p.as_os_str()))
                    .arg("-d")
                    .arg(dir.join("out"))
                    .output()
                    .expect("failed to run kotlinc");
                if !out.status.success() {
                    // The stderr names the offending files, whose paths
                    // carry the case tags.
                    compile_errors.lock().unwrap().push(format!(
                        "kotlinc failed for {}:\n{}",
                        dir.display(),
                        String::from_utf8_lossy(&out.stderr)
                    ));
                }
            });
        }
    });
    let compile_errors = compile_errors.into_inner().unwrap();
    assert!(compile_errors.is_empty(), "{}", compile_errors.join("\n\n"));

    // Run every pending program in parallel, asserting exact stdout.
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let passed: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    let jobs: Vec<(usize, &KotlinCase, &std::path::Path)> = chunks
        .iter()
        .zip(&chunk_dirs)
        .flat_map(|(chunk, (dir, _))| chunk.iter().map(move |(case, _)| (case, dir.as_path())))
        .enumerate()
        .map(|(i, (case, dir))| (i, case, dir))
        .collect();
    let next_job = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers.min(jobs.len()) {
            scope.spawn(|| loop {
                let i = next_job.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some((index, case, dir)) = jobs.get(i).copied() else {
                    return;
                };
                let entry = format!("{}.{}", pkg_prefix(&case.tag), case.entry);
                let run = Command::new("kotlin")
                    .arg("-cp")
                    .arg(dir.join("out"))
                    .arg(&entry)
                    .output()
                    .expect("failed to run kotlin");
                if !run.status.success() {
                    failures.lock().unwrap().push(format!(
                        "case {}: generated program crashed:\n{}",
                        case.tag,
                        String::from_utf8_lossy(&run.stderr)
                    ));
                    return;
                }
                let stdout = String::from_utf8_lossy(&run.stdout);
                if stdout != case.expected {
                    failures.lock().unwrap().push(format!(
                        "case {}: unexpected program output:\n--- expected:\n{}\n--- got:\n{}",
                        case.tag, case.expected, stdout
                    ));
                } else {
                    passed.lock().unwrap().push(index);
                }
            });
        }
    });

    // Stamps only for the cases whose assertions passed; scratch is kept
    // on failure for inspection.
    let passed = passed.into_inner().unwrap();
    let failures = failures.into_inner().unwrap();
    let mut stamps: Vec<Option<salvo_testkit::Stamp>> =
        pending.into_iter().map(|(_, stamp)| Some(stamp)).collect();
    for index in passed {
        if let Some(stamp) = stamps[index].take() {
            stamp.verified();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    for (dir, _) in chunk_dirs {
        let _ = std::fs::remove_dir_all(&dir);
    }
}
/// The cache key for one compile-and-run: the toolchain that would do it,
/// every generated file (path *and* content), and the output asserted.
/// Nothing else can change the outcome, and a change to any of them must
/// invalidate the stamp — which is what makes the cache safe to have on by
/// default.
fn cache_stamp(
    version: &str,
    kind: &str,
    entry: &str,
    files: &[salvo_backend_kotlin::EmittedFile],
    expected: &str,
) -> Option<salvo_testkit::Stamp> {
    let mut parts: Vec<Vec<u8>> = vec![
        kind.as_bytes().to_vec(),
        version.as_bytes().to_vec(),
        entry.as_bytes().to_vec(),
        expected.as_bytes().to_vec(),
    ];
    for f in files {
        parts.push(f.rel_path.to_string_lossy().as_bytes().to_vec());
        parts.push(f.content.as_bytes().to_vec());
    }
    let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
    salvo_testkit::cached(
        env!("CARGO_TARGET_TMPDIR"),
        &format!("{kind} {entry}"),
        &refs,
    )
}

// ===== M6: loops as values =====

/// The M6 demo: loop values from body tails, `break value`, loop `else`
/// in value and statement position, bare `break` (optional value), and a
/// union-typed loop value re-wrapped to the declared type [while-value].
const LOOPS: &str = r#"
struct Range {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> Range {
    return Range {start: start, end: end}
}

iter fn next(r: Range) -> Emitted Int | Finished {
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
    let found = for x in [0, 1, 2, 3, 4, 5] {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    // Statement-position loop with `else`, over a producer.
    for x in range(0, 0) {
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
    println("capped: ${capped!}")

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
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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

// [while-value] [kt-loop-value]
#[test]
fn loops_lower_to_run_blocks() {
    let files = generate_loops_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // Value loops become `run {}` blocks with a nullable result local,
    // unwrapped when the join type has no `None` arm.
    assert!(main.content.contains("val last = run {"));
    assert!(main.content.contains("var __loop1: Int? = null"));
    assert!(main.content.contains("__loop1 = i * 10"));
    assert!(main.content.contains("__loop1!!"));
    // The `else` runs (and assigns) only when the loop never did.
    assert!(main.content.contains("var __loop1_ran = false"));
    assert!(main.content.contains("if (!__loop1_ran) {"));
    assert!(main.content.contains("__loop1 = -1"));
    // `break value` assigns the result local before breaking.
    assert!(main.content.contains("__loop3 = x\n    break"));
    // Without `else` (or with a bare `break`) the local stays nullable:
    // no `!!` unwrap on the `found`/`capped` loops.
    assert!(main.content.contains("__loop3\n}"));
    // The number shifted by one when a `for` over an `Iter<T>` started naming
    // its pass: driving a producer costs one fresh loop name.
    assert!(main.content.contains("__loop6\n}"));
    // Statement-position `else` needs only the ran-flag, no `run {}`.
    assert!(main.content.contains("var __loop4_ran = false"));
    assert!(main.content.contains("if (!__loop4_ran) {"));
    // A union-typed loop value re-wraps to the declared arm order.
    assert!(main
        .content
        .contains("var __loop7: Union2<String, Int>? = null"));
    assert!(main.content.contains("}.let { when (it) {"));
}

// [while-value]
#[test]
fn break_outside_a_loop_is_an_error() {
    let errors = expect_errors("fn f() -> None {\n    break\n}\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`break` outside of a loop")),
        "unexpected errors: {errors:?}"
    );
}

/// Full verification of the loops demo under kotlinc (skipped when
/// kotlinc is not installed).
fn kotlinc_compiles_and_runs_loops() -> KotlinCase {
    let files = generate_loops_demo();
    let expected = "last: 40\nnever: -1\nfound: 4\nempty range\ncapped: 2\nok: 2\n";
    kotlin_case(files, "loops", expected)
}

// ===== M7: reachability, packages/imports, companions =====

/// A multi-module program: `main` uses `geometry` (imported) but not
/// `unused`; `geometry` has a Kotlin companion file.
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(main.content.starts_with("package salvo.main\n"));
    assert!(main.content.contains("import salvo.core.console.*"));
    assert!(main.content.contains("import salvo.geometry.*"));
    let geometry = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "geometry.kt")
        .unwrap();
    assert!(geometry.content.starts_with("package salvo.geometry\n"));
    // `geometry` references nothing foreign: no generated imports.
    assert!(!geometry.content.contains("import salvo."));
}

// [kt-imports] Aliased Salvo imports become Kotlin alias imports, and the
// call site keeps the alias.
#[test]
fn aliased_imports_emit_kotlin_alias_imports() {
    let main = r#"
import geometry.area as rect_area

fn main() [use] -> None {
    use StdOutConsole
    println("area: ${rect_area(3, 4)}")
}
"#;
    let geometry = "export fn area(w: Int, h: Int) -> Int {\n    return w * h\n}\n";
    let program = build_program(&[("main.sv", main), ("geometry.sv", geometry)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(main
        .content
        .contains("import salvo.geometry.area as rect_area"));
    assert!(main.content.contains("rect_area(3, 4)"));
}

// [kt-imports] [kt-qual-mangling] An aliased import of a fn with a
// mangled qualified overload emits one alias import per overload symbol,
// and aliased call sites keep the mangling suffix.
const MANGLED_ALIAS_MAIN: &str = r#"
import lib.shout as holler
import lib.loud

fn main() [use] -> None {
    use StdOutConsole
    println(holler("hi"))
    let l = loud("hey")
    println(holler(l))
}
"#;

const MANGLED_ALIAS_LIB: &str = r#"
export qualifier Loud of Str

export fn loud(s: Str) -> Str as Loud {
    return s
}

export fn shout(s: Str) -> Str {
    return s
}

export fn shout(s: Loud Str) -> Str {
    return "${s}!"
}
"#;

#[test]
fn aliased_import_of_mangled_overload_keeps_suffix() {
    let program = build_program(&[
        ("main.sv", MANGLED_ALIAS_MAIN),
        ("lib.sv", MANGLED_ALIAS_LIB),
    ]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    for needle in [
        "import salvo.lib.shout as holler",
        "import salvo.lib.shout__Loud as holler__Loud",
        "holler(\"hi\")",
        "holler__Loud(l)",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

fn kotlinc_compiles_and_runs_mangled_alias() -> KotlinCase {
    let program = build_program(&[
        ("main.sv", MANGLED_ALIAS_MAIN),
        ("lib.sv", MANGLED_ALIAS_LIB),
    ]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "mangled-alias", "hi\nhey!\n")
}

// [kt-effect-params] Effect parameters avoid user parameter names.
#[test]
fn effect_params_avoid_user_names() {
    let src = r#"
export fn shadowed(console: Str) [Console] -> None => !console {
    println("param: ${console}")
}

export fn main() [use] -> None {
    use StdOutConsole
    shadowed("value")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // The generated effect parameter picks a fresh name.
    assert!(
        main.content
            .contains("fun shadowed(console2: Console, console: String)"),
        "unexpected: {}",
        main.content
    );
    assert!(main
        .content
        .contains("println(console2, \"param: $console\")"));
}

// [let-destructure] Two struct destructures in one block get unique temps.
#[test]
fn struct_destructure_temps_are_unique() {
    let src = r#"
export struct Point {
    x: Int,
    y: Int
}

export fn main() [use] -> None {
    use StdOutConsole
    let {x} = Point {x: 1, y: 2}
    let {y} = Point {x: 3, y: 4}
    println("${x} ${y}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(main.content.contains("val __destructured1 ="));
    assert!(main.content.contains("val __destructured2 ="));
}

/// Full verification of the multi-module program under kotlinc: packages,
/// generated imports, companion file, and reachability all have to hold
/// together for this to compile and run.
fn kotlinc_compiles_and_runs_multi_module() -> KotlinCase {
    let program = build_multi_module();
    let files = salvo_backend_kotlin::emit_program(&program).unwrap();
    kotlin_case(files, "multimod", "area: 12\n")
}

// [backend-companion] A companion must not collide with a generated file.
#[test]
fn companion_collision_with_generated_file_is_an_error() {
    let mut program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdOutConsole\n    println(\"hi\")\n}\n",
    )]);
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: "package salvo.main\n".to_string(),
        platform: false,
    });
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected collision error");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("companion file `main.kt` collides")),
        "unexpected errors: {errors:?}"
    );
}

// [kt-handler-template-return] A handler member with a return type returns
// its template's value (`return run { ... }`), so value-producing handler
// members like `random() -> Double` compile; statement-only members are
// unchanged.
#[test]
fn handler_members_with_return_types_return_their_template() {
    let program = build_program(&[(
        "main.sv",
        "import random.Random\nimport random.DefaultRandom\n\n\
         fn roll() [Random] -> Double {\n    return random()\n}\n\n\
         fn main() [use] -> None {\n    use StdOutConsole\n    use DefaultRandom\n    \
         println(\"${roll() < 2.0}\")\n}\n",
    )]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let random = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "random.kt")
        .expect("random.kt not emitted");
    assert!(
        random.content.contains("override fun random(): Double {"),
        "content: {}",
        random.content
    );
    assert!(
        random.content.contains("return run {"),
        "content: {}",
        random.content
    );
    assert!(
        random.content.contains("kotlin.random.Random.nextDouble()"),
        "content: {}",
        random.content
    );
    // Statement-only members (Console.print) keep their plain body.
    let console = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "core/console.kt")
        .expect("core/console.kt not emitted");
    assert!(
        !console.content.contains("return run {"),
        "content: {}",
        console.content
    );
}

// [lit-numeric] Literal suffixes map onto Kotlin's: `5L` stays `5L`,
// `2.5f` stays `2.5f`, unsuffixed floats are Double (`1.5`).
#[test]
fn numeric_literal_suffixes_emit_kotlin_suffixes() {
    let program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
         let big: Long = 5L\n    let ratio: Float = 2.5f\n    let d: Double = 1.5\n    \
         println(\"${big} ${ratio} ${d}\")\n}\n",
    )]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    assert!(main.content.contains("= 5L"), "content: {}", main.content);
    assert!(main.content.contains("= 2.5f"), "content: {}", main.content);
    assert!(main.content.contains("= 1.5"), "content: {}", main.content);
}

// [type-union] A union-arm argument (`ok("x")` where `Ok Str | Err Str`
// is expected) resolves and wraps into the declared union's arm at the
// call site — checker overload resolution and emitter wrapping agree.
#[test]
fn union_arm_arguments_wrap_at_call_sites() {
    let src = r#"
export fn describe(v: Ok Str | Err Str) -> Str {
    when v {
        is Ok {
            return "ok"
        }
        is Err {
            return "err"
        }
    }
}

export fn main() [use] -> None {
    use StdOutConsole
    println(describe(ok("x")))
    println(describe(err("y")))
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    // Positional arm identity: Ok -> first arm, Err -> second arm.
    assert!(
        main.content
            .contains("describe(U2_1<String, String>(ok(\"x\")))"),
        "content: {}",
        main.content
    );
    assert!(
        main.content
            .contains("describe(U2_2<String, String>(err(\"y\")))"),
        "content: {}",
        main.content
    );
}

// ===== S1: the `copy` intrinsic [intrinsic-fn] [copy-fn] [kt-copy] =====

/// Exercises every Kotlin `copy` lowering shape: identity for immutable
/// data, `.toMutableList()` for `Mut List`, `.copy()` for a `Mut` struct
/// with immutable fields, and `.copyOf()` for arrays — plus a fate-linked
/// alias (`let zs = xs`) that must stay readable.
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

// [intrinsic-fn] [kt-copy] `copy` lowers type-directedly from the
// checker's resolved argument type.
#[test]
fn copy_lowers_type_directedly() {
    let program = build_program(&[("main.sv", COPY_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted");
    // Identity for a transitively immutable type (Str).
    assert!(
        main.content.contains("val t = s\n"),
        "generated:\n{}",
        main.content
    );
    // A real copy for `Mut List<Int>`.
    assert!(
        main.content.contains("val ys = xs.toMutableList()"),
        "generated:\n{}",
        main.content
    );
    // The data-class shallow copy for a `Mut` struct with immutable fields.
    assert!(
        main.content.contains("val q = p.copy()"),
        "generated:\n{}",
        main.content
    );
    // Arrays are index-assignable without `Mut`, so they copy for real.
    assert!(
        main.content.contains("val brr = arr.copyOf()"),
        "generated:\n{}",
        main.content
    );
}

// [kt-copy] [backend-never-wrong] Nested mutability has no correct
// shallow copy on the JVM: codegen error, never a diverging copy.
#[test]
fn copy_of_nested_mutable_type_is_an_error() {
    let program = build_program(&[(
        "bad.sv",
        "fn main() [use] -> None {\n    use StdOutConsole\n    \
         let xs = mut_list_of(mut_list_of(1))\n    let ys = copy(xs)\n    \
         println(\"${ys.size()}\")\n}\n",
    )]);
    let result = salvo_backend_kotlin::emit_program(&program);
    let errors = result.err().expect("expected codegen errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("cannot `copy` a value of type `Mut List<Mut List<Int>>`")),
        "unexpected errors: {errors:?}"
    );
}

fn kotlinc_compiles_and_runs_copy() -> KotlinCase {
    let program = build_program(&[("main.sv", COPY_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "hi\n3 4\na b\n1 9\n3\n";
    kotlin_case(files, "copy", expected)
}

// ===== S2: move-mode bindings [fate-move-mode] =====

/// The same program as the Rust backend's S2 demo: Kotlin emission is
/// unchanged by move modes (aliases stay aliases; the checker's
/// consumption of the ancestors is what keeps the difference from
/// Rust's real moves unobservable — backend-parity principle). The
/// exact stdout must match the Rust run.
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

fn kotlinc_compiles_and_runs_move_modes() -> KotlinCase {
    let program = build_program(&[("main.sv", S2_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "Grace\n3\n";
    kotlin_case(files, "s2-moves", expected)
}

// ===== S3: borrow emission parity mirror [fate-link] =====

/// The same program as the Rust backend's S3 borrow demo: Kotlin's
/// aliases and Rust's borrows are the same semantics, so the stdout
/// must match exactly.
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

fn kotlinc_compiles_and_runs_borrows() -> KotlinCase {
    let program = build_program(&[("main.sv", S3_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "1\n5\n";
    kotlin_case(files, "s3-borrows", expected)
}

// ===== L6: linear types [linear-obligation] =====

/// The resource pattern linearity exists for: open, use, close — with
/// `discard` as the deliberate drop [linear-discard]. The checker
/// guarantees no path leaks the handle; the demo verifies the lowering
/// (Kotlin: `discard` evaluates and ignores via `.let {}`).
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

fn kotlinc_compiles_and_runs_linear() -> KotlinCase {
    let program = build_program(&[("main.sv", LINEAR_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "open data.txt\nfd=8\nclose fd=8\nopen scratch\ndone\n";
    kotlin_case(files, "l6-linear", expected)
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

fn kotlinc_compiles_and_runs_linear_generics() -> KotlinCase {
    let program = build_program(&[("main.sv", LINEAR_GENERICS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "open 9\nheld fd=9\ndone\n";
    kotlin_case(files, "l7a-linear-generics", expected)
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

fn kotlinc_compiles_and_runs_once_fns() -> KotlinCase {
    let program = build_program(&[("main.sv", ONCE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "consumed 3 items\nplain 7\ndone\n";
    kotlin_case(files, "l7b-once", expected)
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

fn find_adult(persons: List<Person>) -> proj[from: persons] Person? => persons {
    for person in persons {
        if person.age >= 18 {
            return person
        }
    }
    return None
}

fn head_of(persons: List<Person>, tag: Str) -> proj[from: persons] Person? => persons, tag {
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

fn kotlinc_compiles_and_runs_derived_returns() -> KotlinCase {
    let program = build_program(&[("main.sv", DERIVED_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "adult: Grace\nhead: Kid\ndone\n";
    kotlin_case(files, "l7c-derived", expected)
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

fn kotlinc_compiles_and_runs_fn_contracts() -> KotlinCase {
    let program = build_program(&[("main.sv", CONTRACTS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "twice=4\nstill=2\nnamed=4\neaten=2\ndone\n";
    kotlin_case(files, "l7d-contracts", expected)
}

// ===== precedence-aware binary rendering =====

/// Parser grouping must survive re-rendering: without precedence-aware
/// parenthesization `(a - b) * c` would emit flat as `a - b * c` and
/// silently re-associate.
const PRECEDENCE_DEMO: &str = r#"
fn main() [use] -> None {
    use StdOutConsole
    let a = 10
    let b = 3
    let c = 2
    let grouped = (a - b) * c
    let nested = a - (b - c)
    let neg = -(a + b)
    let logical = (a < b || b > c) && a > c
    println("${grouped} ${nested} ${neg} ${logical}")
}
"#;

#[test]
fn binary_rendering_preserves_grouping() {
    let program = build_program(&[("main.sv", PRECEDENCE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .expect("main.kt emitted");
    for needle in [
        "(a - b) * c",
        "a - (b - c)",
        "-(a + b)",
        "(a < b || b > c) && a > c",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

fn kotlinc_compiles_and_runs_precedence() -> KotlinCase {
    let program = build_program(&[("main.sv", PRECEDENCE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "precedence", "14 9 -13 true\n")
}

// ===== bare `return` in value-position blocks of iterator bodies =====
// [iter-protocol] [kt-iter-pass]

// ===== `is` on union-typed struct-field subjects =====
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects, and narrow like them [flow-place];
// `when` still requires a variable subject [when-union-subject].

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
    let program = build_program(&[("main.sv", FIELD_IS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("h.result is U2_1<*, *>"),
        "expected union test on the field in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_field_is() -> KotlinCase {
    let program = build_program(&[("main.sv", FIELD_IS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "field-is", "ok 1\nplain 1\nerr bad\n")
}

// [when-union-subject] `when` still requires a plain variable subject.
#[test]
fn when_field_subject_is_rejected() {
    let src = r#"
export qualifier Ok<T> of T

export fn ok<T>(value: T) -> T as Ok {
    return value
}

export struct Holder {
    result: Ok Int | Str
}

export fn main() [use] -> None {
    use StdOutConsole
    let h = Holder {result: ok(1)}
    when h.result {
        is Ok {
            println("ok")
        }
        is Str {
            println("str")
        }
    }
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`when` requires a plain variable as its subject")),
        "unexpected errors: {errors:?}"
    );
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

/// [flow-place] [kt-narrow-field-assert] A narrowed nullable *field* read
/// asserts instead of relying on a smart cast (Kotlin refuses to smart-cast
/// a property, and a `canbe Mut` struct's fields are `var`); a narrowed
/// wrapper-union field reads its arm payload.
#[test]
fn narrowed_field_reads_unwrap() {
    let program = build_program(&[("main.sv", PLACE_NARROW_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("\"${p.name} ${p.surname!!}\""),
        "expected the narrowed field read to assert in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${p.address.city!!}"),
        "expected the narrowed field *chain* read to assert in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${(h.result.value as Int)}"),
        "expected the narrowed wrapper-union field read to unwrap in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_place_narrowing() -> KotlinCase {
    let program = build_program(&[("main.sv", PLACE_NARROW_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "place-narrow", "Ann Lee\nBo\ncity Oslo\nok 3\n")
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

/// [kt-narrow-field-assert] The assert is only needed for `var`
/// properties: a field of a struct without `canbe Mut` emits as `val`,
/// which kotlinc smart-casts — asserting there would produce an
/// "unnecessary non-null assertion" warning.
#[test]
fn narrowed_val_field_relies_on_the_smart_cast() {
    let program = build_program(&[("main.sv", PLACE_OPERAND_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("${r.value + 1}"),
        "expected the plain smart-cast read in:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("r.value!!"),
        "a `val` property should not be asserted in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_place_operand() -> KotlinCase {
    let program = build_program(&[("main.sv", PLACE_OPERAND_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "place-operand", "temp: 22\nnone: no value\n")
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

/// [kt-tuple-component] Tuple elements map onto `Pair`/`Triple`
/// components, and a narrowed one asserts — a stdlib property is not
/// smart-cast [kt-narrow-field-assert].
#[test]
fn tuple_elements_emit_pair_components() {
    let program = build_program(&[("main.sv", TUPLE_INDEX_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("${t.first} ${t.second} ${t.third}"),
        "expected Pair/Triple components in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${nested.second.first}"),
        "expected a nested component chain in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${maybe.first!!}"),
        "expected the narrowed element to assert in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_tuple_index() -> KotlinCase {
    let program = build_program(&[("main.sv", TUPLE_INDEX_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "tuple-index", "1 two true\nin 9\nsome here 6\n")
}

// ===== list constructors carry their element type =====
// `mut_list_of()` must not emit a bare `mutableListOf()`, which kotlinc
// cannot infer: an empty list constructor renders its element type from the
// call's resolved type arguments (`mutableListOf<Int>()`), since Kotlin
// cannot infer a type argument that the source never wrote.

/// The std list constructors use this: `mut_list_of()` must not emit a bare
/// `mutableListOf()`, which kotlinc cannot infer.
#[test]
fn list_constructors_carry_their_element_type() {
    let src = r#"
export fn main() [use] -> None {
    use StdOutConsole
    let xs: Mut List<Int> = mut_list_of()
    let ys = list_of<Str>()
    add(xs, 1)
    println("${size(xs)} ${size(ys)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("mutableListOf<Int>()") && main.content.contains("listOf<String>()"),
        "expected explicit element types:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_list_element_types() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
    use StdOutConsole
    let xs: Mut List<Int> = mut_list_of()
    add(xs, 1)
    add(xs, 2)
    let ys = mut_list_of<Str>()
    add(ys, "a")
    println("${size(xs)} ${size(ys)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "list-element-types", "2 1\n")
}

// ===== collections [col-insertion-order] =====

/// The **same source and the same expected output** as the Rust backend's
/// `rustc_compiles_and_runs_collections`, which is what makes the pair a
/// [backend-parity] test: Kotlin gets insertion order from `LinkedHashMap` /
/// `LinkedHashSet` and Rust from its runtime module [rs-collections], and
/// the two must be indistinguishable from Salvo. Keep the two copies in
/// step — a divergence here is a defect, not a test bug.
fn kotlinc_compiles_and_runs_collections() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "collections",
        "set: {b, a, c} size: 3\n\
         add new: true dup: false now: {b, a, c, d}\n\
         contains: true remove: true again: false left: {b, c, d}\n\
         dedup: {1, 2, 3} size: 3\n\
         map: {one: 1, two: 2, three: 3}\n\
         overwrite keeps position: {one: 111, two: 2, three: 3}\n\
         get: 2\n\
         absent: true removed: 2 left: {one: 111, three: 3}\n\
         contains_key: true size: 2\n\
         dup keys: {x: 9, y: 2}\n\
         int keys: {7: seven, 3: three}\n",
    )
}

/// [iter-pass] The same source and expected output as the Rust backend's
/// `rustc_compiles_and_runs_collection_iteration` — iterating a set's
/// elements and a map's keys [backend-parity].
fn kotlinc_compiles_and_runs_collection_iteration() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "collection-iter",
        "elem: b\n\
     elem: a\n\
     elem: c\n\
     as list: [b, a, c]\n\
     total length: 3\n\
     one -> 1\n\
     two -> 2\n\
     three -> 3\n\
     keys: [one, two, three]\n\
     key length total: 11\n\
     done\n",
    )
}

/// [col-literal] The same source and expected output as the Rust backend's
/// `rustc_compiles_and_runs_collection_literals` [backend-parity].
fn kotlinc_compiles_and_runs_collection_literals() -> KotlinCase {
    let src = r#"
export struct Point {
    x: Int,
    y: Int
}

export fn takes_set(s: Set<Int>) -> Int => s {
    return size(s)
}

export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "collection-literals", "list: [1, 2, 3]\n\
     mut list: [4, 5, 6]\n\
     set: {b, a}\n\
     map: {one: 1, two: 2}\n\
     empty then filled: {9}\n\
     empty map: {} via param: 0\n\
     struct literal still: 1,2\n\
     array: 2 7\n")
}

/// [col-equality] [kt-float-eq] The same source and expected output as the
/// Rust backend's `rustc_compiles_and_runs_equality_and_ordering`
/// [backend-parity] — including `struct nan equality: false`, which the data
/// class's own `equals` would have reported as `true`.
fn kotlinc_compiles_and_runs_equality_and_ordering() -> KotlinCase {
    let src = r#"
export struct Point canbe hashed, ordered {
    x: Int,
    y: Int
}

export struct Version canbe hashed, ordered {
    parts: List<Int>,
    label: (Str, Int)
}

export struct Measure {
    value: Double
}

export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "equality-ordering", "eq: true ne: true lt: true ge: true\n\
     struct key: size 1 dup false\n\
     struct lookup: nine\n\
     list order: true prefix: true\n\
     struct nan equality: false\n")
}

/// [op-arith] [op-promote] [lit-adopt] [op-convert] The same source and
/// expected output as the Rust backend's `rustc_compiles_and_runs_operators`
/// [backend-parity]. Kotlin's own operator set covers the mixed widths
/// (`Long.plus(Int)`, `Int.compareTo(Long)`), so only the adopted literals
/// need rendering care (`1L`, `3.0`).
fn kotlinc_compiles_and_runs_operators() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("val x: Long = 1L") && main.content.contains("val d: Double = 3.0"),
        "expected adopted literals rendered at their checked type:\n{}",
        main.content
    );
    kotlin_case(
        files,
        "operators",
        "11 8000000005 8.25 0.75 false -294967296 -7\n",
    )
}

/// [effect-at] [effect-member-overload] The same source and expected
/// output as the Rust backend's `rustc_compiles_and_runs_effect_selectors`
/// [backend-parity]: two effects declaring `close`, resolved by
/// availability and by the `@Effect` selector.
fn kotlinc_compiles_and_runs_effect_selectors() -> KotlinCase {
    let src = r#"
export effect Store {
    fn open(path: Str) -> Int => path
    fn close(handle: Int) -> Str
}

export effect Net {
    fn close(handle: Int) -> Str
}

export handler MemStore of Store {
    fn open(path: Str) -> Int => path {
        return 7
    }
    fn close(handle: Int) -> Str {
        return "fs closed ${handle}"
    }
}

export handler MemNet of Net {
    fn close(handle: Int) -> Str {
        return "net closed ${handle}"
    }
}

export fn shut(h: Int) [Store] -> Str {
    return close(h)
}

export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "effect-at",
        "fs closed 7\nfs closed 7\nnet closed 9\nnet closed 7\n",
    )
}

/// [col-sorted] The same source and expected output as the Rust backend's
/// `rustc_compiles_and_runs_sorted_collections` [backend-parity].
fn kotlinc_compiles_and_runs_sorted_collections() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "sorted-collections", "set: {apple, fig, pear}\n\
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
     k two\n")
}

/// [kt-ordered] Strings sort by **code point**: the JVM's own
/// `String.compareTo` is UTF-16 code-unit order, which would put an astral
/// character before a BMP one and disagree with Rust [backend-parity]. Paired
/// with the Rust backend's `rustc_compiles_and_runs_codepoint_string_order`.
fn kotlinc_compiles_and_runs_codepoint_string_order() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "codepoint-order", "smallest is the BMP char: true\n")
}

/// [col-by] [col-convert] The same source and expected output as the Rust
/// backend's `rustc_compiles_and_runs_generated_constructors`
/// [backend-parity]. Notable on this backend: the callbacks are handed to
/// Kotlin builders rather than invoked inline, since an immediately applied
/// lambda literal has no expected type for kotlinc to infer from.
fn kotlinc_compiles_and_runs_generated_constructors() -> KotlinCase {
    let src = r#"
export fn main() [use] -> None {
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
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "by-and-convert", "array_by 0 2 4\n\
     list_by [10, 11, 12, 13]\n\
     set_by {0, 1, 2}\n\
     map_by {0: 0, 1: 1, 2: 4}\n\
     to_set {1, 2, 3}\n\
     to_map pairs {a: 1, b: 2}\n\
     to_map rule {alpha: 5, be: 2}\n\
     assigned 9 size 3\n")
}

// ===== interception [effect-intercept] =====
// A handler may depend on the effect it implements, binding strictly
// outward, and a `use` may shadow an earlier registration [use-no-dup].
// Kotlin needs no new machinery for either — a dependency is a constructor
// field, and the fused class keeps one property per instance — but the
// *lookups* had to become innermost-first: reading the environment as a set
// made a shadowed handler answer calls the inner one owned, which printed
// the outer greeting here and the inner one on Rust (found 2026-09-14).
// The program and its expected stdout are shared with the Rust backend.

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

/// [effect-intercept] [use-no-dup] [kt-effect-fusion] An intercepting
/// handler is constructed with the instance registered *before* it, and the
/// fused class carries one property per instance — the shadowed one is not
/// duplicated.
#[test]
fn interception_binds_outward_and_shadows() {
    let program = build_program(&[("main.sv", INTERCEPTION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    let c = &main.content;
    // Each interceptor is handed a fused environment built from the
    // *previous* registration, and each `shout` reads the newest one.
    assert!(
        c.contains("Counting(__Fx_3(__fx3.__fx_Greeter))")
            && c.contains("shout(__fx4, \"c\")")
            && c.contains("Loud(__Fx_3(__fx4.__fx_Greeter))")
            && c.contains("shout(__fx5, \"d\")"),
        "an interceptor must bind the instance registered before it, and \
         later calls must reach the interceptor:\n{c}"
    );
    // Out of the block, calls go back to the fusion that outlived it.
    assert!(
        c.contains("shout(__fx4, \"e\")"),
        "a shadowing registration expires with its block:\n{c}"
    );
    // One property per instance: a shadowed `Greeter` is not a second field,
    // and one effect *set* is one class however many scopes need it.
    for class in c.split("class __Fx_") {
        let count = class.matches("__fx_Greeter: Greeter").count();
        assert!(count <= 1, "duplicated property in a fused class:\n{c}");
    }
    let classes: Vec<&str> = c.match_indices("\nclass __Fx_").map(|(_, s)| s).collect();
    let bodies: std::collections::HashSet<&str> = c
        .split("\nclass __Fx_")
        .skip(1)
        .map(|rest| rest.split_once('(').map(|(_, b)| b).unwrap_or(rest))
        .collect();
    assert_eq!(
        classes.len(),
        bodies.len(),
        "two fused classes with the same effect set were emitted:\n{c}"
    );
}

fn kotlinc_compiles_and_runs_interception() -> KotlinCase {
    let program = build_program(&[("main.sv", INTERCEPTION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "interception",
        "hello a\nGood day, b\nGood day, c (1)\nGood day, d (2)!\n\
         Good day, e (3)\nkept\nkept\nkept kept\nkept\n",
    )
}

// ===== the shareable-by-default interceptor chain [use-local] =====
// The production shape the 2026-09-20 arc was built for, with no `local`
// anywhere: a stateful handler binds as a monitor, a stateless dependent
// handler binds bare with its dependencies injected at construction — one
// of them the effect it implements (interception binds outward), another a
// monitor. The program and its stdout are shared with the Rust backend.

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

/// [use-local] [effect-handler-deps] [kt-monitor] The chain end to end:
/// `MemCounter` behind `__Mon_Counter`, `PlainLogger` bare with an injected
/// console, `Shout` wrapping the `Logger` registered before it — and the
/// count proving both `work` calls went through the interceptor and the
/// monitor. Byte-identical stdout on Rust.
fn kotlinc_compiles_and_runs_a_shareable_interceptor_chain() -> KotlinCase {
    let program = build_program(&[("main.sv", SHAREABLE_CHAIN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "shareable-chain",
        "log: plain\nlog: loud!\nlog: louder!\nshouted 2\n",
    )
}

// ===== spawn-inheritance and the `with` clause [spawn-inherit] =====
// The arc's shape, shared verbatim with the Rust backend (whose test also
// pins the hidden handle bundle that side needs; here a JVM reference already
// is a handle, so nothing extra is emitted).

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

/// [spawn-inherit] [with-clause] The arc end to end: interception wiring in a
/// fn that received the effects it wires, a spawn inheriting its dependency
/// from the scope with no clause, and a `with` clause overriding one with a
/// private instance. Byte-identical stdout on Rust.
fn kotlinc_compiles_and_runs_spawn_inheritance() -> KotlinCase {
    let program = build_program(&[("main.sv", SPAWN_INHERIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "spawn-inherit", SPAWN_INHERIT_OUTPUT)
}

/// [use-local] [effect-handler-deps] A **generic** dependent handler works
/// in the owned-handle form: `Relay<T>`'s dependency is a handle field, so
/// no carrier has to name the handler's generics — the cut that still
/// stands for the fusion form (`a_generic_dependency_is_refused`) does not
/// bind here.
fn kotlinc_compiles_and_runs_a_generic_handler_with_a_handle_dep() -> KotlinCase {
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
    let program = build_program(&[("main.sv", SRC)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "generic-handle-dep", "relayed\nok\n")
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

/// [effect-handler-deps] [kt-effect-fusion] The dependencies arrive as **one
/// fused value, built at the `use` site and stored** for the handler's
/// lifetime; the member reaches them through that field (its signature must
/// match the interface), and `work` takes only the Logger — behind its Has
/// bound, since dependency programs fuse.
///
/// The carrier is a **type parameter** bounded by the Has interfaces, not one
/// of the `__Fx_N` classes: those are minted per *file*, so naming one in the
/// class's signature left a handler declared in another module
/// unconstructable — found 2026-09-14 by std's `DefaultFs [RawFs]`, whose
/// `use` site lives in the program.
#[test]
fn handler_dependencies_inject_at_construction() {
    let program = build_program(&[("main.sv", HANDLER_DEPS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains(
            "class ConsoleLogger<__Fx>(private val __fx: __Fx) : Logger \
             where __Fx : __Has_Console"
        ),
        "expected one stored fused value rather than a field per dependency \
         in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("override fun log(message: String)"),
        "the member signature must match the interface in:\n{}",
        main.content
    );
    // No per-call combiner: the member body opens with its own statements.
    assert!(
        !main
            .content
            .contains("override fun log(message: String) {\n        val __fx"),
        "a member body must not rebuild the fused value per call in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("ConsoleLogger(__Fx_1(__fx.__fx_Console))"),
        "expected the `use` site to build the handler's environment in:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("fun<__Fx> work(__fx: __Fx) where __Fx : __Has_Logger"),
        "callers should not mention the dependency, and a single effect \
         still fuses (uniformity) in:\n{}",
        main.content
    );
    // The Has-accessor interface is generated once, in fx.kt.
    let fx = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "fx.kt")
        .expect("fx.kt emitted");
    assert!(
        fx.content
            .contains("interface __Has_Logger {\n    val __fx_Logger: Logger\n}"),
        "expected the accessor interface in fx.kt:\n{}",
        fx.content
    );
}

fn kotlinc_compiles_and_runs_handler_dependencies() -> KotlinCase {
    let program = build_program(&[("main.sv", HANDLER_DEPS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "handler-deps", "LOG: from work\nLOG: from main\n")
}

// [effect-handler-deps] The same programs the Rust backend runs through its
// own emission, asserting the *same* stdout here: that is what backend
// parity means for handler dependencies. Kotlin needs no fusion — objects
// alias — so these also pin that the two strategies agree on handler state,
// dependency chains, `use` in a loop, and nested effect calls.

const HANDLER_DEPS_FUSION_DEMO: &str = r#"
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

handler ConsoleLogger [Console] of Logger {
    seen: Int = 0
    fn log(message: Str) -> None => message {
        seen = seen + 1
        println("LOG ${seen}: ${message}")
    }
}

handler CountingAudit [Console, Counter] of Audit {
    fn note(message: Str) -> None => message {
        bump()
        println("[${total()}] ${message}")
    }
}

fn shout(message: Str) [Console] -> None => message {
    println("!! ${message}")
}

fn banner() [Console, Logger] -> None {
    log("banner")
    shout("done")
}

fn draw() [Console, Random<Int>, use] -> None {
    use MemCounter
    bump()
    println("drew ${next_random<Int>()} at ${total()}")
}

fn main() [use] -> None {
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
    use CyclicRandom(array_of(10, 20, 30))
    draw()
    draw()
}
"#;

fn kotlinc_compiles_and_runs_handler_deps_in_anger() -> KotlinCase {
    let program = build_program(&[("main.sv", HANDLER_DEPS_FUSION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "handler-deps-anger",
        "LOG 1: banner\n!! done\n[1] inner\nLOG 2: banner\n!! done\n\
         LOG 3: outer again\ndrew 20 at 1\ndrew 30 at 1\n",
    )
}

const HANDLER_DEPS_CHAIN_DEMO: &str = r#"
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

fn kotlinc_compiles_and_runs_handler_deps_chain() -> KotlinCase {
    let program = build_program(&[("main.sv", HANDLER_DEPS_CHAIN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "handler-deps-chain",
        "LOG: note 1: first\nLOG: loop 1 tally 1\nLOG: loop 2 tally 2\n\
         labelling 7\nLOG: n=7\nchecking hello\nLOG: note 2: loud\n",
    )
}

const HANDLER_DEPS_MIXED_DEMO: &str = r#"
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

fn kotlinc_compiles_and_runs_handler_deps_mixed() -> KotlinCase {
    let program = build_program(&[("main.sv", HANDLER_DEPS_MIXED_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "handler-deps-mixed",
        "L[3] in lambda a\n  tallied 1\nL[3] done a\n  tallied 1\nL[3] one #1\n\
         \x20 tallied 1\nL[3] loop 1 #2\n  tallied 1\nL[3] loop 1 #3\n  tallied 1\n",
    )
}



// ===== destructuring a loop element [let-destructure] =====

/// [let-destructure] A `for` binds a **pattern**, not just a name. Source and
/// expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_loop_destructuring`.
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

fn kotlinc_compiles_and_runs_loop_destructuring() -> KotlinCase {
    let program = build_program(&[("main.sv", LOOP_DESTRUCTURE)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "loop-destructure", "a=1\nb=2\nx 7\ny 8\n3\n7\nsum 3\n")
}

/// [let-destructure] What it lowers to: the element in a temporary — cast to its
/// own type, which is what a pass-driven header needs — then one `val` per name,
/// a tuple's by component and a struct's by field. Kotlin *could* destructure a
/// `Pair` in the header, but not the pass's payload and not a struct, so one
/// shape serves every loop and matches the Rust backend's.
#[test]
fn loop_destructuring_binds_the_parts_of_a_temporary_kotlin() {
    let program = build_program(&[("main.sv", LOOP_DESTRUCTURE)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    let text = &main.content;
    assert!(
        text.contains("as Pair<String, Int>"),
        "the element temporary must be cast to its own type:\n{text}"
    );
    assert!(
        text.contains("val k = __elem.first") && text.contains("val v = __elem.second"),
        "expected the tuple parts by component name:\n{text}"
    );
    assert!(
        text.contains("val who = __elem2.name") && text.contains("val score = __elem2.score"),
        "expected the struct fields by name, renamed:\n{text}"
    );
    assert!(
        text.contains("var a = __elem3.first"),
        "expected an assigned-to binding to be a `var`:\n{text}"
    );
}

// ===== tuples past three [kt-tuple-class] =====

/// [kt-tuple-class] [type-tuple] Kotlin has `Pair` and `Triple` and nothing
/// past them, so a bigger tuple is a **generated data class** — which is what
/// makes indexing, destructuring, hashing and ordering work on the same terms
/// as a `Pair`'s. Source and expected stdout are **verbatim** the Rust
/// backend's `rustc_compiles_and_runs_tuples_past_three`.
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

fn kotlinc_compiles_and_runs_tuples_past_three() -> KotlinCase {
    let program = build_program(&[("main.sv", BIG_TUPLE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "big-tuple", "1 two true 4 five\n1 two true 4 five\n1999\n2000\n")
}

/// [kt-tuple-class] What a tuple past three lowers to: a `SalvoTupleN` literal,
/// the `v3`/`v4` component names past `Pair`'s three [kt-tuple-component], and
/// a generated `tuples.kt` whose class is a `data class` (structural equality
/// and `componentN`, so a `Set` element and a destructuring both work)
/// implementing `SalvoTuple` (which is how `__salvoCompare` orders it).
#[test]
fn tuples_past_three_emit_a_generated_class() {
    let program = build_program(&[("main.sv", BIG_TUPLE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("SalvoTuple5(1, \"two\", true, 4, \"five\")"),
        "expected a generated tuple literal in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${q.third} ${q.v3} ${q.v4}"),
        "expected the component names past `Pair`'s three in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("SalvoTuple5<Int, String, Boolean, Int, String>"),
        "expected the generated type in the annotation in:\n{}",
        main.content
    );
    let tuples = files
        .iter()
        .find(|f| f.rel_path.ends_with("tuples.kt"))
        .expect("tuples.kt is emitted for a program that names a big tuple");
    assert!(
        tuples.content.contains("data class SalvoTuple5<")
            && tuples.content.contains(") : SalvoTuple {")
            && tuples.content.contains("val v3: T4"),
        "the generated tuple class is wrong:\n{}",
        tuples.content
    );
    assert!(
        tuples.content.contains("data class SalvoTuple4<"),
        "the 4-tuple the sorted set needs is missing:\n{}",
        tuples.content
    );
}

// ===== union coercion inside arrays/tuples/lambda returns =====
// [type-union] Elements of array/tuple literals and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.

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
    let program = build_program(&[("main.sv", NESTED_COERCION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    for needle in [
        // [col-literal] `[...]` builds a List now, so the coercion wrappers
        // appear inside `listOf` rather than `arrayOf`.
        "listOf<Union2<Int, String>>(U2_1<Int, String>(tag_ok(1)), \
         U2_2<Int, String>(tag_err(\"a\")))",
        "Pair(\"t\", U2_1<Int, String>(tag_ok(2)))",
        "U2_1<Int, String>(tag_ok(3))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

fn kotlinc_compiles_and_runs_nested_coercion() -> KotlinCase {
    let program = build_program(&[("main.sv", NESTED_COERCION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "nested-coercion", "ok 1\nerr a\nok 2\nok 3\nerr b\n")
}

// ===== effect member fns with their own generics =====
// [effect-member-generics] Member generics render on the interface
// member and bind per call from argument types.

const MEMBER_GENERICS_DEMO: &str = r#"
effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T {
        return a
    }
}

fn main() [use] -> None {
    use StdOutConsole
    use FirstStash
    let x = pick(7, 2)
    let s = pick("l", "r")
    println("${x} ${s}")
}
"#;

#[test]
fn effect_member_generics_render_on_the_interface() {
    let program = build_program(&[("main.sv", MEMBER_GENERICS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("fun<T> pick(a: T, b: T): T"),
        "member generics missing from the interface in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_member_generics() -> KotlinCase {
    let program = build_program(&[("main.sv", MEMBER_GENERICS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "member-generics", "7 l\n")
}

// [effect-member-generics] The member's own generics bind per call: the
// checker knows `pick(1, 2)` is `Int`, not an unbound `T`.
#[test]
fn effect_member_generics_bind_per_call() {
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
    let bad: Str = pick(1, 2)
    println(bad)
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("expected `Str`, found `Int`")),
        "unexpected errors: {errors:?}"
    );
}

// ===== effect environment keyed by checker types =====
// [effect-disambiguation] [kt-effect-params] The emitter's effect
// environment is keyed by the checker's lowered effect types
// (`Checked::fn_effects` / `use_effects` / `call_effects`), not by type
// renderings: an effect written through a type alias resolves to the same
// instance a `use` registered under the canonical type.

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
    let program = build_program(&[("main.sv", ALIASED_EFFECT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The `use` registers `Random<Int>`; `roll`'s `Random<Count>`
    // parameter and its call site must agree with it.
    assert!(
        main.content.contains(
            "val random_int: Random<Int> = CyclicRandom<Int>(listOf<Int>(7, 8), { __i0 -> __i0 })"
        ),
        "unexpected use lowering in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("roll(random_int)"),
        "handler not threaded through the aliased effect in:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_aliased_effects() -> KotlinCase {
    let program = build_program(&[("main.sv", ALIASED_EFFECT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "aliased-effects", "7 8\n")
}

// ===== std array functions =====
// [type-array] Arrays get the `core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02 — the LANGUAGE.md `CyclicRandom` example calls
// `values.size()` on a `T[]`).

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
    let program = build_program(&[("main.sv", ARRAY_STD_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    for needle in [
        "nums.size",
        "nums.getOrNull(2)",
        "nums.firstOrNull()",
        "__loop1_pass = iter(nums)",
        "values.size",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

fn kotlinc_compiles_and_runs_array_std() -> KotlinCase {
    let program = build_program(&[("main.sv", ARRAY_STD_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "array-std",
        "size 3 get 5 first 3\niter 3\niter 4\niter 5\nrandom 2 3\n",
    )
}

// ===== N1: dot-names [name-dot] [kt-nested-dot-name] =====

/// A namespace struct with two dot-named members plus a dot-named
/// qualifier driving overload mangling.
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

// [name-dot] [kt-nested-dot-name] Dot-named structs emit as *nested*
// classes (never `inner`) and are referenced with the dotted name;
// a dot-named qualifier canonicalizes to its flat spelling in a mangled
// overload name [kt-qual-mangling].
fn dot_names_emit_nested_classes() -> KotlinCase {
    let program = build_program(&[("main.sv", DOT_NAMES)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted");
    // The namespace class gains a body holding its members, declared
    // under their member segment only.
    assert!(
        main.content.contains("data class Environment(") && main.content.contains(") {\n"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("    data class Id(")
            && main.content.contains("    data class Name("),
        "generated:\n{}",
        main.content
    );
    // Never `inner`: that would need an outer instance to construct.
    assert!(
        !main.content.contains("inner class"),
        "generated:\n{}",
        main.content
    );
    // References keep the dotted spelling — valid Kotlin nested access.
    assert!(
        main.content.contains("val id: Environment.Id"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("Environment.Id(value = \"prod\")"),
        "generated:\n{}",
        main.content
    );
    // Mangling flattens the dot into a single identifier.
    assert!(
        main.content.contains("fun label__EnvironmentTag("),
        "generated:\n{}",
        main.content
    );
    kotlin_case(
        files,
        "dot_names",
        "prod / Production\ntagged t1\nplain t2\n",
    )
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

fn dot_names_in_unions_and_narrowing() -> KotlinCase {
    let program = build_program(&[("main.sv", DOT_NAME_UNIONS)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(
        files,
        "dot_name_unions",
        "id: id\nname: name\nafter\ntagged x\nnone\n",
    )
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
fn provenance_survives_mutation_where_state_does_not() -> KotlinCase {
    let program = build_program(&[("main.sv", QUAL_SUBJECTS)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "qual_subjects", "trusted 3\nplain 3\nchecked 2\n")
}

// ===== E3 step 2: throw and `try` [throw] [try] [kt-throw-signal] =====

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

/// [kt-throw-signal] The JVM's unwinding *is* the propagation: `throw`
/// throws a generated stack-trace-less signal, an intermediate frame does
/// nothing at all (no `ControlFlow`, no colouring), and `try` is Kotlin's
/// own `try`/`catch` *expression*, so the outcome falls out of it. `Throw`
/// is never a handler parameter.
#[test]
fn throw_lowers_to_a_signal_and_try_to_a_catch() {
    let program = build_program(&[("main.sv", THROW_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The signal class is generated once for the program.
    let signal = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("throw.kt"))
        .expect("expected a generated throw.kt");
    assert!(
        signal
            .content
            .contains("class ThrowSignal(val payload: Any?, val tag: String)")
            && signal
                .content
                .contains("RuntimeException(null, null, false, false)"),
        "expected a stack-trace-less signal in:\n{}",
        signal.content
    );
    // [throw] The effect itself emits nothing: there are no handlers to
    // implement, so an interface for it would be dead code.
    let std_throw = files
        .iter()
        .find(|f| f.rel_path.ends_with("core/throw.kt"))
        .map(|f| f.content.clone())
        .unwrap_or_default();
    assert!(
        !std_throw.contains("interface Throw"),
        "expected no interface for the throw effect in:\n{std_throw}"
    );
    // `throw` throws; the message is *not* wrapped at the throw (the
    // throwing frame cannot know which `try` will catch it).
    assert!(
        main.content
            .contains("throw ThrowSignal(\"empty line\", \"Str\")"),
        "expected a tagged throw in:\n{}",
        main.content
    );
    // A propagating frame carries nothing: `parse` is called plainly.
    let measure = main
        .content
        .split("fun measure")
        .nth(1)
        .and_then(|s| s.split("\nfun ").next())
        .unwrap();
    assert!(
        measure.contains("val n = parse(console, line)"),
        "expected plain propagation in:\n{measure}"
    );
    // The delimiter picks the arm at the catch, by tag.
    assert!(
        main.content.contains("catch (__signal: ThrowSignal)")
            && main.content.contains("when (__signal.tag)")
            && main.content.contains("else -> throw __signal"),
        "expected tag dispatch with a rethrow fallback in:\n{}",
        main.content
    );
}

fn kotlin_compiles_and_runs_throw() -> KotlinCase {
    let program = build_program(&[("main.sv", THROW_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "throw", THROW_OUTPUT)
}

// ===== E3 step 3: effects threaded into fn values [fn-effects] =====

/// The program the Rust fusion used to reject: a function *value* that uses
/// an effect, passed to a callee that needs one too. Kotlin always accepted
/// it (objects alias freely), so this is the parity anchor — the same source,
/// the same stdout, now that Rust threads the effect in instead of capturing
/// it.
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

// No effect list of its own: `run_it` *inherits* `[Logger]` from `f`, since
// calling `f` is the only reason it takes it.
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

/// [fn-effects] Kotlin threads the effect as a leading closure parameter
/// too, rather than capturing it — one mechanism on both backends. The
/// fn-value ABI stays per-effect even under the fusion (nominal typing has
/// no blanket impls), and the lambda body opens by combining its effect
/// parameters into the fused carrier [kt-effect-fusion]. A named fn is
/// always adapted: an effectful one gets the carrier built inline, a pure
/// one ignores the threaded effect (the variance rule).
#[test]
fn fn_type_effects_thread_into_lambdas() {
    let program = build_program(&[("main.sv", FN_EFFECTS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The inherited effect fuses `run_it`'s own signature; the fn *type*
    // keeps the per-effect parameter.
    assert!(
        main.content
            .contains("run_it(__fx: __Fx, f: (Logger, String) -> String"),
        "expected the fused signature with a per-effect fn type:\n{}",
        main.content
    );
    // The lambda takes the effect rather than capturing it, and combines
    // it into a fused carrier.
    assert!(
        main.content.contains(": Logger, s ->"),
        "expected the effect as a leading lambda parameter:\n{}",
        main.content
    );
    // Named fns are adapted: the effectful one gets an inline carrier, the
    // pure one drops the threaded effect.
    assert!(
        main.content.contains("shout(__Fx_"),
        "expected the effectful named fn to receive an inline carrier:\n{}",
        main.content
    );
    assert!(
        main.content.contains("plain(__a0)"),
        "expected an adapter for the pure fn:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_fn_type_effects() -> KotlinCase {
    let program = build_program(&[("main.sv", FN_EFFECTS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "fn-effects", FN_EFFECTS_STDOUT)
}

// ===== the widening check `^` [qual-widen] =====

// [qual-widen] `^` tests the arm *and* removes the claim, so a branch can
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

/// [qual-widen] Kotlin materializes the same peel with a shadowing `val`
/// (the alternative, a fresh name, would need every read rewritten).
#[test]
fn widening_materializes_the_peel_kotlin() {
    let program = build_program(&[("main.sv", WIDEN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content
            .contains("val nested = nested.value as Union2<Int, String>"),
        "expected the widened value bound to a shadowing local:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_widening() -> KotlinCase {
    let program = build_program(&[("main.sv", WIDEN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "widen", WIDEN_STDOUT)
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

/// [kt-when-cond] Kotlin has the same construct, so the source shape
/// survives: `cond -> { … }` arms closed by `else -> { … }`. Being total it
/// is an expression without the `else null` filler an `if` chain needs
/// [if-else-none].
#[test]
fn a_subjectless_when_emits_a_subjectless_kotlin_when() {
    let program = build_program(&[("main.sv", WHEN_COND_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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
    assert!(
        classify.contains("return when {") && classify.contains("n < 0 -> {"),
        "expected a subject-less Kotlin `when`:\n{classify}"
    );
    assert!(
        classify.contains("else -> {") && !classify.contains("null"),
        "expected a plain `else` arm and no optional filler:\n{classify}"
    );
    // An `is` head still declares its binding at the top of the arm.
    assert!(
        main.content.contains("val s = value.value as String"),
        "expected the `is` binding inside the arm:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_when_cond() -> KotlinCase {
    let program = build_program(&[("main.sv", WHEN_COND_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "when-cond", WHEN_COND_STDOUT)
}

// ===== a `try` body is ordinary code to the mutability census =====

// [try] A variable assigned *only* inside a `try` body still needs the
// mutable declaration. `collect_mutated_expr` had no `Expr::Try` arm, so it
// emitted `val counter` next to `counter = counter + 1` and kotlinc
// rejected the output — a [backend-never-wrong] miss caught by nothing but
// the toolchain.
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
        risky(3)
    }
    println("counter ${counter}")
}
"#;

#[test]
fn a_variable_mutated_only_inside_a_try_body_is_declared_var() {
    let program = build_program(&[("main.sv", TRY_MUTATION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("var counter = 0"),
        "expected `var` for a variable the `try` body assigns:\n{}",
        main.content
    );
}

fn kotlinc_compiles_and_runs_try_mutation() -> KotlinCase {
    let program = build_program(&[("main.sv", TRY_MUTATION_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "try-mutation", "counter 1\n")
}

// ===== platform effects [platform-effect] =====

/// A platform effect and a `main` that needs it. The host implements
/// `Telemetry` in Kotlin and calls the generated entry point.
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

fn platform_demo_program() -> Program {
    build_program(&[("main.sv", PLATFORM_DEMO)])
}

/// [platform-tree] The host skeleton `salvo platform generate` writes for
/// the demo, as the compiler renders it.
fn platform_skeleton() -> salvo_backend_kotlin::EmittedFile {
    let program = platform_demo_program();
    let mut files = salvo_backend_kotlin::platform_skeletons(&program)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares platform effects");
    files.remove(0)
}

/// Emits the demo with `host` mounted as its platform companion — which is
/// what a source tree with a `platform/` directory produces
/// [platform-tree]. The host is *required*, so every emission test goes
/// through here.
fn generate_platform_demo_with(host: &str) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let mut program = platform_demo_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

fn generate_platform_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_platform_demo_with(&platform_skeleton().content)
}

/// [platform-effect] [kt-platform-entry] A platform effect emits the same
/// `interface` an ordinary effect does — that is the whole point, since the
/// host implements an interface either way — but *no* handler class, and
/// `main` becomes `salvoMain` taking the instance, because the host's own
/// `main` is the program's entry point now.
#[test]
fn platform_effect_emits_an_interface_and_a_host_entry() {
    let files = generate_platform_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt should be generated");
    let src = &main.content;
    assert!(
        src.contains("interface Telemetry {")
            && src.contains("fun record(name: String, value: Int)"),
        "expected the generated interface, got:\n{src}"
    );
    // No handler: the implementation is the host's.
    assert!(
        !src.contains("class Telemetry"),
        "a platform effect must not emit a handler class:\n{src}"
    );
    // The entry point is renamed and takes the instance.
    assert!(
        src.contains("fun salvoMain(telemetry: Telemetry)"),
        "expected the renamed entry point, got:\n{src}"
    );
    assert!(
        !src.contains("fun main("),
        "the host owns `main`, so the generated file must not declare one:\n{src}"
    );
    // Intermediate frames thread it like any other effect.
    assert!(
        src.contains("fun work(telemetry: Telemetry, n: Int): Int"),
        "expected the effect threaded into `work`, got:\n{src}"
    );
}

/// [platform-tree] [kt-platform-host] The generated skeleton: a named class
/// per platform effect implementing the generated interface with every
/// member stubbed, plus the `main` that constructs it and calls the
/// generated entry point. The package is the host's own
/// (`salvo.platform.…`) — sharing the module's would put two `MainKt`
/// facade classes on the classpath.
#[test]
fn platform_generate_renders_a_host_skeleton() {
    let file = platform_skeleton();
    assert_eq!(
        file.rel_path.to_string_lossy(),
        "platform/main.kt",
        "the host file mirrors its module under `platform/`"
    );
    let src = &file.content;
    for expected in [
        "package salvo.platform.main",
        "import salvo.main.*",
        "class TelemetryHost : Telemetry {",
        "override fun record(name: String, value: Int) {",
        "TODO(\"implement Telemetry.record\")",
        "fun main() {\n    salvoMain(TelemetryHost())\n}",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

/// [platform-tree] The host file is not optional: without it the program has
/// no entry point at all, and the error has to name the command that writes
/// one — otherwise the only symptom is `kotlinc` failing to find `main` in
/// generated code, which is what the "loud is not the same as an error"
/// lesson is about.
#[test]
fn a_missing_host_file_names_the_command() {
    let program = platform_demo_program();
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("a platform program without a host must not emit");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("platform/main.kt") && e.contains("salvo platform generate")),
        "expected the missing-host error, got:\n{}",
        errors.join("\n")
    );
}

/// [platform-effect] [platform-tree] The end-to-end shape: the *generated*
/// skeleton with its one stub filled in, compiled and run with kotlinc. Only
/// the `TODO` body is replaced, so the test proves the skeleton is complete
/// and correct everywhere else — package, imports, interface member
/// signature, entry-point call. The asserted stdout is byte-identical to the
/// Rust backend's run of the same program, which is what parity means here.
fn kotlinc_compiles_and_runs_a_platform_effect() -> KotlinCase {
    let skeleton = platform_skeleton();
    let host = skeleton.content.replace(
        "TODO(\"implement Telemetry.record\")",
        "kotlin.io.println(\"[telemetry] $name=$value\")",
    );
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
    let files = generate_platform_demo_with(&host);
    let expected = "[telemetry] work=41\nresult=42\n";
    kotlin_case_with_entry(files, "platform", "salvo.platform.main.MainKt", expected)
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
    use StdOutConsole()
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
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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
        src.contains("interface RawClock {") && src.contains("fun raw_now(): Int"),
        "expected the generated interface, got:\n{src}"
    );
    assert!(
        !src.contains("class HostRawClock"),
        "a platform handler must not emit a class of its own:\n{src}"
    );
    assert!(
        src.contains("salvo.platform.main.HostRawClock(35)"),
        "expected the `use` site to construct the host class, got:\n{src}"
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
        "class HostRawClock(private val offset: Int) : RawClock {",
        "override fun raw_now(): Int {",
        "TODO(\"implement RawClock.raw_now\")",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("fun main()"),
        "a platform handler does not move the entry point:\n{src}"
    );
}

/// [platform-handler] [platform-tree] A `use` of a platform handler with no
/// host file is an error naming the command, not generated code that
/// references a class nobody wrote — the same reasoning as the platform
/// entry point's missing-host error [backend-never-wrong].
#[test]
fn a_missing_host_file_for_a_platform_handler_names_the_command() {
    let program = platform_handler_program();
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("a `use` of a platform handler without a host must not emit");
    assert!(
        errors.iter().any(|e| e.contains("use HostRawClock")
            && e.contains("platform/main.kt")
            && e.contains("salvo platform generate")),
        "expected the missing-host error, got:\n{}",
        errors.join("\n")
    );
}

/// [platform-handler] End to end: the generated skeleton with its one stub
/// filled in, compiled and run by kotlinc. The asserted stdout is
/// byte-identical to the Rust backend's run of the same program.
fn kotlinc_compiles_and_runs_a_platform_handler() -> KotlinCase {
    let skeleton = platform_handler_skeleton();
    let host = skeleton.content.replace(
        "TODO(\"implement RawClock.raw_now\")",
        "return offset + 7",
    );
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
    let files = generate_platform_handler_demo_with(&host);
    kotlin_case(files, "platform-handler", "boot@42\n")
}

// ===== [kt-fn-mangling] overload dispatch is the checker's, not Kotlin's =====

/// Two overloads the *checker* tells apart by Salvo types — a pass and the
/// `List` it walks — where the `List` overload **delegates** to the pass one.
/// That is the shape that turns a second opinion into a crash rather than a
/// wrong answer: before the mangling rule, a delegation whose argument lowered
/// to the same Kotlin type re-resolved to the delegating overload itself.
const OVERLOAD_DELEGATION: &str = r#"
fn twice(p: Mut ListYield<Int>, f: (Int) -> Int) -> Mut List<Int> => p: Mut, f {
    return map(p, f)
}

fn twice(xs: List<Int>, f: (Int) -> Int) -> Mut List<Int> => f, !xs {
    return twice(iter(xs), f)
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdOutConsole()
    let xs = list_of(1, 2, 3)
    for v in xs.twice(double) {
        println("v=${v}")
    }
}
"#;

/// [kt-fn-mangling] [fn-overload] Every emitted overload of a name gets a
/// unique Kotlin name, so Kotlin never re-resolves a call the checker
/// already resolved. Sharing the name let Kotlin apply *its* lattice:
/// `iter(list)` lowers to the identity, so the `List` overload's
/// `twice(xs.iter(), f)` emitted `twice(xs, f)`, whose most specific
/// Kotlin candidate is the `List` overload itself — an infinite recursion,
/// with no diagnostic anywhere [backend-never-wrong].
#[test]
fn every_emitted_overload_gets_its_own_kotlin_name() {
    let program = build_program(&[("main.sv", OVERLOAD_DELEGATION)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    assert!(
        main.contains("fun twice(p: ListYield<Int>") && main.contains("fun twice__2(xs: List<Int>"),
        "expected the second overload to be renamed in:\n{main}"
    );
    // The delegation calls the *other* overload, by its own name.
    assert!(
        main.contains("return twice("),
        "expected the delegation to reach the pass overload in:\n{main}"
    );
}

/// The same program under kotlinc: before the rule it ran until the stack
/// ran out, so the assertion that matters is that it terminates with the
/// right output.
fn kotlinc_compiles_and_runs_overload_delegation() -> KotlinCase {
    let program = build_program(&[("main.sv", OVERLOAD_DELEGATION)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "overload-delegation", "v=2\nv=4\nv=6\n")
}

// ===== [iter-protocol] laziness, now a property of the pass =====

/// [seq-into] The combinator surface — same source and stdout as the Rust
/// backend's `rustc_compiles_and_runs_the_combinator_surface`, which is the
/// parity claim for the eager/into pair. The lazy pair went with the
/// 2026-09-10 removal, so the generic bodies are reached by handing a pass in
/// explicitly.
const SEQ_SURFACE_DEMO: &str = r#"
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

const SEQ_SURFACE_OUTPUT: &str = "eager 4\ngeneric 4\nkept 2\nsink 4\nchained 6\n";

fn kotlinc_compiles_and_runs_the_combinator_surface() -> KotlinCase {
    let program = build_program(&[("main.sv", SEQ_SURFACE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let seq = &files
        .iter()
        .find(|f| f.rel_path.ends_with("seq.kt"))
        .expect("core/seq.kt emitted")
        .content;
    // The generic body drives its subject through the `next` the call site
    // resolved, which arrives as an ordinary function parameter — no trait, no
    // bound [implicit-group].
    assert!(
        seq.contains("next: (It) -> Union2<T, Finished>"),
        "expected the resolved `next` as a plain parameter in:\n{seq}"
    );
    kotlin_case(files, "seq-surface", SEQ_SURFACE_OUTPUT)
}

/// An **unbounded** producer that terminates only because the consumer stops,
/// with the Rust twin's source and stdout. A lazy-chain test until 2026-09-10;
/// the property that mattered is what a plain `for` with a `break` states.
fn kotlinc_compiles_and_runs_a_break_out_of_an_unbounded_producer() -> KotlinCase {
    let program = build_program(&[("main.sv", UNBOUNDED_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "unbounded-break", UNBOUNDED_OUTPUT)
}

const UNBOUNDED_DEMO: &str = r#"
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

const UNBOUNDED_OUTPUT: &str = "v 0\nv 6\nv 12\n";

// ===== [implicit-param] [implicit-group] implicit parameters =====

/// The same source and expected stdout as the Rust backend's
/// `rustc_compiles_and_runs_implicit_parameters`: a group spread with no
/// binder, defaults resolved from the visible `Int` overloads, one member
/// overridden by name, forwarding through an opaque `T`, and an
/// individually declared `?add`.
const IMPLICIT_DEMO: &str = r#"
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
    println("nested=${total_all(list_of(list_of(1, 2), list_of(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
"#;

const IMPLICIT_OUTPUT: &str = "total=6\nproduct=24\nnested=6\npair=42\nlambda=6\n";

/// [implicit-param] Implicit parameters are ordinary trailing parameters of
/// fn type; a resolved default is passed as a Kotlin function reference, and
/// a group leaves no runtime representation [implicit-group].
#[test]
fn implicit_parameters_lower_to_trailing_fn_parameters() {
    let program = build_program(&[("main.sv", IMPLICIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    for expected in [
        "fun<T> total(xs: List<T>, add: (T, T) -> T, zero: () -> T): T {",
        "total(listOf<Int>(1, 2, 3), ::add, ::zero)",
        // `times__2`, not `times`: std's `time` module declares a `times` of
        // its own ([time-types], `times(Duration, Long)`), and overload
        // mangling is program-wide — so a user fn sharing the name takes a
        // suffix whether or not the module is imported [kt-fn-mangling].
        "total(listOf<Int>(2, 3, 4), ::times__2, ::one)",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
    assert!(
        !main.contains("class Field"),
        "a `params` group must leave no runtime representation:\n{main}"
    );
}

/// Under kotlinc, with the stdout the Rust backend asserts byte for byte.
fn kotlinc_compiles_and_runs_implicit_parameters() -> KotlinCase {
    let program = build_program(&[("main.sv", IMPLICIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "implicits", IMPLICIT_OUTPUT)
}

/// [implicit-param] The same source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_effect_member_implicits`: an effect member's
/// implicit parameters reach the interface, every handler's override, and the
/// call site.
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
fn an_effect_member_carries_its_implicits_into_the_interface() {
    let program = build_program(&[("main.sv", MEMBER_IMPLICIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    for expected in [
        "fun show(v: Int, fmt: (Int) -> String): String",
        "override fun show(v: Int, fmt: (Int) -> String): String {",
        "show.show(7, ::fmt)",
        "show.show(7, ::loud)",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
}

fn kotlinc_compiles_and_runs_effect_member_implicits() -> KotlinCase {
    let program = build_program(&[("main.sv", MEMBER_IMPLICIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "member-implicits", MEMBER_IMPLICIT_OUTPUT)
}

// ===== [effect-handler-generics] a `use` gives its handler type arguments =====

/// The same source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_generic_handler`. Kotlin needed the fix too:
/// erasure removes a type argument from the JVM but not from the *source*, so
/// `val h: Show<Int> = Plain()` is a kotlinc error ("cannot infer type for
/// type parameter 'T'") — which the spec used to claim erasure made
/// impossible.
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
    let program = build_program(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    for expected in [
        "val show_int: Show<Int> = Plain<Int>()",
        "val tag_int: Tag<Int> = Prefixed<Int>(\"p\")",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
}

fn kotlinc_compiles_and_runs_a_generic_handler() -> KotlinCase {
    let program = build_program(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "handler-generics", HANDLER_GENERICS_OUTPUT)
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

/// [qual-refn] [qual-erasure] The refined program compiles and runs, and the
/// emitted Kotlin carries no trace of the refinement.
fn kotlinc_compiles_and_runs_a_refined_program() -> KotlinCase {
    let program = build_program(&[("main.sv", REFN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program)
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
    kotlin_case(files, "refn", REFN_EXPECTED)
}

/// [qual-refn-conflict] [diag-structured] A suppressed refinement conflict is
/// a *warning*: the program is legal, so it must still emit — *and* the
/// warning must reach the driver, or the diagnostic exists only in `salvo
/// analyze`. Tested per backend because both the gate and the channel are
/// duplicated in each: the same condition has to fire on both sides.
// `with NonEmpty` on both: std's own `NonEmpty` refines `add` too
// [col-nonempty], so without it the disagreement would be three-way and this
// test would be asserting std's presence rather than the rule. Declaring
// compatibility with std's claim is the one-word remedy a user reaches for —
// it leaves exactly the Q1-vs-Q2 conflict this test is about.
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
    let (files, warnings) = salvo_backend_kotlin::emit_program_reporting(&program)
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

/// Two matching overloads, the generic one declared first. Before O1 the
/// checker picked by declaration order, so this program printed `generic`
/// for `describe(3)` — with correct-looking output on both backends, which
/// is what made it a ranking bug rather than a codegen one.
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

fn kotlinc_runs_the_most_specific_overload() -> KotlinCase {
    let program = build_program(&[("main.sv", OVERLOAD_SPECIFICITY)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "overload-specificity", "concrete\ngeneric\n")
}

// ===== [str-drop-mut] [kt-mut-str] `Mut Str` is a `StringBuilder` =====

/// The whole string surface in one program, shared with the Rust backend's
/// `rustc_compiles_and_runs_strings` — same source, same expected stdout,
/// because a `Mut Str` is a *different type* on this backend and the same
/// one on that, so the parity is the thing worth asserting.
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

/// [kt-mut-str] `Mut Str` maps to `StringBuilder` through the same
/// `mut_type_name` hook `Mut List<T>` uses, and every *drop* of the `Mut`
/// renders the conversion — which is what `MutableList` never needed, since
/// it really is a `List`.
#[test]
fn mut_str_lowers_to_a_string_builder() {
    let program = build_program(&[("main.sv", STRING_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    // Construction is asked for; the parts are joined, which is also what
    // makes a `...spread` work [fn-variadic].
    assert!(
        main.contains(r#"val b = StringBuilder(listOf("he", "llo").joinToString(""))"#),
        "unexpected:\n{main}"
    );
    // [str-drop-mut] The conversion at a call argument, in interpolation,
    // and at an operator — a bare name takes the suffix without parens.
    assert!(main.contains("shout(b.toString())"), "unexpected:\n{main}");
    assert!(
        main.contains("\"size: ${b.toString().length}\""),
        "unexpected:\n{main}"
    );
    assert!(
        main.contains("\"equal: ${x.toString() == y.toString()}\""),
        "unexpected:\n{main}"
    );
    // [kt-copy] A builder's copy is a new builder: identity would alias the
    // buffer.
    assert!(
        main.contains("val dup = StringBuilder(b)"),
        "unexpected:\n{main}"
    );
    // `setCharAt` throws out of range, so `set` guards — and binds its
    // arguments, so a call argument is evaluated once.
    assert!(
        main.contains("if (__i >= 0 && __i < __s.length) __s.setCharAt(__i, 'H')"),
        "unexpected:\n{main}"
    );
}

fn kotlinc_compiles_and_runs_strings() -> KotlinCase {
    let program = build_program(&[("main.sv", STRING_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "strings", STRING_DEMO_OUTPUT)
}

/// [fn-variadic] A `...spread` into a variadic intrinsic uses Kotlin's own
/// spread operator: splicing the array as one argument builds a collection
/// of one array — which kotlinc catches for `listOf`, but *not* for
/// `StringBuilder(...)`, where `append(Any?)` accepts it and prints
/// `[Ljava.lang.String;@…` [backend-never-wrong].
fn a_spread_into_a_variadic_intrinsic_spreads() -> KotlinCase {
    let src = r#"
export fn main() [use] {
    use StdOutConsole()
    let parts = array_of("a", "b")
    let sb = mut_str(...parts)
    let xs = list_of(...parts)
    println("${sb} ${size(xs)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    assert!(
        main.contains(r#"StringBuilder(listOf(*parts).joinToString(""))"#)
            && main.contains("listOf<String>(*parts)"),
        "unexpected:\n{main}"
    );
    kotlin_case(files, "strings-spread", "ab 2\n")
}

/// A `Mut Str` reached through a *parameter* and through a **field**, which
/// is what the generated trait exists for on the Rust side: `set`'s receiver
/// is both read and written, and its lowering has to auto-ref an owned
/// local, a `&mut String` parameter and a field projection alike. Same
/// source and stdout as the Rust backend's
/// `rustc_compiles_and_runs_mut_str_places`.
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

const MUT_STR_PLACES_OUTPUT: &str = "grown 5\nIn-struct!\n";

fn kotlinc_compiles_and_runs_mut_str_places() -> KotlinCase {
    let program = build_program(&[("main.sv", MUT_STR_PLACES)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "mut-str-places", MUT_STR_PLACES_OUTPUT)
}

// ===== [implicit-group] [rs-seq]-equivalent: the sequence functions =====

/// `map`/`filter`/`reduce` over a `List` (the intrinsic fast path), and over
/// the *passes* an array, a `Str`, an origin and a struct of the program's own
/// hand out — `iter` written at each use site [seq-pass]. Shared with the Rust
/// backend's `rustc_compiles_and_runs_sequences`: the *checker* picks the
/// overloads, so the two targets must agree element for element.
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

/// [kt-seq] [seq-pass] A `params` group emits nothing, and the `List` fast
/// paths lower to Kotlin's own collection operations; the generic body is a
/// plain generic function whose implicit parameter arrives as a trailing
/// argument — an *adapter lambda* when the resolved `iter` is an intrinsic,
/// since an intrinsic has no Kotlin name to reference [implicit-intrinsic].
#[test]
fn sequence_functions_lower_to_collection_operations() {
    let program = build_program(&[("main.sv", SEQ_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    assert!(
        main.contains("xs.map({ n -> n * 2 }).toMutableList()")
            && main.contains("xs.filter({ n -> n > 2 }).toMutableList()")
            && main.contains("xs.fold(0, { a, b -> a + b })"),
        "unexpected:\n{main}"
    );
    // The generic overload for a pass subject gets the resolved `next` as its
    // implicit argument.
    assert!(
        main.contains("map(iter(arr), { n -> n + 1 }, ::next)"),
        "expected the resolved `next` as the implicit in:\n{main}"
    );
    // A `params` group is not a value: nothing *declares* `Yield`.
    assert!(
        !files.iter().any(|f| f.content.contains("class Yield")
            || f.content.contains("interface Yield")
            || f.content.contains("object Yield")),
        "a `params` group must emit nothing"
    );
}

fn kotlinc_compiles_and_runs_sequences() -> KotlinCase {
    let program = build_program(&[("main.sv", SEQ_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "sequences", SEQ_DEMO_OUTPUT)
}

// ===== [fn-overload-at] [fn-rename] the caller's two overrides =====

/// A program that overrides overload resolution both ways: a module-level
/// `size` that shadows std's, `@module` to reach past it (and past a local of
/// the same name), and a `rename` that gives one of two unrankable overloads
/// a name of its own. Both are **compile-time only** — the checker records
/// which declaration each call means and mangling keeps the target from
/// re-resolving anything [kt-fn-mangling] — so the point of the pair is that
/// the two backends produce the same answers from the same source.
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

/// [fn-overload-at] [fn-rename] Neither override survives into the output:
/// the calls emit the ordinary (mangled) names of the declarations the
/// checker resolved.
#[test]
fn scope_selectors_and_renames_are_erased() {
    let program = build_program(&[("main.sv", OVERLOAD_OVERRIDE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    // No `@` and no renamed name reaches Kotlin.
    assert!(!main.contains('@'), "unexpected `@` in:\n{main}");
    assert!(
        !main.contains("label_small"),
        "unexpected rename in:\n{main}"
    );
    // `size@core.list(xs)` is std's `size` — the mangled one, since this
    // program declares its own.
    assert!(
        main.contains("println(console, \"core: ${xs.size}\")"),
        "unexpected:\n{main}"
    );
    // The renamed overload is called by its declaration's mangled name —
    // both `label`s are mangled, since they are overloads of one name
    // [kt-fn-mangling].
    assert!(
        main.contains("${label__Even(n)} ${label__Small(n)}"),
        "unexpected:\n{main}"
    );
    // A call reaching past a local of the same name needs nothing special
    // here: Kotlin keeps functions and properties in separate namespaces.
    assert!(
        main.contains("println(console, describe(7))"),
        "unexpected:\n{main}"
    );
}

fn kotlinc_compiles_and_runs_overload_overrides() -> KotlinCase {
    let program = build_program(&[("main.sv", OVERLOAD_OVERRIDE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "overload-overrides", OVERLOAD_OVERRIDE_OUTPUT)
}

// ===== [iter-generic-drive] driving a generic pass =====

/// [iter-drive-in-place] A *kept* pass is advanced where it lives — no local —
/// so the caller's next drive continues from where this one stopped. Kotlin's
/// local aliased the same object and behaved this way already; naming the
/// subject directly is what makes the two backends *say* so identically.
#[test]
fn a_kept_pass_is_driven_in_place() {
    let program = build_program(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains("val __loop1_step = next(it)"),
        "expected the in-place drive in:\n{src}"
    );
    // Precisely: no local for *this* loop. (A substring like `_pass = it` also
    // matches a mint call — `__loop2_pass = iter__4(…)` — so the assertion names
    // the loop.)
    assert!(
        !src.contains("var __loop1_pass"),
        "a kept pass must not be bound into a local in:\n{src}"
    );
}

fn kotlinc_compiles_and_runs_a_generic_drive() -> KotlinCase {
    let program = build_program(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "generic-drive", GENERIC_DRIVE_OUTPUT)
}

/// [kt-suppress-cast] A generic drive reads the union payload through an
/// erased cast (`step.value as T`), which kotlinc reports as an unchecked
/// cast — in code the author cannot edit. The emitted function carries
/// `@Suppress("UNCHECKED_CAST")`; a function whose payload reads cast to
/// concrete types stays unannotated (those casts are checked at run time
/// and draw no warning).
const SUPPRESS_DEMO: &str = r#"
struct Slice<T> : Yield<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj[from: p] T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

// The element type is the combinator's own `T`: the payload read casts to a
// type variable, so the emitted fn needs the suppression.
fn count_all<It, T>(it: Mut It, ?Yield<It, T>) [] -> Int => it: Mut {
    let n = 0
    for x in it {
        n = n + 1
    }
    return n
}

// The element type is concrete (`Int`): the payload read casts to `Int`,
// which the JVM checks at run time — no warning, no annotation.
fn sum_ints<It>(it: Mut It, ?Yield<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdOutConsole()
    let xs = list_of(1, 2, 3)
    let p = slice(xs)
    println("count ${count_all(p)}")
    let ys = list_of(4, 5)
    let q = slice(ys)
    println("sum ${sum_ints(q)}")
}
"#;

#[test]
fn a_payload_cast_gets_the_suppression() {
    let program = build_program(&[("main.sv", SUPPRESS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    // A generic payload is an *unchecked* cast ...
    assert!(
        main.contains(
            "@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\")\nfun<It, T> count_all"
        ),
        "expected the suppression on the generic-element combinator in:\n{main}"
    );
    // ... and a concrete one is a *useless* cast, which kotlinc warns about
    // just as loudly (found 2026-09-14 in std's `fs`, whose `to_str` reads a
    // concrete error arm): one annotation covers both, since neither warning
    // is the author's to silence.
    assert!(
        main.contains(
            "@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\")\nfun<It> sum_ints"
        ),
        "a concrete payload cast is annotated too in:\n{main}"
    );
}

/// [linear-generics] The discharge is the program's own: the pass drives in
/// place and `end(it)` is the explicit terminal — no finally splice (user
/// decision 2026-09-12).
#[test]
fn an_owned_generic_pass_is_discharged_by_the_callback() {
    // [linear-group] No implicit discharge sites (user decision
    // 2026-09-12): no finally splice — `end(it)` is the program's own
    // terminal, an ordinary call through the fn-typed parameter.
    let program = build_program(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        !src.contains("} finally {"),
        "expected no finally splice in:\n{src}"
    );
    assert!(
        src.contains("end(it)"),
        "expected the explicit consuming-callback discharge in:\n{src}"
    );
}

fn kotlinc_compiles_and_runs_a_generic_close() -> KotlinCase {
    let program = build_program(&[("main.sv", GENERIC_CLOSE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "generic-close", GENERIC_CLOSE_OUTPUT)
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

fn kotlinc_compiles_and_runs_the_iter_fn_form() -> KotlinCase {
    let program = build_program(&[("main.sv", ITER_FN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "iter-fn", ITER_FN_OUTPUT)
}

/// [iter-fn] The generated declarations are ordinary Kotlin: a data class for
/// the pass, a function that mints one, and the author's body as a function.
/// Nothing suspends, so there is no `iterator {}` builder and no state number.
#[test]
fn an_iter_fn_emits_a_plain_class_and_next() {
    let program = build_program(&[("main.sv", ITER_FN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    // [iter-fn] Tier 1 — the body never reads the subject, so the pass holds
    // nothing of it.
    assert!(
        src.contains("data class __Pass_Countdown(\n    var at: Int,\n)")
            && src.contains("__Pass_Countdown(at = c.from)"),
        "expected a subject-free pass:\n{src}"
    );
    // Tier 2 — one snapshot field for the one subject field the body reads.
    assert!(
        src.contains("data class __Pass_Fibs(\n    var count: Int,")
            && src.contains("__Pass_Fibs(count = f.count, a = 0, b = 1, made = 0)")
            && !src.contains("var __subject: Fibs"),
        "expected a per-field snapshot:\n{src}"
    );
    // Tier 3 — the whole subject is handed on, so the pass holds it.
    assert!(
        src.contains("data class __Pass_Row(\n    var __subject: Row,")
            && src.contains("__Pass_Row(__subject = r, left = r.times)"),
        "expected the whole subject to be kept:\n{src}"
    );
    assert!(
        !src.contains("__advance") && !src.contains("iterator {"),
        "an `iter fn` needs no state machine:\n{src}"
    );
    // [fn-effects] The effectful `next` gets its handler per turn. The mangling
    // index counts the visible `next` overloads, so it moved when std's lazy
    // pair (two of them) was removed 2026-09-10, again when `Set` and `Map`
    // brought their own passes (two more) 2026-09-13, and again when `Bytes`
    // and the fs chunk pass brought two more 2026-09-15.
    assert!(
        src.contains("next__11(console, __loop"),
        "expected the handler threaded into the drive:\n{src}"
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

fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>) -> Int =>[iter] c, proj[from: c] => c {
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
    println("a list: ${total(list_of(1, 2, 3))}")
}
"#;

const CONTAINER_IMPLICIT_OUTPUT: &str = "generated pass: 6\nwritten iter: 9\na list: 6\n";

fn kotlinc_compiles_and_runs_a_container_combinator() -> KotlinCase {
    let program = build_program(&[("main.sv", CONTAINER_IMPLICIT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "container-implicit", CONTAINER_IMPLICIT_OUTPUT)
}

/// [fate-field-disjoint] The Kotlin half of L5: the same source and stdout as
/// the Rust backend's `rustc_compiles_and_runs_field_disjoint_access`. Nothing
/// changes in this emitter — every field read already aliased on the JVM — so
/// this is the parity check that the newly *legal* programs behave identically
/// on both backends.
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

fn kotlinc_compiles_and_runs_field_disjoint_access() -> KotlinCase {
    let program = build_program(&[("main.sv", FIELD_DISJOINT_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "field-disjoint", FIELD_DISJOINT_OUTPUT)
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

fn kotlinc_compiles_and_runs_a_partial_move() -> KotlinCase {
    let program = build_program(&[("main.sv", PARTIAL_MOVE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "partial-move", PARTIAL_MOVE_OUTPUT)
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

fn kotlinc_compiles_and_runs_inc_dec() -> KotlinCase {
    let program = build_program(&[("main.sv", INC_DEC_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "inc-dec", INC_DEC_OUTPUT)
}

/// [iter-fn] [proj-field] [yield-proj] An `iter fn` over a *generic* subject
/// that emits borrowed elements. The generated pass **borrows** its subject
/// (`__subject: proj Box<T>`, user decision 2026-09-11) instead of copying it,
/// so nothing has to `copy` a value of type `T` — which is what used to refuse
/// generic subjects on the Kotlin backend [kt-copy]. `proj[from: b]` in the
/// written return names the subject; the desugar redirects it to the pass.
const GENERIC_ITER_FN_DEMO: &str = r#"
struct Box<T> {
    items: List<T>
}

iter fn next<T>(b: Box<T>) -> Emitted (proj[from: b] T) | Finished {
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

fn kotlinc_compiles_and_runs_a_generic_subject_iter_fn() -> KotlinCase {
    let program = build_program(&[("main.sv", GENERIC_ITER_FN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "generic-iter-fn", GENERIC_ITER_FN_OUTPUT)
}

/// [deduce-syntax] [proj-anywhere] The `=>` clause's projection forms end to
/// end: a wholesale `proj[from: a, b]` joined across branches, and a
/// re-pointing entry `v.items: proj[from: other]` that makes a view borrow a
/// different list — Rust ties the struct's lifetime to the new source.
const DEDUCTION_CLAUSE_DEMO: &str = r#"
struct View canbe Mut {
    items: proj List<Int>,
    at: Int
}

fn view(items: List<Int>) -> Mut View {
    return Mut View { items: items, at: 0 }
}

fn repoint(v: Mut View, other: List<Int>) -> None => v: Mut, v.items: proj[from: other] {
    v.items = other
    v.at = 0
}

fn either(a: List<Int>, b: List<Int>, flag: Bool) -> proj[from: a, b] List<Int> {
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

fn kotlinc_compiles_and_runs_the_deduction_clause_projections() -> KotlinCase {
    let program = build_program(&[("main.sv", DEDUCTION_CLAUSE_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "deduction-clause", DEDUCTION_CLAUSE_OUTPUT)
}

// ===== [proj-type] a borrowed union arm into a `proj`-typed parameter =====

/// The phase-2b cut, closed 2026-09-12: a pass's `next` returns
/// `Emitted (proj Str) | Finished` — a union whose payload arm *borrows* —
/// and a fn takes that union whole by writing the projection in its
/// parameter type. On the JVM the projection erases; the point here is
/// byte-identical stdout with the Rust backend.
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

fn kotlinc_compiles_and_runs_a_borrowed_union_arm_into_a_proj_parameter() -> KotlinCase {
    let program = build_program(&[("main.sv", PROJ_ARM_PARAM_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "proj-arm-param", PROJ_ARM_PARAM_OUTPUT)
}

// ===== [lambda-view] a capture-rooted projection =====

/// The Kotlin half of the capture-rooted projection case: a callback
/// picking elements out of a captured list. On the JVM a capture is an
/// alias and the projection erases; what this pins is byte-identical
/// stdout with the Rust backend, which holds real borrows.
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

fn kotlinc_compiles_and_runs_a_capture_rooted_projection() -> KotlinCase {
    let program = build_program(&[("main.sv", PICK_LIST_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "pick-list", PICK_LIST_OUTPUT)
}

// ===== [linear-union-arm] the fallible-open shape =====

/// The Kotlin half of O-C2: linearity is checker-only, so the union with a
/// linear arm lowers like any other — what this pins is byte-identical
/// stdout with the Rust backend [linear-static].
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

fn kotlinc_compiles_and_runs_a_linear_union_arm() -> KotlinCase {
    let program = build_program(&[("main.sv", FALLIBLE_OPEN_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "linear-union-arm", FALLIBLE_OPEN_OUTPUT)
}

// ===== [linear-group] [linear-composite] the wrapper pass =====

/// The Kotlin half of acceptance shape 4: byte-identical stdout with the
/// Rust backend for the lazy `take` over a linear source.
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

fn kotlinc_compiles_and_runs_a_linear_wrapper_pass() -> KotlinCase {
    let program = build_program(&[("main.sv", WRAPPER_PASS_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "wrapper-pass", WRAPPER_PASS_OUTPUT)
}

// ===== [fn-value-select] [linear-discard] `drop` as the uniform callback =====

/// The Kotlin half: byte-identical stdout with the Rust backend for the
/// consuming-callback pattern's two callers.
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

fn kotlinc_compiles_and_runs_drop_as_a_consuming_callback() -> KotlinCase {
    let program = build_program(&[("main.sv", DROP_CALLBACK_DEMO)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    kotlin_case(files, "drop-callback", DROP_CALLBACK_OUTPUT)
}

// ===== member overloading within one effect [effect-member-overload] =====

/// [effect-member-overload] Verbatim the Rust backend's `MEMBER_OVERLOADS`:
/// one effect declaring `close` per token type, which is phase 4's `Fs`
/// (FILE_SYSTEM.md §5.10.2 sub-question A). Kotlin *could* overload here, and
/// that is the hazard — it would resolve by Kotlin's type lattice rather than
/// Salvo's [kt-fn-mangling] — so the names are made distinct by the same
/// `salvo_core` rule the Rust backend uses.
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

/// [effect-member-overload] The interface declares the overloads under
/// distinct names (the first keeps the plain one), the handler overrides those
/// same names, and each call site emits the one the *checker* resolved.
#[test]
fn effect_member_overloads_get_distinct_names() {
    let files = generate_files(&[("main.sv", MEMBER_OVERLOADS)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let src = &main.content;
    for expected in [
        "fun close(f: InFile): String",
        "fun close__2(f: OutFile): String",
        "fun describe(f: InFile): String",
        "override fun close(f: InFile): String {",
        "override fun close__2(f: OutFile): String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        src.contains(".close(InFile(") && src.contains(".close__2(OutFile("),
        "expected both call sites to name their own overload, got:\n{src}"
    );
}

/// [effect-member-overload] End to end: which overload runs is the checker's
/// answer, and kotlinc must have no opinion. Byte-identical stdout on Rust.
fn kotlinc_compiles_and_runs_member_overloads() -> KotlinCase {
    let files = generate_files(&[("main.sv", MEMBER_OVERLOADS)]);
    kotlin_case(files, "member-overloads", MEMBER_OVERLOADS_OUTPUT)
}

// ===== member parameter modes [rs-borrows]'s Kotlin twin =====

/// Verbatim the Rust backend's `MEMBER_MODES`: a consuming effect member and
/// a mutating one, forwarded through a dependent handler. Kotlin has no
/// borrow modes, so nothing in the *signatures* changes here — the case earns
/// its place by asserting the two backends still agree on what the program
/// prints, which is what the Rust mode rule must not disturb.
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

handler Doubling [Sink] of Sink {
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
    use Doubling()
    let t: Mut Token = Mut Token { id: 1 }
    bump(t)
    println("bumped ${t.id}")
    println("took ${take(t)}")
}
"#;

fn kotlinc_compiles_and_runs_member_modes() -> KotlinCase {
    let files = generate_files(&[("main.sv", MEMBER_MODES)]);
    kotlin_case(files, "member-modes", "bumped 3\ntook 6\n")
}

// ===== linear tokens discharged by effect members [linear-group] =====

/// Verbatim the Rust backend's `LINEAR_MEMBER_DISCHARGE`: a linear token whose
/// only discharger is an overloaded effect member, discharged inside the
/// handler that implements it. Linearity is erased on the JVM, so what this
/// asserts is parity — the same program printing the same lines.
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

fn kotlinc_compiles_and_runs_a_linear_token_closed_by_a_member() -> KotlinCase {
    let files = generate_files(&[("main.sv", LINEAR_MEMBER_DISCHARGE)]);
    kotlin_case(files, "linear-member-discharge", "line 9\nclosed in\nwrote 5\nclosed out\n")
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
fn kotlinc_compiles_and_runs_bytes() -> KotlinCase {
    let files = generate_files(&[("main.sv", BYTES_PROGRAM)]);
    kotlin_case(files, "bytes", BYTES_OUTPUT)
}

/// [kt-bytes] The buffer is a **shipped class**, emitted once per program that
/// names a `Bytes` — and `Mut Bytes` is the same class, so dropping the `Mut`
/// renders nothing and no conversion happens at a call boundary.
#[test]
fn bytes_ships_one_runtime_class_for_both_shapes() {
    let files = generate_files(&[("main.sv", BYTES_PROGRAM)]);
    let runtime = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("bytes.kt"))
        .expect("bytes.kt emitted");
    for expected in [
        "class SalvoBytes",
        "override fun equals(",
        "operator fun iterator(): Iterator<UByte>",
    ] {
        assert!(
            runtime.content.contains(expected),
            "expected `{expected}` in the buffer runtime:\n{}",
            runtime.content
        );
    }
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt emitted");
    // One class for both shapes: the builder's declared type is the same as a
    // fixed buffer's, and `str_of_bytes(buf)` needs no `.toString()`-style
    // conversion the way a `Mut Str` does [str-drop-mut].
    assert!(
        main.content.contains("val buf = salvo.SalvoBytes.joined()"),
        "expected the builder built by the same class:\n{}",
        main.content
    );
    // A `for` over a buffer is Kotlin's own loop [kt-iter-native]: the class
    // has an `iterator()`, so iterating allocates no pass.
    assert!(
        main.content.contains("for (byte in fixed)"),
        "expected the native byte loop:\n{}",
        main.content
    );
    assert!(
        main.content.contains("buf.asString()"),
        "expected the read surface reached without a conversion:\n{}",
        main.content
    );
}

// ===== std's filesystem [platform-handler] [linear-group] =====

/// std's `core.fs`, exercised end to end against real files — **verbatim the
/// Rust backend's program**, because the layering's promise is that only the
/// host file differs: `platform handler HostRawFs` at the bottom,
/// `DefaultFs [RawFs]` above it (a handler in *std* constructed from the
/// program, which is what made the per-file carrier class a defect), linear
/// tokens discharged by an effect member, the `Lines` pass, the one-shots, a
/// `position` after a ranged open, and a failure path acknowledged once.
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

/// [kt-platform-handler] [use-local] [effect-handler-deps] What the layering
/// emits: the effect interfaces, no class for the platform handler, and
/// `DefaultFs` capturing its dependency as an **owned handle at
/// construction** — a constructor parameter typed as the dep's Has-accessor
/// interface, whose per-effect identity is what lets a std handler be
/// constructed from a program (per-file `__Fx_N` classes could not).
#[test]
fn the_fs_surface_emits_a_host_seam_and_a_generic_carrier() {
    let files = generate_files(&[("main.sv", &fs_program())]);
    let surface = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/fs.kt"))
        .expect("core/fs.kt")
        .content;
    assert!(
        surface.contains("interface Fs {"),
        "expected the effect interface in:\n{surface}"
    );
    // [effect-available] One overload set: the pass's discharger is a *fn*
    // named `close`, beside the two members of that name.
    assert!(
        surface.contains("fun<__Fx> close(__fx: __Fx, p: Lines)"),
        "expected the pass discharger as a fn named `close` in:\n{surface}"
    );
    // The host seam is its own module [mod-used-only]: a program that never
    // opens a file links none of it.
    let host = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("core/hostfs.kt"))
        .expect("core/hostfs.kt")
        .content;
    for expected in [
        "interface RawFs {",
        "fun raw_close_read(handle: Long)",
        "class DefaultFs(private val __dep_RawFs: __Has_RawFs) : Fs",
    ] {
        assert!(host.contains(expected), "expected `{expected}` in:\n{host}");
    }
    assert!(
        !host.contains("class HostRawFs"),
        "the platform handler's class is the host's:\n{host}"
    );
    // The `use` site constructs the shipped host class through its package.
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt");
    assert!(
        main.content
            .contains("salvo.platform.core.hostfs.HostRawFs()"),
        "expected the shipped host class at the `use` site, got:\n{}",
        main.content
    );
    // std ships the host file, and it travels into the output.
    assert!(
        files
            .iter()
            .any(|f| f.rel_path == std::path::Path::new("platform/core/hostfs.kt")),
        "expected std's host companion to be emitted"
    );
}

fn kotlinc_compiles_and_runs_the_fs_surface() -> KotlinCase {
    let files = generate_files(&[("main.sv", &fs_program())]);
    kotlin_case(files, "fs-surface", FS_OUTPUT)
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

fn kotlinc_compiles_and_runs_the_memory_filesystem() -> KotlinCase {
    let files = generate_files(&[("main.sv", MEMFS_PROGRAM)]);
    kotlin_case(files, "memfs", MEMFS_OUTPUT)
}

// ===== [effect-available] a member name that is also a std fn =====

/// The same program the Rust backend runs, with the same expected output. The
/// two bugs it pins were both *emitter*-side and identical on both backends:
/// std's own `core/seq.sv` emitting a member dispatch for its `add(out, x)`
/// (loud — "no handler for effect `Tally`" from inside std), and
/// `to_upper@core.string("hi")` running the **member** and printing `hi!` where
/// `HI` was asked for (silent). Running it is what catches the second.
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
    add(4)
    add(5)
    println("tally ${read()}")
    let xs: Mut List<Int> = mut_list_of()
    add@core.list(xs, 7)
    let mapped = map(iter([1, 2, 3]), double)
    println("list ${size(xs)} mapped ${size(mapped)}")
    let mine = to_upper@Shout("hi")
    let theirs = to_upper@core.string("hi")
    println("member ${mine} std ${theirs}")
}
"#;

fn kotlinc_compiles_and_runs_a_member_named_like_a_std_fn() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", MEMBER_NAME_COLLISION)]),
        "member-name-collision",
        "tally 9\nlist 1 mapped 3\nmember hi! std HI\n",
    )
}

// ===== [kt-actor] asynchronous effect handlers =====

/// [actor-spawn-expr] [actor-use-addr] [actor-waitfor] The same program the
/// Rust backend runs, with **the same expected output** — the parity assertion
/// for the surface, not just for the scheduler library underneath it.
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

fn generate_actor_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", ACTOR)])
}

fn kotlinc_compiles_and_runs_an_actor() -> KotlinCase {
    kotlin_case(generate_actor_demo(), "actor", "sum 5\n")
}

/// [kt-actor] What the lowering *is*, asserted on the generated text: a
/// sealed message class per protocol, an actor class wrapping the handler,
/// and the scheduler file carried into the output.
#[test]
fn an_actor_lowers_to_message_classes_and_a_body() {
    let files = generate_actor_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains("sealed class __Msg_Counter {"),
        "the protocol's message class is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor"),
        "the actor class is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("salvo.SalvoSched.spawn("),
        "the spawn is missing:\n{}",
        main.content
    );
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "scheduler.kt"),
        "the scheduler file is not part of the program"
    );
}

/// [monitor-handler] [kt-monitor] The monitor spawn (SH-3, user decision
/// 2026-09-19): the same program the Rust backend runs, with the same
/// expected output — one plain-effect handler shared behind a lock, serving
/// `main` and a spawned actor.
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

fn generate_monitor_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", MONITOR)])
}

fn kotlinc_compiles_and_runs_a_monitor() -> KotlinCase {
    kotlin_case(
        generate_monitor_demo(),
        "monitor",
        "main drew 12345\nactor drew 95040\nmain drew 58585\n",
    )
}

/// [kt-monitor] What the monitor lowering *is*, asserted on the generated
/// text: the per-effect lock wrapper implementing the effect's interface by
/// synchronizing and delegating, the spawn wrapping the handler instance,
/// and no send stub for a plain effect.
#[test]
fn a_monitor_lowers_to_a_lock_wrapper() {
    let files = generate_monitor_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content
            .contains("class __Mon_Random(private val inner: Random) : Random {"),
        "the lock wrapper is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("synchronized(inner) { inner.next() }"),
        "a member does not lock and delegate:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Mon_Random(CyclicRandom(12345))"),
        "the monitor spawn is missing:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("__Stub_Random"),
        "a plain effect must not get a send stub:\n{}",
        main.content
    );
}

/// [mixed-handler] [kt-mixed] The mixed handler (SH-1): the same program the
/// Rust backend runs, with the same expected output — the servant's state
/// behind a mailbox, the façade on the callers' threads, one instance
/// serving `main` and a spawned actor.
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

/// [defer-deduction] [mixed-handler] The same rung-4 forwarding program the
/// Rust backend runs, with the same output.
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

fn kotlinc_compiles_and_runs_a_deferring_mixed_handler() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", DEFERRING)]),
        "deferring",
        "first 7\nsecond 14\n",
    )
}

/// [defer-deduction] [actor-replyto] [kt-mixed] Parking inside a mixed
/// handler — the full rung-4 shape: the servant captures the caller's reply
/// in a continuation on its own `settled` member (`__Cont_H`, handler-keyed),
/// consults another actor, and answers when the resume delivers the oracle's
/// number one activation later. Same program and output as the Rust backend.
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

fn kotlinc_compiles_and_runs_a_parking_mixed_handler() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", PARKING)]),
        "parking-mixed",
        "first 8\nsecond 16\n",
    )
}

/// [mixed-handler] [actor-self-send] [kt-mixed] A servant reaching its
/// siblings (user decision 2026-09-19): a bare sibling call in a send member
/// and `k@self(…)` in both member kinds, all lowering to enqueues on the
/// servant's own mailbox (`__Msg_H`), the reply travelling hop to hop by
/// value. Same program and output as the Rust backend.
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

fn kotlinc_compiles_and_runs_a_servant_sibling_chain() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", SERVANT_CHAIN)]),
        "servant-chain",
        "first 10\nsecond 20\n",
    )
}

fn generate_mixed_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", MIXED)])
}

fn kotlinc_compiles_and_runs_a_mixed_handler() -> KotlinCase {
    kotlin_case(
        generate_mixed_demo(),
        "mixed",
        "main drew 12345\nactor drew 95040\nmain drew 58585\n",
    )
}

/// [kt-mixed] What the mixed lowering *is*: the handler-keyed message class
/// and actor body (the servant), the façade class carrying the addr and ctor
/// params, the façade send, and the spawn answering the façade.
#[test]
fn a_mixed_handler_lowers_to_a_servant_and_a_facade() {
    let files = generate_mixed_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains("sealed class __Msg_CyclicRandom {"),
        "the servant's message class is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("class __Actor_CyclicRandom(private val handler: CyclicRandom) : salvo.SalvoActor"),
        "the servant's actor body is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("class __Fac_CyclicRandom(private val __addr: Int, private val seed: Int) : Random {"),
        "the façade class is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("salvo.SalvoSched.send(__addr, __Msg_CyclicRandom.Advance("),
        "the façade send is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Fac_CyclicRandom(__a, __c0)"),
        "the mixed spawn does not answer the façade:\n{}",
        main.content
    );
}

// ===== [actor-replyto] [actor-self-send] parked continuations =====

/// The same three programs the Rust backend runs, with the same expected
/// output. The first is the shape the slice exists for — **`main` is not in the
/// loop**: the database replies to the *fetcher*, whose parked continuation then
/// fulfils `main`'s token. The caller's token travels as a continuation
/// *capture*, which is what makes it writable before linearity-in-collections.
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

/// `replyto!` — the gate. While gated the actor serves only the awaited
/// reply, so the already-queued `note("late")` waits: `reply R, user late`
/// rather than the other way round.
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

/// `k@self(args)` and both its readings, answering the same thing from each.
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

fn generate_replyto_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", REPLYTO_CHAIN)])
}

fn kotlinc_compiles_and_runs_a_parked_continuation() -> KotlinCase {
    kotlin_case(generate_replyto_demo(), "replyto-chain", "got row 7\n")
}

fn kotlinc_compiles_and_runs_a_gated_continuation() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", REPLYTO_GATE)]),
        "replyto-gate",
        "reply R, user late\n",
    )
}

fn kotlinc_compiles_and_runs_both_readings_of_a_self_send() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", SELF_SEND)]),
        "self-send",
        "spawned begin 1, again 2\ninline begin 1, again 2\n",
    )
}

/// [actor-watch] The monitor surface, end to end. Source and expected stdout
/// are **verbatim** the Rust backend's `rustc_compiles_and_runs_a_death_watch`:
/// an actor faults, the scheduler answers the watcher's token with an `Exit`,
/// a watch registered *after* the death answers immediately, and a send to the
/// corpse is a silent no-op.
///
/// The reason's *text* is deliberately not printed: it is the host's account of
/// the fault (an exception's message here, a panic message on the Rust
/// backend), the one thing on this surface that is not identical across
/// backends [actor-watch].
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

/// [linear-container] [linear-state] **Obligations in a collection**, end to
/// end. Source and expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_obligations_in_a_collection`: an actor parks reply
/// tokens in `Mut List<Reply<Str>>` state, answers one with `remove_first`, and
/// `drain`s the rest on shutdown, putting a fresh list back.
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

fn kotlinc_compiles_and_runs_obligations_in_a_collection() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", LINEAR_QUEUE)]),
        "linear-queue",
        "second closed: end of day\nfirst served ada\n",
    )
}

/// [kt-actor] [linear-container] The lowering: Kotlin needs no `mem::take`
/// equivalent — objects are references, and the checker made the member assign
/// a fresh list before it returned — so a drain is `forEach` over a snapshot.
#[test]
fn draining_state_lowers_to_a_foreach_kotlin() {
    let files = generate_files(&[("main.sv", LINEAR_QUEUE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains("(waiting).toList().forEach("),
        "the drain is not a forEach over a snapshot:\n{}",
        main.content
    );
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

fn kotlinc_compiles_and_runs_the_waitfor_package() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", WAITFOR_PACKAGE)]),
        "waitfor-package",
        WAITFOR_PACKAGE_OUTPUT,
    )
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

fn kotlinc_compiles_and_runs_the_task_kernel() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", TASK_KERNEL)]),
        "task-kernel",
        TASK_KERNEL_OUTPUT,
    )
}

/// [kt-task] The lowering, mirroring the Rust backend: the lambda is the
/// continuation, and no `__Cont_` class is generated for a task mint.
#[test]
fn a_task_mint_lowers_to_a_scheduled_lambda() {
    let files = generate_files(&[("main.sv", TASK_KERNEL)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool())"),
        "an omitted `on` clause must inherit the current pool:\n{text}"
    );
    assert!(
        text.contains("{ __v -> finish("),
        "the continuation must be a lambda:\n{text}"
    );
    assert!(
        !text.contains("__parked["),
        "a task mint parks nothing: the lambda is the continuation:\n{text}"
    );
}

fn kotlinc_compiles_and_runs_a_death_watch() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", WATCH)]),
        "watch",
        "died with a reason: true\nlate watch answered: true\ndone\n",
    )
}

/// [effect-handler-multi] Handlers of several effects, end to end. Source and
/// expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_a_handler_of_several_effects`: one actor with a
/// public `Timer` face and a `TimerCtl` admin face over one mailbox, whose
/// `spawn` answers one addr per face, beside the synchronous pair one `use`
/// binds.
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

fn kotlinc_compiles_and_runs_a_handler_of_several_effects() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", MULTI_FACE)]),
        "multi-face",
        "pending 0\nfired at 10\npending 0\ntotal 5\n",
    )
}

/// [effect-handler-multi] [kt-actor] What the two faces lower to, mirroring the
/// Rust backend: one class implementing both interfaces, one dispatcher per
/// protocol, a `handle` that asks which protocol the message belongs to, and a
/// spawn whose value is a `Pair` of the same scheduler id.
#[test]
fn several_faces_lower_to_one_actor_with_a_dispatcher_each_kotlin() {
    let files = generate_files(&[("main.sv", MULTI_FACE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("class ManualTime : Timer, TimerCtl {"),
        "the class must implement both interfaces:\n{text}"
    );
    assert!(
        text.contains("private fun __dispatchTimer(m: __Msg_Timer)")
            && text.contains("private fun __dispatchTimerCtl(m: __Msg_TimerCtl)"),
        "one dispatcher per protocol is missing:\n{text}"
    );
    assert!(
        text.contains("is __Msg_Timer -> __dispatchTimer(msg)")
            && text.contains("is __Msg_TimerCtl -> __dispatchTimerCtl(msg)"),
        "the delivery must ask which protocol the message is:\n{text}"
    );
    assert!(
        text.contains("val __a = salvo.SalvoSched.spawn(") && text.contains("Pair(__a, __a)"),
        "the spawn must answer one addr per face:\n{text}"
    );
    assert!(
        text.contains("class Counting : Tally, Stats {"),
        "the synchronous handler's faces are missing:\n{text}"
    );
}

/// [actor-on-idle] The quiescence hook, end to end. Source and expected stdout
/// are **verbatim** the Rust backend's `rustc_compiles_and_runs_a_quiescence_hook`:
/// the program hears about the scheduler running dry twice, once settled and
/// once with an actor gated on an answer another actor has parked and will never
/// send, and the counts are what tell the two apart.
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

fn kotlinc_compiles_and_runs_a_quiescence_hook() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", ON_IDLE)]),
        "on-idle",
        "settled: gates 0, tokens 0\nstuck: gates 1, tokens 1\n",
    )
}

/// [actor-on-idle] [kt-actor] What `on_idle` lowers to, mirroring the Rust
/// backend: the scheduler call **plus the `Idle` builder** the registration site
/// closes over, since the runtime holds two counts and cannot construct a Salvo
/// class.
#[test]
fn a_quiescence_hook_lowers_to_a_scheduler_call_with_an_idle_builder_kotlin() {
    let files = generate_files(&[("main.sv", ON_IDLE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains(
            "salvo.SalvoSched.onIdle(p, i, { __gates, __tokens -> Idle(__gates, __tokens) })"
        ),
        "the idle registration or its `Idle` builder is missing:\n{}",
        main.content
    );
}

/// [actor-watch] [kt-actor] What a `watch` lowers to, mirroring the Rust
/// backend: the scheduler call **plus the `Exit` builder** the watch site
/// closes over, since the runtime cannot construct a Salvo class.
#[test]
fn a_watch_lowers_to_a_scheduler_call_with_an_exit_builder_kotlin() {
    let files = generate_files(&[("main.sv", WATCH)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content
            .contains("salvo.SalvoSched.watch(c, out, { __reason -> Exit(__reason) })"),
        "the watch registration or its `Exit` builder is missing:\n{}",
        main.content
    );
}

/// [kt-actor] [actor-replyto] The lowering, mirroring the Rust backend's: a
/// continuation class beside the message class, the two generated fields on the
/// handler — **not** `private`, since `__Actor_H` is a different class and
/// Kotlin's class-level `private` does not reach across one — a `__dispatch`
/// factored out of `handle`, and a `resume` that pops the slot and casts the
/// answer to the target member's trailing parameter type.
#[test]
fn a_parked_continuation_lowers_to_a_slot_table_kotlin() {
    let files = generate_replyto_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("sealed class __Cont_Fetching {")
            && text.contains("class Arrived(val out: salvo.SalvoReply) : __Cont_Fetching()"),
        "the continuation class is missing, or its captures are wrong:\n{text}"
    );
    assert!(
        text.contains("internal var __addr: Int? = null")
            && text.contains(
                "internal val __parked: MutableMap<Long, __Cont_Fetching> = mutableMapOf()"
            ),
        "the handler's generated fields are missing:\n{text}"
    );
    assert!(
        text.contains("handler.__addr = ctx.addr"),
        "the activation does not write its own address:\n{text}"
    );
    assert!(
        text.contains("salvo.SalvoSched.mint(__addr!!)") && text.contains("__parked[__s] ="),
        "the mint does not park a continuation:\n{text}"
    );
    assert!(
        text.contains("val c = handler.__parked.remove(slot) ?: return")
            && text.contains("value as String"),
        "`resume` does not dispatch the parked continuation:\n{text}"
    );
}

// ===== [actor-use-addr] the forwarding stub =====

/// The same program the Rust backend runs, with the same expected output.
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

fn generate_addr_stub_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", ADDR_STUB)])
}

fn kotlinc_compiles_and_runs_a_stub_bound_effect() -> KotlinCase {
    kotlin_case(generate_addr_stub_demo(), "addr-stub", "noted 2\n")
}

#[test]
fn a_stub_implements_the_effect_by_sending_kotlin() {
    let files = generate_addr_stub_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains("class __Stub_Log(private val addr: Int) : Log"),
        "the forwarding stub is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Stub_Log("),
        "`use addr` does not build the stub:\n{}",
        main.content
    );
}

// ===== [actor-spawn-expr] dependent-handler spawns =====

/// The same program the Rust backend runs, with the same expected output: a
/// child declaring `[Log, Tally]` whose spawn clause supplies one dependency as
/// a **construction** and one as an **`Addr`**. The parity assertion for the
/// binding swap — one handler, compiled once, with a local instance behind one
/// of its effects and an actor behind the other.
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

fn generate_dep_spawn_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", DEP_SPAWN)])
}

fn kotlinc_compiles_and_runs_a_dependent_spawn() -> KotlinCase {
    kotlin_case(
        generate_dep_spawn_demo(),
        "dep-spawn",
        "last bumped 3\nsum 5\n",
    )
}

/// [kt-actor] [kt-effect-fusion] The shape: the actor is generic in the
/// **same carrier** the handler stores, and the spawn site builds one of the
/// generated `__Fx_N` classes out of its clause — a construction for one
/// dependency, a forwarding stub for the addr. Kotlin needs no provider of its
/// own, because the handler already holds the carrier.
#[test]
fn a_dependent_spawn_builds_the_childs_carrier() {
    let files = generate_dep_spawn_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains(
            "class __Actor_Counting<__Fx>(private val handler: Counting<__Fx>) : \
             salvo.SalvoActor where __Fx : __Has_Log, __Fx : __Has_Tally"
        ),
        "the actor class does not carry the handler's carrier:\n{text}"
    );
    // [actor-mailbox] The instance is built into a local first, so the spawn can
    // read the bound off it before handing it over.
    assert!(
        text.contains("val __h = Counting(__Fx_2(Recording(), __Stub_Tally(tally)))")
            && text.contains("__h.__mailboxCapacity, __Actor_Counting(__h)"),
        "the spawn does not build the carrier from its clause, or does not read the \
         handler's mailbox:\n{text}"
    );
}


/// [is-bind-once] The defect this rule closed, as a program — source and
/// expected stdout **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_is_bindings_over_calls`: an `is` binding whose
/// subject is a call must evaluate it exactly once, in all four emitter paths
/// (`while` and `if`, statement and value position). Until 2026-09-16 both
/// backends emitted the subject twice and silently dropped every other element.
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

fn kotlinc_compiles_and_runs_is_bindings_over_calls() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", IS_BIND_ONCE)]),
        "is-bind-once",
        "  took 1\n  took 2\n  took 3\nleft 0\nfirst 7 then 1\nsum 10 left 0\none 5 left 1\n",
    )
}

/// [is-bind-once] …and the shape: one `val`, read by the test and the binding,
/// with the `while` becoming `while (true)` so the single evaluation happens
/// once *per iteration*.
#[test]
fn an_is_binding_over_a_call_hoists_its_subject_kotlin() {
    let files = generate_files(&[("main.sv", IS_BIND_ONCE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("while (true) {") && text.contains("if (!(__is1 != null)) break"),
        "the while did not become a test-inside loop:\n{text}"
    );
    assert!(
        text.contains("val n = __is1 as Int"),
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

/// Emits one example's Kotlin, exactly as `salvo compile` would.
fn emit_example(example: &str) -> Vec<salvo_backend_kotlin::EmittedFile> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    let src = examples_dir().join(example).join("salvo");
    let errors = sources.add_dir(&src, "kt", false);
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
    salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
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
    for example in example_names() {
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
    }
}

/// One compile-and-run case per example, batched with every other
/// case: the `expected.txt` each one asserts is the *same file* the Rust
/// backend asserts, which is where the parity claim in `examples/README.md`
/// actually gets checked.
fn example_case(name: &str) -> KotlinCase {
    let expected = std::fs::read_to_string(examples_dir().join(name).join("expected.txt"))
        .unwrap_or_else(|_| panic!("examples/{name}/expected.txt is missing"));
    kotlin_case(emit_example(name), &format!("example-{name}"), &expected)
}

fn kotlin_example_actors() -> KotlinCase {
    example_case("actors")
}

fn kotlin_example_collections() -> KotlinCase {
    example_case("collections")
}

fn kotlin_example_effects() -> KotlinCase {
    example_case("effects")
}

fn kotlin_example_files() -> KotlinCase {
    example_case("files")
}

fn kotlin_example_iteration() -> KotlinCase {
    example_case("iteration")
}

fn kotlin_example_linearity() -> KotlinCase {
    example_case("linearity")
}

fn kotlin_example_qualifiers() -> KotlinCase {
    example_case("qualifiers")
}

fn kotlin_example_throw_and_release() -> KotlinCase {
    example_case("throw-and-release")
}

fn kotlin_example_time() -> KotlinCase {
    example_case("time")
}

/// Every example has a case above — checked here rather than
/// trusted, since the registry is written by hand.
#[test]
fn every_example_has_a_kotlin_case() {
    let cased: Vec<String> = KOTLIN_CASES
        .iter()
        .map(|case| case().tag)
        .filter(|tag| tag.starts_with("example-"))
        .map(|tag| tag["example-".len()..].to_string())
        .collect();
    for name in example_names() {
        assert!(
            cased.contains(&name),
            "examples/{name} has no Kotlin compile-and-run case: add one to \
             KOTLIN_CASES"
        );
    }
}

// ===================== time [time-types] [time-timer] =====================
//
// The same three programs the Rust backend runs, printing the same text: the
// time surface, a real deadline, and virtual time through `ManualTime`
// [backend-parity].

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

/// [time-coupling] The three test postures of the coupling stance, in the same
/// program the Rust backend runs: time as data (no clock at all), a scripted
/// `Ticker`, and the unified test clock — a `Ticker` whose reading is a zero
/// deadline on the very timer the test advances [backend-parity].
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

/// [time-types] [time-clock] [time-ticker] [mod-import-module] The whole
/// surface, reached with one `import time`.
fn kotlinc_compiles_and_runs_the_time_surface() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", TIME_SURFACE)]),
        "time-surface",
        TIME_SURFACE_OUTPUT,
    )
}

/// [time-timer] A real deadline fires, and fires stay ordered.
fn kotlinc_compiles_and_runs_a_real_timer() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", TIME_TIMER)]),
        "time-timer",
        TIME_TIMER_OUTPUT,
    )
}

/// [time-manual] [effect-handler-multi] Virtual time in pure Salvo, through a
/// two-face handler.
fn kotlinc_compiles_and_runs_manual_time() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", TIME_MANUAL)]),
        "time-manual",
        TIME_MANUAL_OUTPUT,
    )
}

/// [time-coupling] The three postures, and the unified test clock in
/// particular: one clock behind both the deadline and the reading, so the
/// measurement is exact — and the same text as the Rust backend prints.
fn kotlinc_compiles_and_runs_the_coupling_postures() -> KotlinCase {
    kotlin_case(
        generate_files(&[("main.sv", TIME_COUPLING)]),
        "time-coupling",
        TIME_COUPLING_OUTPUT,
    )
}

/// [time-timer] [kt-time] What a deadline registration lowers to, mirroring the
/// Rust backend: the scheduler call plus the `Fired` builder the site closes
/// over — and the time runtime, which travels with the scheduler.
#[test]
fn a_deadline_lowers_to_a_scheduler_call_with_a_fired_builder() {
    let files = generate_files(&[("main.sv", TIME_TIMER)]);
    let time = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "time.kt")
        .expect("time.kt");
    assert!(
        time.content.contains("salvo.SalvoSched.after(")
            && time.content.contains("{ __at -> Fired(Tick(__at)) }"),
        "expected the deadline lowering in:\n{}",
        time.content
    );
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "hosttime.kt"),
        "expected hosttime.kt to be emitted"
    );
}
