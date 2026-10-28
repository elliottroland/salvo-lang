// A read before a mutation in one expression is hoisted [rs-mut-arg-hoist].

//--- a_read_before_a_mutation_is_hoisted_out_of_the_expression
// [rs-mut-arg-hoist] A read that sits *before* a mutation in one expression:
// Rust holds the read's borrow for the whole expression (E0502) where the
// language only orders the two (`arguments are evaluated left to right`,
// [deduce-same-call]), so the read is hoisted into a `let` in front of it.
// Both shapes: a `format!` part (which borrows every argument) and a call
// argument in a borrowed position. Closes the defect filed 2026-09-25.
struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int => b: Mut {
    b.n = b.n + 1
    return b.n
}

fn label(s: Str, n: Int) -> Str => s, n {
    return "${s}/${n}"
}

fn main() [use] {
    use StdConsole()
    let b = Mut Box { n: 1, tag: "t" }
    // An interpolated place read, then a call that mutates it.
    // The interpolation reads the local, not the place.
    //~ emits: { let __part3 = b.n; let __part4 = crate::bumped(&mut b); format!("1. {} {}", __part3, __part4) }
    println("1. ${b.n} ${bumped(b)}")
    // The same across a call's arguments, where the read is a borrow.
    // The call argument does too — owned here, since a borrow is what
    // collided; `&__arg9` is what the position takes.
    //~ emits: { let __arg9 = b.tag.clone(); let __arg10 = crate::bumped(&mut b); crate::label(&__arg9, __arg10) }
    println("2. ${label(b.tag, bumped(b))}")
    // The other order needs nothing: the mutation is over before the read.
    // …and the read *after* the mutation is read after it: it must see the
    // new value, on both backends.
    //~ emits: { let __part15 = crate::bumped(&mut b); let __part16 = b.n; format!("3. {} {}", __part15, __part16) }
    println("3. ${bumped(b)} ${b.n}")
}
