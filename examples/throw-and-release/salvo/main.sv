// Leaving a block early, and releasing what you hold: `throw`/`try` for
// leaving with a message, linearity for making sure nothing is left open.
//
// The two belong together. `throw` is the earliest exit there is, and a
// resource-holding function has to be right on that path too — so the checker
// is what pairs them: a linear value still owed at any exit is an error, and a
// throw is an exit.
//
// There is no `defer` (removed 2026-09-10): a release written once and spliced
// at every exit was the *partial* answer to this, and linearity is the full
// one — it checks the obligation rather than discharging it behind your back.
// The cost is visible below: `close` appears on each path, and the order of
// the work has to put the release before anything that may throw.

// ===== 1. a resource that cannot be forgotten =====
//
// `: Linear<self>` says every value of this type carries a use obligation, and
// the `close` the group asks for is how it is discharged. Forgetting it on any
// path is a compile error naming the value and the path.
struct FileHandle : Linear<self> {
    // What was opened, for the trace this example prints.
    name: Str
}

fn open_file(name: Str) [Console] -> [] FileHandle {
    println("1. open ${name}")
    return FileHandle { name: name }
}

// The discharge. `close` consumes its parameter — the empty deduction list
// moves it — which is what makes it the release rather than a convention.
fn close(handle: FileHandle) [Console] -> [] None {
    println("1. close ${handle.name}")
}

// Two exits, two releases. Leave one out and the compiler says which path
// leaks: "`handle` still owns a linear value when it goes out of scope".
fn read_size(name: Str, want: Int) [Console] -> [] Int {
    let there_is = size(name)
    let handle = open_file(name)
    if want > there_is {
        println("1. asked for more than there is")
        close(handle)
        return there_is
    }
    close(handle)
    return want
}

// ===== 2. leaving early with a message: `throw` =====
//
// A function that may leave early declares `[Throw<Str>]` and keeps its own
// return type — `throw` returns `Nothing`, the bottom type, so the frames in
// between say nothing about it. There is no handler and no `catch`: the
// delimiter is `try`.
fn parse_port(text: Str) [Throw<Str>] -> [text] Int {
    let n = parse_int(text)
    if n is None {
        throw("not a number: ${text}")
    }
    if n < 1 {
        throw("port must be positive")
    }
    return n
}

// An intermediate frame only *inherits* the effect: nothing here mentions the
// outcome, and nothing here is a `?`-style forward — the call is an ordinary
// call.
fn port_of(config: Str) [Throw<Str>] -> [config] Int {
    let port = parse_port(config)
    return port * 1
}

// A resource *and* a throwing call in one function, which is the interaction
// worth seeing. The code after `parse_port` does not run on the throw path, so
// holding the handle across it is rejected — the release has to come first.
// `copy` is what lets the name outlive the handle: a plain binding would share
// its fate and die with it.
fn port_from_file(name: Str, text: Str) [Console, Throw<Str>] -> [text] Int {
    let handle = open_file(name)
    let from = copy(handle.name)
    close(handle)
    println("2. reading a port out of ${from}")
    return parse_port(text)
}

// Messages need not be strings, and two message types meeting at one
// delimiter make the thrown arm their union.
fn strict_port(text: Str) [Throw<Str | Int>] -> [text] Int {
    if size(text) == 0 {
        throw("empty")
    }
    let n = parse_int(text)
    if n is None {
        throw(size(text))
    }
    return n
}

// ===== 3. the delimiter: `try` =====
//
// `try { ... }` is a compiler intrinsic rather than an effect — there is no
// `Try` to declare and no handler to register. Its value is `Ok T | Thrown M`,
// an ordinary union, so `when` reads it like a result.
fn report(label: Str, config: Str) [Console] -> [label, config] None {
    let outcome = try {
        port_of(config)
    }
    when outcome {
        is Ok {
            println("3. ${label}: port ${outcome}")
        }
        is Thrown {
            println("3. ${label}: rejected — ${outcome}")
        }
    }
}

fn main() [use] {
    use StdOutConsole()

    let small = read_size("notes.txt", 3)
    println("1. read ${small}")
    let clamped = read_size("notes.txt", 99)
    println("1. read ${clamped}")

    report("good", "8080")
    report("bad", "http")

    // The throw path with a resource in the same function: the handle is
    // already closed when the throw happens, and the outcome arrives at the
    // delimiter.
    let guarded = try {
        port_from_file("ports.txt", "-1")
    }
    when guarded {
        is Ok {
            println("3. guarded: ${guarded}")
        }
        is Thrown {
            println("3. guarded: rejected — ${guarded}")
        }
    }

    // A union message: the thrown arm is `Thrown (Str | Int)`, and the union
    // inside it is read by binding it at the inner type.
    let mixed = try {
        strict_port("")
    }
    when mixed {
        is Ok {
            println("3. mixed: ${mixed}")
        }
        is Thrown {
            let why: Str | Int = mixed
            when why {
                is Str {
                    println("3. mixed: message ${why}")
                }
                is Int {
                    println("3. mixed: length ${why}")
                }
            }
        }
    }

    // A nested delimiter does not swallow an outer throw: the inner `try`
    // catches only what its own body throws.
    let outer = try {
        let inner = try {
            parse_port("nope")
        }
        when inner {
            is Ok {
                let got: Int = inner
                got
            }
            is Thrown {
                println("3. inner caught: ${inner}")
                parse_port("also nope")
            }
        }
    }
    when outer {
        is Ok {
            println("3. outer: ${outer}")
        }
        is Thrown {
            println("3. outer caught: ${outer}")
        }
    }
}
