// throw and `try` [throw] [throw-try] [rs-throw-controlflow].

//--- throw_and_try
// The whole of non-resumption in one program: propagation through a frame
// that declares the effect, a linear resource released *before* the call that
// may throw (which is the only way since `defer` was removed, 2026-09-10 —
// the checker rejects holding it across the call), two message types meeting
// at one delimiter
// (`Thrown (Str | Int)`), a may-throw call inside a loop, and a nested
// delimiter that must not swallow the outer throw.
//
// throw_lowers_to_controlflow:
// [rs-throw-controlflow] A fn that may throw returns `ControlFlow<M, T>`:
// the message type *is* the `Break` payload, so `throw` is a plain return
// and propagation is `?` — no handler, no dispatch, no allocation. `Throw`
// is never a `&mut dyn` parameter.
//
// try_lowers_to_a_labelled_block:
// [rs-try-label] `try` is a *labelled block*, not a closure: nothing is
// captured (the body reads the fn's effect parameters directly), and an
// throw inside it breaks the label with the outcome's thrown arm.

// [throw] The effect emits no trait: there are no handlers to implement.
//~ !emits(std/throw.rs): trait Throw
import std.throw

linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }


fn open_file(n: Int) [Console] -> FileHandle {
    println("open ${n}")
    return FileHandle {fd: n}
}

fn close_file(h: FileHandle) [Console] -> None => !h {
    println("close fd=${h.fd}")
    close(h)
}

//~ emits(main.rs): pub fn parse(console: &crate::std_core_console::Console, mut line: String) -> std::ops::ControlFlow<String, i32>
fn parse(line: Str) [Throw<Str>, Console] -> Int => !line {
    println("parse ${line}")
    if size(line) == 0 {
        //~ emits(main.rs): return std::ops::ControlFlow::Break(String::from("empty line"));
        throw("empty line")
    }
    //~ emits(main.rs): return std::ops::ControlFlow::Continue(
    return size(line)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

// [rs-exit-splice] With nothing pending at the exit, propagation is the
// plain re-raise: the release was written before the call, so the throw
// path owes nothing.
//~ !emits(main.rs): ControlFlow::Break(__m) => {
fn measure(line: Str) [Throw<Str>, Console] -> Int => !line {
    let h = open_file(1)
    let fd = copy(h.fd)
    //~ emits(main.rs): crate::close_file(console, h);
    //~|     let mut n: i32 = (match crate::parse(console, line) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => return std::ops::ControlFlow::Break(__m) });
    close_file(h)
    let n = parse(line)
    return n + fd
}

fn total(lines: Str[]) [Throw<Str>, Console] -> Int => !lines {
    let sum = 0
    for line in lines {
        //~ emits(main.rs): break 'try_1 crate::unions::Union2::U2(__m)
        let inner = try {
            limit(size(line))
        }
        when inner {
            is Ok {
                println("within limit ${inner}")
            }
            is Thrown {
                println("over limit ${inner}")
            }
        }
        sum = sum + parse(line)
    }
    return sum
}

fn report_text(outcome: Ok Int | Thrown Str) [Console] -> None => !outcome {
    when outcome {
        is Ok {
            println("ok ${outcome}")
        }
        is Thrown {
            println("thrown: ${outcome}")
        }
    }
}

//~ emits(main.rs): 'try_1: {
//~ !emits(main.rs): || -> ControlFlow
fn main() [use] -> None {
    use StdConsole
    report_text(try { measure("hello") })
    report_text(try { measure("") })
    // Two message types meeting at one delimiter wrap into the message
    // union's arms [type-union-arm-identity]; a single type stays bare.
    //~ emits(main.rs): break 'try_3 crate::unions::Union2::U2(crate::unions::Union2::U1(__m))
    //~ emits(main.rs): break 'try_3 crate::unions::Union2::U2(crate::unions::Union2::U2(__m))
    let mixed = try {
        let n = measure("longer line")
        limit(n)
    }
    when mixed {
        is Ok {
            println("mixed ok ${mixed}")
        }
        is Thrown {
            println("mixed thrown")
        }
    }
    let counted = try {
        total(array_of("ab", "cdefg"))
    }
    when counted {
        is Ok {
            println("counted ${counted}")
        }
        is Thrown {
            println("counted thrown: ${counted}")
        }
    }
    println("done")
}
