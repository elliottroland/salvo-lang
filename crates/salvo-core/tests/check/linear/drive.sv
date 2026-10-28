// [iter-generic-drive] The release a generic drive owes. The protocol and a
// linear pass: a `Handle` that declares both obligations, plus the `close`
// that discharges its linearity.

linear struct Handle : Next<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) [] -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) [] -> None => !h { discard(h) }

// [linear-generics] [linear-group] A generic pass the fn **owns** —
// `<It canbe linear>`, `=> !it` — may be carrying a resource, and the loop is
// *not* a discharge site (user decision 2026-09-12): driving it leaves the
// obligation live, so a body that ends without terminating it leaks.
//~ error[ir-linear-check:leak-at-exit]
fn owning_a_possibly_linear_pass_still_owes_after_the_loop<It canbe linear>(it: Mut It, ?Next<It, Int>) [] -> Int => !it {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

// The consuming-callback pattern replaces the `?Linear<It>` spread (user
// decision 2026-09-12): the fn takes `end: (x: It) -> None` with a consuming
// contract, drives the pass in place, and hands it to `end` — an explicit
// discharge on the one path out.
fn drive<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Next<It, Int>) [] -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    end(it)
    return sum
}

// [iter-drive-in-place] A pass the fn **keeps** needs nothing: the obligation
// stayed with the caller, the loop advances the pass where it lives, and
// closing it here would be the caller's use-after-close.
fn keeping_the_pass_leaves_the_release_to_the_caller<It canbe linear>(it: Mut It, ?Next<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

// And a pass that is *not* opted in needs no release either: a non-linear
// pass may be abandoned, which is the same latitude a hand-written `while`
// has [linear-group].
fn a_plain_generic_pass_needs_no_release<It>(it: Mut It, ?Next<It, Int>) [] -> Int => !it {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

// [fn-value-select] [linear-generics] A generic fn passed *by name*
// instantiates from the position's expected fn type (closed 2026-09-12):
// std's `drop` fills a consuming-callback position for a plain pass — and
// the instantiation ban still refuses it for a linear one.
struct Counter : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Counter) [] -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn a_generic_fn_value_instantiates_from_the_position() -> Int {
    let c = Mut Counter { at: 3 }
    return drive(c, drop)
}

fn a_generic_fn_value_still_refuses_a_linear_instantiation() -> Int {
    let h = open_handle(3)
    //~ error[linear-generics:fn-value]
    return drive(h, drop)
}
