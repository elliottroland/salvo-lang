//! [type-literal] [type-union-arm-identity] [type-literal-qualified] The
//! runtime tables a backend reads for unions of literals: the value tests `is`
//! lowers to and the arm a wrap goes into. The diagnostics are test data,
//! `tests/check/literals/` (TESTS.md T1); these read the checker's tables,
//! which annotations cannot.

use salvo_core::Coercion;
use salvo_syntax::ast::TypeLit;

/// `src` checked over the real std, asserting it reports no error, with
/// only the rows of its own file kept (std's are seeded into every check).
fn checked(src: &str) -> salvo_core::Checked {
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let mut c = out.checked;
    c.is_tests.retain(|k, _| k.file() >= user);
    c.coerce.retain(|k, _| k.file() >= user);
    c
}

const CLASS: &str = "type Class = \"STANDARD\" | \"GLACIER\" | Other Str\n";

#[test]
fn the_backend_tables_are_the_runtime_arms() {
    // `is "A"` on an open union is a value test on a plain string (no
    // wrapper); in a mixed union the two `Str` literals share runtime arm 0.
    let src = format!(
        "{CLASS}struct P {{ n: Int }}
type Mixed = \"name\" | 3 | \"other\" | P
fn f(c: Class) [] -> Bool => c {{
    return c is \"GLACIER\"
}}
fn g() [] -> Mixed {{
    return \"other\"
}}
"
    );
    let out = checked(&src);
    let test = out
        .is_tests
        .values()
        .find(|t| !t.values.is_empty())
        .expect("a literal test");
    assert_eq!(test.size, 1);
    assert_eq!(test.values, vec![(0, false, vec![TypeLit::Str("GLACIER".into())])]);
    let wrap = out
        .coerce
        .values()
        .find_map(|c| match c {
            Coercion::WrapUnion { target, arm, .. } => Some((target.clone(), *arm)),
            _ => None,
        })
        .expect("a wrap into the mixed union");
    assert_eq!(wrap.1, 0, "\"other\" shares the first Str arm");
    assert_eq!(wrap.0.to_string(), "Str | Int | P");
}

// ===== [type-literal-qualified] qualified literals =====

const SHORT: &str = "export provenance qualifier Short of Str\ntype Port = \"port\"\n\
type Field = Short Port | Short \"verbose\" | \"host\"\n";

/// [type-literal-qualified] `is Short` on a union of qualified literals is a
/// value test against the literals that carry `Short`, on the one runtime
/// arm they all collapsed into (no wrapper); a `when` over the arms is
/// exhaustive as over any; and the bare literal wraps into nothing.
#[test]
fn a_qualified_literal_test_is_a_value_comparison() {
    let src = format!(
        "{SHORT}fn f(x: Field) [] -> Bool => x {{\n    return x is Short\n}}\n\
         fn g(x: Field) [] -> Int => x {{\n    when x {{\n        is Short {{ return 1 }}\n        is \"host\" {{ return 2 }}\n    }}\n}}\n\
         fn h() [] -> Field {{\n    return \"host\"\n}}\n"
    );
    let out = checked(&src);
    let want = vec![(0, false, vec![TypeLit::Str("port".into()), TypeLit::Str("verbose".into())])];
    assert!(
        out.is_tests.values().any(|t| t.size == 1 && t.values == want),
        "{:?}",
        out.is_tests.values().map(|t| (t.size, t.values.clone())).collect::<Vec<_>>()
    );
    assert!(
        out.is_tests.values().any(|t| t.size == 1 && t.values == vec![(0, false, vec![TypeLit::Str("host".into())])]),
        "the `when`'s `is \"host\"` arm is a value test too"
    );
    assert!(
        !out.coerce.values().any(|c| matches!(c, Coercion::WrapUnion { .. })),
        "a union that collapsed to `Str` has no arm to wrap into: {:?}",
        out.coerce.values().collect::<Vec<_>>()
    );
}
