// [fn-variadic] mixing plain arguments with a `...spread`.

//--- mixed_tail_assembled_once
// The shape: a mixed tail is **assembled** into one vector in written order,
// and — because that vector is fresh — it is *not* cloned again by the
// constructor, which a borrowed forward would be (`parts.clone()`, asserted
// in `a_spread_into_a_variadic_intrinsic_is_the_collection`).
export fn main() [use] {
    use StdConsole()
    let rest = array_of(2, 3)
    // [col-of-nonempty] The list constructors take their leading elements as
    // real parameters now, so the lowering starts from `vec![…]` and extends
    // with the tail rather than pushing each leading element.
    //~ emits: { let mut __v = vec![1i32]; __v.extend(rest.clone().into_iter()); __v }
    // An assembled vector is already owned and must not be cloned again.
    //~ !emits: __v }.clone()
    let all = List(1, ...rest)
    println("${size(all)}")
}
