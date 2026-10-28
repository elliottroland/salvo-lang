//! [obligation-by] [fn-by] [comptime-instantiate] **A stamped member has the
//! signature it is declared with, never the template's** (user decision
//! 2026-10-14, ROADMAP §2c): the written one for `fn … by X`, the group
//! member's with `self` substituted for an obligation clause — which is one
//! `fn … by X` per member — and the template supplies only the body, checked
//! against that signature. Its parameter names, effect list and deductions
//! stop counting.
//!
//! Over the real standard library (`lib/`), so the stamps are `std.core.auto`'s.

use salvo_core::Program;

fn program(files: &[(&str, &str)]) -> (Program, Vec<String>) {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let built = salvo_testkit::front::program("kt", files);
    let errors = built
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| {
            let file = &built.program.files[d.file];
            let line = file.content[..(d.span.start as usize).min(file.content.len())].matches('\n').count() + 1;
            if file.name == "main.sv" { format!("{line}: {}", d.message) } else { format!("{}:{line}: {}", file.name, d.message) }
        })
        .collect();
    (built.program, errors)
}

/// Every error the expansion, resolver and checker report, as `line: message`
/// for the user file `main.sv` (other files' lines as `file:line`).
fn errors_in(files: &[(&str, &str)]) -> Vec<String> {
    let (program, mut errors) = program(files);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    // [ir-analyze] Ownership errors are the IR's, as a build lists them.
    let flow = salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked);
    for d in resolution.errors.iter().chain(checked.errors.iter()).chain(flow.iter()).filter(|d| d.is_error()) {
        let file = &program.files[d.file];
        let line = file.content[..(d.span.start as usize).min(file.content.len())].matches('\n').count() + 1;
        errors.push(if file.name == "main.sv" { format!("{line}: {}", d.message) } else { format!("{}:{line}: {}", file.name, d.message) });
    }
    errors
}

fn errors(src: &str) -> Vec<String> {
    errors_in(&[("main.sv", src)])
}

/// The IR of the user modules, as `salvo ir` renders it.
fn ir(files: &[(&str, &str)]) -> String {
    let (program, errors) = program(files);
    assert!(errors.is_empty(), "{errors:?}");
    let (text, errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(errors.is_empty(), "{errors:?}");
    text
}

/// A derive module whose template names its parameters `v` and `between`,
/// where the group's member says `value` and `sep` — with a default of its
/// own, so that a call may name it [fn-param-default].
const LABEL: &str = r#"
export fns Labelled<T> {
    fn label(value: T, sep: Str = " ") -> Str
}

export comptime fn label<T is Struct>(v: T, between: Str = ",") [] -> Str => v {
    let out: Mut Str = Mut Str("${T.name}")
    [for field in T.fields] {
        append(out, between)
        append(out, "${v.[field]}")
    }
    return out
}
"#;

/// [obligation-by] The clause's member takes the **group's** parameter names:
/// a named argument says `sep`, the group's, and the template's `between` is
/// no name of the stamped fn — while the body, which reads `v` and
/// `between`, reads the declared parameters.
#[test]
fn a_clause_member_takes_the_groups_parameter_names() {
    let main = r#"
import derive

struct P : Labelled<self> by derive { x: Int, y: Str }

fn probe() [] -> Str {
    return label(P { x: 1, y: "a" }, sep = "-")
}

fn plain() [] -> Str {
    return label(P { x: 1, y: "a" })
}
"#;
    let errs = errors_in(&[("derive.sv", LABEL), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
    let dump = ir(&[("derive.sv", LABEL), ("main.sv", main)]);
    assert!(dump.contains("fn main::label(lent value: P, lent sep: Str)"), "{dump}");
    assert!(dump.contains("value.x") && !dump.contains("between"), "{dump}");
    // The declared default, not the template's.
    assert!(dump.contains(r#"y: "a" }, " ")"#), "{dump}");

    let by_template_name = main.replace("sep = ", "between = ");
    let errs = errors_in(&[("derive.sv", LABEL), ("main.sv", &by_template_name)]);
    assert!(errs.iter().any(|e| e.contains("`between`")), "{errs:?}");
}

/// [fn-by] A written declaration keeps its own parameter names and its own
/// deduction clause. `std.core.auto`'s `hash` names its parameter `value` and
/// declares a local `h`: the declared `h` is the parameter, and the template's
/// local is renamed apart (`h__p`) so it cannot capture it — nor be refused as
/// shadowing it [var-no-shadow].
#[test]
fn a_written_declaration_keeps_its_names_and_clause() {
    let src = r#"
struct P : Hashed<self> {
    x: Int,
    y: Str

    fn hash(h: P) -> Long by auto
    fn eq(left: P, right: P) -> Bool => left, right by auto
}

fn probe(p: P, q: P) [] -> Long {
    if eq(p, q) {
        return hash(p)
    }
    return 0L
}
"#;
    let errs = errors(src);
    assert!(errs.is_empty(), "{errs:?}");
    let dump = ir(&[("main.sv", src)]);
    assert!(dump.contains("fn main::hash(lent h: P)") && dump.contains("h.x") && dump.contains("h__p"), "{dump}");
    assert!(dump.contains("fn main::eq(lent left: P, lent right: P)") && dump.contains("left.x"), "{dump}");
}

/// [fn-by] [deduce-default] The declared contract governs, not the
/// template's: `std.core.auto`'s `eq` keeps both parameters (`=> a, b`), but a
/// declaration that consumes its first (`=> !a`) consumes the caller's
/// argument.
#[test]
fn the_declared_deductions_govern_the_callers() {
    let src = r#"
struct P : Eq<self> {
    x: Int,
    y: Str

    fn eq(a: P, b: P) -> Bool => !a by auto
}

fn probe(p: P, q: P) [] -> Bool => !p {
    let same = eq(p, q)
    return eq(p, q)
}
"#;
    let errs = errors(src);
    assert!(errs.iter().any(|e| e.starts_with("11:") && e.contains("`p`") && e.contains("moved")), "{errs:?}");
    // Kept by the default, as the template's clause also says: no move.
    let errs = errors(&src.replace(" => !a by auto", " by auto"));
    assert!(errs.is_empty(), "{errs:?}");
}

/// A derive whose template declares what its group's member does not: an
/// effect (`Console`), and a move of its parameter (`=> !v`).
const LOUD: &str = r#"
export fns Loud<T> {
    fn loud(v: T) -> Int
}

export fns Taken<T> {
    fn take(v: T) -> T
}

export comptime fn loud<T is Struct>(v: T) [Console] -> Int => v {
    print("${T.name}")
    return 0
}

export comptime fn take<T is Struct>(v: T) [] -> T => !v {
    return v
}
"#;

/// [obligation-by] [comptime-instantiate] A template's effect list and
/// deductions do not count: the group's member is pure and keeps its
/// parameter, so a body that prints, or moves the parameter, is an error of
/// the stamped body — at the `by` site, with the stamp's prefix.
#[test]
fn a_template_contract_the_member_lacks_is_an_error_at_the_stamp() {
    let main = r#"
import derive

struct P : Loud<self> by derive, Taken<self> by derive { x: Int, y: Str }
"#;
    let errs = errors_in(&[("derive.sv", LOUD), ("main.sv", main)]);
    assert!(
        errs.iter().any(|e| e.starts_with("4: in `loud` from `derive` for `P`") && e.contains("Console")),
        "{errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.starts_with("4: in `take` from `derive` for `P`") && e.contains("!v")),
        "{errs:?}"
    );
}

/// [fn-by] The same template under a written declaration that says what the
/// body does is accepted: the declaration is the contract.
#[test]
fn a_written_declaration_may_grant_what_the_body_needs() {
    let main = r#"
import derive

struct P : Loud<self>, Taken<self> {
    x: Int,
    y: Str

    fn loud(v: P) [Console] -> Int by derive
    fn take(v: P) -> P => !v by derive
}
"#;
    let errs = errors_in(&[("derive.sv", LOUD), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
}

/// [comptime-generic] [obligation-by] At a generic target the member's
/// signature carries the generic additions, and a template whose names differ
/// from the group's still stamps the group's names.
#[test]
fn a_generic_target_takes_the_members_signature() {
    let derive = r#"
export comptime fn cmp<T is Struct>(x: T, y: T) [] -> Int => x, y {
    [for field in T.fields] {
        let c = cmp(x.[field], y.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
"#;
    let main = r#"
import derive

struct W<T> : Ordered<self> by derive { item: T, n: Int }

fn probe() [] -> Int {
    return cmp(W { item: "a", n: 1 }, W { item: "b", n: 1 })
}
"#;
    let errs = errors_in(&[("derive.sv", derive), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
    let dump = ir(&[("derive.sv", derive), ("main.sv", main)]);
    assert!(dump.contains("fn main::cmp<T>(lent a: W<T>, lent b: W<T>; ?lent cmp: (T, T) -> Int)"), "{dump}");
    assert!(dump.contains("a.item") && !dump.contains("x.item"), "{dump}");
}

/// The `fn` items of an IR dump, keyed by their first line, with blank lines
/// and comments dropped: what two spellings of one program are compared by.
fn fn_items(dump: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    for line in dump.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with("//") {
            continue;
        }
        if !line.starts_with(' ') {
            if let Some(c) = current.take() {
                items.push(c);
            }
            if line.starts_with("fn ") || line.contains(" fn ") {
                current = Some(String::new());
            }
        }
        if let Some(c) = &mut current {
            c.push_str(line);
            c.push('\n');
        }
    }
    items.extend(current);
    items.sort();
    items
}

/// [obligation-by] [fn-by] **A clause is one `fn … by X` per member**: the
/// group clause and the per-member declarations it stands for build the same
/// IR, fn for fn.
#[test]
fn a_clause_is_one_fn_by_per_member() {
    let clause = r#"
struct P : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto { x: Int, y: Str }

fn probe(p: P, q: P) [] -> Str {
    if p == q && hash(p) != 0L {
        return "${p}"
    }
    return "${cmp(p, q)}"
}
"#;
    let per_member = r#"
struct P : Ordered<self>, Hashed<self>, ToStr<self> {
    x: Int,
    y: Str

    fn cmp(a: P, b: P) -> Int by auto
    fn hash(value: P) -> Long by auto
    fn eq(a: P, b: P) -> Bool by auto
    fn to_str(value: P) -> Str by auto
}

fn probe(p: P, q: P) [] -> Str {
    if p == q && hash(p) != 0L {
        return "${p}"
    }
    return "${cmp(p, q)}"
}
"#;
    let a = fn_items(&ir(&[("main.sv", clause)]));
    let b = fn_items(&ir(&[("main.sv", per_member)]));
    assert_eq!(a.len(), 5, "{a:#?}");
    assert_eq!(a, b);
}

/// [fn-by] A declared parameter named like a function the template's body
/// calls would capture those calls once the parameters are renamed (ROADMAP
/// 0u): `std.core.auto`'s `cmp` calls `cmp(…)` per field. The declared names
/// are the contract and a local shadows a name [call-resolve], so it is one
/// error at the parameter — not the stamped body's "`cmp` is not callable" —
/// and nothing is stamped for the member.
#[test]
fn a_declared_parameter_that_hides_the_bodys_callee_is_named_at_the_parameter() {
    let src = r#"
struct P : Ordered<self> {
    x: Int
    fn cmp(cmp: P, b: P) -> Int by auto
}
"#;
    let errs = errors(src);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].starts_with("4: parameter `cmp` hides the function `cmp` that `auto`'s `cmp` calls in its body")
            && errs[0].ends_with("[fn-by]"),
        "{errs:?}"
    );
    // Any other name stamps, the declared one included.
    let renamed = src.replace("cmp(cmp: P", "cmp(first: P");
    assert!(errors(&renamed).is_empty(), "{:?}", errors(&renamed));
    let dump = ir(&[("main.sv", &renamed)]);
    assert!(dump.contains("fn main::cmp(lent first: P, lent b: P)") && dump.contains("first.x"), "{dump}");
}

/// [obligation-by] [fn-by] The same capture through a clause: the group's
/// member names the parameter, so the error lands at the clause and names the
/// group member; nothing is stamped for it.
#[test]
fn a_group_member_parameter_that_hides_the_bodys_callee_is_named_at_the_clause() {
    let src = r#"
export fns Ranked<T> {
    fn cmp(cmp: T, other: T) -> Int
}

struct Q : Ranked<self> by auto { y: Int }
"#;
    let errs = errors(src);
    // The checker's "no visible `cmp`" for the unstamped member is in the
    // same file, and `salvo analyze` drops the checker's errors in a file
    // whose parse or expansion reported one (`parse_broken`, [cli-analyze]):
    // the stamp's is the one diagnostic shown.
    let (_, expansion) = program(&[("main.sv", src)]);
    assert_eq!(expansion.len(), 1, "{expansion:?}");
    assert!(!errs.iter().any(|e| e.contains("not callable")), "{errs:?}");
    assert!(
        expansion[0].starts_with(
            "6: `Ranked<self>`'s `fn cmp` names its parameter `cmp`, which hides the function `cmp` \
             that `auto`'s `cmp` calls in its body"
        ),
        "{expansion:?}"
    );
    let renamed = src.replace("cmp(cmp: T", "cmp(one: T");
    assert!(errors(&renamed).is_empty(), "{:?}", errors(&renamed));
    let dump = ir(&[("main.sv", &renamed)]);
    assert!(dump.contains("fn main::cmp(lent one: Q, lent other: Q)") && dump.contains("one.y"), "{dump}");
}
