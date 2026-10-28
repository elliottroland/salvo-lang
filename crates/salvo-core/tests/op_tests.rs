//! [op-arith] [op-order] [op-bool] [op-promote] [type-lit-adopt] The operator
//! typing slice (user decision 2026-09-14, FILE_SYSTEM.md §5.10.3):
//! arithmetic on numeric operands only, implicit widening *within* the
//! integer and float classes with the promotion recorded for the backends,
//! an explicit-conversion error *across* them, no `Str +`, `Bool`-only
//! logicals, and unsuffixed literals adopting the expected numeric type.
//!
//! Ordering and equality moved on 2026-09-21: they resolve through the
//! capability groups [op-order] [op-equality], so `<` is `cmp(a, b) < 0` and
//! what may be compared is "whatever has the function"; the capability half
//! lives in `compare_tests.rs`. Arithmetic followed on 2026-10-28 (B3.3e):
//! `+ - * /` are the calls `plus`, `minus`, `times`, `divide` [op-arith] and
//! `**` is `pow` [op-pow], which the second half of this file covers; a
//! number's are std's intrinsics, which the prelude below models.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols, Ty};

/// [platform-intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`).
const STD_PRELUDE: &str = concat!(
    concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Float\n",
    "export intrinsic type Double\n",
    "export intrinsic type Byte\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic fn to_long(value: Int) [] -> Long => value\n",
    "export intrinsic fn to_double(value: Int) [] -> Double => value\n",
    "export intrinsic fn to_byte(value: Int) [] -> Byte => value\n",
    "export intrinsic fn to_int(value: Byte) [] -> Int => value\n",
),
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Float, b: Float) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Byte, b: Byte) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn plus(a: Float, b: Float) [] -> Float => a, b\n",
    "export intrinsic fn plus(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn minus(a: Float, b: Float) [] -> Float => a, b\n",
    "export intrinsic fn minus(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn times(a: Float, b: Float) [] -> Float => a, b\n",
    "export intrinsic fn times(a: Double, b: Double) [] -> Double => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn divide(a: Float, b: Float) [] -> Float => a, b\n",
    "export intrinsic fn divide(a: Double, b: Double) [] -> Double => a, b\n",
    // [op-pow] What `**` calls, as std declares it.
    "export intrinsic fn pow(base: Int, exponent: Int) [] -> Int => base, exponent\n",
    "export intrinsic fn pow(base: Long, exponent: Int) [] -> Long => base, exponent\n",
    "export intrinsic fn pow(base: Double, exponent: Int) [] -> Double => base, exponent\n",
    "export intrinsic fn pow(base: Double, exponent: Double) [] -> Double => base, exponent\n",
);

fn checked(src: &str) -> salvo_core::Checked {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    check_program(&program, &resolution, &symbols)
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    let mut diags = checked(src).errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

fn assert_ok(src: &str) {
    let errs = errors(src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

fn body(stmts: &str) -> String {
    format!("fn probe() -> None {{\n{stmts}\n    return None\n}}\n")
}

// ===== arithmetic [op-arith] =====

/// Same-type arithmetic on every numeric type, and the promoted result
/// feeding further arithmetic.
#[test]
fn numeric_arithmetic_is_legal() {
    assert_ok(&body(
        "    let a = 1 + 2 * 3 % 4\n    let b = 1L + 2L\n    let c = 1.5 * 2.0\n    \
         let d = -a\n    let _e = a + d",
    ));
}

/// [op-promote] `Int + Long` widens to `Long`, both directions, and the
/// promotion lands on the narrower operand's span. Same for the float
/// class. The result is the promoted type.
#[test]
fn integer_widening_promotes_and_records() {
    let out = checked(&body(
        "    let x: Long = 9L\n    let n = 5\n    let y = x + n\n    let z = n + x\n    \
         let f: Float = 1.0f\n    let d = 2.5\n    let g = f * d",
    ));
    assert!(
        out.errors.iter().all(|d| !d.is_error()),
        "unexpected errors: {:?}",
        out.errors
    );
    let promoted: Vec<String> = out.promotions.values().map(|t| t.to_string()).collect();
    assert_eq!(
        promoted.iter().filter(|t| *t == "Long").count(),
        2,
        "both `n` operands promote to Long: {promoted:?}"
    );
    assert_eq!(
        promoted.iter().filter(|t| *t == "Double").count(),
        1,
        "the `f` operand promotes to Double: {promoted:?}"
    );
    // The results carry the promoted type onward.
    let types: Vec<String> = out.expr_ty.values().map(|t| t.to_string()).collect();
    assert!(types.iter().any(|t| t == "Long"), "{types:?}");
}

/// Mixing the integer and float classes is an error naming the explicit
/// conversions, for arithmetic and for ordering alike.
#[test]
fn int_float_mix_needs_explicit_conversion() {
    let msgs = messages(&body("    let a = 1 + 2.5"));
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].contains("cannot mix `Int` and `Double`") && msgs[0].contains("to_double"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = 1L < 2.5"));
    assert!(
        msgs[0].contains("cannot mix `Long` and `Double`"),
        "{msgs:?}"
    );
    // And the conversions fix it.
    assert_ok(&body("    let a = to_double(1) + 2.5\n    let b = to_long(1) + 2L"));
}

/// [op-arith] `+` on strings is refused toward interpolation; other
/// non-numeric operands get the general message. `Byte` is deliberately
/// not operator-numeric [type-byte]: it is an octet, and its arithmetic
/// goes through `to_int`/`to_byte`.
#[test]
fn non_numeric_arithmetic_is_refused() {
    let msgs = messages(&body("    let a = \"x\" + \"y\""));
    assert!(
        msgs[0].contains("does not concatenate strings") && msgs[0].contains("${"),
        "{msgs:?}"
    );
    // [op-arith] The operator is the call it names: a missing function is
    // said of the operator.
    let msgs = messages(&body("    let a = true * false"));
    assert!(
        msgs[0].contains("no `times` for `*` on `Bool`: `*` is `times(Bool, Bool)`"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = 1 - \"y\""));
    assert!(msgs[0].contains("no `minus` for `-` on `Int` and `Str`"), "{msgs:?}");
}

/// [type-byte] [op-arith] A `Byte` is not a number to the operators, and
/// the way through is the conversion pair — one call each way, which is
/// also what pins the wrapping (`to_byte` keeps the low 8 bits).
#[test]
fn bytes_compute_through_int() {
    let msgs = messages(&body("    let b = to_byte(1)\n    let a = b + b"));
    assert!(msgs[0].contains("no `plus` for `+` on `Byte`") && msgs[0].contains("`to_int`"), "{msgs:?}");
    assert_ok(&body(
        "    let b = to_byte(200)\n    let n = to_int(b) + 1\n    let c = to_byte(n)",
    ));
    // Equality needs no conversion: a `Byte` compares natively.
    assert_ok(&body(
        "    let b = to_byte(10)\n    let same = b == to_byte(10)",
    ));
}

// ===== arithmetic operators are calls [op-arith] [op-pow] (B3.3e) =====

/// The display of every declaration a call at some node went to.
fn callees(out: &salvo_core::Checked) -> Vec<String> {
    let mut v: Vec<String> = out.call_fn.values().map(|k| k.to_string()).collect();
    v.sort();
    v
}

/// [op-arith] [op-unique-shape] `+ - * /` on a type of your own are its `plus`,
/// `minus`, `times` and `divide`, declared in its file (O1, D-3: any return
/// type, any right operand), resolved as the written calls are.
#[test]
fn arithmetic_on_a_user_type_calls_its_functions() {
    let out = checked(
        r#"
struct V {
    x: Int
}

struct Scaled {
    v: V
}

fn plus(a: V, b: V) -> V {
    return V { x: a.x + b.x }
}

fn minus(a: V, b: V) -> V {
    return V { x: a.x - b.x }
}

fn times(a: V, k: Int) -> Scaled {
    return Scaled { v: V { x: a.x * k } }
}

fn divide(a: V, k: Int) -> V {
    return V { x: a.x / k }
}

fn probe(a: V, b: V) -> Scaled {
    let d = (a + b - a) / 2
    return d * 3
}
"#,
    );
    let errs: Vec<_> = out.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let shown = callees(&out);
    for want in ["plus(V, V) -> V@main", "minus(V, V) -> V@main", "times(V, Int) -> Scaled@main", "divide(V, Int) -> V@main"] {
        assert!(shown.iter().any(|s| s == want), "{want} among {shown:?}");
    }
    // A scalar's operator went to std's intrinsic.
    assert!(shown.iter().any(|s| s.starts_with("plus(Int, Int)")), "{shown:?}");
}

/// [op-arith] A missing function is said of the operator, with the remedy:
/// declare it in the type's own file [op-unique-shape].
#[test]
fn a_missing_arithmetic_function_names_the_operator() {
    let msgs = messages(
        "struct V {\n    x: Int\n}\n\nfn plus(a: V, b: V) -> V {\n    return V { x: a.x + b.x }\n}\n\n\
         fn probe(a: V) -> V {\n    return a + 1\n}\n",
    );
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(
        msgs[0].starts_with("no `plus` for `+` on `V` and `Int`: `+` is `plus(V, Int)`, and no `plus` in scope takes `(V, Int)`")
            && msgs[0].contains("in `V`'s own file"),
        "{msgs:?}"
    );
}

/// [op-arith] [implicit-resolve-body] The enclosing fn's own `?plus` takes
/// `+`, at a scalar too (R10 D1), as it takes the call `plus(a, b)`; at a
/// generic `T` it is the only way to add, and its absence is refused naming
/// it [implicit-forward].
#[test]
fn an_enclosing_plus_takes_the_operator() {
    let out = checked(
        "fn add(a: Int, b: Int, ?plus: (x: Int, y: Int) -> Int) -> Int {\n    return a + b\n}\n\n\
         fn twice<T>(a: T, ?plus: (x: T, y: T) -> T) -> T {\n    return a + a\n}\n\n\
         fn probe() -> Int {\n    return add(1, 2) + twice(3)\n}\n",
    );
    let errs: Vec<_> = out.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(out.local_calls.len(), 2, "both bodies' `+` go through their `?plus`");
    let msgs = messages("fn twice<T>(a: T) -> T {\n    return a + a\n}\n");
    assert!(
        msgs.len() == 1 && msgs[0].starts_with("no `plus` for `+` on `T`") && msgs[0].contains("`?plus: (a: T, b: T) -> T`"),
        "{msgs:?}"
    );
}

/// [op-arith] [effect-member-overload] R10 D3: an available effect's member
/// `times` takes `*`; beside a fn of its signature the operator is the
/// call's ambiguity, said of the operator.
#[test]
fn a_member_times_takes_the_operator_and_ties_with_a_fn() {
    let src = r#"
struct V {
    x: Int
}

effect Scale {
    fn times(a: V, b: V) -> V => a, b
}

fn probe(a: V, b: V) [Scale] -> V {
    return a * b
}
"#;
    let out = checked(src);
    let errs: Vec<_> = out.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(out.member_calls.len(), 1, "`a * b` is a call of the member");
    let tied = format!(
        "{src}\nfn times(a: V, b: V) -> V {{\n    return V {{ x: a.x * b.x }}\n}}\n\nfn call(a: V, b: V) [Scale] -> V {{\n    return times(a, b)\n}}\n"
    );
    let msgs = messages(&tied);
    assert_eq!(msgs.len(), 2, "the operator and the call, once each: {msgs:?}");
    assert!(msgs[0].starts_with("`*` on `V` is `times(V, V)`: "), "{msgs:?}");
    assert_eq!(msgs[0].trim_start_matches("`*` on `V` is `times(V, V)`: "), msgs[1], "the operator says what the call says");
}

/// [op-pow] `**` is `pow(a, b)` at the written types: std's `Int ** Int`,
/// `Long ** Int`, `Double ** Int` and `Double ** Double`, each typed by its
/// result; nothing widens, so `Long ** Long` and a `Float` base are refused
/// naming what std declares.
#[test]
fn power_on_the_numeric_scalars() {
    let out = checked(&body(
        "    let a: Int = 2 ** 10\n    let b: Long = 3L ** 2\n    let c: Double = 2.0 ** 3\n    \
         let d: Double = 2.0 ** 0.5\n    let e: Int = -2 ** 2 ** 3\n    let f: Double = 2.0 ** -1",
    ));
    let errs: Vec<_> = out.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let shown = callees(&out);
    for want in ["pow(Int, Int)", "pow(Long, Int)", "pow(Double, Int)", "pow(Double, Double)"] {
        assert!(shown.iter().any(|s| s.starts_with(want)), "{want} among {shown:?}");
    }
    let msgs = messages(&body("    let a = 2L ** 2L"));
    assert!(
        msgs.len() == 1 && msgs[0].starts_with("no `pow` for `**` on `Long`: `**` is `pow(Long, Long)`") && msgs[0].contains("(Long, Int)"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = 1.5f ** 2"));
    assert!(msgs.len() == 1 && msgs[0].starts_with("no `pow` for `**` on `Float` and `Int`"), "{msgs:?}");
}

/// [op-pow] [op-unique-shape] The brief's `Person`: `**` on a type of your own is
/// the `pow` its own file declares (here inside its body [fn-attached]),
/// with an `Int` exponent.
#[test]
fn power_on_a_user_type_is_its_pow() {
    let out = checked(
        r#"
struct Person {
    name: Str
    age: Int

    fn pow(p: Person, power: Int) -> Person {
        return Person { name: p.name, age: p.age ** power }
    }
}

fn probe(p: Person) -> Int {
    let q = p ** 2
    return q.age
}
"#,
    );
    let errs: Vec<_> = out.errors.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let shown = callees(&out);
    assert!(shown.iter().any(|s| s.starts_with("pow(Person, Int)")), "{shown:?}");
}

/// [op-arith] [backend-never-wrong] A type parameter is not lenient for the
/// operators that name no function either: `%`, unary `-` and `&&` on a `T`
/// passed the checker and both target compilers refused the code (rustc
/// E0369/E0600). Refused here, as `+` on a `T` without a `?plus` is.
#[test]
fn a_generic_operand_is_refused_where_no_function_is_named() {
    let msgs = messages("fn f<T>(a: T) -> T {\n    return a % a\n}\n");
    assert!(msgs.len() == 1 && msgs[0].contains("`%` needs numeric operands") && msgs[0].contains("`T`"), "{msgs:?}");
    let msgs = messages("fn g<T>(a: T) -> T {\n    return -a\n}\n");
    assert!(msgs.len() == 1 && msgs[0].contains("unary `-` needs a numeric operand"), "{msgs:?}");
    let msgs = messages("fn h<T>(a: T) -> Bool {\n    return a && true\n}\n");
    assert!(msgs.iter().any(|m| m.contains("`&&` needs `Bool` operands (found `T`)")), "{msgs:?}");
}

/// Unary `-` is numeric; unary `!` is `Bool`.
#[test]
fn unary_operands_are_checked() {
    assert_ok(&body("    let a = -1\n    let b = -2.5\n    let c = !true\n    let _d = !c"));
    let msgs = messages(&body("    let a = -true"));
    assert!(msgs[0].contains("unary `-` needs a numeric operand"), "{msgs:?}");
    let msgs = messages(&body("    let a = !5"));
    assert!(
        msgs[0].contains("`!` needs `Bool` operands") && msgs[0].contains("truthiness"),
        "{msgs:?}"
    );
}

// ===== logicals [op-bool] =====

/// `&&`/`||` take `Bool` on both sides, in value position too.
#[test]
fn logical_operands_are_bool_only() {
    assert_ok(&body("    let a = true && false\n    let _b = a || true"));
    let msgs = messages(&body("    let a = 1 && true"));
    assert_eq!(msgs.len(), 1, "one mistake, one diagnostic: {msgs:?}");
    assert!(
        msgs[0].contains("`&&` needs `Bool` operands (found `Int`)"),
        "{msgs:?}"
    );
}

// ===== ordering [op-order] =====

/// Ordering works on numerics, mixed widths included — the **native fast
/// path**, where the canonical implementation is the host's own operator
/// [op-order]. Everything else needs a `cmp`, and the refusal says which
/// function is missing and how to get it: this prelude declares none, so even
/// `Str` is refused here (the real `std.core.compare` has one, which is what makes
/// `"a" < "b"` legal and code-point ordered).
#[test]
fn ordering_surface() {
    assert_ok(&body(
        "    let a = 1 < 2\n    let b = 1 <= 2L\n    let c = 2.5 > 1.0f\n    let _d = a == b == c",
    ));
    let msgs = messages(&body("    let a = \"x\" < \"y\""));
    assert!(
        msgs[0].contains("no `cmp` for `<` on `Str`: `<` is `cmp(Str, Str)`") && msgs[0].contains("by auto"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = true < false"));
    assert!(msgs[0].contains("`cmp(Bool, Bool)`"), "{msgs:?}");
    // A struct needs its `cmp` like anything else, and the message names both
    // ways to get one [fn-attached] [obligation-by].
    let src = "struct P {\n    x: Int\n}\n\nfn probe(a: P, b: P) -> Bool => a, b {\n    return a < b\n}\n";
    let msgs = messages(src);
    assert!(
        msgs[0].contains("declare a `cmp` inside `P`'s body") && msgs[0].contains("by auto"),
        "{msgs:?}"
    );
}

// ===== literal adoption [type-lit-adopt] =====

/// An unsuffixed literal adopts the expected numeric type: `let`
/// annotations, negation, optionals' sole value arm. A suffix never
/// adopts, and a non-numeric expectation changes nothing.
#[test]
fn literals_adopt_the_expected_numeric_type() {
    assert_ok(&body(
        "    let a: Long = 1\n    let b: Double = 3\n    let c: Float = 0.5\n    \
         let d: Long = -5\n    let e: Long? = 1\n    let _f = a + 1 == a || b > 0.1 \
         || c > 0.1f || d < (e ?: unreachable!())",
    ));
    // Adopted types are recorded for the emitters.
    let out = checked(&body("    let a: Long = 1\n    let _b = a"));
    assert!(
        out.expr_ty
            .values()
            .any(|t| matches!(t, Ty::Named { name, .. } if name == "Long")),
        "the literal's checked type is Long"
    );
    // A suffixed literal stays what it says.
    let msgs = messages(&body("    let a: Long = 1.5"));
    assert!(
        msgs[0].contains("expected `Long`"),
        "a float literal does not adopt an integer type: {msgs:?}"
    );
    // Variables never widen implicitly — only literals adopt.
    let msgs = messages(&body("    let n = 1\n    let a: Long = n"));
    assert!(msgs[0].contains("expected `Long`, found `Int`"), "{msgs:?}");
}

/// [type-unknown-lenient] Unknown operands stay lenient: one mistake, one
/// diagnostic, no cascade through the operators.
#[test]
fn unknown_operands_stay_lenient() {
    let msgs = messages(&body("    let a = missing()\n    let b = a + 1\n    let c = b < 2"));
    assert_eq!(msgs.len(), 1, "only the unresolved call reports: {msgs:?}");
}

// ===== [op-compound] `+=` `-=` `*=` `/=` =====

/// Pure desugar (user decision 2026-09-22, the heap plan's item 6): `x += e` is
/// `x = x + e`, so the arithmetic rules, the place rules and the narrowing reset
/// are the ones the two halves already have — nothing in the checker or the
/// emitters knows the spelling exists.
#[test]
fn compound_assignment_is_the_arithmetic_it_desugars_to() {
    assert_ok(&body(
        "    let n = 1\n    n += 2\n    n -= 1\n    n *= 3\n    n /= 2",
    ));
}

/// …which means its operands are checked like any other arithmetic: no `Str +`
/// [op-arith], so `+=` on a string is refused with the same message, naming
/// interpolation.
#[test]
fn compound_assignment_inherits_the_no_string_plus_rule() {
    let msgs = messages(&body("    let s = \"a\"\n    s += \"b\""));
    assert!(
        msgs.iter().any(|m| m.contains("does not concatenate strings")),
        "got {msgs:?}"
    );
}

/// And across the numeric classes it is the same explicit-conversion error.
#[test]
fn compound_assignment_inherits_the_class_rule() {
    let msgs = messages(&body("    let n = 1\n    n += to_double(2)"));
    assert!(
        !msgs.is_empty(),
        "an int-float mix should be refused, got {msgs:?}"
    );
}

/// [op-assign] A target that is not a place is one mistake, reported once:
/// no permission or fate check runs on what it would have written (found
/// 2026-10-10: `h!.hp = 40` on an optional handle also said the handle was a
/// read-only projection).
#[test]
fn a_non_place_target_is_reported_once() {
    let msgs = messages(
        "struct E { hp: Int }\n\n\
         fn make() -> E {\n    return E { hp: 1 }\n}\n\n\
         fn probe() -> None {\n    make().hp = 2\n    return None\n}\n",
    );
    assert_eq!(msgs.len(), 1, "got {msgs:?}");
    assert!(msgs[0].contains("this is not a place"), "got {msgs:?}");
}

/// [op-assign] A target must be a **place**. This was accepted before 2026-09-22
/// and then rejected by the target compiler (rustc's E0070) — a
/// [backend-never-wrong] violation `+=` would have inherited.
#[test]
fn a_call_is_not_an_assignment_target() {
    let msgs = messages(
        "fn size_of(s: Str) -> Int => s {\n    return 1\n}\n\n\
         fn probe(s: Str) -> None => s {\n    size_of(s) = 2\n    return None\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("this is not a place")),
        "got {msgs:?}"
    );
    // The compound spelling is the same statement, so it is refused too.
    let msgs = messages(
        "fn size_of(s: Str) -> Int => s {\n    return 1\n}\n\n\
         fn probe(s: Str) -> None => s {\n    size_of(s) += 2\n    return None\n}\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("this is not a place")),
        "got {msgs:?}"
    );
}
