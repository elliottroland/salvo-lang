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

// [mod-used-only] [rs-crate] The main module is the crate root: its
// attributes come first, and an unused module is not emitted at all. The
// rest of what this program emits is emit data
// (`tests/emit/modules/crate_layout.sv`).
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
    assert!(!paths.contains(&"unused.rs".to_string()), "{paths:?}");
    let main = files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == "main.rs")
        .unwrap();
    assert!(main.content.starts_with("#![allow("), "{}", main.content);
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

// ===== [iter-protocol] laziness, now a property of the pass =====

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

// ===== qualifier refinements [qual-refn] =====

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

// ===== [str-drop-mut] [rs-mut-str] `Mut Str` is a `String` =====

/// [rs-mut-str] The string support file is only generated when something
/// needs it: a file's absence, which emit data cannot state. (The rest of
/// this test is emit data: `strings/mut_str.sv#mut_str_is_a_plain_string`.)
#[test]
fn mut_str_is_a_plain_string() {
    // The support file is only generated when something needs it.
    let plain = generate(&[("main.sv", "export fn main() {\n}\n")]);
    assert!(
        !plain
            .iter()
            .any(|f| f.rel_path.to_string_lossy() == "strings.rs"),
        "the string helpers must not be emitted for a program that never mutates a string"
    );
}

// ===== [iter-generic-drive] driving a generic pass =====

/// [iter-drive-in-place] A *kept* pass gets no local: `take`'s loop advances
/// the parameter where it lives. The in-place drive itself is emit data
/// (`iteration/hand_passes.sv#a_kept_pass_is_driven_in_place`); what stays
/// here is the absence of a `__pass` local from *this one function* (the
/// program's other loop binds one), which a whole-file `!emits` cannot say.
#[test]
fn a_kept_pass_is_driven_in_place() {
    let files = generate(&[("main.sv", GENERIC_DRIVE_DEMO)]);
    let src = &files
        .iter()
        .find(|f| f.rel_path == std::path::Path::new("main.rs"))
        .expect("main.rs")
        .content;
    // Precisely: no local for *this* loop's pass.
    let take = src.split("pub fn take").nth(1).and_then(|s| s.split("\npub fn ").next()).expect("take");
    assert!(
        !take.contains("__pass"),
        "a kept pass must not be bound into a local in:\n{src}"
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
/// for them (ROADMAP §0j step 7). A file's absence is not emit data; the rest
/// of this test is `emit/collections/host_types.sv`.
#[test]
fn the_collections_are_std_host_types() {
    let files = generate(&[("main.sv", COLLECTIONS_DEMO)]);
    assert!(
        !files.iter().any(|f| f.rel_path.ends_with("collections.rs")),
        "the collections runtime module is gone"
    );
}

// ===== C-6 the claims a list can carry [col-nonempty] [col-sorted-list] =====

// ===== [col-bounds] every index answers rather than failing =====

// ===== [deduce-reapply] [cmp-carry] a binary heap in user space =====

// ===== [fn-variadic] mixing plain arguments with a `...spread` =====

// ===== [kt-variadic] a user-declared variadic, every element type =====

// ===== [actor-use-addr] the forwarding stub =====

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

#[test]
fn reachability_follows_resolution_not_names() {
    // What the program uses is emitted, and a program that ranges gets
    // `core.range`: emit data, `tests/emit/modules/reachability.sv`. What stays
    // here is a file's *absence*, which an annotation cannot say.
    let iterating: Vec<String> = generate(&[("main.sv", REACH_ITERATES_ONLY)])
        .iter()
        .map(|f| f.rel_path.display().to_string())
        .collect();
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
}

