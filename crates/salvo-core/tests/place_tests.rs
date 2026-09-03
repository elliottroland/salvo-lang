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

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
    let (ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let program = Program {
        files: sources.files,
        modules: vec![ast],
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
internal type Int
internal type Str
internal type Bool

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

external fn touch(p: Mut Person) [] -> [p: Mut] None
external fn read(p: Person) [] -> [p] None
external fn touch_address(a: Mut Address) [] -> [a: Mut] None
"#;

fn check(body: &str) -> Vec<String> {
    errors(&format!("{PRELUDE}\nfn probe(p: Mut Person) -> [p: Mut] Str {{\n{body}\n}}\n"))
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

/// [flow-place] Outside the branch the fact does not hold.
#[test]
fn field_is_not_narrowed_outside_the_branch() {
    let errs = interp_errors(
        r#"
    if p.surname is Str {
        return p.name
    }
    return "${p.surname}"
"#,
    );
    assert_eq!(errs.len(), 1, "expected the read after the branch to fail: {errs:?}");
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
