//! [mod-export] Parsing the `export` modifier: where it may appear, what it
//! sets, that it stays an ordinary identifier everywhere else, and the
//! deduction-list interaction it exposed.

use salvo_syntax::ast::{DeductionKind, Item};

// ===================== [mod-export] the `export` modifier =====================

/// [mod-export] The modifier sets the flag, and only on the declaration it
/// precedes — the rest of the file stays private.
#[test]
fn export_marks_the_declaration_it_precedes() {
    let src = "\
export fn shown() [] -> Int {
    return 1
}

fn hidden() [] -> Int {
    return 2
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {diagnostics:?}"
    );
    let flags: Vec<(String, bool)> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fn(f) => Some((f.name.name.clone(), f.exported)),
            _ => None,
        })
        .collect();
    assert_eq!(
        flags,
        vec![("shown".to_string(), true), ("hidden".to_string(), false)]
    );
}

/// [mod-export] Contextual, not reserved: `export` is still an ordinary name
/// for a variable, a parameter or a field. Only the start of a top-level
/// declaration makes it a modifier.
#[test]
fn export_stays_an_ordinary_name_elsewhere() {
    let src = "\
struct Report {
    export: Bool
}

fn f(export: Int) [] -> Int {
    let export2 = export + 1
    return export2
}
";
    let (_module, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "`export` must stay a usable name: {diagnostics:?}"
    );
}

/// [mod-export] [deduce-syntax] The regression this modifier exposed: a
/// declaration whose last thing is a deduction **qualifier list** has nothing
/// after it to stop at, so the parser's "while the next token is an identifier"
/// loop swallowed the *next* declaration's modifier as a qualifier. The four
/// contextual modifiers are now recognized by shape there.
///
/// It was latent for `iter`, `send` and `actor` before `export` existed; all
/// four are covered here.
#[test]
fn a_trailing_deduction_list_does_not_swallow_the_next_modifier() {
    let src = "\
intrinsic fn clear(data: Mut Bytes) [] -> None => data: Mut

export intrinsic fn size(data: Bytes) [] -> Int => data

intrinsic fn touch(data: Mut Bytes) [] -> None => data: Mut

iter fn next(c: Countdown) [] -> Emitted Int | Finished {
    return finished()
}

intrinsic fn poke(data: Mut Bytes) [] -> None => data: Mut

send fn later(out: Reply<Int>) => !out {
}

intrinsic fn prod(data: Mut Bytes) [] -> None => data: Mut

actor effect Mailer {
    send fn mail(m: Str) => !m
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {diagnostics:?}"
    );
    // Every declaration survived as its own item — the five intrinsics, the
    // `send fn`, the `actor effect`, and the `iter fn`, which expands into its
    // pass struct plus `iter` and `next` [iter-fn].
    let fns: Vec<&str> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fn(f) => Some(f.name.name.as_str()),
            _ => None,
        })
        .collect();
    for want in ["clear", "size", "touch", "poke", "later", "prod", "next", "iter"] {
        assert!(fns.contains(&want), "`{want}` was lost: {fns:?}");
    }
    assert!(
        module.items.iter().any(|i| matches!(i, Item::Effect(e) if e.name.name == "Mailer")),
        "the `actor effect` was lost"
    );
    for item in &module.items {
        if let Item::Fn(f) = item {
            for d in f.deductions.iter().flatten() {
                if let DeductionKind::Exhaustive { quals, .. } = &d.kind {
                    let names: Vec<&str> = quals.iter().map(|q| q.name.name.as_str()).collect();
                    assert_eq!(names, vec!["Mut"], "in `{}`", f.name.name);
                }
            }
        }
    }
}

/// [mod-export] The three items that are not declarations each say why.
#[test]
fn export_is_refused_on_non_declarations() {
    for (src, want) in [
        ("export import core.Str\n", "there is no re-export"),
        (
            "qualifier NonEmpty of Int\nexport refn add(x: Mut Int) => x: +NonEmpty\n",
            "travels with the qualifier",
        ),
        (
            "fn add(a: Int) [] -> Int {\n    return a\n}\nexport rename fn add2 = add(a: Int)\n",
            "not a declaration",
        ),
    ] {
        let (_m, diagnostics) = salvo_syntax::parse_module(src);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.is_error() && d.message.contains(want)),
            "expected `{want}` for `{src}`, got {diagnostics:?}"
        );
    }
}
