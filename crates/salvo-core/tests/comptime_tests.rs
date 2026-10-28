//! [comptime-bound] [comptime-inline] Which copies of an expanded-in-place
//! comptime fn a condition, a pattern or a dispatch keeps: each case's body
//! refuses with the copy's own text (`refuse!("${f.name}")`), and the test
//! reads the texts back. That is what an annotation cannot say, so these stay
//! Rust, over the real std (`salvo_testkit::front`); every diagnostic
//! these cases test is data, `tests/check/comptime/` (TESTS.md T1).
//!
//! The `by`-site half (`std.core.auto`, obligations, `refuse!` at a `by`) is
//! in `check/compare/`; these are about a comptime fn as a fn.

/// What each `refuse!` in `src` said, sorted, checked over the real std: a
/// body that refuses with `${field.name}` reports exactly the copies a
/// condition selected. Any other error fails the test.
fn refused(src: &str) -> Vec<String> {
    let out = salvo_testkit::front::check_main(src);
    let others: Vec<String> = out
        .diagnostics()
        .filter(|d| d.is_error() && d.id != Some("comptime-refuse:refused"))
        .map(|d| d.render(&out.program.files))
        .collect();
    assert!(others.is_empty(), "unexpected errors: {others:#?}");
    let mut texts: Vec<String> = out
        .diagnostics()
        .filter(|d| d.id == Some("comptime-refuse:refused"))
        .filter_map(|d| d.message.split_once(" refused: ").map(|(_, t)| t.to_string()))
        .collect();
    texts.sort();
    texts
}

/// Every warning `src` reports over the real std.
fn warnings(src: &str) -> Vec<String> {
    salvo_testkit::front::check_main(src).warnings()
}

// ===== [comptime-logic] `&&`, `||`, `!` and parentheses =====

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

// ===== [comptime-name] `field.name` is a literal union =====

/// [comptime-name] `field.name` is a literal of the union of the struct's
/// field names, so `is` / `!is` test it against one literal or a union of
/// them (user decision 2026-10-20).
///
/// The refusals are test data: `check/comptime/names.sv`.
#[test]
fn a_field_name_is_tested_against_a_union_of_literals() {
    assert_eq!(refused(&selecting(r#"f.name is "a" | "c""#)), ["a", "c"]);
    assert_eq!(refused(&selecting(r#"f.name !is "a" | "c" | "d""#)), ["b"]);
    assert_eq!(refused(&selecting(r#"f.name != "a" && f.name !is "d""#)), ["b", "c"]);
}

/// [comptime-name] `[when field.name]` dispatches on the name and must cover
/// every field — it takes no `else`, as no `when` with a subject does: a field
/// added later is an error in every `[when field.name]` that did not consider
/// it.
///
/// The refusals are test data: `check/comptime/names.sv`.
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
}

/// [comptime-name] `[if a.name == b.name]` relates binders of two walks
/// (the CLI design record X16): a derive merging two structs finds the shared field
/// names — `!=` too — and a side that is not a binder is an error.
///
/// The refusals are test data: `check/comptime/names.sv`.
#[test]
fn two_binders_names_compare() {
    let src = "struct A { port: Int, host: Str }\nstruct B { host: Str, verbose: Bool }\n\n\
               comptime fn clash() [] -> Int {\n    \
               [for a in A.fields] {\n        [for b in B.fields] {\n            \
               [if a.name == b.name] {\n                refuse!(\"shared ${a.name}\")\n            }\n            \
               [if a.name != b.name && a.first && b.first] {\n                refuse!(\"differ ${a.name} ${b.name}\")\n            }\n        }\n    }\n    \
               return 0\n}\n";
    assert_eq!(refused(src), ["differ port host", "shared host"]);
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
///
/// The refusals are test data: `check/comptime/kinds.sv`.
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
}

// ===== [comptime-generic-fn] an expanded-in-place fn may be generic =====

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

/// [comptime-pattern] `X?` matches any union with a `None` arm, `X` standing
/// for the rest: an optional literal union is three arms, and arm-by-arm
/// matching against the pattern's two found none (the CLI design record 2.2).
#[test]
fn an_optional_pattern_matches_an_optional_literal_union() {
    let src = "struct S {\n    level: (\"low\" | \"high\")?,\n    n: Int?,\n    both: Int | Str | None,\n    either: Int | Str\n}\n\n\
               comptime fn pick(s: S) [] -> Int => s {\n    \
               [for f in S.fields] {\n        \
               [if f.type is <E> E?] {\n            \
               [if E.kind is Union] { refuse!(\"union:${f.name}\") } else { refuse!(\"one:${f.name}\") }\n        \
               }\n    }\n    return 0\n}\n";
    assert_eq!(refused(src), ["one:n", "union:both", "union:level"]);
}

/// [comptime-pattern] What the left side of `&&` bound is in scope on the
/// right (`<S> S? && S.kind is Struct`), and a left side that fails decides
/// the test without reading the binder it did not bind.
#[test]
fn a_binder_is_in_scope_on_the_right_of_and() {
    let src = "struct A { x: Int }\nstruct S {\n    a: A?,\n    n: Int?,\n    m: Int\n}\n\n\
               comptime fn pick(s: S) [] -> Int => s {\n    \
               [for f in S.fields] {\n        \
               [if f.type is <E> E? && E.kind is Struct] { refuse!(\"group:${f.name}\") }\n    \
               }\n    return 0\n}\n";
    assert_eq!(refused(src), ["group:a"]);
    let warns = warnings(src);
    assert!(warns.is_empty(), "a binder read on the right is used: {warns:?}");
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
///
/// The refusals are test data: `check/comptime/patterns.sv`.
#[test]
fn a_binder_binds_nothing_under_or_or_not() {
    let src = over_shapes("[if f.type is <E> List<E> && f.first] { let e: E? = None\n refuse!(\"${f.name}\") }");
    assert_eq!(refused(&src), ["ints"]);
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
///
/// The refusals are test data: `check/comptime/patterns.sv`.
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
}

// ===== [expr-terminal-scope] `[for …] ...` and `[if …] ...` =====

/// [expr-terminal-scope] [comptime-inline] `[for …]` and `[if …]` take a terminal
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

// ===== [comptime-literals] a literal union's members =====

/// [comptime-literals] `[for lit in X.literals]` walks a literal union's
/// members in declaration order — a field's type, a named alias, `None`
/// skipped — with `lit.value` the member as a literal of the union (so it
/// can be returned where the union is expected); a type with no literals,
/// and `.value` on another binder, are errors.
///
/// The refusals are test data: `check/comptime/qualifiers.sv`.
#[test]
fn literal_members_are_walked_in_order() {
    let src = "type Level = \"debug\" | \"info\"\n\
               struct Opts {\n    output: (\"json\" | \"text\")?,\n    level: Level,\n    n: Int\n}\n\n\
               comptime fn names() [] -> Int {\n    \
               [for lit in Opts.fields] { }\n    \
               [for lit in Level.literals] {\n        refuse!(\"level ${lit.index} ${lit.value}\")\n        [if lit.name is \"info\"] {\n            refuse!(\"named info\")\n        }\n    }\n    \
               return 0\n}\n\n\
               comptime fn parse(s: Str) [] -> (\"json\" | \"text\")? => s {\n    \
               [for f in Opts.fields] {\n        [if f.name is \"output\"] {\n            \
               [for lit in f.type.literals] {\n                if s == lit.value {\n                    return lit.value\n                }\n            }\n        }\n    }\n    \
               return None\n}\n";
    assert_eq!(refused(src), ["level 0 debug", "level 1 info", "named info"]);
}
