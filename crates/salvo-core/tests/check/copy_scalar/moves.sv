// [copy-scalar-free] Moving a Copy scalar is a copy (user decision
// 2026-10-20, ROADMAP §2d item 12): every consuming position — a call's
// moved parameter, a literal store, a spread — leaves an `Int` usable,
// judged by the argument's own type, so a generic `T` bound to an `Int`
// moves for free too. A written `!v` on a concrete scalar parameter warns:
// it says nothing.

// A generic moved parameter called with each Copy scalar, twice: the
// argument stays usable, because its own type is a scalar; and a generic
// `!v` says something, so it does not warn.
fn take<T>(v: T) [] -> None => !v {
}

fn each_scalar(i: Int, l: Long, b: Bool, c: Char, d: Double) [] -> Int => i, l, b, c, d {
    take(i)
    take(i)
    take(l)
    take(l)
    take(b)
    take(b)
    take(c)
    take(c)
    take(d)
    take(d)
    let x: Int = 1
    take(x)
    return x + i
}

// A literal store and a written `!v` on an `Int` parameter leave the value
// usable; the `!v` warns that it changes nothing.
struct P {
    x: Int
}

//~ warning[copy-scalar-free:moved-scalar]
fn eat_int(v: Int) [] -> None => !v {
}

fn stores(n: Int) [] -> Int => n {
    let p = P { x: n }
    let q = P { x: n }
    eat_int(n)
    eat_int(n)
    return n + p.x + q.x
}

// Anything that is not a Copy scalar is still consumed: a `Str` handed to a
// moved parameter cannot be read afterwards.
fn eat<T>(v: T) [] -> None => !v {
}

fn strs(s: Str) [] -> None => !s {
    eat(s)
    //~ error[flow-use-after-move:consumed]
    eat(s)
}
