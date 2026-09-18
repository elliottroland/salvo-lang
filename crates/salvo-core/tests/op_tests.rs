//! [op-arith] [op-order] [op-bool] [op-promote] [lit-adopt] The operator
//! typing slice (user decision 2026-09-14, FILE_SYSTEM.md §5.10.3):
//! arithmetic on numeric operands only, implicit widening *within* the
//! integer and float classes with the promotion recorded for the backends,
//! an explicit-conversion error *across* them, no `Str +`, `Bool`-only
//! logicals, ordering on numerics and `canbe ordered` structs, and
//! unsuffixed literals adopting the expected numeric type.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols, Ty};

/// [intrinsic-std-only] The declarations these sources rely on, loaded as a
/// *std* file (only std may write `intrinsic`).
const STD_PRELUDE: &str = concat!(
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
);

fn checked(src: &str) -> salvo_core::Checked {
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
/// not operator-numeric [byte-value]: it is an octet, and its arithmetic
/// goes through `to_int`/`to_byte`.
#[test]
fn non_numeric_arithmetic_is_refused() {
    let msgs = messages(&body("    let a = \"x\" + \"y\""));
    assert!(
        msgs[0].contains("does not concatenate strings") && msgs[0].contains("${"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = true * false"));
    assert!(
        msgs[0].contains("needs numeric operands") && msgs[0].contains("`Bool`"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = 1 - \"y\""));
    assert!(msgs[0].contains("needs numeric operands"), "{msgs:?}");
}

/// [byte-value] [op-arith] A `Byte` is not a number to the operators, and
/// the way through is the conversion pair — one call each way, which is
/// also what pins the wrapping (`to_byte` keeps the low 8 bits).
#[test]
fn bytes_compute_through_int() {
    let msgs = messages(&body("    let b = to_byte(1)\n    let a = b + b"));
    assert!(msgs[0].contains("needs numeric operands"), "{msgs:?}");
    assert_ok(&body(
        "    let b = to_byte(200)\n    let n = to_int(b) + 1\n    let c = to_byte(n)",
    ));
    // Equality needs no conversion: a `Byte` compares natively.
    assert_ok(&body(
        "    let b = to_byte(10)\n    let same = b == to_byte(10)",
    ));
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

/// Ordering works on numerics — mixed widths included — and on `canbe
/// ordered` structs; everything else is refused with the rule spelled out.
#[test]
fn ordering_surface() {
    assert_ok(&body(
        "    let a = 1 < 2\n    let b = 1 <= 2L\n    let c = 2.5 > 1.0f\n    let _d = a == b == c",
    ));
    let msgs = messages(&body("    let a = \"x\" < \"y\""));
    assert!(
        msgs[0].contains("`Str` cannot be ordered") && msgs[0].contains("canbe ordered"),
        "{msgs:?}"
    );
    let msgs = messages(&body("    let a = true < false"));
    assert!(msgs[0].contains("`Bool` cannot be ordered"), "{msgs:?}");
    // Struct ordering still needs the opt-in (the equality slice's rule,
    // unchanged).
    let src = "struct P {\n    x: Int\n}\n\nfn probe(a: P, b: P) -> Bool => a, b {\n    return a < b\n}\n";
    let msgs = messages(src);
    assert!(msgs[0].contains("canbe ordered"), "{msgs:?}");
}

// ===== literal adoption [lit-adopt] =====

/// An unsuffixed literal adopts the expected numeric type: `let`
/// annotations, negation, optionals' sole value arm. A suffix never
/// adopts, and a non-numeric expectation changes nothing.
#[test]
fn literals_adopt_the_expected_numeric_type() {
    assert_ok(&body(
        "    let a: Long = 1\n    let b: Double = 3\n    let c: Float = 0.5\n    \
         let d: Long = -5\n    let e: Long? = 1\n    let _f = a + 1 == a || b > 0.1 \
         || c > 0.1f || d < e!",
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
