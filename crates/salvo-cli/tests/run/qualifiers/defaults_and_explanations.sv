// qualifier defaults, constants and explanations
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [default-predicate] A default whose slot carries a predicate qualifier it
// does not satisfy is tested where it is applied and traps there (user
// decision 2026-10-20, the CLI design record round 2 Q2): a parameter
// default that holds runs, and a field default that does not stops the
// program at the literal with a message naming the field and the claim.
// Before the fix the default was not checked at all and printed 8080
// through an `InRange(1, 10) Int`.
//--- a_predicate_default_is_tested_where_it_is_applied
struct Opts {
    port: InRange(1, 10) Int = 8080
    ok: InRange(1, 10) Int = 3
}

fn needs(p: InRange(1, 10) Int) [] -> Int => p { return p + 0 }
fn f(p: InRange(1, 10) Int = 5) [] -> Int => p { return needs(p) }

fn main() [use] -> None {
    use StdConsole()
    println("${f()}")
    let o = Opts {}
    println("${needs(o.ok)} ${needs(o.port)}")
}
//= trap the default for field `port` is not `InRange(1, 10) Int`
//= stdout
//= 5

// [qual-const] `Char` and `Str` constants: an `is` test passes the constant
// to `qualifies`, and a predicate-qualified default is tested with it where
// it is applied.
//--- char_and_str_qualifier_constants
qualifier Prefix(p: Str) of Str {
    fn qualifies(s: Str, p: Str) -> Bool {
        return starts_with(s, p)
    }
}

qualifier Initial(c: Char) of Str {
    fn qualifies(s: Str, c: Char) -> Bool {
        return char_at(s, 0) == c
    }
}

fn needs(a: Prefix("arn:") Str) [] -> Str {
    return "ok ${a}"
}

struct Opts {
    name: Initial('p') Str = "port",
    arn: Prefix("arn:") Str = "arn:aws:s3"
}

fn main() [use] -> None {
    use StdConsole()
    let x = "arn:aws:sqs"
    if x is Prefix("arn:") {
        println(needs(x))
    }
    let y = "http"
    println("${y is Prefix("arn:")} ${y is Initial('h')}")
    let o = Opts {}
    println("${o.name} ${needs(o.arn)}")
}
//= stdout
//= ok arn:aws:sqs
//= false true
//= port ok arn:aws:s3

// [qual-const] The same program with a failing `Char` default: it traps
// naming the claim with its constant.
//--- a_failing_char_qualifier_default_traps
qualifier Prefix(p: Str) of Str {
    fn qualifies(s: Str, p: Str) -> Bool {
        return starts_with(s, p)
    }
}

qualifier Initial(c: Char) of Str {
    fn qualifies(s: Str, c: Char) -> Bool {
        return char_at(s, 0) == c
    }
}

fn needs(a: Prefix("arn:") Str) [] -> Str {
    return "ok ${a}"
}

struct Opts {
    name: Initial('p') Str = "quit",
    arn: Prefix("arn:") Str = "arn:aws:s3"
}

fn main() [use] -> None {
    use StdConsole()
    let x = "arn:aws:sqs"
    if x is Prefix("arn:") {
        println(needs(x))
    }
    let y = "http"
    println("${y is Prefix("arn:")} ${y is Initial('h')}")
    let o = Opts {}
    println("${o.name} ${needs(o.arn)}")
}
//= trap the default for field `name` is not `Initial('p') Str`
//= stdout
//= ok arn:aws:sqs
//= false true

// [qual-whynot] A predicate qualifier says why a value failed: `whynot?`
// answers `None` or the first failed check's message — its `@whynot`
// template filled, or "<value> is not Q(args)" — and an `assert!(x is Q)`
// with no message of its own traps with the same text. [args-explain] A
// derived parser reports a field's failed qualifier the same way, a user
// qualifier as well as std's `InRange`.
//--- whynot_messages
import std.args

qualifier Even of Int {
    // @whynot [n] is odd
    fn qualifies(n: Int) -> Bool {
        return n % 2 == 0
    }
}

qualifier Small(hi: Int) of Int {
    fn qualifies(n: Int, hi: Int) -> Bool {
        return n < hi
    }
}

qualifier Brief(most: Int) of Str {
    // @whynot "[s]" is longer than [most] characters
    fn qualifies(s: Str, most: Int) -> Bool {
        return size(s) <= most
    }
}

struct P {
    x: Int
}

qualifier Origin of P {
    fn qualifies(p: P) -> Bool {
        return p.x == 0
    }
}

struct Opts : FromArgs<self> by args {
    count: Even Int = 2,
    name: Brief(3) Str = "abc"
}

fn seven() [] -> Int {
    return 7
}

fn show(m: Str?) [Console] -> None => m {
    if m is None {
        println("ok")
    } else {
        println(m)
    }
}

fn main() [use] -> None {
    use StdConsole()
    let n = 70000
    let lim = 5
    show(whynot?(n is InRange(1, 65535) Int))
    show(whynot?(n is Even InRange(1, 65535) Int))
    show(whynot?(seven() is Even Int))
    show(whynot?(n is Small(10) Int))
    show(whynot?(n is Small(lim) Int))
    show(whynot?(n is Int))
    show(whynot?(4 is Even Int))
    let p = P { x: 3 }
    show(whynot?(p is Origin P))
    show(whynot?("long" is Brief(3) Str))
    let r: Ok Opts | Errs NonEmpty List<ArgError> = parse_args(List("--count", "3", "--name", "abcd"))
    when r {
        is Ok { println("parsed") }
        is Errs { for e in r { println(message(copy(e))) } }
    }
    let x = 70000
    assert!(x is InRange(1, 65535))
}
//= trap 70000 is not in 1..65535 at main:
//= stdout
//= 70000 is not in 1..65535
//= 70000 is not in 1..65535
//= 7 is odd
//= 70000 is not Small(10)
//= 70000 is not Small(lim)
//= ok
//= ok
//= the value is not Origin
//= "long" is longer than 3 characters
//= invalid value '3' for '--count': 3 is odd
//= invalid value 'abcd' for '--name': "abcd" is longer than 3 characters
