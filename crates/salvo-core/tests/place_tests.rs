//! Place-based flow narrowing [flow-place] [flow-place-invalidate].
//!
//! `is` narrows *places* — a variable or a field chain out of one — not
//! just variables, so a checked field reads at its narrowed type. The
//! observable used here is `[interp-no-none]`: interpolating a
//! possibly-`None` value is an error, so an accepted interpolation of
//! `p.surname` is proof the field narrowed, and a rejected one is proof
//! the fact was invalidated.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n";

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
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

struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    nick: Str? = None,
    tags: Str?[],
    pair: (Str?, Str?),
    address: Mut Address
}

// Opaque observers. Deduction lists are *written*, which is what these
// tests turn on [decl-explicit]: the declared contract is authoritative, so
// the body only has to exist.
fn touch(p: Mut Person) [] -> None => p: Mut {
    p.name = p.name
}

fn read(p: Person) [] -> None => p {}

fn touch_address(a: Mut Address) [] -> None => a: Mut {
    a.city = a.city
}
"#;

fn check(body: &str) -> Vec<String> {
    errors(&format!("{PRELUDE}\nfn probe(p: Mut Person) -> Str => p: Mut {{\n{body}\n}}\n"))
}

fn interp_errors(body: &str) -> Vec<String> {
    check(body)
        .into_iter()
        .filter(|e| e.contains("may be `None`"))
        .collect()
}

// ===== narrowing =====

/// [flow-place] A checked field reads at its narrowed type, so the
/// interpolation `[interp-no-none]` used to reject is now legal.
#[test]
fn field_narrows_in_branch() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [is-narrow-guard] [flow-place] After a guard whose branch **exits**, the
/// fall-through path carries the *else* fact: `p.surname` is not merely
/// un-narrowed there, it is known to be `None` — so the read still fails,
/// with the message for a value that has no text form rather than the one
/// for a maybe-`None` optional.
#[test]
fn a_guard_narrows_the_fall_through_path() {
    let errs = check(
        r#"
    if p.surname is Str {
        return p.name
    }
    return "${p.surname}"
"#,
    );
    assert_eq!(errs.len(), 1, "expected exactly the None read to fail: {errs:?}");
    assert!(
        errs[0].contains("cannot interpolate `None`"),
        "expected the narrowed-to-`None` message, got {errs:?}"
    );
    // The optional-interpolation error is *not* what fires any more: the
    // place is no longer optional on that path.
    assert!(
        !errs[0].contains("may be `None`"),
        "the fact holds, so the read is not a maybe-`None` one: {errs:?}"
    );
}

/// The other half of the same rule: a guard that does **not** exit leaves
/// the fall-through path un-narrowed, so the read is a maybe-`None` one.
#[test]
fn a_branch_that_falls_through_narrows_nothing_after_it() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        touch(p)
    }
    return "${p.surname}"
"#,
    );
    assert_eq!(
        errs.len(),
        1,
        "expected the maybe-`None` read after the branch to fail: {errs:?}"
    );
}

/// [flow-place] Facts are per place: checking one field says nothing about
/// its sibling.
#[test]
fn sibling_fields_are_independent() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        return "${p.nick}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the sibling read to fail: {errs:?}");
}

/// [flow-place] Field *chains* narrow, not just one level.
#[test]
fn field_chain_narrows() {
    let errs = interp_errors(
        r#"
    if p.address.city is Str {
        return "${p.address.city}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [flow-place] `&&` accumulates place facts, as it does for variables.
#[test]
fn conjunction_accumulates_place_facts() {
    let errs = interp_errors(
        r#"
    if p.surname is Str && p.nick is Str {
        return "${p.surname} ${p.nick}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [flow-place] The `else` branch of a nullable check narrows to `None`,
/// which has no text form — the negative fact reaches reads too.
#[test]
fn else_branch_carries_the_negative_fact() {
    let errs: Vec<String> = check(
        r#"
    if p.surname is Str {
        return p.name
    } else {
        return "${p.surname}"
    }
"#,
    )
    .into_iter()
    .filter(|e| e.contains("`None`"))
    .collect();
    assert_eq!(errs.len(), 1, "expected the else read to fail: {errs:?}");
    assert!(
        errs[0].contains("has no text form"),
        "expected the None-typed message: {errs:?}"
    );
}

// ===== invalidation [flow-place-invalidate] =====

/// [flow-place-invalidate] Assigning to the checked field drops the fact.
#[test]
fn assignment_to_the_place_invalidates() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        p.surname = None
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after assignment to fail: {errs:?}");
}

/// [flow-place-invalidate] Assigning to a *sibling* leaves the fact
/// standing: the write cannot reach that storage.
#[test]
fn assignment_to_a_sibling_preserves_the_fact() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        p.nick = None
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [flow-place-invalidate] Assigning to a *prefix* drops the facts below
/// it: the whole subtree is replaced.
#[test]
fn assignment_to_a_prefix_invalidates_below() {
    let errs = interp_errors(
        r#"
    if p.address.city is Str {
        p.address = Address {city: None}
        return "${p.address.city}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after the prefix write to fail: {errs:?}");
}

/// [flow-place-invalidate] A `Mut`-keeping call may mutate the value, so
/// every fact about its parts falls.
#[test]
fn mut_keeping_call_invalidates() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        touch(p)
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after the `Mut` call to fail: {errs:?}");
}

/// [flow-place-invalidate] A `Mut` call through a *projection* invalidates
/// the facts under that projection only.
#[test]
fn mut_call_through_a_projection_invalidates_that_subtree() {
    let errs = interp_errors(
        r#"
    if p.address.city is Str && p.surname is Str {
        touch_address(p.address)
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "the sibling fact should survive: {errs:?}");

    let errs = interp_errors(
        r#"
    if p.address.city is Str {
        touch_address(p.address)
        return "${p.address.city}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the mutated subtree's fact to fall: {errs:?}");
}

/// [flow-place-invalidate] A call that *keeps* the value immutably cannot
/// mutate it, so narrowing survives it (user decision P1b).
#[test]
fn kept_immutable_call_preserves_the_fact() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        read(p)
        return "${p.surname}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [flow-place] Element places do not narrow (user decision P1a): `arr[i]`
/// cannot be told from `arr[j]`, so the read stays at the declared type.
#[test]
fn array_elements_do_not_narrow() {
    let errs = interp_errors(
        r#"
    if p.tags[0] is Str {
        return "${p.tags[0]}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the element read to stay un-narrowed: {errs:?}");
}

// ===== tuple elements [expr-tuple-index] =====

/// [expr-tuple-index] [flow-place] A constant index names one location, so
/// a checked tuple element narrows exactly like a field.
#[test]
fn tuple_element_narrows() {
    let errs = interp_errors(
        r#"
    if p.pair.0 is Str {
        return "${p.pair.0}"
    }
    return p.name
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [expr-tuple-index] [flow-place] Sibling elements are independent.
#[test]
fn tuple_elements_are_independent() {
    let errs = interp_errors(
        r#"
    if p.pair.0 is Str {
        return "${p.pair.1}"
    }
    return p.name
"#,
    );
    assert_eq!(errs.len(), 1, "expected the sibling element to fail: {errs:?}");
}

/// [expr-tuple-index] The index must exist.
#[test]
fn tuple_index_out_of_range_is_an_error() {
    let errs: Vec<String> = check("    return \"${p.pair.7}\"\n")
        .into_iter()
        .filter(|e| e.contains("has no element"))
        .collect();
    assert_eq!(errs.len(), 1, "expected an out-of-range error: {errs:?}");
}

/// [expr-tuple-index] Only tuples have elements.
#[test]
fn tuple_index_on_a_non_tuple_is_an_error() {
    let errs: Vec<String> = check("    return \"${p.name.0}\"\n")
        .into_iter()
        .filter(|e| e.contains("is not a tuple"))
        .collect();
    assert_eq!(errs.len(), 1, "expected a non-tuple error: {errs:?}");
}

/// [expr-tuple-index] Tuple elements are read-only: `Mut` cannot apply to a
/// tuple, so no tuple value can grant mutation permission.
#[test]
fn tuple_elements_cannot_be_assigned() {
    let errs: Vec<String> = check("    p.pair.1 = \"x\"\n    return p.name\n")
        .into_iter()
        .filter(|e| e.contains("cannot assign to tuple element"))
        .collect();
    assert_eq!(errs.len(), 1, "expected an assignment error: {errs:?}");
}

// ===== merging =====

/// [flow-place] A fact only some fall-through paths agree on does not
/// survive the join.
#[test]
fn facts_must_hold_on_every_path_to_survive() {
    let errs = interp_errors(
        r#"
    if p.name is Str {
        if p.surname is Str {
            read(p)
        }
    }
    return "${p.surname}"
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after the join to fail: {errs:?}");
}
