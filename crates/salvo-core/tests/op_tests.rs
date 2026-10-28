//! [op-arith] [op-promote] [op-pow] [type-lit-adopt] What the checker records
//! about operators, beyond their diagnostics: the promotions the backends
//! widen by, and which function each operator went to. The diagnostics are
//! test data, `tests/check/operators/` (TESTS.md T1); these read the
//! checker's tables, which annotations cannot.

use salvo_core::Ty;

/// `src` checked over the real std, asserting it reports no error, with
/// only the rows of its own file kept (std's are seeded into every check).
fn checked(src: &str) -> salvo_core::Checked {
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    let user = out.program.files.iter().position(|f| !f.is_std).expect("main.sv");
    let mut c = out.checked;
    c.expr_ty.retain(|k, _| k.file() >= user);
    c.promotions.retain(|k, _| k.file() >= user);
    c.call_fn.retain(|k, _| k.file() >= user);
    c.member_calls.retain(|k, _| k.file() >= user);
    c.local_calls.retain(|k| k.file() >= user);
    c
}

fn body(stmts: &str) -> String {
    format!("fn probe() -> None {{\n{stmts}\n    return None\n}}\n")
}

/// The display of every declaration a call at some node went to.
fn callees(out: &salvo_core::Checked) -> Vec<String> {
    let mut v: Vec<String> = out.call_fn.values().map(|k| k.to_string()).collect();
    v.sort();
    v
}

/// [op-promote] `Int + Long` widens to `Long`, both directions, and the
/// promotion lands on the narrower operand's span. Same for the float
/// class. The result is the promoted type.
#[test]
fn integer_widening_promotes_and_records() {
    let out = checked(&body(
        "    let x: Long = 9L\n    let n = 5\n    let _y = x + n\n    let _z = n + x\n    \
         let f: Float = 1.0f\n    let d = 2.5\n    let _g = f * d",
    ));
    let promoted: Vec<String> = out.promotions.values().map(|t| t.to_string()).collect();
    assert_eq!(promoted.iter().filter(|t| *t == "Long").count(), 2, "both `n` operands promote to Long: {promoted:?}");
    assert_eq!(promoted.iter().filter(|t| *t == "Double").count(), 1, "the `f` operand promotes to Double: {promoted:?}");
    let types: Vec<String> = out.expr_ty.values().map(|t| t.to_string()).collect();
    assert!(types.iter().any(|t| t == "Long"), "{types:?}");
}

/// [op-arith] [op-unique-shape] `+ - * /` on a type of your own are its
/// `plus`, `minus`, `times` and `divide`; a scalar's are std's intrinsics.
#[test]
fn arithmetic_on_a_user_type_calls_its_functions() {
    let out = checked(
        "struct V {\n    x: Int\n}\n\nstruct Scaled {\n    v: V\n}\n\n\
         fn plus(a: V, b: V) -> V {\n    return V { x: a.x + b.x }\n}\n\n\
         fn minus(a: V, b: V) -> V {\n    return V { x: a.x - b.x }\n}\n\n\
         fn times(a: V, k: Int) -> Scaled {\n    return Scaled { v: V { x: a.x * k } }\n}\n\n\
         fn divide(a: V, k: Int) -> V {\n    return V { x: a.x / k }\n}\n\n\
         fn probe(a: V, b: V) -> Scaled {\n    let d = (a + b - a) / 2\n    return d * 3\n}\n",
    );
    let shown = callees(&out);
    for want in ["plus(V, V) -> V@main", "minus(V, V) -> V@main", "times(V, Int) -> Scaled@main", "divide(V, Int) -> V@main"] {
        assert!(shown.iter().any(|s| s == want), "{want} among {shown:?}");
    }
    assert!(shown.iter().any(|s| s.starts_with("plus(Int, Int)")), "{shown:?}");
}

/// [op-arith] [implicit-resolve-body] The enclosing fn's own `?plus` takes
/// `+`, at a scalar too (R10 D1), as it takes the call `plus(a, b)`.
#[test]
fn an_enclosing_plus_takes_the_operator() {
    let out = checked(
        "fn add(a: Int, b: Int, ?plus: (x: Int, y: Int) -> Int) -> Int {\n    return a + b\n}\n\n\
         fn twice<T>(a: T, ?plus: (x: T, y: T) -> T) -> T {\n    return a + a\n}\n\n\
         fn probe() -> Int {\n    return add(1, 2) + twice(3)\n}\n",
    );
    assert_eq!(out.local_calls.len(), 2, "both bodies' `+` go through their `?plus`");
}

/// [op-arith] [effect-member-overload] R10 D3: an available effect's member
/// `times` takes `*`.
#[test]
fn a_member_times_takes_the_operator() {
    let out = checked(
        "struct V {\n    x: Int\n}\n\neffect Scale {\n    fn times(a: V, b: V) -> V => a, b\n}\n\n\
         fn probe(a: V, b: V) [Scale] -> V {\n    return a * b\n}\n",
    );
    assert_eq!(out.member_calls.len(), 1, "`a * b` is a call of the member");
}

/// [op-pow] `**` is `pow(a, b)` at the written types, std's four.
#[test]
fn power_goes_to_stds_pow() {
    let out = checked(&body(
        "    let _a: Int = 2 ** 10\n    let _b: Long = 3L ** 2\n    let _c: Double = 2.0 ** 3\n    \
         let _d: Double = 2.0 ** 0.5",
    ));
    let shown = callees(&out);
    for want in ["pow(Int, Int)", "pow(Long, Int)", "pow(Double, Int)", "pow(Double, Double)"] {
        assert!(shown.iter().any(|s| s.starts_with(want)), "{want} among {shown:?}");
    }
}

/// [op-pow] [fn-attached] `**` on a type of your own is the `pow` its own
/// body declares.
#[test]
fn power_on_a_user_type_is_its_pow() {
    let out = checked(
        "struct Person {\n    name: Str\n    age: Int\n\n    \
         fn pow(p: Person, power: Int) -> Person {\n        return Person { name: p.name, age: p.age ** power }\n    }\n}\n\n\
         fn probe(p: Person) -> Int {\n    let q = p ** 2\n    return q.age\n}\n",
    );
    assert!(callees(&out).iter().any(|s| s.starts_with("pow(Person, Int)")), "{:?}", callees(&out));
}

/// [type-lit-adopt] An adopted literal's type is recorded for the emitters.
#[test]
fn an_adopted_literal_records_its_type() {
    let out = checked(&body("    let a: Long = 1\n    let _b = a"));
    assert!(
        out.expr_ty.values().any(|t| matches!(t, Ty::Named { name, .. } if name == "Long")),
        "the literal's checked type is Long"
    );
}
