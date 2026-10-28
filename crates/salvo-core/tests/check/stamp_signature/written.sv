// [fn-by] A written declaration keeps its own parameter names and its own
// deduction clause. `std.core.auto`'s `hash` names its parameter `value` and
// declares a local `h`: the declared `h` is the parameter, and the template's
// local is renamed apart (`h__p`, `stamp_signature_tests.rs`) so it cannot
// capture it — nor be refused as shadowing it [var-no-shadow].
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

// [fn-by] [deduce-default] The declared contract governs, not the
// template's: `std.core.auto`'s `eq` keeps both parameters (`=> a, b`), but a
// declaration that consumes its first (`=> !a`) consumes the caller's
// argument.
struct Q : Eq<self> {
    x: Int,
    y: Str

    fn eq(a: Q, b: Q) -> Bool => !a by auto
}

fn consumed(p: Q, q: Q) [] -> Bool => !p {
    let _same = eq(p, q)
    //~ error[flow-use-after-move:consumed]
    return eq(p, q)
}

// Kept by the default, as the template's clause also says: no move.
struct R : Eq<self> {
    x: Int,
    y: Str

    fn eq(a: R, b: R) -> Bool by auto
}

fn kept(p: R, q: R) [] -> Bool => !p {
    let _same = eq(p, q)
    return eq(p, q)
}
