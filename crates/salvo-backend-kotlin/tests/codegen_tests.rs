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
    let errors = sources.add_dir(&std_dir, "kotlin", true);
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
/// precise `is Err Str` checks, and flow narrowing.
const UNIONS_DEMO: &str = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn parse_age(input: Int) [as] -> Ok Int | Err Str {
    if input >= 0 {
        return input as Ok
    }
    return "negative age" as Err
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

    let precise: Ok Str | Err Str | Err Bool = "yes" as Ok
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
    // Wrap at return boundaries, positional arm identity.
    assert!(main.content.contains("return U2_1<Int, String>(input)"));
    assert!(main.content.contains("return U2_2<Int, String>(\"negative age\")"));
    // Qualifier-tagged wrap picks the right arm of the 3-union.
    assert!(main.content.contains("U3_1<String, String, Boolean>(\"yes\")"));
    // Precise `is Err Str` tests a single arm; `is Ok` another.
    assert!(main.content.contains("precise is U3_2<*, *, *>"));
    assert!(main.content.contains("precise is U3_1<*, *, *>"));
    // `when` lowers to a sealed when with unwrapped uses.
    assert!(main.content.contains("when (result) {"));
    assert!(main.content.contains("is U2_1<*, *> ->"));
    assert!(main.content.contains("(result.value as Int)"));
}

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

#[test]
fn union_wrap_requires_matching_arm() {
    let src = r#"
qualifier Ok<T> of T
qualifier Err<T> of T

fn f() [as] -> Ok Int | Err Str {
    return true as Err
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
        .arg("salvo.MainKt")
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
