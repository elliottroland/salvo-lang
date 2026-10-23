//! [comptime-bound] [comptime-inline] Comptime fns beyond `by` stamping: the
//! expanded-in-place form called from ordinary code, what its conditions can
//! say, and how it reports (the 2026-10-20 comptime expansion; COMPLETED.md's
//! "Design record: comptime expansion").
//!
//! The `by`-site half (`core.auto`, obligations, `refuse!` at a `by`) is in
//! compare_tests.rs; these tests are about a comptime fn as a fn.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic types these sources use, loaded as std.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export intrinsic fn to_str(value: Int) [] -> Str => value\n",
    "export intrinsic fn to_str(value: Bool) [] -> Str => value\n",
    "export intrinsic type Char\n",
    "export intrinsic fn to_str(value: Char) [] -> Str => value\n",
    // [comptime-qualifier] The two subjects a comptime qualifier may have.
    "export comptime struct Field {}\n",
    "export comptime struct Struct {}\n",
);

/// A reported error: its message and the 1-based line it lands on in its file.
#[derive(Debug, Clone)]
struct Reported {
    file: String,
    line: usize,
    message: String,
    warning: bool,
}

fn errors(src: &str) -> Vec<Reported> {
    errors_in(&[("main.sv", src)])
}

fn warnings(src: &str) -> Vec<String> {
    diagnostics_in(&[("main.sv", src)]).into_iter().filter(|d| d.warning).map(|d| d.message).collect()
}

fn messages(src: &str) -> Vec<String> {
    errors(src).into_iter().map(|e| e.message).collect()
}

fn errors_in(files: &[(&str, &str)]) -> Vec<Reported> {
    diagnostics_in(files).into_iter().filter(|d| !d.warning).collect()
}

fn diagnostics_in(files: &[(&str, &str)]) -> Vec<Reported> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(*name, SourceSet::classify(Path::new(name)).unwrap(), src.to_string(), false);
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(parse_errors.is_empty(), "unexpected parse errors in {}: {parse_errors:?}", file.name);
        modules.push(ast);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    expansion
        .diagnostics
        .iter()
        .chain(resolution.errors.iter())
        .chain(checked.errors.iter())
        .map(|d| {
            let file = &program.files[d.file];
            let at = (d.span.start as usize).min(file.content.len());
            Reported {
                file: file.name.clone(),
                line: file.content[..at].matches('\n').count() + 1,
                message: d.message.clone(),
                warning: !d.is_error(),
            }
        })
        .collect()
}

// ===== [comptime-bound] an expanded-in-place comptime fn is an ordinary fn =====

/// [comptime-bound] A comptime fn with no kind bound is expanded once where it
/// is declared, and is then an ordinary fn: ordinary code calls it (user
/// decision 2026-10-20 — this had worked by construction, undecided and
/// untested).
#[test]
fn an_expanded_in_place_comptime_fn_is_callable() {
    let errs = messages(
        r#"
struct P { a: Int, b: Int, c: Int }

comptime fn total(p: P) [] -> Int => p {
    let n = 0
    [for field in P.fields] {
        n = n + p.[field]
    }
    return n
}

fn use_it(p: P) [] -> Int => p {
    return total(p) + total(p)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [comptime-bound] [comptime-inline] The expanded body is checked like a
/// hand-written one: a copy that does not type-check is an error **where the
/// failing code is written**, saying in the code's own terms which copy failed
/// — "when `field` is `b` (`Str`): …" (2026-10-20, two TODOs in
/// std/args/parse.sv: it landed on the field's declaration, away from the code
/// to fix, and its stamp-style prefix read as jargon).
#[test]
fn an_expanded_in_place_body_is_checked_per_copy() {
    let errs = errors(
        r#"
struct P {
    a: Int,
    b: Str
}

comptime fn total(p: P) [] -> Int => p {
    let n = 0
    [for field in P.fields] {
        n = n + p.[field]
    }
    return n
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.starts_with("when `field` is `b` (`Str`): `+` does not concatenate") && e.line == 10),
        "expected the `Str` field's copy to fail at the written `+`: {errs:?}"
    );
}

// ===== [comptime-root-visible] a root is a type name written here =====

/// [comptime-root-visible] `Pt.fields` names `Pt`, so `Pt` must be visible
/// where it is written, like any type name (user decision 2026-10-20). The
/// expansion's lookup is program-wide; before this a struct from a module the
/// file never imported was walked silently.
#[test]
fn a_root_must_be_visible_where_it_is_written() {
    let geo = ("geo.sv", "export struct Pt {\n    x: Int,\n    y: Int\n}\n");
    let body = r#"
comptime fn count() [] -> Int {
    let n = 0
    [for field in Pt.fields] {
        n = n + 1
    }
    return n
}
"#;
    let errs = errors_in(&[geo, ("main.sv", body)]);
    assert!(
        errs.iter().any(|e| e.file == "main.sv" && e.line == 4 && e.message.contains("unknown type `Pt`")),
        "expected the unimported root to be refused where it is written: {errs:?}"
    );

    let imported = format!("import geo\n{body}");
    let errs = errors_in(&[geo, ("main.sv", &imported)]);
    assert!(errs.is_empty(), "unexpected errors once imported: {errs:?}");
}

/// [comptime-root-visible] Of two same-named structs, the root means the one
/// the file imports.
#[test]
fn a_root_means_the_imported_declaration() {
    let a = ("a.sv", "export struct Pt {\n    x: Int\n}\n");
    let b = ("b.sv", "export struct Pt {\n    x: Int,\n    y: Int,\n    z: Int\n}\n");
    let main = r#"
import b

comptime fn count() [] -> Int {
    let n = 0
    [for field in Pt.fields] {
        [if field.name == "z"] {
            n = n + 100
        }
    }
    return n
}
"#;
    let errs = errors_in(&[a, b, ("main.sv", main)]);
    assert!(errs.is_empty(), "`z` exists on the imported `Pt`: {errs:?}");
}

// ===== [comptime-inline] diagnostics land where they are written =====

/// [comptime-inline] A mistake in a condition is reported once, at the
/// condition — not once per unrolled copy, at the `comptime` keyword.
#[test]
fn a_condition_error_is_reported_once_where_written() {
    let errs = errors(
        r#"
struct P { a: Int, b: Int, c: Int }

comptime fn f(p: P) [] -> Int => p {
    let n = 0
    [for field in P.fields] {
        [if field.name == "zz"] {
            n = n + 1
        }
    }
    return n
}
"#,
    );
    let hits: Vec<_> = errs.iter().filter(|e| e.message.contains("has no field `zz`")).collect();
    assert_eq!(hits.len(), 1, "expected one report: {errs:?}");
    assert_eq!(hits[0].line, 7, "expected it at the condition: {errs:?}");
}

/// [comptime-refuse] A `refuse!` in an expanded-in-place fn names the fn (there
/// is no stamped type) and lands at the `refuse!`.
#[test]
fn a_refuse_in_an_expanded_in_place_fn_names_the_fn() {
    let errs = errors(
        r#"
struct P { a: Int, b: Int }

comptime fn f(p: P) [] -> Int => p {
    [for field in P.fields] {
        [if field.name == "b"] {
            refuse!("`b` is not handled yet")
        }
    }
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message == "`f` refused: `b` is not handled yet [comptime-refuse]" && e.line == 7),
        "expected the refusal at the `refuse!`, naming the fn: {errs:?}"
    );
}

/// What each `refuse!` in `src` said, in order: a body that refuses with
/// `${field.name}` reports exactly the copies a condition selected.
fn refused(src: &str) -> Vec<String> {
    let mut out: Vec<String> = messages(src)
        .into_iter()
        .filter_map(|m| {
            m.strip_suffix(" [comptime-refuse]")
                .and_then(|m| m.split_once(" refused: ").map(|(_, t)| t.to_string()))
        })
        .collect();
    out.sort();
    out
}

/// A four-field struct and an expanded-in-place fn whose body refuses with
/// the field's name wherever `cond` holds.
fn selecting(cond: &str) -> String {
    format!(
        "struct P {{ a: Int, b: Int, c: Int, d: Str }}\n\n\
         comptime fn pick(p: P) [] -> Int => p {{\n    \
         [for f in P.fields] {{\n        \
         [if {cond}] {{\n            \
         refuse!(\"${{f.name}}\")\n        \
         }}\n    \
         }}\n    \
         return 0\n}}\n"
    )
}

// ===== [comptime-logic] `&&`, `||`, `!` and parentheses =====

/// [comptime-logic] Conditions combine with `&&`, `||`, `!` and parentheses
/// (user decision 2026-10-20), with the precedence the same operators have in
/// ordinary expressions: `&&` binds tighter than `||`.
#[test]
fn conditions_combine_with_ordinary_precedence() {
    assert_eq!(refused(&selecting(r#"f.name == "a" || f.name == "c""#)), ["a", "c"]);
    assert_eq!(refused(&selecting(r#"f.first && f.name == "a""#)), ["a"]);
    assert_eq!(refused(&selecting(r#"!(f.name == "a" || f.last)"#)), ["b", "c"]);
    // `a || b && c` is `a || (b && c)`: `b` is not last, so only `a`.
    assert_eq!(refused(&selecting(r#"f.name == "a" || f.name == "b" && f.last"#)), ["a"]);
    // Parentheses regroup it: `(a || b) && last` selects nothing.
    assert!(refused(&selecting(r#"(f.name == "a" || f.name == "b") && f.last"#)).is_empty());
}

/// [comptime-logic] Every leaf is validated, including one a short circuit
/// would skip: a misspelled field name is a mistake wherever it stands.
#[test]
fn a_short_circuited_leaf_is_still_checked() {
    let errs = messages(&selecting(r#"f.first || f.name == "zz""#));
    assert!(
        errs.iter().any(|e| e.contains("`P` has no field `zz`")),
        "expected the misspelling behind `||` to be reported: {errs:?}"
    );
}

// ===== [comptime-name] `field.name` is a literal union =====

/// [comptime-name] `field.name` is a literal of the union of the struct's
/// field names, so `is` / `!is` test it against one literal or a union of
/// them (user decision 2026-10-20).
#[test]
fn a_field_name_is_tested_against_a_union_of_literals() {
    assert_eq!(refused(&selecting(r#"f.name is "a" | "c""#)), ["a", "c"]);
    assert_eq!(refused(&selecting(r#"f.name !is "a" | "c" | "d""#)), ["b"]);
    assert_eq!(refused(&selecting(r#"f.name != "a" && f.name !is "d""#)), ["b", "c"]);
    let errs = messages(&selecting(r#"f.name is "a" | "e""#));
    assert!(
        errs.iter().any(|e| e.contains("`P` has no field `e`")),
        "expected a literal outside the union to be refused: {errs:?}"
    );
}

/// [comptime-name] `[when field.name]` dispatches on the name and must cover
/// every field — it takes no `else`, as no `when` with a subject does: a field
/// added later is an error in every `[when field.name]` that did not consider
/// it.
#[test]
fn a_when_over_a_field_name_is_exhaustive() {
    let when = |arms: &str| {
        format!(
            "struct P {{ a: Int, b: Int, c: Int, d: Str }}\n\n\
             comptime fn pick(p: P) [] -> Int => p {{\n    \
             [for f in P.fields] {{\n        \
             [when f.name] {{\n{arms}\n        }}\n    \
             }}\n    \
             return 0\n}}\n"
        )
    };
    // Every field covered: each copy takes its own arm.
    assert_eq!(
        refused(&when(r#"is "a" | "b" { refuse!("ab:${f.name}") } is "c" { refuse!("c") } is "d" {}"#)),
        ["ab:a", "ab:b", "c"]
    );
    // A missing field is named.
    let errs = messages(&when(r#"is "a" | "b" {} is "c" {}"#));
    assert!(
        errs.iter().any(|e| e.contains("does not cover every field of `P`: missing `d`")),
        "expected the uncovered field to be named: {errs:?}"
    );
    // A name covered twice, or one the struct does not have, is an error.
    let errs = messages(&when(r#"is "a" | "b" {} is "b" {} is "c" | "d" {}"#));
    assert!(errs.iter().any(|e| e.contains("`b` is already covered")), "{errs:?}");
    let errs = messages(&when(r#"is "a" | "z" {} is "b" | "c" | "d" {}"#));
    assert!(errs.iter().any(|e| e.contains("`P` has no field `z`")), "{errs:?}");
}

/// [comptime-name] An arm binder's name is the literal union of the union's
/// arm names, with the same tests and the same exhaustiveness.
#[test]
fn a_when_over_an_arm_name_is_exhaustive() {
    let src = |arms: &str| {
        format!(
            "struct A {{ n: Int }}\nstruct B {{ s: Str }}\ntype AB = A | B\n\n\
             comptime fn names() [] -> Int {{\n    \
             [for arm in AB.arms] {{\n        \
             [when arm.name] {{\n{arms}\n        }}\n    \
             }}\n    \
             return 0\n}}\n"
        )
    };
    assert!(messages(&src(r#"is "A" {} is "B" {}"#)).is_empty());
    let errs = messages(&src(r#"is "A" {}"#));
    assert!(
        errs.iter().any(|e| e.contains("does not cover every arm of `AB`: missing `B`")),
        "expected the uncovered arm to be named: {errs:?}"
    );
}

// ===== [comptime-kind] the kind is `.kind`, the type is `.type` =====

/// [comptime-kind] A kind is tested on `.kind` and a type on `.type`, so a
/// program type named like a kind is never mistaken for it (user decision
/// 2026-10-20). Before the split, `field.type is Union` with a user `struct
/// Union` read as the kind test and silently matched nothing.
#[test]
fn a_type_named_like_a_kind_is_a_type() {
    let src = |cond: &str| {
        format!(
            "struct Union {{ n: Int }}\n\
             struct Holder {{ u: Union, m: Int, s: Int? }}\n\n\
             comptime fn pick(h: Holder) [] -> Int => h {{\n    \
             [for f in Holder.fields] {{\n        \
             [if {cond}] {{\n            \
             refuse!(\"${{f.name}}\")\n        \
             }}\n    \
             }}\n    \
             return 0\n}}\n"
        )
    };
    assert_eq!(refused(&src("f.type is Union")), ["u"]);
    assert_eq!(refused(&src("f.kind is Struct")), ["u"]);
    // `Int?` is a nullable — a union kind — and no field's type is the
    // struct `Union`'s kind.
    assert_eq!(refused(&src("f.kind is Union")), ["s"]);
    assert_eq!(refused(&src("f.kind !is Opaque")), ["s", "u"]);
}

/// [comptime-kind] `[when field.kind]` is the kind dispatch, exhaustive over
/// `Kind`'s arms.
#[test]
fn a_when_over_a_kind_is_spelled_over_kind() {
    let src = |arms: &str| {
        format!(
            "struct Inner {{ n: Int }}\n\
             struct Holder {{ i: Inner, m: Int }}\n\n\
             comptime fn pick(h: Holder) [] -> Int => h {{\n    \
             [for f in Holder.fields] {{\n        \
             [when f.kind] {{\n{arms}\n        }}\n    \
             }}\n    \
             return 0\n}}\n"
        )
    };
    assert_eq!(
        refused(&src(r#"is Struct { refuse!("s:${f.name}") } is Union {} is Tuple {} is FnType {} is Opaque { refuse!("o:${f.name}") }"#)),
        ["o:m", "s:i"]
    );
    let errs = messages(&src(r#"is Struct {}"#));
    assert!(
        errs.iter().any(|e| e.contains("does not consider every arm of `Kind`") && e.contains("`Union`")),
        "expected the missing kinds to be named: {errs:?}"
    );
}

/// [comptime-kind] A kind word where a type is compared names no type, and
/// says so — in a test and in a `[when field.type]` alike.
#[test]
fn the_old_kind_spellings_name_the_new_ones() {
    let errs = messages(&selecting("f.type is Struct"));
    assert!(
        errs.iter().any(|e| e.contains("`Struct` is a kind, not a type") && e.contains("field.kind is Struct")),
        "expected the kind word to point at `.kind`: {errs:?}"
    );
    let errs = messages(&selecting("f.type is Strr"));
    assert!(
        errs.iter().any(|e| e.contains("unknown type `Strr` in a compile-time type test")),
        "expected an unknown type to be reported: {errs:?}"
    );
    // `[when x.type]` is the type dispatch now: a kind word there names no
    // type, and says so.
    let errs = messages(
        "struct P { a: Int }\ncomptime fn f(p: P) [] -> Int => p {\n    [for x in P.fields] {\n        [when x.type] {\n            is Struct {}\n            is * {}\n        }\n    }\n    return 0\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("`Struct` is a kind, not a type")),
        "expected the kind word in a type dispatch to point at `.kind`: {errs:?}"
    );
}

// ===== [comptime-generic-fn] an expanded-in-place fn may be generic =====

/// [comptime-generic-fn] An expanded-in-place comptime fn may declare ordinary
/// type parameters, and stays generic: it is expanded once, and the result is
/// an ordinary generic fn, checked under the usual rules — a `${a.default}` of
/// a `T?` field needs the `?to_str` a hand-written generic fn would (user
/// decision 2026-10-20).
#[test]
fn a_generic_comptime_fn_walks_an_instance_and_stays_generic() {
    let src = |sig_extra: &str| {
        format!(
            "struct Arg<T> {{ name: Str, default: T? }}\n\n\
             comptime fn show<T>(a: Arg<T>{sig_extra}) [] -> Str => a {{\n    \
             let out = \"\"\n    \
             [for f in Arg<T>.fields] {{\n        \
             [if f.name is \"default\"] {{\n            \
             if a.default !is None {{\n                \
             out = \"${{out}}${{a.default}}\"\n            \
             }}\n        \
             }} else {{\n            \
             out = \"${{out}}${{a.[f]}}\"\n        \
             }}\n    \
             }}\n    \
             return out\n}}\n\n\
             fn use_int(a: Arg<Int>) [] -> Str => a {{\n    return show(a)\n}}\n\n\
             fn use_bool(a: Arg<Bool>) [] -> Str => a {{\n    return show(a)\n}}\n"
        )
    };
    let errs = messages(&src(", ?to_str: (x: T) -> Str"));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    // Without the implicit, `T` has no text form — the ordinary rule.
    let errs = messages(&src(""));
    assert!(
        errs.iter().any(|e| e.contains("has no text form")),
        "expected the generic body to need `?to_str`: {errs:?}"
    );
}

/// [comptime-generic-fn] A generic struct is walked at an instance: a bare
/// root has no arguments to substitute, and says what to write.
#[test]
fn a_generic_root_must_be_instantiated() {
    let errs = messages(
        "struct Arg<T> { name: Str, default: T? }\n\n\
         comptime fn count<T>(a: Arg<T>) [] -> Int => a {\n    \
         let n = 0\n    \
         [for f in Arg.fields] {\n        n = n + 1\n    }\n    \
         return n\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("`Arg` is generic, so its fields are walked at an instance: write `Arg<T>.fields`")),
        "expected the instantiated form to be named: {errs:?}"
    );
    let errs = messages(
        "struct Box<A, B> { a: A, b: B }\n\n\
         comptime fn count() [] -> Int {\n    \
         let n = 0\n    \
         [for f in Box<Int>.fields] {\n        n = n + 1\n    }\n    \
         return n\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("`Box` takes 2 type argument(s), found 1")),
        "expected the arity to be checked: {errs:?}"
    );
}

/// [comptime-generic-fn] Field types come out substituted: at `Arg<Int>` the
/// `default: T?` field's type is `Int?`, and at the fn's own `Arg<T>` it is
/// `T?` with that `T` — opaque, kind `Generic`.
#[test]
fn field_types_are_substituted_at_the_instance() {
    let concrete = "struct Arg<T> { name: Str, default: T? }\n\n\
                    comptime fn pick() [] -> Int {\n    \
                    [for f in Arg<Int>.fields] {\n        \
                    [if f.type is Int?] {\n            refuse!(\"${f.name}\")\n        }\n    \
                    }\n    \
                    return 0\n}\n";
    assert_eq!(refused(concrete), ["default"]);
    let generic = "struct Arg<T> { name: Str, value: T, default: T? }\n\n\
                   comptime fn pick<T>(a: Arg<T>) [] -> Int => a {\n    \
                   [for f in Arg<T>.fields] {\n        \
                   [if f.type is T?] {\n            refuse!(\"opt:${f.name}\")\n        }\n        \
                   [when f.kind] {\n            \
                   is Opaque { refuse!(\"opaque:${f.name}\") }\n            \
                   is Struct {} is Union {} is Tuple {} is FnType {}\n        \
                   }\n    \
                   }\n    \
                   return 0\n}\n";
    assert_eq!(refused(generic), ["opaque:name", "opaque:value", "opt:default"]);
}

// ===== [comptime-pattern] type patterns: `*` and declared binders =====

/// A struct with fields of several shapes, and an expanded-in-place fn whose
/// body is `body` inside `[for f in S.fields]`.
fn over_shapes(body: &str) -> String {
    format!(
        "struct Arg<T> {{ default: T? }}\n\
         struct Pair<A, B> {{ a: A, b: B }}\n\
         struct S canbe Mut {{\n    \
         ints: List<Int> canbe Mut,\n    \
         args: List<Arg<Bool>>,\n    \
         same: Pair<Int, Int>,\n    \
         mixed: Pair<Int, Str>,\n    \
         opt: Int?,\n    \
         either: Int | Str,\n    \
         name: Str\n}}\n\n\
         comptime fn pick(s: S) [] -> Int => s {{\n    \
         [for f in S.fields] {{\n{body}\n    }}\n    \
         return 0\n}}\n"
    )
}

/// [comptime-pattern] `*` matches any type, at any depth; matching erases
/// qualifiers, so a `List<Int> canbe Mut` field is a `List<*>` (user decision
/// 2026-10-20: `*`, since `_` names a nameless variable).
#[test]
fn a_wildcard_matches_any_type() {
    let pick = |pat: &str| refused(&over_shapes(&format!("[if f.type is {pat}] {{ refuse!(\"${{f.name}}\") }}")));
    assert_eq!(pick("List<*>"), ["args", "ints"]);
    assert_eq!(pick("List<Arg<*>>"), ["args"]);
    assert_eq!(pick("Pair<Int, *>"), ["mixed", "same"]);
    assert_eq!(pick("*?"), ["opt"]);
    assert_eq!(pick("*"), ["args", "either", "ints", "mixed", "name", "opt", "same"]);
}

/// [comptime-pattern] Binders are declared before the pattern (`is <E>
/// List<E>`) and bind the part they stand at, in the then-branch: usable as a
/// type there, and as a root of further comptime tests.
#[test]
fn a_binder_binds_the_part_it_stands_at() {
    let src = over_shapes(
        "[if f.type is <E> List<E>] {\n    \
         let none: E? = None\n    \
         [if E is Int] { refuse!(\"int:${f.name}\") } else { refuse!(\"other:${f.name}\") }\n\
         }",
    );
    assert_eq!(refused(&src), ["int:ints", "other:args"]);
    // A binder met twice must meet one type: `<K> Pair<K, K>`.
    let src = over_shapes("[if f.type is <K> Pair<K, K>] { let k: K? = None\n refuse!(\"${f.name}\") }");
    assert_eq!(refused(&src), ["same"]);
}

/// [comptime-pattern] What a test proves holds only where the test did
/// [flow-or-not]: under `||` or `!` a binder binds nothing, and declaring one
/// there is an error; `&&` keeps it.
#[test]
fn a_binder_binds_nothing_under_or_or_not() {
    for cond in ["f.type is <E> List<E> || f.first", "!(f.type is <E> List<E>)", "f.type !is <E> List<E>"] {
        let errs = messages(&over_shapes(&format!("[if {cond}] {{}}")));
        assert!(
            errs.iter().any(|e| e.contains("bind nothing under `||` or `!`")),
            "expected `{cond}` to be refused: {errs:?}"
        );
    }
    let src = over_shapes("[if f.type is <E> List<E> && f.first] { let e: E? = None\n refuse!(\"${f.name}\") }");
    assert_eq!(refused(&src), ["ints"]);
}

/// [comptime-pattern] A binder may not hide a type, a type parameter or an
/// enclosing binder; one no copy uses warns, naming `*` as the remedy.
#[test]
fn binder_names_are_fresh_and_used() {
    let errs = messages(&over_shapes("[if f.type is <Int> List<Int>] {}"));
    assert!(errs.iter().any(|e| e.contains("pattern binder `Int` would hide")), "{errs:?}");
    let warns = warnings(&over_shapes("[if f.type is <E> List<E>] {}"));
    assert_eq!(
        warns.iter().filter(|w| w.contains("pattern binder `E` is never used: write `*`")).count(),
        1,
        "expected one warning for the unused binder: {warns:?}"
    );
    assert!(
        !warnings(&over_shapes("[if f.type is <E> List<E>] { let e: E? = None }"))
            .iter()
            .any(|w| w.contains("pattern binder")),
        "a used binder does not warn"
    );
}

/// [comptime-pattern] Unions match exactly, arm by arm, and `T?` is the union
/// `T | None`.
#[test]
fn unions_match_arm_by_arm() {
    let pick = |pat: &str| refused(&over_shapes(&format!("[if f.type is {pat}] {{ refuse!(\"${{f.name}}\") }}")));
    assert_eq!(pick("Int | Str"), ["either"]);
    assert!(pick("Str | Int").is_empty(), "arm order is part of a union's shape");
    assert_eq!(pick("Int | None"), ["opt"]);
    let src = over_shapes("[if f.type is <E> E?] { let e: E? = None\n [if E is Int] { refuse!(\"${f.name}\") } }");
    assert_eq!(refused(&src), ["opt"]);
}

/// [comptime-pattern] `[when field.type]` dispatches by pattern, the first
/// matching arm taken.
#[test]
fn a_when_over_a_type_dispatches_by_pattern() {
    let src = over_shapes(
        "[when f.type] {\n    \
         is <E> List<E> { let e: E? = None\n refuse!(\"list:${f.name}\") }\n    \
         is Pair<*, *> { refuse!(\"pair:${f.name}\") }\n    \
         is Str { refuse!(\"str:${f.name}\") }\n    \
         is * {}\n\
         }",
    );
    assert_eq!(refused(&src), ["list:args", "list:ints", "pair:mixed", "pair:same", "str:name"]);
}

/// [comptime-pattern] [when-exhaustive] A `[when field.type]` must cover the
/// types its subject ranges over, with no `else` (`is *` is the catch-all) — known, since
/// the walk is over one struct's fields or one union's arms (user question
/// 2026-10-20). Each uncovered type is reported, naming it.
#[test]
fn a_when_over_a_type_covers_the_range() {
    let covering = over_shapes(
        "[when f.type] {\n    \
         is List<*> { refuse!(\"list:${f.name}\") }\n    \
         is Pair<*, *> { refuse!(\"pair:${f.name}\") }\n    \
         is * { }\n\
         }",
    );
    assert_eq!(refused(&covering), ["list:args", "list:ints", "pair:mixed", "pair:same"]);
    let errs = messages(&over_shapes("[when f.type] { is List<*> {} is Pair<*, *> {} is Str {} }"));
    let uncovered: Vec<&String> = errs.iter().filter(|e| e.contains("no arm of this `[when f.type]` matches")).collect();
    assert_eq!(uncovered.len(), 2, "expected `Int?` and `Int | Str` reported: {errs:?}");
    assert!(uncovered.iter().any(|e| e.contains("`Int?`")), "{errs:?}");
    assert!(uncovered.iter().any(|e| e.contains("`Int | Str`")), "{errs:?}");
    // A union's arms are a known range too.
    let arms = "type U = Int | Str | List<Int>\n\n\
                comptime fn pick(u: U) [] -> Int => u {\n    \
                [for a in U.arms] {\n        \
                [when a.type] { is Int {} is Str {} is List<*> {} }\n    \
                }\n    \
                return 0\n}\n";
    assert!(messages(arms).is_empty(), "every arm is covered: {:?}", messages(arms));
    let short = arms.replace(" is List<*> {}", "");
    let errs = messages(&short);
    assert!(errs.iter().any(|e| e.contains("matches `List<Int>`")), "{errs:?}");
}

/// [when-exhaustive] A `[when]` with a subject follows an ordinary `when`'s
/// rule: it covers its subject's range, so the range must be one the
/// declaration can see. In a **stamped** comptime fn the fields of `T` are
/// known only where it is stamped, so a `[when f.type]` or `[when f.name]`
/// over them is refused at the template — once, with no echo at the `by` site
/// — while `[when f.kind]` (over `Kind`, the same everywhere) stays, and is
/// checked for every kind there too (user decisions 2026-10-20).
#[test]
fn a_stamped_fn_refuses_a_subject_when_over_its_range() {
    let template = |body: &str| {
        format!(
            "export comptime fn tag<T is Struct>(value: T) [] -> Int => value {{\n    \
             [for f in T.fields] {{\n        {body}\n    }}\n    \
             return 0\n}}\n"
        )
    };
    let main = "struct P { x: Int, s: Str }\n\nfn tag(p: P) [] -> Int => p by tags\n";
    for (body, what) in [
        ("[when f.type] { is Int {} is Str {} }", "`[when f.type]` cannot be checked here"),
        ("[when f.type] { is Int {} is * {} }", "`[when f.type]` cannot be checked here"),
        ("[if f.type is <E> List<E>] { [when E] { is Int {} is * {} } }", "`[when E]` cannot be checked here"),
        ("[when f.name] { is \"x\" {} is \"s\" {} }", "`[when f.name]` cannot be checked here"),
        ("[when f.kind] { is Struct {} }", "does not consider every arm of `Kind`"),
    ] {
        let tags = template(body);
        let errs = errors_in(&[("tags.sv", &tags), ("main.sv", main)]);
        assert_eq!(errs.len(), 1, "`{body}`: one diagnostic: {errs:?}");
        assert!(errs[0].message.contains(what) && errs[0].file.ends_with("tags.sv"), "`{body}`: {errs:?}");
    }
    let kind = template("[when f.kind] { is Struct {} is Union {} is Tuple {} is FnType {} is Opaque {} }");
    let errs = errors_in(&[("tags.sv", &kind), ("main.sv", main)]);
    assert!(errs.is_empty(), "a kind dispatch is checkable anywhere: {errs:?}");
}


/// [when-exhaustive] [comptime-kind] An incomplete `[when f.kind]` in an
/// expanded-in-place fn is reported once, at the line it is written on — it
/// was pushed with the copy's virtual span, past the end of the file, once per
/// field (found 2026-10-20 with the arm completions).
#[test]
fn an_incomplete_kind_dispatch_lands_where_written() {
    let src = "struct P { a: Int, b: Str }\n\n\
               comptime fn kinds(p: P) [] -> Int => p {\n    \
               [for f in P.fields] {\n        \
               [when f.kind] {\n            is Struct {}\n        }\n    \
               }\n    \
               return 0\n}\n";
    let errs: Vec<Reported> =
        errors(src).into_iter().filter(|e| e.message.contains("does not consider every arm of `Kind`")).collect();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(errs[0].line, 5, "{errs:?}");
}

// ===== [terminal-scope] `[for …] ...` and `[if …] ...` =====

/// [terminal-scope] [comptime-inline] `[for …]` and `[if …]` take a terminal
/// `...` body as `for` and `if` do (user request 2026-10-20): the rest of the
/// enclosing block, unrolled per field or kept when the condition holds — with
/// an optional inline braced `else` on the `[if]`, kept instead.
#[test]
fn comptime_for_and_if_take_terminal_bodies() {
    let src = |body: &str| {
        format!(
            "struct P {{ a: Int, b: Int, c: Str }}\n\n\
             comptime fn pick(p: P) [] -> Int => p {{\n{body}\n    return 0\n}}\n"
        )
    };
    // A guard inside a braced walk: the rest of the copy is kept per field.
    let guard = src("    [for f in P.fields] {\n        [if f.name !is \"a\"] ...\n        refuse!(\"${f.name}\")\n    }");
    assert_eq!(refused(&guard), ["b", "c"]);
    // A terminal walk: the rest of the body, once per field.
    let walk = "struct P { a: Int, b: Int }\n\n\
                comptime fn pick(p: P) [] -> Int => p {\n    \
                [for f in P.fields] ...\n    \
                refuse!(\"${f.name}\")\n}\n";
    assert_eq!(refused(walk), ["a", "b"]);
    // The inline `else` is kept when the condition fails.
    let with_else = src("    [for f in P.fields] {\n        [if f.type is Int] ... else {\n            refuse!(\"other:${f.name}\")\n        }\n        refuse!(\"int:${f.name}\")\n    }");
    assert_eq!(refused(&with_else), ["int:a", "int:b", "other:c"]);
}

// ===== [comptime-scope] a comptime body is a scope =====

/// An expanded-in-place fn over `struct P { a: Int, b: Str }` whose body is
/// `body`, then `tail`.
fn scoped(body: &str, tail: &str) -> String {
    format!(
        "struct P {{ a: Int, b: Str }}\n\n\
         comptime fn pick(p: P) [] -> Int => p {{\n    \
         let total: Int = 0\n    \
         {body}\n    \
         {tail}\n}}\n"
    )
}

/// [comptime-scope] What a kept `[if]` branch, its `else`, or a chosen
/// `[when]` arm declares is not visible after it — the body was spliced flat
/// before, and `return k` compiled (user request 2026-10-20).
#[test]
fn a_kept_comptime_body_does_not_leak_its_locals() {
    for body in [
        "[if P.kind is Struct] { let k = 1\n total = k }",
        "[if P.kind is Union] {} else { let k = 1\n total = k }",
        "[when P.kind] { is Struct { let k = 1\n total = k } is Union {} is Tuple {} is FnType {} is Opaque {} }",
        "[for f in P.fields] { [if f.first] { let k = 1\n total = k } }",
    ] {
        let errs = messages(&scoped(body, "return k"));
        assert!(
            errs.len() == 1 && errs[0].contains("no variable or function named `k`"),
            "`{body}`: {errs:?}"
        );
        // Assigning to a local declared outside is unaffected.
        assert_eq!(messages(&scoped(body, "return total")), Vec::<String>::new(), "`{body}`");
    }
}

/// [comptime-scope] [var-no-shadow] The rename that keeps a body's locals in
/// does not let them hide a visible one: refused by the name as written, also
/// in a `[for]` copy (which renamed, and so allowed it, before), and through
/// both renamings when an `[if]` sits in a copy.
#[test]
fn a_comptime_body_may_not_shadow_a_visible_local() {
    for body in [
        "[if P.kind is Struct] { let total = 1 }",
        "[for f in P.fields] { let total = 1 }",
        "[for f in P.fields] { let k = 1\n [if f.first] { let k = 2 } }",
    ] {
        let errs = messages(&scoped(body, "return total"));
        assert!(
            !errs.is_empty()
                && errs.iter().all(|e| e.contains("is already declared (shadowing is not allowed)") && !e.contains("__c")),
            "`{body}`: {errs:?}"
        );
    }
    // Sibling copies and sibling branches each declare their own.
    let ok = "[for f in P.fields] { [if f.first] { let k = 1\n total = k } else { let k = 2\n total = k } }";
    assert_eq!(messages(&scoped(ok, "return total")), Vec::<String>::new());
}

/// [comptime-scope] [terminal-scope] A terminal `[if …] ...` renames the rest
/// of the block, which holds every use, so it reads as before; and a renamed
/// local is named as written in a warning.
#[test]
fn a_terminal_comptime_if_still_sees_its_rest() {
    assert_eq!(messages(&scoped("[if P.kind is Struct] ...", "let k = total\n    return k")), Vec::<String>::new());
    let warns = warnings(&scoped("[if P.kind is Struct] { let unused = 1 }", "return total"));
    assert!(warns.iter().any(|w| w.starts_with("`unused` is never used")), "{warns:?}");
}

// ===== [comptime-copy-warn] warnings raised inside copies =====

/// [comptime-copy-warn] A warning inside a copy of an expanded-in-place fn
/// lands on the written code (it kept the copy's synthetic span, past the end
/// of the file, before 2026-10-20) and is reported once: an unused local only
/// when no copy used it, and any other warning once for every copy that
/// raises it alike.
#[test]
fn a_warning_in_copies_is_reported_once_on_the_written_code() {
    let src = "struct S { a: Int, b: Str }\n\
               comptime fn pick(s: S) [] -> Int => s {\n    \
               let total: Int = 0\n    \
               [for f in S.fields] {\n        \
               let y = 1\n        \
               [if f.type is Int] { total = y }\n        \
               let z = 2\n        \
               [if f.first] { todo!() }\n    \
               }\n    \
               return total\n}\n";
    let all = diagnostics_in(&[("main.sv", src)]);
    let warns: Vec<&Reported> = all.iter().filter(|d| d.warning).collect();
    let lines = src.lines().count();
    assert!(warns.iter().all(|w| w.line <= lines), "a warning past the end of the file: {warns:?}");
    let unused: Vec<&&Reported> = warns.iter().filter(|w| w.message.contains("is never used")).collect();
    assert_eq!(unused.len(), 1, "only `z`, unused in every copy: {warns:?}");
    assert!(unused[0].message.starts_with("`z` is never used") && unused[0].line == 7, "{warns:?}");
    let todos = warns.iter().filter(|w| w.message.contains("todo!")).count();
    assert_eq!(todos, 1, "one `todo!()`, reported once: {warns:?}");
}

// ===== [comptime-inline] a binder's type spliced into a type position =====

/// [comptime-inline] [comptime-fields] `[field.type]` stands for the field's
/// type in every type position — a `let` annotation, a type argument of a
/// call or of another type, under `?` and after a qualifier — substituted
/// per copy (ARG_PARSE.md §13 (B), user decision 2026-10-20).
#[test]
fn a_spliced_binder_type_is_the_fields_type_in_each_copy() {
    let src = |init: &str| {
        format!(
            "struct P {{ n: Int, s: Str }}\n\n\
             fn probe<T>(x: T) [] -> Int => x {{\n    return 0\n}}\n\n\
             fn none<T>() [] -> T? {{\n    return None\n}}\n\n\
             comptime fn count(p: P) [] -> Int => p {{\n    \
             let total = 0\n    \
             [for f in P.fields] {{\n        \
             let v: [f.type] = {init}\n        \
             let w = probe<[f.type]>(v)\n        \
             let xs: List<[f.type]>? = None\n        \
             let o: [f.type]? = none<[f.type]>()\n        \
             total = total + 1\n    \
             }}\n    \
             return total\n}}\n"
        )
    };
    let errs = messages(&src("p.[f]"));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    // Only the `Str` copy fails, so the type really is the field's.
    let errs = messages(&src("1"));
    assert_eq!(errs.len(), 1, "expected one error, from the `s` copy: {errs:?}");
    assert!(errs[0].contains("`f` is `s`"), "expected the copy to be named: {errs:?}");
}

/// [comptime-inline] `[arm.type] { … }` is a struct literal whose head is the
/// arm's type, per copy: over a union of structs each copy builds its own arm
/// (the literal checked against that struct's fields). Building a qualified or
/// literal arm is ARG_PARSE.md step 1.6.
#[test]
fn a_spliced_arm_type_heads_a_struct_literal() {
    let src = |entries: &str| {
        format!(
            "struct A {{ n: Int }}\nstruct B {{ n: Int, m: Int }}\ntype U = A | B\n\n\
             fn keep(u: U) [] -> Int => u {{\n    return 0\n}}\n\n\
             comptime fn build() [] -> Int {{\n    \
             let total = 0\n    \
             [for arm in U.arms] {{\n        \
             let v: [arm.type] = [arm.type] {{ {entries} }}\n        \
             total = total + keep(v)\n    \
             }}\n    \
             return total\n}}\n"
        )
    };
    let errs = messages(&src("n: arm.index"));
    // `B` also needs `m`, so only its copy fails.
    assert_eq!(errs.len(), 1, "expected one error, from the `B` copy: {errs:?}");
    assert!(errs[0].contains("`arm` is `B`") && errs[0].contains("`m`"), "{errs:?}");
}

// ===== [comptime-default] a field's declared default =====

/// [comptime-default] `field.default` is the field's declared default where it
/// has one and `None` where it has none, and `[if field.default is None]`
/// tells the two apart: guarded, every copy checks; unguarded, only the copy
/// for the field without a default fails, as `None` is not an `Int`.
#[test]
fn a_field_default_is_its_declared_default_or_none() {
    let src = |body: &str| {
        format!(
            "struct P {{ a: Int = 3, b: Int, c: Int? = None }}\n\n\
             comptime fn sum(p: P) [] -> Int => p {{\n    \
             let total = 0\n    \
             [for f in P.fields] {{\n{body}\n    }}\n    \
             return total\n}}\n"
        )
    };
    let errs = messages(&src("        [if f.default !is None && f.name !is \"c\"] {\n            let d: [f.type] = f.default\n            total = total + d\n        }"));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    let errs = messages(&src("        let d: Int? = f.default\n        total = total + (d ?: 0)"));
    assert!(errs.is_empty(), "a default passes to an optional either way: {errs:?}");
    let errs = messages(&src("        let d: Int = f.default\n        total = total + d"));
    assert_eq!(errs.len(), 2, "expected the `b` and `c` copies to fail: {errs:?}");
    assert!(errs.iter().any(|e| e.contains("`f` is `b`")) && errs.iter().any(|e| e.contains("`f` is `c`")), "{errs:?}");
}

/// [comptime-default] Only a field has a default: `.default` on an arm binder,
/// as a value or in a head, is an error naming why.
#[test]
fn an_arm_has_no_default() {
    for body in ["        let d = arm.default", "        [if arm.default is None] { total = total + 1 }"] {
        let src = format!(
            "struct A {{ n: Int }}\nstruct B {{ n: Int }}\ntype U = A | B\n\n\
             comptime fn count() [] -> Int {{\n    let total = 0\n    [for arm in U.arms] {{\n{body}\n    }}\n    return total\n}}\n"
        );
        let errs = messages(&src);
        assert!(errs.iter().any(|e| e.contains("only a field has a default")), "{body}: {errs:?}");
    }
}

// ===== [comptime-docs] doc comments as values =====

/// [comptime-docs] `field.docs`, `arm.docs`, `T.docs` and a named root's
/// `.docs` are the doc comment above the declaration, its lines joined as
/// hover joins them, or `None`; `[if x.docs is None]` tells which. An arm's
/// is the declaration it names.
#[test]
fn docs_are_the_doc_comment_or_none() {
    let src = "// Serve files.\n// Over HTTP.\nstruct Serve {\n    // The address.\n    host: Str,\n    port: Int\n}\n\n\
               // An arm.\nstruct A { n: Int }\nstruct B { n: Int }\ntype U = A | B\n\n\
               comptime fn help() [] -> Int {\n    \
               [if Serve.docs !is None] {\n        refuse!(\"struct ${Serve.docs}\")\n    }\n    \
               [for f in Serve.fields] {\n        [if f.docs is None] {\n            refuse!(\"${f.name} none\")\n        } else {\n            refuse!(\"${f.name} ${f.docs}\")\n        }\n    }\n    \
               [for arm in U.arms] {\n        [if arm.docs !is None] {\n            refuse!(\"${arm.name} ${arm.docs}\")\n        }\n    }\n    \
               return 0\n}\n";
    assert_eq!(
        refused(src),
        ["A An arm.", "host The address.", "port none", "struct Serve files.\nOver HTTP."]
    );
}

/// [comptime-docs] A copy's `docs` is a `Str` literal or `None`, so it is
/// tested in a head: `?:` on it is refused in each copy as on any value with
/// one arm, which the error names.
#[test]
fn a_copys_docs_are_tested_in_a_head() {
    let errs = messages(
        "struct S {\n    // Doc.\n    a: Int\n}\n\n\
         comptime fn help() [] -> Str {\n    let out: Mut Str = Mut Str(\"\")\n    \
         [for f in S.fields] {\n        append(out, f.docs ?: \"-\")\n    }\n    return out\n}\n",
    );
    assert!(errs.iter().any(|e| e.contains("when `f` is `a`") && e.contains("`?:`")), "{errs:?}");
}

// ===== [comptime-qualifier] qualifiers on declarations, read by comptime =====

const ANNOTATED: &str = "comptime qualifier Short(c: Char) of Field\n\
                         comptime qualifier Positional of Field\n\
                         comptime qualifier Named(name: Str) of Struct\n\
                         qualifier Small(hi: Int) of Int {\n    fn qualifies(n: Int, hi: Int) -> Bool {\n        return n < hi\n    }\n}\n\n\
                         // Serves.\n\
                         Named(\"srv\")\n\
                         struct Serve {\n    Short('p') port: Small(100) Int = 80,\n    host: Str,\n    Positional Short('f') files: List<Str>\n}\n\n";

/// [comptime-qualifier] `[if field is Short]`, `[if field is <c> Short(c)]`
/// (the constant bound and read as a value), a constant compared
/// (`Short('f')`), `!is`, a struct's own (`Serve is <n> Named(n)`), and a
/// field type's qualifier constants (`<hi> Small(hi) *`).
#[test]
fn comptime_qualifiers_are_read_where_they_are_written() {
    let src = format!(
        "{ANNOTATED}comptime fn help() [] -> Int {{\n    \
         [if Serve is <n> Named(n)] {{\n        refuse!(\"named ${{n}}\")\n    }}\n    \
         [for f in Serve.fields] {{\n        \
         [if f is <c> Short(c)] {{\n            refuse!(\"${{f.name}} short ${{c}}\")\n        }}\n        \
         [if f is Short('f')] {{\n            refuse!(\"${{f.name}} is f\")\n        }}\n        \
         [if f !is Positional] {{\n            refuse!(\"${{f.name}} named\")\n        }}\n        \
         [if f.type is <hi> Small(hi) *] {{\n            refuse!(\"${{f.name}} below ${{hi}}\")\n        }}\n    \
         }}\n    return 0\n}}\n"
    );
    assert_eq!(
        refused(&src),
        ["files is f", "files short f", "host named", "named srv", "port below 100", "port named", "port short p"]
    );
}

/// [comptime-qualifier] A comptime qualifier's declaration and its uses are
/// checked: its subject, its body, its constants, and where it is written.
#[test]
fn comptime_qualifiers_are_checked_where_written() {
    let errs = messages(
        "comptime qualifier Short(c: Char) of Field\n\
         comptime qualifier Named(name: Str) of Struct\n\
         comptime qualifier Bad of Int\n\
         comptime qualifier Wide(x: List<Int>) of Field\n\
         qualifier Plain of Int\n\n\
         struct S {\n    Short(\"p\") a: Int,\n    Named(\"x\") c: Int,\n    Plain d: Int,\n    e: Short('p') Int\n}\n",
    );
    for want in [
        "describes a `core.comptime` `Field` or `Struct`, not `Int`",
        "a comptime qualifier's slot holds a constant: `Int`, `Char` or `Str`, not `List<Int>`",
        "`\"p\"` fills the `c` slot of `Short`, which needs `Char`, not `Str`",
        "`Named` describes a `Struct`, not a `Field`",
        "`Plain` is not a comptime qualifier",
        "`Short` is a comptime qualifier: it is written before a field's name",
    ] {
        assert!(errs.iter().any(|e| e.contains(want)), "missing {want:?} in {errs:?}");
    }
    assert!(!errs.iter().any(|e| e.contains("does not apply to")), "one error per misplaced qualifier: {errs:?}");
}

/// [comptime-qualifier] A constant binder is lowercase (an uppercase one would
/// bind a type), binds nothing under `!`, and an arm carries no comptime
/// qualifiers yet.
#[test]
fn comptime_qualifier_tests_refuse_what_they_cannot_mean() {
    for (body, want) in [
        ("        [if f is <C> Short] { }", "would bind a type"),
        ("        [if !(f is <c> Short(c))] { }", "bind nothing under `||` or `!`"),
    ] {
        let src = format!("{ANNOTATED}comptime fn help() [] -> Int {{\n    [for f in Serve.fields] {{\n{body}\n    }}\n    return 0\n}}\n");
        let errs = messages(&src);
        assert!(errs.iter().any(|e| e.contains(want)), "{body}: {errs:?}");
    }
    let errs = messages(
        "comptime qualifier Short(c: Char) of Field\nstruct A { n: Int }\nstruct B { n: Int }\ntype U = A | B\n\n\
         comptime fn count() [] -> Int {\n    [for arm in U.arms] {\n        [if arm is Short] { }\n    }\n    return 0\n}\n",
    );
    assert!(errs.iter().any(|e| e.contains("an arm carries no comptime qualifiers yet")), "{errs:?}");
}
