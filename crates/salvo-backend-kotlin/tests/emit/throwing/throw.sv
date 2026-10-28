// Throwing [throw] [throw-try] [kt-throw-signal]: the signal class, `throw`
// and the catching `try`.

//--- throw_lowers_to_a_signal_and_try_to_a_catch
// [kt-throw-signal] The JVM's unwinding *is* the propagation: `throw`
// throws a generated stack-trace-less signal, an intermediate frame does
// nothing at all (no `ControlFlow`, no colouring), and `try` is Kotlin's
// own `try`/`catch` *expression*, so the outcome falls out of it. `Throw`
// is never a handler parameter.
// The whole of non-resumption in one program (`THROW_DEMO` in
// codegen_tests.rs, which also keeps the plain-propagation assertion scoped to
// `measure`).
// The signal class is generated once for the program.
//~ emits(throwsignal.kt): class ThrowSignal(val payload: Any?, val tag: String)
//~ emits(throwsignal.kt): RuntimeException(null, null, false, false)
// [throw] The effect itself emits nothing: there are no handlers to
// implement, so an interface for it would be dead code. (The test this came
// from looked in `std/core/throw.kt`, which is never emitted.)
//~ !emits(std/throw.kt): interface Throw
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

fn parse(line: Str) [Throw<Str>, Console] -> Int => !line {
    println("parse ${line}")
    if size(line) == 0 {
        // `throw` throws; the message is *not* wrapped at the throw (the
        // throwing frame cannot know which `try` will catch it).
        //~ emits: throw ThrowSignal("empty line", "Str")
        throw("empty line")
    }
    return size(line)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn measure(line: Str) [Throw<Str>, Console] -> Int => !line {
    let h = open_file(1)
    let fd = copy(h.fd)
    close_file(h)
    let n = parse(line)
    return n + fd
}

fn total(lines: Str[]) [Throw<Str>, Console] -> Int => !lines {
    let sum = 0
    for line in lines {
        // The delimiter picks the arm at the catch, by tag.
        //~ emits: catch (__signal: ThrowSignal)
        //~ emits: when (__signal.tag)
        //~ emits: else -> throw __signal
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

fn main() [use] -> None {
    use StdConsole
    report_text(try { measure("hello") })
    report_text(try { measure("") })
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

//--- a_variable_mutated_only_inside_a_try_body_is_declared_var
// [throw-try] A variable assigned *only* inside a `try` body still needs the
// mutable declaration. `collect_mutated_expr` had no `Expr::Try` arm, so it
// emitted `val counter` next to `counter = counter + 1` and kotlinc
// rejected the output — a [backend-never-wrong] miss caught by nothing but
// the toolchain. (`TRY_MUTATION_DEMO` in codegen_tests.rs.)
import std.throw

fn risky(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

fn main() [use] -> None {
    use StdConsole
    //~ emits: var counter: Int = 0
    let counter = 0
    let outcome = try {
        counter = counter + 1
        risky(3)
    }
    println("counter ${counter}")
}
