//! [obligation-by] [fn-by] [comptime-instantiate] **A stamped member has the
//! signature it is declared with, never the template's** (user decision
//! 2026-10-14, ROADMAP §2c): the written one for `fn … by X`, the group
//! member's with `self` substituted for an obligation clause — which is one
//! `fn … by X` per member — and the template supplies only the body, checked
//! against that signature. Its effect list and deductions stop counting; its
//! parameter names are the template's (user decision 2026-10-30).
//!
//! Over the real standard library (`lib/`), so the stamps are `std.core.auto`'s.
//!
//! What the stamps' diagnostics are is data, `tests/check/stamp_signature/`
//! (TESTS.md T1); what is left here reads the IR they build, and the one case
//! whose diagnostics are not yet right (ROADMAP §0ar).

use salvo_core::Program;

/// [diag-id] An error as these tests read it: `line: id` for the user file
/// `main.sv`, `file:line: id` for another.
fn shown(program: &Program, file: usize, start: u32, id: Option<&str>) -> String {
    let file = &program.files[file];
    let line = file.content[..(start as usize).min(file.content.len())].matches('\n').count() + 1;
    let id = id.unwrap_or("?");
    if file.name == "main.sv" { format!("{line}: {id}") } else { format!("{}:{line}: {id}", file.name) }
}

fn program(files: &[(&str, &str)]) -> (Program, Vec<String>) {
    // [std-cache] Over the process's std front (TESTS.md T3).
    let built = salvo_testkit::front::program("kt", files);
    let errors = built
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| shown(&built.program, d.file, d.span.start, d.id))
        .collect();
    (built.program, errors)
}

/// Every error the expansion, resolver and checker report, as `line: id`
/// for the user file `main.sv` (other files' lines as `file:line`).
fn errors_in(files: &[(&str, &str)]) -> Vec<String> {
    let (program, mut errors) = program(files);
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_testkit::front::check_with("kt", &program, &resolution, &symbols);
    // [ir-analyze] Ownership errors are the IR's, as a build lists them.
    let flow = salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked);
    for d in resolution.errors.iter().chain(checked.errors.iter()).chain(flow.iter()).filter(|d| d.is_error()) {
        errors.push(shown(&program, d.file, d.span.start, d.id));
    }
    errors
}

/// The IR of the user modules, as `salvo ir` renders it.
fn ir(files: &[(&str, &str)]) -> String {
    let (program, errors) = program(files);
    assert!(errors.is_empty(), "{errors:?}");
    let (text, errors) = salvo_testkit::front::dump_ir("kt", &program, false, None).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(errors.is_empty(), "{errors:?}");
    text
}

/// A derive module whose template names its parameters as the group's member
/// does, with a default of its own that the stamp does not take
/// [fn-param-default].
const LABEL: &str = r#"
export fns Labelled<T> {
    fn label(value: T, sep: Str = " ") -> Str
}

export comptime fn label<T is Struct>(value: T, sep: Str = ",") [] -> Str => value {
    let out: Mut Str = Mut Str("${T.name}")
    [for field in T.fields] {
        append@string(out, sep)
        append@string(out, "${value.[field]}")
    }
    return out
}
"#;

/// [obligation-by] The clause's member is the group's: its names (which the
/// template shares) and its default, not the template's.
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
    // (That it checks clean, and that `between = ` is refused, is
    // `check/stamp_signature/names.sv`.)
    let dump = ir(&[("derive.sv", LABEL), ("main.sv", main)]);
    assert!(dump.contains("fn label@main(lent value: P@main, lent sep: Str)"), "{dump}");
    assert!(dump.contains("value.x"), "{dump}");
    // The declared default, not the template's.
    assert!(dump.contains(r#"y: "a" }, " ")"#), "{dump}");
}

/// [fn-by] A written declaration keeps its own deduction clause, and names its
/// parameters as the template does: `std.core.auto`'s `hash` reads `value`,
/// and its own local `h` stays `h`, since nothing is renamed.
#[test]
fn a_written_declaration_keeps_its_names_and_clause() {
    let src = r#"
struct P : Hashed<self> {
    x: Int,
    y: Str

    fn hash(value: P) -> Long by auto
    fn eq(a: P, b: P) -> Bool => a, b by auto
}

fn probe(p: P, q: P) [] -> Long {
    if eq(p, q) {
        return hash(p)
    }
    return 0L
}
"#;
    // (That it checks clean is `check/stamp_signature/written.sv`.)
    let dump = ir(&[("main.sv", src)]);
    assert!(dump.contains("fn hash@main(lent value: P@main)") && dump.contains("value.x") && !dump.contains("__p"), "{dump}");
    assert!(dump.contains("fn eq@main(lent a: P@main, lent b: P@main)") && dump.contains("a.x"), "{dump}");
}

/// [comptime-generic] [obligation-by] At a generic target the member's
/// signature carries the generic additions.
#[test]
fn a_generic_target_takes_the_members_signature() {
    let derive = r#"
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp@[site](a.[field], b.[field])
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
    // (That it checks clean is `check/stamp_signature/generic.sv`.)
    let dump = ir(&[("derive.sv", derive), ("main.sv", main)]);
    assert!(dump.contains("fn cmp@main<T>(lent a: W@main<T>, lent b: W@main<T>; ?lent cmp: (T, T) -> Int)"), "{dump}");
    assert!(dump.contains("a.item"), "{dump}");
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

/// [fn-by] [comptime-hygiene] A template's parameter named like the function
/// it calls at the site does not capture the call: a `name@[site]` does not
/// see the stamped fn's parameters (user decision 2026-10-30, P23 item 6).
#[test]
fn a_parameter_named_like_the_templates_site_callee_stamps() {
    let derive = r#"
export comptime fn cmp<T is Struct>(cmp: T, other: T) [] -> Int => cmp, other {
    [for field in T.fields] {
        let c = cmp@[site](cmp.[field], other.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
"#;
    let main = r#"
import derive

export fns Ranked<T> {
    fn cmp(cmp: T, other: T) -> Int
}

struct Q : Ranked<self> by derive { y: Int }
"#;
    let files = [("derive.sv", derive), ("main.sv", main)];
    assert!(errors_in(&files).is_empty(), "{:?}", errors_in(&files));
    let dump = ir(&files);
    assert!(dump.contains("fn cmp@main(lent cmp: Q@main, lent other: Q@main)") && dump.contains("cmp.y"), "{dump}");
}
