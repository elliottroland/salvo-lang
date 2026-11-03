//! Assertions [assert-op] [assert-fn] [assert-narrow] (user decisions
//! 2026-09-23, ASSERTIONS.md's A-1…A-7).
//!
//! Two forms, one idea: a place where the program states a fact it cannot
//! prove, and fails if it is wrong (the postfix `!` presence assertion was
//! removed 2026-10-14; presence is `?: unreachable!(…)`). `assert!(cond)`
//! asserts a condition — and **narrows** when the condition is an `is` test,
//! which is what makes it more than a check — and `unreachable!()` asserts that
//! a path is not taken.
//!
//! The observable for the narrowing tests is overload resolution: a
//! `NonEmpty`-demanding function matches only while the checker believes the
//! claim, so "clean" means the assertion informed the type.
//!
//! The cases are test data now, `tests/check/assertions/` (TESTS.md T1).
//! One stays here until a defect is fixed (ROADMAP §0ar): the postfix-`!`
//! test is parse-only, because past the parse error the recovered `return
//! v` also reports `fn-syntax:return-mismatch`. (`assert!(n)` moved to the
//! data on 2026-10-30, when its second report went.)

/// [assert-op] There is no postfix `!`: a presence assertion is spelled out,
/// so it is not the easy thing to write (user decision 2026-10-14). The parse
/// error names both replacements.
#[test]
fn a_postfix_bang_is_a_parse_error_naming_the_alternatives() {
    let src = "fn f(v: Int?) -> Int => v {\n    return v!\n}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(src);
    let shown: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).map(|d| d.id).collect();
    assert!(shown.contains(&Some("assert-op:postfix-bang")), "{shown:?}");
}
