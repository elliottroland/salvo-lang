// [str-drop-mut] [type-canbe-mut] `Str canbe Mut`, and what it refuses. The
// `Mut` drops the checker records at every site a `Mut Str` flows into a
// plain `Str` are `str_tests.rs`'s (a table).

// [type-canbe-mut] `Str canbe Mut`, so `Mut Str` is a type. (That `Mut Int`
// still is not stays `str_tests.rs`'s: the refusal also reports the
// annotation mismatch, a second diagnostic for one mistake.)
fn opts_into_mut() -> None {
    let _b: Mut Str = Mut Str()
}

// A `Str` literal is not a `Mut Str`: mutability is asked for, which is what
// makes `Mut Str()` mirror `Mut List`.
fn a_literal_is_not_a_builder() -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _b: Mut Str = "plain"
}

// [op-arith] `+` on strings is refused outright (user decision 2026-09-14):
// interpolation is the concatenation story, and the error says so.
fn no_concatenation() -> None {
    let x = Mut Str()
    //~ error[call-resolve:no-overload]
    let _joined = x + x
}

// ===== interpolation renderability [interp-to-str] [interp-struct] =====

// [interp-to-str] [backend-never-wrong] A value with no text form used to
// pass the checker and become rustc's "doesn't implement Display". It is a
// Salvo error now, naming the `to_str` that would fix it.
struct Inner { n: Int }
struct Opaque { inner: Inner }

fn no_text_form(o: Opaque) -> None => o {
    //~ error[interp-to-str:no-text]
    let _s = "${o}"
}

// [interp-union] A union is interpolated by the `to_str` of the arm it holds,
// so every arm needs a text form: the scalars and `Str` have one, a struct
// needs a `to_str` of its own, and the error names the arm.
struct Bare { n: Int }

fn union_arm_without_text(u: Int | Bare) -> None => u {
    //~ error[interp-union:arm-no-text]
    let _s = "${u}"
}

// [interp-to-str] A `to_str` in scope is resolved *at the interpolation
// site* (user decision 2026-09-11), so declaring one is all it takes — for a
// union arm and a plain value alike.
struct Shown { inner: Inner }

fn to_str(o: Shown) [] -> Str => o { return "op" }

fn union_arms_with_text(u: Int | Shown | Str) -> None => u {
    let _s = "${u}"
}

fn a_to_str_in_scope(o: Shown) -> None => o {
    let _s = "${o}"
}

// [interp-to-str] [obligation-by] A struct interpolates by **opting in**
// (user decision 2026-09-28, replacing the default-on derivation): with no
// `to_str`, the diagnostic names the clause that stamps the field-wise one.
struct Person { name: Str, age: Int }

fn native_fields(p: Person) -> None => p {
    //~ error[interp-to-str:no-text]
    let _s = "${p}"
}

// [interp-to-str] A struct holding another struct has no text form either
// without a declaration; the diagnostic asks for one.
struct Outer { inner: Inner }

fn non_native_field(o: Outer) -> None => o {
    //~ error[interp-to-str:no-text]
    let _s = "${o}"
}
