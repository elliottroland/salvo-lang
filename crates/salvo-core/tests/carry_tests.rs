//! [cmp-carry] Structures that **hold** an ordering, and the rest of what
//! this file used to hold (type-named fns, calls through fn values, keys),
//! are test data, `tests/check/carry/` (TESTS.md T1). What is left here are
//! cases whose diagnostics over the real std are not yet the ones they should
//! be (ROADMAP §0ar), kept as they were until that is fixed:
//!
//! - over the heap declarations and their prelude: a local named in a type's
//!   slot (also warned unused, and the annotation mismatch), an unknown
//!   identity and an unknown `cmp@Nothing` selector (each reported twice), and
//!   a partial type-argument list (reported twice);
//! - over the prelude: a slot in a fn's `<…>` (two errors), and a bare value
//!   two provenance arms accept (`type-union:no-arm`, "no arm accepts", beside
//!   the return mismatch);
//! - over the real std: `to_set(List())` (`cannot infer` twice, for `List`'s
//!   `T` and `to_set`'s).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The capability groups and the intrinsic canonicals, as
/// `lib/std/core/compare.sv` declares them, plus the list surface a heap is built
/// on.
const STD_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export fns Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export fns Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn Mut List<T>(...elems: T[]) [] -> Mut List<T>\n",
    "export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n",
    "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
),
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b\n",
);

/// The user code every test shares: a struct with a canonical ordering
/// [fn-attached], a second ordering of its own, and the heap qualifier with
/// its fn slot.
const HEAP: &str = r#"
// [fn-attached] The obligation declares the capability, and this file's `cmp`
// fulfils it — so `cmp` is declared on `Person` and travels with it.
export struct Person : Ordered<self> { name: Str, age: Int }

export fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

export fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

export qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>

// [cmp-carry] The same thing through a **group spread**: one slot per member of
// the group, which is what `?Ordered<T>` means in a slot list.
export qualifier Ranked<T>(?Ordered<T>) of List<T>

export fn empty_ranked<T>(?cmp: (T, T) -> Int) [] -> +Ranked<T>(?cmp) Mut List<T> {
    return Mut List()
}

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> +Heap<T>(?cmp) Mut List<T> {
    return Mut List()
}

export fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) [] -> +Heap<T>(?cmp) Mut List<T> => !heap, !elem {
    add(heap, elem)
    return heap
}

export fn heap_size<T>(heap: Heap Mut List<T>) [] -> Int => heap {
    return size(heap)
}

// [deduce-reapply] Establishing the claim on a parameter that arrives without
// it, which only this file may do.
export fn heapify<T>(list: Mut List<T>, elem: T, ?cmp: (T, T) -> Int) [] -> None
=> list: +Heap<T>(?cmp) Mut, !elem {
    add(list, elem)
}
"#;

fn errors(src: &str) -> Vec<&'static str> {
    errors_in(&[("main.sv", src)])
}

/// The shared declarations plus one file of test code.
fn errors_with_heap(src: &str) -> Vec<&'static str> {
    errors_in(&[("heap.sv", HEAP), ("main.sv", src)])
}

/// [diag-id] The id of every parse and checker error, and of every IR
/// analysis diagnostic.
fn errors_in(files: &[(&str, &str)]) -> Vec<&'static str> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/compare.sv",
        SourceSet::classify(Path::new("std/core/compare.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            src.to_string(),
            false,
        );
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut parse_errors: Vec<&'static str> = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        parse_errors.extend(
            diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.id.unwrap_or("?")),
        );
        modules.push(ast);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut ids: Vec<&'static str> = parse_errors
        .into_iter()
        .chain(
            resolution
                .errors
                .iter()
                .chain(checked.errors.iter())
                .filter(|d| d.is_error())
                .map(|d| d.id.unwrap_or("?")),
        )
        .collect();
    ids.extend(
        salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked)
            .into_iter()
            .map(|d| d.id.unwrap_or("?")),
    );
    ids
}

/// [cmp-carry] A local of fn type, written in the type itself, has no
/// identity a type can carry.
#[test]
fn a_local_is_not_an_identity() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name

fn run() [] -> Int {
    let mine = by_name
    let h: Heap<Person>(mine) Mut List<Person> = Mut List()
    return 0
}
"#,
    );
    assert!(
        errs.contains(&"cmp-carry:local-identity"),
        "expected the static-identity refusal: {errs:?}"
    );
}

/// A slot filled with a name nothing declares is the ordinary unresolved-name
/// error, naming what was looked for.
#[test]
fn an_unknown_identity_is_reported() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn run(h: Heap<Person>(nowhere) Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.contains(&"cmp-carry:unknown-fn"),
        "expected the unknown-function error: {errs:?}"
    );
}

/// `cmp@Person` names the canonical [fn-attached]; `cmp@Nothing` names
/// nowhere, and the error says where one would live.
#[test]
fn a_selector_must_name_a_declaration() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn run(h: Heap<Person>(cmp@Nothing) Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.contains(&"fn-attached:no-identity-on-type"),
        "expected the selector error: {errs:?}"
    );
}

/// [qual-value-arg] Type generics are all-or-none at a use site: a partial
/// list is an error naming the rule and the block.
#[test]
fn type_arguments_are_all_or_none() {
    let errs = errors_with_heap(
        r#"
import heap
struct Person { age: Int }
fn by_age(a: Person, b: Person) [] -> Int => a, b {
    return a.age - b.age
}
fn f(h: Ranked<Person, Int>(by_age) Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.contains(&"qual-value-arg:partial-type-args"),
        "expected the all-or-none error: {errs:?}"
    );
}

/// Only a qualifier or an `intrinsic type` may declare one: on a fn the
/// binder binds bare in the signature instead (the ordering round's decision 11).
#[test]
fn a_fn_declares_no_slot() {
    let errs = errors(
        r#"
fn bad<T, ?cmp: (T, T) -> Int>(a: T) [] -> Int => a {
    return 0
}
"#,
    );
    assert!(
        errs.contains(&"cmp-carry:fn-slot-misplaced"),
        "expected the slot-position error: {errs:?}"
    );
}

#[test]
fn provenance_arm_is_inferred_from_a_bare_value() {
    // [qual-arm-infer] A bare value that is the payload of exactly one
    // provenance-tagged arm is wrapped into it: `return n` for an
    // `Ok Int | Err Str` return is implicitly `Ok(n)`.
    let prelude = "provenance qualifier Ok<T> of T\n\
         provenance qualifier Err<T> of T\n\
         fn Ok<T canbe linear>(v: T) [] -> +Ok T => !v {\n    return v\n}\n\
         fn Err<T canbe linear>(v: T) [] -> +Err T => !v {\n    return v\n}\n";

    let ok = errors(&format!(
        "{prelude}fn classify(n: Int) -> Ok Int | Err Str {{\n    \
         if n < 0 {{\n        return \"neg\"\n    }}\n    return n\n}}\n"
    ));
    assert!(ok.is_empty(), "bare values infer their provenance arm: {ok:?}");

    // Ambiguous: two provenance arms accept an `Int`, so the bare value is
    // refused and the explicit constructor is required.
    let ambig = errors(&format!(
        "{prelude}fn two(n: Int) -> Ok Int | Err Int {{\n    return n\n}}\n"
    ));
    assert!(
        !ambig.is_empty(),
        "a bare value matching two provenance arms is refused: {ambig:?}"
    );

    // [qual-arm-infer] Not for a literal union with an open provenance arm: a
    // plain value must be wrapped explicitly with the open constructor.
    let open = errors(
        "provenance qualifier Other<T> of T\n\
         fn other<T>(v: T) [] -> +Other T => !v {\n    return v\n}\n\
         type Class = \"A\" | \"B\" | Other Str\n\
         fn f(s: Str) -> Class => s {\n    return s\n}\n",
    );
    assert!(
        !open.is_empty(),
        "a plain value is not auto-wrapped into an open literal-union arm: {open:?}"
    );
}

/// [diag-id] The ids of the errors of `src` checked over **std's own**
/// declarations.
fn std_errors(src: &str) -> Vec<&'static str> {
    let files = [("main.sv", src)];
    // [std-cache] Over the process's std front (TESTS.md T3); the
    // expansion's own diagnostics are not these tests' subject.
    let program = salvo_testkit::front::program("rs", &files).program;
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = salvo_testkit::front::check_with("rs", &program, &resolution, &symbols);
    let mut all: Vec<_> = resolution.errors.iter().chain(checked.errors.iter()).filter(|d| d.is_error()).cloned().collect();
    // [fate-poison] As the gates do, the IR analyses run on every
    // declaration the checker's errors leave clean [ir-analyze]: a poisoned
    // read is the IR flow pass's since R8 piece
    // 3c [flow-use-after-move].
    all.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked).into_iter().filter(|d| d.is_error()));
    all.into_iter().map(|d| d.id.unwrap_or("?")).collect()
}

/// [col-no-literal] A bare collection constructor as an argument determines
/// the callee's type parameter, and the diagnostic still fires where nothing
/// *can* determine the argument (an empty `List()`), which is the rule the fix
/// had to leave standing.
#[test]
fn an_empty_literal_argument_still_needs_its_type() {
    let errs = std_errors(
        "fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let a = to_set(List())\n    \
         println(\"${size(a)}\")\n}\n",
    );
    assert!(
        errs.contains(&"call-type-args:cannot-infer"),
        "an empty literal determines nothing: {errs:?}"
    );
}
