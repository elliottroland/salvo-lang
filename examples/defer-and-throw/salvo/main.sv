// Leaving a block, on every path: `defer` for the way out, `throw`/`try` for
// leaving early with a message.
//
// The two belong together. `defer` is what makes early exit safe — a release
// written once runs on every path out of its block — and `throw` is the
// earliest exit there is, so the pair is what a resource-holding function
// needs.

// ===== 1. `defer` runs at the end of its block, latest first =====
//
// `defer` is *block*-scoped, not function-scoped, and its body is spliced at
// every exit of the enclosing block: its end, and each
// `return`/`break`/`continue` that leaves it. Several in one block run in
// reverse declaration order, so a release always precedes what it depends on.
fn lifo() [Console] -> None {
    println("1. enter")
    defer { println("1. first declared, last to run") }
    defer { println("1. second declared, first to run") }
    println("1. body")
}

// A `defer` in a loop body belongs to *that* body, so it runs once per
// iteration — including on the iteration that `continue`s or `break`s out.
fn per_iteration() [Console] -> None {
    let i = 0
    while i < 4 {
        defer { println("1. leaving iteration ${i}") }
        i = i + 1
        if i == 2 {
            continue
        }
        if i == 3 {
            break
        }
        println("1. working on iteration ${i}")
    }
}

// ===== 2. a resource that cannot be forgotten =====
//
// `: Linear<self>` says every value of this type carries a use obligation, and
// the `close` the group asks for is how it is discharged. Forgetting it on any
// path is a compile error, so `defer { close(h) }` is the pattern: written
// once, it covers the ordinary end, the early `return` and the throw path
// below.
struct FileHandle : Linear<self> {
    // What was opened, for the trace this example prints.
    name: Str
}

fn open_file(name: Str) [Console] -> [] FileHandle {
    println("2. open ${name}")
    return FileHandle { name: name }
}

// The discharge. `close` consumes its parameter — the empty deduction list
// moves it — which is what makes it the release rather than a convention.
fn close(handle: FileHandle) [Console] -> [] None {
    println("2. close ${handle.name}")
}

// Two exits, one release: the `defer` runs on both.
fn read_size(name: Str, want: Int) [Console] -> [] Int {
    let there_is = size(name)
    let handle = open_file(name)
    defer { close(handle) }
    if want > there_is {
        println("2. asked for more than there is")
        return there_is
    }
    return want
}

// ===== 3. leaving early with a message: `throw` =====
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

// The same, with a resource live across the throwing call, which is the
// interaction worth seeing: the `defer` releases the handle on the throw path
// too, and the checker is what guarantees it — the obligation has to be
// discharged on *every* exit, and a throw is one.
fn port_from_file(name: Str, text: Str) [Console, Throw<Str>] -> [text] Int {
    let handle = open_file(name)
    defer { close(handle) }
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

// ===== 4. the delimiter: `try` =====
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
            println("4. ${label}: port ${outcome}")
        }
        is Thrown {
            println("4. ${label}: rejected — ${outcome}")
        }
    }
}

fn main() [use] {
    use StdOutConsole()

    lifo()
    per_iteration()

    let small = read_size("notes.txt", 3)
    println("2. read ${small}")
    let clamped = read_size("notes.txt", 99)
    println("2. read ${clamped}")

    report("good", "8080")
    report("bad", "http")

    // The throw path with a live resource: `close` still runs, and the
    // outcome arrives at the delimiter.
    let guarded = try {
        port_from_file("ports.txt", "-1")
    }
    when guarded {
        is Ok {
            println("4. guarded: ${guarded}")
        }
        is Thrown {
            println("4. guarded: rejected — ${guarded}")
        }
    }

    // A union message: the thrown arm is `Thrown (Str | Int)`, and the union
    // inside it is read by binding it at the inner type.
    let mixed = try {
        strict_port("")
    }
    when mixed {
        is Ok {
            println("4. mixed: ${mixed}")
        }
        is Thrown {
            let why: Str | Int = mixed
            when why {
                is Str {
                    println("4. mixed: message ${why}")
                }
                is Int {
                    println("4. mixed: length ${why}")
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
                println("4. inner caught: ${inner}")
                parse_port("also nope")
            }
        }
    }
    when outer {
        is Ok {
            println("4. outer: ${outer}")
        }
        is Thrown {
            println("4. outer caught: ${outer}")
        }
    }
}
