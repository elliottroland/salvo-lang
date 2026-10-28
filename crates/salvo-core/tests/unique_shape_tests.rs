//! [op-unique-shape] A function of a unique shape on a type is declared in the
//! type's own file (user decision O1-D, 2026-10-27). The cases are data,
//! `tests/check/unique_shape/` (TESTS.md T1); what is left is a fact about
//! std itself, read off a resolution of std alone rather than off a program
//! a data file builds (whose check may take std's rows from the front's
//! cache).

/// std itself follows the rule: every shape it declares is in its type's
/// file (D-7's adoption, checked rather than assumed).
#[test]
fn std_declares_every_shape_in_its_types_file() {
    // [std-cache] std alone, over the process's std front (TESTS.md T3).
    let program = salvo_testkit::front::clean_program("rs", &[]);
    let resolution = salvo_core::resolve(&program);
    let refusals: Vec<String> = resolution
        .errors
        .iter()
        .filter(|d| d.id.is_some_and(|id| salvo_syntax::messages::label_of(id) == "op-unique-shape"))
        .map(|d| d.render(&program.files))
        .collect();
    assert!(refusals.is_empty(), "{refusals:#?}");
}
