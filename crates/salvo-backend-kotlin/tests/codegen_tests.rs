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

external fn list_size<T>(list: List<T>) [] -> [list] Int
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

// [type-canbe-mut] `Mut List<T>` maps through the define's `Mut inline:`
// template; `Mut` on a type without `canbe Mut` is an error.
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

// [type-canbe-mut] `Mut` only applies to declarations that say `canbe Mut`.
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
    fn next_random() -> [] T
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

// [kt-imports] [kt-qual-mangling] An aliased import of a fn with a
// mangled qualified overload emits one alias import per overload symbol,
// and aliased call sites keep the mangling suffix.
const MANGLED_ALIAS_MAIN: &str = r#"
import lib.shout as holler
import lib.loud

fn main() [use] -> [] None {
    use StdOutConsole
    println(holler("hi"))
    let l = loud("hey")
    println(holler(l))
}
"#;

const MANGLED_ALIAS_LIB: &str = r#"
qualifier Loud of Str

fn loud(s: Str) -> Str as Loud {
    return s
}

fn shout(s: Str) -> Str {
    return s
}

fn shout(s: Loud Str) -> Str {
    return "${s}!"
}
"#;

#[test]
fn aliased_import_of_mangled_overload_keeps_suffix() {
    let program = build_program(&[
        ("main.sv", MANGLED_ALIAS_MAIN, false),
        ("lib.sv", MANGLED_ALIAS_LIB, false),
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

#[test]
fn kotlinc_compiles_and_runs_mangled_alias() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[
        ("main.sv", MANGLED_ALIAS_MAIN, false),
        ("lib.sv", MANGLED_ALIAS_LIB, false),
    ]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "mangled-alias", "hi\nhey!\n");
}

// [backend-external] A used external fn with no define for the backend is
// a compile-time error.
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
    let fake_core = "external fn uncovered_core_fn(x: Int) [] -> [x] Int\n";
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

// ===== S1: the `copy` intrinsic [internal-fn] [copy-fn] [kt-copy] =====

/// Exercises every Kotlin `copy` lowering shape: identity for immutable
/// data, `.toMutableList()` for `Mut List`, `.copy()` for a `Mut` struct
/// with immutable fields, and `.copyOf()` for arrays — plus a fate-linked
/// alias (`let zs = xs`) that must stay readable.
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

// [internal-fn] [kt-copy] `copy` bypasses define templates and lowers
// type-directedly from the checker's resolved argument type.
#[test]
fn copy_lowers_type_directedly() {
    let program = build_program(&[("main.sv", COPY_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.kt")
        .expect("main.kt emitted");
    // Identity for a transitively immutable type (Str).
    assert!(main.content.contains("val t = s\n"), "generated:\n{}", main.content);
    // A real copy for `Mut List<Int>`.
    assert!(
        main.content.contains("val ys = xs.toMutableList()"),
        "generated:\n{}",
        main.content
    );
    // The data-class shallow copy for a `Mut` struct with immutable fields.
    assert!(main.content.contains("val q = p.copy()"), "generated:\n{}", main.content);
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
        "fn main() [use] -> [] None {\n    use StdOutConsole\n    \
         let xs = mutable_list(mutable_list(1))\n    let ys = copy(xs)\n    \
         println(\"${ys.size()}\")\n}\n",
        false,
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

#[test]
fn kotlinc_compiles_and_runs_copy() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", COPY_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "hi\n3 4\na b\n1 9\n3\n";
    run_kotlin_files(&files, "copy", expected);
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

#[test]
fn kotlinc_compiles_and_runs_move_modes() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", S2_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "Grace\n3\n";
    run_kotlin_files(&files, "s2-moves", expected);
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

#[test]
fn kotlinc_compiles_and_runs_borrows() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", S3_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "1\n5\n";
    run_kotlin_files(&files, "s3-borrows", expected);
}

// ===== L6: linear types [linear-obligation] =====

/// The resource pattern linearity exists for: open, use, close — with
/// `discard` as the deliberate drop [linear-discard]. The checker
/// guarantees no path leaks the handle; the demo verifies the lowering
/// (Kotlin: `discard` evaluates and ignores via `.let {}`).
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

#[test]
fn kotlinc_compiles_and_runs_linear() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", LINEAR_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "open data.txt\nfd=8\nclose fd=8\nopen scratch\ndone\n";
    run_kotlin_files(&files, "l6-linear", expected);
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
fn kotlinc_compiles_and_runs_linear_generics() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", LINEAR_GENERICS_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "open 9\nheld fd=9\nopen 1\nopen 2\ncount=2\ndone\n";
    run_kotlin_files(&files, "l7a-linear-generics", expected);
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

#[test]
fn kotlinc_compiles_and_runs_once_fns() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", ONCE_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "consumed 3 items\nplain 7\ndone\n";
    run_kotlin_files(&files, "l7b-once", expected);
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

#[test]
fn kotlinc_compiles_and_runs_derived_returns() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", DERIVED_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "adult: Grace\nhead: Kid\ndone\n";
    run_kotlin_files(&files, "l7c-derived", expected);
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

#[test]
fn kotlinc_compiles_and_runs_fn_contracts() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", CONTRACTS_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let expected = "twice=4\nstill=2\nnamed=4\neaten=2\ndone\n";
    run_kotlin_files(&files, "l7d-contracts", expected);
}

// ===== precedence-aware binary rendering =====

/// Parser grouping must survive re-rendering: without precedence-aware
/// parenthesization `(a - b) * c` would emit flat as `a - b * c` and
/// silently re-associate.
const PRECEDENCE_DEMO: &str = r#"
fn main() [use] -> [] None {
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
    let program = build_program(&[("main.sv", PRECEDENCE_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .expect("main.kt emitted");
    for needle in ["(a - b) * c", "a - (b - c)", "-(a + b)", "(a < b || b > c) && a > c"] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

#[test]
fn kotlinc_compiles_and_runs_precedence() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", PRECEDENCE_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "precedence", "14 9 -13 true\n");
}

// ===== bare `return` in value-position blocks of iterator bodies =====
// [fn-iterator] [kt-iter-iterable]

/// A bare `return` inside a value-position loop in an iterator body must
/// re-target to `return@iterator` like any other iterator-body return —
/// the context survives the `run {}` value lowering (inline, so the
/// non-local return stays legal Kotlin) but not lambda boundaries.
/// (`yield` itself cannot appear inside the `run {}` lowering — Kotlin's
/// restricted suspension scope forbids it; separate known leftover.)
const ITER_RETURN_DEMO: &str = r#"
fn nums(limit: Int) -> Iter<Int> {
    let i = 0
    yield 0
    let last = while i++ < limit {
        if i == 3 {
            return
        }
        i
    }
    yield 99
}

fn main() [use] -> [] None {
    use StdOutConsole
    for n in nums(5) {
        println("a${n}")
    }
    for n in nums(2) {
        println("b${n}")
    }
}
"#;

#[test]
fn iterator_bare_return_in_value_loop_retargets() {
    let program = build_program(&[("main.sv", ITER_RETURN_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .expect("main.kt emitted");
    assert!(
        main.content.contains("return@iterator"),
        "expected retargeted return in:\n{}",
        main.content
    );
    // No bare `return` may remain inside the iterator builder.
    let iter_body = main.content
        .split("iterator {")
        .nth(1)
        .expect("iterator builder emitted");
    let bare_returns = iter_body
        .lines()
        .filter(|l| l.trim() == "return")
        .count();
    assert_eq!(bare_returns, 0, "bare return left in iterator body:\n{iter_body}");
}

#[test]
fn kotlinc_compiles_and_runs_iterator_return() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", ITER_RETURN_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "iter-return", "a0\nb0\nb99\n");
}

// ===== `is` on union-typed struct-field subjects =====
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects, and narrow like them [flow-place];
// `when` still requires a variable subject [when-union-subject].

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
    let program = build_program(&[("main.sv", FIELD_IS_DEMO, false)]);
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

#[test]
fn kotlinc_compiles_and_runs_field_is() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", FIELD_IS_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "field-is", "ok 1\nplain 1\nerr bad\n");
}

// [when-union-subject] `when` still requires a plain variable subject.
#[test]
fn when_field_subject_is_rejected() {
    let src = r#"
qualifier Ok<T> of T

fn ok<T>(value: T) -> T as Ok {
    return value
}

struct Holder {
    result: Ok Int | Str
}

fn main() [use] -> [] None {
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

/// [flow-place] [kt-narrow-field-assert] A narrowed nullable *field* read
/// asserts instead of relying on a smart cast (Kotlin refuses to smart-cast
/// a property, and a `canbe Mut` struct's fields are `var`); a narrowed
/// wrapper-union field reads its arm payload.
#[test]
fn narrowed_field_reads_unwrap() {
    let program = build_program(&[("main.sv", PLACE_NARROW_DEMO, false)]);
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

#[test]
fn kotlinc_compiles_and_runs_place_narrowing() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", PLACE_NARROW_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "place-narrow", "Ann Lee\nBo\ncity Oslo\nok 3\n");
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

/// [kt-narrow-field-assert] The assert is only needed for `var`
/// properties: a field of a struct without `canbe Mut` emits as `val`,
/// which kotlinc smart-casts — asserting there would produce an
/// "unnecessary non-null assertion" warning.
#[test]
fn narrowed_val_field_relies_on_the_smart_cast() {
    let program = build_program(&[("main.sv", PLACE_OPERAND_DEMO, false)]);
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

#[test]
fn kotlinc_compiles_and_runs_place_operand() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", PLACE_OPERAND_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "place-operand", "temp: 22\nnone: no value\n");
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

/// [kt-tuple-component] Tuple elements map onto `Pair`/`Triple`
/// components, and a narrowed one asserts — a stdlib property is not
/// smart-cast [kt-narrow-field-assert].
#[test]
fn tuple_elements_emit_pair_components() {
    let program = build_program(&[("main.sv", TUPLE_INDEX_DEMO, false)]);
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

#[test]
fn kotlinc_compiles_and_runs_tuple_index() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", TUPLE_INDEX_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(
        &files,
        "tuple-index",
        "1 two true\nin 9\nsome here 6\n",
    );
}

// ===== union coercion inside arrays/tuples/lambda returns =====
// [union-wrap] Elements of array/tuple literals and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.

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
    let program = build_program(&[("main.sv", NESTED_COERCION_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    for needle in [
        "arrayOf(U2_1<Int, String>(ok(1)), U2_2<Int, String>(err(\"a\")))",
        "Pair(\"t\", U2_1<Int, String>(ok(2)))",
        "U2_1<Int, String>(ok(3))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

#[test]
fn kotlinc_compiles_and_runs_nested_coercion() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", NESTED_COERCION_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "nested-coercion", "ok 1\nerr a\nok 2\nok 3\nerr b\n");
}

// ===== same-name defines and the define/external pairing =====
// [decl-explicit] Every `define fn` implements exactly one `external fn`:
// the external carries the contract, the define the native template.
// Same-name externals (`twice(Str)` / `twice(Int)`) pair with their
// defines by parameter base types.

const PAIRED_EXTERNALS: &str = r#"
external fn twice(s: Str) [] -> [s] Str
external fn twice(i: Int) [] -> [i] Int
"#;

const PAIRED_DEFINES: &str = r#"
define fn twice(s: Str) -> Str {
    inline: ``
    (${s} + ${s})
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
    let program = build_program(&[
        ("main.sv", &format!("{PAIRED_EXTERNALS}{main}"), false),
        ("main.kotlin.sv", PAIRED_DEFINES, true),
    ]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.kt"))
        .unwrap();
    assert!(
        main.content.contains(r#"("hi" + "hi")"#),
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
        ("main.kotlin.sv", PAIRED_DEFINES, true),
    ]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected codegen errors");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("implements no `external fn` declaration")),
        "unexpected errors: {errors:?}"
    );
}

// [decl-explicit] ...and one external may not have two defines.
#[test]
fn duplicate_defines_for_one_external_are_an_error() {
    let twice_twice = r#"
define fn twice(s: Str) -> Str {
    inline: ``
    (${s} + ${s})
    ``
}

define fn twice(s: Str) -> Str {
    inline: ``
    (${s} + "!")
    ``
}
"#;
    let program = build_program(&[
        (
            "main.sv",
            "external fn twice(s: Str) [] -> [s] Str\n\nfn main() [use] -> [] None {\n    use StdOutConsole\n}\n",
            false,
        ),
        ("main.kotlin.sv", twice_twice, true),
    ]);
    let errors = salvo_backend_kotlin::emit_program(&program)
        .err()
        .expect("expected codegen errors");
    assert!(
        errors.iter().any(|e| e.contains("is defined twice for the same signature")),
        "unexpected errors: {errors:?}"
    );
}

#[test]
fn kotlinc_compiles_and_runs_paired_defines() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let main = r#"
fn main() [use] -> [] None {
    use StdOutConsole
    println(twice("hi"))
    println("${twice(3)}")
}
"#;
    let program = build_program(&[
        ("main.sv", &format!("{PAIRED_EXTERNALS}{main}"), false),
        ("main.kotlin.sv", PAIRED_DEFINES, true),
    ]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "paired-defines", "hihi\n6\n");
}

// ===== effect member fns with their own generics =====
// [effect-member-generics] Member generics render on the interface
// member and bind per call from argument types.

const MEMBER_GENERICS_DEMO: &str = r#"
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
    let x = pick(7, 2)
    let s = pick("l", "r")
    println("${x} ${s}")
}
"#;

#[test]
fn effect_member_generics_render_on_the_interface() {
    let program = build_program(&[("main.sv", MEMBER_GENERICS_DEMO, false)]);
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

#[test]
fn kotlinc_compiles_and_runs_member_generics() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", MEMBER_GENERICS_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "member-generics", "7 l\n");
}

// [effect-member-generics] The member's own generics bind per call: the
// checker knows `pick(1, 2)` is `Int`, not an unbound `T`.
#[test]
fn effect_member_generics_bind_per_call() {
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
    let program = build_program(&[("main.sv", ALIASED_EFFECT_DEMO, false)]);
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
        main.content.contains("val random_int: Random<Int> = CyclicRandom(listOf(7, 8))"),
        "unexpected use lowering in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("roll(random_int)"),
        "handler not threaded through the aliased effect in:\n{}",
        main.content
    );
}

#[test]
fn kotlinc_compiles_and_runs_aliased_effects() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", ALIASED_EFFECT_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(&files, "aliased-effects", "7 8\n");
}

// ===== std array functions =====
// [type-array] Arrays get the `core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02 — the LANGUAGE.md `CyclicRandom` example calls
// `values.size()` on a `T[]`).

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
    let program = build_program(&[("main.sv", ARRAY_STD_DEMO, false)]);
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
        "nums.asIterable()",
        "values.size",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
}

#[test]
fn kotlinc_compiles_and_runs_array_std() {
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping: kotlinc not found on PATH");
        return;
    }
    let program = build_program(&[("main.sv", ARRAY_STD_DEMO, false)]);
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(
        &files,
        "array-std",
        "size 3 get 5 first 3\niter 3\niter 4\niter 5\nrandom 2 3\n",
    );
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

// [name-dot] [kt-nested-dot-name] Dot-named structs emit as *nested*
// classes (never `inner`) and are referenced with the dotted name;
// a dot-named qualifier canonicalizes to its flat spelling in a mangled
// overload name [kt-qual-mangling].
#[test]
fn dot_names_emit_nested_classes() {
    let program = build_program(&[("main.sv", DOT_NAMES, false)]);
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
        main.content.contains("    data class Id(") && main.content.contains("    data class Name("),
        "generated:\n{}",
        main.content
    );
    // Never `inner`: that would need an outer instance to construct.
    assert!(!main.content.contains("inner class"), "generated:\n{}", main.content);
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
    run_kotlin_files(&files, "dot_names", "prod / Production\ntagged t1\nplain t2\n");
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(
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
    let files = salvo_backend_kotlin::emit_program(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    });
    run_kotlin_files(
        &files,
        "qual_subjects",
        "trusted 3\nplain 3\nchecked 2\n",
    );
}
