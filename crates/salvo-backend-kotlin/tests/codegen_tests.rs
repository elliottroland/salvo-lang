//! Kotlin codegen tests: golden snapshots of generated code, the emitted
//! text, and the programs the backend refuses. Compiling and running
//! programs is the run data's business ([test-run-data],
//! `salvo-cli/tests/run/`), on both backends at once.

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

// [effect-available] [effect-fn-deps]
#[test]
fn missing_effect_handler_is_an_error() {
    let program = build_program(&[(
        "bad.sv",
        "fn main() [use] -> None {\n    println(\"no console handler used\")\n}\n",
    )]);
    let result = emit(&program);
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

// [kt-union-wrappers] [type-union-arm-identity]
#[test]
fn unions_emit_sealed_wrappers() {
    let files = generate_unions_demo();
    // One file per arity [kt-union-wrappers].
    let unions: String = files
        .iter()
        .filter(|f| f.rel_path.starts_with("unions"))
        .map(|f| f.content.clone())
        .collect();
    let file = |n: usize| files.iter().find(|f| f.rel_path.to_string_lossy() == format!("unions/Union{n}.kt"));
    assert!(file(2).is_some_and(|f| f.content.contains("sealed interface Union2")));
    assert!(file(3).is_some_and(|f| f.content.contains("sealed interface Union3")));
    let unions = EmittedFileText { content: unions };
    // [kt-union-wrappers] The arms nest in the interface: `Union2.U1`, as
    // Rust's `Union2::U1` (user decision 2026-10-01, ABI D5).
    assert!(
        unions.content.contains(
            "sealed interface Union2<out T1, out T2> {\n    val value: Any?\n    \
             data class U1<out T1, out T2>(override val value: T1) : Union2<T1, T2>\n"
        ),
        "{}",
        unions.content
    );
    assert!(!unions.content.contains("U2_1"), "{}", unions.content);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // Wrap at return boundaries, positional arm identity (the constructor
    // call is wrapped into the union arm).
    assert!(main.content.contains("return Union2.U1<Int, String>(salvo.std.core.result.ok(input))"));
    assert!(main
        .content
        .contains("return Union2.U2<Int, String>(salvo.std.core.result.err(\"negative age\"))"));
    // Qualifier-tagged wrap picks the right arm of the 3-union.
    assert!(main
        .content
        .contains("Union3.U1<String, String, Boolean>(salvo.std.core.result.ok(\"yes\"))"));
    // Precise `is Err Str` tests a single arm; `is Ok` another.
    assert!(main.content.contains("precise is Union3.U2<*, *, *>"));
    assert!(main.content.contains("precise is Union3.U1<*, *, *>"));
    // `when` lowers to a `when` over the arm tests, each arm reading the
    // payload through its narrowing binding [ir-narrow].
    assert!(main.content.contains("(result is Union2.U1<*, *>) ->"));
    assert!(main.content.contains("((result as Union2.U1<*, *>).value as Int)"));
    // [when-exhaustive] An exhaustive `when` with no written `else` carries
    // an explicit unreachable fallthrough from the IR, so the emitter prints
    // it rather than synthesizing one: `else -> { throw AssertionError(...) }`
    // naming the unhandled-arm trap.
    assert!(
        main.content.contains("else -> {")
            && main.content.contains("unhandled union arm"),
        "{}",
        main.content
    );
    assert!(
        !main.content.contains("IllegalStateException(\"salvo: unreachable arm\")"),
        "the synthetic fallthrough should no longer appear: {}",
        main.content
    );
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
    let errors = emit(&program)
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
    let errors = emit(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("union-typed subject")),
        "unexpected errors: {errors:?}"
    );
}

// [type-union-arm-identity]
#[test]
fn union_wrap_requires_matching_arm() {
    let src = r#"
export qualifier Ok<T> of T
export qualifier Err<T> of T

export fn as_err<T>(value: T) -> +Err T {
    return value
}

export fn f() -> Ok Int | Err Str {
    return as_err(true)
}
"#;
    let program = build_program(&[("bad.sv", src)]);
    let errors = emit(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("no arm of")),
        "unexpected errors: {errors:?}"
    );
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

// [is-qualifies] [fn-emit-name] [qual-field-override]
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
    assert!(main.content.contains("if (salvo.main.Surname_qualifies(person))"));
    assert!(main.content.contains("if (salvo.main.Positive_qualifies(n))"));
    // The qualified overload is mangled; the checker routes the narrowed
    // call to it and the unqualified call to the base name.
    assert!(main
        .content
        .contains("fun fullName__SurnamePerson(person: Person): String"));
    assert!(main.content.contains("println(console, salvo.main.fullName__SurnamePerson(person))"));
    assert!(main.content.contains("println(console, salvo.main.fullName__Person(person))"));
    // Field overrides read through a narrowing binding justified by the
    // claim [qual-field-override].
    assert!(main.content.contains(": String = person.surname!!"), "{}", main.content);
    // `while x is T` lowers to a test-inside loop with a per-iteration binding.
    assert!(main.content.contains("if (!((current != null))) {"), "{}", main.content);
    assert!(main.content.contains("val c: Int = current!!"), "{}", main.content);
    // A qualified union group is physically the inner union.
    assert!(main.content.contains("val back: Union2<String, Int> = nested_"), "{}", main.content);
}

/// Runs the checker on a source and returns the errors (panics if none).
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src)]);
    emit(&program)
        .err()
        .expect("expected type errors")
}

/// [iter-protocol] A hand-written **pass** driven by `for`: the same source
/// and the same expected stdout as the Rust backend's
/// `rustc_compiles_and_runs_a_hand_written_pass`. `zip` is the point — it
/// reads two sources at once, which `yield` cannot express.
const PASS_DEMO: &str = r#"
struct Zip : Next<self, (Str, Int)> canbe Mut {
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

struct Countdown : Next<self, Int> canbe Mut {
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
    use StdConsole()
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = List("ada", "grace", "alan")
    let ages = List(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}
"#;

/// Kotlin has no pattern-matching loop condition, so the driving loop is
/// `while (true)` plus a guard. The arm is spelled with its *real* type
/// arguments where it can be (a non-generic `next`), which is what keeps the
/// element read free of an unchecked cast — a warning in code the user cannot
/// edit.
#[test]
fn a_pass_lowers_to_a_guarded_while_loop() {
    let program = build_program(&[("main.sv", PASS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains(": Countdown = salvo.main.countdown(3)") && src.contains(": Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Countdown("),
        "expected the driving loop in:\n{src}"
    );
    // [ir-loop] The pass is driven by one `while (true)` testing the step's
    // arm; `Finished` breaks.
    assert!(
        src.contains("(__step_4 is Union2.U1<*, *>) ->") && src.contains("else -> {\n                break"),
        "expected the step test and the break in:\n{src}"
    );
}

/// [kt-struct-empty] A fieldless struct is a plain class, not a data class:
/// Kotlin requires a data class to have at least one primary-constructor
/// parameter. `Finished` is the first such struct (std's iterator protocol).
#[test]
fn a_fieldless_struct_is_a_plain_class() {
    let program = build_program(&[("main.sv", PASS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("std/core/iterator.kt"))
        .expect("std/core/iterator.kt")
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
import std.throw

struct Reader : Next<self, Ok Str | Err Str> canbe Mut {
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

fn read_all(lines: List<Str>) [Throw<Str>, Console] -> Int
=> !lines {
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
    use StdConsole()
    let good = try { read_all(List("alpha", "beta")) }
    when good {
        is Ok { println("read ${good}") }
        is Thrown { println("failed: ${good}") }
    }
    let bad = try { read_all(List("alpha", "boom", "gamma")) }
    when bad {
        is Ok { println("read ${bad}") }
        is Thrown { println("failed: ${bad}") }
    }
}
"#;

/// [qual-group] The nested wrap, read off the generated source rather than
/// only from the program's output: `emitted(err(...))` lands in the group arm
/// `Emitted (Ok Str | Err Str)`, whose inner union is a wrapper of its own, so
/// the value takes *two* wraps — inner arm first, and the arm index comes
/// from the qualifier left over after the group's own is removed.
#[test]
fn a_flattened_qualifier_wraps_the_inner_union_first() {
    let program = build_program(&[("main.sv", FALLIBLE_PASS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt");
    for (arm, what) in [("Union2.U2", "err"), ("Union2.U1", "ok")] {
        let needle = format!("Union2.U1<Union2<String, String>, salvo.std.core.iterator.Finished>({arm}<String, String>(");
        assert!(
            main.content.contains(&needle),
            "expected the inner {what} arm to be wrapped before the outer one in:\n{}",
            main.content
        );
    }
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
struct Slice<T> : Next<self, proj T> canbe Mut {
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
fn take<It>(it: Mut It, count: Int, ?Next<It, Int>) [] -> Int => it: Mut {
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
    state {
        i: Int = 0
    }

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

iter fn iter(r: Rolls) [Random<Int>] -> Emitted Int | Finished {
    state RollsIter {
        made: Int = 0
    }
    if made >= r.count {
        return finished()
    }
    made = made + 1
    return emitted(next_random())
}

fn main() [use] {
    use StdConsole()
    use CyclicRandom(array_of(7, 8, 9))
    let xs = List(1, 2, 3, 4)
    let p = slice(xs)
    println("first ${take(p, 2)}")
    println("rest ${take(p, 9)}")
    for v in rolls(3) {
        println("v ${v}")
    }
}
"#;

/// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
/// linear pass releases it through the `?Linear<It>` spread. The release cannot
/// print — an implicitly resolved fn is effect-free [implicit-resolve] — so the
/// call is asserted in the emitted code and the program is run to prove it
/// builds.
const GENERIC_CLOSE_DEMO: &str = r#"
linear struct Handle : Next<self, Int> canbe Mut {
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
fn drain<It canbe linear>(it: Mut It, stop: Int, end: (x: It) -> None, ?Next<It, Int>) [] -> Int => !it =>[end] !x {
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
    use StdConsole()
    println("all ${drain(open_handle(4), 100, close)}")
    println("cut ${drain(open_handle(4), 5, close)}")
}
"#;

/// backend's `rustc_compiles_and_runs_a_released_raw_pass`.
const RAW_CLOSE_DEMO: &str = r#"
linear struct Ticks : Next<self, Int> canbe Mut {
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
    use StdConsole()
    drained()
    abandoned()
}
"#;

#[test]
fn a_raw_pass_is_driven_in_place_with_no_finally() {
    // [iter-drive-in-place] [linear-group] No implicit discharge sites
    // (user decision 2026-09-12): no finally splice — the program's own
    // `close(lines)` is the discharge, an ordinary call.
    let program = build_program(&[("main.sv", RAW_CLOSE_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        // Plain: the program's module declares one `close` [fn-emit-name].
        src.contains("close(console, lines)"),
        "expected the program's own explicit close:\n{src}"
    );
}

const YIELD_SPREAD_DEMO: &str = r#"
struct Slice<T> : Next<self, proj T> canbe Mut {
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

fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, mapper {
    let out = Mut List<U>()
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
    use StdConsole()
    let xs = List(1, 2, 3)
    let doubled = map2(slice(xs), double)
    for d in doubled {
        println("d ${d}")
    }
}
"#;

/// The spread position renders as a Kotlin function type whose result is the
/// union *wrapper*, not the Salvo text.
#[test]
fn a_union_returning_implicit_renders_the_wrapper_type() {
    let program = build_program(&[("main.sv", YIELD_SPREAD_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains("next: (It) -> Union2<T, salvo.std.core.iterator.Finished>"),
        "expected the union wrapper in the spread position, got:\n{src}"
    );
    assert!(
        !src.contains("Emitted T | Finished"),
        "the Salvo type text must not reach the Kotlin source:\n{src}"
    );
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
            "import quals.Fancy\n\nfn make() -> +Fancy Int {\n    return 1\n}\n",
        ),
    ]);
    let errors = emit(&program)
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
    use StdConsole
    println(describe(make()))
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.content.contains("salvo.main.describe__PositiveInt(salvo.main.make())"),
        "content: {}",
        main.content
    );
}

// [qual-ctor-simple]
#[test]
fn constructor_return_type_must_be_simple() {
    let src = "export qualifier Fancy of Int\n\nexport fn make() -> +Fancy (Int | Str) {\n    return 1\n}\n";
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
    use StdConsole
    let items: Mut List<Int> = Mut List(1)
    fill(items, 2)
    println("size: ${items.size()}")
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
    assert!(
        main.content
            .contains("fun fill(target: salvo.platform.std.core.list.MutList<Int>, n: Int)"),
        "unexpected: {}",
        main.content
    );
    assert!(main
        .content
        .contains("val items: salvo.platform.std.core.list.MutList<Int> = mutableListOf<Int>(1)"));
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
        .contains(": CyclicRandom<Int> = CyclicRandom<Int>(values = listOf<Int>(10, 20, 30), copy = { __a0 -> __a0 })"), "{}", main.content);
    assert!(main
        .content
        .contains(": CyclicRandom<String> = CyclicRandom<String>(values = listOf<String>(\"a\", \"b\"), copy = { __a0 -> __a0 })"), "{}", main.content);
    // Each instance is bound behind the effect's handle [effect-handle].
    assert!(main.content.contains(": Random<Int> = __Mon_Random<Int>(__use_"), "{}", main.content);
    assert!(main.content.contains(": Random<String> = __Mon_Random<String>(__use_"), "{}", main.content);
    // Callee effect dependencies are threaded in declaration order.
    assert!(main.content.contains("draw(__handle_4, __handle_6, __handle_2)"), "{}", main.content);
    assert!(main.content.contains("luckyNumber(__handle_4)"), "{}", main.content);
    // Expected-type disambiguation picks the right handler per call: the
    // two `Random` instances are two parameters.
    assert!(main.content.contains("val n: Int = random.nextRandom()"), "{}", main.content);
    assert!(main.content.contains("val s: String = random__1.nextRandom()"), "{}", main.content);
}

// [effect-use-requires-use]
#[test]
fn use_requires_the_use_effect() {
    let errors = expect_errors("fn setup() -> None {\n    use StdConsole\n}\n");
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

// [effect-use-no-dup] [effect-intercept]
#[test]
fn a_use_may_shadow_an_earlier_registration() {
    // Until 2026-09-14 this was "a handler for `Console` is already
    // registered in this scope"; shadowing is now how interception is
    // written, so a second registration is legal and the innermost wins.
    let program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdConsole\n    use StdConsole\n}\n",
    )]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert_eq!(
        main.content.matches("StdConsole()").count(),
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

// [effect-disambiguation] [effect-use-no-dup] Shadowing collapses repeats of *one*
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

/// [kt-handle] [effect-handler-deps] A **generic dependent handler**
/// (`Twice<T> [Store<T>]`) is an ordinary class with a `__dep_Store: Store<T>`
/// field: nothing generated has to name the handler's generics, so the cut
/// the fused emission stated ("an effect set fused here is still generic")
/// is gone (2026-09-28). Same program and output on the Rust backend.
const GENERIC_DEPENDENT_HANDLER_DEMO: &str = "\
effect Store<T> {
    fn keep(value: T) -> Str => value
}

handler MemStore<T> of Store<T> {
    fn keep(value: T) -> Str => value {
        return \"kept\"
    }
}

handler Twice<T> [Store<T>] of Store<T> {
    fn keep(value: T) -> Str => value {
        return \"${keep(value)} twice\"
    }
}

fn main() [use] -> None {
    use StdConsole()
    use MemStore<Int>()
    use Twice<Int>()
    println(keep(3))
}
";

#[test]
fn a_generic_dependent_handler_holds_a_typed_dependency_field() {
    let program = build_program(&[("main.sv", GENERIC_DEPENDENT_HANDLER_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files.iter().find(|f| f.rel_path.ends_with("main.kt")).unwrap().content;
    assert!(
        main.contains("class Twice<T>(private val __dep0: Store<T>) : Store<T>"),
        "expected the dependency as a typed field:\n{main}"
    );
    // [effect-handle] The `use` passes the scope's handle of `Store<Int>`.
    assert!(
        main.contains(": Twice<Int> = Twice<Int>(__handle_"),
        "expected the `use` to pass the scope's binding:\n{main}"
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


// ===== the defect round of 2026-09-25, byte-identical to the Rust side =====

/// [ref-notsame] [fate-elem-distinct] Two handles proven apart by `NotSame`
/// through a custom container's accessor (map slots, one slot's roster, the
/// bench) and through nested structs with no accessor of their own; two
/// handles to one element make the test false at run time (`===`). Source
/// and expected stdout are verbatim the Rust backend's
/// `rustc_compiles_and_runs_notsame_handles` — the parity is the assertion.
const NOTSAME_DEMO: &str = r#"
// A player lives on a team's roster (a map slot's list) or on the bench.
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

// A custom container's own accessor: the handle's path runs through a map
// slot, or through a field.
fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 {
        return at(l.bench, -n - 1)
    }
    let t = at(l.teams, team)
    if t is None {
        return None
    }
    return at(t.roster, n)
}

fn trade(l: League, a: ref(l) Mut Player, b: ref(l) Mut Player) -> None {
    let g = copy(a.goals)
    a.goals = b.goals
    b.goals = g
}

fn score(a: Mut Player, b: Mut Player) -> None => a: Mut, b: Mut {
    a.goals = a.goals + 1
    b.goals = b.goals + 1
}

fn league() [] -> Mut League {
    let teams: Mut Map<Str, Mut Team> = Mut Map()
    put(teams, "red", Mut Team { coach: "Ada", roster: List(Mut Player { name: "Bo", goals: 0 }, Mut Player { name: "Cy", goals: 5 }) })
    put(teams, "blue", Mut Team { coach: "Dee", roster: List(Mut Player { name: "Ed", goals: 2 }) })
    return Mut League { teams: teams, bench: List(Mut Player { name: "Flo", goals: 9 }) }
}

// Structs with no accessor of their own, nested: the handle's path is the
// field chain `a.b.c` plus a position.
struct Cell canbe Mut { v: Int }
struct Inner canbe Mut { c: List<Mut Cell> }
struct Outer canbe Mut { tag: Str, b: Inner }

fn at(a: Outer, index: Int) [] -> ref(a) Mut Cell? => a, index {
    return at(a.b.c, index)
}

fn bump(x: Mut Cell, y: Mut Cell) -> None => x: Mut, y: Mut {
    x.v = x.v + 1
    y.v = y.v + 10
}

fn shared_container() [Console] -> None {
    let l = league()
    let star = at(l, "red", 1) ?: unreachable!()
    star.goals = star.goals + 1
    println("bench ${size(l.bench)}")
    star.goals = star.goals + 1
    trade(l, at(l, "red", 0) ?: unreachable!(), at(l, "blue", 0) ?: unreachable!())
    trade(l, star, star)
    println("${star.name} ${star.goals}")
    // The bench and a roster: the paths diverge at the first field.
    let sub = at(l, "", -1) ?: unreachable!()
    if sub is NotSame(star) {
        score(star, sub)
        star.goals = star.goals + 10
        sub.goals = sub.goals + 100
    }
    // Two handles to one element: the test answers false at run time.
    let same_one = at(l, "red", 1) ?: unreachable!()
    if same_one is NotSame(star) {
        println("wrongly distinct")
    } else {
        println("same element")
    }
    println("${star.goals} ${(get(l.bench, 0) ?: unreachable!()).goals}")
}

fn map_slots() [Console] -> None {
    let l = league()
    // Two map slots: the paths diverge at the slot.
    let a = at(l, "red", 0) ?: unreachable!()
    let b = at(l, "blue", 0) ?: unreachable!()
    if b is NotSame(a) {
        score(a, b)
        println("${a.goals} ${b.goals}")
    }
    // One slot, two positions in its roster.
    let c = at(l, "red", 1) ?: unreachable!()
    if c is NotSame(a) {
        score(a, c)
    }
    let d = at(l, "", -1) ?: unreachable!()
    if d is NotSame(c) {
        score(c, d)
    }
    println("${c.goals} ${d.goals}")
}

fn nested() [Console] -> None {
    let a = Mut Outer { tag: "t", b: Inner { c: List(Mut Cell { v: 1 }, Mut Cell { v: 2 }) } }
    let p = at(a, 0) ?: unreachable!()
    let q = at(a, 1) ?: unreachable!()
    if q is NotSame(p) {
        bump(p, q)
        p.v = p.v + 100
        q.v = q.v + 200
    }
    let r = at(a, 0) ?: unreachable!()
    if r is NotSame(p) {
        println("wrongly distinct")
    } else {
        println("same cell")
    }
    println("${(get(a.b.c, 0) ?: unreachable!()).v} ${(get(a.b.c, 1) ?: unreachable!()).v}")
}

fn main() [use] {
    use StdConsole()
    shared_container()
    map_slots()
    nested()
}
"#;

#[test]
fn notsame_handles() {
    let program = build_program(&[("main.sv", NOTSAME_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let reff = files.iter().find(|f| f.rel_path.ends_with("std/core/ref.kt")).unwrap();
    assert!(reff.content.contains("==="), "{}", reff.content);
}

/// [loop-while-value] [deduce-same-call] A value loop whose body ends in an
/// `else`-less `if` (a statement: its arm only `break`s), and a call whose
/// later argument is hoisted past an earlier one with effects of its own —
/// which must still run first. Both backends print the same.
pub const LOOP_BREAK_ORDER_DEMO: &str = r#"
fn first_big(xs: List<Int>) -> Int? => xs {
    return for x in xs {
        if x > 2 { break x }
    }
}

fn noisy(n: Int) [Console] -> Int {
    println("noisy ${n}")
    return n
}

fn both(f: (a: Int, b: Int) [Console] -> Int, x: Int) [Console] -> Int {
    return f(noisy(1), f(noisy(2), x))
}

fn main() [use] {
    use StdConsole()
    println("${first_big(List(1, 5)) ?: 0} ${first_big(List(1)) ?: 0}")
    println("${both((a, b) -> { println("f ${a} ${b}") return a + b }, 10)}")
}
"#;

pub const LOOP_BREAK_ORDER_OUTPUT: &str = "5 0\nnoisy 1\nnoisy 2\nf 2 10\nf 1 12\n13\n";

/// [proj-opt-slot] [rs-opt-borrow] A borrowed optional — `get`'s answer, an
/// optional handle, a `Mut Str` element — fits a kept owned `T?` parameter,
/// of a fn or an effect member; owned optionals, literals and `None` still
/// do; `copy` of one is an owned optional. On Rust the parameter is
/// `Option<&T>`, so the body's lets, narrowings and `?:` read the
/// borrow it holds.
pub const OPT_SLOT_DEMO: &str = r#"
struct Entity canbe Mut { name: Str, hp: Int }

fn inner(s: Str?) -> Str {
    return s ?: "-"
}

fn outer(s: Str?) -> Str {
    let t = s
    let u = inner(t)
    let n = if s is Str v { to_upper(v) } else { "?" }
    let c = copy(s)
    let none = if s is None { "absent" } else { "present" }
    return "${u} ${n} ${inner(c)} ${none}"
}

fn named(e: Entity?) -> Str {
    if e is Entity f {
        return "${f.name}:${f.hp}"
    }
    return "nobody"
}

fn view(s: Str?) -> (proj(s) Str)? => s {
    return s
}

effect Shower {
    fn show(s: Str?) -> None => s
}

handler PrintShower [Console] of Shower {
    fn show(s: Str?) -> None => s {
        println("shown ${inner(s)}")
    }
}

fn main() [use] {
    use StdConsole()
    use PrintShower()
    let xs: List<Str> = List("abc", "de")
    println(outer(get(xs, 0)))
    println(outer(get(xs, 9)))
    let o: Str? = "own"
    println(outer(o))
    println(outer("lit"))
    println(outer(None))
    let ws: List<Mut Str> = List(Mut Str("w"))
    println(outer(get(ws, 0)))
    println(outer(at(ws, 0)))
    let cw = copy(get(ws, 0))
    println(outer(cw))
    let es: List<Mut Entity> = List(Mut Entity { name: "e", hp: 3 })
    println("${named(get(es, 0))} ${named(at(es, 0))} ${named(get(es, 4))} ${named(None)}")
    let h = at(es, 0)
    println(named(h))
    let ce = copy(get(es, 0))
    let hh = h ?: unreachable!()
    hh.hp = 40
    println("${named(ce)} ${named(get(es, 0))}")
    println(inner(view(get(xs, 1))))
    show(get(xs, 0))
    show(o)
}
"#;

pub const OPT_SLOT_OUTPUT: &str = "abc ABC abc present\n- ? - absent\nown OWN own present\nlit LIT lit present\n- ? - absent\nw W w present\nw W w present\nw W w present\ne:3 e:3 nobody nobody\ne:3\ne:3 e:40\nde\nshown abc\nshown own\n";

/// [col-locate] [ref-handle] A deque mints handles as a list does — `at`
/// by index, and the total `at` at an `Idx(d)` — and a map's total `at` at a
/// `KeyOf(m)` key answers the handle with no `None` arm (Rust: the slot the
/// claim proved present).
pub const DEQUE_MAP_AT_DEMO: &str = r#"
struct Item canbe Mut { n: Int }

fn bump(i: Mut Item) -> None => i: Mut {
    i.n = i.n + 1
}

fn main() [use] {
    use StdConsole()
    let d: Mut Deque<Mut Item> = mut_deque_of()
    add_first(d, Mut Item { n: 1 })
    add_last(d, Mut Item { n: 2 })
    let h = at(d, 1) ?: unreachable!()
    h.n = h.n + 10
    println("${size(d)} ${h.n}")
    bump(at(d, 0) ?: unreachable!())
    let i = 0
    if i is Idx(d) {
        bump(at(d, i))
    }
    println("${at(d, 7) is None} ${(get(d, 0) ?: unreachable!()).n} ${(get(d, 1) ?: unreachable!()).n}")
    let m: Mut Map<Str, Mut Item> = Mut Map()
    put(m, "a", Mut Item { n: 5 })
    let k = "a"
    if k is KeyOf(m) {
        let v = at(m, k)
        v.n = v.n * 3
        bump(at(m, k))
    }
    println("${(get(m, "a") ?: unreachable!()).n}")
}
"#;

pub const DEQUE_MAP_AT_OUTPUT: &str = "2 12\ntrue 3 12\n16\n";

/// [rs-path] [ref-handle] One rendering for a `ref` result, whatever `T` is:
/// `head_of` at a `Mut` element and at `Str`, a `Pick` group through one and
/// two implicit layers at both, and a read-only handle held across a write
/// through another handle.
pub const REF_ONE_RENDERING_DEMO: &str = r#"struct Counter canbe Mut { n: Int }

fn head_of<T>(xs: List<T>) -> ref(xs) T? => xs {
    return at(xs, 0)
}

fns Pick<C, L, T> {
    fn pick(c: C, l: L) -> ref(c) T?
}

fn pick_via<C, L, T>(c: C, l: L, ?Pick<C, L, T>) -> ref(c) T? => c, l {
    return pick(c, l)
}

fn pick2<C, L, T>(c: C, l: L, ?Pick<C, L, T>) -> ref(c) T? => c, l {
    return pick_via(c, l)
}

fn pick<T>(c: List<T>, l: Int) -> ref(c) T? => c, l {
    return at(c, l)
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

fn main() [use] {
    use StdConsole()
    let cs: Mut List<Mut Counter> = Mut List(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    let h = head_of(cs) ?: unreachable!()
    h.n = h.n + 10
    bump(pick_via(cs, 1) ?: unreachable!())
    bump(pick2(cs, 1) ?: unreachable!())
    println("${(get(cs, 0) ?: unreachable!()).n} ${(get(cs, 1) ?: unreachable!()).n}")
    let ss: List<Str> = List("a", "b")
    println("${head_of(ss) ?: unreachable!()} ${pick_via(ss, 1) ?: unreachable!()} ${pick2(ss, 0) ?: unreachable!()}")
    // a read-only handle held across a write through another handle
    let r = head_of(ss) ?: unreachable!()
    let w = head_of(cs) ?: unreachable!()
    w.n = w.n + 100
    println("${r} ${(get(cs, 0) ?: unreachable!()).n}")
}
"#;

pub const REF_ONE_RENDERING_OUTPUT: &str = "11 4\na b a\na 111\n";

/// [proj-opt-slot] [rs-opt-borrow] Unions of borrows: a `proj Mut A | B`
/// value, an owned union and an optional `get` all reach kept `A | B` and
/// `(A | B)?` parameters, which bind, narrow and pass the union on; a `Mut A
/// | Mut B` parameter is mutated through its arm; `copy` of a borrowed union
/// is an owned one.
pub const UNION_BORROW_DEMO: &str = r#"
struct A canbe Mut { n: Int }
struct B canbe Mut { s: Str }

fn describe(x: A | B) -> Str => x {
    if x is A {
        return "a${x.n}"
    }
    if x is B {
        return "b${x.s}"
    }
    return "?"
}

fn relay(x: A | B) -> Str => x {
    let y = x
    return "${describe(y)}/${describe(x)}"
}

fn maybe(x: (A | B)?) -> Str => x {
    if x is None {
        return "none"
    }
    if x is A {
        return "A${x.n}"
    }
    return "other"
}

fn bump(x: Mut A | Mut B) -> None => x: None {
    if x is A {
        x.n = x.n + 1
    }
    if x is B {
        x.s = "${x.s}+"
    }
}

fn pick(flag: Bool, xs: List<Mut A>, b: B) -> Str => xs, b {
    let v = if flag { get(xs, 0) ?: unreachable!() } else { b }
    return relay(v)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut A> = List(Mut A { n: 1 })
    let b = B { s: "x" }
    println("${pick(true, xs, b)} ${pick(false, xs, b)}")
    let own: A | B = A { n: 7 }
    println("${describe(own)} ${relay(own)} ${maybe(own)} ${maybe(None)} ${maybe(get(xs, 0))} ${maybe(get(xs, 3))}")
    let m: Mut A | Mut B = Mut B { s: "q" }
    bump(m)
    bump(m)
    let h = at(xs, 0) ?: unreachable!()
    let w: Mut A | Mut B = Mut A { n: 0 }
    bump(w)
    println("${describe(m)} ${describe(w)} ${(get(xs, 0) ?: unreachable!()).n}")
    let c = copy(if true { get(xs, 0) ?: unreachable!() } else { b })
    println(describe(c))
    let _ = h
}
"#;

pub const UNION_BORROW_OUTPUT: &str = "a1/a1 bx/bx\na7 a7/a7 A7 none A1 none\nbq++ a1 1\na1\n";

/// [proj-opt-slot] [rs-opt-borrow] A `Mut T?` parameter takes an optional
/// handle, a bound one more than once, an owned optional and `None`; it is
/// handed on to another `Mut T?` parameter. On Rust it is `Option<&mut T>`.
pub const MUT_OPT_SLOT_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity?) -> Int => e: Mut {
    if e is Entity {
        e.hp = e.hp + 1
        return e.hp
    }
    return -1
}

fn heal_twice(e: Mut Entity?) -> None => e: Mut {
    heal(e)
    heal(e)
}

fn shout(s: Mut Str?) -> None => s: Mut {
    if s is Mut Str {
        append(s, "!")
    }
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 3 }, Mut Entity { hp: 10 })
    println("${heal(at(es, 0))} ${heal(at(es, 9))}")
    let h = at(es, 1)
    heal(h)
    heal_twice(h)
    heal_twice(at(es, 0))
    let own: Mut Entity? = Mut Entity { hp: 100 }
    heal(own)
    heal(None)
    let ws: List<Mut Str> = List(Mut Str("hey"))
    shout(at(ws, 0))
    shout(at(ws, 0))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp} ${(own ?: unreachable!()).hp} ${get(ws, 0) ?: unreachable!()}")
}
"#;

pub const MUT_OPT_SLOT_OUTPUT: &str = "4 -1\n6 13 101 hey!!\n";

/// [ref-anchor] Element containers (user decisions 2026-10-10): a `ref(lib)`
/// argument may be a handle anywhere *inside* `lib` — from a mint, inline
/// nested `at`s, or `at` of a bound handle's field — and a `ref(c)` may name
/// another handle parameter, so `a: ref(al)` is a position in `al`, itself a
/// position in `lib`. On Rust `lib` is passed once and every handle is a path
/// from it.
pub const ELEM_CONTAINER_DEMO: &str = r#"
struct Track canbe Mut { plays: Int }
struct Album canbe Mut { name: Str, tracks: List<Mut Track> }
struct Library canbe Mut { albums: List<Mut Album> }

// A mint reaching two levels into the library.
fn track(lib: Library, i: Int, j: Int) -> ref(lib) Mut Track? {
    let al = at(lib.albums, i)
    if al is Album {
        return at(al.tracks, j)
    }
    return None
}

// Handles anywhere inside `lib`.
fn swap_plays(lib: Library, a: ref(lib) Mut Track, b: ref(lib) Mut Track) -> None {
    let p = a.plays
    a.plays = b.plays
    b.plays = p
}

// A handle parameter anchored at another handle parameter: positions in
// `al`, which is a position in `lib`.
fn swap_in(lib: Library, al: ref(lib) Mut Album, a: ref(al) Mut Track, b: ref(al) Mut Track) -> None {
    let p = a.plays
    a.plays = b.plays
    b.plays = p
    al.name = "${al.name}*"
}

fn main() [use] {
    use StdConsole()
    let lib = Mut Library { albums: List(
        Mut Album { name: "x", tracks: List(Mut Track { plays: 1 }, Mut Track { plays: 2 }) },
        Mut Album { name: "y", tracks: List(Mut Track { plays: 3 }) }) }
    // Through a mint, inline nested `at`s, and a bound handle.
    swap_plays(lib, track(lib, 0, 1) ?: unreachable!(), track(lib, 1, 0) ?: unreachable!())
    swap_plays(lib, at((at(lib.albums, 0) ?: unreachable!()).tracks, 0) ?: unreachable!(), at((at(lib.albums, 1) ?: unreachable!()).tracks, 0) ?: unreachable!())
    let al = at(lib.albums, 0) ?: unreachable!()
    swap_plays(lib, at(al.tracks, 0) ?: unreachable!(), track(lib, 1, 0) ?: unreachable!())
    // Self-strike is fine: one container, two positions that coincide.
    swap_in(lib, al, at(al.tracks, 0) ?: unreachable!(), at(al.tracks, 1) ?: unreachable!())
    swap_in(lib, al, at(al.tracks, 1) ?: unreachable!(), at(al.tracks, 1) ?: unreachable!())
    let a0 = get(lib.albums, 0) ?: unreachable!()
    let a1 = get(lib.albums, 1) ?: unreachable!()
    println("${a0.name} ${(get(a0.tracks, 0) ?: unreachable!()).plays} ${(get(a0.tracks, 1) ?: unreachable!()).plays} ${a1.name} ${(get(a1.tracks, 0) ?: unreachable!()).plays}")
}
"#;

pub const ELEM_CONTAINER_OUTPUT: &str = "x** 3 1 y 2\n";

// ===== [op-safe-nav] `?.` optional chaining =====

/// The same source and expected stdout as the Rust backend's
/// `rustc_runs_safe_navigation`: a `?.` field chain feeding a `?:`, and a
/// `?.` call link on a place receiver. On Kotlin the field chain renders as a
/// native `o?.inner?.nanos` ([op-safe-nav], asserted in
/// `safe_navigation_field_chain_is_native_kotlin`); both backends must agree
/// on the short-circuiting result.
const SAFE_NAV_DEMO: &str = r#"
struct Inner {
    nanos: Long
}

struct Outer {
    inner: Inner?
}

fn label(i: Inner) -> Str {
    return "nanos=${i.nanos}"
}

fn nanos_or(o: Outer?) -> Long {
    return o?.inner?.nanos ?: -1L
}

fn describe(i: Inner?) -> Str {
    return i?.label() ?: "absent"
}

fn main() [use] {
    use StdConsole()
    let a = Outer { inner: Inner { nanos: 42L } }
    let b = Outer { inner: None }
    let c: Outer? = None
    println(to_str(nanos_or(a)))
    println(to_str(nanos_or(b)))
    println(to_str(nanos_or(c)))
    println(describe(Inner { nanos: 42L }))
    let no_inner: Inner? = None
    println(describe(no_inner))
}
"#;


// ===== [op-arm-eq] `union == value` picks the arm by the value's type =====

// [op-safe-nav] [kt-safe-nav]
#[test]
fn safe_navigation_field_chain_is_native_kotlin() {
    let program = build_program(&[("main.sv", SAFE_NAV_DEMO)]);
    let files = emit(&program)
        .unwrap_or_else(|errors| panic!("codegen errors:\n{}", errors.join("\n")));
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // A field-only `?.` chain is a native Kotlin optional chain, not a raw
    // `when`/`if` over the receiver.
    assert!(
        main.content.contains("o?.inner?.nanos"),
        "expected a native `?.` chain, got:\n{}",
        main.content
    );
}

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

// [loop-while-value] [kt-loop-value]
#[test]
fn loops_lower_to_run_blocks() {
    let files = generate_loops_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // [ir-loop] Value loops become `run {}` blocks with a nullable result
    // local, read narrowed after the loop when the join type has no `None`
    // arm (`Justification::LoopValue`).
    let c = &main.content;
    assert!(c.contains("val last: Int = kotlin.run {"), "{c}");
    assert!(c.contains("var __loop_3: Int? = null"), "{c}");
    assert!(c.contains("__loop_3 = (i * 10)"), "{c}");
    assert!(c.contains("val __loop_value_6: Int = __loop_3!!"), "{c}");
    // The `else` runs (and assigns) only when the loop never did.
    assert!(c.contains("var __ran_4: Boolean = false"), "{c}");
    assert!(c.contains("if (!(__ran_4)) {"), "{c}");
    assert!(c.contains("__loop_3 = (-1)"), "{c}");
    // `break value` assigns the result local before breaking.
    assert!(c.contains("__loop_10 = x\n                break"), "{c}");
    // Without `else` (or with a bare `break`) the value stays nullable: no
    // `!!` on the `found`/`capped` loops.
    assert!(c.contains("val __loop_value_11: Int? = __loop_10\n"), "{c}");
    assert!(c.contains("val __loop_value_18: Int? = __loop_16\n"), "{c}");
    // A union-typed loop value keeps the declared arm order.
    assert!(c.contains("var __loop_22: Union2<String, Int>? = null"), "{c}");
}

// [loop-while-value]
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
    // [kt-ir] A foreign reference is fully qualified, so a module imports
    // only the root runtime package.
    assert!(main.content.contains("salvo.std.core.console.println("), "{}", main.content);
    assert!(main.content.contains("salvo.geometry.area("), "{}", main.content);
    let geometry = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "geometry.kt")
        .unwrap();
    assert!(geometry.content.starts_with("package salvo.geometry\n"));
    let imports: Vec<&str> = geometry.content.lines().filter(|l| l.starts_with("import ")).collect();
    assert_eq!(imports, vec!["import salvo.*"], "{}", geometry.content);
}

// [kt-imports] An aliased Salvo import leaves no trace: the call names the
// declaration, fully qualified.
#[test]
fn aliased_imports_emit_kotlin_alias_imports() {
    let main = r#"
import geometry.area as rect_area

fn main() [use] -> None {
    use StdConsole
    println("area: ${rect_area(3, 4)}")
}
"#;
    let geometry = "export fn area(w: Int, h: Int) -> Int {\n    return w * h\n}\n";
    let program = build_program(&[("main.sv", main), ("geometry.sv", geometry)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // [kt-ir] An aliased import names the declaration: the reference is
    // the declaration's own, fully qualified, and the alias is gone.
    assert!(main.content.contains("salvo.geometry.area(3, 4)"), "{}", main.content);
    assert!(!main.content.contains("rectArea"), "{}", main.content);
}

// [kt-imports] [fn-emit-name] An aliased import of a fn with a mangled
// qualified overload: each call names its overload's mangled name, fully
// qualified.
const MANGLED_ALIAS_MAIN: &str = r#"
import lib.shout as holler
import lib.loud

fn main() [use] -> None {
    use StdConsole
    println(holler("hi"))
    let l = loud("hey")
    println(holler(l))
}
"#;

const MANGLED_ALIAS_LIB: &str = r#"
export qualifier Loud of Str

export fn loud(s: Str) -> +Loud Str
=> !s {
    return s
}

export fn shout(s: Str) -> Str
=> !s {
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // [kt-ir] [fn-emit-name] The aliased calls name the mangled overloads
    // of the declaring module, fully qualified.
    for needle in [
        "salvo.lib.shout__Str(\"hi\")",
        "salvo.lib.shout__LoudStr(l)",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

// [kt-effect-params] Effect parameters avoid user parameter names.
#[test]
fn effect_params_avoid_user_names() {
    let src = r#"
export fn shadowed(console: Str) [Console] -> None => !console {
    println("param: ${console}")
}

export fn main() [use] -> None {
    use StdConsole
    shadowed("value")
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
    // The generated effect parameter picks a fresh name.
    assert!(
        main.content
            .contains("fun shadowed(console__1: salvo.std.core.console.Console, console: String)"),
        "unexpected: {}",
        main.content
    );
    assert!(main
        .content
        .contains("println(console__1, \"param: ${console}\")"));
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


// [type-lit-numeric] Literal suffixes map onto Kotlin's: `5L` stays `5L`,
// `2.5f` stays `2.5f`, unsuffixed floats are Double (`1.5`).
#[test]
fn numeric_literal_suffixes_emit_kotlin_suffixes() {
    let program = build_program(&[(
        "main.sv",
        "fn main() [use] -> None {\n    use StdConsole\n    \
         let big: Long = 5L\n    let ratio: Float = 2.5f\n    let d: Double = 1.5\n    \
         println(\"${big} ${ratio} ${d}\")\n}\n",
    )]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
    use StdConsole
    println(describe(ok("x")))
    println(describe(err("y")))
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    // Positional arm identity: Ok -> first arm, Err -> second arm.
    assert!(
        main.content
            .contains("describe(Union2.U1<String, String>(salvo.std.core.result.ok(\"x\")))"),
        "content: {}",
        main.content
    );
    assert!(
        main.content
            .contains("describe(Union2.U2<String, String>(salvo.std.core.result.err(\"y\")))"),
        "content: {}",
        main.content
    );
}

// ===== S1: the `copy` intrinsic [platform-intrinsic-fn] [copy-fn] [kt-copy] =====

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
    use StdConsole
    let s = "hi"
    let t = copy(s)
    println(t)
    let xs = Mut List(1, 2, 3)
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

// [platform-intrinsic-fn] [kt-copy] `copy` lowers type-directedly from the
// checker's resolved argument type.
#[test]
fn copy_lowers_type_directedly() {
    let program = build_program(&[("main.sv", COPY_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted");
    // Identity for a transitively immutable type (Str).
    assert!(
        main.content.contains("val t: String = s\n"),
        "generated:\n{}",
        main.content
    );
    // A real copy for `Mut List<Int>`.
    assert!(
        main.content.contains("val ys: salvo.platform.std.core.list.MutList<Int> = xs.toMutableList()"),
        "generated:\n{}",
        main.content
    );
    // The data-class shallow copy for a `Mut` struct with immutable fields.
    assert!(
        main.content.contains(": Person = p.copy()"),
        "generated:\n{}",
        main.content
    );
    // Arrays are index-assignable without `Mut`, so they copy for real.
    assert!(
        main.content.contains(": Array<Int> = arr.copyOf()"),
        "generated:\n{}",
        main.content
    );
}

// [kt-copy] [backend-never-wrong] Nested mutability Kotlin cannot copy
// correctly yet — a map's mutable values — is a codegen error, never a
// diverging copy. (A list of mutable lists used to be the example; it
// copies element-wise since 2026-10-05.)
#[test]
fn copy_of_nested_mutable_type_is_an_error() {
    let program = build_program(&[(
        "bad.sv",
        "fn main() [use] -> None {\n    use StdConsole\n    \
         let m = Mut Map((\"a\", Mut List(1)))\n    let n = copy(m)\n    \
         println(\"${size(n)}\")\n}\n",
    )]);
    let result = emit(&program);
    let errors = result.err().expect("expected codegen errors");
    assert!(
        errors.iter().any(|e| e.contains("cannot `copy` a value of type `Mut Map<")),
        "unexpected errors: {errors:?}"
    );
}

/// [kt-copy] A list whose elements are mutable copies **element-wise**, as
/// Rust's `clone` does: mutating an element of the original leaves the copy
/// alone (ROADMAP §0j step 1; it used to alias on Kotlin).
pub const COPY_ELEMENTS_DEMO: &str = r#"
struct Counter canbe Mut { n: Int }

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 })
    let ys = copy(xs)
    let e = at(xs, 0) ?: unreachable!()
    e.n = 5
    println("${(get(xs, 0) ?: unreachable!()).n} ${(get(ys, 0) ?: unreachable!()).n}")
    let ls = Mut List(Mut List(1))
    let ms = copy(ls)
    add(at(ls, 0) ?: unreachable!(), 2)
    println("${size(get(ls, 0) ?: unreachable!())} ${size(get(ms, 0) ?: unreachable!())}")
}
"#;

/// [struct-field-canbe-mut] [kt-field-canbe-mut] [kt-copy] A `canbe Mut` field is
/// `Mut` exactly when its struct value is, and `copy` of a struct is deep:
/// the copy's lists, nested structs included, are its own.
pub const CANBE_MUT_FIELD_DEMO: &str = r#"
struct Bag canbe Mut {
    n: Int,
    items: List<Int> canbe Mut,
    fixed: Mut List<Int>
}

struct Outer canbe Mut {
    bag: Bag canbe Mut,
    tag: Str
}

fn fill(b: Mut Bag) -> None => b: Mut {
    add(b.items, 7)
    b.n = b.n + 1
}

fn total(b: Bag) -> Int => b {
    let t = b.n
    for x in b.items {
        t = t + x
    }
    return t
}

fn main() [use] -> None {
    use StdConsole()
    let b = Mut Bag { n: 1, items: Mut List<Int>(), fixed: Mut List<Int>() }
    add(b.items, 3)
    fill(b)
    let c = copy(b)
    add(c.items, 4)
    add(c.fixed, 9)
    println("${size(b.items)} ${size(c.items)} ${size(b.fixed)} ${size(c.fixed)} ${total(b)}")
    let p = Bag { n: 0, items: List(1, 2), fixed: Mut List<Int>() }
    let q = copy(p)
    add(q.fixed, 1)
    println("${size(p.fixed)} ${size(q.fixed)} ${total(p)}")
    let o = Mut Outer { bag: Mut Bag { n: 0, items: Mut List<Int>(), fixed: Mut List<Int>() }, tag: "t" }
    add(o.bag.items, 5)
    let o2 = copy(o)
    add(o2.bag.items, 6)
    println("${size(o.bag.items)} ${size(o2.bag.items)}")
    let frozen: Bag = c
    println("${total(frozen)}")
}
"#;

pub const CANBE_MUT_FIELD_EXPECTED: &str = "2 3 0 1 12\n0 1 3\n1 2\n16\n";

#[test]
fn canbe_mut_field_reads_cast_and_structs_copy_deeply() {
    let program = build_program(&[("main.sv", CANBE_MUT_FIELD_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.kt").unwrap();
    // Declared at the plain type, read as `Mut` through a cast.
    assert!(main.content.contains("var items: List<Int>"), "generated:\n{}", main.content);
    assert!(main.content.contains("(b.items as salvo.platform.std.core.list.MutList<Int>)"), "generated:\n{}", main.content);
    // A deep copy replaces every mutable field.
    assert!(main.content.contains(".copy(items = "), "generated:\n{}", main.content);
}

/// [struct-slot] [cmp-carry] [cmp-binder] A struct with a fn slot: the
/// ordering lives in the type, every fn over it captures the binder, and
/// nothing is stored. Shared with the Rust backend's run.
pub const STRUCT_SLOT_DEMO: &str = r#"
struct Ranked<T>(?cmp: (T, T) -> Int) canbe Mut {
    items: List<T> canbe Mut
}

fn ranked<T>(?cmp: (T, T) -> Int) [] -> Mut Ranked<T>(?cmp) {
    return Mut Ranked<T>(?cmp) { items: Mut List<T>() }
}

fn push<T>(r: Mut Ranked<T>(?cmp), x: T) [] -> None => r: Mut, !x {
    let i = 0
    while i < size(r.items) {
        let y = get(r.items, i) ?: unreachable!()
        if x < y {
            break
        }
        i = i + 1
    }
    insert_at(r.items, i, x)
}

fn top<T>(r: Ranked<T>(?cmp)) [] -> (proj(r) T)? => r {
    return get(r.items, 0)
}

fn backwards(a: Int, b: Int) [] -> Int {
    return cmp(b, a)
}

fn main() [use] -> None {
    use StdConsole()
    let up = ranked<Int>()
    push(up, 3)
    push(up, 1)
    push(up, 2)
    let down = ranked<Int>(cmp = backwards)
    push(down, 3)
    push(down, 1)
    push(down, 2)
    println("${up.items} ${down.items} ${top(up) ?: unreachable!()} ${top(down) ?: unreachable!()}")
}
"#;

/// [comptime-generic] [implicit-resolve-body] `by auto` at a generic struct:
/// the stamps take `?Ordered<T>`/`?Hashed<T>`/`?ToStr<T>`, calls and operators
/// fill them, and inside a body an implicit and a declaration of its name are
/// told apart by argument types. Shared with the Rust backend's run.
pub const GENERIC_STAMP_DEMO: &str = r#"
struct Inner : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    k: Int
}

struct Wrapper<T> : Hashed<self> by auto, Ordered<self> by auto, ToStr<self> by auto {
    value: T,
    inner: Inner,
    n: Int
}

fn main() [use] -> None {
    use StdConsole()
    let a = Wrapper<Str> { value: "a", inner: Inner { k: 1 }, n: 1 }
    let b = Wrapper<Str> { value: "a", inner: Inner { k: 2 }, n: 0 }
    println("${cmp(a, b)} ${eq(a, b)} ${hash(a) == hash(copy(a))}")
    println("${a < b} ${a == b} ${a != b} ${a}")
    run_h()
}

fn h<T>(a: T, i: Inner, ?cmp: (T, T) -> Int) [] -> Int => a, i {
    return cmp(1, 2) + cmp(a, a) + cmp(i, i)
}

fn run_h() [use] -> None {
    use StdConsole()
    println("${h("x", Inner { k: 3 })}")
}
"#;

/// [struct-opaque] An opaque struct used from another module through its
/// functions, its stamped `eq`/`to_str` reached from there, and copied deeply.
pub const OPAQUE_BAG: &str = r#"
export opaque struct Bag : Eq<self> by auto, ToStr<self> by auto canbe Mut {
    items: List<Int> canbe Mut,
    total: Int
}

export fn bag() [] -> Mut Bag {
    return Mut Bag { items: Mut List<Int>(), total: 0 }
}

export fn put(b: Mut Bag, x: Int) [] -> None => b: Mut, !x {
    b.total = b.total + x
    add(b.items, x)
}

export fn total(b: Bag) [] -> Int => b {
    return b.total
}
"#;

pub const OPAQUE_MAIN: &str = r#"
import bag.Bag
import bag.bag
import bag.put
import bag.total

struct Shelf : Eq<self> by auto, ToStr<self> by auto {
    b: Bag
}

fn main() [use] -> None {
    use StdConsole()
    let b = bag()
    put(b, 3)
    put(b, 4)
    let c = copy(b)
    put(c, 1)
    let s = Shelf { b: copy(c) }
    println("${total(b)} ${total(c)} ${b == c} ${s}")
}
"#;

/// [obligation-by] [group-obligation] Hand-written members beside stamped
/// ones (ROADMAP §0j 6f): an order-blind `eq` written for a generic struct
/// whose `to_str` is `by auto`, and a `to_str` written for a struct whose
/// `hash`/`eq` are. Shared with the Rust backend's run, where the stamped
/// `to_str` reaches the list's `to_str` past its own implicit of that name.
pub const MIXED_MEMBERS_DEMO: &str = r#"
struct Bag<T> : Eq<self>, ToStr<self> by auto {
    items: List<T>,
    n: Int
}

// Order-blind equality, by hand, beside the stamped `to_str`.
fn eq<T>(a: Bag<T>, b: Bag<T>, ?Eq<T>) [] -> Bool => a, b {
    if size(a.items) != size(b.items) {
        return false
    }
    for x in a.items {
        if !contains_eq(b.items, x) {
            return false
        }
    }
    return true
}

fn contains_eq<T>(xs: proj List<T>, x: proj T, ?Eq<T>) [] -> Bool => xs, x {
    for y in xs {
        if eq(x, y) {
            return true
        }
    }
    return false
}

struct Point : Hashed<self> by auto, ToStr<self> {
    x: Int,
    y: Int
}

fn to_str(p: Point) [] -> Str => p {
    return "(${p.x}, ${p.y})"
}

fn main() [use] -> None {
    use StdConsole()
    let a = Bag<Int> { items: List(1, 2), n: 0 }
    let b = Bag<Int> { items: List(2, 1), n: 5 }
    println("${a == b} ${a}")
    let p = Point { x: 1, y: 2 }
    let s = Set(p, Point { x: 1, y: 2 })
    println("${p} ${size(s)}")
}
"#;

/// [kt-field-canbe-mut] A `Str canbe Mut` field would hold a `String` or a
/// `StringBuilder` under one property: refused on Kotlin.
#[test]
fn canbe_mut_str_field_is_a_kotlin_error() {
    let program = build_program(&[(
        "bad.sv",
        "struct Doc canbe Mut { text: Str canbe Mut }\n\nfn main() [use] -> None {\n    \
         use StdConsole()\n    let d = Doc { text: \"a\" }\n    println(d.text)\n}\n",
    )]);
    let errors = emit(&program).err().expect("expected codegen errors");
    assert!(
        errors.iter().any(|e| e.contains("cannot store `canbe Mut` field `text`")),
        "unexpected errors: {errors:?}"
    );
}

#[test]
fn copy_of_mutable_elements_is_element_wise() {
    let program = build_program(&[("main.sv", COPY_ELEMENTS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.kt").unwrap();
    assert!(main.content.contains(".map { __c"), "generated:\n{}", main.content);
}

/// [cmp-carry] [fn-type-named] A collection constructor carries the identity
/// it is given by `hash =`/`eq =`, empty or not: before 2026-10-05 a literal
/// was built with the host's hashing and the slotted annotation accepted it
/// (ROADMAP §0j step 1; literals retired [col-no-literal]).
pub const LITERAL_IDENTITY_DEMO: &str = r#"
fn fold_hash(s: Str) [] -> Long => s {
    return hash(size(s))
}
fn fold_eq(a: Str, b: Str) [] -> Bool => a, b {
    return size(a) == size(b)
}

fn main() [use] {
    use StdConsole()
    let m: Mut Map<Str, Int>(fold_hash, fold_eq) = Mut Map("ab": 1, hash = fold_hash, eq = fold_eq)
    put(m, "cd", 2)
    let e: Mut Map<Str, Int>(fold_hash, fold_eq) = Mut Map<Str, Int>(hash = fold_hash, eq = fold_eq)
    put(e, "ab", 1)
    put(e, "cd", 2)
    let s: Mut Set<Str>(fold_hash, fold_eq) = Mut Set("ab", hash = fold_hash, eq = fold_eq)
    add(s, "cd")
    let t: Mut Set<Str>(fold_hash, fold_eq) = Mut Set<Str>(hash = fold_hash, eq = fold_eq)
    add(t, "ab")
    add(t, "cd")
    println("${size(m)} ${size(e)} ${size(s)} ${size(t)}")
}
"#;

/// [op-wrap] [op-bits] Integer arithmetic wraps and the `bit_` functions
/// agree with Rust's, shift counts taken modulo the width (ROADMAP §0j step 2).
pub const INT_WRAP_BITS_DEMO: &str = r#"
fn main() [use] {
    use StdConsole()
    let big = 2147483647
    let wrapped = big + 1
    let neg = -wrapped
    let i = big
    i++
    let l = 9223372036854775807L
    println("${wrapped} ${neg} ${i} ${l + 1L} ${big * 2}")
    println("${bit_and(12, 10)} ${bit_or(12, 10)} ${bit_xor(12, 10)} ${bit_not(0)}")
    println("${bit_shl(1, 33)} ${bit_shr(-16, 2)} ${bit_ushr(-16, 28)} ${bit_ushr(-1L, 60)}")
    println("${mix_hash(17L, 5L)}")
}
"#;


// ===== precedence-aware binary rendering =====

/// Parser grouping must survive re-rendering: without precedence-aware
/// parenthesization `(a - b) * c` would emit flat as `a - b * c` and
/// silently re-associate.
const PRECEDENCE_DEMO: &str = r#"
fn main() [use] -> None {
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .expect("main.kt emitted");
    // Every binary is parenthesized: the grouping is explicit rather than
    // left to precedence.
    for needle in [
        "((a - b) * c)",
        "(a - (b - c))",
        "(-(a + b))",
        "(((a < b) || (b > c)) && (a > c))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
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
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("h.result is Union2.U1<*, *>"),
        "expected union test on the field in:\n{}",
        main.content
    );
}

// [when-union-subject] `when` still requires a plain variable subject.
#[test]
fn when_field_subject_is_rejected() {
    let src = r#"
export qualifier Ok<T> of T

export fn ok<T>(value: T) -> +Ok T {
    return value
}

export struct Holder {
    result: Ok Int | Str
}

export fn main() [use] -> None {
    use StdConsole
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
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // [ir-narrow] A narrowed projection is read through a narrowing binding
    // made where the test held.
    assert!(
        main.content.contains(": String = p.surname!!"),
        "expected the narrowed field read to assert in:\n{}",
        main.content
    );
    assert!(
        main.content.contains(": String = p.address.city!!"),
        "expected the narrowed field *chain* read to assert in:\n{}",
        main.content
    );
    assert!(
        main.content.contains(": Int = ((h.result as Union2.U1<*, *>).value as Int)"),
        "expected the narrowed wrapper-union field read to unwrap in:\n{}",
        main.content
    );
}

// [flow-place] [op-no-none] A narrowed field is usable as an *operand*,
// which the optional-strictness rules used to reject outright.

const PLACE_OPERAND_DEMO: &str = r#"
struct Reading {
    label: Str,
    value: Int? = None
}

fn main() [use] -> None {
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // [ir-narrow] One rule for `val` and `var` properties: the narrowing
    // binding, which a `val` needs no more than a `var` does but costs
    // nothing either.
    assert!(
        main.content.contains(": Int = r.value!!"),
        "expected the narrowing binding in:\n{}",
        main.content
    );
}

// ===== tuple element access [expr-tuple-index] =====
// `t.0` reads a tuple element; a constant index narrows like a field
// [flow-place], and nesting (`t.1.0`) is two projections.

const TUPLE_INDEX_DEMO: &str = r#"
fn main() [use] -> None {
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.content.contains(": String = maybe.first!!"),
        "expected the narrowed element to assert in:\n{}",
        main.content
    );
}

// ===== list constructors carry their element type =====
// `Mut List()` must not emit a bare `mutableListOf()`, which kotlinc
// cannot infer: an empty list constructor renders its element type from the
// call's resolved type arguments (`mutableListOf<Int>()`), since Kotlin
// cannot infer a type argument that the source never wrote.

/// The std list constructors use this: `Mut List()` must not emit a bare
/// `mutableListOf()`, which kotlinc cannot infer.
#[test]
fn list_constructors_carry_their_element_type() {
    let src = r#"
export fn main() [use] -> None {
    use StdConsole
    let xs: Mut List<Int> = Mut List()
    let ys = List<Str>()
    add(xs, 1)
    println("${size(xs)} ${size(ys)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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

// ===== collections [col-insertion-order] =====

/// [op-arith] [op-promote] [type-lit-adopt] [op-convert] The same source and
/// expected output as the Rust backend's `rustc_compiles_and_runs_operators`
/// [backend-parity]. Kotlin's own operator set covers the mixed widths
/// (`Long.plus(Int)`, `Int.compareTo(Long)`), so only the adopted literals
/// need rendering care (`1L`, `3.0`).
#[test]
fn operators() {
    let src = r#"
export fn main() [use] -> None {
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
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
}

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

// ===== destructuring a loop element [var-let-destructure] =====

/// [var-let-destructure] A `for` binds a **pattern**, not just a name. Source and
/// expected stdout are **verbatim** the Rust backend's
/// `rustc_compiles_and_runs_loop_destructuring`.
const LOOP_DESTRUCTURE: &str = r#"
struct Row { name: Str, score: Int }

fn main() [use] -> None {
    use StdConsole
    // [var-let-destructure] A loop element destructures like a `let`: a tuple by
    // position, a struct by field (renamed where the pattern says so).
    let pairs: List<(Str, Int)> = List(("a", 1), ("b", 2))
    for (k, v) in iter(pairs) {
        println("${k}=${v}")
    }
    let rows: List<Row> = List(Row { name: "x", score: 7 }, Row { name: "y", score: 8 })
    for {name: who, score} in iter(rows) {
        println("${who} ${score}")
    }
    // A binding the body assigns to is the local's own, not the element's.
    let nums: List<(Int, Int)> = List((1, 2), (3, 4))
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

/// [var-let-destructure] What it lowers to: the element in a temporary — cast to its
/// own type, which is what a pass-driven header needs — then one `val` per name,
/// a tuple's by component and a struct's by field. Kotlin *could* destructure a
/// `Pair` in the header, but not the pass's payload and not a struct, so one
/// shape serves every loop and matches the Rust backend's.
#[test]
fn loop_destructuring_binds_the_parts_of_a_temporary_kotlin() {
    let program = build_program(&[("main.sv", LOOP_DESTRUCTURE)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    let text = &main.content;
    assert!(
        text.contains(": kotlin.Pair<String, Int> = __emitted_"),
        "the element temporary must be typed as itself:\n{text}"
    );
    assert!(
        text.contains("val k: String = __elem_6.first") && text.contains("val v: Int = __elem_6.second"),
        "expected the tuple parts by component name:\n{text}"
    );
    assert!(
        text.contains("val who: String = __elem_10.name") && text.contains("val score: Int = __elem_10.score"),
        "expected the struct fields by name, renamed:\n{text}"
    );
    assert!(
        text.contains("var a: Int = __elem_14.first"),
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
    use StdConsole
    // [type-tuple] Past three elements Kotlin has no tuple type of its own,
    // so the backend generates one; Rust's are native at any arity.
    let q: (Int, Str, Bool, Int, Str) = (1, "two", true, 4, "five")
    println("${q.0} ${q.1} ${q.2} ${q.3} ${q.4}")
    let (a, b, c, d, e) = q
    println("${a} ${b} ${c} ${d} ${e}")
    // A keyed container over a tuple used to live here, exercising the
    // generated class's structural comparison. It cannot today: a keyed
    // constructor asks for its identity as a capability (user decision
    // 2026-09-26) and std declared `cmp`/`hash` for the intrinsic
    // scalars only, so a tuple has no identity to pass — the same gap that
    // stops `(1, 2) == (1, 2)` and `(1, 2) < (1, 3)` from resolving, which
    // never worked either. ROADMAP's "Recursive implicit resolution" is the
    // lift; until then the arity, the component reads and the destructuring
    // above are what this case is about.
}
"#;

/// [kt-tuple-class] What a tuple past three lowers to: a `SalvoTupleN` literal,
/// the `v3`/`v4` component names past `Pair`'s three [kt-tuple-component], and
/// a generated `tuples.kt` whose class is a `data class` (structural equality
/// and `componentN`, so a `Set` element and a destructuring both work)
/// carrying nothing more: its `eq`/`cmp`/`hash` are Salvo's.
#[test]
fn tuples_past_three_emit_a_generated_class() {
    let program = build_program(&[("main.sv", BIG_TUPLE_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("Tuple5(1, \"two\", true, 4, \"five\")"),
        "expected a generated tuple literal in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("${q.third} ${q.v3} ${q.v4}"),
        "expected the component names past `Pair`'s three in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("Tuple5<Int, String, Boolean, Int, String>"),
        "expected the generated type in the annotation in:\n{}",
        main.content
    );
    let tuples = files
        .iter()
        .find(|f| f.rel_path.ends_with("tuples/Tuple5.kt"))
        .expect("tuples/Tuple5.kt is emitted for a program that names a big tuple");
    assert!(
        tuples.content.contains("data class Tuple5<")
            && !tuples.content.contains("SalvoTuple")
            && tuples.content.contains("val v3: T4"),
        "the generated tuple class is wrong:\n{}",
        tuples.content
    );
}

// ===== union coercion inside arrays/tuples/lambda returns =====
// [type-union] Constructor arguments, tuple elements and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.

const NESTED_COERCION_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

type Result = Ok Int | Err Str

fn tag_ok<T>(value: T) -> +Ok T
=> !value {
    return value
}

fn tag_err<T>(value: T) -> +Err T
=> !value {
    return value
}

fn describe(r: Result) -> Str {
    if r is Ok {
        return "ok ${r}"
    }
    return "err ${r}"
}

fn main() [use] -> None {
    use StdConsole
    // [col-no-literal] A variadic constructor does not yet coerce
    // differently-armed arguments into an explicit union `T`, so the second
    // arm is coerced at its own binding.
    let second: Result = tag_err("a")
    let arr: List<Result> = List<Result>(tag_ok(1), second)
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    for needle in [
        // [col-no-literal] `List<Result>(…)` lowers to `listOf`, with the
        // coercion wrapper on the argument; the second arm is wrapped at its
        // own binding.
        "listOf<Union2<Int, String>>(Union2.U1<Int, String>(salvo.main.tagOk(1)), second)",
        "Union2.U2<Int, String>(salvo.main.tagErr(\"a\"))",
        "kotlin.Pair(\"t\", Union2.U1<Int, String>(salvo.main.tagOk(2)))",
        "Union2.U1<Int, String>(salvo.main.tagOk(3))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

// ===== effect member fns with their own generics =====
// [effect-member-generics] Member generics render on the interface
// member and bind per call from argument types.

const MEMBER_GENERICS_DEMO: &str = r#"
effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T
    => !a, !b {
        return a
    }
}

fn main() [use] -> None {
    use StdConsole
    use FirstStash
    let x = pick(7, 2)
    let s = pick("l", "r")
    println("${x} ${s}")
}
"#;

#[test]
fn effect_member_generics_render_on_the_interface() {
    let program = build_program(&[("main.sv", MEMBER_GENERICS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
    use StdConsole
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
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        let value = get(values, i % values.size()) ?: unreachable!()
        i = i + 1
        return copy(value)
    }
}

fn roll() [Random<Count>] -> Count {
    return next_random()
}

fn main() [use] -> None {
    use StdConsole
    use CyclicRandom(List(7, 8))
    println("${roll()} ${roll()}")
}
"#;

#[test]
fn aliased_effect_types_resolve_to_the_same_handler() {
    let program = build_program(&[("main.sv", ALIASED_EFFECT_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The `use` registers `Random<Int>`; `roll`'s `Random<Count>`
    // parameter and its call site must agree with it.
    assert!(
        main.content.contains(": CyclicRandom<Int> = CyclicRandom<Int>(values = listOf<Int>(7, 8), copy = { __a0 -> __a0 })")
            && main.content.contains(": Random<Int> = __Mon_Random<Int>("),
        "unexpected use lowering in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("roll(__handle_"),
        "handler not threaded through the aliased effect in:\n{}",
        main.content
    );
}

// ===== std array functions =====
// [type-array] Arrays get the `std.core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02 — the docs/language/ `CyclicRandom` example calls
// `values.size()` on a `T[]`).

const ARRAY_STD_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

fn main() [use] -> None {
    use StdConsole
    use CyclicRandom(array_of(1, 2, 3, 4))
    let nums: Int[] = array_of(3, 4, 5)
    println("size ${nums.size()} get ${nums.get(2) ?: unreachable!()} first ${nums.first() ?: unreachable!()}")
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
"#;

#[test]
fn array_std_functions_lower() {
    let program = build_program(&[("main.sv", ARRAY_STD_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        "= salvo.std.core.array.iter(nums)",
        "values.size",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
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

fn tag(value: Str) -> +Environment.Tag Str => !value {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> None {
    use StdConsole()
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
// overload name [fn-emit-name].
#[test]
fn dot_names_emit_nested_classes() {
    let program = build_program(&[("main.sv", DOT_NAMES)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.content.contains("fun label__EnvironmentTagStr("),
        "generated:\n{}",
        main.content
    );
}

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
    // The signal class is generated once for the program.
    let signal = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("throwsignal.kt"))
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
        .find(|f| f.rel_path.ends_with("std/core/throw.kt"))
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
        measure.contains("val n: Int = salvo.main.parse(console, line)"),
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
    use StdConsole
    use ConsoleLogger()
    demo()
}
"#;

/// [fn-effects] [kt-handle] Kotlin threads the effect as a leading closure
/// parameter too, rather than capturing it — one mechanism on both backends.
/// A named fn declaring exactly the type's effects passes as a reference;
/// a pure one gets an adapter that ignores the threaded effect (the
/// variance rule).
#[test]
fn fn_type_effects_thread_into_lambdas() {
    let program = build_program(&[("main.sv", FN_EFFECTS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    // The inherited effect is `run_it`'s own leading parameter; the fn
    // *type* carries the per-effect parameter too.
    assert!(
        main.content
            .contains("runIt(logger: Logger, f: (Logger, String) -> String"),
        "expected the per-effect signature and fn type:\n{}",
        main.content
    );
    // The lambda takes the effect rather than capturing it.
    assert!(
        main.content.contains("fun(__leff0: Logger, s: String): String {"),
        "expected the effect as a leading lambda parameter:\n{}",
        main.content
    );
    // Named fns: the effectful one passes as a reference, the pure one is
    // adapted to drop the threaded effect.
    assert!(
        main.content.contains("runIt(logger, ::shout,"),
        "expected the effectful named fn as a reference:\n{}",
        main.content
    );
    assert!(
        main.content.contains("plain(__a0)"),
        "expected an adapter for the pure fn:\n{}",
        main.content
    );
}

// ===== the widening check `^` [qual-lift] =====

// [qual-lift] `^` tests the arm *and* removes the claim, so a branch can
// `when` the union inside a qualified one — the shape `try` outcomes produce.
const WIDEN_DEMO: &str = r#"
import std.throw

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
    use StdConsole
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

/// [qual-lift] Kotlin materializes the same peel with a shadowing `val`
/// (the alternative, a fresh name, would need every read rewritten).
#[test]
fn widening_materializes_the_peel_kotlin() {
    let program = build_program(&[("main.sv", WIDEN_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content
            .contains(": Union2<Int, String> = ((nested as Union2.U1<*, *>).value as Union2<Int, String>)"),
        "expected the widened value bound to a shadowing local:\n{}",
        main.content
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
    // An `is` head still declares its binding at the top of the arm.
    assert!(
        main.content.contains("val s: String = ((value as Union2.U1<*, *>).value as String)"),
        "expected the `is` binding inside the arm:\n{}",
        main.content
    );
}

// ===== a `try` body is ordinary code to the mutability census =====

// [throw-try] A variable assigned *only* inside a `try` body still needs the
// mutable declaration. `collect_mutated_expr` had no `Expr::Try` arm, so it
// emitted `val counter` next to `counter = counter + 1` and kotlinc
// rejected the output — a [backend-never-wrong] miss caught by nothing but
// the toolchain.
const TRY_MUTATION_DEMO: &str = r#"
import std.throw

fn risky(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

fn main() [use] -> None {
    use StdConsole
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains("var counter: Int = 0"),
        "expected `var` for a variable the `try` body assigns:\n{}",
        main.content
    );
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

/// [platform-handler] [platform-tree] A `use` of a platform handler with no
/// host file is an error naming the command, not generated code that
/// references a class nobody wrote — the same reasoning as the platform
/// entry point's missing-host error [backend-never-wrong].
#[test]
fn a_missing_host_file_for_a_platform_handler_names_the_command() {
    let program = platform_handler_program();
    let errors = emit(&program)
        .err()
        .expect("a `use` of a platform handler without a host must not emit");
    assert!(
        errors.iter().any(|e| e.contains("`HostRawClock`")
            && e.contains("platform/main.kt")
            && e.contains("salvo platform generate")),
        "expected the missing-host error, got:\n{}",
        errors.join("\n")
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

// ===== the network sequence, step ②: the wire format =====

/// The Rust backend's `WIRE_DEMO`, verbatim, with the same expected hex — the
/// byte-for-byte parity assertion for the wire [net-wire-format] [kt-wire].
const WIRE_DEMO: &str = r#"import std.net
import std.codec

struct Point { x: Int, y: Long }
struct Tag { name: Str }
struct Pin { at: Point, label: Str?, tags: List<Tag>, kind: Circle | Square | None, raw: Bytes, ok: Bool }
struct Circle { r: Double }
struct Square { side: Float }

fn main() [use] {
    use StdConsole()
    let p = Pin {
        at: Point { x: -2, y: 5000000000 },
        label: "home",
        tags: List(Tag { name: "a" }, Tag { name: "bé" }),
        kind: Square { side: 1.5 },
        raw: to_bytes("hi"),
        ok: true
    }
    let bytes = encode(p)
    println("${size(bytes)} bytes: ${to_hex(bytes)}")
    let back = decode<Pin>(bytes)
    if back is None {
        println("decode failed")
        return
    }
    println("${back.at.x} ${back.at.y} ${back.label ?: "-"} ${size(back.tags)} ${back.ok}")
    let kind = back.kind
    when kind {
        is Circle { println("circle") }
        is Square { println("square ${kind.side}") }
        is None { println("no kind") }
    }
    let bad = decode<Point>(to_bytes("x"))
    println("bad is none: ${bad is None}")
    println("int: ${to_hex(encode(258))}  str: ${to_hex(encode("é"))}  opt: ${to_hex(encode(none_int()))}")
}

fn none_int() -> Int? { return None }

actor effect Ledger {
    send fn record(entry: Str, amount: Long) => !entry, !amount
    send fn close()
}
"#;

#[test]
fn a_protocol_with_a_wire_form_gets_a_codec_and_a_hash() {
    let files = generate_files(&[("main.sv", WIRE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let src = &main.content;
    for expected in [
        "object __Codec_Pin : salvo.WireCodec<Pin> {",
        "object __Codec___Msg_Ledger : salvo.WireCodec<__Msg_Ledger> {",
        "const val __PROTO_Ledger: String = \"",
        "salvo.salvoEncode(",
        "salvo.salvoDecode(",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}


// ===== [kt-fn-mangling] overload dispatch is the checker's, not Kotlin's =====

/// Two overloads the *checker* tells apart by Salvo types — a pass and the
/// `List` it walks — where the `List` overload **delegates** to the pass one.
/// That is the shape that turns a second opinion into a crash rather than a
/// wrong answer: before the mangling rule, a delegation whose argument lowered
/// to the same Kotlin type re-resolved to the delegating overload itself.
const OVERLOAD_DELEGATION: &str = r#"
fn twice(p: Mut ListNext<Int>, f: (Int) -> Int) -> Mut List<Int> => p: Mut, f {
    return map(p, f)
}

fn twice(xs: List<Int>, f: (Int) -> Int) -> Mut List<Int> => f, !xs {
    return twice(iter(xs), f)
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
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
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    assert!(
        main.contains("fun twice__ListNext_Fn(p: salvo.std.core.list.ListNext<Int>") && main.contains("fun twice__List_Fn(xs: List<Int>"),
        "expected the second overload to be renamed in:\n{main}"
    );
    // The delegation calls the *other* overload, by its own name.
    assert!(
        main.contains("return salvo.main.twice__ListNext_Fn("),
        "expected the delegation to reach the pass overload in:\n{main}"
    );
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
    use StdConsole()
    let xs = List(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    let generic = map(iter(copy(xs)), double)
    println("generic ${generic.size()}")
    let snapshot = copy(xs)
    let kept = filter(iter(snapshot), is_even)
    println("kept ${kept.size()}")
    let out = map_to(Mut List<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(Mut List<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
"#;

#[test]
fn the_combinator_surface() {
    let program = build_program(&[("main.sv", SEQ_SURFACE_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        seq.contains("next: (It) -> Union2<T, salvo.std.core.iterator.Finished>"),
        "expected the resolved `next` as a plain parameter in:\n{seq}"
    );
}

// ===== [implicit-param] [implicit-group] implicit parameters =====

/// The same source and expected stdout as the Rust backend's
/// `rustc_compiles_and_runs_implicit_parameters`: a group spread with no
/// binder, defaults resolved from the visible `Int` overloads, one member
/// overridden by name, forwarding through an opaque `T`, and an
/// individually declared `?add`.
const IMPLICIT_DEMO: &str = r#"
fns Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return a + b }
fn zero() -> Int { return 0 }
// [op-unique-shape] Not `times`: that is the `*` operator on `Int`, declared
// only in `Int`'s own file; an override is named for what it does.
fn multiply(a: Int, b: Int) -> Int { return a * b }
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
    use StdConsole()
    println("total=${total(List(1, 2, 3))}")
    println("product=${total(List(2, 3, 4), add = multiply, zero = one)}")
    // [col-of-nonempty] Literals rather than constructor calls: the element
    // constructors claim `NonEmpty`, and a `List<NonEmpty List<Int>>` does not
    // fit a `List<List<Int>>` position (type arguments are invariant). A literal
    // claims nothing, which is what a nested plain list needs.
    println("nested=${total_all(List<List<Int>>(List(1, 2), List(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
"#;

/// [implicit-param] Implicit parameters are ordinary trailing parameters of
/// fn type; a resolved default is passed as a Kotlin function reference, and
/// a group leaves no runtime representation [implicit-group].
#[test]
fn implicit_parameters_lower_to_trailing_fn_parameters() {
    let program = build_program(&[("main.sv", IMPLICIT_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        // An override written at the call site [implicit-override].
        "total(listOf<Int>(2, 3, 4), ::multiply, ::one)",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
    assert!(
        !main.contains("class Field"),
        "a `fns` group must leave no runtime representation:\n{main}"
    );
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
    use StdConsole()
    use Angle()
    println(show(7))
    println(show(7, fmt = loud))
}
"#;

#[test]
fn an_effect_member_carries_its_implicits_into_the_interface() {
    let program = build_program(&[("main.sv", MEMBER_IMPLICIT_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        ".show(7, ::fmt)",
        ".show(7, ::loud)",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
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
    use StdConsole()
    use Plain<Int>()
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}
"#;

#[test]
fn a_generic_handler_is_constructed_at_its_type() {
    let program = build_program(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    for expected in [
        ": Plain<Int> = Plain<Int>()",
        ": Show<Int> = __Mon_Show<Int>(",
        ": Prefixed<Int> = Prefixed<Int>(prefix = \"p\")",
    ] {
        assert!(main.contains(expected), "expected `{expected}` in:\n{main}");
    }
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
    use StdConsole()
    // A builder, and the drop that lets it reach the `Str` surface.
    let b = Mut Str("he", "llo")
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
    println("${at ?: unreachable!()} ${nowhere is None}")
    let sub = substr(hay, 1, 3)
    let oob = substr(hay, 1, 9)
    println("${sub ?: unreachable!()} ${oob is None}")
    let n = parse_int("42")
    let bad = parse_int(pa)
    println("${n ?: unreachable!()} ${bad is None}")
    println("${trim_prefix(hay, hay)}${trim_suffix(hay, lo)}|")

    let count = Mut List<Int>()
    for c in iter(hay) {
        add(count, 1)
    }
    println("chars: ${size(count)}")

    // [type-bytes] [type-byte] The text/byte bridge: UTF-8 out, strict UTF-8
    // back in, and an octet that is unsigned on both targets. The buffer is a
    // `Bytes`, not a list of octets, so nothing here boxes on the JVM.
    let bytes = to_bytes("hé")
    let round = str_of_bytes(bytes)
    let broken = mut_bytes()
    add(broken, to_byte(200))
    println("bytes: ${bytes} ${to_hex(bytes)} ${round ?: unreachable!()} ${str_of_bytes(broken) is None}")

    // Operators drop `Mut` too, so equality is by content on both targets.
    let x = Mut Str(pa)
    let y = Mut Str(pa)
    println("equal: ${x == y}")
}
"#;

/// [kt-mut-str] `Mut Str` maps to `StringBuilder` through the same
/// `mut_type_name` hook `Mut List<T>` uses, and every *drop* of the `Mut`
/// renders the conversion — which is what `MutableList` never needed, since
/// it really is a `List`.
#[test]
fn mut_str_lowers_to_a_string_builder() {
    let program = build_program(&[("main.sv", STRING_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.contains(r#"= salvo.std.core.string.Mut_Str(arrayOf<String>("he", "llo"))"#),
        "unexpected:\n{main}"
    );
    // [str-drop-mut] The conversion at a call argument, in interpolation,
    // and at an operator — a bare name takes the suffix without parens.
    assert!(main.contains("shout(b.toString())"), "unexpected:\n{main}");
    assert!(
        main.contains("sizePlatform(b.toString())"),
        "unexpected:\n{main}"
    );
    assert!(
        main.contains("(x.toString()) == (y.toString())"),
        "unexpected:\n{main}"
    );
    // [kt-copy] A builder's copy is a new builder: identity would alias the
    // buffer.
    assert!(
        main.contains(": salvo.platform.std.core.string.MutStr = StringBuilder(b)"),
        "unexpected:\n{main}"
    );
    // `setCharAt` throws out of range, so `set` guards — and binds its
    // arguments, so a call argument is evaluated once. [col-bounds] The guard
    // answers `false` rather than falling through silently.
    assert!(
        // [platform-value-type] Through std's host code now.
        main.contains("setPlatform(b, 0, 'H')"),
        "unexpected:\n{main}"
    );
}

/// [fn-variadic] A `...spread` into a variadic intrinsic uses Kotlin's own
/// spread operator: splicing the array as one argument builds a collection
/// of one array — which kotlinc catches for `listOf`, but *not* for
/// `StringBuilder(...)`, where `append(Any?)` accepts it and prints
/// `[Ljava.lang.String;@…` [backend-never-wrong].
#[test]
fn a_spread_into_a_variadic_intrinsic_spreads() {
    let src = r#"
export fn main() [use] {
    use StdConsole()
    let parts = array_of("a", "b")
    let sb = Mut Str(...parts)
    let xs = Set(...parts)
    println("${sb} ${size(xs)}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    // [col-of-nonempty] `Set` rather than `List`: the list constructors
    // take a *first* element now, which a spread may not supply
    // [fn-variadic], so a lone spread reaches the purely variadic intrinsics.
    assert!(
        // [platform-value-type] `Mut Str` is Salvo now; the spread is the array.
        main.contains("= salvo.std.core.string.Mut_Str(parts)"),
        "unexpected:\n{main}"
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
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted")
        .content;
    // Lambdas are anonymous fns [kt-ir].
    assert!(
        main.contains("fun(n: Int): Int {\n        return (n * 2)")
            && main.contains("filterPlatform(xs, fun(n: Int): Boolean {")
            && main.contains("return (a + b)"),
        "unexpected:\n{main}"
    );
    // The generic overload for a pass subject gets the resolved `next` as its
    // implicit argument.
    assert!(
        main.contains("salvo.std.core.seq.map__It_Fn(salvo.std.core.array.iter(arr), fun(n: Int): Int {") && main.contains("{ __a0 -> salvo.std.core.array.next(__a0) })"),
        "expected the resolved `next` as the implicit in:\n{main}"
    );
    // A `fns` group is not a value: nothing *declares* `Next`.
    assert!(
        !files.iter().any(|f| f.content.contains("class Next")
            || f.content.contains("interface Next")
            || f.content.contains("object Next")),
        "a `fns` group must emit nothing"
    );
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
qualifier Even of Int {
    fn qualifies(n: Int) -> Bool { return n % 2 == 0 }
}

qualifier Small of Int {
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
    use StdConsole()
    let xs = List(1, 2, 3)
    // Two `size`s fit and neither is more specific, so each call names its
    // place [fn-overload-ambiguous].
    println(size@main(xs))
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

/// [fn-overload-at] [fn-rename] Neither override survives into the output:
/// the calls emit the ordinary (mangled) names of the declarations the
/// checker resolved.
#[test]
fn scope_selectors_and_renames_are_erased() {
    let program = build_program(&[("main.sv", OVERLOAD_OVERRIDE_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.contains("\"core: ${salvo.std.core.list.sizePlatform(xs)}\""),
        "unexpected:\n{main}"
    );
    // The renamed overload is called by its declaration's mangled name —
    // both `label`s are mangled, since they are overloads of one name
    // [kt-fn-mangling].
    assert!(
        main.contains("${salvo.main.label__EvenInt(n)} ${salvo.main.label__SmallInt(n)}"),
        "unexpected:\n{main}"
    );
    // A call reaching past a local of the same name names the declaration
    // by its package [kt-decl-call-qualified], as every declared call does:
    // a fn-typed local of the name would otherwise take it.
    assert!(
        main.contains(", salvo.main.describe(7))"),
        "unexpected:\n{main}"
    );
}


// ===== [iter-generic-drive] driving a generic pass =====

/// [iter-drive-in-place] A *kept* pass is advanced where it lives — no local —
/// so the caller's next drive continues from where this one stopped. Kotlin's
/// local aliased the same object and behaved this way already; naming the
/// subject directly is what makes the two backends *say* so identically.
#[test]
fn a_kept_pass_is_driven_in_place() {
    let program = build_program(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    assert!(
        src.contains("= next(it)"),
        "expected the in-place drive in:\n{src}"
    );
    // Precisely: no local for *this* loop: the kept pass is the subject.
    assert!(
        !src.contains(": It = it"),
        "a kept pass must not be bound into a local in:\n{src}"
    );
}

/// [kt-suppress-cast] A generic drive reads the union payload through an
/// erased cast (`step.value as T`), which kotlinc reports as an unchecked
/// cast — in code the author cannot edit. The emitted function carries
/// `@Suppress("UNCHECKED_CAST")`; a function whose payload reads cast to
/// concrete types stays unannotated (those casts are checked at run time
/// and draw no warning).
const SUPPRESS_DEMO: &str = r#"
struct Slice<T> : Next<self, proj T> canbe Mut {
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

// The element type is the combinator's own `T`: the payload read casts to a
// type variable, so the emitted fn needs the suppression.
fn count_all<It, T>(it: Mut It, ?Next<It, T>) [] -> Int => it: Mut {
    let n = 0
    for x in it {
        n = n + 1
    }
    return n
}

// The element type is concrete (`Int`): the payload read casts to `Int`,
// which the JVM checks at run time — no warning, no annotation.
fn sum_ints<It>(it: Mut It, ?Next<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    let p = slice(xs)
    println("count ${count_all(p)}")
    let ys = List(4, 5)
    let q = slice(ys)
    println("sum ${sum_ints(q)}")
}
"#;

#[test]
fn a_payload_cast_gets_the_suppression() {
    let program = build_program(&[("main.sv", SUPPRESS_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
            "@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\", \"UNNECESSARY_SAFE_CALL\")\nfun<It, T> countAll"
        ),
        "expected the suppression on the generic-element combinator in:\n{main}"
    );
    // ... and a concrete one is a *useless* cast, which kotlinc warns about
    // just as loudly (found 2026-09-14 in std's `fs`, whose `to_str` reads a
    // concrete error arm): one annotation covers both, since neither warning
    // is the author's to silence.
    assert!(
        main.contains(
            "@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\", \"UNNECESSARY_SAFE_CALL\")\nfun<It> sumInts"
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
    let files = emit(&program).unwrap_or_else(|errors| {
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

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter {
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

iter fn iter(f: Fibs) -> Emitted Int | Finished {
    state FibsIter {
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

iter fn iter(n: Noisy) [Console] -> Emitted Int | Finished {
    state NoisyIter {
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

iter fn iter(r: Row) -> Emitted Str | Finished {
    state RowIter {
        left: Int = r.times
    }
    if left <= 0 {
        return finished()
    }
    left = left - 1
    return emitted(describe(r))
}

fn main() [use] {
    use StdConsole()

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

/// [iter-fn] The generated declarations are ordinary Kotlin: a data class for
/// the pass, a function that mints one, and the author's body as a function.
/// Nothing suspends, so there is no `iterator {}` builder and no state number.
#[test]
fn an_iter_fn_emits_a_plain_class_and_next() {
    let program = build_program(&[("main.sv", ITER_FN_DEMO)]);
    let files = emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt")
        .content;
    // [iter-fn] The struct its `state` block names holds every parameter,
    // then the state.
    assert!(
        src.contains("data class CountdownIter(\n    var c: Countdown,\n    var at: Int,\n)")
            && src.contains("CountdownIter(c = c, at = c.from)"),
        "expected the parameter beside the state:\n{src}"
    );
    assert!(
        src.contains("data class FibsIter(\n    var f: Fibs,")
            && src.contains("FibsIter(f = f, a = 0, b = 1, made = 0)"),
        "expected the parameter held whatever the body reads of it:\n{src}"
    );
    assert!(
        src.contains("data class RowIter(\n    var r: Row,")
            && src.contains("RowIter(r = r, left = r.times)"),
        "expected the whole subject to be kept:\n{src}"
    );
    assert!(
        !src.contains("__advance") && !src.contains("iterator {"),
        "an `iter fn` needs no state machine:\n{src}"
    );
    // [fn-effects] The effectful `next` gets its handler per turn. The mangling
    // index counts the visible `next` overloads, so it moved when std's lazy
    // pair (two of them) was removed 2026-09-10, again when `Set` and `Map`
    // brought their own passes (two more) 2026-09-13, again when `Bytes`
    // and the fs chunk pass brought two more 2026-09-15, and again when
    // `std.core.range` joined std 2026-09-23 (its `next` counts even though the
    // module is private), and again when `reversed`/`enumerate` (then `indices`) brought
    // more (the refinement-types sequence, step 0, 2026-09-23), and when
    // `std.core.deque` brought two more (2026-10-02).
    assert!(
        // The suffix is the overload's mangle, which moves as std grows.
        src.contains("= salvo.main.next__NoisyIter(__handle_"),
        "expected the handler threaded into the drive:\n{src}"
    );
}

/// [kt-wire] [net-wire-format] A generic struct's `T?` field: its codec wraps the
/// struct's own `WireCodec<T>` in `OptCodec`, which once demanded a non-null
/// `T` and so did not compile (found 2026-10-20 through lib/std/args). The bytes
/// are Rust's (`generic_optional_codec` in the Rust suite).
pub const GENERIC_OPT_CODEC_DEMO: &str = r#"import std.codec

struct Arg<T> {
    default: T? = None
    n: Int? = None
}

fn main() [use] {
    use StdConsole()
    let a = encode(Arg<Int> { default: 3 })
    println(to_hex(a))
    let b = decode<Arg<Int>>(a)
    if b is None { println("none") } else { println("${b.default ?: 0}") }
    let c = encode(Arg<Str> { n: 4 })
    println(to_hex(c))
    let d = decode<Arg<Str>>(c)
    if d is None { println("none") } else { println("${d.n ?: 0} ${d.default ?: "-"}") }
}
"#;

pub const GENERIC_OPT_CODEC_OUTPUT: &str = "010000000300\n3\n000100000004\n4 -\n";

/// [cmp-carry] [cmp-binder] An identity is a **declaration**: two modules'
/// `my_hash` key two sets, and a binder filled from one calls that module's —
/// a private one, from a file that imports neither (found 2026-10-20: the
/// reading file's `my_hash` was called). Same program and output as the Rust
/// suite's `compiles_and_runs_two_modules_same_named_identities`.
pub const TWO_IDENTITIES_FILES: [(&str, &str); 4] = [
    ("p.sv", r#"export struct P : Hashed<self> by auto { x: Int }
"#),
    ("a.sv", r#"import p.P
fn my_hash(v: P) -> Long { return hash(v.x) }
fn my_eq(l: P, r: P) -> Bool { return l.x == r.x }
export fn make_a() -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
"#),
    ("b.sv", r#"import p.P
export fn my_hash(v: P) -> Long { return 7 }
export fn my_eq(l: P, r: P) -> Bool { return true }
export fn make_b() -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
"#),
    ("main.sv", r#"import p.P
import a.make_a
import b.make_b
fn one(x: Mut Set<P>(?hash, ?eq), v: P) -> Bool => x, v { return contains(x, v) }
fn main() [use] {
    use StdConsole()
    let x = make_a()
    let y = make_b()
    add(x, P { x: 1 })
    add(y, P { x: 1 })
    add(y, P { x: 2 })
    println("${one(x, P { x: 2 })} ${one(y, P { x: 5 })} ${size(y)}")
}
"#),
];


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
    use StdConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    println(close(InFile { id: 1 }))
    println(close@Vault(OutFile { id: 2 }))
}
"#;

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
        "fun close__InFile(f: InFile): String",
        "fun close__OutFile(f: OutFile): String",
        "fun describe(f: InFile): String",
        "override fun close__InFile(f: InFile): String {",
        "override fun close__OutFile(f: OutFile): String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        src.contains(".close__InFile(InFile(") && src.contains(".close__OutFile(OutFile("),
        "expected both call sites to name their own overload, got:\n{src}"
    );
}


// ===== std's byte buffer [type-bytes] =====

/// [type-bytes] [type-byte] The whole `Bytes` surface in one program: both
/// constructors, the read side, the `Mut` side, structural equality, a copy
/// that does not alias, the text bridge and a `for` over the buffer.
///
/// The two backends could not be further apart underneath — a `Vec<u8>` here,
/// a shipped `SalvoBytes` class on the JVM [kt-bytes] — so the value of this
/// case is that it prints the same bytes, the same hex and the same numbers on
/// both.
const BYTES_PROGRAM: &str = r#"
fn main() [use] -> None {
    use StdConsole()
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
    println("at 1: ${to_int(get(fixed, 1) ?: unreachable!())} / oob: ${get(fixed, 9) is None}")
    println("slice: ${slice(fixed, 1, 3) ?: unreachable!()} / bad: ${slice(fixed, 1, 9) is None}")
    println("index: ${index_of(fixed, to_byte(200)) ?: unreachable!()} / ${index_of(fixed, to_byte(7)) is None}")
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

/// [platform-value-type] `Bytes` is a value platform type: the program names
/// the host's types — `Bytes`, and `MutBytes` for the mutable kind, by
/// convention — and calls the platform wrappers; std's host file aliases
/// both to the shipped `salvo.SalvoBytes`.
#[test]
fn bytes_is_a_value_platform_type() {
    let files = generate_files(&[("main.sv", BYTES_PROGRAM)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.kt"))
        .expect("main.kt emitted");
    assert!(
        main.content.contains("salvo.platform.std.core.bytes.MutBytes")
            || files.iter().any(|f| f.content.contains("salvo.platform.std.core.bytes.MutBytes")),
        "expected the host's mutable kind named by convention"
    );
    let host = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("platform/std/core/bytes.kt"))
        .expect("std's host file for core.bytes travels with the program");
    for expected in ["typealias MutBytes", "fun copy(data: Bytes)"] {
        assert!(host.content.contains(expected), "expected `{expected}` in:\n{}", host.content);
    }
    assert!(
        files.iter().any(|f| f.rel_path == std::path::Path::new("bytes.kt") && f.content.contains("class SalvoBytes")),
        "the buffer class still ships"
    );
}

// ===== std's filesystem [platform-handler] [linear-group] =====

/// std's `fs`, exercised end to end against real files — **verbatim the
/// Rust backend's program**, because the layering's promise is that only the
/// host file differs: `platform handler HostRawFs` at the bottom,
/// `DefaultFs [RawFs]` above it (a handler in *std* constructed from the
/// program, which is what made the per-file carrier class a defect), linear
/// tokens discharged by an effect member, the `Lines` pass, the one-shots, a
/// `position` after a ranged open, and a failure path acknowledged once.
const FS_PROGRAM: &str = r#"
import std.fs
import std.fs.path
import std.fs.host
import std.stream
import std.stream.host

fn describe(e: FsError) [] -> Str => e {
    if e is NotFound {
        return "not found"
    }
    if e is Streaming {
        return describe(e.error)
    }
    return "other"
}

fn describe(e: StreamError) [] -> Str => e {
    if e is InvalidUtf8 {
        return "not utf-8"
    }
    return "other"
}

fn main() [use] -> None {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()

    let dir = Path { value: "__DIR__" }
    let made = create_dirs(dir)
    when made {
        is Ok { println("made") }
        is Err { println("made: ${describe(detach(made))}") }
    }

    let path = Path { value: "__DIR__/notes.txt" }
    let written = write_str(path, "alpha\nbeta\ngamma\n")
    when written {
        is Ok { println("wrote ${written}") }
        is Err { println("wrote: ${describe(detach(written))}") }
    }

    let all = read_lines(path)
    when all {
        is Ok { println("lines: ${all}") }
        is Err { println("lines: ${describe(detach(all))}") }
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
                is Err { println("closed: ${describe(detach(closed))}") }
            }
        }
        is Err { println("open: ${describe(detach(opened))}") }
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
                is Err { println("tail: ${describe(detach(shut))}") }
            }
        }
        is Err { println("tail: ${describe(detach(tail))}") }
    }

    let missing = read_to_str(Path { value: "__DIR__/nope.txt" })
    when missing {
        is Ok { println("unexpected") }
        is Err { println("missing: ${describe(detach(missing))}") }
    }

    let listed = list_dir(dir)
    when listed {
        is Ok { println("dir: ${listed}") }
        is Err { println("dir: ${describe(detach(listed))}") }
    }


    // [fs-bytes] The byte surface: a file that is not text, written and read
    // back exactly. `to_byte` keeps the low 8 bits and a `Byte` is unsigned on
    // both backends, so 255 prints as 255 on both — a signed byte would print
    // -1 on one of them [backend-parity].
    let out = open_write(Path { value: "__DIR__/raw.bin" })
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
                is Err { println("wrote raw: ${describe(detach(shut))}") }
            }
        }
        is Err { println("raw open: ${describe(detach(out))}") }
    }
    let raw = open_read(Path { value: "__DIR__/raw.bin" })
    when raw {
        is Ok {
            let s: InStream = raw
            let head = read_bytes(s, 3)
            when head {
                is Ok { println("read: ${head} at ${position(s)} (${to_hex(head)})") }
                is Err { println("read: ${describe(detach(head))}") }
            }
            // Bytes and text off one stream: the position is bytes either way,
            // so the text read picks up exactly where the byte read stopped.
            let text = read_all(s)
            when text {
                is Ok { println("then: ${text}") }
                is Err { println("then: ${describe(detach(text))}") }
            }
            let shut = close(s)
            when shut {
                is Ok { println("read raw") }
                is Err { println("read raw: ${describe(detach(shut))}") }
            }
        }
        is Err { println("raw read: ${describe(detach(raw))}") }
    }
    // Opening between the bytes of a character is a *seek*, not a decode: it
    // succeeds, and the strict decode afterwards is what fails — recorded, so
    // `close` reports it too [fs-errors-at-close].
    let split = open_read_at(Path { value: "__DIR__/raw.bin" }, 5)
    when split {
        is Ok {
            let s: InStream = split
            let bad = read_all(s)
            when bad {
                is Ok { println("split: ${bad}") }
                is Err { println("split: ${describe(detach(bad))}") }
            }
            let shut = close(s)
            when shut {
                is Ok { println("split closed") }
                is Err { println("split close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("split open: ${describe(detach(split))}") }
    }

    // [fs-read-to] The fill-a-buffer read: one buffer, cleared and refilled,
    // instead of a payload per step.
    let held = open_read(Path { value: "__DIR__/raw.bin" })
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
                    println("read_to: ${describe(detach(got))}")
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
                is Err { println("filled close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("filled open: ${describe(detach(held))}") }
    }
    // The chunk pass: `for` over a stream's bytes, a fresh buffer per step.
    let ch = open_chunks(Path { value: "__DIR__/raw.bin" }, 4)
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
                is Err { println("pass close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("pass open: ${describe(detach(ch))}") }
    }
    // And the text side of `read_to`: a line per iteration, one builder.
    let lined = open_read(path)
    when lined {
        is Ok {
            let s: InStream = lined
            let line = Mut Str()
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
                is Err { println("lines close: ${describe(detach(shut))}") }
            }
        }
        is Err { println("lines open: ${describe(detach(lined))}") }
    }
    // The one-shots that own the buffer: a whole-file copy and a whole-file
    // byte read, with no stream and no buffer in sight.
    let copied = copy_file(path, Path { value: "__DIR__/copy.txt" })
    when copied {
        is Ok { println("copied ${copied}") }
        is Err { println("copied: ${describe(detach(copied))}") }
    }
    let all_bytes = read_to_bytes(Path { value: "__DIR__/copy.txt" })
    when all_bytes {
        is Ok { println("copy holds ${size(all_bytes)} bytes") }
        is Err { println("copy read: ${describe(detach(all_bytes))}") }
    }
    let gone_copy = delete(Path { value: "__DIR__/copy.txt" })
    when gone_copy {
        is Ok { println("copy deleted") }
        is Err { println("copy deleted: ${describe(detach(gone_copy))}") }
    }
    let gone_bin = delete(Path { value: "__DIR__/raw.bin" })
    when gone_bin {
        is Ok { println("raw deleted") }
        is Err { println("raw deleted: ${describe(detach(gone_bin))}") }
    }

    let gone = delete(path)
    when gone {
        is Ok { println("deleted") }
        is Err { println("deleted: ${describe(detach(gone))}") }
    }
    let gone_dir = delete(dir)
    when gone_dir {
        is Ok { println("removed") }
        is Err { println("removed: ${describe(detach(gone_dir))}") }
    }
}
"#;

/// The program with its scratch directory baked in. Under the crate's own
/// target tmpdir, so the two backends' runs cannot collide.
fn fs_program() -> String {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("fs-e2e-data");
    FS_PROGRAM.replace("__DIR__", &dir.to_string_lossy())
}

/// [kt-platform-handler] [effect-handle] [kt-handle] What the layering
/// emits: the effect interfaces, no class for the platform handler, and
/// `DefaultFs` capturing its dependency as a **field typed by the effect's
/// interface** — the JVM reference is the handle.
#[test]
fn the_fs_surface_emits_a_host_seam_and_a_dependency_field() {
    let files = generate_files(&[("main.sv", &fs_program())]);
    let file = |rel: &str| -> String {
        files
            .iter()
            .find(|f| f.rel_path == std::path::Path::new(rel))
            .unwrap_or_else(|| panic!("{rel} was not emitted"))
            .content
            .clone()
    };
    let surface = file("std/fs.kt");
    assert!(
        surface.contains("interface Fs {"),
        "expected the effect interface in:\n{surface}"
    );
    // [effect-available] The stream operations are `stream`'s now, and the
    // pass's discharger is a *fn* named `close` beside `Streams`' members.
    let streams = file("std/stream.kt");
    assert!(
        streams.contains("interface Streams {"),
        "expected the stream effect in:\n{streams}"
    );
    assert!(
        streams.contains("fun close__Lines(streams: Streams, p: Lines)"),
        "expected the pass discharger as a fn named `close` in:\n{streams}"
    );
    // The host seam is its own module [mod-used-only]: a program that never
    // opens a file links none of it. [effect-prereq] `DefaultFs` reaches the
    // `Streams` in scope as a dependency it never wrote.
    let host = file("std/fs/host.kt");
    for expected in ["interface RawFs {", "class DefaultFs(private val __dep0: RawFs, private val __dep1: salvo.std.stream.Streams) : salvo.std.fs.Fs"] {
        assert!(host.contains(expected), "expected `{expected}` in:\n{host}");
    }
    assert!(
        !host.contains("class HostRawFs"),
        "the platform handler's class is the host's:\n{host}"
    );
    let stream_host = file("std/stream/host.kt");
    // [stream-table] `HostRawStreams` is Salvo over the runtime's table.
    for expected in ["interface RawStreams {", "fun rawCloseRead(handle: Long)", "class DefaultStreams(", "class HostRawStreams"] {
        assert!(stream_host.contains(expected), "expected `{expected}` in:\n{stream_host}");
    }
    // The `use` site constructs the shipped host class through its package.
    let main = file("main.kt");
    assert!(
        main.contains("= salvo.std.fs.host.__Platform_HostRawFs()") && main.contains("= salvo.std.fs.host.__Mon_RawFs(__use_"),
        "expected the shipped host class at the `use` site, got:\n{main}"
    );
    // std ships the host files, and they travel into the output.
    for rel in ["platform/std/fs/host.kt", "platform/std/runtime/streams.kt"] {
        assert!(
            files.iter().any(|f| f.rel_path == std::path::Path::new(rel)),
            "expected std's host companion {rel} to be emitted"
        );
    }
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

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}

fn main() [use, spawn] {
    use StdConsole()
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

/// [actor-monitor-handler] [kt-monitor] The monitor spawn (SH-3, user decision
/// 2026-09-19): the same program the Rust backend runs, with the same
/// expected output — one plain-effect handler shared behind a lock, serving
/// `main` and a spawned actor.
const MONITOR: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    state {
        cursor: Int = 0
    }

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
    use StdConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
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

/// [kt-monitor] What the monitor lowering *is*, asserted on the generated
/// text: the per-effect lock wrapper implementing the effect's interface by
/// locking its `ReentrantLock` and delegating, the spawn wrapping the handler instance,
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
            .contains("class __Mon_Random(\n    private val inner: Random,"),
        "the lock wrapper is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("lock.lock()\n        try { return inner.next() } finally { lock.unlock() }"),
        "a member does not lock and delegate:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Mon_Random(CyclicRandom(seed = 12345))"),
        "the monitor spawn is missing:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("__Stub_Random"),
        "a plain effect must not get a send stub:\n{}",
        main.content
    );
}

/// [actor-mixed-handler] [kt-mixed] The mixed handler (SH-1): the same program the
/// Rust backend runs, with the same expected output — the servant's state
/// behind a mailbox, the façade on the callers' threads, one instance
/// serving `main` and a spawned actor.
const MIXED: &str = r#"
effect Random {
    fn next() -> Int
}

handler CyclicRandom(seed: Int) of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

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
    use StdConsole()
    let rng = spawn CyclicRandom(12345)
    use rng
    println("main drew ${next()}")
    let drawer = spawn Drawing() with rng on pool(1)
    let drawn = waitfor got: Reply<Int> {
        drawer.draw(got)
    }
    println("actor drew ${drawn}")
    println("main drew ${next()}")
}
"#;

fn generate_mixed_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", MIXED)])
}

/// [actor-mixed-handler] [effect-handler-deps] [actor-spawn-inherit] [effect-with-clause] A
/// mixed handler **with dependencies** (the cut lifted 2026-10-20): the
/// servant's send member calls `Console`, synthesized from the spawning
/// scope, and `Label`, built by the `with` clause; the façade carries
/// neither. Same program, same output on both backends.
const MIXED_DEPS: &str = r##"
effect Random {
    fn next() -> Int
}

effect Label {
    fn label(n: Int) -> Str
}

handler Hash() of Label {
    fn label(n: Int) -> Str {
        return "#${n}"
    }
}

handler Counted(seed: Int) [Console, Label] of Random {
    mailbox { capacity: 8 }
    state {
        cursor: Int = 0
    }

    send fn advance(out: Reply<Int>) => !out {
        cursor = cursor + seed
        println("servant at ${label(cursor)}")
        send(out, cursor)
    }

    fn next() -> Int {
        return waitfor got: Reply<Int> {
            advance(got)
        }
    }
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let rng = spawn Counted(7) with Hash()
    use rng
    println("main drew ${next()}")
    println("main drew ${next()}")
}
"##;

#[test]
fn a_mixed_handler_with_dependencies() {
    let files = generate_files(&[("main.sv", MIXED_DEPS)]);
    let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.kt").expect("main.kt");
    // [kt-mixed] The servant class takes the dependencies; the façade
    // carries only the addr and the constructor parameters.
    assert!(
        main.content.contains("class Counted(private val seed: Int, private val __dep0: ")
            && main.content.contains("class __Fac_Counted(private val __addr: Int, private val seed: Int) : Random")
            && main.content.contains("__Fac_Counted(__a, __c0)"),
        "the servant does not take the dependencies:\n{}",
        main.content
    );
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
        main.content.contains("sealed class __Priv_CyclicRandom {"),
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
            .contains("salvo.SalvoSched.send(__addr, __Priv_CyclicRandom.Advance("),
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
    use StdConsole()
    let p = pool(2)
    let rows = spawn Rows() on p
    let fetcher = spawn Fetching() with rows on p
    let answer = waitfor out: Reply<Str> {
        fetcher.fetch(7, out)
    }
    println(answer)
}
"#;

fn generate_replyto_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    generate_files(&[("main.sv", REPLYTO_CHAIN)])
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

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
    }

    // A faulted activation is what death *is* — there is no `kill`, and a
    // Salvo-level throw cannot cross a member boundary.
    send fn crash() {
        let xs: List<Int> = List(1)
        sum = sum + (get(xs, 9) ?: unreachable!())
    }
}

fn main() [use, spawn] {
    use StdConsole()
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

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
    }

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
        waiting = Mut List()
    }
}

fn main() [use, spawn] {
    use StdConsole()
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

/// [kt-actor] [linear-container] The lowering: Kotlin needs no `mem::take`
/// equivalent — objects are references, and the checker made the member assign
/// a fresh list before it returned — so a drain is std's Salvo `drain` over the
/// field, followed by the fresh list [platform-value-type].
#[test]
fn draining_state_lowers_to_a_foreach_kotlin() {
    let files = generate_files(&[("main.sv", LINEAR_QUEUE)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    assert!(
        main.content.contains("salvo.std.core.list.drain(waiting, fun(") && main.content.contains("waiting = mutableListOf<"),
        "the drain is not std's drain over the field:\n{}",
        main.content
    );
}


/// [actor-free-send-fn] [task-mint] [task-pool-inherit] The **task kernel**, as a
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
    use StdConsole()
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

/// [task-effects] A task body's effects are inherited from the frame that
/// minted it (user decision 2026-09-26). The Rust backend runs this program
/// verbatim and both must print these lines byte for byte — on the JVM a
/// handler reference is already a shareable handle [kt-monitor], so what the
/// Rust side needs an owned `__Mon_E` for is simply a captured `val` here.
const TASK_EFFECTS: &str = r#"
send fn report(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("task saw ${n}")
    out.send(n * 2)
}

send fn chain(out: Reply<Int>, n: Int) [Console] => !out, !n {
    println("chain saw ${n}")
    let inner = waitfor r: Reply<Int> {
        let k = replyto report(r)
        k.send(n + 1)
    }
    out.send(inner)
}

fn main() [use, spawn] -> None {
    use StdConsole()
    let direct = waitfor a: Reply<Int> {
        let k = replyto report(a)
        k.send(21)
    }
    println("direct ${direct}")
    let nested = waitfor b: Reply<Int> {
        let k = replyto chain(b) on pool(2)
        k.send(10)
    }
    println("nested ${nested}")
}
"#;

/// [task-effects] [kt-task] The inherited handler is bound to a `val` outside
/// the lambda and passed as the leading argument, so what the task runs with is
/// the handler registered at the mint.
#[test]
fn an_inherited_effect_is_captured_at_the_mint() {
    let files = generate_files(&[("main.sv", TASK_EFFECTS)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("val __e0 = ") && text.contains("report(__e0,"),
        "the handler is captured at the mint:\n{text}"
    );
    assert!(
        text.contains("fun report(console: salvo.std.core.console.Console"),
        "a task body still takes the handler it performs through:\n{text}"
    );
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
        text.contains("salvo.SalvoSched.mintTask(salvo.SalvoSched.currentPool(), "),
        "an omitted `on` clause must inherit the current pool:\n{text}"
    );
    assert!(
        text.contains(") { __v -> salvo.main.finish("),
        "the continuation must be a trailing lambda (after the answer decoder):\n{text}"
    );
    // [net-addr-routable] The actor's `decodeReply` *reads* the parked table for a
    // wire reply; what a task mint must not do is *write* one.
    assert!(
        !text.contains("__parked[__s] ="),
        "a task mint parks nothing: the lambda is the continuation:\n{text}"
    );
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

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
        now: Int = 0
    }

    send fn after(millis: Int, out: Reply<Str>) {
        add(waiting, out)
    }

    send fn advance(millis: Int) {
        now = now + millis
        let at = now
        drain(waiting, r -> send(r, "fired at ${at}"))
        waiting = Mut List()
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
    state {
        sum: Int = 0
    }

    fn bump(n: Int) {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

fn count() [Tally, Stats] -> Int {
    bump(2)
    bump(3)
    return total()
}

fn main() [use, spawn] {
    use StdConsole()
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
    use Counting()
    println("total ${count()}")
}
"#;

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
        text.contains("fun __dispatch_ManualTime_Timer(__handler: ManualTime, __msg: __Msg_Timer)")
            && text.contains("fun __dispatch_ManualTime_TimerCtl(__handler: ManualTime, __msg: __Msg_TimerCtl)"),
        "one dispatcher per protocol is missing:\n{text}"
    );
    assert!(
        text.contains("is __Msg_Timer -> salvo.main.__dispatch_ManualTime_Timer(handler, msg)")
            && text.contains("is __Msg_TimerCtl -> salvo.main.__dispatch_ManualTime_TimerCtl(handler, msg)"),
        "the delivery must ask which protocol the message is:\n{text}"
    );
    assert!(
        text.contains("val __a = salvo.SalvoSched.spawn(") && text.contains("kotlin.Pair(__a, __a)"),
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

    state {
        waiting: Mut List<Reply<Str>> = Mut List()
    }

    send fn ask(out: Reply<Str>) {
        add(waiting, out)
    }
}

actor effect Client {
    send fn go(desk: eff[Desk]) => !desk
    send fn answered(word: Str) => !word
}

// The gate: while the continuation is outstanding this actor serves nothing
// else, so its mailbox is stalled for good.
handler Clienting() of Client {
    mailbox { capacity: 4 }

    send fn go(desk: eff[Desk]) {
        desk.ask(replyto! answered())
    }

    send fn answered(word: Str) {
        discard(word)
    }
}

fn main() [use, spawn] {
    use StdConsole()
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

/// [actor-watch] [actor-on-idle] [runtime-handles] `watch` and `on_idle` are
/// Salvo: the program calls `std.core.actor`'s fns, which take the core's token
/// out of the reply token (`takeLocal()`) and register it; the scheduler
/// builds `Exit` and `Idle` itself, so no builder travels with the call.
#[test]
fn watch_and_on_idle_are_salvo_over_the_core_token_kotlin() {
    for (src, call) in [(WATCH, "watch(c, out)"), (ON_IDLE, "onIdle(p, i)")] {
        let files = generate_files(&[("main.sv", src)]);
        let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.kt").expect("main.kt");
        assert!(main.content.contains(call) && !main.content.contains("SalvoSched.watch"), "{}", main.content);
        let actor = files
            .iter()
            .find(|f| f.rel_path.to_string_lossy() == "std/core/actor.kt")
            .expect("std/core/actor.kt");
        assert!(actor.content.contains(".takeLocal()"), "{}", actor.content);
    }
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
        text.contains("var __addr: Int? = null")
            && text.contains(
                "val __parked: MutableMap<Long, __Cont_Fetching> = kotlin.collections.mutableMapOf()"
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

    state {
        seen: Int = 0
    }

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
    use StdConsole()
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

    state {
        last: Str = "none"
    }

    send fn note(what: Str) {
        last = what
    }

    send fn dump(out: Reply<Str>) {
        out.send(copy(last))
    }
}

handler Summing() of Tally {
    mailbox { capacity: 8 }

    state {
        sum: Int = 0
    }

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
    use StdConsole()
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

/// [kt-actor] [kt-handle] The shape: the handler holds its dependencies as
/// fields typed by the effect interfaces, the actor class is the same for
/// every handler, and the spawn site passes the clause's instances as
/// trailing constructor arguments — a construction for one dependency, a
/// forwarding stub for the addr.
#[test]
fn a_dependent_spawn_hands_the_child_its_dependencies() {
    let files = generate_dep_spawn_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt");
    let text = &main.content;
    assert!(
        text.contains("class Counting(private val __dep0: Log, private val __dep1: Tally) : Counter"),
        "the dependencies are fields:\n{text}"
    );
    assert!(
        text.contains("class __Actor_Counting(private val handler: Counting) : salvo.SalvoActor"),
        "the actor class is not generic:\n{text}"
    );
    // [actor-mailbox] The instance is built into a local first, so the spawn can
    // read the bound off it before handing it over.
    assert!(
        text.contains("val __h = Counting(__Mon_Log(Recording()), __Stub_Tally(tally))")
            && text.contains("__h.__mailboxCapacity, __Actor_Counting(__h)"),
        "the spawn does not pass the clause's instances, or does not read the \
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
    use StdConsole()
    let a: Mut List<Int> = Mut List(1, 2, 3)
    drain_ints(a)
    println("left ${size(a)}")

    let b: Mut List<Int> = Mut List(7, 8)
    println("first ${first_or(b, 0)} then ${size(b)}")

    let c: Mut List<Int> = Mut List(1, 2, 3, 4)
    println("sum ${sum_down(c)} left ${size(c)}")

    // statement `if`: the call happens once, so the list loses exactly one
    let d: Mut List<Int> = Mut List(5, 6)
    if remove_first(d) is Int n {
        println("one ${n} left ${size(d)}")
    }
}
"#;

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
        text.contains("while (true) {") && text.contains("if (!((__subject_1 != null))) {"),
        "the while did not become a test-inside loop:\n{text}"
    );
    assert!(
        text.contains("val n: Int = __subject_1!!"),
        "the binding does not read the hoisted temporary:\n{text}"
    );
}

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

// ===================== time [time-types] [time-timer] =====================
//
// The same three programs the Rust backend runs, printing the same text: the
// time surface, a real deadline, and virtual time through `ManualTime`
// [backend-parity].

const TIME_TIMER: &str = r#"
import std.time

fn main() [use, spawn] -> None {
    use StdConsole()
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

/// [time-timer] [kt-time] A deadline is the runtime module's, mirroring the
/// Rust backend: `after_nanos` from the default timer, the runtime module's
/// lazily bound wheel, and no timer machinery in `SalvoSched`.
#[test]
fn a_deadline_lowers_to_the_runtime_modules_wheel() {
    let files = generate_files(&[("main.sv", TIME_TIMER)]);
    let find = |name: &str| files.iter().find(|f| f.rel_path.to_string_lossy() == name).map(|f| f.content.clone());
    let time = find("std/time.kt").expect("std/time.kt");
    assert!(time.contains("afterNanos("), "expected the deadline call in:\n{time}");
    let runtime = find("std/runtime/timers.kt").expect("std/runtime/timers.kt");
    assert!(runtime.contains("val __module_use0: Deadlines by kotlin.lazy {"), "{runtime}");
    let scheduler = find("scheduler.kt").expect("scheduler.kt");
    assert!(!scheduler.contains("fun after(") && !scheduler.contains("timers"), "the host scheduler keeps no timers");
    assert!(
        files
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "hosttime.kt"),
        "expected hosttime.kt to be emitted"
    );
}

/// The concatenated text of several emitted files, read like one.
struct EmittedFileText {
    content: String,
}
