//! Rust codegen tests: golden snapshots of generated code, the emitted
//! text, and the programs the backend refuses. Compiling and running
//! programs is the run data's business ([test-run-data],
//! `salvo-cli/tests/run/`), on both backends at once.

use std::path::Path;

use salvo_core::{Program, SourceSet};


/// [std-cache] A program of std and `extra`, over the process's std front
/// (`salvo_testkit::front`, TESTS.md C2): std was read, parsed, expanded and
/// checked once, and the program holds its very modules, so only `extra` is
/// parsed, expanded ([actor-route-stub], what `salvo compile` runs before
/// checking) and — through [`emit`] — checked.
fn build_program(extra: &[(&str, &str)]) -> Program {
    let built = salvo_testkit::front::program("rs", extra);
    let errors: Vec<_> = built.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse or expansion errors: {errors:?}");
    built.program
}

/// [std-cache] `emit_program` over the std front [`build_program`] built on.
fn emit(program: &Program) -> Result<Vec<salvo_backend_rust::EmittedFile>, Vec<String>> {
    let front = salvo_testkit::front::std_front("rs");
    salvo_backend_rust::emit_program_over(program, front)
}

fn generate(extra: &[(&str, &str)]) -> Vec<salvo_backend_rust::EmittedFile> {
    let program = build_program(extra);
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// Runs the checker+emitter on a source and returns the errors.
fn expect_errors(src: &str) -> Vec<String> {
    let program = build_program(&[("bad.sv", src)]);
    emit(&program)
        .err()
        .expect("expected errors")
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
    use StdConsole
    let items: Mut List<Int> = Mut List(1, 2)
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
            .contains("pub fn fill(list: &mut Vec<i32>, mut n: i32)"),
        " {}",
        main.content
    );
    assert!(
        main.content
            .contains("pub fn consume(mut list: Vec<i32>) -> i32"),
        " {}",
        main.content
    );
    // Call sites render the matching argument shapes; the moved arg
    // passes by value.
    assert!(
        main.content.contains("crate::fill(&mut items, 3i32)"),
        "{}",
        main.content
    );
    assert!(main.content.contains("crate::read(&items)"), "{}", main.content);
    assert!(main.content.contains("crate::consume(items)"), "{}", main.content);
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

// [rs-union-enums] [type-union-arm-identity]
#[test]
fn unions_emit_enums() {
    let files = generate_unions_demo();
    // One file per arity under `unions/mod.rs` [rs-union-enums].
    let unions = union_text(&files);
    assert!(files.iter().any(|f| f.rel_path.to_string_lossy() == "unions/union3.rs"));
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
        .contains("return crate::unions::Union2::U1(crate::std_core_result::ok::<i32>(input));"));
    assert!(main
        .content
        .contains("return crate::unions::Union2::U2(crate::std_core_result::err::<String>(String::from(\"negative age\")));"));
    // Precise `is Err Str` tests a single arm.
    assert!(main.content.contains("matches!(precise, crate::unions::Union3::U2(_))"));
    assert!(main.content.contains("matches!(precise, crate::unions::Union3::U1(_))"));
    // `when` lowers to an arm test per branch, each with its narrowed read
    // peeled out of the union of borrows the kept parameter is
    // [rs-opt-borrow].
    assert!(main.content.contains("pub fn describe(mut result: crate::unions::Union2<i32, &String>) -> String"));
    assert!(main.content.contains("if matches!(result, crate::unions::Union2::U1(_))"));
    assert!(main.content.contains("match result { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() }"));
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
    // Predicate qualifiers become top-level fns taking borrows, named with
    // their subject [fn-emit-name].
    assert!(main
        .content
        .contains("pub fn Surname__Person_qualifies(person: &crate::Person) -> bool"), "{}", main.content);
    assert!(main
        .content
        .contains("pub fn Positive__Int_qualifies(mut int: i32) -> bool"));
    assert!(main.content.contains("if crate::Surname__Person_qualifies(person)"));
    assert!(main.content.contains("if crate::Positive__Int_qualifies(n)"));
    // The overloads are named by their parameter types, the qualifier
    // included where it is what tells them apart.
    assert!(main
        .content
        .contains("pub fn full_name__SurnamePerson(person: &crate::Person) -> String"), "{}", main.content);
    // Field overrides read out of the declared representation, by reference.
    assert!(main
        .content
        .contains("let mut __claimed_1 = person.surname.as_ref().unwrap();"));
    // `while x is T` re-binds per iteration; `T?` narrows physically.
    assert!(main.content.contains("if !(current.is_some()) {"));
    assert!(main.content.contains("let mut c = current.unwrap();"));
}

// ===== effects =====

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
    assert!(main.content.contains("pub trait __Stateless_Random<T>: Send + Sync {"));
    assert!(main.content.contains("fn next_random(&self) -> T;"));
    assert!(main
        .content
        .contains("impl<T: Clone + Send + Sync + 'static> crate::__Stateful_Random<T> for CyclicRandom<T>"));
    // [rs-handle] Effect deps as leading `&E` handle parameters, named for
    // the effect (a second instance of one effect numbered).
    assert!(main.content.contains(
        "pub fn draw(random: &crate::Random<i32>, random__1: &crate::Random<String>, console: &crate::std_core_console::Console)"
    ));
    // `use` instantiates handlers into locals holding the handle: a stateless
    // one `shared`, a stateful one `locked` [rs-handle].
    assert!(main.content.contains("let __handle_2 = crate::std_core_console::Console::shared(__use_1);"));
    // [effect-handler-generics] The handler is constructed *at* a type: the
    // local's annotation says which.
    assert!(main.content.contains(
        "let mut __use_3: crate::CyclicRandom<i32> = crate::CyclicRandom::new(vec![10i32, 20i32, 30i32], move |__a0: &i32| *__a0);"
    ));
    assert!(main.content.contains("let __handle_4 = crate::Random::locked(__use_3);"));
    // Threading and expected-type disambiguation.
    assert!(main.content.contains("crate::draw(&__handle_4, &__handle_6, &__handle_2);"));
    assert!(main.content.contains("let mut n: i32 = random.next_random();"));
    assert!(main.content.contains("let mut s: String = random__1.next_random();"));
}

// ===== loops as values =====

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

    let found = for x in List(0, 1, 2, 3, 4, 5) {
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
    println("capped: ${capped ?: unreachable!()}")

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

// [loop-while-value] [rs-loop-value]
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
        .contains("let mut __loop_3: Option<i32> = None;"));
    assert!(main.content.contains("__loop_3 = Some(i32::wrapping_mul(i, 10i32));"));
    assert!(main.content.contains("let mut __loop_value_6 = __loop_3.unwrap();"));
    // `else` runs only when the loop never did.
    assert!(main.content.contains("let mut __ran_4: bool = false;"));
    assert!(main.content.contains("if !(__ran_4) {"));
    // `break value` assigns before breaking.
    assert!(main.content.contains("__loop_10 = Some(x);\n                break;"));
    // Optional joins keep the plain Option local (no unwrap).
    assert!(main.content.contains("let mut __loop_value_11 = __loop_10;"));
    // A union-typed loop value re-wraps to the declared arm order.
    assert!(main.content.contains("} { crate::unions::Union2::U1(__v) => crate::unions::Union2::U2(__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U1(__v),"));
}

// ===== multi-module: reachability, crate layout, imports =====

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
    let files = emit(&program).unwrap_or_else(|errors| {
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
        .contains("#[path = \"std/core/console.rs\"]\npub mod std_core_console;"));
    assert!(!main.content.contains("pub mod unused;"));
    // Generated imports: one per name used [rs-imports].
    assert!(main.content.contains("use crate::geometry::area;"), "{}", main.content);
    assert!(main.content.contains("use crate::std_core_console::println;"), "{}", main.content);
    assert!(!main.content.contains("use crate::geometry::*;"), "{}", main.content);
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
    fn pick<T>(value: T) -> T
    => !value {
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

// [type-lit-numeric] [type-basic] Literal suffixes emit explicit Rust types:
// `5L` -> `5i64`, `2.5f` -> `2.5f32`; unsuffixed literals stay bare.
#[test]
fn numeric_literal_suffixes_emit_rust_types() {
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
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs not emitted");
    assert!(
        main.content.contains("fn make() -> i32"),
        "content: {}",
        main.content
    );
    // The Positive overload wins at the call site (a Copy scalar travels
    // by value [rs-copy-by-value]).
    assert!(
        main.content.contains("crate::describe__PositiveInt(crate::make())"),
        "content: {}",
        main.content
    );
}

// ===== S1: the `copy` intrinsic [platform-intrinsic-fn] [copy-fn] [rs-copy] =====

/// The same shape as the Kotlin copy demo: identity/clone lowering per
/// type, plus a fate-linked alias (`let zs = xs`) whose source must stay
/// physically valid ([fate-link]: linked bindings clone, not move).
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

// [platform-intrinsic-fn] [rs-copy] `copy` lowers to
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
        main.content.contains("let mut t: String = (s).clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut ys: Vec<i32> = (xs).clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut q: crate::Person = (p).clone();"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut brr: Vec<i32> = (arr).clone();"),
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

fn longest_name(persons: List<Person>) -> Str
=> !persons {
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
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
    for s in List("x", "y") {
        consume(s)
    }
    let xs = Mut List(1, 2)
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
            .contains("pub fn longest_name(mut persons: Vec<crate::Person>) -> String"),
        "generated:\n{}",
        main.content
    );
    // Move-mode loop: by value, no clone.
    assert!(
        main.content.contains("for mut person in persons.into_iter() {"),
        "generated:\n{}",
        main.content
    );
    // Move-mode bindings: a real partial move and a real move.
    assert!(
        main.content.contains("let mut name: String = person.name;"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut ys: Vec<i32> = xs;"),
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
    let xs = Mut List(1, 2)
    let ys = xs
    let n = size(ys)
    add(xs, 9)
    return n + size(xs)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
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
            .contains("pub fn count_long(persons: &Vec<crate::Person>) -> i32"),
        "generated:\n{}",
        main.content
    );
    // By-reference iteration over the borrowed parameter.
    assert!(
        main.content.contains("for mut person in persons.iter() {"),
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
    use StdConsole
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
    // now the escape hatch for. Plain: the program's module declares one
    // `close`, and std's do not take part [fn-emit-name].
    assert!(
        main.content.contains("close(h)"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("close(temp)"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains("drop(note)"),
        "generated:\n{}",
        main.content
    );
}


// ===== L7b: once fn types [fn-once] =====

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
    use StdConsole
    let xs = List(1, 2, 3)
    let g = () -> { consume_list(xs) }
    run_once(g)
    let n = 7
    let plain = () -> { println("plain ${n}") }
    run_once(plain)
    println("done")
}
"#;

// [fn-once] `once` fn parameters emit `impl FnOnce`.
#[test]
fn once_fn_params_emit_fnonce() {
    let files = generate(&[("main.sv", ONCE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains(
            "pub fn run_once(console: &crate::std_core_console::Console, f: impl FnOnce(&crate::std_core_console::Console))"
        ),
        "generated:\n{}",
        main.content
    );
}

// ===== L7c: derived returns [proj-readonly-return] =====

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
    use StdConsole
    let people = List(Person {name: "Kid", age: 9}, Person {name: "Grace", age: 45})
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

// [proj-readonly-return] Derived returns emit borrows: elided lifetime for a
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
            .contains("pub fn find_adult(persons: &Vec<crate::Person>) -> Option<&crate::Person>"),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "pub fn head_of<'a>(persons: &'a Vec<crate::Person>, tag: &String) -> Option<&'a crate::Person>"
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
        main.content.contains("return crate::std_core_list::first_platform::<crate::Person>(persons);"),
        "generated:\n{}",
        main.content
    );
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

fn apply_consuming(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] !v => !data {
    return f(data)
}

fn count(people: List<Person>) -> Int => people {
    return size(people)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
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
// params are `&mut dyn FnMut`; named fns wrap in adapters.
#[test]
fn fn_type_contracts_emit_modes() {
    let files = generate(&[("main.sv", CONTRACTS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content.contains(
            "pub fn apply_keeping(f: &mut dyn FnMut(&Vec<crate::Person>) -> i32, data: &Vec<crate::Person>) -> i32"
        ),
        "generated:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "pub fn apply_consuming(f: &mut dyn FnMut(Vec<crate::Person>) -> i32, mut data: Vec<crate::Person>) -> i32"
        ),
        "generated:\n{}",
        main.content
    );
    assert!(
        // A path names the module's own `count`, whatever core declares.
        main.content.contains("crate::apply_keeping(&mut |__a0: &Vec<crate::Person>| crate::count(__a0), &people)"),
        "adapter expected:\n{}",
        main.content
    );
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
    let files = generate(&[("main.sv", FIELD_IS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("matches!(h.result, crate::unions::Union2::U1(_))"),
        "expected union test on the field in:\n{}",
        main.content
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
        main.content.contains("let mut __narrowed_1 = p.surname.as_ref().unwrap();"),
        "expected the narrowed field read to unwrap in:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("let mut __narrowed_3 = p.address.city.as_ref().unwrap();"),
        "expected the narrowed field *chain* read to unwrap in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut __narrowed_4 = match &h.result { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };"),
        "expected the narrowed wrapper-union field read to use the arm \
         accessor in:\n{}",
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
        main.content.contains("let mut __narrowed_3 = r.value.unwrap();"),
        "expected the narrowed operand to unwrap in:\n{}",
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
        main.content.contains("nested.1.0, nested.1.1"),
        "expected a nested index chain in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut __narrowed_3 = maybe.0.as_ref().unwrap();"),
        "expected the narrowed element to unwrap in:\n{}",
        main.content
    );
}

// ===== effect dependencies on handlers [effect-handler-deps] =====
// A handler constructor parameter of effect type is a dependency: the
// member body may use that effect, the `use` site supplies it from scope,
// and callers of the outer effect never mention it.

/// [rs-handle] One parameter per declared effect, in declaration order,
/// typed by the effect's handle — the only shape there is.
#[test]
fn a_fn_takes_one_handle_per_declared_effect() {
    let files = generate(&[("main.sv", NO_DEPS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert!(
        c.contains("pub fn work(logger: &crate::Logger, console: &crate::std_core_console::Console)"),
        "expected one handle parameter per effect:\n{c}"
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
    use StdConsole
    use PlainLogger
    work()
}
"#;

// ===== interception [effect-intercept] =====
// A handler that depends on the effect it implements wraps the instance
// registered before it (binds strictly outward), and a `use` may shadow an
// earlier registration — innermost wins [effect-use-no-dup]. Both are new in
// 2026-09-14; the emission constraint is that a fusion must carry exactly
// **one** `__Has_E` impl per effect (two is E0119), so the shadowed
// instance loses its inherited accessor while staying reachable through
// `__outer` for the intercepting handler's own dependency.

// ===== the interceptor chain [effect-handle] =====
// The production shape the 2026-09-20 arc was built for:
// a stateful handler binds as a monitor, a stateless dependent
// handler binds bare and captures its dependencies as owned handles at
// construction — one of them the effect it implements (interception binds
// outward), another a monitor's handle.

// ===== spawn-inheritance and the `with` clause [actor-spawn-inherit] =====
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
    state {
        t: Int = 0
    }
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
    use StdConsole
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

/// [rs-handle] [actor-spawn-inherit] The one shape (user decision 2026-09-28): a fn
/// declaring `[Logger, Clock]` takes one `&E` handle per effect, a
/// dependent handler captures clones of them (`Stamped::new(logger.clone(),
/// clock.clone())`) whether the effect arrived through the signature or a
/// `use`, a `with` item is a private instance behind its own handle, and the
/// caller passes `&mut` of its own handles — no bundles, no fused value.
#[test]
fn signature_supplied_effects_are_handles() {
    let files = generate(&[("main.sv", SPAWN_INHERIT_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    let c = &main.content;
    assert!(
        c.contains("pub fn interception(logger: &crate::Logger, clock: &crate::Clock)"),
        "expected one handle parameter per declared effect:\n{c}"
    );
    assert!(
        c.contains("crate::Stamped::new(logger.clone(), clock.clone());\n    let __handle_2 = crate::Logger::shared(__use_1);"),
        "the capture must clone the handles in scope:\n{c}"
    );
    assert!(
        c.contains("crate::Stamped::new(__handle_6.clone(), crate::Clock::shared(crate::FixedClock::new()))"),
        "a `with` item is a private instance behind its own handle:\n{c}"
    );
    assert!(
        c.contains("crate::interception(&__handle_6, &__handle_4);"),
        "the caller passes its handles:\n{c}"
    );
    assert!(
        !c.contains("__Hs_") && !c.contains("__Fx") && !c.contains("__Has_"),
        "no bundle, fused value or accessor trait remains:\n{c}"
    );
}

/// [rs-handle] A dependent handler is one struct with a handle field per
/// dependency; a `use` of it anywhere is `E::shared(H::new(deps…))`
/// — nothing per site is generated.
#[test]
fn a_dependent_handler_is_one_struct_however_often_it_is_used() {
    let src = r#"
export effect Logger {
    fn log(m: Str) -> None => m
}

export handler PlainLogger [Console] of Logger {
    fn log(m: Str) -> None => m {
        println("log: ${m}")
    }
}

export fn first() [Console, use] -> None {
    use PlainLogger()
    log("first")
}

export fn second() [Console, use] -> None {
    use PlainLogger()
    log("second")
}

export fn main() [use] -> None {
    use StdConsole
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
        c.matches("pub struct PlainLogger {").count(),
        1,
        "one struct for the handler:\n{c}"
    );
    assert!(
        c.contains("__dep0: crate::std_core_console::Console,"),
        "the dependency is a handle field:\n{c}"
    );
    assert_eq!(
        c.matches("crate::PlainLogger::new(console.clone());\n    let __handle_2 = crate::Logger::shared(__use_1);").count(),
        2,
        "both fns construct the handler from their own handle:\n{c}"
    );
    assert!(!c.contains("__Fx_"), "nothing per `use` site is generated:\n{c}");
}

// ===== dependent handlers in anger [rs-handle] =====
// Handler state behind a dependency, a *two*-dependency handler whose
// second dependency is another handler's effect, a fn needing two effects
// calling one needing a subset, nested `use` scopes with the outer one used
// again afterwards, a `use` inside a fn that already has effects, and two
// effect calls in one expression (which must not borrow the fused value
// twice).

// [rs-handle] A dependency *chain* (Audit needs Logger needs
// Console), a `use` inside a loop body, effect calls nested in another
// call's arguments, and a predicate qualifier whose `qualifies` declares an
// effect — all through one fused value.

// [rs-handle] Constructor parameters mixing a dependency with plain
// data, a dependent member calling a fn that does its own `use`, an
// effect-using lambda passed to an effect-*free* higher-order fn (allowed:
// the call threads nothing, so nothing aliases), three effects in one
// signature, a `use` inside a `while` body, and an effect member taking a
// `Mut` parameter.

/// [fn-effects] The program the fusion used to lose to Kotlin: a function
/// *value* that uses an effect, passed to a callee that needs one too.
/// Capturing made the closure hold a borrow that overlapped the call's own
/// (`E0499`), and hoisting could not separate them. Threading the effect
/// *into* the value lifts it: the closure captures nothing, so the two
/// borrows are of different things.
#[test]
fn effect_using_fn_value() {
    const SRC: &str = r#"
export effect Logger {
    fn log(message: Str) -> None => message
}

export handler ConsoleLogger [Console] of Logger {
    fn log(message: Str) -> None => message {
        println("LOG: ${message}")
    }
}

export fn run_it(f: (s: Str) [Logger] -> Str) [Console, Logger] -> None =>[f] s {
    println(f("x"))
}

export fn demo() [Console, Logger] -> None {
    run_it(s -> {
        log("in lambda ${s}")
        return "done ${s}"
    })
}

export fn main() [use] -> None {
    use StdConsole
    use ConsoleLogger()
    demo()
}
"#;
    let files = generate(&[("main.sv", SRC)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    // The effect arrives as a parameter of the closure, not a capture: the
    // effect's handle, typed on the lambda's leading parameter [rs-handle].
    assert!(
        main.content.contains(": &crate::Logger, "),
        "expected the effect threaded into the closure as a handle:\n{}",
        main.content
    );
}

// ===== operator typing: widening, literal adoption, conversions =====
// [op-arith] [op-promote] [type-lit-adopt] [op-convert] The same source and
// expected stdout as the Kotlin backend's case [backend-parity]: `Long`
// arithmetic with widened `Int` operands, adopted literals, mixed-width
// ordering, and the explicit conversions. Values are chosen fractional
// where floats print, since whole-double rendering differs between the
// backends today (open defect).

const OPERATOR_DEMO: &str = r#"
fn main() [use] -> None {
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

/// [op-promote] [type-lit-adopt] Rust has no mixed-width operators, so widened
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
        c.contains("i64::wrapping_add(x, ((i32::wrapping_mul(n, 2i32)) as i64))"),
        "expected the widened operand cast as a whole:\n{c}"
    );
    assert!(
        c.contains("(((n) as i64) < x)"),
        "expected the ordering operand cast:\n{c}"
    );
    // [op-convert] The operand is parenthesized inside the cast: `as` binds
    // tighter than unary minus, so `-1 as u8` would be `-(1 as u8)` — which
    // rustc refuses for `to_byte(-1)` and would have meant the wrong thing
    // for a saturating target [type-byte].
    assert!(
        c.contains("((big) as i32)"),
        "expected the conversion intrinsic lowered to a cast:\n{c}"
    );
}

// ===== [effect-at] [effect-member-overload] shared member names =====
// The same source and expected stdout as the Kotlin backend's case
// [backend-parity]: two effects declaring `close`, resolved by
// availability in `shut` and by the `@Effect` selector in `main`.

// ===== destructuring a loop element [var-let-destructure] =====

/// [var-let-destructure] A `for` binds a **pattern**, not just a name: the header
/// binds the element to a temporary and the body opens with the pattern's
/// bindings. Expected output is verbatim the Kotlin backend's.
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

/// [var-let-destructure] [rs-borrow-locals] What it lowers to: the element in a
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
        text.contains("let mut k = &__elem_6.0;") && text.contains("let mut v: i32 = __elem_6.1;"),
        "expected the tuple parts bound by reference (a scalar copied):\n{text}"
    );
    assert!(
        text.contains("let mut who = &__elem_10.name;") && text.contains("let mut score: i32 = __elem_10.score;"),
        "expected the struct fields bound by reference, renamed:\n{text}"
    );
    assert!(
        text.contains("let mut a: i32 = __elem_14.0;"),
        "expected an assigned-to binding to own its copy:\n{text}"
    );
}


// ===== union coercion inside arrays/tuples/lambda returns =====
// [type-union] Constructor arguments, tuple elements and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.
// Also covers: a fn-type `let` annotation is dropped in Rust
// (`impl Trait` is invalid on bindings [fn-contract]).

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
        "crate::unions::Union2::U1(crate::tag_ok::<i32>(1i32))",
        "crate::unions::Union2::U2(crate::tag_err::<String>(String::from(\"a\")))",
        "crate::unions::Union2::U1(crate::tag_ok::<i32>(2i32))",
        "crate::unions::Union2::U1(crate::tag_ok::<i32>(3i32))",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
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
    fn pick<T>(a: T, b: T) -> T
    => !a, !b {
        return a
    }
}

export fn main() [use] -> None {
    use StdConsole
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
    let files = generate(&[("main.sv", ALIASED_EFFECT_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    assert!(
        main.content.contains("crate::roll(&__handle_4), crate::roll(&__handle_4)"),
        "handler not threaded through the aliased effect in:\n{}",
        main.content
    );
}

// ===== std array functions =====
// [type-array] Arrays get the `std.core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02). `T[]` and `List<T>` share the `Vec<T>`
// rendering, so their lowerings mirror each other.

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
    let files = generate(&[("main.sv", ARRAY_STD_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .unwrap();
    for needle in [
        "(nums.len() as i32)",
        "nums.get((2i32) as i64 as usize).copied()",
        "nums.first().copied()",
    ] {
        assert!(
            main.content.contains(needle),
            "expected `{needle}` in:\n{}",
            main.content
        );
    }
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

fn tag(value: Str) -> +Environment.Tag Str
=> !value {
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

// [name-dot] [rs-fn-mangling] Rust concatenates a dot-name
// (`Environment.Id` → `EnvironmentId`): modules and structs share one
// type namespace, so a nested `mod Environment` beside
// `struct Environment` would be E0428. Mangled overload names use the
// same flat spelling.
#[test]
fn dot_names_flatten() {
    let program = build_program(&[("main.sv", DOT_NAMES)]);
    let files = emit(&program).unwrap_or_else(|errors| {
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
        main.content.contains("pub id: crate::EnvironmentId,"),
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
        main.content.contains("pub fn label__EnvironmentTagStr("),
        "generated:\n{}",
        main.content
    );
}


// ===== E3 step 2: throw and `try` [throw] [throw-try] [rs-throw-controlflow] =====

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
            "pub fn parse(console: &crate::std_core_console::Console, mut line: String) -> std::ops::ControlFlow<String, i32>"
        ),
        "expected a ControlFlow return shape in:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("return std::ops::ControlFlow::Break(String::from(\"empty line\"));"),
        "expected `throw` to return Break in:\n{}",
        main.content
    );
    assert!(
        main.content.contains("return std::ops::ControlFlow::Continue("),
        "expected returns to wrap in Continue in:\n{}",
        main.content
    );
    // [throw] The effect emits no trait: there are no handlers to implement.
    let std_throw = files
        .iter()
        .find(|f| f.rel_path.ends_with("std/core/throw.rs"))
        .map(|f| f.content.clone())
        .unwrap_or_default();
    assert!(
        !std_throw.contains("trait Throw"),
        "expected no trait for the throw effect in:\n{std_throw}"
    );
    // [rs-exit-splice] With nothing pending at the exit, propagation is the
    // plain re-raise: the release was written before the call, so the throw
    // path owes nothing.
    let measure = main
        .content
        .split("pub fn measure")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert!(
        measure.contains("crate::close_file(console, h);\n    let mut n: i32 = (match crate::parse(console, line) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => return std::ops::ControlFlow::Break(__m) });")
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
    // union's arms [type-union-arm-identity]; a single type stays bare.
    assert!(
        main.content.contains("break 'try_3 crate::unions::Union2::U2(crate::unions::Union2::U1(__m))")
            && main.content.contains("break 'try_3 crate::unions::Union2::U2(crate::unions::Union2::U2(__m))")
            && main.content.contains("break 'try_1 crate::unions::Union2::U2(__m)"),
        "expected the message wrapped into its arm in:\n{}",
        main.content
    );
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
    use StdConsole
    use ConsoleLogger()
    demo()
}
"#;

/// [fn-effects] [rs-handle] The effect is a leading `&E` handle
/// parameter of the closure type, so nothing is captured — which is what
/// lets the value cross a call that borrows the same effect value. The
/// lambda takes it as a typed parameter and the named-fn adapter forwards
/// the reborrow to a declaration that needs it (`shout`).
#[test]
fn fn_type_effects_thread_into_closures() {
    let files = generate(&[("main.sv", FN_EFFECTS_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("f: &mut dyn FnMut(&crate::Logger, &String) -> String"),
        "expected the handle in the closure type:\n{}",
        main.content
    );
    assert!(
        main.content.contains("&mut |mut __leff0, mut s| -> String {\n        __leff0.log("),
        "expected the handle as a leading closure parameter:\n{}",
        main.content
    );
    assert!(
        main.content.contains("|__e0: &crate::Logger, __a0: &String| crate::shout(__e0, __a0)"),
        "expected the named-fn adapter to forward the handle:\n{}",
        main.content
    );
    assert!(
        main.content.contains("|mut __fx0, mut __a0| -> String {\n        crate::plain(__a0)"),
        "expected the pure fn to ignore the threaded effect:\n{}",
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
            .contains("let mut nested_1 = match &nested { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };"),
        "expected the widened value peeled into a local, by reference:\n{}",
        main.content
    );
    // The nested `when` then tests the peeled (inner) value.
    let describe = main
        .content
        .split("pub fn describe")
        .nth(1)
        .and_then(|s| s.split("pub fn").next())
        .unwrap();
    assert!(
        describe.contains("if matches!(nested, crate::unions::Union2::U1(_))")
            && describe.contains("if matches!(nested_1, crate::unions::Union2::U1(_))"),
        "expected an outer and an inner test in:\n{describe}"
    );
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

/// [rs-when-cond] Rust has no subject-less `match`, so the chain lowers to
/// `if`/`else if`/`else`. The mandatory `else` makes it total, so no
/// `unreachable!()` filler and — in value position — no `else { None }`
/// [expr-if-else-none].
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
        classify.contains("if (n < 0i32) {") && classify.contains("} else if (n == 0i32) {"),
        "expected an if/else-if chain:\n{classify}"
    );
    assert!(
        classify.contains("} else {") && !classify.contains("None"),
        "expected a plain `else` and no optional filler:\n{classify}"
    );
    assert!(
        !classify.contains("unreachable!"),
        "a total chain needs no filler arm:\n{classify}"
    );
    // An `is` head still declares its binding at the top of the branch.
    assert!(
        main.content.contains("let mut s = match &value { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };"),
        "expected the `is` binding inside the branch:\n{}",
        main.content
    );
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
    use StdConsole()
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
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// [platform-handler] [rs-platform-handler] What the compiler emits: the
/// effect's `trait` as for any effect, **no struct** — the struct is the
/// host's — and a `use` site constructing it through its mounted path.
/// `main` stays in generated code: nothing
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
        src.contains("pub trait __Stateless_RawClock: Send + Sync {") && src.contains("fn raw_now(&mut self) -> i32"),
        "expected the generated trait, got:\n{src}"
    );
    assert!(
        !src.contains("pub struct HostRawClock"),
        "a platform handler must not emit a struct of its own:\n{src}"
    );
    assert!(
        src.contains("crate::__Platform_HostRawClock::new(35i32)"),
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
        "impl crate::RawClockPlatform for HostRawClock {",
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
    let errors = emit(&program)
        .err()
        .expect("a `use` of a platform handler without a host must not emit");
    assert!(
        errors.iter().any(|e| e.contains("`HostRawClock`")
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
fn a_platform_handler() {
    let skeleton = platform_handler_skeleton();
    let host = skeleton
        .content
        .replace("todo!(\"implement RawClock.raw_now\")", "self.offset + 7");
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
}

// ===== [platform-threadsafe] the thread-safety contract =====

/// [platform-threadsafe] [rs-handle] An **undeclared** platform handler
/// binds like every handler: the `use` site wraps the host in the effect's
/// handle, which serializes it (user decision 2026-09-26, kept under the one
/// shape of 2026-09-28).
#[test]
fn an_undeclared_platform_handler_is_serialized_behind_the_handle() {
    let files = generate_platform_handler_demo_with(&platform_handler_skeleton().content);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs should be generated");
    let src = &main.content;
    assert!(
        src.contains("crate::__Platform_HostRawClock::new(35i32);\n    let __handle_4 = crate::RawClock::locked(__use_3);"),
        "expected the handle around the host, got:\n{src}"
    );
}

/// The demo above with the contract declared: the host promises concurrent
/// safety, so the compiler shares it without a lock.
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

fn threadsafe_platform_handler_program() -> salvo_core::Program {
    build_program(&[("main.sv", THREADSAFE_PLATFORM_HANDLER_DEMO)])
}

fn threadsafe_platform_handler_skeleton() -> salvo_backend_rust::EmittedFile {
    let program = threadsafe_platform_handler_program();
    let mut files = salvo_backend_rust::platform_skeletons(&program, None)
        .unwrap_or_else(|errors| panic!("skeleton errors:\n{}", errors.join("\n")));
    assert_eq!(files.len(), 1, "one module declares a platform handler");
    files.remove(0)
}

fn generate_threadsafe_platform_handler_demo_with(
    host: &str,
) -> Vec<salvo_backend_rust::EmittedFile> {
    let mut program = threadsafe_platform_handler_program();
    program.companions.push(salvo_core::CompanionFile {
        rel_path: std::path::PathBuf::from("platform/main.rs"),
        module: salvo_core::ModulePath(vec!["main".into()]),
        content: host.to_string(),
        platform: true,
    });
    emit(&program).unwrap_or_else(|errors| {
        panic!("codegen errors:\n{}", errors.join("\n"));
    })
}

/// [platform-threadsafe] [rs-handle] A **`threadsafe`** platform handler is
/// emission-neutral (user decision 2026-09-28): the word is the contract the
/// skeleton prints, and the `use` site wraps the host in the effect's handle
/// exactly as an undeclared one — no twin trait, no `Arc` adapter.
#[test]
fn a_threadsafe_platform_handler_binds_like_any_other() {
    let files = generate_threadsafe_platform_handler_demo_with(
        &threadsafe_platform_handler_skeleton().content,
    );
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs should be generated");
    let src = &main.content;
    assert!(
        src.contains("crate::__Platform_HostRawClock::new(35i32);\n    let __handle_4 = crate::RawClock::shared(__use_3);"),
        "expected the handle around the host, got:\n{src}"
    );
    assert!(
        !src.contains("__Shared_HostRawClock") && !src.contains("__Arc_HostRawClock"),
        "`threadsafe` emits nothing of its own:\n{src}"
    );
}

/// [platform-threadsafe] [platform-tree] The skeleton prints the contract the
/// implementer signs; the receivers are `&mut self` implementing the effect's
/// own trait either way, since the handle serializes every instance today.
#[test]
fn platform_generate_prints_the_threadsafe_contract_into_the_skeleton() {
    let file = threadsafe_platform_handler_skeleton();
    let src = &file.content;
    for expected in [
        "`threadsafe platform handler HostRawClock` — THE CONTRACT YOU ARE SIGNING",
        "safe to run concurrently",
        "pub struct HostRawClock {",
        "impl crate::RawClockPlatformSync for HostRawClock {",
        "fn raw_now(&self) -> i32 {",
        "[platform-threadsafe]",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // And the undeclared skeleton says what the compiler does.
    let plain = platform_handler_skeleton();
    assert!(
        plain.content.contains("the compiler SERIALIZES this instance")
            && plain.content.contains("fn raw_now(&mut self) -> i32 {"),
        "expected the serialized contract on the undeclared skeleton:\n{}",
        plain.content
    );
}

/// [platform-threadsafe] End to end under rustc with the *generated*
/// threadsafe skeleton, one stub filled in. Same program, same output as
/// the undeclared run — and byte-identical to the Kotlin backend's.
#[test]
fn a_threadsafe_platform_handler() {
    let skeleton = threadsafe_platform_handler_skeleton();
    let host = skeleton
        .content
        .replace("todo!(\"implement RawClock.raw_now\")", "self.offset + 7");
    assert_ne!(host, skeleton.content, "the stub should have been replaced");
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
    use StdConsole()
    let ns = List(1, 2)
    for v in map(iter(copy(ns)), n -> n * 2) { println("bare=${v}") }
    for v in map(iter(ns), (n: Int) -> n * 3) { println("ann=${v}") }
    let ws = List("hi")
    for w in map(iter(copy(ws)), (s: Str) -> shout(s)) { println("str=${w}") }
    for w in map(iter(copy(ws)), shout) { println("named=${w}") }
    for n in map(iter(ws), s -> size(s)) { println("size=${n}") }
    println("applied=${apply(2, (n: Int) -> n + 1)}")
}
"#;

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
        "f: &mut dyn FnMut(&'value T) -> U",
        // The lambda's *inner* convention follows the declaration: its
        // parameter arrives borrowed, whatever the annotation says.
        "&mut |mut n| -> i32 {\n        i32::wrapping_mul(*n, 3i32)",
        // [iter-yield-proj] The element is a borrow, so the parameter is one
        // reference deeper than the annotation, and is peeled once.
        "&mut |mut s| -> String {\n        let s = *s;\n        crate::shout(s)",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
}

// ===== [iter-protocol] laziness, now a property of the pass =====

/// [seq-into] The combinator surface: **eager**, over a pass, with `map_to`
/// for mapping into a collection the caller provides. The lazy pair went with
/// the 2026-09-10 removal, so the generic bodies are reached by passing a pass
/// explicitly — which is what `map(iter(copy(xs)), …)` is for here.
///
/// Same source and stdout as the Kotlin backend's twin. What it exercises that
/// nothing did before: the *generic* `?Next` body — `map`/`filter` over a
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

pub const SEQ_SURFACE_OUTPUT: &str = "eager 4\ngeneric 4\nkept 2\nsink 4\nchained 6\n";

#[test]
fn the_combinator_surface() {
    let files = generate(&[("main.sv", SEQ_SURFACE_DEMO)]);
    let seq = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("std/core/seq.rs"))
        .expect("std/core/seq.rs")
        .content;
    // The generic body drives its subject through the `next` the call site
    // resolved, which arrives as an ordinary borrowed callback [implicit-group].
    assert!(
        seq.contains("next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>"),
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
        main.contains("crate::std_core_seq::map_to::<Vec<i32>, crate::std_core_list::ListNext<'_, i32>, i32, i32>("),
        "expected the type arguments spelled out in:\n{main}"
    );
}

pub const UNBOUNDED_DEMO: &str = r#"
struct Naturals {
    from: Int
}

fn naturals() -> Naturals {
    return Naturals {from: 0}
}

iter fn iter(n: Naturals) -> Emitted Int | Finished {
    state NaturalsIter {
        at = n.from
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
    use StdConsole()
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

/// Every path through the feature in one program: a `fns` group spread
/// with no binder, its members resolved from the visible `Int` overloads, a
/// call overriding *one* member by name, forwarding through a generic fn
/// whose `T` is opaque, and an individually declared `?add` reached by the
/// same name. The Kotlin backend asserts the same stdout for the same
/// source.
/// [implicit-group] [iter-protocol] The **composition rendering** R5's
/// combinators are built on (user decision 2026-09-09): a generic combinator
/// takes the *pass* as its subject and reaches its `next` through the
/// `?Next<It, T>` spread. Two things had never been exercised together
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
/// declares `: Linear<self>` beside `: Next<self, T>`, so [group-obligation]
/// requires the `close` — and *driving* is what releases it (user decision
/// 2026-09-09). The loop calls `close` on every exit: exhaustion, `break` and
/// `return` alike. Before this the linear obligation counted as discharged by
/// the move into the loop — bookkeeping — while the resource leaked.
pub const RAW_CLOSE_DEMO: &str = r#"
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
        // Plain: the program's module declares one `close` [fn-emit-name].
        main.content.contains("close(console, lines);"),
        "expected the program's own explicit close:\n{}",
        main.content
    );
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

pub const GENERIC_DRIVE_OUTPUT: &str = "first 3\nrest 7\nv 8\nv 9\nv 7\n";

/// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
/// linear pass releases it through the `?Linear<It>` spread. The release cannot
/// print — an implicitly resolved fn is effect-free [implicit-resolve] — so the
/// call is asserted in the emitted code and the program is run to prove it
/// builds.
pub const GENERIC_CLOSE_DEMO: &str = r#"
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

pub const GENERIC_CLOSE_OUTPUT: &str = "all 10\ncut 7\n";

pub const YIELD_SPREAD_DEMO: &str = r#"
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

pub const NEXT_SPREAD_OUTPUT: &str = "d 2\nd 4\nd 6\n";

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
        main.content.contains("&mut |__a0: &mut crate::Slice<'_, i32>| (match crate::next(&mut *__a0) {"),
        "expected the adapter to pass the `&mut` parameter through, got:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>"),
        "expected the spread position to be a `&mut`-taking, union-returning fn, got:\n{}",
        main.content
    );
}

pub const IMPLICIT_DEMO: &str = r#"
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
    use StdConsole()
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
        src.contains("crate::bigger::<String>(a, b, &mut |__a0: &String, __a1: &String|"),
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
        // takes its `Int`s by value, so the adapter derefs the borrows.
        "&mut |__a0: &i32, __a1: &i32| crate::add(*__a0, *__a1)",
        // The same bridging for an override written at the call site
        // [implicit-override].
        "&mut |__a0: &i32, __a1: &i32| crate::multiply(*__a0, *__a1)",
        // Forwarding reborrows the enclosing fn's own parameter.
        "&mut *add",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("struct Field"),
        "a `fns` group must leave no runtime representation:\n{src}"
    );
}

/// [cmp-groups] Comparison, equality and hashing as **fns groups** — the
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
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn alike<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
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
    use StdConsole()
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
    println("same ${alike(7, 7)} ${alike(ab, b)} ${alike(ab, same_ab)}")
    println("digest ${digest_agrees(ab, same_ab)} ${digest_agrees(3, 3)}")
    // `min_of` answers one of its arguments, so it **moves** them: the
    // deduction is declared `=> !a, !b` [deduce-default], and these two are
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
        "(Ord::cmp(&(1i32), &(2i32)) as i32)",
        "(Ord::cmp(&ab[..], &b[..]) as i32)",
        "(&ab[..] == &b[..])",
        "std::hash::DefaultHasher::new()",
        // [rs-fn-param-convention] A kept position borrows: the generic body
        // compares `a` and `b` and still owns them afterwards.
        "pub fn min_of<T: Clone>(mut a: T, mut b: T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> T",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // A group is never a value, so nothing of the three survives as a type.
    assert!(
        !src.contains("struct Ordered") && !src.contains("struct Hashed"),
        "a `fns` group must leave no runtime representation:\n{src}"
    );
}

/// [fn-attached] The **canonical** implementation of a capability for a type,
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

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

// The canonical ordering for `Person`: an ordinary overload that travels with
// the type, exported to match it.
"#;

pub const CANONICAL_MAIN: &str = r#"
// Only the *type* is imported; its canonical rides along [fn-attached].
import people.Person

// A second ordering is named for what it does and passed explicitly: a second
// `cmp(Person, Person)` here would be refused, since `cmp` on `Person` is
// declared only in `Person`'s file [op-unique-shape].
fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn main() [use] {
    use StdConsole()
    let ada = Person { name: "ada", age: 36 }
    let bob = Person { name: "bob", age: 5 }
    // Both spellings of the selector: at a call, and as a value filling an
    // implicit parameter [implicit-override].
    println("by age ${cmp@Person(ada, bob)}")
    println("by name ${by_name(ada, bob)}")
    let younger = min_of(ada, bob, cmp = cmp@Person)
    println("younger ${younger.name}")
}
"#;

pub const CANONICAL_OUTPUT: &str = "by age 1\nby name -1\nyounger bob\n";

/// [cmp-auto] `: Ordered<self> by auto` and friends: the compiler writes the
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
struct Point : Ordered<self> by auto, Hashed<self> by auto {
    x: Int,
    y: Int
}

// [fn-by] The **function-level** form, with no obligation clause at all: one
// member stamped from `std.core.auto`, declared on the type by being inside it.
struct Tag {
    label: Str

    fn eq(a: Tag, b: Tag) [] -> Bool => a, b by auto
}

// A generic struct is not stamped at yet (ROADMAP §2c): its `eq` is
// written by hand, with the implicit the copy would have needed.
struct Box<T> : Eq<self> {
    item: T
}

fn eq<T>(a: Box<T>, b: Box<T>, ?Eq<T>) [] -> Bool => a, b {
    return eq(a.item, b.item)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
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
    use StdConsole()
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

/// [obligation-by] The lowering: a stamped member is an **ordinary Rust fn**
/// whose body is the unrolled Salvo — a `cmp` per field, in declaration order —
/// and the struct derives nothing beyond `Clone`, `Debug` and `PartialEq`: a
/// container over a `List<Point>` goes through Salvo's list `eq`/`hash`/`cmp`
/// [col-hashed-ordered].
#[test]
fn default_obligations_lower_to_salvo_fns() {
    let files = generate(&[("main.sv", AUTO_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        "#[derive(Clone, Debug, PartialEq)]\npub struct Point",
        // The stamped body is field-wise Salvo: one per-field `cmp`, then `0`.
        "let mut c__c1: i32 = (Ord::cmp(&(a.x), &(b.x)) as i32);",
        "return 0i32;",
        // The hand-written generic `eq` takes its implicit.
        "pub fn eq__",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // No structural member is lowered to the host's derived operation any more.
    assert!(!src.contains("(Ord::cmp(a, b) as i32)"), "{src}");
}

/// [comptime-bound] [obligation-by] [fn-by] [comptime-inline] The comptime
/// round's first slice, end to end: `by auto` on a struct and on a **named
/// union** (`type Source = … : Ordered<self> by auto`), the function form
/// mixing a stamped `hash` with a hand-written `eq`, and `ToStr<self> by auto`
/// rendering the literal's shape. The union bodies dispatch with `inline when`
/// over the arms, `None` included. Source and expected stdout are **verbatim**
/// the Kotlin backend's `kotlinc_compiles_and_runs_comptime`.
pub const COMPTIME_DEMO: &str = r#"
struct Point : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    x: Int,
    y: Int
}

struct Manual { note: Str }
struct Imported { feed: Str, n: Int }
type Source = Manual | Imported | None : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto

fn cmp(a: Manual, b: Manual) -> Int => a, b { return cmp(a.note, b.note) }
fn eq(a: Manual, b: Manual) -> Bool => a, b { return eq(a.note, b.note) }
fn hash(v: Manual) -> Long => v { return hash(v.note) }
fn to_str(v: Manual) -> Str => v { return "Manual(${v.note})" }

struct Imported2 : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto { feed: Str, n: Int }
fn cmp(a: Imported, b: Imported) -> Int => a, b { return cmp(a.n, b.n) }
fn eq(a: Imported, b: Imported) -> Bool => a, b { return eq(a.n, b.n) }
fn hash(v: Imported) -> Long => v { return hash(v.n) }
fn to_str(v: Imported) -> Str => v { return "Imported(${v.feed}, ${v.n})" }

struct Reading : Hashed<self> {
    sensor: Str,
    value: Int

    fn hash(r: Reading) -> Long by auto
    fn eq(a: Reading, b: Reading) -> Bool => a, b { return a.sensor == b.sensor }
}

fn main() [use] {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 3 }
    println("${p}")
    println("cmp: ${cmp(p, q)} ${cmp(q, p)} ${cmp(p, p)}")
    println("eq: ${p == q} ${p == Point { x: 1, y: 2 }}")
    println("hash agrees: ${hash(p) == hash(Point { x: 1, y: 2 })}")
    let s: Set<Point> = Set(p, q, Point { x: 1, y: 2 })
    println("set size: ${size(s)}")
    let a: Source = Manual { note: "m" }
    let b: Source = Imported { feed: "f", n: 3 }
    let c: Source = None
    println("${to_str(a)} ${to_str(b)} ${to_str(c)}")
    println("union cmp: ${cmp(a, b)} ${cmp(b, a)} ${cmp(a, a)} ${cmp(c, a)}")
    println("union eq: ${eq(a, b)} ${eq(a, Manual { note: "m" })} ${eq(c, c)}")
    println("union hash: ${hash(a) == hash(Manual { note: "m" })}")
    println("${Imported2 { feed: "x", n: 1 }}")
    println("reading: ${hash(Reading { sensor: "s", value: 1 }) == hash(Reading { sensor: "s", value: 1 })}")
}
"#;

pub const COMPTIME_OUTPUT: &str = "Point { x: 1, y: 2 }\ncmp: -1 1 0\neq: false true\nhash agrees: true\nset size: 2\nManual(m) Imported(f, 3) None\nunion cmp: -1 1 0 1\nunion eq: false true true\nunion hash: true\nImported2 { feed: x, n: 1 }\nreading: true\n";

/// [op-order] [op-equality] The **operators** through the groups: `a < b` is
/// `cmp(a, b) < 0` and `a == b` is `eq(a, b)`, at a struct, at a `Str` (which
/// had no ordering at all before, and now orders by code point on both
/// backends) and at a generic `T` whose capability arrives as an implicit.
///
/// Source and expected stdout are **verbatim** the Kotlin backend's
/// `kotlinc_compiles_and_runs_operators_through_the_groups`. The `Str` line is
/// the parity claim: the JVM's own `<` would compare UTF-16 code units.
pub const GROUP_OPERATOR_DEMO: &str = r#"
struct Point : Ordered<self> by auto, Eq<self> by auto {
    x: Int,
    y: Int
}

// A hand-written canonical, which the operators reach exactly as they reach a
// generated one [fn-attached].
struct Age {
    years: Int,
    label: Str

    fn eq(a: Age, b: Age) [] -> Bool => a, b {
        return eq(a.years, b.years)
    }

    fn cmp(a: Age, b: Age) [] -> Int => a, b {
        return cmp(a.years, b.years)
    }
}

// [implicit-forward] Generic code publishes the capability in its signature,
// and the operators inside go through the parameter.
fn larger_of<T>(a: T, b: T, ?Ordered<T>) [] -> T => !a, !b {
    if a > b {
        return a
    }
    return b
}

fn same_of<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return a == b
}

fn main() [use] {
    use StdConsole()
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
    // [type-basic] The basic types compare with the host's operator, `Bool`
    // and `Char` included.
    println("basic ${true > false} ${false >= true} ${'a' < 'b'} ${3 != 4} ${2.5 == 1.5}")
}
"#;

pub const GROUP_OPERATOR_OUTPUT: &str = "struct true false true true\nby hand true false\n\
                                   strs true false false\ngeneric 9 9 ada\n\
                                   basic true false true true false\n";

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

// [fn-attached] The obligation names the capability; this file's `cmp` fulfils
// it, so `cmp` is declared on `Person` and travels with it.
struct Person : Ordered<self> { name: Str, age: Int }

fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

// A second ordering a ranked list may be built with instead.
fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

// [cmp-binder] Here the binder is an implicit parameter the fn declares, so
// resolution fills it and the result type publishes what it chose.
fn empty_ranked<T>(?cmp: (T, T) -> Int) -> +Ranked<T>(?cmp) Mut List<T> {
    return Mut List()
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
        if cmp(get(r, i) ?: unreachable!(), get(r, best) ?: unreachable!()) < 0 {
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
    use StdConsole()

    // Built with the canonical ordering: the youngest ranks first.
    let by_age = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(), Person {name: "Ada", age: 36}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Cyd", age: 31}
    )
    let youngest = get(by_age, least_index(by_age)) ?: unreachable!()
    println("by age: ${youngest.name} of ${ranked_size(by_age)}")

    // Built with another ordering: the same code, a different answer.
    let alpha = rank_add(
        rank_add(
            rank_add(empty_ranked<Person>(cmp = by_name), Person {name: "Cyd", age: 31}),
            Person {name: "Bob", age: 24}
        ),
        Person {name: "Ada", age: 36}
    )
    let first = get(alpha, least_index(alpha)) ?: unreachable!()
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
struct Person : Ordered<self> by auto, Eq<self> by auto { name: Str, age: Int }

fn by_age(a: Person, b: Person) -> Int => a, b {
    return cmp(a.age, b.age)
}

// One body, either ordering: the identity is the container's, not the caller's.
fn lowest<T>(s: SortedSet<T>) -> T? => s {
    return min(s)
}

fn main() [use] {
    use StdConsole()
    let byname = SortedSet(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24}
    )
    // [cmp-binder] The ordering is **passed**: what fills the binder decides
    // what the container carries (user decision 2026-09-26).
    let byage: SortedSet<Person>(by_age) = SortedSet(
        Person {name: "Cyd", age: 31},
        Person {name: "Ada", age: 36},
        Person {name: "Bob", age: 24},
        cmp = by_age
    )
    println("byname ${(lowest(byname) ?: unreachable!()).name} of ${size(byname)}")
    println("byage ${(lowest(byage) ?: unreachable!()).name} of ${size(byage)}")

    // `cmp`-distinct membership: a second 24-year-old is the same member under
    // `by_age`, and its own member under the canonical ordering.
    let twin = Person {name: "Eve", age: 24}
    let grown_age: Mut SortedSet<Person>(by_age) = Mut SortedSet(cmp = by_age)
    let grown_name: Mut SortedSet<Person> = Mut SortedSet()
    add(grown_age, Person {name: "Bob", age: 24})
    add(grown_name, Person {name: "Bob", age: 24})
    println("twin ${add(grown_age, copy(twin))} ${add(grown_name, twin)}")
}
"#;

pub const KEYED_OUTPUT: &str = "byname Ada of 3\nbyage Bob of 3\ntwin false true\n";

/// [cmp-carry] [col-membership] A **hash** container keyed by the `hash` and `eq`
/// its type names: membership is `eq`-distinct, so a second person of an age
/// already present is one member under a pair that keys by age and a new one under
/// the canonical pair. Source and expected stdout are **verbatim** the Kotlin
/// backend's `kotlinc_compiles_and_runs_a_keyed_hash_pair` — there the pair is
/// passed to a runtime container as two function references, here it is two
/// zero-sized markers.
pub const KEYED_HASH_DEMO: &str = r#"
struct Person : Hashed<self> by auto { name: Str, age: Int }

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
    use StdConsole()
    let all: Mut Set<Person> = Mut Set()
    // [cmp-binder] The pair is **passed**, never inferred from the annotation:
    // what fills the binder decides what the container is keyed by (user
    // decision 2026-09-26 — one way to choose an identity).
    let byage: Mut Set<Person>(age_hash, same_age) = Mut Set(hash = age_hash, eq = same_age)
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

/// [rs-fn-field] A **composed pass, hand-written**: it stores both its source
/// and its callback. Storing a function in a struct field used to be a Rust
/// codegen error while Kotlin accepted it (a live backend divergence, recorded
/// as R0 finding 5); the field is now an `Rc<dyn Fn…>`, the same
/// representation a generated pass has always used for a callback, so the
/// program builds on both targets — which is what makes user-written
/// combinators possible at all, not just std's generated ones.
pub const FN_FIELD_DEMO: &str = r#"
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

struct Doubling : Next<self, Int> canbe Mut {
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
    use StdConsole()
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
/// callback as `<fn>`, and `Arc::new` at the store ([rs-handle]: `Arc<dyn Fn
/// + Send + Sync>` since 2026-09-28, so a callback may be handler state).
#[test]
fn a_function_in_a_struct_field_is_an_arc_dyn_fn() {
    let files = generate(&[("main.sv", FN_FIELD_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted");
    assert!(
        main.content
            .contains("pub f: std::sync::Arc<dyn Fn(i32) -> i32 + Send + Sync>,"),
        "expected an Arc<dyn Fn> field, got:\n{}",
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
        main.content.contains("f: std::sync::Arc::new(move |__a0: i32| crate::twice(__a0))"),
        "expected the store to wrap in Arc::new, got:\n{}",
        main.content
    );
}

/// [iter-protocol] A hand-written **pass**: `zip`, which `yield` cannot
/// express because it reads two sources at once. A struct, a `next`, and
/// nothing else — no state machine, no compiler support beyond driving it.
pub const PASS_DEMO: &str = r#"
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

pub const PASS_OUTPUT: &str = "n 3\nn 2\nn 1\nada is 36\ngrace is 45\ndone\n";

/// The driving loop: the subject is moved into a mutable local and each turn
/// calls the resolved `next`, testing for the `Emitted` arm the *checker*
/// chose [type-union-arm-identity]; any other arm ends the loop.
#[test]
fn a_pass_lowers_to_a_driving_loop() {
    let files = generate(&[("main.sv", PASS_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        src.contains("let mut __pass_3: crate::Countdown = crate::countdown(3i32);"),
        "expected the subject bound to a local in:\n{src}"
    );
    assert!(
        src.contains("crate::next__Countdown(&mut __pass_3);\n        if matches!(__step_4, crate::unions::Union2::U1(_)) {"),
        "expected a driving loop in:\n{src}"
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

pub const FALLIBLE_PASS_OUTPUT: &str =
    "line alpha\nline beta\nread 2\nline alpha\nfailed: stopped: bad line at 2\n";

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
        let needle = format!("crate::unions::Union2::U1(crate::unions::Union2::{arm}(crate::std_core_iterator::emitted::<String>(");
        assert!(
            main.content.contains(&needle),
            "expected the inner {what} arm to be wrapped before the outer one in:\n{}",
            main.content
        );
    }
}

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

iter fn iter(f: Flat) -> Emitted Int | Finished {
    state FlatIter { at: Int = 0, inner: Mut ListNext<Int>? = None }
    while true {
        if inner is Mut ListNext<Int> {
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
    use StdConsole()
    let base = List(1, 2)
    let p: Mut ListNext<Int>? = iter(base)
    if p is Mut ListNext<Int> {
        show(next(p))
        show(next(p))
        show(next(p))
    }
    let qs = List(7, 8)
    let q: Mut ListNext<Int> | Int = iter(qs)
    if q is Mut ListNext<Int> {
        show(next(q))
        show(next(q))
    }
    let rs = List(1, 2, 3)
    let r: Mut ListNext<Int>? = iter(rs)
    if r is Mut ListNext<Int> {
        r.at = 2
        show(next(r))
    }
    // Literals, not constructor calls: the element constructors claim
    // `NonEmpty` [col-of-nonempty], which a plain `List<List<Int>>` field
    // refuses (type arguments are invariant).
    let all = Flat { rows: List<List<Int>>(List(1, 2), List(3, 4, 5)) }
    for n in iter(all) {
        println("n ${n}")
    }
}
"#;

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
        // The optional, and the `state` slot inside the generated pass:
        // each narrowed into a `&mut` binding of the storage.
        "let mut p_3 = p.as_mut().unwrap();",
        "next__ListNext::<i32>(&mut *p_3)",
        "let mut __narrowed_1 = __p.inner.as_mut().unwrap();",
        // The union arm.
        "let mut q_4 = match &mut q { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };",
        // The assignment base.
        "let mut r_5 = r.as_mut().unwrap();\n        r_5.at = 2i32;",
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
    state {
        held: Ok Mut List<Int> | Err Str = ok(Mut List(0))
    }

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
    use StdConsole()
    use Holder()

    let a: Mut List<Int>? = Mut List(1)
    if a is Mut { add(a, 9) }
    if a is Mut { println("var ${to_str(a)}") }

    let b = Mut Shelf { items: Mut List(1) }
    if b.items is Mut { add(b.items, 9) }
    if b.items is Mut { println("field ${to_str(b.items)}") }

    let c: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    if c is ^Ok { add(c, 9) }
    if c is ^Ok { println("widen ${to_str(c)}") }

    let d: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    when d { is Ok { add(d, 9) } is Err { } }
    if d is ^Ok { println("when ${to_str(d)}") }

    let e: Ok Mut List<Int> | Err Str | None = ok(Mut List(1))
    if e is ^Ok { add(e, 9) }
    if e is ^Ok { println("nullable ${to_str(e)}") }

    println("moved ${eat(ok(Mut List(1)))}")
    stash(5)
    println("state ${dump()}")
}
"#;

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
        // A variable and a field over the nullable repr, each narrowed into
        // a `&mut` binding of the storage.
        "let mut a_5 = a.as_mut().unwrap();\n        crate::std_core_list::add_platform::<i32>(&mut *a_5, 9i32);",
        "let mut __narrowed_7 = b.items.as_mut().unwrap();",
        // A wrapper arm, narrowed by `when`/`is`.
        "let mut d_11 = match &mut d { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };",
        // The `^` shadow: a borrow into the storage.
        "let mut c_9 = match &mut c { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };",
        "let mut e_14 = match &mut e { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };",
        // A moved union parameter binds `mut`, or the peel cannot borrow it.
        "pub fn eat(mut o: ",
        // A handler state field peels through `self`.
        "let mut held_1 = match &mut self.held { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };",
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

/// [rs-narrow-mut] [rs-opt-borrow] A union-typed parameter whose arm is `Mut`
/// is lent mutably, arm by arm (`Union2<&mut Vec<i32>, &String>`), so peeling
/// the `Mut` arm out of it and mutating reaches the caller's value — as on
/// Kotlin. (Until 2026-10-10 the frame received it read-only, and this was a
/// reported error.)
#[test]
fn peeling_a_mut_arm_out_of_a_borrowed_parameter() {
    let src = r#"
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is ^Ok {
        add(o, 7)
    }
}

fn main() [use] {
    use StdConsole()
    let r: Ok Mut List<Int> | Err Str = ok(Mut List(1))
    bump(r)
    bump(r)
    if r is ^Ok {
        println("${size(r)}")
    }
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(main.content.contains("o: crate::unions::Union2<&mut Vec<i32>, &String>"), "{}", main.content);
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
    use StdConsole()
    use Angle()
    println(show(7))
    println(show(7, fmt = loud))
}
"#;

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
        // The implementation has to match it exactly (stateless: `&self`).
        "fn show(&self, v: i32, fmt: &mut dyn FnMut(i32) -> String) -> String {",
        // And the call site fills it.
        "__handle_4.show(7i32, &mut |__a0: i32| crate::fmt(__a0))",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
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
    use StdConsole()
    use Plain<Int>()
    use Prefixed<Int>("p")
    println(show(7))
    println(tagged(7))
}
"#;

#[test]
fn a_generic_handler_is_constructed_at_its_type() {
    let files = generate(&[("main.sv", HANDLER_GENERICS_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    for expected in [
        // The instance's annotation, from the written type argument alone...
        "let mut __use_3: crate::Plain<i32> = crate::Plain::new();",
        // ...and where a constructor argument could also have bound it.
        "let mut __use_5: crate::Prefixed<i32> = crate::Prefixed::new(String::from(\"p\"));",
        // A type parameter no field mentions needs `PhantomData`, or the
        // struct itself does not compile (`E0392`).
        "__phantom_T: std::marker::PhantomData<T>,",
        "__phantom_T: std::marker::PhantomData\n",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
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

/// [qual-refn] [qual-erasure] The same source and the same expected stdout as
/// the Kotlin backend asserts — which is the parity claim itself. The emitted
/// Rust carries no trace of the refinement.
#[test]
fn a_refined_program() {
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
}

/// [qual-refn] [diag-structured] A suppressed refinement conflict is
/// a *warning*: the program is legal, so it must still emit — *and* the
/// warning must reach the driver, or the diagnostic exists only in `salvo
/// analyze`. Tested per backend because both the gate and the channel are
/// duplicated in each: the same condition has to fire on both sides.
// `with NonEmpty` on both: std's own `NonEmpty` refines `add` too
// [col-nonempty], so without it this would assert a three-way disagreement
// including std's, rather than the rule. Declaring compatibility with std's
// claim is the one-word remedy, and leaves the Q1-vs-Q2 conflict intact.
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
    let (files, warnings) = salvo_backend_rust::emit_program_reporting(&program, None)
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


// ===== [op-safe-nav] `?.` optional chaining =====

/// The same source and expected stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_safe_navigation`. A `?.` field chain feeds a
/// `?:`, and a `?.` call link on a place receiver rides on dot-call lowering;
/// the whole thing short-circuits to `None` at the first absent link. Run on
/// both backends, since the lowering (a short-circuiting `Switch`) and both
/// the native (Kotlin) and generic (Rust) renderings must agree.
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

// [op-safe-nav] [rs-safe-nav] A field-only `?.` chain is a combinator chain.
#[test]
fn safe_navigation_field_chain_is_a_combinator_chain() {
    let files = generate(&[("main.sv", SAFE_NAV_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    // `o?.inner?.nanos` → an `and_then`/`map` chain, not nested `is_none()`
    // blocks.
    assert!(
        main.content.contains(".and_then(|") && main.content.contains(".map(|"),
        "expected a combinator chain, got:\n{}",
        main.content
    );
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
    assert!(main.contains("crate::shout(&b)"), "unexpected:\n{main}");
    assert!(
        main.contains(r#"format!("{} / {}", b, dup)"#),
        "unexpected:\n{main}"
    );
    // The parts are borrowed, not moved: a variadic position is untracked by
    // the flow analysis, so an owned `vec![pa]` would move a variable the
    // checker still considers live.
    assert!(
        main.contains(r#"let mut x: String = crate::std_core_string::Mut_Str(vec![pa.clone()]);"#),
        "unexpected:\n{main}"
    );
    // [platform-value-type] The operations are std's host code
    // (`lib/platform/std/core/string.rs`), called through their wrappers; a
    // `Mut Str` goes as `&mut`.
    assert!(
        main.contains("crate::std_core_string::index_of_platform(&hay, &ll)"),
        "unexpected:\n{main}"
    );
    assert!(main.contains("crate::std_core_string::set_platform(&mut b, 0i32, 'H')"), "unexpected:\n{main}");
    // The support file is only generated when something needs it.
    let plain = generate(&[("main.sv", "export fn main() {\n}\n")]);
    assert!(
        !plain
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "strings.rs"),
        "the string helpers must not be emitted for a program that never mutates a string"
    );
}

/// [fn-variadic] A `...spread` into a variadic intrinsic *is* the whole
/// collection: `vec![parts]` would be a vector of one vector. Cloned rather
/// than moved, matching Kotlin's `listOf(*arr)` — and the string parts are
/// borrowed, since `mut_str` reads them.
#[test]
fn a_spread_into_a_variadic_intrinsic_is_the_collection() {
    let src = r#"
export fn main() [use] {
    use StdConsole()
    let parts = array_of("a", "b")
    let sb = Mut Str(...parts)
    let xs = Set(...parts)
    println("${sb} ${size(xs)}")
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [col-of-nonempty] `Set` rather than `List`: the list constructors
    // now have a *first* parameter, which a spread may not supply
    // [fn-variadic], so a lone spread reaches the purely variadic intrinsics —
    // which is where this lowering lives anyway.
    assert!(
        main.contains("let mut sb: String = crate::std_core_string::Mut_Str(parts.clone());")
            && main.contains("Set_platform::<String>(parts.clone(), "),
        "unexpected:\n{main}"
    );
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
    use StdConsole()
    let b = Mut Str("seed")
    grow(b)
    println("${b} ${size(b)}")
    let buf = Mut Buf {text: Mut Str("in-struct")}
    append(buf.text, "!")
    set(buf.text, 0, 'I')
    println("${buf.text}")
}
"#;

#[test]
fn mut_str_places() {
    let files = generate(&[("main.sv", MUT_STR_PLACES)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // A `Mut Str` parameter is a `&mut String`, reborrowed for the helper.
    assert!(
        main.contains("pub fn grow(s: &mut String)") && main.contains("crate::std_core_string::set_platform(&mut *s, 0i32, 'G')"),
        "unexpected:\n{main}"
    );
    assert!(
        main.contains("crate::std_core_string::set_platform(&mut buf.text, 0i32, 'I')"),
        "unexpected:\n{main}"
    );
}

// ===== [rs-seq] [seq-iterator] [implicit-group] the sequence functions =====

/// The same source and stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_sequences`: `map`/`filter`/`reduce` over a `List`
/// (the intrinsic fast path), and over the *passes* an array, a `Str`, an origin
/// and a struct of the program's own hand out — `iter` written at each use site
/// [seq-iterator].
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

/// [rs-seq] The `List` fast paths are std functions (`filter` a platform fn),
/// whose parameter types are what give a callback its expected type.
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
        main.contains("map__List_Fn::<i32, i32>(&xs, &mut |mut n| -> i32 {\n        i32::wrapping_mul(*n, 2i32)")
            && main.contains("reduce__List_A_Fn::<i32, i32>(&xs, 0i32, &mut |mut a, mut b| -> i32 {\n        i32::wrapping_add(*a, *b)")
            && main.contains("filter_platform::<i32>(&xs, &mut |mut n| -> bool {\n        (*n > 2i32)"),
        "unexpected:\n{main}"
    );
    // A named fn as the callback wraps in an adapter: a fn item's own
    // convention is by value, and `map` hands it `&T` [fn-contract].
    assert!(
        main.contains("(&xs, &mut |__a0: &i32| crate::double(*__a0))"),
        "unexpected:\n{main}"
    );
    // [iter-fn] A pass subject reaches the generic overload through its `iter`,
    // and the `next` the checker resolved arrives as the implicit argument.
    assert!(
        main.contains("NaturalsIter2"),
        "expected the pass to be minted through its generated `iter` in:\n{main}"
    );
    let _ = &files;
}

// ===== [fn-overload-at] [fn-rename] the caller's two overrides =====

/// The same source and stdout as the Kotlin backend's
/// `kotlinc_compiles_and_runs_overload_overrides`: a module-level `size` that
/// shadows std's, `@module` to reach past it (and past a *local* of the same
/// name), and a `rename` giving one of two unrankable overloads a name of its
/// own. Both mechanisms are compile-time only, so what the pair asserts is
/// that the two targets agree about which declaration each call meant.
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
    // `@core.list` is std's `size`, a platform fn's wrapper called by its
    // module's path.
    assert!(
        main.contains("crate::std_core_list::size_platform::<i32>(&xs)"),
        "expected the core lowering:\n{main}"
    );
    // The renamed overload is called by its declaration's mangled name.
    assert!(
        main.contains("crate::label__SmallInt(n)"),
        "unexpected:\n{main}"
    );
    // The shadowed call goes through the crate path.
    assert!(
        main.contains("crate::describe(7i32)"),
        "expected a path past the local:\n{main}"
    );
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
        src.contains("= next(&mut *it);"),
        "expected the in-place drive in:\n{src}"
    );
    // Precisely: no local for *this* loop's pass.
    let take = src.split("pub fn take").nth(1).and_then(|s| s.split("\npub fn ").next()).expect("take");
    assert!(
        !take.contains("__pass"),
        "a kept pass must not be bound into a local in:\n{src}"
    );
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

/// [iter-fn] What the expansion emits, and what it does *not*: a plain struct
/// borrowing the parameters beside the `state` fields, a plain minter, and a
/// plain `next` — no state machine, no `__advance`, no state
/// number, and no per-`defer` flag.
#[test]
fn an_iter_fn_emits_a_plain_struct_and_next() {
    let files = generate(&[("main.sv", ITER_FN_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs emitted")
        .content;
    // [iter-fn] The struct its `state` block names holds every parameter —
    // **borrowed** whole, as a `proj` field [proj-field], so it carries a
    // lifetime and the mint clones nothing [copy-opt-in] — then the state.
    assert!(
        main.contains("pub struct CountdownIter<'s> {\n    pub c: &'s crate::Countdown,\n    pub at: i32,\n}")
            && main.contains("crate::CountdownIter { c: c, at: c.from }"),
        "expected the parameter borrowed beside the state:\n{main}"
    );
    assert!(
        main.contains("pub struct FibsIter<'s> {\n    pub f: &'s crate::Fibs,")
            && main.contains("crate::FibsIter { f: f, a: 0i32, b: 1i32, made: 0i32 }"),
        "expected the parameter held whatever the body reads of it:\n{main}"
    );
    assert!(
        main.contains("pub struct RowIter<'s> {\n    pub r: &'s crate::Row,")
            && main.contains("crate::RowIter { r: r, left: r.times }"),
        "expected the whole subject to be borrowed:\n{main}"
    );
    assert!(
        !main.contains("__advance") && !main.contains("__state"),
        "an `iter fn` needs no state machine:\n{main}"
    );
    // [fn-effects] An effectful `next` takes its handlers as leading arguments,
    // threaded into every turn of the loop.
    assert!(
        main.contains("crate::next__NoisyIter(&__handle_2, &mut __pass_14);"),
        "expected the handler threaded into the drive:\n{main}"
    );
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
    use StdConsole()
    // read one field, mutate another
    let p = Person { name: "ann", tags: Mut List("x") }
    let n = p.name
    add(p.tags, "y")
    println("A ${n} ${size(p.tags)}")

    // two disjoint mutable fields
    let q = Pair { left: Mut List("l"), right: Mut List("r") }
    let l = q.left
    add(q.right, "r2")
    println("C ${size(l)} ${size(q.right)}")

    // read a field, hand a disjoint field to a mutating fn
    let p2 = Person { name: "dee", tags: Mut List("x") }
    let n2 = p2.name
    touch(p2.tags)
    println("D ${n2} ${size(p2.tags)}")

    // assignment to a disjoint field
    let p3 = Mut Person { name: "eve", tags: Mut List("x") }
    let n3 = p3.name
    p3.tags = Mut List("q", "r")
    println("E ${n3} ${size(p3.tags)}")
}
"#;

#[test]
fn field_disjoint_access() {
    let files = generate(&[("main.sv", FIELD_DISJOINT_DEMO)]);
    // The borrow is real and lives across the disjoint mutation: that is the
    // shape rustc has to accept for L5 to be clone-free here.
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        main.contains("let mut n = &p.name;") && main.contains("add_platform::<String>(&mut p.tags, "),
        "expected a disjoint-field borrow held across the mutation in:\n{main}"
    );
}

/// [fate-partial-move] L5's move half end to end: a field handed to a
/// consuming fn is a **real partial move** in the emitted Rust
/// (`crate::eat(p.tags);` then a read of `p.name`), and reassigning the moved field
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
    use StdConsole()
    // hand one field away, keep reading the other
    let p = Person { name: "ann", tags: Mut List("x") }
    eat(p.tags)
    println("1 ${p.name}")

    // the same through a move-mode binding
    let q = Person { name: "bob", tags: Mut List("y") }
    let t = q.tags
    add(t, "z")
    println("2 ${q.name} ${size(t)}")

    // put the field back, and the whole value works again
    let u = Mut Person { name: "eve", tags: Mut List("t") }
    eat(u.tags)
    u.tags = Mut List("new", "pair")
    println("3 ${u.name} ${size(u.tags)}")
}
"#;

#[test]
fn a_partial_move() {
    let files = generate(&[("main.sv", PARTIAL_MOVE_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // The field moves out as a raw place (no clone), and the sibling is
    // still read afterwards: a genuine partial move.
    assert!(
        main.contains("crate::eat(p.tags);\n    crate::std_core_console::println(&__handle_2, &format!(\"1 {}\", p.name));")
            && main.contains("let mut t: Vec<String> = q.tags;"),
        "expected a partial move with a live sibling read in:\n{main}"
    );
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
    use StdConsole()
    // Annotated to a plain list: the element constructor claims `NonEmpty`
    // [col-of-nonempty], and on a non-empty list `first` answers an element
    // rather than the optional this case is about.
    let people: List<Person> = List(Person { name: "ann" }, Person { name: "bob" })
    let head = first(people)
    if head is None {
        return
    }
    println("${take(copy(head))}")
    println("${head.name}")
}
"#;

#[test]
fn an_optional_borrow_unwrap() {
    let files = generate(&[("main.sv", OPT_BORROW_DEMO)]);
    let main = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    assert!(
        main.contains("let mut head_3 = head.unwrap();\n    crate::std_core_console::println(&__handle_2, &format!(\"{}\", crate::take((head_3).clone())));")
            && !main.contains("head.as_ref().unwrap()"),
        "expected the reference unwrap, not a clone of the reference, in:\n{main}"
    );
}

// ===== [rs-repoint] a re-pointed view: one Rust variable per live range =====

/// [rs-repoint] [rs-proj-generic] [fate-link] ROADMAP 0v: a local holding a
/// view, re-pointed at another source, compiles and runs as on Kotlin
/// (`kotlinc_compiles_and_runs_re_pointed_views` runs the same program). The
/// repro (`pick`) and its neighbours: over an iterator, in both arms of an
/// `if`, in an arm that returns, on every turn of a loop, not returned while
/// the first source is written, the empty-then-borrowed and
/// borrowed-then-empty orders, Copy elements, and the shapes whose stored
/// value flows out of its block (one variable). Two causes, both Rust's: the
/// `List` fast path of `filter` answered owned clones where its type is a
/// view (`Vec<String>` against `Vec<&String>`), and one variable has one
/// lifetime for every borrow ever stored in it (E0515).
const REPOINT_DEMO: &str = r#"
// A local view re-pointed at a parameter and returned (ROADMAP 0v).
fn pick(ws: List<Str>, keep: (Str) -> Bool) [] -> Mut List<proj Str> holds proj(ws) => ws, keep {
    let local = List("x", "y")
    let out = filter(local, keep)
    count(out)
    out = filter(ws, keep)
    return out
}

fn count(xs: List<proj Str>) [] -> Int {
    return size(xs)
}

// The same over an iterator: the generic `filter` holds the iterator's source.
fn yielded(ws: List<Str>) [] -> Mut List<proj Str> holds proj(ws) => ws {
    let local = List("x", "yy")
    let li = iter(local)
    let out = filter(li, s -> size(s) > 1)
    count(out)
    let wi = iter(ws)
    out = filter(wi, s -> size(s) > 1)
    return out
}

// Re-pointed in both arms of an `if`, then returned.
fn either(a: List<Str>, b: List<Str>, first: Bool) [] -> Mut List<proj Str> holds proj(a, b) => a, b {
    let local = List("long", "longer")
    let out = filter(local, s -> size(s) > 2)
    count(out)
    if first {
        out = filter(a, s -> size(s) > 1)
    } else {
        out = filter(b, s -> size(s) > 1)
    }
    return out
}

// Re-pointed in one arm that returns; the old view is used after.
fn early(ws: List<Str>, quit: Bool) [Console] -> Mut List<proj Str> holds proj(ws) => ws {
    let local = List("aa", "b")
    let out = filter(local, s -> size(s) > 1)
    if quit {
        out = filter(ws, s -> size(s) > 1)
        return out
    }
    println("early kept ${size(out)} of the local")
    return filter(ws, s -> size(s) > 5)
}

// Re-pointed on every turn of a loop at a local of the loop body, and in a
// second loop at the parameters, used after it.
fn turns(ws: List<Str>, more: List<Str>) [Console] -> None => ws, more {
    let total = List("t")
    let out = filter(total, s -> size(s) > 0)
    println("total kept ${size(out)}")
    for w in ws {
        let row = List(copy(w), "zz", "y")
        out = filter(row, s -> size(s) > 1)
        println("turn kept ${size(out)}")
    }
    let seen = filter(total, s -> size(s) > 0)
    for w in more {
        if size(w) > 1 {
            seen = filter(ws, s -> size(s) > 1)
        } else {
            seen = filter(more, s -> size(s) > 1)
        }
    }
    println("after the loop: ${size(seen)}")
}

// A re-pointed view that is not returned, its first source written after
// the re-point (Rust: the first borrow must have ended).
fn rewrite(ws: List<Str>) [Console] -> None => ws {
    let mine = Mut List("p", "qq")
    let out = filter(mine, s -> size(s) > 1)
    println("mine kept ${size(out)}")
    out = filter(ws, s -> size(s) > 1)
    add(mine, "rrr")
    println("ws kept ${size(out)}, mine has ${size(mine)}")
}

// Owned (an empty list of views) then borrowed, and borrowed then owned.
fn orders(ws: List<Str>) [Console] -> None => ws {
    let a: Mut List<proj Str> = Mut List()
    println("a starts ${size(a)}")
    a = filter(ws, s -> size(s) > 1)
    println("a then ${size(a)}")
    let b = filter(ws, s -> size(s) > 0)
    println("b starts ${size(b)}")
    b = Mut List()
    println("b then ${size(b)}")
}

// Ints: the view of Copy scalars is a list of its own.
fn ints() [Console] -> None {
    let ns = List(1, 5, 9)
    let big = filter(ns, n -> n > 2)
    println("ints kept ${size(big)}")
    big = filter(List(10, 1), n -> n > 2)
    println("ints then ${size(big)}")
}

// The value an arm or a loop stores flows out of it: one variable.
fn flows(ws: List<Str>, c: Bool) [Console] -> None => ws {
    let out = filter(ws, s -> size(s) > 5)
    if c {
        out = filter(ws, s -> size(s) > 0)
    }
    println("flows if ${size(out)}")
    for w in ws {
        out = filter(ws, s -> size(s) > 1)
        if size(w) > 0 {
            break
        }
    }
    println("flows break ${size(out)}")
    let n = 0
    while n < 2 {
        n = n + 1
        if n == 1 {
            out = filter(ws, s -> size(s) > 2)
            continue
        }
        println("flows continue ${size(out)}")
        out = filter(ws, s -> size(s) > 0)
    }
    println("flows while ${size(out)}")
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    println("pick ${size(pick(ws, s -> size(s) > 1))}")
    println("yielded ${size(yielded(ws))}")
    let a = List("aa", "bb", "c")
    let b = List("z")
    println("either ${size(either(a, b, true))} ${size(either(a, b, false))}")
    println("early ${size(early(ws, true))}")
    println("late ${size(early(ws, false))}")
    turns(ws, List("q", "rs"))
    rewrite(ws)
    orders(ws)
    ints()
    flows(ws, true)
    flows(ws, false)
}
"#;

#[test]
fn re_pointed_views() {
    let files = generate(&[("main.sv", REPOINT_DEMO)]);
    let main = &files.iter().find(|f| f.rel_path.ends_with("main.rs")).expect("main.rs").content;
    let seq = &files.iter().find(|f| f.rel_path.ends_with("std/core/seq.rs")).expect("seq.rs").content;
    // [rs-proj-generic] The fast path's view borrows: no clones, and Copy
    // elements are copied out at the call.
    assert!(seq.contains("-> Vec<&'a T> {"), "{seq}");
    assert!(main.contains(").into_iter().copied().collect::<Vec<_>>();"), "{main}");
    // [rs-repoint] A plain re-point in the declaring block shadows; arms
    // that all re-point share a fresh variable declared before them; an arm
    // that returns its re-point shadows inside it; a value that flows out of
    // its arm or loop stays an assignment.
    for needle in [
        "    let mut out: Vec<&String> = crate::std_core_seq::filter_platform::<String>(ws, &mut *keep);\n    return out;",
        "    let mut out: Vec<&String>;\n    if first {\n        out = ",
        "    if quit {\n        let mut out: Vec<&String> = ",
        "    if c {\n        out = ",
    ] {
        assert!(main.contains(needle), "missing `{needle}` in:\n{main}");
    }
}

// ===== [rs-proj-arm] a forwarded fn value into a retagged slot =====

/// [rs-proj-arm] [rs-fn-param-convention] [backend-never-wrong] ROADMAP 0y:
/// a fn *value* passed to a slot whose element generic the call retags to a
/// borrow (`filter`, `map`, `reduce` over a list iterator: `T = &String`, so
/// the slot is `&mut dyn FnMut(&&String)`) is adapted as a lambda or a named
/// fn already was: a closure peels the extra reference
/// (`&mut |__a0: &&String| keep(*__a0)`). The repro (a parameter) and its
/// neighbours: forwarded two levels down, a local bound from a lambda or a
/// named fn, a field read (also into a plain generic slot, whose call-through
/// closure was typed by the uninstantiated `&T`), a forwarded implicit, a
/// generic body's own parameter, and a fn value a call answers (an
/// `Arc<dyn Fn>`: returned wrapped, bound once and called through when passed,
/// directly or from a local). `kotlinc_compiles_and_runs_forwarded_fn_values`
/// runs the same program.
const FORWARDED_FN_DEMO: &str = r#"
struct Picker {
    keep: (Str) -> Bool
}

fn long(s: Str) -> Bool {
    return size(s) > 1
}

// The repro (ROADMAP 0y): a fn parameter forwarded to `filter` over an
// iterator, whose element the call retags to a borrow.
fn longs(ws: List<Str>, keep: (Str) -> Bool) [] -> Int => ws, keep {
    let it = iter(ws)
    return size(filter(it, keep))
}

// Received as a parameter and forwarded two levels down.
fn longs2(ws: List<Str>, keep: (Str) -> Bool) [] -> Int => ws, keep {
    return longs(ws, keep)
}

// A lambda stored in a local first.
fn local_lambda(ws: List<Str>) [] -> Int => ws {
    let k: (Str) -> Bool = s -> size(s) > 1
    let it = iter(ws)
    return size(filter(it, k))
}

// A named fn stored in a local first.
fn local_named(ws: List<Str>) [] -> Int => ws {
    let k: (Str) -> Bool = long
    let it = iter(ws)
    return size(filter(it, k))
}

// A field read, into a retagged slot and into a plain generic one.
fn field(ws: List<Str>, p: Picker) [] -> Int => ws, p {
    let it = iter(ws)
    return size(filter(it, p.keep))
}
fn field_any(ws: List<Str>, p: Picker) [] -> Bool => ws, p {
    return any(ws, p.keep)
}

// An implicit forwarded.
fn longs_i(ws: List<Str>, ?keep: (x: Str) -> Bool) [] -> Int => ws {
    let it = iter(ws)
    return size(filter(it, keep))
}

// `map` and `reduce` over an iterator.
fn lens(ws: List<Str>, f: (Str) -> Int) [] -> Int => ws, f {
    let it = iter(ws)
    let ns = map(it, f)
    return reduce(ns, 0, (a, n) -> a + n)
}
fn total(ws: List<Str>, f: (Int, Str) -> Int) [] -> Int => ws, f {
    let it = iter(ws)
    return reduce(it, 0, f)
}

// A generic body forwarding its own fn parameter.
fn glongs<T>(xs: List<T>, keep: (T) -> Bool) [] -> Int => xs, keep {
    let it = iter(xs)
    return size(filter(it, keep))
}

// A fn value a call answers: passed directly, and through a local.
fn picker() [] -> (Str) -> Bool {
    return long
}
fn from_call(ws: List<Str>) [] -> Int => ws {
    let it = iter(ws)
    return size(filter(it, picker()))
}
fn from_call_local(ws: List<Str>) [] -> Int => ws {
    let k = picker()
    let it = iter(ws)
    return size(filter(it, k))
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    println("param lambda ${longs(ws, s -> size(s) > 2)}")
    println("param named ${longs(ws, long)}")
    println("two levels ${longs2(ws, s -> size(s) > 1)}")
    println("local lambda ${local_lambda(ws)}")
    println("local named ${local_named(ws)}")
    println("field ${field(ws, Picker { keep: long })}")
    println("field any ${field_any(ws, Picker { keep: s -> size(s) > 2 })}")
    println("implicit ${longs_i(ws, keep = long)}")
    println("map ${lens(ws, s -> size(s))}")
    println("reduce ${total(ws, (a, s) -> a + size(s))}")
    println("generic ${glongs(ws, long)}")
    println("call ${from_call(ws)}")
    println("call local ${from_call_local(ws)}")
    let k = picker()
    println("call any ${any(ws, k)} ${any(ws, picker())} ${k("x")}")
}
"#;

#[test]
fn forwarded_fn_values() {
    let files = generate(&[("main.sv", FORWARDED_FN_DEMO)]);
    let main = &files.iter().find(|f| f.rel_path.ends_with("main.rs")).expect("main.rs").content;
    for needle in [
        // A parameter (and a forwarded implicit) peels the retagged borrow.
        "&mut |__a0: &&String| keep(*__a0)",
        // A local closure, a field, a generic body's parameter.
        "&mut |__a0: &&String| k(*__a0)",
        "&mut |__a0: &&String| (p.keep)(*__a0)",
        "&mut |__a0: &&T| keep(*__a0)",
        // A convention change and a retag in one adapter.
        "&mut |__a0: &i32, __a1: &&String| f((*__a0).clone(), *__a1)",
        // A plain generic slot types the call-through closure as instantiated.
        "crate::std_core_list::any::<String>(ws, &mut |__a0: &String| (p.keep)(__a0))",
        // A fn value a call answers: returned as an `Arc`, passed through it.
        "return std::sync::Arc::new(move |__a0: &String| crate::long(__a0));",
        "&mut { let __f = crate::picker(); move |__a0: &&String| __f(*__a0) }",
        "&mut |__a0: &&String| (k)(*__a0)",
        // Into a slot of the value's own convention, still passed as it is.
        "return crate::longs(ws, &mut *keep);",
    ] {
        assert!(main.contains(needle), "missing `{needle}` in:\n{main}");
    }
}

// ===== [rs-proj-flat] a borrowed element read through a generic `proj T` =====

/// [rs-proj-flat] [backend-never-wrong] ROADMAP 0z: a std fn whose result is
/// a generic `proj T` (rendered `&T`), called on a list of borrows
/// (`T = &String`), answers `&&String` where Salvo's type is `proj Str`
/// (`proj proj Str` is `proj Str`); the outer reference is copied out — on
/// `get`/`last` (`.copied()`), `first` (`*`), and a pass's `next`, in a
/// `for` loop and in every `?Next` adapter (`U1(*__v)`). The call's
/// instantiation is read off its arguments where the IR states no type
/// arguments (a `for` loop's `next`, a forwarding closure's call).
/// `kotlinc_compiles_and_runs_borrowed_element_reads` runs the same program.
const BORROWED_ELEMENT_DEMO: &str = r#"
struct Bag {
    names: List<proj Str>
}

// A `proj` element list in a struct field.
fn show(b: Bag) [use] -> None {
    use StdConsole
    for n in iter(b.names) {
        println("field for ${n}")
    }
    let g = get(b.names, 0) ?: unreachable!()
    println("field get ${g}")
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    let vs = filter(iter(ws), s -> size(s) > 1)
    // The repro (ROADMAP 0z).
    for v in vs {
        println("list for ${v}")
    }
    let h = get(vs, 0) ?: unreachable!()
    println("get ${h}")
    for v in iter(vs) {
        println("iter for ${v}")
    }
    // Its neighbours.
    let l = last(vs) ?: unreachable!()
    println("last ${l}")
    if vs is NonEmpty {
        println("first ${first(vs)}")
    }
    let ms = map(iter(vs), s -> size(s))
    println("map ${size(ms)} ${get(ms, 1) ?: 0}")
    let fs = filter(iter(vs), s -> size(s) > 2)
    for f in fs {
        println("filter ${f}")
    }
    println("count ${count(iter(vs))}")
    let at = find_first(vs, s -> size(s) > 2) ?: unreachable!()
    println("find_first ${at}")
    println("reduce ${reduce(iter(vs), 0, (a, s) -> a + size(s))}")
    let t = to_list(iter(vs))
    for x in iter(t) {
        println("to_list ${x}")
    }
    let m = to_map(vs, v -> (copy(v), v))
    let mv = get(m, "def") ?: unreachable!()
    println("map get ${mv}")
    show(Bag {names: vs})
    // Nested: a `List<proj List<Str>>`.
    let ls = List(List("p", "q"), List("r"))
    let big = filter(iter(ls), x -> size(x) > 0)
    let fst = get(big, 0) ?: unreachable!()
    println("nested get ${size(fst)}")
    for b in iter(big) {
        println("nested for ${size(b)}")
    }
}
"#;

#[test]
fn borrowed_element_reads() {
    let files = generate(&[("main.sv", BORROWED_ELEMENT_DEMO)]);
    let main = &files.iter().find(|f| f.rel_path.ends_with("main.rs")).expect("main.rs").content;
    for needle in [
        // `get` and `last` copy the outer reference out of the optional.
        "let mut __pick_3: Option<&String> = crate::std_core_list::get_platform(&vs, 0i32).copied();",
        "crate::std_core_list::last::<&String>(&vs).copied()",
        "(*crate::std_core_list::first::<&String>(&vs))",
        // A `for` loop's `next`, whose instantiation the arguments state.
        "= (match crate::std_core_list::next__ListNext(&mut __pass_6) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) });",
        // A `?Next` adapter.
        "&mut |__a0: &mut crate::std_core_list::ListNext<'_, &String>| (match crate::std_core_list::next__ListNext(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v),",
        // Nested.
        "let mut __pick_25: Option<&Vec<String>> = crate::std_core_list::get_platform(&big, 0i32).copied();",
    ] {
        assert!(main.contains(needle), "missing `{needle}` in:\n{main}");
    }
}

// ===== [fn-kept-borrow] a kept owned parameter reads a borrowed argument =====

/// [fn-kept-borrow] [rs-proj-arm] A written fn value whose parameter is an
/// owned `Str` it keeps fills a slot whose parameter is `proj Str` (user
/// decision 2026-10-14, ROADMAP §7 option (b)). Rust adapts the convention:
/// a named fn's forwarding closure peels the element's extra reference
/// (`|__a0: &&String| { let __a0 = *__a0; crate::longer(__a0) }`), a
/// forwarded parameter goes through `fn_adapter` (`keep(*__a0)`), a slot
/// written `proj` passes the borrow as it is (`|__a0: &String|
/// crate::longer(__a0)`), and an owned key at a slot the call instantiates as
/// a borrow is lent twice (`&&String::from("def")`). The resolved implicits
/// over the view type (`hash`, `eq`, `cmp`, `to_str` of `Str`) take the same
/// rule. `kotlinc_compiles_and_runs_kept_fns_over_views` runs the same program.
const KEPT_BORROW_DEMO: &str = r#"
fn long(s: Str) -> Bool {
    return size(s) > 1
}

fn longer(s: Str) -> Bool {
    return size(s) > 2
}

fn len(s: Str) -> Int {
    return size(s)
}

fn add_len(a: Int, s: Str) -> Int {
    return a + size(s)
}

// A parameter forwarded into a view-element slot.
fn keep_count(vs: List<proj Str>, keep: (Str) -> Bool) [] -> Int => vs, keep {
    return count(vs, keep)
}

// A slot whose parameter is written `proj`.
fn count_views(vs: List<proj Str>, keep: (x: proj Str) -> Bool) [] -> Int => vs, keep {
    return reduce(vs, 0, (a, v) -> if keep(v) { a + 1 } else { a })
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("def", "c", "ab")
    let vs = filter(iter(ws), s -> size(s) > 1)
    // Each combinator over the view list, with a named kept fn.
    println("any ${any(vs, longer)} ${any(vs, s -> size(s) > 5)}")
    println("all ${all(vs, long)} ${all(vs, longer)}")
    println("count ${count(vs, longer)}")
    let at = find_first(vs, longer) ?: unreachable!()
    println("find_first ${at}")
    let ns = map(vs, len)
    println("map ${size(ns)} ${get(ns, 0) ?: 0} ${get(ns, 1) ?: 0}")
    let fs = filter(vs, longer)
    for f in fs {
        println("filter ${f}")
    }
    println("reduce ${reduce(vs, 0, add_len)}")
    // By a parameter, into a written `proj` slot, and through a local.
    println("param ${keep_count(vs, longer)} ${keep_count(vs, long)}")
    println("slot ${count_views(vs, longer)}")
    let k: (proj Str) -> Bool = longer
    println("local ${k(get(vs, 1) ?: unreachable!())} ${count_views(vs, k)}")
    // Resolved implicits over the view type: `hash`, `eq`, `cmp`, `to_str`.
    let m = to_map(vs, v -> (v, size(v)))
    let n = get(m, "def") ?: unreachable!()
    println("map ${n} ${size(m)} ${contains_key(m, "ab")}")
    let st = to_set(vs)
    println("set ${size(st)} ${contains(st, "ab")} ${contains(st, "zz")}")
    println("list ${contains(vs, "ab")} ${eq(vs, vs)} ${cmp(vs, vs)}")
    println("to_str ${to_str(vs)} ${to_str(sort(vs))}")
}
"#;

#[test]
fn kept_fns_over_views() {
    let files = generate(&[("main.sv", KEPT_BORROW_DEMO)]);
    let main = &files.iter().find(|f| f.rel_path.ends_with("main.rs")).expect("main.rs").content;
    for needle in [
        "any::<&String>(&vs, &mut |__a0: &&String| { let __a0 = *__a0; crate::longer(__a0) })",
        "reduce__List_A_Fn::<&String, i32>(&vs, 0i32, &mut |__a0: &i32, __a1: &&String| { let __a1 = *__a1; crate::add_len(*__a0, __a1) })",
        "count::<&String>(vs, &mut |__a0: &&String| keep(*__a0))",
        "crate::count_views(&vs, &mut |__a0: &String| crate::longer(__a0))",
        "let mut k = |__a0: &String| crate::longer(__a0);",
        "get_platform(&m, &&String::from(\"def\")",
    ] {
        assert!(main.contains(needle), "missing `{needle}` in:\n{main}");
    }
}

/// [fn-kept-borrow] Only a *kept* parameter reads a borrow: a fn consuming
/// its parameter (`=> !s`) is refused at a view-element slot, by an overload,
/// an annotated local and an override, with the reason and the remedies; and
/// a `Mut` difference stays invariant ([fn-erased-variance]): neither a
/// mutating `grow(xs: Mut List<Str>)` nor a reading `peek(xs: List<Str>)`
/// fills a slot whose parameter is a `proj Mut` list.
#[test]
fn a_consuming_fn_does_not_read_a_borrow() {
    let errors = expect_errors(
        r#"
fn eat(s: Str) -> Bool => !s {
    return size(s) > 1
}

fn same(a: Str, b: Str) -> Bool => !a, b {
    return a == b
}

fn grow(xs: Mut List<Str>) -> Bool => xs: Mut {
    add(xs, "x")
    return true
}

fn peek(xs: List<Str>) -> Bool => xs {
    return size(xs) > 0
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    let vs = filter(iter(ws), s -> size(s) > 1)
    println("any ${any(vs, eat)}")
    let k: (proj Str) -> Bool = eat
    println("has ${contains(vs, "ab", eq = same)}")
    let ls = List(Mut List("p"), Mut List("q"))
    let big = filter(iter(ls), x -> size(x) > 0)
    println("grow ${any(big, grow)}")
    println("peek ${any(big, peek)}")
}
"#,
    );
    let consumed = "consumes its parameter `s` (`=> !s`), and here it would be given a borrowed `proj Str`, which cannot be given away. Keep the parameter, or pass a lambda that copies the argument";
    for needle in [
        format!("no matching overload for `any(Mut List<proj Str>, (Str) -> Bool)`: `eat` {consumed}"),
        format!("expected `(proj Str) -> Bool`, found `(Str) -> Bool`: `eat` {consumed}"),
        "`eq` does not fit here: `same` consumes its parameter `a` (`=> !a`)".to_string(),
        "no matching overload for `any(Mut List<proj Mut NonEmpty List<Str>>, (Mut List<Str>) -> Bool)`".to_string(),
        "no matching overload for `any(Mut List<proj Mut NonEmpty List<Str>>, (List<Str>) -> Bool)`".to_string(),
    ] {
        assert!(errors.iter().any(|e| e.contains(&needle)), "missing `{needle}` in {errors:?}");
    }
}


// ===== [fate-lambda-view] a capture-rooted projection, clone-free =====

/// A callback picking elements out of a *captured* list — the projection
/// is rooted in the capture, which no fn type can name, so the closure
/// itself carries the link [fate-lambda-view]. The emitted Rust must hold the
/// borrow (`Vec<&String>`, no clone in the lambda) and deref the retagged
/// index in the cast (`(*i) as usize` — E0606 without it; both were open
/// defects, closed 2026-09-12).
const PICK_LIST_DEMO: &str = r#"
fn main() [use] {
    use StdConsole()
    let all = List("a", "b", "c", "d")
    let indices = List(1, 3)
    let picked = map(indices, i -> (get(all, i) ?: unreachable!()))
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

#[test]
fn a_capture_rooted_projection() {
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
        main.contains("&mut |mut i| -> &String {")
            && main.contains("let mut __pick_3: Option<&String> = crate::std_core_list::get_platform::<String>(&all, *i);")
            && !main.contains("__picked_4.clone()"),
        "expected a clone-free, deref'd pick lambda:\n{main}"
    );
}


// ===== collections =====

/// [col-insertion-order] [rs-collections] Set and Map, whose observable
/// behavior is the part that has to be identical on both backends: the
/// iteration order, a position-preserving overwrite, an order-preserving
/// removal that hands the value back, and last-wins duplicate keys. It runs
/// on both backends in the run data ([test-run-data], `collections/`).
const COLLECTIONS_DEMO: &str = r#"
fn main() [use] -> None {
    use StdConsole()

    let s: Mut Set<Str> = Mut Set("b", "a", "c")
    println("set: ${to_str(s)} size: ${size(s)}")
    let added_new = add(s, "d")
    let added_dup = add(s, "a")
    println("add new: ${added_new} dup: ${added_dup} now: ${to_str(s)}")
    let has_a = contains(s, "a")
    let gone = remove(s, "a")
    let gone_again = remove(s, "a")
    println("contains: ${has_a} remove: ${gone} again: ${gone_again} left: ${to_str(s)}")
    let dedup = Set(1, 2, 1, 3)
    println("dedup: ${to_str(dedup)} size: ${size(dedup)}")

    let m: Mut Map<Str, Int> = Mut Map(("one", 1), ("two", 2))
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
    let dup_keys = Map(("x", 1), ("y", 2), ("x", 9))
    println("dup keys: ${to_str(dup_keys)}")
    let ints: Mut Map<Int, Str> = Mut Map()
    put(ints, 7, "seven")
    put(ints, 3, "three")
    println("int keys: ${to_str(ints)}")
}
"#;

/// [platform-value-type] The keyed collections are std's host types
/// (`lib/platform/std/core/{set,map,sorted}.rs`), so no runtime module is emitted
/// for them (ROADMAP §0j step 7), and the emitted code names Rust's unordered
/// `HashMap`/`HashSet` nowhere [col-insertion-order].
#[test]
fn the_collections_are_std_host_types() {
    let files = generate(&[("main.sv", COLLECTIONS_DEMO)]);
    assert!(
        !files.iter().any(|f| f.rel_path.ends_with("collections.rs")),
        "the collections runtime module is gone"
    );
    let main = files
        .iter()
        .find(|f| f.rel_path.ends_with("main.rs"))
        .expect("main.rs");
    assert!(
        !main.content.contains("HashMap<") && !main.content.contains("HashSet<"),
        "emitted code should use the ordered types, not Rust's unordered ones: {}",
        main.content
    );
    assert!(
        files.iter().any(|f| f.rel_path.to_string_lossy() == "platform/std/core/map.rs"),
        "the map's host file is carried"
    );
}


// ===== C-6 the claims a list can carry [col-nonempty] [col-sorted-list] =====

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
    use StdConsole()

    // NonEmpty by construction: the element constructor requires a first, so
    // `first` answers with an element, not `Int?` [col-of-nonempty].
    let built = List(10, 20)
    println("built ${first(built)}")

    // NonEmpty by refinement: `add` establishes the claim on the qualifier's
    // behalf, so `first` resolves to the same overload.
    let grown: Mut List<Int> = Mut List()
    add(grown, 7)
    println("grown ${first(grown)}")

    // NonEmpty by test.
    let maybe = List(1, 2)
    if maybe is NonEmpty {
        println("tested ${first(maybe)}")
    }

    // Sorted, minted by `sort`; equal elements answer the lowest index.
    let ordered = sort(List(5, 1, 4, 1, 3))
    println("sorted ${to_str(ordered)}")
    if binary_search(ordered, 1) is Int at {
        println("lowest 1 at ${at}")
    }
    if binary_search(ordered, 9) is None {
        println("9 absent")
    }

    // add_sorted keeps the claim across the mutation.
    let live: Mut Sorted List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
    add_sorted(live, 5)
    println("still sorted ${to_str(live)}")

    // Strings order by code point on both backends.
    let words = sort(List("pear", "Apple", "fig"))
    println("words ${to_str(words)}")

    // [cmp-carry] …unless the claim names another ordering, which is what the
    // slot is for: `by_len` sorts, and `binary_search` and `add_sorted` are
    // computed with it because the list's *type* carries it. Under the
    // code-point order none of these three lines would come out this way.
    let bylen = sort(List("pear", "fig", "Apple"), cmp = by_len)
    println("by length ${to_str(bylen)}")
    if binary_search(bylen, "kiwi") is Int at {
        println("a 4-letter word at ${at}")
    }
    let growing: Mut Sorted List<Str> = mut_sort(List("pear", "fig", "Apple"), cmp = by_len)
    add_sorted(growing, "durian")
    println("still by length ${to_str(growing)}")

    // Distinct, minted by a set.
    let unique = to_list(Set(3, 1, 3, 2))
    println("distinct ${to_str(unique)} of ${count_unique(unique)}")

    // [qual-overload] The same claim over four more subjects: `NonEmpty` is
    // one name whose meaning the subject decides.
    let seen: Mut Set<Str> = Mut Set()
    add(seen, "a")
    if seen is NonEmpty {
        println("set claim ${size(seen)}")
    }
    let tally: Mut Map<Str, Int> = Mut Map()
    put(tally, "k", 1)
    if tally is NonEmpty {
        println("map claim ${size(tally)}")
    }
    let ranked = SortedSet(30, 10, 20)
    if ranked is NonEmpty {
        let lo = min(ranked)
        let hi = max(ranked)
        println("sorted claim ${lo}..${hi}")
    }
    let bykey = SortedMap(("b", 2), ("a", 1))
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
    use StdConsole()

    // [col-bounds] A total exchange, and the one positional write a list of
    // obligations can have: nothing enters, nothing leaves, nothing is dropped.
    let xs: Mut List<Str> = Mut List("a", "b", "c")
    println("swapped ${detach(swap(xs, 0, 2))} ${to_str(xs)}")
    println("in place ${detach(swap(xs, 1, 1))} ${to_str(xs)}")
    println("past the end ${detach(swap(xs, 0, 3))} ${to_str(xs)}")
    println("negative ${detach(swap(xs, -1, 0))} ${to_str(xs)}")

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
    let text = Mut Str("hello")
    println("wrote ${set(text, 0, 'H')} ${text}")
    println("write past the end ${set(text, 9, 'X')} ${text}")
    let buf = mut_bytes(bytes_of(to_byte(1), to_byte(2)))
    println("wrote a byte ${set(buf, 1, to_byte(9))} ${to_hex(buf)}")
    println("byte past the end ${set(buf, 2, to_byte(9))} ${to_hex(buf)}")

    // The heap's own move, in the spelling it uses [fn-dot].
    let heap: Mut List<Int> = Mut List(5, 9, 7)
    ignore(heap.swap(0, 2))
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
export qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> +Heap<T>(?cmp) Mut List<T> {
    return Mut List()
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
            if child + 1 < size(list) && cmp(list.get(child + 1) ?: unreachable!(), list.get(child) ?: unreachable!()) < 0 {
                smaller += 1
            }
            if cmp(list.get(at) ?: unreachable!(), list.get(smaller) ?: unreachable!()) <= 0 {
                break
            }
            ignore(list.swap(at, smaller))
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
        if cmp(heap.get(parent) ?: unreachable!(), heap.get(i) ?: unreachable!()) <= 0 {
            break
        }
        ignore(heap.swap(i, parent))
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
    ignore(heap.swap(0, last))
    let least = heap.remove_at(last) ?: unreachable!()
    let i = 0
    while i < size(heap) {
        let child = i * 2 + 1
        if child >= size(heap) {
            break
        }
        let smaller = copy(child)
        if child + 1 < size(heap) && cmp(heap.get(child + 1) ?: unreachable!(), heap.get(child) ?: unreachable!()) < 0 {
            smaller += 1
        }
        if cmp(heap.get(i) ?: unreachable!(), heap.get(smaller) ?: unreachable!()) <= 0 {
            break
        }
        ignore(heap.swap(i, smaller))
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
    let out = Mut Str()
    while heap_pop(heap) is Int n {
        append(out, "${n} ")
    }
    println("${out}")
}

fn main() [use] {
    use StdConsole()

    let h = empty_heap<Int>()
    heap_push(h, 5)
    heap_push(h, 3)
    heap_push(h, 9)
    heap_push(h, 1)
    heap_push(h, 7)
    println("root ${first(h) ?: unreachable!()}")
    drain_heap(h)

    // A list nobody claimed anything about, made a heap by a function that
    // establishes the claim [deduce-reapply] — and then read by the same
    // surface, which only accepts a heap.
    let raw: Mut List<Int> = Mut List(8, 2, 6, 4)
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

// ===== [fn-variadic] mixing plain arguments with a `...spread` =====

/// The shape: a mixed tail is **assembled** into one vector in written order,
/// and — because that vector is fresh — it is *not* cloned again by the
/// constructor, which a borrowed forward would be (`parts.clone()`, asserted
/// in `a_spread_into_a_variadic_intrinsic_is_the_collection`).
#[test]
fn a_mixed_variadic_tail_is_assembled_once() {
    let src = r#"
export fn main() [use] {
    use StdConsole()
    let rest = array_of(2, 3)
    let all = List(1, ...rest)
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
        main.contains("{ let mut __v = vec![1i32]; __v.extend(rest.clone().into_iter()); __v }"),
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
    let out = Mut Str(...parts)
    println("${sep} ${out}")
}

fn main() [use] {
    use StdConsole()
    let rest = array_of(2, 3)
    let all = List(1, ...rest)
    println("List ${to_str(all)}")

    // The spread source is still usable: a variadic position is not tracked,
    // so the store clones rather than moves.
    println("reusable ${size(rest)}")

    let s = Set(9, ...rest)
    println("Set ${to_str(s)}")
    let sorted = SortedSet(9, ...rest)
    println("SortedSet ${to_str(sorted)}")

    let pairs = array_of(("b", 2))
    let m = Map(("a", 1), ...pairs)
    println("Map ${to_str(m)}")

    let words = array_of("b", "c")
    let joined = Mut Str("a", ...words)
    println("mut_str ${joined}")
    join_all("ordinary", "x", ...words)

    // A lone spread still forwards the whole collection — into a purely
    // variadic constructor. `List` is not one any more [col-of-nonempty]:
    // its element shape takes a *first*, which a spread may not supply.
    let lone = Set(...rest)
    println("lone ${to_str(lone)}")
}
"#;

pub const MIXED_SPREAD_OUTPUT: &str = "List [1, 2, 3]\n\
     reusable 2\n\
     Set {9, 2, 3}\n\
     SortedSet {2, 3, 9}\n\
     Map {a: 1, b: 2}\n\
     mut_str abc\n\
     ordinary xbc\n\
     lone {2, 3}\n";

// ===== [kt-variadic] a user-declared variadic, every element type =====

/// Shared verbatim with the Kotlin backend's
/// `kotlinc_compiles_and_runs_user_variadics`. The Kotlin side is the one this
/// exists for: a `vararg` of a *primitive* element type is an `IntArray`, not
/// an `Array<Int>`, and those are unrelated on the JVM — so the parameter is
/// emitted as a plain array instead. Rust is here to hold the parity.
pub const USER_VARIADIC_DEMO: &str = r#"
fn count_ints(label: Str, ...ns: Int[]) [Console] -> Int => label, ns {
    let seen = Mut List(0)
    for n in iter(ns) {
        add(seen, copy(n))
    }
    println("${label} ${size(ns)}")
    return size(seen)
}

fn join_strs(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let joined = Mut Str(...parts)
    println("${sep} ${joined}")
}

fn main() [use] {
    use StdConsole()

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
    use StdConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    println(close(InFile { id: 1 }))
    println(close@Vault(OutFile { id: 2 }))
}
"#;

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
        // The trait: each `close` suffixed by its parameter's type
        // [fn-emit-name], and the un-overloaded member keeps its own name.
        // [rs-borrows] The **consuming** overloads take their parameter by
        // value (`=> !f`) while the keeping `describe` borrows — a member's
        // written clause is its whole contract.
        "fn close__InFile(&mut self, f: crate::InFile) -> String;",
        "fn close__OutFile(&mut self, f: crate::OutFile) -> String;",
        "fn describe(&mut self, f: &crate::InFile) -> String;",
        // The handler implements both under the trait's names, with the
        // trait's modes.
        "impl crate::__Stateless_Vault for Files {",
        "fn close__InFile(&self, f: crate::InFile) -> String {",
        "fn close__OutFile(&self, f: crate::OutFile) -> String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    // The call sites, the `OutFile` one written with the `@Vault` selector.
    assert!(
        src.contains(".close__InFile(crate::InFile {") && src.contains(".close__OutFile(crate::OutFile {"),
        "expected both call sites to name their own overload and pass the \
         consumed argument owned, got:\n{src}"
    );
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
    use StdConsole()
    use Direct()
    use Doubling()
    let t: Mut Token = Mut Token { id: 1 }
    bump(t)
    println("bumped ${t.id}")
    println("took ${take(t)}")
}
"#;

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
        "fn take(&mut self, t: crate::Token) -> i32;",
        "fn bump(&mut self, t: &mut crate::Token);",
        // The handlers, independent and dependent alike (stateless: `&self`).
        "fn take(&self, t: crate::Token) -> i32 {",
        "fn bump(&self, t: &mut crate::Token) {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
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
    use StdConsole()
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
        "fn close__InTape(&mut self, s: crate::InTape) -> String;",
        "fn close__OutTape(&mut self, s: crate::OutTape) -> String;",
        "fn read_line(&mut self, s: &mut crate::InTape) -> String;",
        // A discharged token is dropped, so `discard` emits nothing that
        // could resurrect it: the value simply ends there. (The handler is
        // stateless, so its receiver is `&self` [rs-handle].)
        "fn close__InTape(&self, s: crate::InTape) -> String {",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
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

/// [platform-value-type] `Bytes` is a value platform type whose Rust host is
/// `pub type Bytes = Vec<u8>` — unboxed, no class — re-exported from
/// `core::bytes`, and its operations are the platform wrappers.
#[test]
fn bytes_is_a_value_platform_type() {
    let files = generate(&[("main.sv", BYTES_PROGRAM)]);
    let core = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "std/core/bytes.rs")
        .expect("core/bytes.rs emitted");
    assert!(core.content.contains("pub use crate::platform_std_core_bytes::Bytes;"), "{}", core.content);
    let host = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "platform/std/core/bytes.rs")
        .expect("std's host file for core.bytes travels with the program");
    assert!(host.content.contains("pub type Bytes = Vec<u8>;"), "{}", host.content);
}

// ===== std's filesystem [platform-handler] [linear-group] =====

/// std's `fs`, exercised end to end against real files: the
/// `platform handler HostRawFs` at the bottom, `DefaultFs [RawFs]` above it,
/// linear stream tokens discharged by an effect member, the `Lines` pass,
/// the one-shots, a `position` after a ranged open, and a failure path whose
/// `Checked<FsError>` is acknowledged once. `__DIR__` is replaced with a
/// scratch directory: the program makes it, works in it and removes it, so
/// nothing is left behind and the output mentions no paths.
///
/// The Kotlin backend runs this program verbatim, and both must print these
/// lines byte for byte — the whole point of the layering is that only the
/// host file differs.
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

/// [rs-platform-handler] What the layering emits: no struct for the platform
/// handler, `DefaultFs` as an ordinary Salvo handler reaching `RawFs` through
/// the fusion, and the consuming member taking its token **by value** so the
/// discharge is a move rather than a clone.
#[test]
fn the_fs_surface_emits_a_host_seam_and_owned_tokens() {
    let files = generate(&[("main.sv", &fs_program())]);
    let file = |rel: &str| -> String {
        files
            .iter()
            .find(|f| f.rel_path == std::path::Path::new(rel))
            .unwrap_or_else(|| panic!("{rel} was not emitted"))
            .content
            .clone()
    };
    let surface = file("std/stream.rs");
    for expected in [
        "pub trait __Stateful_Streams: Send {",
        // The token is consumed: by value, not `&InStream`.
        "fn close__InStream(&mut self, s: crate::std_stream::InStream)",
        // …and kept members borrow it.
        "fn read_line(&mut self, s: &crate::std_stream::InStream) -> Option<String>",
        // [effect-available] One overload set: the pass's discharger is a
        // *fn* named `close`, beside the two members of that name.
        "pub fn close__Lines(streams: &crate::std_stream::Streams, mut p: crate::std_stream::Lines)",
    ] {
        assert!(
            surface.contains(expected),
            "expected `{expected}` in:\n{surface}"
        );
    }
    // The host seams are their own modules [mod-used-only]: a program that
    // never opens a file links none of them.
    let host = file("std/fs/host.rs");
    for expected in ["pub trait __Stateful_RawFs: Send {", "pub struct DefaultFs"] {
        assert!(host.contains(expected), "expected `{expected}` in:\n{host}");
    }
    assert!(
        !host.contains("pub struct HostRawFs"),
        "the platform handler's struct is the host's:\n{host}"
    );
    let stream_host = file("std/stream/host.rs");
    // [stream-table] `HostRawStreams` is Salvo over the runtime's table.
    for expected in ["fn raw_close_read(&self, handle: i64)", "pub struct DefaultStreams", "pub struct HostRawStreams"] {
        assert!(stream_host.contains(expected), "expected `{expected}` in:\n{stream_host}");
    }
    // std ships the host files, and they travel into the output.
    for rel in ["platform/std/fs/host.rs", "platform/std/runtime/streams.rs"] {
        assert!(
            files.iter().any(|f| f.rel_path == std::path::Path::new(rel)),
            "expected std's host companion {rel} to be emitted"
        );
    }
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

fn generate_actor_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", ACTOR)])
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

/// [actor-monitor-handler] [rs-handle] The monitor spawn (SH-3, user decision
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

fn generate_monitor_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", MONITOR)])
}

/// [rs-handle] What the monitor lowering *is*, asserted on the
/// generated text: the per-effect handle (`Arc<Mutex<dyn E + Send>>`)
/// implementing the effect's trait by lock-and-delegate, the spawn building
/// one with no scheduler call, and the handle passing into the actor's
/// dependency clause as itself (cloned, not stub-wrapped).
#[test]
fn a_monitor_lowers_to_the_effects_handle() {
    let files = generate_monitor_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    assert!(
        main.content.contains("pub struct Random {\n    inner: __Inner_Random,")
            && main.content.contains("Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Random>>),"),
        "the handle is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("pub fn next(&self) -> i32 {")
            && main.content.contains("__Inner_Random::Locked(h) => h.lock().unwrap().next(),"),
        "the handle does not implement the effect:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut rng: crate::Random = crate::Random::locked(crate::CyclicRandom::new(12345i32));"),
        "the monitor spawn is missing:\n{}",
        main.content
    );
    assert!(
        main.content.contains("__Inner_Random::Locked(h) => h.lock().unwrap().next(),"),
        "a member does not lock and delegate:\n{}",
        main.content
    );
    assert!(
        !main.content.contains("__Stub_Random") && !main.content.contains("__Lock_") && !main.content.contains("__Share_"),
        "a plain effect gets the handle and nothing else:\n{}",
        main.content
    );
}

/// [actor-mixed-handler] [rs-mixed] The mixed handler (SH-1): state confined to
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

fn generate_mixed_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", MIXED)])
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
        main.content.contains("pub enum __Priv_CyclicRandom {"),
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
        main.content.contains("impl crate::__Stateless_Random for __Fac_CyclicRandom {"),
        "the façade does not implement the effect:\n{}",
        main.content
    );
    assert!(
        main.content.contains(
            "crate::scheduler::salvo_send(self.__addr, std::boxed::Box::new(crate::__Priv_CyclicRandom::Advance("
        ),
        "the façade send is missing:\n{}",
        main.content
    );
    assert!(
        main.content
            .contains("crate::Random::shared(crate::__Fac_CyclicRandom { __addr: __a, seed: __c0 })"),
        "the mixed spawn does not answer the handle over the façade:\n{}",
        main.content
    );
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
    let files = generate(&[("main.sv", MIXED_DEPS)]);
    let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.rs").expect("main.rs");
    // [rs-mixed] The servant is built with the dependencies; the façade
    // carries only the addr and the constructor parameters.
    assert!(
        main.content.contains("let __h = crate::Counted::new(__c0.clone(), ")
            && main.content.contains("crate::Label::shared(crate::Hash::new())")
            && main.content.contains("crate::__Fac_Counted { __addr: __a, seed: __c0 }"),
        "the servant does not take the dependencies:\n{}",
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

fn generate_dep_spawn_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", DEP_SPAWN)])
}

/// [rs-actor] [rs-handle] The shape a dependent spawn lowers to: the same
/// handler struct a `use` builds, its dependencies handle fields filled from
/// the clause — a construction behind its handle for `Log`, the send stub
/// behind its handle for the `eff[Tally]` — and an actor body that owns the
/// instance and dispatches onto its members; no provider, no view.
#[test]
fn a_dependent_spawn_hands_the_actor_its_handles() {
    let files = generate_dep_spawn_demo();
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        text.contains("__dep0: crate::Log,")
            && text.contains("__dep1: crate::Tally,"),
        "the dependencies are handle fields:\n{text}"
    );
    assert!(
        text.contains("self.__dep0.note(") && text.contains("self.__dep1.tick(n)"),
        "members reach their dependencies through the fields:\n{text}"
    );
    assert!(
        text.contains("std::boxed::Box::new(crate::__Actor_Counting::new(__h))")
            && text.contains("crate::__Stateless_Counter::bump(&mut *__handler, n)"),
        "the actor body owns the handler and dispatches onto it:\n{text}"
    );
    // The clause's two kinds, in the handler's declaration order.
    assert!(
        text.contains(
            "crate::Counting::new(crate::Log::locked(crate::Recording::new()), crate::Tally::shared(crate::__Stub_Tally::new(tally)))"
        ),
        "the spawn does not build the handles from its clause:\n{text}"
    );
    assert!(
        !text.contains("__Prov_") && !text.contains("__Deps_") && !text.contains("__Impl_"),
        "no provider machinery remains:\n{text}"
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

fn generate_replyto_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", REPLYTO_CHAIN)])
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
            .contains("drain::<crate::scheduler::SalvoReply>(std::mem::take(&mut self.waiting), "),
        "the drain of a state field is not a take:\n{}",
        main.content
    );
    // [net-addr-routable] A `send(reply, v)` with a payload that has a wire form
    // takes the typed path, which receives the token by value all the same.
    assert!(
        main.content.contains("let mut next_1 = next.unwrap();\n            crate::scheduler::salvo_reply_wire::<String>(next_1, "),
        "a taken obligation must be moved, not cloned:\n{}",
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
        text.contains("impl crate::__Stateful_Timer for ManualTime {")
            && text.contains("impl crate::__Stateful_TimerCtl for ManualTime {"),
        "one trait impl per face is missing:\n{text}"
    );
    assert!(
        text.contains("pub fn __dispatch_ManualTime_Timer(__handler: &mut crate::ManualTime, mut __msg: crate::__Msg_Timer)")
            && text.contains("pub fn __dispatch_ManualTime_TimerCtl(__handler: &mut crate::ManualTime, mut __msg: crate::__Msg_TimerCtl)"),
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
        text.contains("impl crate::__Stateful_Tally for Counting {")
            && text.contains("impl crate::__Stateful_Stats for Counting {"),
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
            .any(|e| e.contains("mints a continuation with `replyto`, or names its own address with `self@…`, so it can only be `spawn`ed")),
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

fn generate_addr_stub_demo() -> Vec<salvo_backend_rust::EmittedFile> {
    generate(&[("main.sv", ADDR_STUB)])
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

/// [proj-infer-fn-type] [rs-proj-lends] A fn-typed slot whose **return is
/// opaque** — a bare type parameter, which every pass-minting slot is — lends
/// its kept positions without saying so (user decision 2026-09-26). Before
/// this, the most ordinary generic-iteration signature in the language emitted
/// a closure type with an anonymous input lifetime while the pass it returns
/// holds a borrow of the input, and rustc said "lifetime may not live long
/// enough" about code nobody wrote. Kotlin ran it, so it was an accept/reject
/// divergence [backend-parity].
///
/// Two parameters, because one shared lifetime is not merely the simple choice:
/// the opaque return's hidden lifetime has to *be* one of them, and separate
/// lifetimes would leave rustc no way to know which.
const INFERRED_LEND: &str = r#"
struct Stepped : Next<self, proj Str> canbe Mut {
    items: proj List<Str>,
    at: Int,
    step: Int
}

fn next(p: Mut Stepped) [] -> Emitted (proj(p) Str) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + p.step
    return emitted(elem)
}

fn iter(list: List<Str>, step: Int) [] -> Mut Stepped => list, step {
    return Mut Stepped { items: list, at: 0, step: copy(step) }
}

// No `holds proj(...)` written on either slot.
fn count<C, K, It, T>(c: C, k: K, ?iter: (c: C, k: K) -> Mut It, ?Next<It, T>) -> Int
=> c, k {
    let n = 0
    for _ in iter(c, k) {
        n = n + 1
    }
    return n
}

fn total<C, It, T>(c: C, ?iter: (c: C) -> Mut It, ?Next<It, T>) -> Int => c {
    let n = 0
    for _ in iter(c) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${count(List("a", "b", "c", "d", "e", "f"), 2)}")
    println("${total(List("x", "y", "z"))}")
    // The owning pass still fits the same slot: the lifetime it does not use
    // costs it nothing, which is why inferring the lend is free.
    println("${total(Set("p", "q"))}")
}
"#;

/// [proj-infer-fn-type] The signature it produces: **one** lifetime, shared by
/// the container and by the slot's matching parameter, so the pass the slot
/// mints can hold it.
#[test]
fn an_opaque_returning_slot_ties_the_parameters_lifetimes() {
    let files = generate(&[("main.sv", INFERRED_LEND)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    assert!(
        // Each lent parameter names its lifetime, and the slot takes both.
        text.contains("pub fn count<'k, 'c, C: Clone, K: Clone, It: Clone, T: Clone>(c: &'c C, k: &'k K, iter: &mut dyn FnMut(&'c C, &'k K) -> It"),
        "both parameters tie their lifetimes to the slot's:\n{text}"
    );
    assert!(
        text.contains("pub fn total<'c, C: Clone, It: Clone, T: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> It"),
        "the one-parameter case ties the same way:\n{text}"
    );
}

/// [proj-infer-fn-type] The inference is scoped to an **opaque** return, and a
/// lend **names a source** — so a slot with no parameter name (std's
/// `?copy: (T) -> T`) gets nothing, which is what stops a `&'c T` no signature
/// declares. `std.core.seq`'s combinators are the regression: they carry exactly
/// that slot.
#[test]
fn an_unnamed_or_concrete_slot_infers_no_lend() {
    let files = generate(&[("main.sv", INFERRED_LEND)]);
    let seq = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "std/core/seq.rs");
    if let Some(seq) = seq {
        assert!(
            !seq.content.contains("&'c T"),
            "an unnamed slot must not render a lifetime:\n{}",
            seq.content
        );
    }
}

/// [task-effects] A task body's effects are **inherited from the frame that
/// minted it** (user decision 2026-09-26). Two shapes in one program, because
/// they exercise different halves: a task minted from a frame that *binds* the
/// handler, and a task that itself mints a task on another pool — which is the
/// case that needs the inherited handle to be *owned*, since the second closure
/// has to hold one too.
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

/// [task-effects] [rs-task] The lowering: the inherited handle is an **owned**
/// handle bound outside the closure and cloned inside it, so nothing borrows
/// the minting frame — and a task's own effect parameters are owned handles for
/// the same reason, which is what lets `chain` mint `report`.
#[test]
fn an_inherited_effect_travels_as_an_owned_handle() {
    let files = generate(&[("main.sv", TASK_EFFECTS)]);
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .expect("main.rs");
    let text = &main.content;
    // A task body takes the handle like any fn.
    assert!(
        text.contains("pub fn report(console: &crate::std_core_console::Console"),
        "a task's effect parameter is a handle:\n{text}"
    );
    // The mint clones the handle outside the closure, which owns the clone.
    assert!(
        text.contains("let __e0 = console.clone();") && text.contains("std::boxed::Box::new(move |__v| crate::report(&__e0, "),
        "the mint hands over an owned clone:\n{text}"
    );
    // And nothing borrowed the frame for it.
    assert!(
        !text.contains("report(&mut console"),
        "a borrow of the minting frame cannot cross:\n{text}"
    );
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
        text.contains("std::boxed::Box::new(move |__v| crate::finish(__c0, __c1, "),
        "the continuation must be a moved one-shot closure:\n{text}"
    );
    assert!(
        !text.contains("__parked.insert"),
        "a task mint parks nothing: the closure is the continuation:\n{text}"
    );
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
            && main.content.contains("impl crate::__Stateless_Log for __Stub_Log"),
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
        text.contains("loop {\n        let mut __subject_1: Option<i32> = crate::std_core_list::remove_first_platform::<i32>(&mut *xs);\n        if !(__subject_1.is_some()) {"),
        "the while did not become a test-inside loop:\n{text}"
    );
    assert!(
        !text.contains("while ({ let __l"),
        "a subject is still emitted inside the loop condition:\n{text}"
    );
    // The binding reads the temporary, not a second call.
    assert!(
        text.contains("let mut n = __subject_1.unwrap();"),
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

/// Emits one example's Rust, exactly as `salvo compile` would.
fn emit_example(example: &str) -> Vec<salvo_backend_rust::EmittedFile> {
    // [std-cache] std is the process's front (`salvo_testkit::front`); the
    // dependencies and the example's tree come after it.
    let mut sources = SourceSet::default();
    // [manifest-deps] The example's manifest names its dependencies, loaded
    // between std and the example's own tree as `salvo compile` does.
    let project = salvo_core::Project::load(&examples_dir().join(example).join("salvo.toml"))
        .unwrap_or_else(|e| panic!("examples/{example}/salvo.toml: {e}"));
    for dep in project.dependencies().unwrap_or_else(|e| panic!("{e}")) {
        let errors = sources.add_dependency(&dep, "rs");
        assert!(errors.is_empty(), "failed to load dependency `{}`: {errors:?}", dep.name);
    }
    let src = examples_dir().join(example).join("salvo");
    let errors = sources.add_dir(&src, "rs", false, false);
    assert!(errors.is_empty(), "failed to read {example}: {errors:?}");
    // The drivers' pre-resolution expansions [actor-route-stub].
    let built = salvo_testkit::front::std_front("rs").program(sources.files, sources.companions);
    let errors: Vec<_> = built.diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse or expansion errors: {errors:?}");
    let program = built.program;
    emit(&program).unwrap_or_else(|errors| {
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
    for_each_example(|example| {
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
    });
}

// ===================== time [time-types] [time-timer] =====================

/// [time-timer] A real deadline: `DefaultTimer` spawned like any actor, a
/// `waitfor` bridging the fire into `main`, and the fire's `at` proving the
/// deadline was honoured. Asserted as *inequalities*, since a real clock cannot
/// promise an exact number.
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

// ===== the network sequence, step ①: `net` and `MemTransport` =====

/// Two virtual nodes over one `MemNetwork`: an `Inbound` actor per node, a
/// delivery, a partition that makes the same delivery `Unreachable`, a heal, a
/// frame to an endpoint nobody listens at, and a killed node. What this
/// exercises: a platform *effect's* shape realised by a pure-Salvo double
/// (`MemTransport` is a stateless handler over an actor), the host→scheduler
/// door (`Inbound` is an actor effect and `deliver` sends to it), and a
/// `waitfor` inside a plain handler's member (the route lookup) serving its
/// pool while it waits. Both backends print this text byte for byte.
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

/// [platform-threadsafe] A `threadsafe` platform handler nobody binds emits
/// nothing and constructs nothing: the effect's trait is the whole surface
/// std's shipped host companion implements.
#[test]
fn an_unused_threadsafe_platform_handler_emits_nothing() {
    let files = generate(&[("main.sv", NET_MEM_TRANSPORT)]);
    let all: String = files.iter().map(|f| f.content.as_str()).collect();
    assert!(
        !all.contains("__Shared_HostTcpTransport")
            && !all.contains("__Arc_HostTcpTransport")
            && !all.contains("__Platform_HostTcpTransport::new("),
        "an unused threadsafe platform handler emitted something:\n{all}"
    );
}

/// [platform-threadsafe] The real wire, on localhost: `HostTcpTransport` is
/// std's first `threadsafe` platform handler, so this is the `Arc`-shared,
/// `&self`-membered host compiled and run under rustc — and the host→scheduler
/// door for real: a frame read on a socket thread becomes an activation
/// through `__Stub_Inbound`. One smoke test per backend; everything else about
/// the network runs on `MemTransport`. The port is this backend's own so the
/// two suites can run at once.
const NET_TCP_SMOKE: &str = r#"import std.net
import std.time

actor effect Seen {
    send fn wait_one(out: Reply<Str>) => !out
}

// Records the one frame it expects and answers whoever asked for it —
// ordering by data dependency, since the frame arrives on a socket thread.
handler Echoing() of Inbound, Seen {
    mailbox { capacity: 16 }
    state {
        got: Str = ""
        waiting: Mut List<Reply<Str>> = Mut List()
    }

    send fn receive_frame(from: NodeEndpoint, frame: Bytes) => !from, !frame {
        got = "${to_str(from)}: ${str_of_bytes(frame) ?: "?"}"
        let text = copy(got)
        drain(waiting, r -> send(r, copy(text)))
        waiting = Mut List()
    }

    send fn wait_one(out: Reply<Str>) => !out {
        if got == "" {
            add(waiting, out)
            return
        }
        send(out, copy(got))
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
    let me = NodeEndpoint { host: "127.0.0.1", port: 47311 }
    let nobody = NodeEndpoint { host: "127.0.0.1", port: 47312 }
    use HostTcpTransport(copy(me))
    let workers = pool(2)
    let (echo, seen) = spawn Echoing() on workers
    report("listen", listen(copy(me), echo))
    report("void", deliver(copy(nobody), to_bytes("nobody home")))
    report("hello", deliver(copy(me), to_bytes("hello over tcp")))
    let frame = waitfor out: Reply<Str> { seen.wait_one(out) }
    println("me <- ${frame}")
    println("local ${to_str(local_endpoint())}")
}
"#;

#[test]
fn the_tcp_transport_on_localhost() {
    let files = generate(&[("main.sv", NET_TCP_SMOKE)]);
    let all: String = files.iter().map(|f| f.content.as_str()).collect();
    assert!(
        all.contains("= crate::std_net::__Platform_HostTcpTransport::new((me).clone());")
            && all.contains("= crate::std_net::Transport::shared(__use_"),
        "expected the host behind the effect's handle:\n{all}"
    );
}

// ===== the network sequence, step ②: the wire format =====

/// [net-wire-format] [rs-wire] Every scalar, `Str`, `Bytes`, an optional, a list of
/// structs, a three-armed union with a `None` arm — encoded and decoded, and
/// the bytes printed as hex. **The hex is the parity assertion**: the Kotlin
/// backend runs the same program and must print the same bytes, since a
/// Kotlin node and a Rust node share a group. A malformed input decodes to
/// `None`. The trailing `Ledger` protocol has a wire form, so its message
/// codec and `__PROTO_Ledger` are generated.
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
    let files = generate(&[("main.sv", WIRE_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs");
    let src = &main.content;
    for expected in [
        "impl crate::wire::__Wire for Pin {",
        "impl crate::wire::__Wire for __Msg_Ledger {",
        "pub const __PROTO_Ledger: &str = \"",
        "crate::wire::salvo_encode(&",
        "crate::wire::salvo_decode::<crate::Pin>(&",
    ] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    let unions = union_text(&files);
    assert!(
        unions.content.contains("impl<T1: crate::wire::__Wire, T2: crate::wire::__Wire> crate::wire::__Wire for Union2<T1, T2>"),
        "expected the union codec:\n{}",
        unions.content
    );
}


// ===== the network sequence, step ④: node groups =====

/// [node-group] [actor-protocol-hash] Two virtual nodes form a `StaticNodeGroup`
/// over `MemTransport`: the HELLO/ACK handshake, the protocol table (B knows
/// A's `Counter` hash), a subscriber hearing the join, a LEAVE heard as a
/// departure. Then three fresh nodes on a second network, each seeded with
/// the first only, converge on a full mesh through `GossipNodeGroup`'s
/// introductions. Byte-identical on both backends.
const NODE_GROUP_DEMO: &str = r#"import std.net
import std.time

handler Announcing(label: Str) [Console] of NodeGroupWatcher {
    mailbox { capacity: 32 }
    send fn joined(n: Node) => !n { println("${label}: + ${to_str(n.at)}") }
    send fn left(n: Node, why: Str) => !n, !why { println("${label}: - ${to_str(n.at)} (${why})") }
}

actor effect Boot {
    send fn boot(done: Reply<eff[NodeGroup]>) => !done
}

// Brings up node B from a pool of its own: transport, wire, listener, and its
// group over the same endpoint list.
handler Booting(at: NodeEndpoint, all: List<NodeEndpoint>, net: eff[MemNet], gossip: Bool) [Transport, spawn] of Boot {
    mailbox { capacity: 1 }
    send fn boot(done: Reply<eff[NodeGroup]>) => !done {
        let p = pool(1)
        if gossip {
            let group = spawn GossipNodeGroup("mesh", copy(all)) on p
            done.send(group)
            return
        }
        let group = spawn StaticNodeGroup("demo", copy(all)) on p
        done.send(group)
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let all = List(copy(a), copy(b))
    let net = spawn MemNetwork() on pool(1)

    // Node A: this one.
    use MemTransport(copy(a), copy(net))
    let pa = pool(1)
    let _connected = connect(copy(a), copy(pa))
    let group_a = spawn StaticNodeGroup("demo", copy(all)) on pa
    group_a.subscribe(spawn Announcing("a") on pa)

    // Node B: hosted beside it.
    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(b), copy(all), copy(net), false) with MemTransport(copy(b), copy(net)) on pb
    let group_b = waitfor done: Reply<eff[NodeGroup]> { booter.boot(done) }

    // Give the handshake a moment, then read both views.
    let timer = spawn DefaultTimer() on pool(1)
    let _t = waitfor f: Reply<Fired> { timer.after(millis(400), f) }
    let seen_a = waitfor out: Reply<List<Node>> { group_a.members(out) }
    let seen_b = waitfor out: Reply<List<Node>> { group_b.members(out) }
    println("a sees ${size(seen_a)}: ${names_of(seen_a)}")
    println("b sees ${size(seen_b)}: ${names_of(seen_b)}")
    let first_b = get(seen_b, 0)
    if !(first_b is None) {
        println("b knows a's Counter protocol: ${!(peer_protocol(first_b.id, "Counter") is None)}")
    }

    // B leaves; A hears it.
    group_b.leave()
    let _t2 = waitfor f: Reply<Fired> { timer.after(millis(300), f) }
    let after = waitfor out: Reply<List<Node>> { group_a.members(out) }
    println("a sees ${size(after)} after b left")

    // Gossip: three fresh nodes on a second network, each seeded with the
    // first only, converge on a full mesh through introductions.
    let net2 = spawn MemNetwork() on pool(1)
    let x = NodeEndpoint { host: "x", port: 2 }
    let y = NodeEndpoint { host: "y", port: 2 }
    let z = NodeEndpoint { host: "z", port: 2 }
    let gx = boot_node(copy(x), List(copy(x)), copy(net2))
    let gy = boot_node(copy(y), List(copy(x)), copy(net2))
    let gz = boot_node(copy(z), List(copy(x)), copy(net2))
    // Converging takes a few gossip rounds, and how long those take depends on
    // the machine's load: poll, bounded, rather than sleep a fixed time (a
    // fixed 500ms failed under a fully loaded fresh run, 2026-09-29).
    let _t3 = waitfor f: Reply<Fired> { timer.after(millis(200), f) }
    let sx = waitfor out: Reply<List<Node>> { gx.members(out) }
    let sy = waitfor out: Reply<List<Node>> { gy.members(out) }
    let sz = waitfor out: Reply<List<Node>> { gz.members(out) }
    let tries = 0
    while (size(sx) < 2 || size(sy) < 2 || size(sz) < 2) && tries < 100 {
        let _t4 = waitfor f: Reply<Fired> { timer.after(millis(100), f) }
        sx = waitfor out: Reply<List<Node>> { gx.members(out) }
        sy = waitfor out: Reply<List<Node>> { gy.members(out) }
        sz = waitfor out: Reply<List<Node>> { gz.members(out) }
        tries = tries + 1
    }
    println("mesh: x sees ${size(sx)}, y sees ${size(sy)}, z sees ${size(sz)}")
}

fn boot_node(at: NodeEndpoint, seeds: List<NodeEndpoint>, net: eff[MemNet]) [spawn] -> eff[NodeGroup] => !at, !seeds, !net {
    let p = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(at), seeds, copy(net), true) with MemTransport(at, net) on p
    let group = waitfor done: Reply<eff[NodeGroup]> { booter.boot(done) }
    // A group's `init` connects its node; `members` is answered after it, so
    // the next node's seeds are on the wire before it dials them.
    let _started = waitfor out: Reply<List<Node>> { group.members(out) }
    return group
}

actor effect Counter { send fn bump(n: Int) => !n }

fn names_of(nodes: List<Node>) -> Str => nodes {
    let out: Mut Str = Mut Str()
    for n in nodes {
        append(out, to_str(n.at))
    }
    return to_str(out)
}
"#;

#[test]
fn node_groups_over_mem_transport() {
    let files = generate(&[("main.sv", NODE_GROUP_DEMO)]);
    let all: String = files.iter().map(|f| f.content.as_str()).collect();
    assert!(
        all.contains("crate::scheduler::salvo_set_protocols(vec![("),
        "main must register the program's protocol table:\n{all}"
    );
}

// ===== the network sequence, step ⑤: actor groups =====

/// [effect-generic-decl] An effect generic over an *effect*, erased for the
/// output: `Reg<E>` with `eff[E]` payloads and `Registering<E>` are emitted as
/// monomorphic items, and `spawn Registering<Ping>()` needs no turbofish.
const EFFECT_GENERIC_DEMO: &str = r#"actor effect Ping { send fn ping(n: Int) => !n
    send fn count(out: Reply<Int>) => !out }
handler Pinging() [Console] of Ping {
    mailbox { capacity: 4 }
    state {
        seen: Int = 0
    }
    send fn ping(n: Int) => !n {
        seen = seen + 1
        println("ping ${n}")
    }
    send fn count(out: Reply<Int>) => !out { out.send(seen) }
}

actor effect Reg<E> {
    send fn join(member: eff[E]) => !member
    send fn members(out: Reply<List<eff[E]>>) => !out
}

handler Registering<E>() of Reg<E> {
    mailbox { capacity: 8 }
    state {
        all: Mut List<eff[E]> = Mut List()
    }
    send fn join(member: eff[E]) => !member { add(all, member) }
    send fn members(out: Reply<List<eff[E]>>) => !out { out.send(copy(all)) }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(1)
    let reg = spawn Registering<Ping>() on p
    let a = spawn Pinging() on p
    reg.join(copy(a))
    let ms = waitfor out: Reply<List<eff[Ping]>> { reg.members(out) }
    println("members: ${size(ms)}")
    for m in ms { m.ping(7) }
    let _seen = waitfor out: Reply<Int> { a.count(out) }
}
"#;

#[test]
fn effect_only_generics_erase_to_monomorphic_items() {
    let files = generate(&[("main.sv", EFFECT_GENERIC_DEMO)]);
    let main = files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs");
    let src = &main.content;
    for expected in ["pub enum __Msg_Reg {", "pub struct Registering {", "pub trait __Stateless_Reg: Send + Sync {"] {
        assert!(src.contains(expected), "expected `{expected}` in:\n{src}");
    }
    assert!(
        !src.contains("Registering::<") && !src.contains("Reg<"),
        "an erased declaration keeps no type arguments:\n{src}"
    );
}

/// [time-timer] [rs-time] A deadline is the runtime module's, in Salvo: the
/// default timer calls `runtime.after_nanos`, the runtime module carries the
/// wheel and its `OnceLock`-bound host, and the host scheduler has no timer
/// machinery left to call.
#[test]
fn a_deadline_lowers_to_the_runtime_modules_wheel() {
    let files = generate(&[("main.sv", TIME_TIMER)]);
    let find = |name: &str| files.iter().find(|f| f.rel_path.to_string_lossy() == name).map(|f| f.content.clone());
    let time = find("std/time.rs").expect("std/time.rs");
    assert!(time.contains("after_nanos("), "expected the deadline call in:\n{time}");
    let runtime = find("std/runtime/timers.rs").expect("std/runtime/timers.rs");
    assert!(runtime.contains("fn __module_use0() -> &'static") && runtime.contains("std::sync::OnceLock"), "{runtime}");
    let scheduler = find("scheduler.rs").expect("scheduler.rs");
    assert!(!scheduler.contains("salvo_after") && !scheduler.contains("timers"), "the host scheduler keeps no timers");
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

/// [rs-opt-borrow] Two `?:` picks on one optional **local** are two *reads*:
/// the checker allows them (reading an optional is free), so the lowering must
/// not move the value the first time. The form is the one a narrowed read
/// already takes — borrow the `Option`, unwrap the borrow, clone the payload
/// only where an owned value is wanted (found 2026-09-23 writing
/// `lib/std/test.test.sv` with the since-removed postfix `!`).
const REREAD_OPTIONAL: &str = r#"
fn len_of(s: Str) -> Int => s {
    return size(s)
}

fn shout(s: Str) -> Str => !s {
    return s
}

fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    println("once ${size(maybe ?: "")}")
    println("twice ${size(maybe ?: "")}")
    let kept: Str? = "hello"
    println("kept ${len_of(kept ?: unreachable!())}")
    let owned: Str? = "hello"
    println("owned ${shout(owned ?: unreachable!())}")
    let n: Int? = 3
    println("copy ${(n ?: 0) + (n ?: 0)}")
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
    // Two reads of the same local, both of them borrowing the `Option`.
    assert_eq!(main.matches("= &maybe;").count(), 2, "expected every read to borrow:\n{main}");
    assert!(!main.contains("maybe.unwrap()"), "a read moved the local:\n{main}");
    // A Copy payload needs none of it: the `Option` is Copy, so `.unwrap`
    // moves nothing.
    assert!(main.contains("let mut __picked_16 = __pick_15.unwrap();"), "expected the Copy payload to stay direct:\n{main}");
    // [rs-read-mode] A **kept** parameter is a read: the unwrap answers the
    // `&String` straight into the position — no clone, and no second `&`.
    assert!(
        main.contains("crate::len_of({") && main.contains("let mut __picked_10 = __pick_9.as_ref().unwrap();\n            __picked_10\n"),
        "a kept argument cloned:\n{main}"
    );
    // …and a *consuming* one still gets a value of its own.
    assert!(
        main.contains("let mut __picked_13 = __pick_12.as_ref().unwrap();\n            __picked_13.clone()\n"),
        "a consumed argument did not get an owned value:\n{main}"
    );
}

/// [rs-read-mode] An **intrinsic** argument takes its mode from the
/// intrinsic's own declaration, not from the position the call sits in:
/// `contains(str, needle) => str, needle` keeps both, so a `?:` pick on a local
/// hands the lowering the reference it already has. `add(list: Mut List<T>,
/// elem: T) => !elem` keeps the *list* and consumes the element — so the list
/// is reached mutably and the element arrives owned.
const INTRINSIC_OPTIONAL_ARGS: &str = r#"
fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    if contains(maybe ?: unreachable!(), "ell") { println("found") }
    let other: Str? = "hello"
    println("upper ${to_upper(other ?: unreachable!())}")
    let xs: Mut List<Int>? = Mut List(1)
    add(xs ?: unreachable!(), 3)
    println("xs ${size(xs)}")
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
        main.contains("let mut __picked_4 = __pick_3.as_ref().unwrap();\n            __picked_4\n        } else {\n            panic!(\"salvo: unreachable at main:5:26\")\n        }\n    }, &String::from(\"ell\"))"),
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
        main.contains("let mut __pick_9 = &mut xs;")
            && main.contains("let mut __picked_10 = __pick_9.as_mut().unwrap();\n            &mut *__picked_10\n"),
        "a `Mut` intrinsic argument did not reach the storage:\n{main}"
    );
    assert!(!main.contains(".clone()"), "{main}");
}

/// [rs-read-mode] A **narrowed** read is the same question as a `?:` pick, one path
/// over: `narrow_unwrap` renders through a borrow either way, and the
/// `.clone()` is the position's to ask for.
const NARROWED_READS: &str = r#"
fn len_of(s: Str) -> Int => s { return size(s) }
fn shout(s: Str) -> Str => !s { return s }

struct Box { label: Str? }

fn main() [use] {
    use StdConsole()
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
    // The narrowing is consumed by its last use, so the payload is moved out
    // once; kept positions borrow it — no clone anywhere.
    assert!(
        main.contains("let mut name_3 = name.unwrap();")
            && main.contains("crate::std_core_string::size_platform(&name_3)")
            && main.contains("crate::len_of(&name_3)"),
        "a narrowed kept argument cloned:\n{main}"
    );
    // A narrowed **field** read is a borrow of the field's payload.
    assert!(
        main.contains("let mut __narrowed_4 = b.label.as_ref().unwrap();")
            && main.contains("crate::len_of(__narrowed_4)"),
        "a narrowed field argument cloned:\n{main}"
    );
    // A binding never consumed borrows; the consuming parameter takes the value.
    assert!(main.contains("let mut held = &name_3;"), "{main}");
    assert!(main.contains("crate::shout(name_3)"), "a consumed argument took a reference:\n{main}");
    assert!(!main.contains(".clone()"), "{main}");
    // A Copy payload is copied out of the representation.
    assert!(
        main.contains("let mut n_5 = n.unwrap();") && main.contains("i32::wrapping_add(n_5, n_5)"),
        "a Copy narrowed read grew an `as_ref`:\n{main}"
    );
}

// ===== mutable element handles [ref-handle] [rs-elem-mut] =====

/// [ref-handle] Mutation through element handles of a `List<Mut T>`: a
/// statement-scoped one (`bump(at(xs, 0)!)` → an `Option<&mut T>` from `at`), a bound
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
    let h = at(xs, 1) ?: unreachable!()
    h.n = h.n + 10
    bump(h)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 }, Mut Counter { n: 2 })
    bump(at(xs, 0) ?: unreachable!())
    poke(xs)
    let a = get(xs, 0) ?: unreachable!()
    let b = get(xs, 1) ?: unreachable!()
    println("${a.n} ${b.n}")
}
"#;

/// [rs-elem-mut] [rs-path] The three renderings: the container parameter
/// arrives `&mut` (element-level `Mut` lends), the statement-scoped handle is
/// `at`'s path materialized as `&mut xs[pos]` in place, and the bound handle
/// is a captured index re-materialized at every use — never a bound `&mut`.
#[test]
fn elem_mut_handles_render_as_get_mut_and_captured_indices() {
    let files = generate(&[("main.sv", ELEM_MUT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn poke(xs: &mut Vec<crate::Counter>)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("let __h1: usize = crate::std_core_list::at_platform(&*xs, 1i32).expect("),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("xs[__h1].n = i32::wrapping_add(xs[__h1].n, 10i32);"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("crate::bump(&mut xs[__h1]);"),
        "{}",
        main.content
    );
    // A temporary handle is `at`'s position, read from a shared borrow and
    // only then materialized as the element borrowed `&mut` in place.
    assert!(
        main.content.contains("let __h1: Option<usize> = crate::std_core_list::at_platform(&xs, 0i32);")
            && main.content.contains("let __h2 = __h1.expect(\"salvo: value is absent\");\n            &mut xs[__h2]"),
        "{}",
        main.content
    );
}

/// [rs-loc] ④a slice 5 — a **search loop** whose element binding is
/// returned: in the minting fn's path rendering the `for` over a list becomes
/// an *indexed* loop and the binding a captured-index handle, so the found
/// **position** travels out [rs-path]. Rust can write this function (NLL accepts an
/// `iter_mut` search returning `Option<&mut T>`); what it cannot do is read
/// the container while the result is live, which is what the position
/// rendering buys — see `examples/borrowing/README.md`.
const SEARCH_LOOP_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn wounded(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    for e in es {
        if e.hp < 10 {
            return e
        }
    }
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 50 }, Mut Entity { hp: 3 })
    heal(wounded(es) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn a_search_loop_returns_the_found_position() {
    let files = generate(&[("main.sv", SEARCH_LOOP_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn wounded(es: &Vec<crate::Entity>) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("for __li1 in 0..es.len()"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("return Some(__li1);"),
        "{}",
        main.content
    );
    // The position fn is the only rendering, and the caller materializes it.
    assert!(
        !main.content.contains("Option<&mut crate::Entity>")
            && main.content.contains("let __h1: Option<usize> = crate::wounded(&es);"),
        "{}",
        main.content
    );
}

/// [rs-path] A user mint whose container is a **nested field** of structs
/// that have no accessor of their own: the path fn takes the outer struct
/// by shared borrow and answers the list position, and every use indexes the
/// same field path (`a.b.c[pos]`) — a statement-scoped handle and a bound
/// one alike.
const NESTED_FIELD_PATH_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

struct B canbe Mut {
    c: List<Mut Entity>
}

struct A canbe Mut {
    b: B
}

fn at(a: A, index: Int) -> (ref(a) Mut Entity)? {
    return a.b.c.at(index)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let a = Mut A { b: Mut B { c: List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 }) } }
    heal(at(a, 1) ?: unreachable!())
    let h = at(a, 0) ?: unreachable!()
    h.hp = h.hp + 100
    println("${(get(a.b.c, 0) ?: unreachable!()).hp} ${(get(a.b.c, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn a_mint_into_a_nested_field_paths_through_the_fields() {
    let files = generate(&[("main.sv", NESTED_FIELD_PATH_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn at(a: &crate::A, mut index: i32) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("&mut a.b.c[__h2]")
            && main.content.contains("let __h3: usize = crate::at(&a, 0i32).expect(")
            && main.content.contains("a.b.c[__h3].hp = i32::wrapping_add(a.b.c[__h3].hp, 100i32);"),
        "{}",
        main.content
    );
}

/// [rs-cmp-deref] A borrowed Copy scalar is **copied out in a comparison**:
/// Rust implements `&i32 + i32` but not `&i32 == i32` (E0277), so a lending
/// call's scalar result — the total `get`'s — derefs there and nowhere else.
/// Kotlin has no references and needs nothing.
const CMP_DEREF_DEMO: &str = r#"
fn main() [use] {
    use StdConsole()
    let xs = List(10, 20, 30)
    let i = 1
    if i is Idx(xs) {
        // A borrowed Copy scalar in a *comparison*: the operand is copied out.
        if get(xs, i) == 20 { println("eq") }
        if get(xs, i) < 30 { println("lt") }
        // Arithmetic and interpolation need no deref (Rust implements
        // `&i32 + i32`), and must not grow one.
        println("${get(xs, i) + 1}")
    }
}
"#;

#[test]
fn a_borrowed_copy_scalar_derefs_in_a_comparison_only() {
    let files = generate(&[("main.sv", CMP_DEREF_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("if ((*crate::std_core_list::get::<i32>(&xs, i)) == 20i32) {")
            && main.content.contains("if ((*crate::std_core_list::get::<i32>(&xs, i)) < 30i32) {"),
        "{}",
        main.content
    );
    // Arithmetic takes the scalar the same way.
    assert!(
        main.content.contains("i32::wrapping_add((*crate::std_core_list::get::<i32>(&xs, i)), 1i32)"),
        "{}",
        main.content
    );
}

/// [deduce-field] Rung ⑤ v1 — a **field-granular** mutation entry
/// (`=> e.hp: Mut`) narrows the event a call produces, so a derivation of
/// a *disjoint* field survives it by the ordinary overlap rule. The
/// surviving binding cannot be a live borrow (the callee still takes the
/// whole struct `&mut`), so it renders as a **virtual place**:
/// re-materialized per use, which agrees with Kotlin because the field was
/// untouched.
const FIELD_GRANULAR_DEMO: &str = r#"
struct Ring { power: Int }
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn damage(e: Mut Entity, n: Int) -> None => e.hp: Mut, n {
    e.hp = e.hp - n
}

fn main() [use] {
    use StdConsole()
    let e = Mut Entity { hp: 20, rings: Mut List(Ring { power: 3 }) }
    let rings = e.rings
    damage(e, 5)
    println("${size(rings)} ${e.hp}")
}
"#;

#[test]
fn a_surviving_field_derivation_renders_as_a_virtual_place() {
    let files = generate(&[("main.sv", FIELD_GRANULAR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    // No binding at all: the place is re-read where it is used.
    assert!(
        !main.content.contains("let mut rings"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("size_platform::<crate::Ring>(&e.rings)"),
        "{}",
        main.content
    );
}

/// [ref-anchor] [rs-loc] ④b — a callee whose two handle parameters are
/// `ref(c)` of one container parameter takes two element handles that **may
/// be the same element**: the handles render as the container once plus a
/// position each (two `&mut` into one container cannot coexist), and the
/// same-call rule stands down without any disjointness proof.
const REF_ANCHOR_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int, energy: Int }

fn attack(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) -> None {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(
        Mut Entity { hp: 10, energy: 5 },
        Mut Entity { hp: 20, energy: 8 })
    let i = 0
    let j = 1
    attack(es, at(es, i) ?: unreachable!(), at(es, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one element, both
    // handles.
    attack(es, at(es, i) ?: unreachable!(), at(es, i) ?: unreachable!())
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn ref_handles_render_as_their_container_and_positions() {
    let files = generate(&[("main.sv", REF_ANCHOR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content
            .contains("pub fn attack(c: &mut Vec<crate::Entity>, __c1: usize, __c2: usize)"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("c[__c1].energy = i32::wrapping_sub(c[__c1].energy, 1i32);"),
        "{}",
        main.content
    );
    // Each handle is minted as a position before the container is lent.
    assert!(
        main.content.contains("let __q1 = crate::std_core_list::at_platform(&es, i).expect(")
            && main.content.contains("let __c2 = __q1;")
            && main.content.contains("crate::attack(&mut es, __c2, __c4)"),
        "{}",
        main.content
    );
}

/// [cmp-carry] The limitation the capability makes visible: a **tuple** key,
/// refused with a Salvo diagnostic rather than a target-compiler one
/// [backend-never-wrong]. Accepted deliberately (user, 2026-09-26): the lift
/// is ROADMAP's "Recursive implicit resolution".
#[test]
fn a_tuple_key_resolves_the_structural_identity() {
    // [col-hashed-ordered] **Interim** (2026-09-28): a tuple or a list key
    // resolves the structural identity std declares for it,
    // so this container builds — the same on both backends for scalar
    // elements. What is still not consulted is an element struct's *own*
    // `cmp` inside a tuple; std owning these waits on ROADMAP §6.
    let files = generate(&[(
        "main.sv",
        "fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let ns: SortedSet<(Int, Int)> = SortedSet((1, 2), (1, 2), (0, 5))\n    \
         println(\"${size(ns)}\")\n}\n",
    )]);
    assert!(!files.is_empty());
}


// ===== the defect round of 2026-09-25 =====

/// [mod-used-only] Reachability follows **resolution** for functions, not
/// names (user decision 2026-09-25, closing a defect open since 2026-09-23).
/// A name resolves to every module declaring it, so a program that merely
/// iterates — using the name `next` — used to emit `std.core.range` as well,
/// because `std.core.range` declares a `next` too. The checker knows which `next`
/// the loop drives, so the edge follows that.
const REACH_ITERATES_ONLY: &str = r#"
fn main() [use] -> None {
    use StdConsole()
    let xs: List<Int> = List(1, 2, 3)
    for x in xs {
        println("${x}")
    }
    let total = reduce(iter(xs), 0, (a: Int, b: Int) -> { return a + b })
    println("${total}")
}
"#;

/// …and a program that really does range still gets it, which is the half a
/// narrowing could quietly break.
const REACH_RANGES: &str = r#"
fn main() [use] -> None {
    use StdConsole()
    for i in range(1, 4) {
        println("${i}")
    }
}
"#;

#[test]
fn reachability_follows_resolution_not_names() {
    let emitted = |src: &str| -> Vec<String> {
        generate(&[("main.sv", src)])
            .iter()
            .map(|f| f.rel_path.display().to_string())
            .collect()
    };
    let iterating = emitted(REACH_ITERATES_ONLY);
    assert!(
        !iterating.iter().any(|p| p.contains("range")),
        "iterating must not emit `core.range`: {iterating:?}"
    );
    // The same for the other names a program merely mentions: `fs`
    // declares `read`/`write`/`close`, `std.core.bytes` a `size`.
    assert!(
        !iterating.iter().any(|p| p.contains("fs") || p.contains("bytes")),
        "…nor the modules that merely share a name: {iterating:?}"
    );
    // What it does use is still there.
    for needed in ["std/core/list.rs", "std/core/console.rs", "std/core/seq.rs"] {
        assert!(
            iterating.iter().any(|p| p.ends_with(needed)),
            "{needed} is used and must be emitted: {iterating:?}"
        );
    }
    let ranging = emitted(REACH_RANGES);
    assert!(
        ranging.iter().any(|p| p.ends_with("std/core/range.rs")),
        "a program that ranges needs `core.range`: {ranging:?}"
    );
}

/// [interp-float] A float's text is **Salvo's rule, which is Kotlin's** (user
/// decision 2026-09-25): the shortest digits that round-trip, always a decimal
/// point, and scientific notation outside `[10^-3, 10^7)`. Rust's `Display`
/// writes the number in full and drops the `.0`, so the same value printed
/// differently on the two backends (`2` against `2.0`,
/// `100000000000000000000` against `1.0E20`) — a [backend-parity] break in
/// program *output*, which is why the expected text below is the assertion.
///
/// The table walks both ends of the plain window (`1234567.0` plain,
/// `1.2345678E7` scientific), the signs, zero, `NaN` and both infinities, a
/// struct rendered field-wise [interp-struct], a list, an explicit `to_str`, a
/// one-element list, and a map *value* — every path where a float becomes text
/// (a nested list waits on recursive implicit resolution, ROADMAP 6).
const FLOAT_TEXT_DEMO: &str = r#"
struct Point : ToStr<self> by auto {
    x: Double,
    ratio: Float
}

fn main() [use] -> None {
    use StdConsole()
    println("${2.0} ${8.25} ${0.0 - 2.0} ${0.0} ${0.001} ${0.0001}")
    println("${1234567.0} ${12345678.0} ${100000000000000000000.0} ${0.00000000000001}")
    let zero = 0.0
    let one = 1.0
    println("${zero / zero} ${one / zero} ${0.0 - one / zero}")
    println("${Point { x: 2.0, ratio: 3.0 }}")
    let xs: List<Double> = List(2.0, 0.5)
    println("${xs} ${to_str(xs)} ${List(4.0)}")
    let m: Map<Str, Double> = Map("a": 2.0)
    println("${m}")
}
"#;

#[test]
fn a_float_renders_by_salvos_rule() {
    let files = generate(&[("main.sv", FLOAT_TEXT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("crate::strings::salvo_f64_text"),
        "a float goes through the helper, not `Display`:\n{}",
        main.content
    );
    assert!(
        main.content.contains("crate::strings::salvo_f32_text"),
        "…and a `Float` through its own:\n{}",
        main.content
    );
}

/// [rs-loop-temp] Iterating a **temporary** container: the loop *binds* the
/// pass, so a Rust temporary the subject borrows dies too early (E0716) while
/// Kotlin's reference does not. The subject's temporaries are hoisted into
/// locals in front of the loop — rustc's own suggestion — including through a
/// view that holds the borrow (`filter(iter(…), …)`).
const LOOP_TEMP_DEMO: &str = r#"
fn main() [use] -> None {
    use StdConsole()
    for x in iter(List(1, 2)) {
        println("a ${x}")
    }
    for x in filter(iter(List(3, 4, 5)), (n: Int) -> { return n > 3 }) {
        println("b ${x}")
    }
    let xs: List<Int> = List(9)
    for x in xs {
        println("c ${x}")
    }
}
"#;

#[test]
fn a_loops_temporary_subject_is_hoisted() {
    let files = generate(&[("main.sv", LOOP_TEMP_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("let mut __tmp1 = vec![1i32, 2i32];"),
        "the temporary must be bound before the loop:\n{}",
        main.content
    );
    assert!(
        main.content.contains("let mut __pass_3: crate::std_core_list::ListNext<'_, i32> = crate::std_core_list::iter::<i32>(&__tmp1);"),
        "…and the pass must borrow the local:\n{}",
        main.content
    );
    // Exactly one hoist: the `filter(…)` loop iterates the returned list
    // *natively* [iter-for-native], where the temporary lives to the end of
    // the `for` statement and needs nothing, and the place subject (`xs`) is
    // already a local.
    assert_eq!(
        main.content.matches("let mut __tmp").count(),
        1,
        "only the pass-driving loop over a temporary hoists:\n{}",
        main.content
    );
}

/// [interp-to-str] [iter-resolve] A `to_str` the checker resolved in **another
/// module**, which the file never *names*: name-based reachability left the
/// module un-emitted and name-based imports left it un-imported, so the call
/// had nothing to bind to. Resolution edges now reach it
/// (`reach::resolved_dep_files`). [op-unique-shape] The `to_str` used to be in a
/// third module (`fmt`) the file imported whole and never named; it now has to
/// be in `Pair`'s own file, so the case is `Pair`'s attached `to_str`,
/// imported with `Pair` [fn-attached] and never named: the import half of the
/// bug (the reachability half cannot arise for an operator, whose module is
/// the type's).
const INTERP_TO_STR_MODULES: &[(&str, &str)] = &[
    (
        "pair.sv",
        r#"
export struct Pair {
    a: Int,
    b: Int

    fn to_str(p: Pair) -> Str => p {
        return "(${p.a}, ${p.b})"
    }
}
"#,
    ),
    (
        "main.sv",
        r#"
import pair.Pair

fn main() [use] -> None {
    use StdConsole()
    let p = Pair { a: 1, b: 2 }
    println("${p}")
}
"#,
    ),
];

#[test]
fn a_to_str_from_another_module() {
    let files = generate(INTERP_TO_STR_MODULES);
    assert!(
        files.iter().any(|f| f.rel_path.ends_with("pair.rs")),
        "the module the interpolation resolved into must be emitted"
    );
}

/// [implicit-infer] A **qualified argument** through an implicit fn position:
/// `total(List(1, 2, 3))`'s argument is `NonEmpty List<Int>`
/// [col-of-nonempty], and matching the candidate `iter` strictly against that
/// claim taught the call nothing — so `It` stayed unbound and the `?next`
/// beside it resolved by *rung*, picking a sibling pass's `next` and emitting
/// a call the target compiler rejected. The unqualified literal always worked,
/// which is what pointed at the qualifier.
const QUALIFIED_IMPLICIT_DEMO: &str = r#"
struct Sib : Next<self, Int> canbe Mut {
    at: Int
}

fn next(p: Mut Sib) -> Emitted Int | Finished => p: Mut {
    if p.at > 2 {
        return finished()
    }
    p.at = p.at + 1
    return emitted(p.at)
}

fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Next<It, Int>) -> Int => c {
    let t = 0
    for x in iter(c) {
        t = t + x
    }
    return t
}

fn main() [use] -> None {
    use StdConsole()
    println("${total(List(1, 2, 3))}")
    println("${total(List(1, 2, 3))}")
}
"#;

#[test]
fn a_qualified_argument_still_binds_an_implicits_type_variable() {
    let files = generate(&[("main.sv", QUALIFIED_IMPLICIT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    // Both calls fill `?next` from `std.core.list`, not from the sibling `Sib`.
    assert_eq!(
        main.content.matches("next__ListNext(").count(),
        2,
        "both calls must drive the list's `next`:\n{}",
        main.content
    );
}

/// [ref-anchor] [rs-loc] A handle container that is a **field path** at the
/// call: the callee's container parameter is the `&mut` the call lends
/// (`&mut squad.members`) and the handles add nothing but an index each — no
/// second borrow of the container is synthesized, which made every such call
/// E0499 before the defect closed 2026-09-25.
const REF_ANCHORED_FIELD_DEMO: &str = r#"
struct Fighter canbe Mut { name: Str, hp: Int, energy: Int }

struct Squad canbe Mut {
    banner: Str,
    members: List<Mut Fighter>
}

fn trade(members: List<Mut Fighter>, one: ref(members) Mut Fighter, other: ref(members) Mut Fighter) -> None {
    one.energy = one.energy - 1
    other.energy = other.energy + 1
    return None
}

fn main() [use] {
    use StdConsole()
    let squad = Mut Squad { banner: "red", members: List(
        Mut Fighter { name: "Ada", hp: 30, energy: 4 },
        Mut Fighter { name: "Bo", hp: 8, energy: 9 }) }
    let i = 0
    let j = 1
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, j) ?: unreachable!())
    // …and the aliasing case `ref(c)` exists for: one fighter, both roles.
    trade(squad.members, at(squad.members, i) ?: unreachable!(), at(squad.members, i) ?: unreachable!())
    println("${(get(squad.members, 0) ?: unreachable!()).energy} ${(get(squad.members, 1) ?: unreachable!()).energy}")
}
"#;

#[test]
fn a_ref_anchored_handle_indexes_its_container_parameter() {
    let files = generate(&[("main.sv", REF_ANCHORED_FIELD_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    // One borrow of the container, two positions — and no `__anchor`, since
    // the container is a parameter the callee already has.
    assert!(
        main.content
            .contains("pub fn trade(members: &mut Vec<crate::Fighter>, __c1: usize, __c2: usize)"),
        "{}",
        main.content
    );
    assert!(
        !main.content.contains("__anchor"),
        "a ref-anchored call must not synthesize a second borrow:\n{}",
        main.content
    );
    assert!(
        main.content.contains("members[__c1].energy = i32::wrapping_sub(members[__c1].energy, 1i32);"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("crate::trade(&mut squad.members, __c2, __c4) };")
            && main.content.contains("let __q1 = crate::std_core_list::at_platform(&squad.members, i).expect("),
        "{}",
        main.content
    );
}

/// [rs-mut-arg-hoist] A read that sits *before* a mutation in one expression:
/// Rust holds the read's borrow for the whole expression (E0502) where the
/// language only orders the two (`arguments are evaluated left to right`,
/// [deduce-same-call]), so the read is hoisted into a `let` in front of it.
/// Both shapes: a `format!` part (which borrows every argument) and a call
/// argument in a borrowed position. Closes the defect filed 2026-09-25.
const READ_BEFORE_MUT_DEMO: &str = r#"
struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int => b: Mut {
    b.n = b.n + 1
    return b.n
}

fn label(s: Str, n: Int) -> Str => s, n {
    return "${s}/${n}"
}

fn main() [use] {
    use StdConsole()
    let b = Mut Box { n: 1, tag: "t" }
    // An interpolated place read, then a call that mutates it.
    println("1. ${b.n} ${bumped(b)}")
    // The same across a call's arguments, where the read is a borrow.
    println("2. ${label(b.tag, bumped(b))}")
    // The other order needs nothing: the mutation is over before the read.
    println("3. ${bumped(b)} ${b.n}")
}
"#;

#[test]
fn a_read_before_a_mutation_is_hoisted_out_of_the_expression() {
    let files = generate(&[("main.sv", READ_BEFORE_MUT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    // The interpolation reads the local, not the place.
    assert!(
        main.content
            .contains("{ let __part3 = b.n; let __part4 = crate::bumped(&mut b); format!(\"1. {} {}\", __part3, __part4) }"),
        "{}",
        main.content
    );
    // The call argument does too — owned here, since a borrow is what
    // collided; `&__arg9` is what the position takes.
    assert!(
        main.content
            .contains("{ let __arg9 = b.tag.clone(); let __arg10 = crate::bumped(&mut b); crate::label(&__arg9, __arg10) }"),
        "{}",
        main.content
    );
    // …and the read *after* the mutation is read after it: it must see the
    // new value, on both backends.
    assert!(
        main.content.contains("{ let __part15 = crate::bumped(&mut b); let __part16 = b.n; format!(\"3. {} {}\", __part15, __part16) }"),
        "{}",
        main.content
    );
}

/// [rs-mut-arg-hoist] …and the shape the hoist cannot take: the read is
/// **mutable data**, so copying it out of the way would be a snapshot on Rust
/// and a live handle on Kotlin. Reported with the two remedies the program
/// can choose between, never silently either way [backend-never-wrong].
#[test]
fn a_mutable_read_before_a_mutation_is_refused() {
    let src = r#"
struct Holder canbe Mut { items: Mut List<Int>, n: Int }

fn grow(h: Mut Holder) -> Int => h: Mut {
    add(h.items, 9)
    return size(h.items)
}

fn pair(xs: List<Int>, n: Int) -> Str => xs, n {
    return "${to_str(xs)}/${n}"
}

fn main() [use] {
    use StdConsole()
    let h = Mut Holder { items: Mut List(1, 2), n: 0 }
    println("${pair(h.items, grow(h))}")
}
"#;
    let program = build_program(&[("main.sv", src)]);
    let errors = emit(&program)
        .err()
        .expect("a mutable read before a mutation must be a codegen error");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("copy(h.items)") && e.contains("its own statement")),
        "got {errors:?}"
    );
}

/// [fate-elem-distinct] [ref-notsame] [rs-elem-mut] The distinct-pair shapes: a
/// `NotSame`-proven pair of bound handles in one call, interleaved mutation
/// through both, and the pair in a call again — the checker spares the
/// sibling, the emitter splits the container once per pair call.
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
    use StdConsole()
    let es: List<Mut Entity> = List(
        Mut Entity { hp: 10, energy: 5 },
        Mut Entity { hp: 20, energy: 8 })
    let i = 0
    let j = 1
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        attack(a, d)
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        attack(a, d)
    }
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 0) ?: unreachable!()).energy} ${(get(es, 1) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).energy}")
}
"#;

/// [rs-elem-mut] A proven pair call renders as one `salvo_pair_mut`
/// preamble (a `split_at_mut`, no aliasing) whose two `&mut` halves are the
/// call's arguments, guarded by the two positions differing
/// [rs-runtime-source].
#[test]
fn a_distinct_pair_call_splits_the_container_once() {
    let files = generate(&[("main.sv", DISTINCT_PAIR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    let splices = main.content.matches("salvo_pair_mut(&mut es[..],").count();
    assert_eq!(splices, 2, "one preamble per pair call:\n{}", main.content);
    assert!(
        main.content.contains("crate::attack(__pm5, __pm6)")
            && main.content.contains("if __h1 != __h2 { let (__u3, __w4) = crate::seq::salvo_pair_mut(&mut es[..], __h1, __h2)")
            && main.content.contains("crate::std_core_ref::NotSame_qualifies(&es[__h2], &es[__h1])"),
        "{}",
        main.content
    );
}

/// [rs-loc] [rs-path] Locator-specialized lending (④a slice 1, re-founding
/// ③'s mechanism): a user-written accessor that mints a mutable handle
/// renders only as its path fn, under its own name, and the C family is
/// ordinary std Salvo riding the same mechanism.
const LEND_MUT_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal(front(es) ?: unreachable!())
    let hp = (front(es) ?: unreachable!()).hp
    println("${hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

/// [rs-path] The minting fn has one emission — the path fn answering
/// position data from a *read-mode* search, under the plain name, with no
/// `&mut`-returning face beside it — and every call site takes it: the
/// mutable use materializes `&mut es[pos]` in a block whose read borrow
/// ends before the write borrow begins, the read use indexes the container.
#[test]
fn a_mut_used_lender_renders_as_its_path_fn() {
    let files = generate(&[("main.sv", LEND_MUT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content
            .contains("pub fn front(es: &Vec<crate::Entity>) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        !main.content.contains("Option<&mut crate::Entity>") && !main.content.contains("__loc"),
        "a minting fn keeps no `&mut` face:\n{}",
        main.content
    );
    // The mutable use materializes the position where it stands.
    assert!(
        main.content.contains("crate::heal({\n        let __h1: Option<usize> = crate::front(&es);")
            && main.content.contains("&mut es[__h2]"),
        "{}",
        main.content
    );
    // A handle read later is a position: the read borrow ends at once.
    assert!(
        main.content.contains("let __h3: usize = crate::front(&es).expect(") && main.content.contains("es[__h3].hp"),
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

fn front(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    return at(es, 0)
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    let boss = front(es) ?: unreachable!()
    boss.hp = boss.hp + 100
    let n = size(es)
    boss.hp = boss.hp + n
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn a_bound_accessor_handle_is_a_captured_locator() {
    let files = generate(&[("main.sv", BOUND_ACCESSOR_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("let __h1: usize = crate::front(&es).expect("),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("es[__h1].hp = i32::wrapping_add(es[__h1].hp"),
        "{}",
        main.content
    );
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
           at: (c: List<Mut Entity>, k: Int) -> ref(c) Mut Entity?) -> None {
    heal(at(es, i) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    bump_at(es, 1, (c: List<Mut Entity>, k: Int) -> at(c, k))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
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
        main.content.contains("let __h1: Option<usize> = at(&*es, i);")
            && main.content.contains("&mut es[__h2]"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("&mut |mut c, mut k| -> Option<usize> {\n        crate::std_core_list::at_platform(c, k)\n"),
        "{}",
        main.content
    );
}

/// [col-locate] [rs-loc] ④a slice 3′ — std's `Ref` bundle: a generic
/// algorithm hands out mutable handles because the caller supplies `at`,
/// and the implicit renders as a **locator** closure (position data out,
/// read-mode parameters) with the handle materialized at the use site.
const LOCATE_BUNDLE_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn heal_at<L>(c: List<Mut Entity>, l: L, ?Ref<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    heal_at(es, 1)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn a_locate_implicit_renders_as_a_locator_closure() {
    let files = generate(&[("main.sv", LOCATE_BUNDLE_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("at: &mut dyn FnMut(&Vec<crate::Entity>, &L) -> Option<usize>"),
        "{}",
        main.content
    );
    assert!(
        main.content.contains("crate::std_core_list::at_platform(__a0, *__a1)"),
        "{}",
        main.content
    );
}

/// [rs-path] An **effect member** that mints a handle: the trait carries its
/// path face only, under the member's own name, every handler and adapter
/// implements it, and a use routes to it.
const LENDING_MEMBER_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

effect Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es
}

handler FirstLender of Lender {
    fn lease(es: List<Mut Entity>) -> (ref(es) Mut Entity)? => es {
        return at(es, 0)
    }
}

fn run(es: List<Mut Entity>) [Lender] -> None {
    heal(lease(es) ?: unreachable!())
    return None
}

fn main() [use] {
    use StdConsole()
    use FirstLender()
    let es: List<Mut Entity> = List(Mut Entity { hp: 5 }, Mut Entity { hp: 7 })
    run(es)
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp}")
}
"#;

/// [rs-path] A lending effect member renders as its path face only, under its
/// own name: the trait answers where the element is, and the caller walks it.
#[test]
fn a_lending_effect_member_lends_through_the_trait() {
    let files = generate(&[("main.sv", LENDING_MEMBER_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content
            .contains("fn lease(&mut self, es: &Vec<crate::Entity>) -> Option<usize>;")
            && !main.content.contains("lease_loc"),
        "{}",
        main.content
    );
}

/// [rs-elem-mut] A proven pair call in a *value* position: the split is a
/// block expression around the call, so the position needs no statement of
/// its own.
#[test]
fn a_distinct_pair_call_in_a_value_position_splits_in_place() {
    let src = r#"
struct Entity canbe Mut { hp: Int }

fn poke(a: Mut Entity, d: Mut Entity) -> Int => a: Mut, d: Mut {
    a.hp = a.hp + 1
    d.hp = d.hp + 1
    return a.hp
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 })
    let i = 0
    let j = 1
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        let x = poke(a, d)
        println("${x}")
    }
}
"#;
    let files = generate(&[("main.sv", src)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(
        main.content.contains("let mut x: i32 = { let (__pm5, __pm6) = if __h1 != __h2 { ")
            && main.content.contains("crate::seq::salvo_pair_mut(&mut es[..], __h1, __h2)")
            && main.content.contains("crate::poke(__pm5, __pm6) };"),
        "expected the split around the call in:\n{}",
        main.content
    );
}

/// The union files' text, read as one [rs-union-enums].
struct UnionText {
    content: String,
}

fn union_text(files: &[salvo_backend_rust::EmittedFile]) -> UnionText {
    UnionText {
        content: files
            .iter()
            .filter(|f| f.rel_path.starts_with("unions"))
            .map(|f| f.content.clone())
            .collect(),
    }
}

/// [actor-watch] [actor-on-idle] [runtime-handles] `watch` and `on_idle` are
/// Salvo: the program calls `std.core.actor`'s fns, which take the core's token
/// out of the reply token (`take_local()`) and register it; the scheduler
/// builds `Exit` and `Idle` itself, so no builder travels with the call.
#[test]
fn watch_and_on_idle_are_salvo_over_the_core_token() {
    for (src, call) in [(WATCH, "crate::std_core_actor::watch(c, out)"), (ON_IDLE, "crate::std_core_actor::on_idle(")] {
        let files = generate(&[("main.sv", src)]);
        let main = files.iter().find(|f| f.rel_path.to_string_lossy() == "main.rs").expect("main.rs");
        assert!(main.content.contains(call) && !main.content.contains("salvo_watch("), "{}", main.content);
        let actor = files
            .iter()
            .find(|f| f.rel_path.to_string_lossy() == "std/core/actor.rs")
            .expect("std/core/actor.rs");
        assert!(actor.content.contains(".take_local()"), "{}", actor.content);
    }
}

/// [op-unique-shape] The program above as it was before 2026-10-27, with a
/// module's catch-all `to_str<T>` beside std's: refused at the catch-all's
/// declaration, used or not — a `to_str` on a type parameter would be
/// interpolation's function for every type at once, and a type parameter has
/// no file to declare it in. The written override (`to_str = to_str@main`)
/// goes with it; the catch-all is given another name and passed explicitly.
const IMPLICIT_CATCH_ALL_DEMO: &str = r##"
fn to_str<T>(x: T) [] -> Str => x { return "?" }

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2)
    println("${to_str(xs, to_str = to_str@main)}")
}
"##;

#[test]
fn a_catch_all_to_str_is_refused_at_its_declaration() {
    let errors = expect_errors(IMPLICIT_CATCH_ALL_DEMO);
    assert!(
        errors.iter().any(|e| e.contains("`to_str(T)` has the shape of `to_str` on the type parameter `T`")
            && e.contains("give this one another name and pass it explicitly")
            && e.contains("where it is wanted (`to_str = by_name`)")),
        "{errors:?}"
    );
}

/// [fn-erased-variance] A difference the backends render stays invariant
/// (option (a), not chosen): a `count(xs: List<Int>)` does not fill a
/// `(Mut List<Int>) -> Int`, as an override or by name — `&Vec` is not
/// `&mut Vec` — and the messages are the ones it always had.
#[test]
fn a_written_fn_value_does_not_fit_across_mut() {
    let errors = expect_errors(
        r#"
fn count(xs: List<Int>) [] -> Int => xs { return size(xs) }
fn measure_all(xs: Mut List<Int>, ?measure: (xs: Mut List<Int>) -> Int) [] -> Int => xs: Mut { return measure(xs) }
fn apply_mut(f: (xs: Mut List<Int>) -> Int, xs: Mut List<Int>) [] -> Int => f, xs: Mut { return f(xs) }
fn main() [use] {
    use StdConsole()
    let ys = Mut List(1, 2)
    println("${measure_all(ys, measure = count)}")
    println("${apply_mut(count, ys)}")
}
"#,
    );
    assert!(
        errors.iter().any(|e| e.contains(
            "`measure` does not fit here: expects `(Mut List<Int>) -> Int`, found `(List<Int>) -> Int`"
        )),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("no matching overload for `apply_mut((List<Int>) -> Int, Mut List<Int>)`")),
        "{errors:?}"
    );
}

/// [rs-path] [ref-handle] A lending effect member is its path face only: a
/// handler delegating to a mint answers a path through a struct field, two
/// handles from it stay live across a write to the container's other field,
/// and one is passed on as a `Mut` argument.
const LENDING_MEMBER_PATH_DEMO: &str = r#"
struct Entity canbe Mut { hp: Int }
struct Team canbe Mut { name: Str, members: List<Mut Entity> }

fn pick(t: Team, i: Int) -> (ref(t) Mut Entity)? => t {
    return at(t.members, i)
}

effect Roster {
    fn member(t: Team, i: Int) -> (ref(t) Mut Entity)? => t
}

handler Plain of Roster {
    fn member(t: Team, i: Int) -> (ref(t) Mut Entity)? => t {
        return pick(t, i)
    }
}

fn heal(e: Mut Entity, by: Int) -> None => e: Mut {
    e.hp = e.hp + by
    return None
}

fn run(t: Mut Team) [Roster] -> None => t: Mut {
    let a = member(t, 0) ?: unreachable!()
    let b = member(t, 1) ?: unreachable!()
    t.name = "renamed"
    if b is NotSame(a) {
        a.hp = a.hp + b.hp
        heal(b, 100)
    }
    heal(member(t, 0) ?: unreachable!(), 1000)
    return None
}

fn main() [use] {
    use StdConsole()
    use Plain()
    let t = Mut Team { name: "t", members: List(Mut Entity { hp: 1 }, Mut Entity { hp: 2 }) }
    run(t)
    println("${t.name} ${(get(t.members, 0) ?: unreachable!()).hp} ${(get(t.members, 1) ?: unreachable!()).hp}")
}
"#;

#[test]
fn a_lending_member_through_a_struct_field() {
    let files = generate(&[("main.sv", LENDING_MEMBER_PATH_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(main.content.contains("fn member(&self, t: &crate::Team, i: i32) -> Option<usize>;"), "{}", main.content);
}

// [rs-path] Handles into a container that is a type parameter: the
// handle's path is the fn's own type parameter `P: Walk<C, E>`, walked at
// each use — `ref(c)` parameters, a pair proven apart and split by
// `walk_pair`, a handle minted through `?Ref`, and a delegating mint.
pub const GENERIC_CONTAINER_DEMO: &str = r#"
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

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

// Generic over the container: the handle's path is opaque here.
fn bump<C>(c: C, a: ref(c) Mut Player) -> None {
    a.goals = a.goals + 1
}

fn swap_goals<C>(c: C, a: ref(c) Mut Player, b: ref(c) Mut Player) -> None {
    let g = a.goals
    a.goals = b.goals
    b.goals = g
}

// A delegating mint: the accessor comes in through `?Ref`.
fn borrow_via<C, L>(c: C, l: L, ?Ref<C, L, Player>) [] -> ref(c) Mut Player? => c, l {
    return at(c, l)
}

fn bump_at<C, L>(c: C, l: L, ?Ref<C, L, Player>) -> None {
    let h = at(c, l) ?: unreachable!()
    h.goals = h.goals + 10
    h.goals = h.goals + 1
}

fn score(a: Mut Player, b: Mut Player) -> None => a: Mut, b: Mut {
    a.goals = a.goals + 1000
    b.goals = b.goals + 2000
}

// Two handles into a generic container, proven apart: one split, walked.
fn score_both<C>(c: C, a: ref(c) Mut Player, b: ref(c) Mut Player) -> None {
    if b is NotSame(a) {
        score(a, b)
    }
}

export fn main() [use] -> None {
    use StdConsole()
    let teams: Mut Map<Str, Mut Team> = Mut Map()
    put(teams, "red", Mut Team { coach: "Ada", roster: List(Mut Player { name: "Bo", goals: 0 }, Mut Player { name: "Cy", goals: 5 }) })
    let league = Mut League { teams: teams, bench: List(Mut Player { name: "Flo", goals: 9 }) }
    let squad: List<Mut Player> = List(Mut Player { name: "Ed", goals: 2 }, Mut Player { name: "Di", goals: 3 })

    bump(league, at(league, "red", 1) ?: unreachable!())
    bump(squad, at(squad, 0) ?: unreachable!())
    swap_goals(league, at(league, "red", 0) ?: unreachable!(), at(league, "", -1) ?: unreachable!())
    swap_goals(squad, at(squad, 0) ?: unreachable!(), at(squad, 1) ?: unreachable!())
    let h = borrow_via(squad, 1) ?: unreachable!()
    h.goals = h.goals + 100
    bump_at(squad, 0)
    score_both(league, at(league, "red", 0) ?: unreachable!(), at(league, "", -1) ?: unreachable!())
    score_both(squad, at(squad, 0) ?: unreachable!(), at(squad, 0) ?: unreachable!())
    let red = get(league.teams, "red") ?: unreachable!()
    println("${(get(red.roster, 0) ?: unreachable!()).goals} ${(get(red.roster, 1) ?: unreachable!()).goals} ${(get(league.bench, 0) ?: unreachable!()).goals}")
    println("${(get(squad, 0) ?: unreachable!()).goals} ${(get(squad, 1) ?: unreachable!()).goals}")
}
"#;

pub const GENERIC_CONTAINER_OUTPUT: &str = "1009 6 2000\n14 103\n";

#[test]
fn handles_into_generic_containers() {
    let files = generate(&[("main.sv", GENERIC_CONTAINER_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    assert!(
        main.content.contains("pub fn bump<C: Clone, __P0: crate::seq::Walk<C, crate::Player> + Copy + PartialEq>(c: &mut C, __c1: __P0)")
            && main.content.contains("(*__c1.walk(&mut *c)).goals")
            && main.content.contains("impl crate::seq::Walk<crate::League, crate::Player> for crate::__Path_League__Player")
            && main.content.contains(".walk_pair(&"),
        "{}",
        main.content
    );
}

// [col-idx] [ref-handle] The total `at`: at an `Idx` the mint answers the
// handle itself, so a bound handle is a bare position, `update` materializes
// it with no `Option` match, and two bound total handles proven apart by
// `NotSame` [ref-notsame] split through their two paths.
const TOTAL_AT_DEMO: &str = r#"
struct E canbe Mut { hp: Int }

fn bump(e: Mut E) -> None => e: Mut {
    e.hp = e.hp + 1
}

fn hit(c: List<Mut E>, a: ref(c) Mut E, b: ref(c) Mut E) -> None {
    a.hp = a.hp - 1
    b.hp = b.hp - 2
}

fn merge(p: Mut E, q: Mut E) -> None => p: Mut, q: Mut {
    p.hp = p.hp + q.hp
}

export fn main() [use] -> None {
    use StdConsole()
    let xs: List<Mut E> = List(Mut E { hp: 10 }, Mut E { hp: 20 })
    let i = 0
    let j = 1
    if i is Idx(xs) {
        if j is Idx(xs) {
            let a = at(xs, i)
            a.hp = a.hp + 5
            bump(at(xs, j))
            hit(xs, at(xs, i), at(xs, i))
            update(xs, i, (e: Mut E) -> { e.hp = e.hp * 2 })
            let p = at(xs, i)
            let q = at(xs, j)
            if q is NotSame(p) {
                merge(p, q)
            }
        }
    }
    println("${(get(xs, i) ?: unreachable!()).hp} ${(get(xs, j) ?: unreachable!()).hp}")
}
"#;

#[test]
fn the_total_at() {
    let files = generate(&[("main.sv", TOTAL_AT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    assert!(
        main.content.contains("let __h1: usize = crate::std_core_list::at(&xs, i);")
            && main.content.contains("crate::bump({ let __l2 = crate::std_core_list::at(&xs, j); &mut xs[__l2] });")
            && main.content.contains("let __h8: usize = crate::std_core_list::at(&xs, j);")
            && main.content.contains("crate::seq::salvo_pair_mut(&mut xs[..], __h7, __h8)")
            && main.content.contains("crate::merge(__pm11, __pm12)"),
        "{}",
        main.content
    );
    let list = files.iter().find(|f| f.rel_path.ends_with("std/core/list.rs")).unwrap();
    assert!(
        list.content.contains("f({ let __l1 = crate::std_core_list::at(&*list, index); &mut list[__l1] });"),
        "{}",
        list.content
    );
}

/// [ref-notsame] [fate-elem-distinct] [rs-path] Two handles proven apart by
/// `NotSame`, through a custom container's own accessor: the split lands
/// where the handles' paths diverge — two map slots (`pair_mut` on the map),
/// two positions of one slot's roster, a roster and the bench (different
/// fields) — and through nested structs with no accessor of their own
/// (`a.b.c`). Two handles to one element make the test false at run time
/// (`std::ptr::eq`). Source and expected stdout are verbatim the Kotlin
/// backend's `kotlinc_compiles_and_runs_notsame_handles`.
pub const NOTSAME_DEMO: &str = r#"
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

pub const NOTSAME_OUTPUT: &str = "bench 1\n\
     Cy 7\n\
     same element\n\
     18 110\n\
     1 3\n\
     7 10\n\
     same cell\n\
     102 212\n";

#[test]
fn notsame_handles() {
    let files = generate(&[("main.sv", NOTSAME_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    assert!(
        main.content.contains("l.teams.pair_mut(")
            && main.content.contains("crate::seq::salvo_pair_mut(&mut a.b.c[..], __h1, __h2)")
            && main.content.contains("panic!(\"salvo: two handles to one element\")"),
        "{}",
        main.content
    );
    let reff = files.iter().find(|f| f.rel_path.ends_with("std/core/ref.rs")).unwrap();
    assert!(reff.content.contains("std::ptr::eq(a, b)"), "{}", reff.content);
}

// [ref-handle] [rs-path] A handle read where a value is expected: unwrapped
// into a kept owned slot, passed whole (optional) to a borrowed optional
// slot (`ref(c) T <: proj T`, the optional walked if present), and copied
// (`copy` binds `T` to the element, not the handle) — the copy is a value
// of its own, so a later write through the handle leaves it alone.
pub const HANDLE_READ_DEMO: &str = r#"
struct Word canbe Mut { text: Str }
struct B canbe Mut { c: List<Mut Word> }
struct A canbe Mut { b: B, d: List<Mut Word> }

fn at(a: A, index: Int) [] -> ref(a) Mut Word? => a, index {
    if index < 0 {
        return a.d.at(-index)
    }
    return a.b.c.at(index)
}

fn print_one(w: Word) [Console] => w {
    println(w.text)
}

fn maybe_print(w: (proj Word)?) [Console] => w {
    if w is Word {
        println(w.text)
    }
}

fn keep(w: Word) [] -> Word => !w {
    return w
}

export fn main() [use] -> None {
    use StdConsole()
    let a = A { b: B { c: List(Mut Word { text: "something" }) }, d: List(Mut Word { text: "one" }, Mut Word { text: "two" }) }
    let x = at(a, -1)
    let y = at(a, 0)
    let z = at(a, 5)
    print_one(y ?: unreachable!())
    maybe_print(x)
    maybe_print(z)
    let owned = keep(copy(y))
    let w = y
    w.text = "changed"
    println("${owned.text} ${(get(a.b.c, 0) ?: unreachable!()).text}")
}
"#;

pub const HANDLE_READ_OUTPUT: &str = "something\ntwo\nsomething changed\n";

#[test]
fn handle_reads() {
    let files = generate(&[("main.sv", HANDLE_READ_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    assert!(main.content.contains(".map(|__x"), "the optional handle is walked when present:\n{}", main.content);
}

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

#[test]
fn borrowed_optionals_in_owned_optional_slots() {
    let files = generate(&[("main.sv", OPT_SLOT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    assert!(main.content.contains("pub fn inner(mut s: Option<&String>) -> String"), "a kept optional is the optional of a borrow:\n{}", main.content);
    assert!(main.content.contains("fn show(&self, s: Option<&String>);"), "an effect member's too:\n{}", main.content);
    assert!(main.content.contains("(&o).as_ref()"), "an owned optional lends with `.as_ref()`:\n{}", main.content);
}

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

#[test]
fn one_rendering_for_a_ref_result() {
    let files = generate(&[("main.sv", REF_ONE_RENDERING_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.to_str() == Some("main.rs")).unwrap();
    // The path function only: no borrow-answering face beside it.
    assert!(!main.content.contains("__loc") && main.content.contains("-> Option<usize>"), "{}", main.content);
}

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

#[test]
fn unions_of_borrows() {
    let files = generate(&[("main.sv", UNION_BORROW_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(main.content.contains("pub fn describe(mut x: crate::unions::Union2<&crate::A, &crate::B>) -> String"), "{}", main.content);
    assert!(main.content.contains("x: crate::unions::Union2<&mut crate::A, &mut crate::B>"), "{}", main.content);
}

#[test]
fn mut_optional_slots() {
    let files = generate(&[("main.sv", MUT_OPT_SLOT_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(main.content.contains("pub fn heal(mut e: Option<&mut crate::Entity>) -> i32"), "{}", main.content);
}

/// [type-none-unit] Returning a `None`-typed expression (a call to a
/// `None`-returning fn, used for its effect) where the function's own return
/// type is an *optional* evaluates the call and then returns the absent
/// value — `return log_it()` in a `Str?` fn is `{ log_it(); None }`, not a
/// bare `return log_it()` (which would be `()` where `Option<String>` is
/// wanted). The elvis `read() ??: return ignore(_)` produces exactly this
/// shape.
pub const NONE_RETURN_DEMO: &str = r#"
fn log_it() -> None {
    return None
}

fn pick(which: Int) -> Str? {
    if which == 0 { return log_it() }
    return "value"
}

fn show(r: Str?, tag: Str) [Console] -> None {
    when r {
        is Str { println("${tag}: ${r}") }
        is None { println("${tag}: none") }
    }
}

fn main() [use] {
    use StdConsole()
    show(pick(0), "zero")
    show(pick(1), "one")
}
"#;

pub const NONE_RETURN_OUTPUT: &str = "zero: none\none: value\n";

/// [loop-never] A non-escaping `while true` is `Never`: a `-> T` fn whose
/// tail is one needs no return, and it lowers to a bare `loop {}` (no exit
/// test) that rustc types as `!`. The loop is guarded so the program ends.
pub const LOOP_NEVER_DEMO: &str = r#"
fn pick(n: Int) -> Int {
    if n > 0 { return n }
    while true {
    }
}

fn main() [use] {
    use StdConsole()
    println("${pick(7)}")
}
"#;

pub const LOOP_NEVER_OUTPUT: &str = "7\n";

#[test]
fn an_infinite_loop_as_never() {
    let files = generate(&[("main.sv", LOOP_NEVER_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    assert!(main.content.contains("loop {"), "a bare `loop {{` is emitted: {}", main.content);
}


/// `detach<T, C>(...T | Box2<C>)` meets `Str | Int | Box2<Bool>` with
/// `T = Str | Int` (the `...T` spread captures both), `C = Bool`, and answers
/// the flat `Str | Int | Bool`. The callee is one generic body over the
/// nested `Union2<T, Box2<C>>`; the call site rewraps the flat argument to
/// nested and the nested result back to flat. [type-union-spread]
pub const UNION_SPAN_DEMO: &str = r#"
struct Box2<C> { value: C }

fn detach<T, C>(value: ...T | Box2<C>) -> T | C => !value {
    when value {
        is Box2<C> { return value.value }
        is T { return value }
    }
}

fn show(v: Str | Int | Bool) [Console] -> None {
    when v {
        is Str { println("str ${v}") }
        is Int { println("int ${v}") }
        is Bool { println("bool ${v}") }
    }
}

fn main() [use] {
    use StdConsole()
    let a: Str | Int | Box2<Bool> = "hi"
    let b: Str | Int | Box2<Bool> = 7
    let c: Str | Int | Box2<Bool> = Box2<Bool> { value: true }
    let d: Str | Int | Box2<Bool> = "tf"
    show(detach(a))
    show(detach(b))
    show(detach(c))
    show(detach<Str | Int, Bool>(d))
}
"#;

pub const UNION_SPAN_OUTPUT: &str = "str hi\nint 7\nbool true\nstr tf\n";

#[test]
fn union_spanning_generic() {
    let files = generate(&[("main.sv", UNION_SPAN_DEMO)]);
    let main = files.iter().find(|f| f.rel_path.ends_with("main.rs")).unwrap();
    // The callee is a Rust generic over the nested slot (`T` one arm).
    assert!(
        main.content.contains("pub fn detach<T") && main.content.contains("crate::unions::Union2<T, crate::Box2<C>>"),
        "{}",
        main.content
    );
}

/// [type-union-spread] Two bare generic arms `T | S` — each captures **exactly
/// one** arm (no spread needed), the capability the spread change enables:
/// before, two unbound generic arms could not be partitioned. `swap` reorders
/// the two arms of a two-arm union.
pub const UNION_TWO_GENERIC_DEMO: &str = r#"
fn swap<T, S>(v: T | S) -> S | T => !v {
    when v {
        is T { return v }
        is S { return v }
    }
}

fn show(v: Int | Str) [Console] -> None {
    when v {
        is Int { println("int ${v}") }
        is Str { println("str ${v}") }
    }
}

fn main() [use] {
    use StdConsole()
    let a: Str | Int = "hi"
    let b: Int | Str = 7
    show(swap(a))
    show(swap(b))
}
"#;

pub const UNION_TWO_GENERIC_OUTPUT: &str = "str hi\nint 7\n";

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

pub const PARAM_DEFAULT_OUTPUT: &str =
    "Hi, Ada.\nHi, Ada!\nHello, Ada!\nHello, Ada?\n10\n15\nsize is 3\n3,4\n";

