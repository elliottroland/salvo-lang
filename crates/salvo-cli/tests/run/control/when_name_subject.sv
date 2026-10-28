// [when-bind] `when name = subject { … }`
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- when_binding

struct Meta {
    last_updated: Long?
}

fn nanos(m: Meta) -> Long {
    return when last_updated = m.last_updated {
        is Long { last_updated }
        is None { -1L }
    }
}

fn main() [use] {
    use StdConsole()
    println(to_str(nanos(Meta { last_updated: 7L })))
    println(to_str(nanos(Meta { last_updated: None })))
    let last_updated = 99L
    println(to_str(last_updated))
}
//= stdout
//= 7
//= -1
//= 99
