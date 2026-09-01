//! Kotlin codegen tests: golden snapshots of generated code, and (when
//! `kotlinc` is on PATH) a full compile-and-run verification.

use std::path::Path;
use std::process::Command;

use salvo_core::{Program, SourceSet};

/// The demo program exercising M2 features: structs, defaults, spread/copy,
/// nullability + `is` with binding, string interpolation, effects (Console),
/// `use`, iterator functions with `yield`, `for`/`while` loops, defines.
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
    println("size: ${mut_names.list_size()}")
}
"#;

/// `size` is already defined for Str, and M2 resolves overloads by arity
/// only (type-based overload resolution needs the typechecker), so the list
/// length helper gets its own name here.
const DEMO_DEFINES: &str = r#"
define fn list_size<T>(list: List<T>) -> Int {
    inline: ``
    ${list}.size
    ``
}

external fn list_size<T>(list: List<T>) -> Int
"#;

fn build_program(extra: &[(&str, &str, bool)]) -> Program {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kotlin", "kt", true);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, content, is_define) in extra {
        let rel = Path::new(name);
        let (module, kind) = SourceSet::classify(rel, "kotlin").unwrap();
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

fn generate_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[
        ("main.sv", DEMO, false),
        ("main.kotlin.sv", DEMO_DEFINES, true),
    ]);
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
        "fn main() [use] -> [] None {\n    println(\"no console handler used\")\n}\n",
        false,
    )]);
    let result = salvo_backend_kotlin::emit_program(&program);
    let errors = result.err().expect("expected codegen errors");
    assert!(
        errors.iter().any(|e| e.contains("no handler for effect `Console`")),
        "unexpected errors: {errors:?}"
    );
}

/// The M3 demo: Result-style unions with qualifiers, `when` exhaustiveness,
/// precise `is Err Str` checks, and flow narrowing. `ok`/`err` are
/// constructive-qualifier constructor functions (`-> T as Ok`).
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

fn generate_unions_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", UNIONS_DEMO, false)]);
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
    assert!(main.content.contains("return U2_2<Int, String>(err(\"negative age\"))"));
    // Qualifier-tagged wrap picks the right arm of the 3-union.
    assert!(main.content.contains("U3_1<String, String, Boolean>(ok(\"yes\"))"));
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
qualifier Ok<T> of T
qualifier Err<T> of T

fn f(x: Ok Int | Err Str) -> Int {
    let v = when x {
        is Ok {
            1
        }
    }
    return v
}
"#;
    let program = build_program(&[("bad.sv", src, false)]);
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
    let src = "fn f(x: Int) -> None {\n    when x {\n        is Int {\n            x\n        }\n    }\n}\n";
    let program = build_program(&[("bad.sv", src, false)]);
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
qualifier Ok<T> of T
qualifier Err<T> of T

fn err<T>(value: T) -> T as Err {
    return value
}

fn f() -> Ok Int | Err Str {
    return err(true)
}
"#;
    let program = build_program(&[("bad.sv", src, false)]);
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
#[test]
fn kotlinc_compiles_and_runs_unions() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let files = generate_unions_demo();
    let expected = "age 36\nerror: negative age\nok: yes\nvalue plus one is 37\n";
    run_kotlin_files(&files, "unions", expected);
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

fn generate_qualifiers_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", QUALIFIERS_DEMO, false)]);
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
    assert!(main.content.contains("fun Surname_qualifies(person: Person): Boolean"));
    assert!(main.content.contains("fun Positive_qualifies(int: Int): Boolean"));
    // `is` predicate checks call them.
    assert!(main.content.contains("if (Surname_qualifies(person))"));
    assert!(main.content.contains("if (Positive_qualifies(n))"));
    // The qualified overload is mangled; the checker routes the narrowed
    // call to it and the unqualified call to the base name.
    assert!(main.content.contains("fun full_name__Surname(person: Person): String"));
    assert!(main.content.contains("println(console, full_name__Surname(person))"));
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
#[test]
fn kotlinc_compiles_and_runs_qualifiers() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let files = generate_qualifiers_demo();
    let expected = "Roland Elliott\nAnon\n5 is positive\n-2 is not positive\n\
                    tick 3\ntick 2\ntick 1\ninner ok: yes\n";
    run_kotlin_files(&files, "qualifiers", expected);
}

/// Runs the checker on a source and returns the errors (panics if none).
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src, false)]);
    salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors")
}

// [qual-no-dup]
#[test]
fn duplicate_qualifier_is_rejected() {
    let errors = expect_errors(
        "qualifier Ok<T> of T\n\nfn f(x: Ok Ok Int) -> None {\n}\n",
    );
    assert!(
        errors.iter().any(|e| e.contains("applied more than once")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-with]
#[test]
fn incompatible_qualifiers_are_rejected() {
    let src = r#"
struct Person {
    name: Str
}

qualifier Old of Person
qualifier Surname of Person

fn f(p: Old Surname Person) -> None {
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
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

fn f(x: Positive Str) -> None {
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
        ("quals.sv", "qualifier Fancy of Int\n", false),
        (
            "other.sv",
            "import quals.Fancy\n\nfn make() -> Int as Fancy {\n    return 1\n}\n",
            false,
        ),
    ]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected type errors");
    assert!(
        errors.iter().any(|e| e.contains("must be declared in the same file")),
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    // The constructor is a plain fn after erasure; the call site resolves
    // to the Positive overload (mangled name).
    assert!(main.content.contains("fun make(): Int"), "content: {}", main.content);
    assert!(
        main.content.contains("describe__Positive(make())"),
        "content: {}",
        main.content
    );
}

// [qual-ctor-simple]
#[test]
fn constructor_return_type_must_be_simple() {
    let src = "qualifier Fancy of Int\n\nfn make() -> (Int | Str) as Fancy {\n    return 1\n}\n";
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("must return a simple type")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-constructive]
#[test]
fn constructive_qualifier_cannot_be_is_tested() {
    let src = "qualifier Fancy of Int\n\nfn f(x: Int) -> None {\n    if x is Fancy {\n    }\n}\n";
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
qualifier Weird of Int {
    fn qualifies(int: Int) -> Str {
        return "nope"
    }
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("`qualifies` must return `Bool`")),
        "unexpected errors: {errors:?}"
    );
}

// [qual-constructive] [qual-ctor-fn]
#[test]
fn constructive_values_only_come_from_constructors() {
    // Plain values never satisfy a constructive qualifier: the assignment
    // is a type error, so the only way in is the constructor fn.
    let src = "qualifier Fancy of Int\n\nfn f() -> None {\n    let x: Fancy Int = 1\n}\n";
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("expected `Fancy Int`, found `Int`")),
        "unexpected errors: {errors:?}"
    );
}

// [type-with-mut] `Mut List<T>` maps through the define's `Mut inline:`
// template; `Mut` on a type without `with Mut` is an error.
#[test]
fn mut_types_map_through_the_mut_inline_template() {
    let src = r#"
fn fill(target: Mut List<Int>, n: Int) -> [target: Mut] None {
    add(target, n)
}

fn main() [use] -> [] None {
    use StdOutConsole
    let items: Mut List<Int> = mutable_list(1)
    fill(items, 2)
    println("size: ${items.size()}")
}
"#;
    let program = build_program(&[("main.sv", src, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    assert!(
        main.content.contains("fun fill(target: MutableList<Int>, n: Int)"),
        "unexpected: {}",
        main.content
    );
    assert!(main.content.contains("val items: MutableList<Int> = mutableListOf(1)"));
}

// [type-with-mut] `Mut` only applies to declarations that say `with Mut`.
#[test]
fn mut_requires_a_with_mut_declaration() {
    let errors = expect_errors("fn f(x: Mut Str) -> None {\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("`Mut` does not apply to `Str`")),
        "unexpected errors: {errors:?}"
    );
}

/// Full verification: compile the generated Kotlin with kotlinc and run it,
/// checking the program output. Skipped when kotlinc is not installed.
#[test]
fn kotlinc_compiles_and_runs_demo() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let files = generate_demo();
    let expected = "Hello, Roland Elliott!\n  1: 37\n  2: 38\n  3: 39\n\
                    Hello, Roland!\n  1: 37\n  2: 38\n  3: 39\n\
                    first: a\nsize: 2\n";
    run_kotlin_files(&files, "demo", expected);
}

/// The M5 demo: generic effects with multiple instances in scope,
/// checker-driven disambiguation (explicit type args, expected type),
/// handler generic inference at `use` sites, and handler threading through
/// fn call sites — all resolved from the checker's effect tables.
const EFFECTS_DEMO: &str = r#"
effect Random<T> {
    fn next_random() -> T
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

fn generate_effects_demo() -> Vec<salvo_backend_kotlin::EmittedFile> {
    let program = build_program(&[("main.sv", EFFECTS_DEMO, false)]);
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
    // `use` registers the *concrete* effect instance: the handler's
    // generics are inferred from the constructor arguments.
    assert!(main
        .content
        .contains("val random_int: Random<Int> = CyclicRandom(listOf(10, 20, 30))"));
    assert!(main
        .content
        .contains("val random_string: Random<String> = CyclicRandom(listOf(\"a\", \"b\"))"));
    // Callee effect dependencies are threaded in declaration order.
    assert!(main.content.contains("draw(random_int, random_string, console)"));
    assert!(main.content.contains("lucky_number(random_int)"));
    // Expected-type disambiguation picks the right handler per call.
    assert!(main.content.contains("val n: Int = random_int.next_random()"));
    assert!(main.content.contains("val s: String = random_string.next_random()"));
}

/// Full verification of the effects demo under kotlinc (skipped when
/// kotlinc is not installed).
#[test]
fn kotlinc_compiles_and_runs_effects() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let files = generate_effects_demo();
    let expected = "a: 10\nb: 20\nlucky: 30\nagain: 10\n";
    run_kotlin_files(&files, "effects", expected);
}

// [use-requires-use]
#[test]
fn use_requires_the_use_effect() {
    let errors = expect_errors("fn setup() -> None {\n    use StdOutConsole\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("requires the `use` effect")),
        "unexpected errors: {errors:?}"
    );
}

// [effect-no-dup]
#[test]
fn duplicate_effect_in_list_is_rejected() {
    let errors = expect_errors("fn f() [Console, Console] -> None {\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("duplicate effect `Console`")),
        "unexpected errors: {errors:?}"
    );
}

// [use-no-dup]
#[test]
fn duplicate_use_registration_is_rejected() {
    let errors = expect_errors(
        "fn main() [use] -> None {\n    use StdOutConsole\n    use StdOutConsole\n}\n",
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("a handler for `Console` is already registered")),
        "unexpected errors: {errors:?}"
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
effect Random<T> {
    fn next_random() -> T
}

fn f() [Random<Int>, Random<Double>] -> None {
    let x = next_random()
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("ambiguous effect call")),
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
effect Ping {
    fn ping()
}

handler LoudPing of Ping {
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
fn run_kotlin_files(files: &[salvo_backend_kotlin::EmittedFile], tag: &str, expected: &str) {
    let dir = std::env::temp_dir().join(format!("salvo-kt-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let src_dir = dir.join("src");
    let out_dir = dir.join("out");
    let mut kt_paths = Vec::new();
    for f in files {
        let path = src_dir.join(&f.rel_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &f.content).unwrap();
        kt_paths.push(path);
    }

    let compile = Command::new("kotlinc")
        .args(kt_paths.iter().map(|p| p.as_os_str()))
        .arg("-d")
        .arg(&out_dir)
        .output()
        .expect("failed to run kotlinc");
    assert!(
        compile.status.success(),
        "kotlinc failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new("kotlin")
        .arg("-cp")
        .arg(&out_dir)
        .arg("salvo.main.MainKt")
        .output()
        .expect("failed to run kotlin");
    assert!(
        run.status.success(),
        "generated program crashed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(stdout, expected, "unexpected program output");

    let _ = std::fs::remove_dir_all(&dir);
}

// ===== M6: loops as values =====

/// The M6 demo: loop values from body tails, `break value`, loop `else`
/// in value and statement position, bare `break` (optional value), and a
/// union-typed loop value re-wrapped to the declared type [while-value].
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
    let found = for x in range(0, 10) {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    // Statement-position loop with `else`.
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
    println("capped: ${capped}")

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
    let program = build_program(&[("main.sv", LOOPS, false)]);
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
    assert!(main.content.contains("__loop5\n}"));
    // Statement-position `else` needs only the ran-flag, no `run {}`.
    assert!(main.content.contains("var __loop4_ran = false"));
    assert!(main.content.contains("if (!__loop4_ran) {"));
    // A union-typed loop value re-wraps to the declared arm order.
    assert!(main.content.contains("var __loop6: Union2<String, Int>? = null"));
    assert!(main.content.contains("}.let { when (it) {"));
}

// [while-value]
#[test]
fn break_outside_a_loop_is_an_error() {
    let errors = expect_errors("fn f() -> None {\n    break\n}\n");
    assert!(
        errors.iter().any(|e| e.contains("`break` outside of a loop")),
        "unexpected errors: {errors:?}"
    );
}

/// Full verification of the loops demo under kotlinc (skipped when
/// kotlinc is not installed).
#[test]
fn kotlinc_compiles_and_runs_loops() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let files = generate_loops_demo();
    let expected = "last: 40\nnever: -1\nfound: 4\nempty range\ncapped: 2\nok: 2\n";
    run_kotlin_files(&files, "loops", expected);
}

// ===== M7: reachability, packages/imports, define coverage, companions =====

/// A multi-module program: `main` uses `geometry` (imported) but not
/// `unused`; `geometry` has a Kotlin companion file.
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
    let mut program = build_program(&[
        ("main.sv", main, false),
        ("geometry.sv", geometry, false),
        ("unused.sv", unused, false),
    ]);
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("geometry_helpers.kt"),
        module: salvo_core::ModulePath(vec!["geometry".into()]),
        content: "package salvo.geometry\n\nfun helper(): Int = 1\n".to_string(),
    });
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("unused_helpers.kt"),
        module: salvo_core::ModulePath(vec!["unused".into()]),
        content: "package salvo.unused\n".to_string(),
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
    assert!(paths.contains(&"geometry_helpers.kt".to_string()), "{paths:?}");
    assert!(!paths.contains(&"unused_helpers.kt".to_string()), "{paths:?}");
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

fn main() [use] -> [] None {
    use StdOutConsole
    println("area: ${rect_area(3, 4)}")
}
"#;
    let geometry = "fn area(w: Int, h: Int) -> Int {\n    return w * h\n}\n";
    let program = build_program(&[("main.sv", main, false), ("geometry.sv", geometry, false)]);
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

// [backend-external] A used external fn with no define for the backend is
// a compile-time error.
#[test]
fn missing_define_for_external_fn_is_an_error() {
    let src = r#"
external fn mystery(x: Int) -> Int

fn main() [use] -> [] None {
    use StdOutConsole
    println("${mystery(1)}")
}
"#;
    let errors = expect_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("external fn `mystery` has no kotlin `define fn`")),
        "unexpected errors: {errors:?}"
    );
}

// [backend-external] A referenced external type with no define is an
// error, not a silent pass-through.
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
            .any(|e| e.contains("external type `Mystery` has no kotlin `define type`")),
        "unexpected errors: {errors:?}"
    );
}

// [backend-external] Everything external in `core.*` must be covered by
// the backend's define files, used or not.
#[test]
fn core_externals_must_be_fully_covered() {
    let fake_core = "external fn uncovered_core_fn(x: Int) -> Int\n";
    let program = build_program(&[
        ("core/fake.sv", fake_core, false),
        (
            "main.sv",
            "fn main() [use] -> [] None {\n    use StdOutConsole\n    println(\"hi\")\n}\n",
            false,
        ),
    ]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected coverage errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("external fn `uncovered_core_fn` in core has no kotlin `define fn`")),
        "unexpected errors: {errors:?}"
    );
}

// [kt-effect-params] Effect parameters avoid user parameter names.
#[test]
fn effect_params_avoid_user_names() {
    let src = r#"
fn shadowed(console: Str) [Console] -> [] None {
    println("param: ${console}")
}

fn main() [use] -> [] None {
    use StdOutConsole
    shadowed("value")
}
"#;
    let program = build_program(&[("main.sv", src, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .unwrap();
    // The generated effect parameter picks a fresh name.
    assert!(
        main.content.contains("fun shadowed(console2: Console, console: String)"),
        "unexpected: {}",
        main.content
    );
    assert!(main.content.contains("println(console2, \"param: $console\")"));
}

// [let-destructure] Two struct destructures in one block get unique temps.
#[test]
fn struct_destructure_temps_are_unique() {
    let src = r#"
struct Point {
    x: Int,
    y: Int
}

fn main() [use] -> [] None {
    use StdOutConsole
    let {x} = Point {x: 1, y: 2}
    let {y} = Point {x: 3, y: 4}
    println("${x} ${y}")
}
"#;
    let program = build_program(&[("main.sv", src, false)]);
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
#[test]
fn kotlinc_compiles_and_runs_multi_module() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_multi_module();
    let files = salvo_backend_kotlin::emit_program(&program).unwrap();
    run_kotlin_files(&files, "multimod", "area: 12\n");
}

// [backend-companion] A companion must not collide with a generated file.
#[test]
fn companion_collision_with_generated_file_is_an_error() {
    let mut program = build_program(&[(
        "main.sv",
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    println(\"hi\")\n}\n",
        false,
    )]);
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("main.kt"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: "package salvo.main\n".to_string(),
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
// its template's value (`return run { ... }`), so value-producing defines
// like `random() -> Double` compile; statement-only members are unchanged.
#[test]
fn handler_members_with_return_types_return_their_template() {
    let program = build_program(&[(
        "main.sv",
        "import random.Random\nimport random.DefaultRandom\n\n\
         fn roll() [Random] -> Double {\n    return random()\n}\n\n\
         fn main() [use] -> [] None {\n    use StdOutConsole\n    use DefaultRandom\n    \
         println(\"${roll() < 2.0}\")\n}\n",
        false,
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
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    \
         let big: Long = 5L\n    let ratio: Float = 2.5f\n    let d: Double = 1.5\n    \
         println(\"${big} ${ratio} ${d}\")\n}\n",
        false,
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
qualifier Ok<T> of T
qualifier Err<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

fn describe(v: Ok Str | Err Str) -> Str {
    when v {
        is Ok {
            return "ok"
        }
        is Err {
            return "err"
        }
    }
}

fn main() [use] -> [] None {
    use StdOutConsole
    println(describe(ok("x")))
    println(describe(err("y")))
}
"#;
    let program = build_program(&[("main.sv", src, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt not emitted");
    // Positional arm identity: Ok -> first arm, Err -> second arm.
    assert!(
        main.content.contains("describe(U2_1<String, String>(ok(\"x\")))"),
        "content: {}",
        main.content
    );
    assert!(
        main.content.contains("describe(U2_2<String, String>(err(\"y\")))"),
        "content: {}",
        main.content
    );
}
