//! [group-obligation] [group-self] [group-not-a-value] Obligation groups as
//! a mechanism (roadmap R1, user decisions 2026-09-08): `: Group<Args>` on a
//! struct declaration promises the group's members, checked at the *struct*;
//! `Self` inside a `params` group stands for the declaring type; and no value
//! may ever have a group as its type — the single restriction that keeps the
//! mechanism a where-clause rather than a trait.
//!
//! Nothing is designated here: the groups under test are declared in the
//! tests, which is the point — R1 is the mechanism, `Yield<T>`/`Linear` get
//! designated in R2/R4.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as a *std* file since only std may write `intrinsic`.
const STD_PRELUDE: &str =
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic type List<T> canbe Mut\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export intrinsic fn get<T>(list: List<T>, index: Int) [] -> T? => list, index\n\
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T>(value: T) [] -> +Emitted T => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n";

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

/// A group shaped like the iteration protocol, declared in the test rather
/// than designated by the compiler. The state is an ordinary parameter and
/// comes first; a declaring type writes itself as `self` at the obligation
/// [group-self].
const PROTO: &str = r#"
params Step<It, T> {
    fn advance(it: Mut It) -> Emitted T | Finished => it: Mut
}
"#;

// --- satisfaction [group-obligation] ----------------------------------------

#[test]
fn a_satisfied_obligation_is_clean() {
    let errs = errors(&format!(
        "{PROTO}
struct Countdown : Step<self, Int> canbe Mut {{
    at: Int
}}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {{
    if c.at <= 0 {{
        return finished()
    }}
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}}
"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The declaration is the error site, and the message names the missing
/// signature with `Self` substituted.
#[test]
fn a_missing_member_errors_at_the_struct() {
    let errs = errors(&format!(
        "{PROTO}
struct Countdown : Step<self, Int> canbe Mut {{
    at: Int
}}
"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("`Countdown` declares `: Step`")
            && errs[0].contains("fn advance(Mut Countdown) -> Emitted Int | Finished"),
        "expected the missing signature named, got {errs:?}"
    );
}

/// Right name, wrong shape — the state is not `Mut` — is still missing.
#[test]
fn a_wrong_signature_does_not_satisfy() {
    let errs = errors(&format!(
        "{PROTO}
struct Countdown : Step<self, Int> canbe Mut {{
    at: Int
}}

fn advance(c: Countdown) -> Emitted Int | Finished => c {{
    return finished()
}}
"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("no visible `advance` matches"),
        "got {errs:?}"
    );
}

/// A generic struct satisfies a group through its own type parameters: the
/// candidate's variables are matched against the struct's by renaming.
#[test]
fn a_generic_struct_satisfies_through_its_generics() {
    let errs = errors(&format!(
        "{PROTO}
struct Zip<A, B> : Step<self, (A, B)> canbe Mut {{
    left: List<A>,
    right: List<B>,
    at: Int
}}

fn advance<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut {{
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is A && r is B {{
        z.at = z.at + 1
        return emitted((l, r))
    }}
    return finished()
}}
"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Variable matching is bijective: an `advance` over `Twin<A, A>` does not
/// satisfy an obligation whose expected signature is over `Twin<A, B>`.
#[test]
fn variable_matching_is_bijective() {
    let errs = errors(&format!(
        "{PROTO}
struct Twin<A, B> : Step<self, (A, B)> canbe Mut {{
    xs: List<A>
}}

fn advance<A>(t: Mut Twin<A, A>) -> Emitted (A, A) | Finished => t: Mut {{
    return finished()
}}
"
    ));
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("no visible `advance` matches"),
        "got {errs:?}"
    );
}

/// Parameter *names* are the group's own; an implementation picks its own.
/// (The satisfied tests above already use `c`/`z` against the group's `s` —
/// this pins that a same-named parameter is not *required* by using a
/// different name for every parameter of a two-parameter member.)
#[test]
fn parameter_names_are_not_part_of_matching() {
    let errs = errors(
        "params Pair<S, T> {
    fn combine(a: S, b: S) -> T
}

struct Point : Pair<self, Int> {
    x: Int
}

fn combine(first: Point, second: Point) -> Int {
    return first.x + second.x
}
",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- the clause itself -------------------------------------------------------

#[test]
fn an_unknown_group_errors_with_position_hints() {
    let errs = errors(
        "struct A : Missing {
    x: Int
}

struct B : Int {
    x: Int
}
",
    );
    assert_eq!(errs.len(), 2, "got {errs:?}");
    assert!(errs[0].contains("unknown `params` group `Missing`"), "got {errs:?}");
    assert!(
        errs[1].contains("`Int` is a type; an obligation names a `params` group"),
        "got {errs:?}"
    );
}

#[test]
fn arity_and_duplicates_are_declaration_errors() {
    let errs = errors(&format!(
        "{PROTO}
struct A : Step<self, Int, Str> canbe Mut {{
    x: Int
}}

struct B : Step<self, Int>, Step<self, Int> canbe Mut {{
    x: Int
}}

fn advance(b: Mut B) -> Emitted Int | Finished => b: Mut {{
    return finished()
}}
"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Step` takes 2 type argument(s), found 3")),
        "got {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("obligation `Step` is declared more than once")),
        "got {errs:?}"
    );
    // A missing-member error for the arity-failed struct would be noise: the
    // obligation could not even be read. One error for A, one for B's dup.
    assert_eq!(errs.len(), 2, "got {errs:?}");
}

// --- `self` at the obligation [group-self] -----------------------------------

/// The point of writing the declaring type as an **argument** rather than as a
/// magic `Self` inside the group (user decision 2026-09-08): the group stays
/// ordinary, so *one* declaration serves both the obligation and the `?`
/// spread. This is what composition needs — a generic combinator reaches its
/// source's `next` through an implicit.
#[test]
fn the_same_group_serves_an_obligation_and_a_spread() {
    let errs = errors(&format!(
        "{PROTO}
struct Countdown : Step<self, Int> canbe Mut {{
    at: Int
}}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {{
    return finished()
}}

fn drive<It>(source: Mut It, ?Step<It, Int>) -> Int => source: Mut {{
    let step = advance(source)
    return 0
}}
"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [implicit-group] [fn-contract] A spread member's **deduction list is part
/// of the position**: `fn advance(it: Mut It) -> …` is filled by an => it: Mut
/// implementation that mutates its parameter, which is the whole point of a
/// pass. The member's fn type used to be built without its contract, so
/// every parameter read as kept-and-immutable and no real `advance` could
/// fill the position ("`advance` mutates `it`, which this position does not
/// permit") — found when R5's combinator surface was probed.
#[test]
fn a_mutating_member_fills_a_mut_spread_position() {
    let errs = errors(&format!(
        "{PROTO}
struct Countdown : Step<self, Int> canbe Mut {{
    at: Int
}}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {{
    if c.at <= 0 {{
        return finished()
    }}
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}}

fn drain<It>(source: Mut It, ?Step<It, Int>) -> Int => source: Mut {{
    let n = 0
    let going = true
    while going {{
        let step = advance(source)
        when step {{
            is Emitted {{
                n = n + 1
            }}
            is Finished {{
                going = false
            }}
        }}
    }}
    return n
}}

fn go() -> Int {{
    return drain(Mut Countdown {{ at: 3 }})
}}
"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// `self` is the *obligation's* shorthand and nothing else: in a spread it has
/// no declaration to refer to.
#[test]
fn self_is_not_a_type_outside_an_obligation() {
    let errs = errors(&format!(
        "{PROTO}
fn drive<It>(source: Mut It, ?Step<self, Int>) -> Int {{
    return 0
}}
"
    ));
    assert!(
        errs.iter().any(|e| e.contains("unknown type `self`")),
        "got {errs:?}"
    );
}

/// A group with no obligation involved still spreads exactly as before — the
/// mechanism is additive.
#[test]
fn a_selfless_group_still_spreads() {
    let errs = errors(
        "params Field<T> {
    fn add(a: T, b: T) -> T
}

fn total(x: Int, y: Int, ?Field<Int>) -> Int {
    return add(x, y)
}

fn add(a: Int, b: Int) -> Int {
    return a + b
}
",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- no value has a group type [group-not-a-value] ---------------------------

/// The rule that keeps the mechanism from being a trait: a group name is
/// refused in *every* value-type position, with one diagnostic (the
/// unknown-type error is suppressed, not doubled).
#[test]
fn a_group_is_not_a_type() {
    let cases = [
        ("fn f(p: Pair<Int>) -> Int {\n    return 0\n}", "parameter"),
        ("fn f() -> Pair<Int> {\n    return 0\n}", "return"),
        (
            "fn f() -> Int {\n    let x: Pair<Int> = 0\n    return 0\n}",
            "let annotation",
        ),
        ("struct S {\n    p: Pair<Int>\n}", "field"),
        ("fn f(xs: List<Pair<Int>>) -> Int {\n    return 0\n}", "type argument"),
        ("fn f(u: Pair<Int> | Str) -> Int {\n    return 0\n}", "union arm"),
    ];
    for (src, place) in cases {
        let errs = errors(&format!(
            "params Pair<T> {{\n    fn combine(a: T, b: T) -> T\n}}\n\n{src}\n"
        ));
        assert_eq!(errs.len(), 1, "{place}: got {errs:?}");
        assert!(
            errs[0].contains("`Pair` is a `params` group, not a type"),
            "{place}: got {errs:?}"
        );
    }
}
