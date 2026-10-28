// [is-bind-once] An `is` **binding** reads its subject twice — once for the
// test, once for the payload — so a subject with side effects has to be
// evaluated exactly once and shared.
//
// This was a defect until 2026-09-16, and the worst kind: both backends
// emitted the subject twice, so
//
//     while remove_first(queue) is Ticket next { redeem(next) }
//
// called `remove_first` twice per turn and silently discarded every other
// element — with a *linear* element, its obligation went with it, and the
// Rust arm cloned the value the checker believed it had moved. Never
// reported anything.
//
// The rule now: a binding `is` whose subject is **not a place** is legal as
// the whole condition of a `while` or of an `if`'s first branch, where the
// emitters hoist it into one temporary that the test and the binding share;
// anywhere else it is refused, because the alternative — hoisting out of a
// `&&` chain or an `elif` — would evaluate a subject that short-circuiting
// says should not run.

// A subject with a side effect, and a place to compare against.
struct Box canbe Mut { n: Int }

fn pull(b: Mut Box) [] -> Int? => b: Mut {
    b.n = 1
    return copy(b.n)
}

// [is-bind-once] The two hoistable shapes: a `while` condition and an `if`'s
// first branch. These are the forms a program actually wants — the drain loop
// above all — so they stay legal and the emitters make the single evaluation
// true.
fn whole_condition(b: Mut Box) [] -> Int => b: Mut {
    while pull(b) is Int v {
        return v
    }
    if pull(b) is Int w {
        return w
    }
    return 0
}

// [is-bind-once] Inside a `&&` chain it is refused: hoisting would evaluate
// the subject even where short-circuiting says it must not run, and *not*
// hoisting is the defect this rule closed.
fn inside_a_chain(b: Mut Box) [] -> Int => b: Mut {
    //~ error[is-bind-once:not-hoistable]
    if pull(b) is Int v && v > 1 {
        return v
    }
    return 0
}

// [is-bind-once] …and in an `elif` condition, where there is no statement
// position for the temporary. The diagnostic names the remedy, which is the
// `let` the author would have written anyway.
fn in_an_elif(b: Mut Box, flag: Bool) [] -> Int => b: Mut, flag {
    if flag {
        return 0
    //~ error[is-bind-once:not-hoistable]
    } elif pull(b) is Int v {
        return v
    }
    return 0
}

// [is-bind-once] A **place** subject is unaffected anywhere: reading a
// variable or a field chain twice is free, so the old rendering stands and
// none of the positions above are restricted for it.
struct Holder { maybe: Int? }

fn place_subject(h: Holder, o: Int?, flag: Bool) [] -> Int => h, o, flag {
    if o is Int a && a > 1 {
        return a
    } elif h.maybe is Int b {
        return b
    }
    while o is Int c {
        return c
    }
    return 0
}

// [is-bind-once] A subject *without* a binding is read once — by the test —
// so it needs no hoisting and no restriction.
fn without_a_binding(b: Mut Box, flag: Bool) [] -> Int => b: Mut, flag {
    if flag && pull(b) is Int {
        return 1
    }
    return 0
}
