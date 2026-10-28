// literal unions and the `is` tests that narrow them
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [type-literal-qualified] A union of provenance-qualified literals compiles
// and runs: the qualified literals collapse into `Str` like bare ones,
// `is Short` compares against the literals carrying it, and a `when` over
// the arms dispatches. Before 2026-10-20 both backends emitted a two-arm
// union and constructed a third arm, and rustc and kotlinc refused the
// output.
//--- a_union_of_qualified_literals
provenance qualifier Short of Str
type Port = "port"
type FieldName = Short Port | Short "verbose" | "host"

fn describe(f: FieldName) [] -> Str => f {
    if f is Short {
        return "short ${f}"
    }
    return "long ${f}"
}

fn code(f: FieldName) [] -> Int => f {
    when f {
        is Short { return 1 }
        is "host" { return 2 }
    }
}

fn main() [use] -> None {
    use StdConsole()
    let h: FieldName = "host"
    println(describe(h))
    println("${code(h)}")
}
//= stdout
//= long host
//= 2

// [is-union] [type-literal] [type-rewrap] `s is "json" | "text"` narrows a
// `Str` to the literal union (into a field of that type), `x is Int | Str`
// and `x !is Bool` narrow to the sub-union (into a parameter of it), and a
// literal test on a plain `Str` compares the value — it used to pass for
// every string.
//--- union_is_tests_narrow
struct Opts {
    output: "json" | "text" = "text"
}

fn wants(v: Int | Str) [] -> Str => v {
    when v {
        is Int { return "int ${v}" }
        is Str { return "str ${v}" }
    }
}

fn describe(x: Int | Str | Bool) [] -> Str => x {
    if x is Int | Str {
        return wants(x)
    }
    return "bool"
}

fn main() [use] -> None {
    use StdConsole()
    for s in List("json", "xml", "text") {
        if s !is "json" | "text" {
            println("refused ${s}")
        }
        if s is "json" | "text" {
            let o = Opts { output: s }
            println("ok ${o.output}")
        }
    }
    println("${describe(3)} ${describe("a")} ${describe(true)} ${rest(4)} ${rest(false)}")
    println("${lit("json")}${lit("text")}${lit("xml")}")
}

fn rest(x: Int | Str | Bool) [] -> Str => x {
    if x !is Bool {
        return wants(x)
    }
    return "bool"
}

fn lit(s: Str) [] -> Str => s {
    when {
        s is "json" { return "J" }
        s is "text" { return "T" }
        else { return "?" }
    }
}
//= stdout
//= ok json
//= refused xml
//= ok text
//= int 3 str a bool int 4 bool
//= JT?

// [is-union] [type-literal] `s is "json"` on an un-narrowed `Str?` tests the
// `Str` arm for that value, and narrows `s` to the literal inside. It was
// refused as "this check can never succeed" until 2026-10-20 (ROADMAP §2d
// item 13): the narrowing matched the literal against the arms and found
// none, though the runtime test was already recorded.
//--- a_literal_test_on_an_optional_str
fn pick(s: Str?) [] -> Str => !s {
    if s is "json" {
        return s
    }
    if s is None {
        return "none"
    }
    return "other ${s}"
}

fn main() [use] -> None {
    use StdConsole()
    println(pick("json"))
    println(pick("x"))
    println(pick(None))
}
//= stdout
//= json
//= other x
//= none

// [type-literal] [type-union-arm-identity] Unions of literals: an open
// union (`Other Str`) that is a plain string at run time, `is "A"` and
// `is Other` in a `when`, a `?`-typed literal union assigned later and
// narrowed, a named literal sub-union tested with `is`, widening to `Str`
// for a call and for `==`, and a mixed union whose two `Str` literals share
// one arm beside an `Int` and a struct.
//--- unions_of_literals
type Class = "STANDARD" | "GLACIER" | Other Str

fn describe_class(c: Class) [] -> Str => c {
    when c {
        is "STANDARD" { return "hot" }
        is "GLACIER" { return "cold" }
        is Other { return "unknown: ${c}" }
    }
}

fn part_one() [Console] {
    let a: Class = "STANDARD"
    println(describe_class(a))
    println(describe_class("GLACIER"))
    println(describe_class(other("DEEP")))
    let later: Class? = None
    later = "GLACIER"
    if later is None {
        return None
    }
    println(describe_class(later))
}

struct Person { name: Str }

type Kms = "KmsDisabled" | "KmsThrottled"
type Code = Kms | "QueueDoesNotExist" | Other Str
type Mixed = "name" | 3 | "other" | Person

// An exported struct has a wire codec, and a field whose type is an alias
// holding an alias of literals must collapse to its base there too.
export struct Failure { code: Code, status: Int }

fn kind(c: Code) [] -> Str => c {
    if c is Kms {
        return "kms"
    }
    when c {
        is "QueueDoesNotExist" { return "missing" }
        is Other { return "other ${c}" }
    }
}

fn shout(s: Str) [] -> Str => s {
    return "${s}!"
}

fn mixed(m: Mixed) [] -> Str => m {
    when m {
        is "name" { return "the name" }
        is 3 { return "three" }
        is "other" { return "the other" }
        is Person { return "person ${m.name}" }
    }
}

fn main() [use] {
    use StdConsole()
    part_one()
    println(kind("KmsThrottled"))
    println(kind("QueueDoesNotExist"))
    println(kind(other("Throttling")))
    let c: Code = "KmsDisabled"
    println(shout(c))
    if c == "KmsDisabled" {
        println("equal")
    }
    println(mixed("name"))
    println(mixed(3))
    println(mixed("other"))
    println(mixed(Person { name: "Ada" }))
    let f = Failure { code: "QueueDoesNotExist", status: 400 }
    println("${kind(f.code)} ${f.status}")
}
//= stdout
//= hot
//= cold
//= unknown: DEEP
//= cold
//= kms
//= missing
//= other Throttling
//= KmsDisabled!
//= equal
//= the name
//= three
//= the other
//= person Ada
//= missing 400
