//! [iter-effects] A producer's effects, spelled in qualifier position.
//!
//! D8, decided 2026-09-07: `FileSystem Iter<Str>` is a producer whose
//! *driving* performs `FileSystem`. The spelling is a qualifier's — a type
//! with no arrow has nowhere to put a `[…]` list, and a prefixed one would
//! read as a deduction list in return position — but the rules are the ones
//! [fn-effects] already gave function values: the type carries the effects,
//! whoever invokes the value inherits them, and a value performing *fewer*
//! effects fits where more are expected.
//!
//! What is under test here is the surface, not the emission: threading the
//! handlers into a generated pass is I4's second half, and until it lands both
//! backends refuse an effectful producer outright.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n\
     intrinsic type Iter<T>\nintrinsic fn copy<T>(value: T) [] -> [value] T\n";

const PRELUDE: &str = r#"
effect Logger {
    fn log(message: Str) -> [message] None
}

handler QuietLogger of Logger {
    fn log(message: Str) -> [message] None {}
}

effect Counter {
    fn bump() -> [] None
}

handler NoCounter of Counter {
    fn bump() -> [] None {}
}

qualifier NonEmpty of Int

struct Countdown canbe Mut, Once {
    at: Int
}
"#;

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
        format!("{PRELUDE}{src}"),
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
        .map(|d| d.message.clone())
        .collect()
}

/// The checked program, for the side tables the emitters read.
fn checked_of(src: &str) -> (salvo_core::Checked, Program) {
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
        format!("{PRELUDE}{src}"),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, _) = salvo_syntax::parse_module(&file.content);
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
    (checked, program)
}

// ===== the table the emitters read =====

/// [iter-effects] A producer's claim is recorded per fn, **sorted**, and the
/// program's distinct effect *sets* are recorded alongside it: one generated
/// trait (interface) per set, the way `union_sizes` drives one `UnionN` per
/// arity. Sorted and not written-order, because the two spellings of one set
/// are the same type — so two producers written the two ways have to agree on
/// their machine's parameter order.
#[test]
fn a_producers_claim_is_recorded_in_canonical_order() {
    let (checked, _program) = checked_of(
        "fn one(n: Int) -> Counter Logger Iter<Int> {
    bump()
    log(\"x\")
    yield n
}

fn two(n: Int) -> Logger Counter Iter<Int> {
    bump()
    log(\"y\")
    yield n
}

fn pure_one(n: Int) -> Iter<Int> {
    yield n
}
",
    );
    let mut claims: Vec<Vec<String>> = checked
        .producer_effects
        .values()
        .map(|tys| tys.iter().map(|t| t.to_string()).collect())
        .collect();
    claims.sort();
    assert_eq!(
        claims,
        vec![
            vec!["Counter".to_string(), "Logger".to_string()],
            vec!["Counter".to_string(), "Logger".to_string()],
        ],
        "both producers claim the same set, in the same order"
    );
    // The effect-free producer has no entry, and one set means one trait.
    assert_eq!(checked.producer_effects.len(), 2);
    let sets: Vec<Vec<String>> = checked.pass_effect_sets.iter().cloned().collect();
    assert_eq!(sets, vec![vec!["Counter".to_string(), "Logger".to_string()]]);
}

// ===== where the claim may be written (D8 position rule 2) =====

/// `Iter<T>` is the built-in position: driving it is what performs the effect.
#[test]
fn a_claim_on_iter_is_accepted() {
    assert!(
        errors(
            "fn counted(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}
"
        )
        .is_empty(),
        "a claim on `Iter<T>` should be accepted"
    );
}

/// And a type of one's own reaches it by opting in — the same `canbe Once`
/// that makes it a pass, which is what position rule 2 shares with `Once`.
#[test]
fn a_claim_on_a_canbe_once_type_is_accepted() {
    let errs = errors(
        "fn drain(c: Logger Countdown) -> None {
    return None
}
",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// A type that is neither is refused, and the diagnostic names both positions.
#[test]
fn a_claim_on_an_ordinary_type_is_refused() {
    let errs = errors(
        "fn f(n: Logger Int) -> None {
    return None
}
",
    );
    assert!(
        errs.iter().any(|e| {
            e.contains("claims what *driving* a producer performs")
                && e.contains("canbe Once")
        }),
        "expected the position error, got: {errs:?}"
    );
}

/// A fn type is excluded on purpose: it has the bracket spelling already, and
/// two ways to say one thing is how they drift.
#[test]
fn a_claim_on_a_fn_type_names_the_bracket_form() {
    let errs = errors(
        "fn f(g: Logger (Int) -> Int) -> None {
    return None
}
",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("declares its effects in its own list")),
        "expected the fn-type redirection, got: {errs:?}"
    );
}

// ===== the rules the claim inherits from [fn-effects] =====

/// Variance: a producer performing *fewer* effects fits a position expecting
/// more — the caller supplies a handler the producer simply never uses.
#[test]
fn fewer_effects_fit_where_more_are_expected() {
    let errs = errors(
        "fn pure_producer(n: Int) -> Iter<Int> {
    yield n
}

fn drive(xs: Logger Iter<Int>) -> None {
    for v in xs {
        log(\"got\")
    }
    return None
}

fn main() [use] -> None {
    use QuietLogger()
    drive(pure_producer(1))
}
",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// And never the reverse: a producer that performs `Logger` may not be driven
/// where no handler can be supplied.
#[test]
fn more_effects_do_not_fit_where_fewer_are_expected() {
    let errs = errors(
        "fn noisy(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}

fn drive(xs: Iter<Int>) -> None {
    for v in xs {
        return None
    }
    return None
}

fn main() [use] -> None {
    use QuietLogger()
    drive(noisy(1))
}
",
    );
    assert!(
        errs.iter().any(|e| e.contains("Iter<Int>")),
        "expected the argument-type rejection, got: {errs:?}"
    );
}

/// The claim never drops [qual-widen]: `^` would make an effectful producer
/// look pure, which is the one direction that reaches a driver with no handler.
#[test]
fn a_claim_cannot_be_widened_away() {
    let errs = errors(
        "fn noisy(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}

fn main() [use] -> None {
    use QuietLogger()
    let xs = noisy(1)
    if xs ^ Logger {
        return None
    }
    return None
}
",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("restricts rather than refines")),
        "expected the never-drop error, got: {errs:?}"
    );
}

/// Inheritance: a fn taking a producer needs no list of its own — the only
/// reason to take it is to drive it — and its callers supply the handler.
#[test]
fn a_fn_inherits_a_producer_parameters_effects() {
    let errs = errors(
        "fn drive(xs: Logger Iter<Int>) -> None {
    for v in xs {
        log(\"got\")
    }
    return None
}
",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// The caller of such a fn must have the handler, exactly as for an inherited
/// fn-typed parameter's effects.
#[test]
fn a_caller_must_supply_an_inherited_producer_effect() {
    let errs = errors(
        "fn noisy(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}

fn drive(xs: Logger Iter<Int>) -> None {
    for v in xs {
        return None
    }
    return None
}

fn nowhere(n: Int) -> None {
    drive(noisy(n))
    return None
}
",
    );
    assert!(
        errs.iter().any(|e| e.contains("Logger")),
        "expected the missing-handler error at the call, got: {errs:?}"
    );
}

// ===== driving is what needs the handler =====

/// A `for` over a claiming producer performs its effects, so the loop's own
/// function must have them — the same check a fn value's *call* makes.
#[test]
fn driving_a_producer_needs_its_claimed_effects() {
    let errs = errors(
        "fn noisy(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}

fn consume(n: Int) -> None {
    for v in noisy(n) {
        return None
    }
    return None
}
",
    );
    assert!(
        errs.iter().any(|e| e.contains("Logger")),
        "expected the missing-handler error at the `for`, got: {errs:?}"
    );
}

/// With the handler in scope it is accepted, and *holding* the producer never
/// needed one — only driving it does [fn-effects].
#[test]
fn holding_a_producer_needs_nothing() {
    let errs = errors(
        "fn noisy(n: Int) -> Logger Iter<Int> {
    log(\"one\")
    yield n
}

fn hold(n: Int) -> Logger Iter<Int> {
    return noisy(n)
}

fn consume(n: Int) [Logger] -> None {
    for v in noisy(n) {
        return None
    }
    return None
}
",
    );
    assert!(errs.is_empty(), "{errs:?}");
}
