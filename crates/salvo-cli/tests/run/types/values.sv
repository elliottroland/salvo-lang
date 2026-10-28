// values: accumulated errors, parsed scalars, printing and copying
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [type-result-errs] The accumulating outcome keeps every error: `map2` of
// two successes, of two failures (both errors, in order), and of one
// failure.
//--- accumulated_errors
fn port(s: Str) [] -> Ok Int | Errs NonEmpty List<Str> => s {
    let n = parse_int(s)
    if n is None {
        return errs("not a port: ${s}")
    }
    return Ok(n)
}

fn name(s: Str) [] -> Ok Str | Errs NonEmpty List<Str> => !s {
    if size(s) == 0 {
        return errs("empty name")
    }
    return Ok(s)
}

fn show(o: Ok Str | Errs NonEmpty List<Str>) [] -> Str => o {
    when o {
        is Ok { return "ok ${o}" }
        is Errs { return "errors ${o}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(show(map2(port("80"), name("web"), (p: Int, n: Str) -> "${n}:${p}")))
    println(show(map2(port("x"), name(""), (p: Int, n: Str) -> "${n}:${p}")))
    println(show(map2(port("x"), name("ok"), (p: Int, n: Str) -> "${n}:${p}")))
}
//= stdout
//= ok web:80
//= errors [not a port: x, empty name]
//= errors [not a port: x]

// [str-parse] The value parsers read the same strings on both backends:
// `parse_double` checks one decimal grammar before converting (the hosts'
// own parsers disagree on whitespace, `NaN`, `inf`, hex and `f` suffixes),
// `parse_bool` takes exactly `true`/`false`, and `from_str` is one name for
// every scalar, chosen by the expected type and resolvable as an implicit.
//--- the_value_parsers_agree
fn show(d: Double?) [] -> Str => d {
    if d is None { return "none" }
    return "${d}"
}

fn read<T>(s: Str, ?from_str: (s: Str) -> T?) [] -> T? => s {
    return from_str(s)
}

fn main() [use] -> None {
    use StdConsole()
    for s in List("1.5", "-2", "1e3", "2.5E-4", ".5", "1.", " 1", "1.5f", "NaN", "inf", "0x10", "", "+", "1e", "-.5e+2") {
        println("${s}|${show(parse_double(s))}")
    }
    let l: Long? = from_str("9000000000")
    let b: Bool? = from_str("true")
    let bad: Bool? = from_str("True")
    let i: Int? = read("42")
    let t: Str? = from_str("text")
    println("${l ?: 0L} ${b ?: false} ${bad is None} ${i ?: 0} ${t ?: "-"}")
}
//= stdout
//= 1.5|1.5
//= -2|-2.0
//= 1e3|1000.0
//= 2.5E-4|2.5E-4
//= .5|0.5
//= 1.|1.0
//=  1|none
//= 1.5f|none
//= NaN|none
//= inf|none
//= 0x10|none
//= |none
//= +|none
//= 1e|none
//= -.5e+2|-50.0
//= 9000000000 true true 42 text

// [rs-narrow] A scalar arm of a union, narrowed and handed to an intrinsic
// (`to_str(value)`), reads through the arm rather than the whole union: a
// generic union with a generic struct arm used to render `*value` and need
// `Box<A>: Display` (closed 2026-10-06).
//--- a_generic_union_with_a_scalar_arm_prints
struct Box<A> : ToStr<self> by auto {
    v: A
}

type Either<A> = Box<A> | Int : ToStr<self> by auto

fn main() [use] {
    use StdConsole()
    let e: Either<Str> = Box { v: "x" }
    let f: Either<Str> = 7
    println("${e} ${f}")
}
//= stdout
//= Box { v: x } 7

// [kt-copy] [copy-plan] A struct that holds a list of itself copies deeply:
// Kotlin generates the struct's own copy fn.
//--- a_struct_holding_itself_copies
struct Node canbe Mut {
    label: Str
    kids: List<Mut Node>
}

fn relabel(n: Mut Node) [] -> None {
    n.label = "changed"
}

fn main() [use] {
    use StdConsole()
    let kid: Mut Node = Mut Node { label: "kid", kids: List<Mut Node>() }
    let root: Mut Node = Mut Node { label: "root", kids: List(kid) }
    let twin = copy(root)
    relabel(root)
    println("${root.label} ${twin.label} ${size(twin.kids)}")
}
//= stdout
//= changed root 1
