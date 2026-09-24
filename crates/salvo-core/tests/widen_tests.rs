//! The widening check `^` [qual-lift].
//!
//! `^` is the dual of `is`: where a successful `is` narrows the subject (a
//! qualifier added, a union arm picked), a successful `^` **generalizes** it
//! by removing the listed qualifiers. Boolean-valued, same places, same
//! runtime test — only the type in the branch differs. Its reason to exist
//! is the qualified union: `Ok (Ok Int | Err Str)` is a claim *about* a
//! union, and `is ^Ok` is what lets a branch `when` the union inside it.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn first<T>(list: List<T>) [] -> proj(list) T? => list\n";

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
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
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

struct Person canbe Mut {
    name: Str
}

qualifier Ok<T> of T
qualifier Err<T> of T
qualifier Surname of Person

fn note(text: Str) [] -> None => text {}
fn read(p: Person) [] -> None => p {}
fn touch(p: Mut Person) [] -> None => p: Mut {}

fn ok(value: Int) [] -> +Ok Int {
    return value
}

fn err(value: Str) [] -> +Err Str => !value {
    return value
}
"#;

/// [qual-lift] The motivating case: a `^` branch head tests the arm *and*
/// removes the claim, so the branch can `when` the union inside it — no
/// intermediate binding and no repeated type.
#[test]
fn a_widening_branch_opens_a_nested_union() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn outcome() [] -> Ok (Ok Int | Err Str) | Err Str {{ return err(\"e\") }}\n\
         fn probe() [] -> None {{\n\
         let o = outcome()\n\
         when o {{\n\
         is ^Ok {{\n\
         when o {{\n\
         is Ok {{ note(\"value\") }}\n\
         is Err {{ note(\"inner error\") }}\n\
         }}\n\
         }}\n\
         is Err {{ note(\"outer error\") }}\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-lift] In an `if`, the same check on a plain qualified value: the
/// qualifier is statically present, so it cannot fail — and the subject
/// reads without it inside the branch, which is what picks the *unqualified*
/// overload.
#[test]
fn widening_strips_a_qualifier_in_an_if() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Mut Person) [] -> None => p: Mut {{\n\
         if p is ^Mut {{\n\
         read(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-lift] And the point of it: after widening, the value no longer
/// satisfies what the qualifier granted.
#[test]
fn a_widened_value_loses_what_the_qualifier_granted() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Mut Person) [] -> None => p: Mut {{\n\
         if p is ^Mut {{\n\
         touch(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        !errs.is_empty(),
        "expected the `Mut`-needing call to be rejected after widening"
    );
}

// ===== the exclusion list =====

/// [qual-lift] The capability qualifiers that may never be dropped, read
/// from the *same* list `Qual T <: T` uses (user decision 2026-09-05), so
/// the two cannot drift.
#[test]
fn intrinsic_capability_qualifiers_cannot_be_widened_away() {
    // `proj` is in the same list; since 2026-09-11 it *is* writable
    // [proj-anywhere], but it is stripped at lowering (a borrow is
    // provenance, not part of the type), so a `^` never meets it.
    for (qual, needle) in [("once", "once-callable"), ("Linear", "use obligation")] {
        let errs = errors(&format!(
            "{PRELUDE}\n\
             fn probe(p: Person) [] -> None => p {{\n\
             if p is ^{qual} {{\n\
             read(p)\n\
             }}\n\
             }}\n"
        ));
        assert!(
            errs.iter()
                .any(|e| e.contains(&format!("`{qual}` cannot be removed with `^`"))
                    && e.contains(needle)),
            "expected `{qual}` to be rejected with its reason, got: {errs:?}"
        );
    }
}

// ===== the error cases =====

/// [qual-lift] Nothing to remove is an error, not a silently-false test
/// (user decision 2026-09-05).
#[test]
fn widening_a_qualifier_the_value_lacks_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Person) [] -> None => p {{\n\
         if p is ^Surname {{\n\
         read(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("does not carry `Surname`")),
        "expected the nothing-to-remove error, got: {errs:?}"
    );
}

/// [qual-lift] `^` removes qualifiers; a *type* on the right is an `is`
/// [qual-lift] A check **mixes** lifted and kept terms — `is ^Ok Int` marks
/// the qualifier but not the base type — and that is refused at the parse, not
/// the check: some terms marked and some not is a syntactic property, and the
/// parser cannot tell a qualifier from a type name (both are uppercase). The
/// old spelling `^ Ok Int` was refused too, by the checker's qualifiers-only
/// rule, so nothing expressible has been lost. Lifting a qualifier *while*
/// naming the arm's base type stays unexpressible for now.
#[test]
fn a_check_mixing_lifted_and_kept_terms_is_rejected() {
    let src = "fn probe(o: Ok Int | Err Str) {\n    if o is ^Ok Int {\n        note(o)\n    }\n}\n";
    let (_ast, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("lifts every qualifier or none")),
        "expected the mixed-check error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// [qual-lift] One widened view cannot stand for two arms: each would peel
/// a different wrapper position.
#[test]
fn widening_more_than_one_arm_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn pair() [] -> Ok Int | Ok Str {{ return ok(1) }}\n\
         fn probe() [] -> None {{\n\
         let o = pair()\n\
         if o is ^Ok {{\n\
         note(\"ok\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("matches more than one arm")),
        "expected the multi-arm rejection, got: {errs:?}"
    );
}

/// [qual-lift] A lift **takes a binding** (user decision 2026-09-21,
/// superseding the 2026-09-05 no-binding rule): `is ^Mut plain` names the
/// lifted value. That is what makes a multi-arm lift possible — the lifted
/// value is materialized once, into the binding — so the test asserts both
/// halves: the single-arm binding checks clean, and a *two*-arm lift is
/// accepted with a binding and refused without one, naming it.
#[test]
fn a_lift_takes_a_binding_and_a_multi_arm_lift_needs_one() {
    let single = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Mut Person) [] -> None => p: Mut {{\n\
         if p is ^Mut plain {{\n\
         read(plain)\n\
         }}\n\
         }}\n"
    ));
    assert!(single.is_empty(), "expected a clean lift binding, got: {single:?}");

    // Two `Ok` arms: the lifted value is `Int | Str`, which needs a name.
    let bound = errors(&format!(
        "{PRELUDE}\n\
         fn outcome() [] -> Ok Int | Ok Str | Err Str {{ return err(\"e\") }}\n\
         fn probe() [] -> None {{\n\
         let o = outcome()\n\
         if o is ^Ok value {{\n\
         note(\"lifted\")\n\
         }}\n\
         }}\n"
    ));
    // [rewrap] A multi-arm lift is built end to end since 2026-09-21: the bound
    // value spans fewer arms than the storage, so it is produced by mapping arm
    // to arm — the same mapping an annotated `let` gets [let-infer].
    assert!(bound.is_empty(), "expected a clean multi-arm lift, got: {bound:?}");

    let unbound = errors(&format!(
        "{PRELUDE}\n\
         fn outcome() [] -> Ok Int | Ok Str | Err Str {{ return err(\"e\") }}\n\
         fn probe() [] -> None {{\n\
         let o = outcome()\n\
         if o is ^Ok {{\n\
         note(\"lifted\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        unbound
            .iter()
            .any(|e| e.contains("matches more than one arm") && e.contains("bind the")),
        "expected the multi-arm refusal naming the binding, got: {unbound:?}"
    );
}

/// [qual-lift] A `^` branch consumes the arms it matched, exactly as `is`
/// does, so exhaustiveness needs no new rule.
#[test]
fn a_widening_branch_consumes_its_arms() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn outcome() [] -> Ok Int | Err Str {{ return err(\"e\") }}\n\
         fn probe() [] -> None {{\n\
         let o = outcome()\n\
         when o {{\n\
         is ^Ok {{ note(\"ok\") }}\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("non-exhaustive `when`") && e.contains("Err Str")),
        "expected the unhandled arm to be reported, got: {errs:?}"
    );
}

// ===== constructing a group [qual-group] =====
//
// The other half of the qualified union: `^` *reads* a group, and these build
// one. `Ty::Qualified` keeps a flat, sorted, deduplicated qualifier list whose
// base is never itself `Qualified`, so applying a qualifier to an
// already-qualified value appends rather than nests — `emitted(ok(1))` is
// `Qualified { quals: [Emitted, Ok], base: Int }`, shape-identical to "two
// qualifiers on an `Int`". Where the expected arm is `Q (A | B)`, the value is
// read the other way round: the group's qualifiers come off the list and the
// remainder tries the union's arms. Fixed 2026-09-10; before that the
// construction needed an annotated intermediate `let`.

/// The declarations that make a nested group reachable: a second qualifier
/// (`Ok` alone cannot nest, see the deduplication test below) and its
/// constructor, in this file as [qual-ctor-same-file] requires.
const GROUP_PRELUDE: &str = r#"
qualifier Emitted<T> of T

fn emitted<T>(value: T) [] -> +Emitted T => !value {
    return value
}
"#;

/// [qual-group] The defect this closed: a qualifier applied to an
/// already-qualified value flattens, and the arm it must land in is a
/// qualified union group.
#[test]
fn a_qualifier_applied_to_a_qualified_value_reaches_a_group_arm() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step(flag: Bool) [] -> Emitted (Ok Int | Err Str) | Err Str {{\n\
         if flag {{ return emitted(ok(1)) }}\n\
         return err(\"done\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-group] Either inner arm, so the arm index really is derived from the
/// remainder rather than assumed to be the first.
#[test]
fn the_remainder_picks_the_inner_arm() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step(flag: Bool) [] -> Emitted (Ok Int | Err Str) | Ok Bool {{\n\
         if flag {{ return emitted(err(\"bad\")) }}\n\
         return emitted(ok(1))\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-group] The remainder still has to fit: the group's qualifier being
/// present is not on its own a licence to enter the arm. (The outer union's
/// other arm is deliberately unrelated — `Emitted` is droppable, so an
/// `Err Str` arm would legitimately accept `Emitted Err Str` and this would
/// test nothing.)
#[test]
fn a_remainder_that_fits_no_inner_arm_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step() [] -> Emitted (Ok Int | Ok Str) | Ok Bool {{\n\
         return emitted(err(\"bad\"))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no arm of") && e.contains("Emitted Err Str")),
        "expected the no-arm error, got: {errs:?}"
    );
}

/// [qual-group] The limit of the flat representation, and the reason the
/// annotated-`let` leftover survives: the *same* qualifier twice deduplicates
/// to one, so `ok(ok(x))` is indistinguishable from `ok(x)` and no rule can
/// recover the nesting. The diagnostic says so, since the type it prints
/// looks like it should fit.
#[test]
fn the_same_qualifier_twice_deduplicates_and_says_so() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn step() [] -> Ok (Ok Int | Err Str) | Err Str {{\n\
         return ok(ok(1))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("deduplicates") && e.contains("annotated local")),
        "expected the deduplication hint, got: {errs:?}"
    );
}

// ===== `proj` placement [proj-anywhere] [proj-field] =====

/// [proj-anywhere] `proj(p)` is an ordinary qualifier now, writable
/// wherever a type appears: the old return prefix and the union-arm spelling
/// mean the same thing, and a parameter may be a bare `proj`.
#[test]
fn proj_is_writable_in_arm_and_parameter_positions() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn head_a(list: List<Person>) [] -> proj(list) Person? => list {{\n    return first(list)\n}}\n\
         fn head_b(list: List<Person>) [] -> (proj(list) Person)? => list {{\n    return first(list)\n}}\n\
         fn hold(p: proj Person) [] -> Int => p {{\n    return 1\n}}\n"
    ));
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [proj-anywhere] A `proj` in a *return* type must say what it borrows from.
#[test]
fn a_proj_return_without_a_source_is_an_error() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn head(list: List<Person>) [] -> proj Person? => list {{\n    return first(list)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("must name its source")),
        "got: {errs:?}"
    );
}

/// [proj-anywhere] The source must be a parameter — checked for every
/// occurrence, so a tuple borrowing from two parameters names each.
#[test]
fn a_proj_source_must_be_a_parameter() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn head(list: List<Person>) [] -> (proj(nope) Person)? => list {{\n    return first(list)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`proj(nope)` names no parameter")),
        "got: {errs:?}"
    );
}


/// [proj-anywhere] A `proj` parameter is a kept parameter: the body may read
/// it and may not move it.
#[test]
fn a_proj_parameter_cannot_be_moved() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn eat(p: Person) [] -> None => !p {{}}\n\
         fn hold(p: proj Person) [] -> Int => p {{\n    eat(p)\n    return 1\n}}\n"
    ));
    // [proj-type] Caught at the type level: a projection into a consuming
    // position.
    assert!(
        errs.iter().any(|e| e.contains("`p` is a projection (`proj Person`), and `eat` consumes `p`")),
        "got: {errs:?}"
    );
}

// ---- [proj-field] [proj-infer] [proj-readonly] (user decisions 2026-09-11) ----

const VIEW_PRELUDE: &str = "struct View<T> canbe Mut {\n    items: proj List<T>,\n    at: Int\n}\n\
fn view<T>(items: List<T>) -> Mut View<T> => items {\n    return Mut View<T> { items: items, at: 0 }\n}\n\
fn advance<T>(v: Mut View<T>) -> Int => v: Mut {\n    v.at = v.at + 1\n    return v.at\n}\n";

/// [proj-field] Any struct may hold a `proj` field; it is written without a
/// source — each literal decides what it projects.
#[test]
fn a_proj_field_is_allowed_on_any_struct_and_names_no_source() {
    let errs = errors(VIEW_PRELUDE);
    assert!(errs.is_empty(), "{errs:?}");
    let errs = errors("struct Bad<T> {\n    items: proj(x) List<T>\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("a `proj` field names no source")),
        "{errs:?}"
    );
}

/// [proj-infer] A constructed view is an owned object: it may be advanced
/// (its `Mut` is real) while it keeps the source alive. Since [proj-mut]
/// (P-3's lift, user decisions 2026-09-24) a wholesale projection that
/// *carries `Mut`* — an element handle out of `List<Mut T>` — may be
/// mutated too; only the read-only projection stays read-only
/// [proj-readonly].
#[test]
fn a_held_view_may_be_advanced_and_a_mut_element_handle_may_too() {
    let src = format!(
        "{VIEW_PRELUDE}\
         fn main() {{\n    let xs = list_of(1, 2)\n    let v = view(xs)\n    advance(v)\n    advance(v)\n}}\n\
         fn list_of<T>(a: T, b: T) -> List<T> {{ return first(items)! }}\n"
    );
    let errs = errors(&src);
    // (the fake `list` body is nonsense but well-typed enough: only the
    // main body matters here)
    assert!(
        !errs.iter().any(|e| e.contains("cannot mutate `v`")),
        "advancing a held view must be allowed: {errs:?}"
    );
    // [proj-mut] The projection carries `Mut`: a mutable element handle,
    // which the `Mut` position accepts.
    let src = format!(
        "{VIEW_PRELUDE}\
         fn main(vs: List<Mut View<Int>>) {{\n    let v = first(vs)!\n    advance(v)\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        !errs.iter().any(|e| e.contains("can only be read")),
        "a Mut-carrying handle may be mutated: {errs:?}"
    );
    // A read-only projection still may not.
    let src = format!(
        "{VIEW_PRELUDE}\
         fn main(vs: List<View<Int>>) {{\n    let v = first(vs)!\n    advance(v)\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter()
            .any(|e| e.contains("can only be read") || e.contains("no matching overload")),
        "a read-only projection must still be refused: {errs:?}"
    );
}

/// [proj-readonly] A parameter written with a top-level `proj Mut` is
/// refused at the *declaration* (user decision 2026-09-12): the `proj`
/// promises to accept borrows, the `Mut` refuses every one of them. The
/// nested spelling (`List<proj Mut Int>`) stays legal — there the pair is
/// the element type a view really holds — and so does a local annotation.
#[test]
fn a_proj_mut_parameter_is_refused_at_the_declaration() {
    let errs = errors(
        "fn read(s: proj Mut Str) -> Str => s {\n    return s\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains(
            "parameter `s` cannot be written `proj Mut`"
        )),
        "{errs:?}"
    );
    // Nested `proj Mut` in a type argument is not the contradiction.
    let errs = errors(
        "fn read(xs: List<proj Mut Str>) -> Int {\n    return 0\n}\n",
    );
    assert!(
        !errs.iter().any(|e| e.contains("cannot be written `proj Mut`")),
        "{errs:?}"
    );
}

/// [proj-infer] Moving the source while a view of it lives is refused —
/// the link is inferred from `view`'s body, with nothing written.
#[test]
fn a_view_keeps_its_source_alive_through_an_inferred_lend() {
    let src = format!(
        "{VIEW_PRELUDE}\
         fn eat<T>(xs: List<T>) -> Int => !xs {{ return 0 }}\n\
         fn main(xs: List<Int>) {{\n    let v = view(xs)\n    eat(xs)\n    advance(v)\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("`v` cannot be used here")),
        "{errs:?}"
    );
}

/// [proj-infer] A written `[p: proj]` list must match the body exactly; a
/// returned view rooted in a local is refused.
#[test]
fn declared_lends_must_match_the_body_and_locals_cannot_be_lent() {
    let src = format!(
        "{VIEW_PRELUDE}\
         fn mk<T>(a: List<T>, b: List<T>) -> proj(a) in (Mut View<T>) => a, b {{\n    return Mut View<T> {{ items: b, at: 0 }}\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("says the result projects `a`, but the body returns a value that projects `b`")),
        "{errs:?}"
    );
    let src = format!(
        "{VIEW_PRELUDE}\
         fn mk<T>(a: List<T>) -> Mut View<T> => a {{\n    let local = a\n    return Mut View<T> {{ items: local, at: 0 }}\n}}\n"
    );
    // `local` aliases `a` (a parameter), so this is fine: the root is `a`.
    let errs = errors(&src);
    assert!(errs.is_empty(), "{errs:?}");
}

/// [proj-infer] `[p: proj]` alone keeps everything (it is not an
/// afterwards-qualifier), and a bodiless declaration may carry it.
#[test]
fn a_written_proj_entry_keeps_and_declares() {
    let src = format!(
        "{VIEW_PRELUDE}\
         effect Src {{\n    fn borrowed(xs: List<Int>, tag: Str) -> proj(xs) in (Mut View<Int>) => xs, tag\n}}\n\
         fn eat(xs: List<Int>) -> Int => !xs {{ return 0 }}\n\
         fn main(xs: List<Int>, tag: Str) [Src] {{\n    let v = borrowed(xs, tag)\n    advance(v)\n    eat(xs)\n    advance(v)\n}}\n"
    );
    let errs = errors(&src);
    // The first `advance` is fine (the entry keeps `xs` and lends it); the
    // second sees the view poisoned by `eat(xs)` — the link came from the
    // written `proj`, there being no body to infer it from.
    assert!(
        errs.iter().any(|e| e.contains("`v` cannot be used here")),
        "{errs:?}"
    );
}

/// [proj-mut] The mutable-element-handle discipline (P-3's lifts + P-9,
/// user decisions 2026-09-24): two *read* handles coexist; mutating through
/// one handle poisons a sibling derivation of the container; the acting
/// handle itself survives its own mutation (its storage did not move).
#[test]
fn mutable_element_handles_poison_siblings_but_not_themselves() {
    // Std-free: `at` reproduces the intrinsic `get`'s shape — a derived
    // optional return — so the handle type (`proj Mut Counter`) and links
    // are identical to `get(xs, i)!`'s.
    let prelude = "struct Counter canbe Mut {\n    n: Int\n}\n\
                   fn at<T>(list: List<T>, i: Int) -> (proj(list) T)? => list, i {\n    return None\n}\n";
    // Two read handles, then reads: fine (P-9: mode is inferred, and
    // nothing here mutates).
    let src = format!(
        "{prelude}fn f(xs: List<Mut Counter>) -> Int {{\n    let a = at(xs, 0)!\n    let b = at(xs, 1)!\n    return a.n + b.n\n}}\n"
    );
    let errs = errors(&src);
    assert!(errs.is_empty(), "two read handles must coexist: {errs:?}");
    // The handle survives its own mutation.
    let src = format!(
        "{prelude}fn f(xs: List<Mut Counter>) -> Int {{\n    let h = at(xs, 0)!\n    h.n = 5\n    return h.n\n}}\n"
    );
    let errs = errors(&src);
    assert!(errs.is_empty(), "a handle survives its own mutation: {errs:?}");
    // A sibling derivation of the container dies at the mutation.
    let src = format!(
        "{prelude}fn f(xs: List<Mut Counter>) -> Int {{\n    let a = at(xs, 0)!\n    let h = at(xs, 0)!\n    h.n = 5\n    return a.n\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("`a` cannot be used here")),
        "the sibling must be poisoned: {errs:?}"
    );
}
