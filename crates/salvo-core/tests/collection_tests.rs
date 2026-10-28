//! [col-key-eligible] What may be a **key**: a `Set` element, or a `Map`
//! key.
//!
//! Hashing and equality are what a hash container needs, and this version
//! answers them from the intrinsic types (`Int`, `Long`, `Str`, `Char`,
//! `Bool`). The two refusals worth testing are the ones a user will hit:
//! `Double`/`Float`, which Rust's `f64` cannot hash or compare for equality
//! at all (so accepting it would diverge between backends
//! [backend-parity]), and a struct, whose route in is `canbe hashed`.
//!
//! The rule is enforced in two places, because a key can arrive two ways:
//! *written* (`Map<Double, Str>` in a signature, caught by the declaration
//! walk) and *inferred* (`Set(1.5)`, where the key exists only in the
//! substitution the call resolved).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

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

fn errors(src: &str) -> Vec<FileDiagnostic> {
    let mut diags = checked(src).errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

fn assert_ok(src: &str) {
    let errs = errors(src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

/// The intrinsic key types are keys, in both containers.
#[test]
fn intrinsic_key_types_are_eligible() {
    assert_ok(
        "fn probe(a: Set<Str>, b: Set<Int>, c: Map<Long, Str>, d: Map<Char, Int>, \
         e: Map<Bool, Str>) -> Int => a, b, c, d, e {\n    return size(a)\n}\n",
    );
}

/// [col-key-eligible] `Double` is refused as a key, and the message says
/// why rather than just refusing — the reason (backend divergence) is not
/// guessable.
#[test]
fn a_double_key_is_refused_with_its_reason() {
    let msgs = messages("fn probe(s: Set<Double>) -> Int => s {\n    return size(s)\n}\n");
    assert_eq!(msgs.len(), 1, "expected exactly one error: {msgs:?}");
    assert!(
        msgs[0].contains("`Double` cannot be a key") && msgs[0].contains("NaN"),
        "unexpected message: {}",
        msgs[0]
    );
    let map = messages("fn probe(m: Map<Double, Str>) -> Int => m {\n    return size(m)\n}\n");
    assert_eq!(map.len(), 1, "expected exactly one error: {map:?}");
    assert!(map[0].contains("`Double` cannot be a key"), "{}", map[0]);
}

/// A map's **value** side is unrestricted: only the key has to hash.
#[test]
fn a_float_value_is_fine() {
    assert_ok("fn probe(m: Map<Str, Double>) -> Int => m {\n    return size(m)\n}\n");
}

/// [col-key-eligible] [obligation-by] A struct with no `hash` is not a key, and
/// the diagnostic names the clause that makes it one.
#[test]
fn a_struct_key_names_the_default_hashed_clause() {
    let msgs = messages(
        "struct Point {\n    x: Int,\n    y: Int\n}\n\n\
         fn probe(m: Map<Point, Str>) -> Int => m {\n    return size(m)\n}\n",
    );
    assert_eq!(msgs.len(), 1, "expected exactly one error: {msgs:?}");
    assert!(
        msgs[0].contains("Hashed<self> by auto"),
        "the message should name the clause: {}",
        msgs[0]
    );
}

/// The written rule reaches every declaration site the type walk visits,
/// not just parameters.
#[test]
fn an_ineligible_key_is_refused_at_every_declaration_site() {
    // return type
    assert_eq!(
        messages("fn probe() -> Set<Double> {\n    return Set()\n}\n").len(),
        1
    );
    // struct field
    assert_eq!(
        messages("struct Holder {\n    seen: Set<Double>\n}\n").len(),
        1
    );
    // `let` annotation
    assert_eq!(
        messages(
            "fn probe() -> Int {\n    let s: Set<Double> = Set()\n    return size(s)\n}\n"
        )
        .len(),
        1
    );
    // nested inside another container
    assert_eq!(
        messages("fn probe(xs: List<Map<Double, Int>>) -> Int => xs {\n    return 0\n}\n").len(),
        1
    );
}

/// [col-key-eligible] The **inferred** half: `Set(1.5)` writes no type
/// anywhere, so the ineligible key exists only in the resolved
/// substitution.
#[test]
fn an_inferred_ineligible_key_is_refused() {
    let msgs = messages("fn probe() -> Int {\n    let s = Set(1.5)\n    return size(s)\n}\n");
    assert!(
        msgs.iter().any(|m| m.contains("`Double` cannot be a key")),
        "expected the key refusal, got: {msgs:?}"
    );
    let map = messages(
        "fn probe() -> Int {\n    let m = Map((1.5, 2))\n    return 0\n}\n",
    );
    assert!(
        map.iter().any(|m| m.contains("`Double` cannot be a key")),
        "expected the key refusal, got: {map:?}"
    );
}

/// A generic parameter is *not* refused at the declaration: like
/// [linear-generics], the instantiation is where it is checked, so generic
/// code over keyed collections stays writable.
#[test]
fn a_type_variable_key_is_checked_at_the_instantiation_not_the_declaration() {
    assert_ok(
        "fn count<K>(m: Map<K, Str>) -> Int => m {\n    return size(m)\n}\n",
    );
    // ... and the instantiation is where the refusal lands.
    let msgs = messages(
        "fn count<K>(m: Map<K, Str>) -> Int => m {\n    return size(m)\n}\n\n\
         fn probe() -> Int {\n    let m = Map((1.5, \"x\"))\n    return count(m)\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("cannot be a key")),
        "expected the key refusal, got: {msgs:?}"
    );
}

/// [linear-generics] Linear values can never be keys, and this costs no new
/// rule: the constructors declare no `<T canbe linear>`, so the existing
/// instantiation ban refuses them. Worth a test because the *reason* is
/// specific — a set would silently drop a duplicate, discharging an
/// obligation by data-dependent control flow the checker cannot see
/// [col-key-eligible].
#[test]
fn a_linear_element_is_refused_by_the_instantiation_ban() {
    let msgs = messages(
        "linear struct Handle {\n    id: Int\n}\n\n\
         fn close(h: Handle) -> None => !h {\n    discard(h)\n}\n\n\
         fn probe() -> Int {\n    let h = Handle { id: 1 }\n    let s = Set(h)\n    \
         return size(s)\n}\n",
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("cannot instantiate generic parameter `T` of `Set`")),
        "expected the linear instantiation ban, got: {msgs:?}"
    );
}


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
    assert_ok(src);
    let out = checked(src);
    assert_eq!(
        out.local_calls.len(),
        1,
        "the call to the `keep` parameter should be recorded as a local call, \
         so the emitters do not dispatch it as an effect member"
    );
}


// ===== literals [col-no-literal] =====

/// The prelude above plus what the literals need: `List` for the bracket
/// form, and a struct to keep the bare-struct-literal reading honest.
const LIT_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Char\n",
    "export intrinsic type Double\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
    "export intrinsic type Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut\n",
    "export fns Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export fns Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export fns Hashed<T> {\n    fn hash(value: T) -> Long\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Str) [] -> Long => value\n",
    // [col-hashed-ordered] The interim structural container identities, as
    // std declared them (the fixture's one std file is every type's home).
    "export intrinsic fn cmp<T>(a: List<T>, b: List<T>) [] -> Int => a, b\n",
    "export intrinsic fn eq<T>(a: List<T>, b: List<T>) [] -> Bool => a, b\n",
    "export intrinsic fn hash<T>(value: List<T>) [] -> Long => value\n",
    "export intrinsic fn cmp<A, B>(a: (A, B), b: (A, B)) [] -> Int => a, b\n",
    "export intrinsic fn eq<A, B>(a: (A, B), b: (A, B)) [] -> Bool => a, b\n",
    "export intrinsic fn hash<A, B>(value: (A, B)) [] -> Long => value\n",
    "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
    "export intrinsic fn size<T>(set: Set<T>) [] -> Int => set\n",
    "export intrinsic fn size<K, V>(map: Map<K, V>) [] -> Int => map\n",
),
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b\n",
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

/// [fn-type-named] The collection constructors the type-named spellings
/// (`List(…)`, `Mut Set(…)`, `Map("k": v)`) resolve to, each in the `std.core`
/// module that owns it, as std declares them.
const LIT_CONSTRUCTORS: &[(&str, &str)] = &[
    (
        "core/list.sv",
        concat!(
            "export intrinsic fn List<T>() [] -> List<T>\n",
            "export intrinsic fn List<T>(first: T, ...rest: T[]) [] -> List<T> => !first\n",
            "export intrinsic fn Mut List<T>() [] -> Mut List<T>\n",
            "export intrinsic fn Mut List<T>(first: T, ...rest: T[]) [] -> Mut List<T> => !first\n",
        ),
    ),
    (
        "core/set.sv",
        concat!(
            "export intrinsic fn Set<T>(...elems: T[], ?Hashed<T>) [] -> Set<T>(?hash, ?eq)\n",
            "export intrinsic fn Mut Set<T>(...elems: T[], ?Hashed<T>) [] -> Mut Set<T>(?hash, ?eq)\n",
        ),
    ),
    (
        "core/map.sv",
        concat!(
            "export intrinsic fn Map<K, V>(...entries: (K, V)[], ?Hashed<K>) [] -> Map<K, V>(?hash, ?eq)\n",
            "export intrinsic fn Mut Map<K, V>(...entries: (K, V)[], ?Hashed<K>) [] -> Mut Map<K, V>(?hash, ?eq)\n",
        ),
    ),
];

fn lit_checked(src: &str) -> salvo_core::Checked {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        LIT_PRELUDE.to_string(),
        true,
    );
    for (path, text) in LIT_CONSTRUCTORS {
        sources.add(
            &format!("std/{path}"),
            SourceSet::classify(Path::new(&format!("std/{path}"))).unwrap(),
            text.to_string(),
            true,
        );
    }
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

fn lit_messages(src: &str) -> Vec<String> {
    lit_checked(src)
        .errors
        .into_iter()
        .filter(|d| d.is_error())
        .map(|d| d.message)
        .collect()
}

/// [col-no-literal] [fn-type-named] The three constructor forms type as their
/// collections, and `Mut` on the type name builds the mutable one.
#[test]
fn the_literals_type_as_their_collections() {
    let msgs = lit_messages(
        "fn probe() -> Int {\n    \
         let xs: List<Int> = List(1, 2)\n    \
         let s: Set<Str> = Set(\"a\", \"b\")\n    \
         let m: Map<Str, Int> = Map(\"a\": 1)\n    \
         let ms: Mut Set<Int> = Mut Set(1)\n    \
         let mm: Mut Map<Str, Int> = Mut Map(\"a\": 1)\n    \
         let mxs: Mut List<Int> = Mut List(3)\n    \
         return size(xs) + size(s) + size(m) + size(ms) + size(mm) + size(mxs)\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [col-no-literal] A brace whose first entry is `identifier:` is still a bare
/// **struct** literal — which is what makes a map key an expression rather
/// than a name.
#[test]
fn an_identifier_key_is_still_a_struct_literal() {
    let msgs = lit_messages(
        "struct Point {\n    x: Int,\n    y: Int\n}\n\n\
         fn probe() -> Int {\n    let p: Point = {x: 1, y: 2}\n    return p.x\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [col-no-literal] A *named* fieldless struct literal keeps working: only a
/// **bare** `{}` is an empty collection.
#[test]
fn a_named_fieldless_struct_literal_still_parses() {
    let msgs = lit_messages(
        "struct Finished {}\n\n\
         fn probe() -> Finished {\n    return Finished {}\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [col-no-literal] [fn-type-named] An empty constructor takes its element type
/// from its type arguments or from the position — and says so when there is
/// none.
#[test]
fn an_empty_literal_needs_its_type_from_the_position() {
    // From the constructor's own type arguments, either kind.
    let ok = lit_messages(
        "fn probe() -> Int {\n    \
         let s: Set<Int> = Set<Int>()\n    let m: Map<Str, Int> = Map<Str, Int>()\n    \
         let xs: List<Int> = List<Int>()\n    \
         return size(s) + size(m) + size(xs)\n}\n",
    );
    assert!(ok.is_empty(), "expected no errors, got: {ok:?}");

    // From a parameter type.
    let via_param = lit_messages(
        "fn takes(s: Set<Int>) -> Int => s {\n    return size(s)\n}\n\n\
         fn probe() -> Int {\n    return takes(Set())\n}\n",
    );
    assert!(via_param.is_empty(), "expected no errors, got: {via_param:?}");

    // With nothing to say what it is, an error naming the remedies.
    let bare_list = lit_messages("fn probe() -> Int {\n    let x = List()\n    return 0\n}\n");
    assert_eq!(bare_list.len(), 1, "expected one error: {bare_list:?}");
    assert!(
        bare_list[0].contains("cannot infer type argument `T` of `List`"),
        "unexpected message: {}",
        bare_list[0]
    );
    let bare = lit_messages("fn probe() -> Int {\n    let x = Set()\n    return 0\n}\n");
    assert!(
        bare.iter().any(|m| m.contains("cannot infer type argument `T` of `Set`")),
        "unexpected messages: {bare:?}"
    );
}

/// [col-key-eligible] A constructor's key type is checked like any other:
/// with no `hash` for `Double`, `Set(1.5)` and `Map(1.5: 1)` are refused —
/// as a key, once: the key rule is checked before the constructor's
/// implicits, and a key refused as such silences the `hash` it then cannot
/// find (2026-10-20; until then the refusal was the implicit's "no `hash`
/// fits").
#[test]
fn a_literal_with_an_ineligible_key_is_refused() {
    let set = lit_messages("fn probe() -> Int {\n    let s = Set(1.5)\n    return size(s)\n}\n");
    assert!(
        set.len() == 1 && set[0].contains("`Double` cannot be a key"),
        "expected the key refusal, once: {set:?}"
    );
    let map = lit_messages("fn probe() -> Int {\n    let m = Map(1.5: 1)\n    return size(m)\n}\n");
    assert!(
        map.len() == 1 && map[0].contains("`Double` cannot be a key"),
        "expected the key refusal, once: {map:?}"
    );
}

/// [col-pairs] A `Map(…)` call's entries all need values: pair arguments are
/// all-or-nothing, so a bare key among them is a parse error.
#[test]
fn a_map_literal_wants_a_value_for_every_entry() {
    let mut sources = SourceSet::default();
    let src = "fn probe() -> Int {\n    let m = Map(\"a\": 1, \"b\")\n    return 0\n}\n";
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let (_, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let errs: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert!(
        errs.iter().any(|m| m.contains("cannot be mixed with `k: v` pair arguments")),
        "expected the parse error, got: {errs:?}"
    );
}


// ===== equality and the key opt-ins [col-equality] [col-hashed-ordered] =====

/// [op-equality] Equality **ignores qualifiers** — it is about the data at the
/// moment of the check, so `Tagged Point == Point` resolves the same
/// `eq(Point, Point)`. (Since 2026-09-21 the struct has to *have* one: equality
/// is opt-in, and `Eq<self> by auto` is the one-token way to ask.)
#[test]
fn equality_ignores_qualifiers() {
    let msgs = lit_messages(
        "qualifier Tagged of Point\n\n\
         struct Point : Eq<self> by auto {\n    x: Int,\n    y: Int\n}\n\n\
         fn tagged(p: Point) -> +Tagged Point\n=> !p {\n    return p\n}\n\n\
         fn probe(a: Point, b: Point) -> Bool => !a, b {\n    \
         let t = tagged(a)\n    return t == b\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [op-equality] …and a struct with no `eq` cannot be compared at all, which is
/// what "equality is opt-in" means (user decision 2026-09-21, overturning
/// [col-equality]'s every-struct rule). The diagnostic names both ways to get
/// one.
#[test]
fn a_struct_without_an_eq_cannot_be_compared() {
    let msgs = lit_messages(
        "struct Point {\n    x: Int\n}\n\n\
         fn probe(a: Point, b: Point) -> Bool => a, b {\n    return a == b\n}\n",
    );
    assert_eq!(msgs.len(), 1, "expected one error: {msgs:?}");
    assert!(
        msgs[0].contains("declare a `eq` inside `Point`'s body")
            && msgs[0].contains("by auto"),
        "unexpected message: {}",
        msgs[0]
    );
}

/// [col-equality] [op-equality] Comparing two *different* struct types with no
/// `eq` taking the pair is an error, not a constant `false`: the operator is
/// the call `eq(A, B)`, which nothing declares (R10 D6 dropped the same-type
/// rule; the call's refusal is what remains).
#[test]
fn comparing_different_struct_types_is_refused() {
    let msgs = lit_messages(
        "struct A {\n    v: Int\n}\n\nstruct B {\n    v: Int\n}\n\n\
         fn probe(a: A, b: B) -> Bool => a, b {\n    return a == b\n}\n",
    );
    assert_eq!(msgs.len(), 1, "expected one error: {msgs:?}");
    assert!(
        msgs[0].contains("no `eq` for `==` on `A` and `B`"),
        "unexpected message: {}",
        msgs[0]
    );
}

/// [obligation-by] [comptime-instantiate] A fn-typed field bars the stamped
/// `eq`: the copy for that field finds no `eq` over a function value, and the
/// error lands at the field, prefixed with what was being stamped. It does not
/// bar the *struct* from equality: declare an `eq` that ignores the field, and
/// the type is comparable (and hashable, with a `hash` to match).
#[test]
fn a_fn_field_bars_the_structural_eq_but_not_a_hand_written_one() {
    let structural = lit_messages(
        "struct Holder : Eq<self> by auto {\n    f: (Int) -> Int\n}\n",
    );
    assert_eq!(structural.len(), 1, "expected one error: {structural:?}");
    assert!(
        structural[0].contains("in `eq` from `auto` for `Holder.f: (Int) -> Int`")
            && structural[0].contains("no matching overload for `eq(")
            && structural[0].contains("-> Int, (Int) -> Int)"),
        "unexpected message: {}",
        structural[0]
    );

    // [fn-attached] Declared *inside* the body: the hand-written `eq` is on the
    // type, and ignores the field the structural one could not compare.
    let by_hand = lit_messages(
        "struct Holder {\n    f: (Int) -> Int,\n    tag: Int\n\n\
         fn eq(a: Holder, b: Holder) [] -> Bool => a, b {\n        \
         return eq(a.tag, b.tag)\n    }\n}\n\n\
         fn probe(a: Holder, b: Holder) -> Bool => a, b {\n    return a == b\n}\n",
    );
    assert!(by_hand.is_empty(), "expected no errors, got: {by_hand:?}");
}

/// [op-order] Ordering needs a `cmp` — generated or hand-written — and says so
/// when there is none.
#[test]
fn ordering_needs_a_cmp() {
    let generated = lit_messages(
        "struct P : Ordered<self> by auto {\n    x: Int\n}\n\n\
         fn probe(a: P, b: P) -> Bool => a, b {\n    return a < b\n}\n",
    );
    assert!(generated.is_empty(), "expected no errors, got: {generated:?}");

    // [fn-attached] Hand-written, declared inside the body.
    let by_hand = lit_messages(
        "struct P {\n    x: Int\n\n\
         fn cmp(a: P, b: P) [] -> Int => a, b {\n        return cmp(a.x, b.x)\n    }\n}\n\n\
         fn probe(a: P, b: P) -> Bool => a, b {\n    return a < b\n}\n",
    );
    assert!(by_hand.is_empty(), "expected no errors, got: {by_hand:?}");

    // And with neither, the diagnostic names both ways to declare one.
    let msgs = lit_messages(
        "struct P {\n    x: Int\n}\n\n\
         fn probe(a: P, b: P) -> Bool => a, b {\n    return a < b\n}\n",
    );
    assert_eq!(msgs.len(), 1, "expected one error: {msgs:?}");
    assert!(
        msgs[0].contains("declare a `cmp` inside `P`'s body")
            && msgs[0].contains("by auto"),
        "unexpected message: {}",
        msgs[0]
    );
}

/// [obligation-by] [comptime-refuse] The `by auto` clauses are validated where
/// they are written: `std.core.auto`'s `hash` **refuses** a `canbe Mut` struct (a key
/// that can change under the collection holding it), and every field has to
/// resolve its own `hash`/`cmp` — the rules `canbe hashed`/`canbe ordered` used
/// to carry, now stated in std rather than in the compiler.
#[test]
fn the_default_clauses_are_validated_at_the_declaration() {
    // A `canbe Mut` struct could change under the collection holding it. The
    // refusal is one error; the obligation left unfulfilled by it is another,
    // at the clause.
    let mutable = lit_checked("struct K : Hashed<self> by auto canbe Mut {\n    v: Int\n}\n").errors;
    assert!(
        mutable.iter().any(|d| d.id == Some("comptime-refuse:refused")
            && d.message.contains("`K` refused")
            && d.message.contains("`Mut`-capable struct")),
        "unexpected messages: {mutable:?}"
    );

    // A float field has no `hash` (by neither backend) — reported at the
    // field, in the stamped copy's terms. `Hashed` brings `eq`, and the
    // prelude here declares no `eq(Double)`, so that copy fails too.
    let float_hash = lit_messages("struct K : Hashed<self> by auto {\n    v: Double\n}\n");
    assert!(
        float_hash
            .iter()
            .any(|m| m.contains("in `hash` from `auto` for `K.v: Double`") && m.contains("hash(Double)")),
        "unexpected messages: {float_hash:?}"
    );

    // ... nor a `cmp`.
    let float_ord = lit_messages("struct K : Ordered<self> by auto {\n    v: Double\n}\n");
    assert_eq!(float_ord.len(), 1, "expected one error: {float_ord:?}");
    assert!(
        float_ord[0].contains("in `cmp` from `auto` for `K.v: Double`") && float_ord[0].contains("cmp(Double, Double)"),
        "unexpected message: {}",
        float_ord[0]
    );

    // A float field is fine for **equality**, which Salvo owns [kt-float-eq].
    let float_eq = lit_messages("struct K : Eq<self> by auto {\n    v: Double\n}\n");
    assert!(float_eq.is_empty(), "expected no errors, got: {float_eq:?}");

    // A nested struct must itself be hashable — which now means *having* a
    // `hash`, not declaring an opt-in.
    let nested = lit_messages(
        "struct Inner {\n    v: Int\n}\n\n\
         struct K : Hashed<self> by auto {\n    i: Inner\n}\n",
    );
    assert!(
        nested
            .iter()
            .any(|m| m.contains("in `hash` from `auto` for `K.i: Inner`") && m.contains("hash(Inner)")),
        "unexpected messages: {nested:?}"
    );

    // Lists and tuples qualify when their elements do (user, 2026-09-12).
    let containers = lit_messages(
        "struct K : Hashed<self> by auto, Ordered<self> by auto {\n    \
         parts: List<Int>,\n    pair: (Str, Int)\n}\n",
    );
    assert!(containers.is_empty(), "expected no errors, got: {containers:?}");
}

/// [col-key-eligible] [cmp-auto] A struct with the structural pair *is* a
/// valid key — the whole point of asking for it.
#[test]
fn a_hashed_struct_is_a_valid_key() {
    let msgs = lit_messages(
        "struct Point : Hashed<self> by auto {\n    x: Int,\n    y: Int\n}\n\n\
         fn probe(s: Set<Point>, m: Map<Point, Str>) -> Int => s, m {\n    \
         return size(s) + size(m)\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [qual-canbe-optin] The `canbe` list is closed — and since 2026-09-21 it holds only
/// `Mut` and `once`: `hashed`/`ordered` were deleted, with a message naming the
/// obligation that replaced them [cmp-auto].
#[test]
fn canbe_rejects_an_unknown_optin() {
    let msgs = lit_messages("struct K canbe Sorted {\n    v: Int\n}\n");
    assert!(
        msgs.iter()
            .any(|m| m.contains("`Mut`") && m.contains("`once`")),
        "the message should list the opt-ins, got: {msgs:?}"
    );

    let deleted = lit_messages("struct K canbe hashed {\n    v: Int\n}\n");
    assert!(
        deleted
            .iter()
            .any(|m| m.contains("no longer exists") && m.contains(": Hashed<self> by auto")),
        "the message should name the replacement, got: {deleted:?}"
    );
}


// ===== sorted collections [col-sorted] =====

/// The literal prelude plus the sorted types, so the key rules can be
/// compared side by side.
const SORTED_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Char\n",
    "export intrinsic type Double\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
    "export intrinsic type Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut\n",
    "export intrinsic type SortedSet<T>(?cmp: (T, T) -> Int) canbe Mut\n",
    "export intrinsic type SortedMap<K, V>(?cmp: (K, K) -> Int) canbe Mut\n",
    "export fns Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export fns Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export fns Hashed<T> {\n    fn hash(value: T) -> Long\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn size<T>(set: SortedSet<T>) [] -> Int => set\n",
    "export intrinsic fn size<K, V>(map: SortedMap<K, V>) [] -> Int => map\n",
    "export intrinsic fn size<T>(set: Set<T>) [] -> Int => set\n",
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

fn sorted_messages(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        SORTED_PRELUDE.to_string(),
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
    checked.errors.extend(expansion.diagnostics);
    checked
        .errors
        .into_iter()
        .filter(|d| d.is_error())
        .map(|d| d.message)
        .collect()
}

/// [col-sorted] A sorted collection's key must be **orderable**, which is a
/// different bar from hashable — and the diagnostic says which one it is.
#[test]
fn a_sorted_key_must_be_orderable() {
    let ok = sorted_messages(
        "struct P : Ordered<self> by auto {\n    x: Int\n}\n\n\
         fn probe(a: SortedSet<Str>, b: SortedMap<Int, Str>, c: SortedSet<P>) -> Int \
         => a, b, c {\n    return size(a) + size(b) + size(c)\n}\n",
    );
    assert!(ok.is_empty(), "expected no errors, got: {ok:?}");

    // A struct with only the hashing pair is a `Set` key but not a `SortedSet`
    // one: the two axes are separate.
    let hashed_only = sorted_messages(
        "struct P : Hashed<self> by auto {\n    x: Int\n}\n\n\
         fn probe(a: SortedSet<P>) -> Int => a {\n    return size(a)\n}\n",
    );
    assert_eq!(hashed_only.len(), 1, "expected one error: {hashed_only:?}");
    assert!(
        hashed_only[0].contains("sorted collection's key")
            && hashed_only[0].contains("Ordered<self> by auto"),
        "unexpected message: {}",
        hashed_only[0]
    );
}

/// [col-sorted] A **union** can be hashed but never ordered: comparing values
/// of different types has no obvious meaning (user decision 2026-09-12).
#[test]
fn a_union_is_not_a_sorted_key() {
    let msgs = sorted_messages(
        "fn probe(a: SortedSet<Int | Str>) -> Int => a {\n    return size(a)\n}\n",
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("sorted collection's key") && m.contains("union")),
        "expected the union refusal, got: {msgs:?}"
    );
}

/// [col-sorted] Floats are out of both: no total order on Rust's `f64`.
#[test]
fn a_float_is_not_a_sorted_key() {
    let msgs = sorted_messages(
        "fn probe(a: SortedSet<Double>) -> Int => a {\n    return size(a)\n}\n",
    );
    assert_eq!(msgs.len(), 1, "expected one error: {msgs:?}");
    assert!(
        msgs[0].contains("total order"),
        "unexpected message: {}",
        msgs[0]
    );
}


// ===== generated constructors and converters [col-by] [col-convert] =====

/// [col-by] [col-convert] The generated constructors and the converters
/// type as their collections, and the two `to_map` forms are told apart by
/// arity.
#[test]
fn the_generated_constructors_and_converters_type_correctly() {
    let prelude = concat!(
        "export intrinsic type Int\n",
        "export intrinsic type Str canbe Mut\n",
        "export intrinsic type List<T> canbe Mut\n",
        "export intrinsic type Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool) canbe Mut\n",
        "export intrinsic type Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) -> Bool) canbe Mut\n",
        "export intrinsic fn size<T>(set: Set<T>) [] -> Int => set\n",
        "export intrinsic fn size<K, V>(map: Map<K, V>) [] -> Int => map\n",
        "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
        "export intrinsic fn list_by<T>(size: Int, init: (Int) -> T) [] -> List<T> => size, init\n",
        "export intrinsic fn set_by<T>(size: Int, init: (Int) -> T) [] -> Set<T> => size, init\n",
        "export intrinsic fn map_by<K, V>(size: Int, init: (Int) -> (K, V)) [] -> Map<K, V>\n    => size, init\n",
        "export intrinsic fn to_set<T>(list: List<T>) [] -> Set<T> => list\n",
        "export intrinsic fn to_map<K, V>(pairs: List<(K, V)>) [] -> Map<K, V> => pairs\n",
        "export intrinsic fn to_map<T, K, V>(items: List<T>, entry: (T) -> (K, V)) [] -> Map<K, V>\n    => items, entry\n",
        // [cmp-groups] What the containers' slots default to.
        "export intrinsic type Long\n",
        "export intrinsic type Bool\n",
        "export intrinsic fn hash(value: Int) [] -> Long => value\n",
        "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
        // [op-arith] What `+` calls.
        "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    );
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        prelude.to_string(),
        true,
    );
    let src = "fn probe() -> Int {\n    \
               let xs = list_by(3, i -> i + 1)\n    \
               let s = set_by(3, i -> i)\n    \
               let m = map_by(2, i -> (i, i))\n    \
               let u = to_set(xs)\n    \
               let pairs = list_by(1, i -> (1, 2))\n    \
               let m2 = to_map(pairs)\n    \
               let m3 = to_map(xs, x -> (x, x))\n    \
               return size(xs) + size(s) + size(m) + size(u) + size(m2) + size(m3)\n}\n";
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
    checked.errors.extend(expansion.diagnostics);
    let errs: Vec<String> = checked
        .errors
        .into_iter()
        .filter(|d| d.is_error())
        .map(|d| d.message)
        .collect();
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}


// ===== C-6: the claims std ships over a list =====
//
// Only the parts that need no std declarations live here: this harness builds
// its own prelude, and the element bar below is checked on the qualifier's
// *name*, so it fires without `Sorted` being in scope. The rest of C-6 is
// asserted end to end against the real std, in each backend's
// `compiles_and_runs_list_claims` case — where parity is asserted too.

/// [col-sorted-list] A `Sorted` claim is about the element order, so the
/// elements must be orderable — refused where the claim is *written*.
#[test]
fn a_sorted_list_needs_orderable_elements() {
    let errs = messages("fn f(xs: Sorted List<Double>) [] -> None => xs {\n}\n");
    assert!(
        errs.iter()
            .any(|m| m.contains("cannot be the element of a `Sorted List`")),
        "expected the sorted-element bar, got: {errs:?}"
    );
}


// ===== [qual-overload] one qualifier name over several subject types =====
//
// The *mechanics* are asserted here with locally declared qualifiers, since
// this harness builds its own prelude; that std really declares `NonEmpty`
// five times over is asserted end to end in each backend's
// `compiles_and_runs_list_claims` case.

/// Two qualifiers may share a name when their subjects differ, and each
/// applies to its own. Before overloading the second declaration was a
/// "duplicate qualifier" error and only one subject could have the name.
#[test]
fn one_qualifier_name_serves_several_subjects() {
    let errs = messages(
        "qualifier Filled<T> of List<T>\n\
         qualifier Filled<T> of Set<T>\n\n\
         fn f(a: Filled List<Int>, b: Filled Set<Int>) [] -> None => a, b {\n}\n",
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// …and the applies-to check is still on: a subject no declaration of that
/// name accepts is an error, so overloading did not turn it off.
#[test]
fn a_qualifier_still_has_to_apply_to_its_subject() {
    let errs = messages(
        "qualifier Filled<T> of List<T>\n\
         qualifier Filled<T> of Set<T>\n\n\
         fn f(n: Filled Int) [] -> None => n {\n}\n",
    );
    assert!(
        errs.iter().any(|m| m.contains("does not apply to `Int`")),
        "expected the applies-to check, got: {errs:?}"
    );
}

/// Two declarations of one name over **one** subject are a duplicate, not an
/// overload: nothing at a use site could tell them apart.
#[test]
fn one_name_twice_over_one_subject_is_a_duplicate() {
    let errs = messages(
        "qualifier Twice<T> of List<T>\n\
         qualifier Twice<T> of List<T>\n",
    );
    assert!(
        errs.iter().any(|m| m.contains("duplicate qualifier `Twice`")),
        "expected the duplicate error, got: {errs:?}"
    );
}

/// [cmp-carry] The keyed containers **name** the ordering or the hash they keep
/// their keys by, defaulted to the canonical one (user decision 2026-09-22). The
/// default is not materialized, so `Set<Str>` is the type it always was — which
/// is what keeps this out of everything that reads a container's arguments.
#[test]
fn a_keyed_container_defaults_to_the_canonical_identity() {
    let msgs = sorted_messages(
        "fn probe(s: Set<Int>, o: SortedSet<Int>) -> Int => s, o {\n    \
         return size(s) + size(o)\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// [cmp-carry] A signature may **omit** a keyed container's identity arguments,
/// and then it says nothing about them: `size(set: SortedSet<T>)` takes a set
/// however it is ordered, which is what keeps std's own surface generic over the
/// ordering without writing a binder in thirty signatures. Unstated is
/// unconstrained, the same rule an unwritten slot has.
#[test]
fn a_signature_may_omit_a_containers_identity() {
    let msgs = sorted_messages(
        "fn count<T>(s: SortedSet<T>) -> Int => s {\n    return size(s)\n}\n\n\
         fn probe(a: SortedSet<Int>, b: SortedSet<Str>) -> Int => a, b {\n    \
         return count(a) + count(b)\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// …and a **named** one is accepted: the ordering is part of the type, and what
/// a backend does with it is the backend's business — the Rust emitter keeps the
/// collection by it, and the Kotlin emitter refuses until its runtime can
/// [backend-never-wrong].
#[test]
fn a_keyed_container_may_name_its_ordering() {
    let msgs = sorted_messages(
        "fn by_size(a: Int, b: Int) -> Int => a, b {\n    return 0\n}\n\n\
         fn probe(s: SortedSet<Int>(by_size)) -> Int => s {\n    return size(s)\n}\n",
    );
    assert!(msgs.is_empty(), "expected no errors, got: {msgs:?}");
}

/// …and an identity that does not *fit* the slot is the ordinary mismatch, named
/// at the type that wrote it.
#[test]
fn a_container_identity_must_fit_its_slot() {
    let msgs = sorted_messages(
        "fn wrong(a: Int) -> Int => a {\n    return a\n}\n\n\
         fn probe(s: SortedSet<Int>(wrong)) -> Int => s {\n    return size(s)\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("does not fit the `cmp` slot")),
        "expected the slot mismatch, got: {msgs:?}"
    );
}


/// [type-canbe-mut] A struct literal's written `Mut` is validated like any
/// written type: `Mut Cell { … }` for a struct without `canbe Mut` is refused
/// (ROADMAP §0g: it used to pass the checker, run on Rust and fail in kotlinc).
#[test]
fn mut_literal_of_a_struct_without_canbe_mut_is_refused() {
    let msgs = messages(
        "struct Cell { v: Int }\nfn probe() -> Int {\n    let c = Mut Cell { v: 1 }\n    \
         return c.v\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("`Mut` does not apply to `Cell`")),
        "unexpected messages: {msgs:?}"
    );
}
