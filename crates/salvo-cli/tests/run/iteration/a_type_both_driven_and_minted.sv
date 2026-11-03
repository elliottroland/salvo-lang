// [iter-for] [iter-step-call] A type both a `next` and an `iter` take is
// accepted; `for x in it` over it is ambiguous, so the loop names the call
// it means (user decision 2026-10-30): a step call drives it, `iter` mints
// from it.
//
// Compile-and-run data ([test-run-data]).

//--- both_forms
struct Letters canbe Mut {
    at: Int
}

struct Count canbe Mut {
    left: Int
}

fn next(l: Mut Letters) -> Emitted Str | Finished => l: Mut {
    if l.at >= 2 {
        return finished()
    }
    l.at = l.at + 1
    return emitted("x${l.at}")
}

fn iter(l: Letters) -> Mut Count => l {
    return Mut Count { left: 3 }
}

fn next(c: Mut Count) -> Emitted Int | Finished => c: Mut {
    if c.left == 0 {
        return finished()
    }
    c.left = c.left - 1
    return emitted(c.left)
}

fn main() [use] {
    use StdConsole()
    let l = Mut Letters { at: 0 }
    for s in iter(l) {
        println("${s}")
    }
    for s in next(l) {
        println(s)
    }
}
//= stdout
//= 2
//= 1
//= 0
//= x1
//= x2
