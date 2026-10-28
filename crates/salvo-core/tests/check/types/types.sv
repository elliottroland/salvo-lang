// Types the checker refuses: a value no arm of its union takes, and `Mut` on
// a type that cannot be mutable.

//--- union_wrap_requires_matching_arm
// [type-union-arm-identity]
export qualifier Ok<T> of T
export qualifier Err<T> of T

//~ error[deduce-default:kept-by-default-moved]
export fn as_err<T>(value: T) -> +Err T {
    return value
}

export fn f() -> Ok Int | Err Str {
    //~ error[fn-syntax:return-mismatch]
    //~ error[type-union:no-arm]
    return as_err(true)
}

//--- mut_requires_a_with_mut_declaration
// [type-canbe-mut] `Mut` only applies to declarations that say `canbe Mut`.
// (`Str` does say so; `Int` is the primitive that never can.)
//~ error[struct-mut:not-canbe-mut]
fn f(x: Mut Int) -> None {
}
