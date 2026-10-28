// [op-arm-eq] `union == value` picks the arm by the value's type
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- arm_equality

fn normalize(raw: Long) -> Long? {
    let last_updated: Long? = raw
    if last_updated == -1L { last_updated = None }
    return last_updated
}

fn show(v: Long?) -> Str {
    return when u = v {
        is Long { to_str(u) }
        is None { "none" }
    }
}

fn main() [use] {
    use StdConsole()
    println(show(normalize(-1L)))
    println(show(normalize(42L)))
    // `!=` is the negation: present-and-not-(-1), or absent.
    let n: Long? = 7L
    println(to_str(n != -1L))
    let m: Long? = None
    println(to_str(m != -1L))
}
//= stdout
//= none
//= 42
//= true
//= true

//--- arm_equality_2

fn normalize(raw: Long) -> Long? {
    let last_updated: Long? = raw
    if last_updated == -1L { last_updated = None }
    return last_updated
}

fn show(v: Long?) -> Str {
    return when u = v {
        is Long { to_str(u) }
        is None { "none" }
    }
}

fn main() [use] {
    use StdConsole()
    println(show(normalize(-1L)))
    println(show(normalize(42L)))
    let n: Long? = 7L
    println(to_str(n != -1L))
    let m: Long? = None
    println(to_str(m != -1L))
}
//= stdout
//= none
//= 42
//= true
//= true
