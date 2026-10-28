//! [col-key-eligible] [call-resolve] What is left of the collection tests
//! once their diagnostics became test data, `tests/check/collections/`
//! (TESTS.md T1): a test of the checker's `local_calls` table, over the real
//! std, and one diagnostic test kept on its old prelude, which says why.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [call-resolve] A **local** holding a function outranks a same-named
/// effect member.
///
/// Reproduced 2026-09-13 as a latent defect: `Symbols::effect_of_fn` is a
/// program-wide name map with no notion of scope or locals, and both
/// emitters asked it first — so a program declaring
/// `effect Sink { fn keep(...) }` made std's
/// `filter(it, keep: (T) -> Bool)` emit an effect dispatch for its own
/// parameter, surfacing as "no handler for effect `Sink`" *from inside
/// `core/seq.sv`*. The checker always had the precedence right; the fix
/// records its decision (`Checked::local_calls`) so the emitters agree.
///
/// Checked here rather than in a backend crate because the record is the
/// checker's: an emitter test would only see the symptom.
#[test]
fn a_fn_typed_local_outranks_a_same_named_effect_member() {
    let src = "effect Sink {\n    fn keep(n: Int) -> None\n}\n\n\
               fn apply(keep: (Int) -> Bool, x: Int) -> Bool => keep, x {\n    \
               return keep(x)\n}\n";
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "expected no errors, got: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    assert_eq!(
        out.checked.local_calls.iter().filter(|k| k.file() >= user).count(),
        1,
        "the call to the `keep` parameter should be recorded as a local call, \
         so the emitters do not dispatch it as an effect member"
    );
}

/// [linear-generics] Linear values can never be keys, and this costs no new
/// rule: the constructors declare no `<T canbe linear>`, so the existing
/// instantiation ban refuses them. Worth a test because the *reason* is
/// specific — a set would silently drop a duplicate, discharging an
/// obligation by data-dependent control flow the checker cannot see
/// [col-key-eligible].
//
// Kept on the old prelude: over the real std, the one `Set(h)` also reports
// `implicit-resolve:no-fit` twice (no `hash`, no `eq` for `Handle`, the `eq`
// "in scope" shown as `(eff[E], eff[E]) -> Bool`) beside the two
// `linear-generics` errors, where a written `Set<Handle>` would be one
// `col-key-eligible:key`: one mistake, several diagnostics.
#[test]
fn a_linear_element_is_refused_by_the_instantiation_ban() {
    let ids = error_ids(
        "linear struct Handle {\n    id: Int\n}\n\n\
         fn close(h: Handle) -> None => !h {\n    discard(h)\n}\n\n\
         fn probe() -> Int {\n    let h = Handle { id: 1 }\n    let s = Set(h)\n    \
         return size(s)\n}\n",
    );
    assert!(ids.contains(&"linear-generics:not-opted"), "expected the linear instantiation ban, got: {ids:?}");
}

// ===== the old prelude, for `a_linear_element_is_refused_by_the_instantiation_ban` =====

/// [platform-intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`).
const STD_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Char\n",
    "export intrinsic type Double\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
    // [cmp-groups] The capability functions the containers' slots default to.
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic type Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut\n",
    "export intrinsic fn Set<T>(...elems: T[]) [] -> Set<T>\n",
    "export intrinsic fn Mut Set<T>(...elems: T[]) [] -> Mut Set<T>\n",
    "export intrinsic fn Map<K, V>(...entries: (K, V)[]) [] -> Map<K, V>\n",
    "export intrinsic fn add<T>(set: Mut Set<T>, elem: T) [] -> Bool => set: Mut, !elem\n",
    "export intrinsic fn size<T>(set: Set<T>) [] -> Int => set\n",
    "export intrinsic fn size<K, V>(map: Map<K, V>) [] -> Int => map\n",
    "export intrinsic fn put<K, V>(map: Mut Map<K, V>, key: K, value: V) [] -> None\n    => map: Mut, !key, !value\n",
),
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn plus(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn minus(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn times(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn divide(a: Double, b: Double) [] -> Double => a, b\n",
);

/// [obligation-by] The structural `comptime fn`s `by auto` stamps, mirroring
/// `lib/std/core/auto.sv`'s struct half: the tests here are about what a `by`
/// clause does to a type, not about the bodies.
const AUTO_PRELUDE: &str = concat!(
    "export intrinsic fn mix_hash(seed: Long, value: Long) [] -> Long => seed, value\n",
    "export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {\n",
    "    [for field in T.fields] {\n",
    "        let c = cmp(a.[field], b.[field])\n",
    "        if c != 0 {\n            return c\n        }\n",
    "    }\n    return 0\n}\n",
    "export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {\n",
    "    [for field in T.fields] {\n",
    "        if !eq(a.[field], b.[field]) {\n            return false\n        }\n",
    "    }\n    return true\n}\n",
    "export comptime fn hash<T is Struct>(value: T) [] -> Long => value {\n",
    "    [if T.mutable] {\n",
    "        refuse!(\"a `Mut`-capable struct can change while a collection holds it, so it cannot be a key\")\n",
    "    }\n",
    "    let h = 17L\n",
    "    [for field in T.fields] {\n",
    "        h = mix_hash(h, hash(value.[field]))\n",
    "    }\n    return h\n}\n",
);

fn checked(src: &str) -> salvo_core::Checked {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/auto.sv",
        SourceSet::classify(Path::new("std/core/auto.sv")).unwrap(),
        AUTO_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    // [comptime-instantiate] The `by` sites stamp here, before resolution.
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let mut checked = check_program(&program, &resolution, &symbols);
    // A stamping refusal is an error of the program like any other.
    checked.errors.extend(expansion.diagnostics);
    checked
}

/// [diag-id] The id of every error `src` reports.
fn error_ids(src: &str) -> Vec<&'static str> {
    checked(src).errors.into_iter().filter(|d| d.is_error()).map(|d| d.id.unwrap_or("?")).collect()
}
