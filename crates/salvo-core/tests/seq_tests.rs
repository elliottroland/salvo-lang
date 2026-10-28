//! [seq-iterator] [fn-overload-rank] Which `map` a call goes to: std's `List`
//! fast path or the generic body over any pass. The diagnostics are test
//! data, `tests/check/seq/` (TESTS.md T1); these read the checker's
//! `call_fn` table, which annotations cannot.

/// The display of every declaration a call to `callee` in the user's file
/// went to, over the real std, asserting the program reports no error.
fn callees(src: &str, callee: &str) -> Vec<String> {
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let mut found: Vec<String> = out
        .checked
        .call_fn
        .iter()
        .filter(|(k, _)| k.file() >= user)
        .map(|(_, d)| d.to_string())
        .filter(|d| d.starts_with(&format!("{callee}(")))
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no call to `{callee}` was resolved");
    found
}

/// Whether every `map` call resolved to std's `List` fast path (`map(List<T>,
/// …)`) rather than the generic body (`map(Mut It, …)`).
fn picked_fast_path(src: &str) -> bool {
    let found = callees(src, "map");
    let fast = found.iter().filter(|d| d.starts_with("map(List<")).count();
    assert!(fast == 0 || fast == found.len(), "mixed picks: {found:?}");
    fast == found.len()
}

fn probe(body: &str) -> String {
    format!("fn probe() -> None {{\n{body}\n}}\n")
}

/// [fn-overload-rank] With a `List` subject the fast path wins: `List<T>` is
/// more specific than a bare `It`. It is also what keeps the short spelling
/// for the type people map most.
#[test]
fn a_list_subject_picks_the_fast_path() {
    let src = probe("    let xs = List(1, 2)\n    let _ys: Mut List<Int> = map(xs, n -> n * 2)");
    assert!(picked_fast_path(&src), "expected the `List` overload: {:?}", callees(&src, "map"));
}

/// And with a pass subject the generic body takes it — the fast path is not
/// viable, so specificity never gets to prefer it.
#[test]
fn a_pass_subject_picks_the_generic_body() {
    let src = probe("    let xs = List(1, 2)\n    let _ys: Mut List<Int> = map(iter(xs), n -> n + 1)");
    assert!(!picked_fast_path(&src), "expected the generic overload: {:?}", callees(&src, "map"));
}

/// [fn-overload-rank] The lead candidate is re-narrowed *per argument*: the
/// `List` candidate leads on specificity, and typing the subject has to drop
/// it before the lambda is checked against `List<T>`'s element type. (That
/// the call checks clean is `seq/seq.sv`'s `a_str_pass_iterates_characters`.)
#[test]
fn the_lead_narrows_before_the_lambda_is_typed() {
    let src = probe("    let _ys: Mut List<Bool> = map(iter(\"ab\"), c -> c == 'a')");
    assert!(!picked_fast_path(&src), "{:?}", callees(&src, "map"));
}
