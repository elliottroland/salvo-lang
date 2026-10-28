// [obligation-by] [comptime-instantiate] A template's effect list and
// deductions do not count: the group's member is pure and keeps its
// parameter, so a body that prints, or moves the parameter, is an error of
// the stamped body — at the `by` site, with the stamp's prefix.
//
// A derive whose template declares what its group's member does not: an
// effect (`Console`), and a move of its parameter (`=> !v`).
// FILE: derive.sv
export fns Loud<T> {
    fn loud(v: T) -> Int
}

export fns Taken<T> {
    fn take(v: T) -> T
}

export comptime fn loud<T is Struct>(v: T) [Console] -> Int => v {
    print("${T.name}")
    return 0
}

export comptime fn take<T is Struct>(v: T) [] -> T => !v {
    return v
}

// FILE: main.sv
import derive

//~ error[deduce-default:kept-by-default-moved]
//~ error[effect-available:no-handler]
struct P : Loud<self> by derive, Taken<self> by derive { x: Int, y: Str }

// [fn-by] The same template under a written declaration that says what the
// body does is accepted: the declaration is the contract.
struct R : Loud<self>, Taken<self> {
    x: Int,
    y: Str

    fn loud(v: R) [Console] -> Int by derive
    fn take(v: R) -> R => !v by derive
}
