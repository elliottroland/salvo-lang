// Assigning to a narrowed place [flow-place-invalidate].

//--- a_narrowed_field_is_assigned_at_its_declared_type
// Under `if p.surname is Str`, `p.surname = None` writes the field's declared
// `Str?`: the assignment ends the narrowing, as it does for a local. Found
// 2026-10-28: the assignment was refused as `None` into `Str`.
struct P canbe Mut {
    name: Str,
    surname: Str? = None
}

fn forget(p: Mut P) -> Str => p: Mut {
    if p.surname is Str {
        let was = "had ${p.surname}"
        p.surname = None
        return was
    }
    return "had none"
}

fn main() [use] -> None {
    use StdConsole()
    let p = Mut P { name: "Ada", surname: "Lovelace" }
    println(forget(p))
    println(forget(p))
    println("${p.surname ?: "gone"}")
}
//= stdout
//= had Lovelace
//= had none
//= gone

//--- a_narrowed_local_written_in_its_branch_holds_the_write_after_it
// [ir-narrow] A write to a local narrowed from an optional (a step, `=`,
// `+=`) goes to the local, not to a copy the narrowing took: inside the
// branch it reads the new value, and after the branch the local holds it.
// Found 2026-11-11: Kotlin assigned the narrowing's `val`, which kotlinc
// refused, and both backends read the value from before the write after
// the branch (and, under a loop, on the next turn).
fn start() [] -> Int? {
    return 12
}

fn main() [use] -> None {
    use StdConsole()
    let o = start()
    if o is Int {
        o++
        println("step in: ${o}")
    }
    println("step after: ${o ?: -1}")
    let p = start()
    if p is Int {
        p = 5
    }
    println("assign after: ${p ?: -1}")
    let q = start()
    if q is Int {
        q += 3
        println("compound in: ${q}")
    }
    println("compound after: ${q ?: -1}")
    let v = start()
    if v is Int {
        let old = v++
        let new = ++v
        println("values: ${old} ${new} ${v}")
    }
    println("values after: ${v ?: -1}")
    let l: Long? = 5L
    if l is Long {
        l--
    }
    println("long after: ${l ?: -1L}")
    // An `is` binding is a local of its own: stepping it leaves the
    // subject as it was.
    let b = start()
    if b is Int k {
        k++
        k += 2
        println("binding: ${k} ${b}")
    }
    when b {
        is Int k {
            k = 3
            println("when binding: ${k} ${b}")
        }
        is None {
            println("none")
        }
    }
}
//= stdout
//= step in: 13
//= step after: 13
//= assign after: 5
//= compound in: 15
//= compound after: 15
//= values: 12 14 14
//= values after: 14
//= long after: 4
//= binding: 15 12
//= when binding: 3 12

//--- a_narrowed_local_written_under_a_nested_branch_or_a_loop
// [ir-narrow] The write may sit under an inner `if` or in a loop: a read
// after the inner branch, or on a loop's next turn, reads the local again.
fn start() [] -> Int? {
    return 12
}

fn name() [] -> Str? {
    return "a"
}

fn main() [use] -> None {
    use StdConsole()
    let q = start()
    if q is Int {
        if q > 0 {
            q = 5
        }
        println("nested: ${q ?: -1}")
    }
    let w = start()
    if w is Int {
        let n = 0
        while n < 3 {
            w = w + 1
            n++
        }
        println("while: ${w ?: -1}")
    }
    println("while after: ${w ?: -1}")
    let f = start()
    if f is Int {
        for x in List(1, 2, 3) {
            f += x
            f--
        }
        println("for: ${f ?: -1}")
    }
    let s = name()
    if s is Str {
        println("first: ${s}")
        s = "bc"
        println("then: ${s}")
    }
    let t = name()
    if t is Str {
        let n = 0
        while n < 2 {
            t = "${t}x"
            n++
        }
    }
    println("after: ${s ?: "-"} ${t ?: "-"}")
}
//= stdout
//= nested: 5
//= while: 15
//= while after: 15
//= for: 15
//= first: a
//= then: bc
//= after: bc axx

//--- a_narrowed_field_struct_or_when_arm_is_stepped
// [ir-narrow] [rs-narrow-mut] A narrowed field is stepped in place; a
// struct narrowed out of an optional is written through (on Rust, a `&mut`
// to the storage's payload); a `when` arm and a wrapper union's arm write
// the subject.
struct Counter canbe Mut {
    n: Int?
}

struct Inner canbe Mut {
    n: Int
}

fn start() [] -> Int? {
    return 12
}

fn either(flag: Bool) [] -> Int | Str {
    if flag {
        return 3
    }
    return "s"
}

fn inner() [] -> Mut Inner? {
    return Mut Inner { n: 1 }
}

fn main() [use] -> None {
    use StdConsole()
    let c = Mut Counter { n: 7 }
    if c.n is Int {
        c.n++
        println("field in: ${c.n ?: -1}")
    }
    println("field after: ${c.n ?: -1}")
    let p = inner()
    if p is Inner {
        p.n++
        p.n = p.n * 10
        println("struct in: ${p.n}")
    }
    if p is Inner k {
        k.n++
    }
    if p is Inner {
        println("struct after: ${p.n}")
    }
    let w = start()
    when w {
        is Int {
            w++
            w += 2
            println("when in: ${w}")
        }
        is None {
            println("none")
        }
    }
    println("when after: ${w ?: -1}")
    let u = either(true)
    if u is Int {
        ++u
    }
    when u {
        is Int {
            println("union after: ${u}")
        }
        is Str {
            println("union after: str")
        }
    }
}
//= stdout
//= field in: 8
//= field after: 8
//= struct in: 20
//= struct after: 21
//= when in: 15
//= when after: 15
//= union after: 4

//--- a_narrowed_local_read_after_a_step_in_the_same_statement
// [ir-narrow] [op-inc-dec] A read later in the statement that stepped the
// local, or on the right of a stepping test's `&&`, reads the new value:
// the narrowing it reads is stepped with the storage.
fn start() [] -> Int? {
    return 12
}

fn main() [use] -> None {
    use StdConsole()
    let o = start()
    if o is Int {
        println("${o++} ${o++}")
        let s = o++ + o
        println("${s} ${o}")
    }
    let text = "0123456789abcdefghijklmnopqrstuvwxyz"
    let p = start()
    if p is Int {
        while ++p is Idx(text) && p < 15 {
            println("in: ${p}")
        }
        println("after: ${p}")
    }
    println("outside: ${o ?: -1} ${p ?: -1}")
}
//= stdout
//= 12 13
//= 29 15
//= in: 13
//= in: 14
//= after: 15
//= outside: 15 15
